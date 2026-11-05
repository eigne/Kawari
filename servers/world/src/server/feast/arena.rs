//! Runs a medal match (`rules.rs`) inside a world instance of the arena.
//!
//! Every world tick, `tick` collects the participants, steps the rules and then
//! makes the world match them: team spawns, jail gates, battalions (who is an
//! enemy), Marked for Culling, medal piles on the ground, chat announcements
//! and the director snapshots the HUD reads.
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
    time::{Duration, Instant},
};

use kawari::{
    common::{
        ContentDirectorEvent, DirectorEvent, InstanceContentDirectorEvent, ObjectId, Position,
        SharedGroupTimelineState,
    },
    ipc::zone::{ActorControlCategory, ObjectKind, SpawnObject},
};
use parking_lot::Mutex;

use super::rules::{Input, Match, Phase, Team};
use crate::{
    FromServer,
    common::PartyUpdateTargets,
    server::{
        actor::NetworkedActor,
        instance::Instance,
        network::{DestinationNetwork, NetworkState},
    },
};

/// How long a KO'd player must wait before returning.
pub const RESPAWN_DELAY: Duration = Duration::from_secs(10);
/// An arena without players keeps its match this long, so a reconnect resumes it.
const EMPTY_ARENA_GRACE: f64 = 30.0;
/// Director snapshots per second are 1 / this.
const DIRECTOR_INTERVAL: f64 = 0.25;
/// The Marked for Culling status, reused for its native stack icon.
const MARKED_FOR_CULLING: u16 = 994;

// The start_col layer centers the two jails at x = +/-58, y = 0.25. Collision
// (first two) and VFX (last two) are separate shared groups in the layout.
const JAIL_LAYOUTS: [u32; 4] = [7403947, 7403946, 7515404, 7515405];
const JAIL_EOBJS: [u32; 2] = [2008986, 2008987];

/// One match in one arena instance, plus the world objects it owns.
#[derive(Debug)]
pub struct Arena {
    started: Instant,
    pub rules: Match,
    pub supplies: super::supplies::Supplies,
    /// Medal pile object for each `rules.drops` entry.
    medal_piles: BTreeMap<u32, ObjectId>,
    /// Picked-up piles still playing their pickup animation, until a deadline.
    fading: BTreeMap<u32, (ObjectId, f64)>,
    empty_since: Option<f64>,
    /// Players already moved into their team's jail this match.
    placed: BTreeSet<u32>,
    /// Gate state each (viewer, controller) last received; true is closed.
    gate_viewers: BTreeMap<(u32, u32), bool>,
    /// Pile look each (viewer, pile) last received.
    medal_viewers: BTreeMap<(u32, u32), SharedGroupTimelineState>,
    next_director_snapshot: f64,
}

impl Arena {
    pub fn new(rules: Match) -> Self {
        Self {
            started: Instant::now(),
            rules,
            supplies: Default::default(),
            medal_piles: BTreeMap::new(),
            fading: BTreeMap::new(),
            empty_since: None,
            placed: BTreeSet::new(),
            gate_viewers: BTreeMap::new(),
            medal_viewers: BTreeMap::new(),
            next_director_snapshot: 0.0,
        }
    }

    /// Seconds since this arena started; the clock `rules` runs on.
    pub fn now(&self) -> f64 {
        self.started.elapsed().as_secs_f64()
    }

    /// Removes every object this match spawned.
    fn despawn(mut self, instance: &mut Instance, network: &Arc<Mutex<NetworkState>>) {
        self.supplies.clear(instance, network);
        for object in self
            .medal_piles
            .values()
            .chain(self.fading.values().map(|(id, _)| id))
        {
            network.lock().remove_actor(instance, *object);
        }
    }
}

/// Ends the current match, if any, and forgets all combat state. The next
/// tick starts a fresh match with a new combat engine.
pub fn end_match(instance: &mut Instance, network: &Arc<Mutex<NetworkState>>) {
    if let Some(arena) = instance.feast.arena.take() {
        arena.despawn(instance, network);
    }
    instance.feast.combat = None;
}

/// Native duty time for a player zoning into the arena, from the live match clock.
pub fn duty_time_left(instance: &Instance) -> Option<u32> {
    let arena = instance.feast.arena.as_ref()?;
    Some(arena.rules.duty_time_left(arena.now()))
}

/// Pile model by size: the original Feast prefabs grow with the medal count.
fn medal_prefab(medals: u16) -> u32 {
    match medals {
        0..=25 => 2007627,
        26..=50 => 2007628,
        51..=75 => 2007629,
        _ => 2008631,
    }
}

/// Pile look for one viewer: team colour, dimmed if they cannot collect it.
fn medal_state(team: Team, blocked: bool) -> SharedGroupTimelineState {
    // All four prefabs expose normal Fangs/Claws in slots 3/5 and maxed in 9/11.
    SharedGroupTimelineState::from_bits_retain(
        1 << match (team, blocked) {
            (Team::Fang, false) => 2,
            (Team::Claw, false) => 4,
            (Team::Fang, true) => 8,
            (Team::Claw, true) => 10,
        },
    )
}

/// Where a team member stands in their jail (four slots per side).
pub fn team_spawn(team: Team, slot: usize) -> (Position, f32) {
    let (x, rotation) = match team {
        Team::Fang => (-58.0, std::f32::consts::FRAC_PI_2),
        Team::Claw => (58.0, -std::f32::consts::FRAC_PI_2),
    };
    (
        Position(glam::Vec3A::new(x, 0.5, (slot % 4) as f32 * 2.0 - 3.0)),
        rotation,
    )
}

/// Battalions 1-3 are mutually hostile in the Battalion sheet (the Frontline
/// Grand Companies). The client derives player hostility from them, so each
/// team takes one and opposing players become targetable enemies. The client
/// only compares player battalions when the territory's BattalionMode is 4;
/// territory 745 now ships with 1, which the Feast HUD changes in memory.
pub fn team_battalion(team: Team) -> u8 {
    match team {
        Team::Fang => 1,
        Team::Claw => 2,
    }
}

/// A zone load means a fresh client view: resend gates and pile looks, and
/// move the player into their jail again if the match has not started.
pub fn on_zone_loaded(instance: &mut Instance, actor: ObjectId) {
    if let Some(arena) = &mut instance.feast.arena {
        arena.placed.remove(&actor.0);
        arena
            .medal_viewers
            .retain(|(viewer, _), _| *viewer != actor.0);
        arena
            .gate_viewers
            .retain(|(viewer, _), _| *viewer != actor.0);
    }
}

/// Starts the respawn wait at the first death notification; later ones keep it.
pub fn begin_respawn_lock(instance: &mut Instance, actor: ObjectId, now: Instant) {
    instance
        .feast
        .respawns
        .entry(actor)
        .or_insert(now + RESPAWN_DELAY);
}

/// Milliseconds until `actor` may return, zero if they may now.
pub fn respawn_remaining(instance: &Instance, actor: ObjectId, now: Instant) -> u32 {
    instance
        .feast
        .respawns
        .get(&actor)
        .map(|until| {
            until
                .saturating_duration_since(now)
                .as_millis()
                .min(u32::MAX as u128) as u32
        })
        .unwrap_or(0)
}

/// Where a returning player reappears: their own team's jail.
pub fn respawn_position(instance: &Instance, actor: ObjectId) -> (Position, f32) {
    let team = instance
        .feast
        .arena
        .as_ref()
        .and_then(|a| a.rules.players.get(&actor.0))
        .map(|p| p.team)
        .unwrap_or(Team::Fang);
    team_spawn(team, 1)
}

/// Whether `actor` may use actions now. Outside a match (no arena) everyone may.
pub fn can_act(instance: &Instance, actor: ObjectId) -> bool {
    instance.feast.arena.as_ref().is_none_or(|a| {
        instance.find_actor(actor).is_some_and(|actor_data| {
            a.rules
                .active(actor.0, actor_data.get_common_spawn().class_job)
        })
    })
}

/// Players and practice dummies that can take part in the match.
fn participants(instance: &Instance) -> Vec<Input> {
    instance
        .actors
        .iter()
        .filter_map(|(id, actor)| {
            if !matches!(actor, NetworkedActor::Player { .. })
                && !instance.feast.practice.contains_key(id)
            {
                return None;
            }
            let s = actor.get_common_spawn();
            // Not yet initialized by the zone load.
            if s.max_health_points == 0 {
                return None;
            }
            Some(Input {
                id: id.0,
                job: s.class_job,
                alive: s.health_points > 0,
                position: s.position.0.to_array(),
            })
        })
        .collect()
}

fn is_player(instance: &Instance, id: ObjectId) -> bool {
    matches!(instance.find_actor(id), Some(NetworkedActor::Player { .. }))
}

/// One world tick of the match. Called for every instance; does nothing
/// outside the arena.
pub fn tick(instance: &mut Instance, network: &Arc<Mutex<NetworkState>>) {
    if !super::in_arena(instance) {
        return;
    }
    let inputs = participants(instance);
    let has_players = instance
        .actors
        .values()
        .any(|actor| matches!(actor, NetworkedActor::Player { .. }));
    if !has_players {
        tick_empty_arena(instance, network);
        return;
    }
    // Take the arena out of the instance so both can be borrowed mutably.
    let mut arena = instance
        .feast
        .arena
        .take()
        .unwrap_or_else(|| Arena::new(Match::new(0.0)));
    instance
        .feast
        .respawns
        .retain(|id, _| inputs.iter().any(|i| i.id == id.0 && !i.alive));
    for input in inputs.iter().filter(|i| !i.alive) {
        if is_player(instance, ObjectId(input.id)) {
            begin_respawn_lock(instance, ObjectId(input.id), Instant::now());
        }
    }
    let now = arena.now();
    arena.empty_since = None;
    let previous_phase = arena.rules.phase;
    arena.rules.step(now, &inputs);
    if previous_phase != arena.rules.phase && !matches!(arena.rules.phase, Phase::Finished(_)) {
        restart_duty_timer(instance, network, &arena.rules, now, &inputs);
    }
    sync_culling(instance, network, &arena.rules, now, &inputs);
    super::supplies::tick(instance, network, &arena.rules, &mut arena.supplies, now);
    sync_battalions(instance, network, &arena.rules, &inputs);
    split_parties(network, &arena.rules);
    sync_jails(instance, network, &mut arena, &inputs);
    sync_medal_piles(instance, network, &mut arena, now, &inputs);
    for message in arena.rules.announcements.drain(..) {
        tracing::info!(%message, "Feast match");
        for input in &inputs {
            super::notice(network, ObjectId(input.id), message.clone());
        }
    }
    if now >= arena.next_director_snapshot {
        super::director::send(instance, &arena.rules, now, &mut network.lock());
        arena.next_director_snapshot = now + DIRECTOR_INTERVAL;
    }
    instance.feast.arena = Some(arena);
}

/// Keeps the match for a while so reconnecting players resume it, then
/// removes everything.
fn tick_empty_arena(instance: &mut Instance, network: &Arc<Mutex<NetworkState>>) {
    let Some(mut arena) = instance.feast.arena.take() else {
        return;
    };
    let now = arena.now();
    let empty_since = *arena.empty_since.get_or_insert(now);
    arena.rules.step(now, &[]);
    if now - empty_since < EMPTY_ARENA_GRACE {
        instance.feast.arena = Some(arena);
        return;
    }
    arena.despawn(instance, network);
    instance.feast.combat = None;
    let practice: Vec<_> = instance.feast.practice.keys().copied().collect();
    for id in practice {
        network.lock().remove_actor(instance, id);
    }
    instance.feast.practice.clear();
}

/// Keeps the native duty timer on the match clock, including restarts and
/// the sudden-death extension. The client ignores SetDutyTimeRemaining for
/// this duty, so the timer is restarted instead.
fn restart_duty_timer(
    instance: &Instance,
    network: &Arc<Mutex<NetworkState>>,
    rules: &Match,
    now: f64,
    inputs: &[Input],
) {
    let Some(director) = instance.directors.first() else {
        return;
    };
    let time_limit = rules.duty_time_left(now);
    for input in inputs {
        let id = ObjectId(input.id);
        if is_player(instance, id) {
            network.lock().send_to_by_actor_id(
                id,
                FromServer::ActorControlSelf(ActorControlCategory::DirectorEvent {
                    handler_id: director.id,
                    event: DirectorEvent::ContentDirector(ContentDirectorEvent::InstanceContent(
                        InstanceContentDirectorEvent::DutyCommence { time_limit },
                    )),
                }),
                DestinationNetwork::ZoneClients,
            );
        }
    }
}

/// Applies Marked for Culling stacks to every living participant.
fn sync_culling(
    instance: &mut Instance,
    network: &Arc<Mutex<NetworkState>>,
    rules: &Match,
    now: f64,
    inputs: &[Input],
) {
    let stacks = rules.culling_stacks(now);
    for input in inputs {
        let id = ObjectId(input.id);
        let desired = if input.alive { u16::from(stacks) } else { 0 };
        let Some(effects) = instance
            .find_actor_mut(id)
            .and_then(|a| a.status_effects_mut())
        else {
            continue;
        };
        if effects.get(MARKED_FOR_CULLING).map_or(0, |s| s.param) != desired {
            effects.remove(MARKED_FOR_CULLING);
            if desired > 0 {
                effects.add(MARKED_FOR_CULLING, desired, -1.0);
            }
            crate::server::effect::send_effects_list(network.clone(), instance, id);
        }
    }
}

/// Teams are assigned after players have spawned, and the client only reads
/// battalions from spawn packets, so a change respawns the player everywhere.
fn sync_battalions(
    instance: &mut Instance,
    network: &Arc<Mutex<NetworkState>>,
    rules: &Match,
    inputs: &[Input],
) {
    for input in inputs {
        let id = ObjectId(input.id);
        let battalion = rules
            .players
            .get(&input.id)
            .map_or(0, |p| team_battalion(p.team));
        let Some(actor @ NetworkedActor::Player { .. }) = instance.find_actor_mut(id) else {
            continue;
        };
        let spawn = actor.get_common_spawn_mut();
        if spawn.battalion == battalion {
            continue;
        }
        spawn.battalion = battalion;
        tracing::info!(actor = input.id, battalion, "Feast team battalion changed");
        let mut network = network.lock();
        // The player's own connection respawns them for themselves.
        network.send_to_by_actor_id(
            id,
            FromServer::SetFeastBattalion(battalion),
            DestinationNetwork::ZoneClients,
        );
        // Despawn for everyone else; the view update spawns it again with the new battalion.
        for (handle, state) in network.clients.values_mut() {
            if handle.actor_id == id {
                continue;
            }
            if let Some(spawn_index) = state.actor_allocator.free(id) {
                let _ = handle.send(FromServer::DeleteActor(id, spawn_index));
            }
        }
    }
}

/// The client classifies party members as allies before it checks battalions,
/// so a party spanning both teams would hide its opponents. Such a party is
/// only the custom-match invite; disband it once teams are assigned.
fn split_parties(network: &Arc<Mutex<NetworkState>>, rules: &Match) {
    let mut network = network.lock();
    let mixed: Vec<_> = network
        .parties
        .iter()
        .filter(|(_, party)| {
            let mut teams = party
                .members
                .iter()
                .filter_map(|m| rules.players.get(&m.actor_id.0).map(|p| p.team));
            teams
                .next()
                .is_some_and(|first| teams.any(|team| team != first))
        })
        .map(|(id, party)| {
            let leader = party
                .get_member_by_actor_id(party.leader_id)
                .unwrap_or_default();
            (*id, leader)
        })
        .collect();
    for (party_id, leader) in mixed {
        tracing::info!(party_id, "Disbanding a party split across Feast teams");
        crate::server::party::disband_party(
            &mut network,
            party_id,
            PartyUpdateTargets {
                execute_account_id: leader.account_id,
                execute_content_id: leader.content_id,
                execute_name: leader.name,
                ..Default::default()
            },
        );
    }
}

/// Closes the jail gates during preparation and opens them for the match.
/// Also moves each player into their team's jail once per match.
///
/// The gates are layout shared groups, not spawned objects, so the server
/// spawns invisible EventObjs bound to them (`bind_layout_id`) and animates
/// those: timeline slot 1 closes, slot 3 opens. Each viewer gets the update
/// only after their client has spawned the controller.
fn sync_jails(
    instance: &mut Instance,
    network: &Arc<Mutex<NetworkState>>,
    arena: &mut Arena,
    inputs: &[Input],
) {
    let closed = arena.rules.phase == Phase::Countdown;
    let state = if closed {
        SharedGroupTimelineState::TIMELINE_1
    } else {
        SharedGroupTimelineState::TIMELINE_3
    };
    let controllers = jail_controllers(instance, state);
    arena.placed.retain(|id| inputs.iter().any(|i| i.id == *id));
    arena
        .gate_viewers
        .retain(|(id, _), _| inputs.iter().any(|i| i.id == *id));
    for input in inputs {
        let id = ObjectId(input.id);
        if !is_player(instance, id) {
            continue;
        }
        if arena.placed.insert(input.id)
            && let Some(player) = arena.rules.players.get(&input.id)
        {
            // Slot = how many teammates with a lower ID came before.
            let slot = arena
                .rules
                .players
                .iter()
                .filter(|(other, p)| **other < input.id && p.team == player.team)
                .count();
            let (position, rotation) = team_spawn(player.team, slot);
            if let Some(actor) = instance.find_actor_mut(id) {
                let spawn = actor.get_common_spawn_mut();
                spawn.position = position;
                spawn.rotation = rotation;
            }
            arena.rules.players.get_mut(&input.id).unwrap().position = position.0.to_array();
            super::correct_position(instance, network, id, position);
        }
        for controller in &controllers {
            let key = (input.id, controller.0);
            let mut network = network.lock();
            if !super::has_spawned_for(&network, id, *controller) {
                // Do not consume the update before SpawnObject has been sent.
                arena.gate_viewers.remove(&key);
                continue;
            }
            if arena.gate_viewers.get(&key) == Some(&closed) {
                continue;
            }
            network.send_to_by_actor_id(
                id,
                FromServer::ActorControl(
                    *controller,
                    ActorControlCategory::SetSharedGroupTimelineState {
                        state,
                        arg2: 0,
                        object_type: 0,
                        layout_id: 0,
                    },
                ),
                DestinationNetwork::ZoneClients,
            );
            arena.gate_viewers.insert(key, closed);
        }
    }
}

/// The four gate controller objects, spawning any that are missing.
/// Their spawn state (`args1`) is kept current for late joiners.
fn jail_controllers(instance: &mut Instance, state: SharedGroupTimelineState) -> Vec<ObjectId> {
    let mut controllers = Vec::new();
    for (index, layout) in JAIL_LAYOUTS.into_iter().enumerate() {
        let existing = instance.actors.iter().find_map(|(id, actor)| match actor {
            NetworkedActor::Object { object, .. } if object.bind_layout_id == layout => Some(*id),
            _ => None,
        });
        let id = existing.unwrap_or_else(|| {
            let id = Instance::generate_actor_id();
            let west = index % 2 == 0;
            instance.insert_object(
                id,
                SpawnObject {
                    kind: ObjectKind::EventObj,
                    entity_id: id,
                    base_id: JAIL_EOBJS[index % 2],
                    bind_layout_id: layout,
                    position: Position(glam::Vec3A::new(
                        if west { -58.0 } else { 58.0 },
                        0.25,
                        0.0,
                    )),
                    rotation: if west { 0.0 } else { std::f32::consts::PI },
                    radius: 1.0,
                    not_targetable: true,
                    args1: state.bits(),
                    ..Default::default()
                },
                "FeastJail".into(),
            );
            id
        });
        if let Some(NetworkedActor::Object { object, .. }) = instance.find_actor_mut(id) {
            object.args1 = state.bits();
        }
        controllers.push(id);
    }
    controllers
}

fn spawn_medal_pile(
    instance: &mut Instance,
    medals: u16,
    position: [f32; 3],
    state: SharedGroupTimelineState,
) -> ObjectId {
    let id = Instance::generate_actor_id();
    instance.insert_object(
        id,
        SpawnObject {
            kind: ObjectKind::EventObj,
            base_id: medal_prefab(medals),
            entity_id: id,
            position: Position(glam::Vec3A::from_array(position)),
            radius: 1.0,
            not_targetable: true,
            args1: state.bits(),
            ..Default::default()
        },
        String::new(),
    );
    id
}

/// Makes the medal pile objects match `rules.drops`: spawns new piles, plays
/// the pickup animation, removes collected or expired piles, and shows each
/// viewer whether they can collect each pile.
fn sync_medal_piles(
    instance: &mut Instance,
    network: &Arc<Mutex<NetworkState>>,
    arena: &mut Arena,
    now: f64,
    inputs: &[Input],
) {
    let expired: Vec<_> = arena
        .fading
        .iter()
        .filter(|(_, (_, until))| now >= *until)
        .map(|(id, (actor, _))| (*id, *actor))
        .collect();
    for (id, actor) in expired {
        network.lock().remove_actor(instance, actor);
        arena.fading.remove(&id);
    }
    for pickup in &arena.rules.pickups {
        if arena.fading.contains_key(&pickup.drop_id) {
            continue;
        }
        // A close-range KO can create and collect a pile in the same tick.
        // Still spawn its native pickup effect instead of losing the feedback.
        let actor = arena
            .medal_piles
            .remove(&pickup.drop_id)
            .unwrap_or_else(|| {
                spawn_medal_pile(
                    instance,
                    pickup.medals,
                    pickup.position,
                    SharedGroupTimelineState::TIMELINE_2,
                )
            });
        // Slot 2 is dropmedal_off_a: the animated removal, including the native
        // placednpc_PvPpoint_depop sound. Its 50-frame timeline needs time to finish.
        crate::server::actor::set_shared_group_timeline_state(
            instance,
            &mut network.lock(),
            actor,
            &[2],
        );
        arena.fading.insert(pickup.drop_id, (actor, now + 1.0));
    }
    let obsolete: Vec<_> = arena
        .medal_piles
        .iter()
        .filter(|(id, _)| !arena.rules.drops.iter().any(|d| d.id == **id))
        .map(|(id, object)| (*id, *object))
        .collect();
    for (id, object) in obsolete {
        network.lock().remove_actor(instance, object);
        arena.medal_piles.remove(&id);
    }
    for drop in &arena.rules.drops {
        if arena.medal_piles.contains_key(&drop.id) || arena.fading.contains_key(&drop.id) {
            continue;
        }
        let state = match drop.team {
            Team::Fang => SharedGroupTimelineState::TIMELINE_3,
            Team::Claw => SharedGroupTimelineState::TIMELINE_5,
        };
        let id = spawn_medal_pile(instance, drop.medals, drop.position, state);
        arena.medal_piles.insert(drop.id, id);
    }
    // The look is per viewer: teammates may have different capacity left.
    arena.medal_viewers.retain(|(viewer, actor), _| {
        inputs.iter().any(|i| i.id == *viewer)
            && arena.medal_piles.values().any(|id| id.0 == *actor)
    });
    for drop in &arena.rules.drops {
        let Some(&actor) = arena.medal_piles.get(&drop.id) else {
            continue;
        };
        for input in inputs {
            let viewer = ObjectId(input.id);
            let state = medal_state(drop.team, !arena.rules.can_collect(input, drop));
            let key = (input.id, actor.0);
            let mut network = network.lock();
            if !super::has_spawned_for(&network, viewer, actor) {
                arena.medal_viewers.remove(&key);
                continue;
            }
            if arena.medal_viewers.get(&key) == Some(&state) {
                continue;
            }
            network.send_to_by_actor_id(
                viewer,
                FromServer::ActorControl(
                    actor,
                    ActorControlCategory::SetSharedGroupTimelineState {
                        state,
                        arg2: 0,
                        object_type: 0,
                        layout_id: 0,
                    },
                ),
                DestinationNetwork::ZoneClients,
            );
            arena.medal_viewers.insert(key, state);
        }
    }
}

/// Starts a new match whose roster is everyone currently in the arena,
/// with practice dummies as Claws and players as Fangs.
pub fn start_practice_match(instance: &mut Instance, network: &Arc<Mutex<NetworkState>>) {
    end_match(instance, network);
    let mut rules = Match::new(0.0);
    for (id, actor) in &instance.actors {
        if !matches!(actor, NetworkedActor::Player { .. })
            && !instance.feast.practice.contains_key(id)
        {
            continue;
        }
        let s = actor.get_common_spawn();
        rules.players.insert(
            id.0,
            super::rules::Player {
                team: if instance.feast.practice.contains_key(id) {
                    Team::Claw
                } else {
                    Team::Fang
                },
                job: s.class_job,
                medals: super::rules::STARTING_MEDALS,
                position: s.position.0.to_array(),
                alive: s.health_points > 0,
                present: true,
            },
        );
    }
    instance.feast.arena = Some(Arena::new(rules));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::feast::{TERRITORY, commands::command};

    fn arena_with_players(ids: &[u32]) -> (Instance, Arc<Mutex<NetworkState>>) {
        let mut instance = Instance::default();
        instance.zone.id = TERRITORY;
        for &id in ids {
            instance.insert_empty_actor(ObjectId(id));
            let p = instance
                .find_actor_mut(ObjectId(id))
                .unwrap()
                .get_common_spawn_mut();
            p.class_job = 24;
            p.health_points = 10000;
            p.max_health_points = 10000;
        }
        (instance, Arc::new(Mutex::new(NetworkState::default())))
    }

    #[test]
    fn jail_packets_target_spawned_controllers_and_retry_after_loading() {
        let (mut instance, _) = arena_with_players(&[1]);
        let viewer = ObjectId(1);
        let (sender, mut receiver) = tokio::sync::mpsc::channel(64);
        let client = crate::ClientId::default();
        let mut net = NetworkState::default();
        net.clients.insert(
            client,
            (
                crate::ClientHandle {
                    id: client,
                    channel: sender,
                    actor_id: viewer,
                    account_id: 0,
                    content_id: 0,
                },
                crate::server::ClientState::default(),
            ),
        );
        let network = Arc::new(Mutex::new(net));
        tick(&mut instance, &network);
        // Gate updates must wait for the receiver's object spawn allocation.
        while let Ok(message) = receiver.try_recv() {
            assert!(!matches!(
                message,
                FromServer::ActorControl(
                    _,
                    ActorControlCategory::SetSharedGroupTimelineState { .. }
                )
            ));
        }
        let controllers: Vec<_> = instance
            .actors
            .iter()
            .filter_map(|(id, actor)| match actor {
                NetworkedActor::Object { object, .. }
                    if JAIL_LAYOUTS.contains(&object.bind_layout_id) =>
                {
                    Some(*id)
                }
                _ => None,
            })
            .collect();
        assert_eq!(controllers.len(), 4);
        for id in &controllers {
            network
                .lock()
                .clients
                .get_mut(&client)
                .unwrap()
                .1
                .object_allocator
                .reserve(*id);
        }
        for expected in [
            SharedGroupTimelineState::TIMELINE_1,
            SharedGroupTimelineState::TIMELINE_3,
        ] {
            if expected == SharedGroupTimelineState::TIMELINE_3 {
                instance.feast.arena.as_mut().unwrap().rules.deadline = 0.0;
            }
            tick(&mut instance, &network);
            let mut sent = BTreeSet::new();
            while let Ok(message) = receiver.try_recv() {
                if let FromServer::ActorControl(
                    actor,
                    ActorControlCategory::SetSharedGroupTimelineState {
                        state,
                        object_type,
                        layout_id,
                        ..
                    },
                ) = message
                {
                    assert!(controllers.contains(&actor));
                    assert_ne!(actor, viewer);
                    assert_eq!(
                        object_type, 0,
                        "use network EObj lookup, not StandObjectManager"
                    );
                    assert_eq!(layout_id, 0);
                    assert_eq!(state, expected);
                    sent.insert(actor.0);
                }
            }
            assert_eq!(sent.len(), 4);
        }
        tick(&mut instance, &network);
        assert!(
            receiver.try_recv().is_err(),
            "stable state must not replay animations"
        );
    }

    #[test]
    fn jails_place_teams_open_at_start_and_reset_for_next_match() {
        let (mut instance, network) = arena_with_players(&[1, 2]);
        for (n, base_id) in JAIL_EOBJS.into_iter().enumerate() {
            let id = ObjectId(100 + n as u32);
            instance.insert_object(
                id,
                SpawnObject {
                    kind: ObjectKind::EventObj,
                    entity_id: id,
                    base_id,
                    bind_layout_id: JAIL_LAYOUTS[n],
                    ..Default::default()
                },
                String::new(),
            );
        }
        tick(&mut instance, &network);
        let controllers = instance
            .actors
            .values()
            .filter(|actor| {
                matches!(actor, NetworkedActor::Object { object, .. }
                    if JAIL_LAYOUTS.contains(&object.bind_layout_id))
            })
            .count();
        assert_eq!(
            controllers, 4,
            "two collision objects plus two VFX controllers"
        );
        let actor_count = instance.actors.len();
        tick(&mut instance, &network);
        assert_eq!(
            instance.actors.len(),
            actor_count,
            "controllers must not duplicate each tick"
        );
        let x = |instance: &Instance, id: u32| {
            instance
                .find_actor(ObjectId(id))
                .unwrap()
                .get_common_spawn()
                .position
                .0
                .x
        };
        for id in [1, 2] {
            let team = instance.feast.arena.as_ref().unwrap().rules.players[&id].team;
            assert_eq!(x(&instance, id), team_spawn(team, 0).0.0.x);
        }
        let gate = |instance: &Instance| match instance.find_actor(ObjectId(100)).unwrap() {
            NetworkedActor::Object { object, .. } => object.args1,
            _ => unreachable!(),
        };
        assert_eq!(gate(&instance), SharedGroupTimelineState::TIMELINE_1.bits());
        instance.feast.arena.as_mut().unwrap().rules.deadline = 0.0;
        tick(&mut instance, &network);
        assert_eq!(
            instance.feast.arena.as_ref().unwrap().rules.phase,
            Phase::Active
        );
        assert_eq!(gate(&instance), SharedGroupTimelineState::TIMELINE_3.bits());
        // Normal movement after the opening must not be snapped back into jail.
        instance
            .find_actor_mut(ObjectId(1))
            .unwrap()
            .get_common_spawn_mut()
            .position = Position(glam::Vec3A::ZERO);
        tick(&mut instance, &network);
        assert_eq!(x(&instance, 1), 0.0);
        instance.feast.arena.as_mut().unwrap().rules.phase = Phase::Finished(None);
        command(&mut instance, &network, ObjectId(1), "restart");
        assert_eq!(gate(&instance), SharedGroupTimelineState::TIMELINE_1.bits());
        assert_eq!(x(&instance, 1).abs(), 58.0);
    }

    #[test]
    fn pickup_animates_before_cleanup_and_restores_partial_pile() {
        let (mut instance, network) = arena_with_players(&[1, 2]);
        tick(&mut instance, &network);
        instance.feast.arena.as_mut().unwrap().rules.deadline = 0.0;
        tick(&mut instance, &network);
        for id in [ObjectId(1), ObjectId(2)] {
            instance
                .find_actor_mut(id)
                .unwrap()
                .get_common_spawn_mut()
                .position = Position(glam::Vec3A::ZERO);
        }
        instance
            .find_actor_mut(ObjectId(1))
            .unwrap()
            .get_common_spawn_mut()
            .health_points = 0;
        tick(&mut instance, &network); // KO and partial pickup happen in the same tick.
        let arena = instance.feast.arena.as_ref().unwrap();
        assert_eq!(arena.rules.players[&2].medals, 125);
        assert_eq!(arena.rules.drops[0].medals, 25);
        assert_eq!(arena.rules.pickups.len(), 1);
        let drop_id = arena.rules.drops[0].id;
        let fading_actor = arena.fading[&drop_id].0;
        let NetworkedActor::Object { object, .. } = instance.find_actor(fading_actor).unwrap()
        else {
            panic!()
        };
        assert_eq!(object.args1, SharedGroupTimelineState::TIMELINE_2.bits());
        tick(&mut instance, &network);
        assert!(
            instance.find_actor(fading_actor).is_some(),
            "retain the actor while its pickup animation/sound plays"
        );
        assert!(
            instance
                .feast
                .arena
                .as_ref()
                .unwrap()
                .rules
                .pickups
                .is_empty()
        );
        instance
            .feast
            .arena
            .as_mut()
            .unwrap()
            .fading
            .get_mut(&drop_id)
            .unwrap()
            .1 = 0.0;
        tick(&mut instance, &network);
        assert!(instance.find_actor(fading_actor).is_none());
        let arena = instance.feast.arena.as_ref().unwrap();
        assert!(
            arena.medal_piles.contains_key(&drop_id),
            "remaining medals must reappear"
        );
        assert_eq!(arena.rules.drops[0].medals, 25);
        instance
            .feast
            .arena
            .as_mut()
            .unwrap()
            .rules
            .players
            .get_mut(&2)
            .unwrap()
            .medals = 100;
        tick(&mut instance, &network); // Collect the remainder.
        assert!(
            instance
                .feast
                .arena
                .as_ref()
                .unwrap()
                .rules
                .drops
                .is_empty()
        );
        instance
            .feast
            .arena
            .as_mut()
            .unwrap()
            .fading
            .get_mut(&drop_id)
            .unwrap()
            .1 = 0.0;
        tick(&mut instance, &network);
        let arena = instance.feast.arena.as_ref().unwrap();
        assert!(arena.medal_piles.is_empty() && arena.fading.is_empty());
    }

    #[test]
    fn respawn_lock_expires_at_ten_seconds_without_restarting() {
        let mut instance = Instance::default();
        let id = ObjectId(1);
        let start = Instant::now();
        begin_respawn_lock(&mut instance, id, start);
        assert_eq!(respawn_remaining(&instance, id, start), 10000);
        begin_respawn_lock(&mut instance, id, start + Duration::from_secs(3));
        assert_eq!(
            respawn_remaining(&instance, id, start + Duration::from_millis(9999)),
            1
        );
        assert_eq!(
            respawn_remaining(&instance, id, start + Duration::from_secs(10)),
            0
        );
        instance.feast.respawns.remove(&id);
        begin_respawn_lock(&mut instance, id, start + Duration::from_secs(20));
        assert_eq!(
            respawn_remaining(&instance, id, start + Duration::from_secs(20)),
            10000
        );
    }

    #[test]
    fn medal_piles_grow_with_size_and_dim_when_blocked() {
        for (count, prefab) in [
            (1, 2007627),
            (25, 2007627),
            (26, 2007628),
            (50, 2007628),
            (51, 2007629),
            (75, 2007629),
            (76, 2008631),
            (100, 2008631),
            (200, 2008631),
        ] {
            assert_eq!(medal_prefab(count), prefab);
        }
        assert_eq!(medal_state(Team::Claw, false).bits(), 1 << 4);
        assert_eq!(medal_state(Team::Claw, true).bits(), 1 << 10);
        assert_eq!(medal_state(Team::Fang, true).bits(), 1 << 8);
    }
}
