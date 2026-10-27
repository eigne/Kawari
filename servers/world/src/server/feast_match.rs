//! Small-team Feast match rules. Time and positions are supplied by the world server.
use std::collections::{BTreeMap, BTreeSet};

pub const COUNTDOWN: f64 = 60.0;
pub const MATCH_TIME: f64 = 360.0;
pub const SUDDEN_DEATH: f64 = 120.0;
// The official guide does not specify these two values. Prototype tuning.
const DROP_LIFETIME: f64 = 60.0;
const PICKUP_RADIUS: f32 = 2.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Team {
    Fang,
    Claw,
}
impl Team {
    pub fn name(self) -> &'static str {
        match self {
            Self::Fang => "fang",
            Self::Claw => "claw",
        }
    }
}

pub fn medal_limit(job: u8) -> Option<u16> {
    match job {
        19 | 21 | 32 | 37 => Some(200),
        20 | 22 | 30 | 34 | 39 | 41 => Some(175),
        23 | 25 | 27 | 31 | 35 | 38 | 42 => Some(150),
        24 | 28 | 33 | 40 => Some(125),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    Countdown,
    Active,
    SuddenDeath,
    Finished(Option<Team>),
}

#[derive(Clone, Debug)]
pub struct Player {
    pub team: Team,
    pub job: u8,
    pub medals: u16,
    pub position: [f32; 3],
    pub alive: bool,
    pub present: bool,
}

#[derive(Clone, Debug)]
pub struct Input {
    pub id: u32,
    pub job: u8,
    pub position: [f32; 3],
    pub alive: bool,
}

#[derive(Clone, Debug)]
pub struct Drop {
    pub id: u32,
    pub owner: u32,
    pub team: Team,
    pub medals: u16,
    pub position: [f32; 3],
    pub expires: f64,
    pub abandoned: bool,
}

#[derive(Clone, Debug)]
pub struct PickupVisual {
    pub drop_id: u32,
    pub medals: u16,
    pub position: [f32; 3],
}

#[derive(Debug)]
pub struct Match {
    pub phase: Phase,
    pub deadline: f64,
    pub players: BTreeMap<u32, Player>,
    pub drops: Vec<Drop>,
    next_drop: u32,
    last_clock: u32,
    pub announcements: Vec<String>,
    pub pickups: Vec<PickupVisual>,
    last_knockout: f64,
}

impl Match {
    pub fn new(now: f64) -> Self {
        Self {
            phase: Phase::Countdown,
            deadline: now + COUNTDOWN,
            players: BTreeMap::new(),
            drops: Vec::new(),
            next_drop: 1,
            last_clock: u32::MAX,
            announcements: Vec::new(),
            pickups: Vec::new(),
            last_knockout: now + COUNTDOWN,
        }
    }

    pub fn scores(&self) -> (u16, u16) {
        self.players
            .values()
            .fold((0, 0), |(f, c), p| match p.team {
                Team::Fang => (f + p.medals, c),
                Team::Claw => (f, c + p.medals),
            })
    }

    /// Final Feast: 90 seconds without a KO, then one stack every 15 seconds.
    /// Sudden death starts at one immediately and ignores further KOs.
    pub fn culling_stacks(&self, now: f64) -> u8 {
        let first = match self.phase {
            Phase::Active => self.last_knockout + 90.0,
            Phase::SuddenDeath => self.deadline - SUDDEN_DEATH,
            _ => return 0,
        };
        if now < first { 0 } else { (1.0 + ((now - first) / 15.0).floor()).min(5.0) as u8 }
    }

    pub fn active(&self, id: u32, job: u8) -> bool {
        matches!(self.phase, Phase::Active | Phase::SuddenDeath)
            && self
                .players
                .get(&id)
                .is_some_and(|p| p.present && p.job == job)
    }

    pub(super) fn pickup_space(&self, id: u32) -> u16 {
        let Some(p) = self.players.get(&id) else {
            return 0;
        };
        let reserved: u16 = self
            .drops
            .iter()
            .filter(|d| d.owner == id)
            .map(|d| d.medals)
            .sum();
        medal_limit(p.job)
            .unwrap_or(0)
            .saturating_sub(p.medals + reserved)
    }

    fn can_collect(&self, input: &Input, drop: &Drop) -> bool {
        self.active(input.id, input.job)
            && self
                .players
                .get(&input.id)
                .is_some_and(|p| p.alive && p.team != drop.team)
            && self.pickup_space(input.id) > 0
    }

    pub fn status(&self, now: f64) -> String {
        let (f, c) = self.scores();
        let state = match self.phase {
            Phase::Countdown => "Preparation",
            Phase::Active => "Match",
            Phase::SuddenDeath => "Sudden death",
            Phase::Finished(Some(Team::Fang)) => "Fangs win",
            Phase::Finished(Some(Team::Claw)) => "Claws win",
            Phase::Finished(None) => "Draw",
        };
        let seconds = if matches!(self.phase, Phase::Finished(_)) {
            0
        } else {
            (self.deadline - now).max(0.0).ceil() as u32
        };
        format!(
            "Feast: {state} {:02}:{:02} | Fangs {f} - Claws {c}",
            seconds / 60,
            seconds % 60
        )
    }

    /// Versioned display snapshot. Values always come from authoritative match state.
    pub fn hud_snapshot(&self, now: f64, actor: u32) -> String {
        let (fang, claw) = self.scores();
        let phase = match self.phase {
            Phase::Countdown => "prep",
            Phase::Active => "active",
            Phase::SuddenDeath => "sudden",
            Phase::Finished(Some(Team::Fang)) => "fang",
            Phase::Finished(Some(Team::Claw)) => "claw",
            Phase::Finished(None) => "draw",
        };
        let seconds = if matches!(self.phase, Phase::Finished(_)) {
            0
        } else {
            (self.deadline - now).max(0.0).ceil() as u32
        };
        let (team, medals, cap) = self
            .players
            .get(&actor)
            .map(|p| (p.team.name(), p.medals, medal_limit(p.job).unwrap_or(0)))
            .unwrap_or(("spectator", 0, 0));
        let ground: u32 = self.drops.iter().map(|d| u32::from(d.medals)).sum();
        format!("KFEASTHUD1|{phase}|{seconds}|{fang}|{claw}|{team}|{medals}|{cap}|{ground}")
    }

    fn finish(&mut self, winner: Option<Team>, now: f64) {
        self.phase = Phase::Finished(winner);
        self.announcements.push(self.status(now));
        self.drops.clear();
    }

    fn drop_medals(&mut self, id: u32, all: bool, now: f64) {
        let p = self.players.get_mut(&id).unwrap();
        let count = if all { p.medals } else { p.medals.div_ceil(2) };
        if count == 0 {
            return;
        }
        p.medals -= count;
        self.drops.push(Drop {
            id: self.next_drop,
            owner: id,
            team: p.team,
            medals: count,
            position: p.position,
            expires: now + DROP_LIFETIME,
            abandoned: all,
        });
        self.next_drop += 1;
        self.announcements.push(format!(
            "{} player {id} dropped {count} medals.",
            p.team.name()
        ));
    }

    pub fn step(&mut self, now: f64, inputs: &[Input]) {
        self.pickups.clear();
        if matches!(self.phase, Phase::Finished(_)) {
            return;
        }
        // Close the scoring window before processing movement/deaths from this tick.
        if self.phase == Phase::Active && now >= self.deadline {
            let (f, c) = self.scores();
            if f != c {
                self.finish(Some(if f > c { Team::Fang } else { Team::Claw }), now);
                return;
            }
            self.phase = Phase::SuddenDeath;
            self.deadline += SUDDEN_DEATH;
            self.announcements
                .push("Sudden death: the next medal pickup wins!".into());
        }
        if self.phase == Phase::SuddenDeath && now >= self.deadline {
            self.finish(None, now);
            return;
        }
        let inputs: BTreeMap<_, _> = inputs.iter().map(|p| (p.id, p)).collect();
        if self.phase == Phase::Countdown {
            // Entry order is deterministic; at most four participants on each side.
            for (&id, input) in &inputs {
                if self.players.contains_key(&id) || medal_limit(input.job).is_none() {
                    continue;
                }
                let f = self
                    .players
                    .values()
                    .filter(|p| p.team == Team::Fang)
                    .count();
                let c = self.players.len() - f;
                if f + c == 8 {
                    continue;
                }
                let team = if f <= c { Team::Fang } else { Team::Claw };
                self.players.insert(
                    id,
                    Player {
                        team,
                        job: input.job,
                        medals: 100,
                        position: input.position,
                        alive: input.alive,
                        present: true,
                    },
                );
                self.announcements.push(format!(
                    "Player {id} joined {} (100 medals, cap {}).",
                    team.name(),
                    medal_limit(input.job).unwrap()
                ));
            }
            self.players.retain(|id, _| inputs.contains_key(id));
            if now >= self.deadline {
                self.phase = Phase::Active;
                self.deadline += MATCH_TIME;
                self.announcements
                    .push("Feast begins! Six minutes remaining.".into());
            }
        }
        let ids: Vec<_> = self.players.keys().copied().collect();
        for id in ids {
            let input = inputs.get(&id);
            let p = self.players.get_mut(&id).unwrap();
            let was_present = p.present;
            let was_alive = p.alive;
            p.present = input.is_some();
            if let Some(input) = input {
                p.position = input.position;
                p.alive = input.alive;
                if self.phase == Phase::Countdown && medal_limit(input.job).is_some() {
                    p.job = input.job;
                }
            }
            if self.phase == Phase::Countdown {
                continue;
            }
            if was_present && !p.present {
                self.drop_medals(id, true, now);
            } else if was_present && was_alive && !p.alive {
                self.last_knockout = now;
                self.drop_medals(id, false, now);
            }
            // Rejoining restores only unclaimed disconnect drops, not ordinary KO drops.
            if !was_present && input.is_some() {
                self.drops.retain(|drop| {
                    if drop.owner == id && drop.abandoned {
                        self.players.get_mut(&id).unwrap().medals += drop.medals;
                        false
                    } else {
                        true
                    }
                });
            }
        }
        if self.phase == Phase::Countdown {
            self.clock(now);
            return;
        }
        self.drops.retain(|drop| {
            if !drop.abandoned && now >= drop.expires {
                self.players.get_mut(&drop.owner).unwrap().medals += drop.medals;
                false
            } else {
                true
            }
        });
        let mut winner = None;
        for index in 0..self.drops.len() {
            let ids: Vec<_> = self.players.keys().copied().collect();
            for id in ids {
                let p = &self.players[&id];
                let drop = &self.drops[index];
                if drop.medals == 0
                    || !p.present
                    || !p.alive
                    || p.team == drop.team
                    || inputs.get(&id).is_none_or(|input| input.job != p.job)
                {
                    continue;
                }
                let distance: f32 = p
                    .position
                    .iter()
                    .zip(drop.position)
                    .map(|(a, b)| (a - b).powi(2))
                    .sum();
                if !distance.is_finite() || distance > PICKUP_RADIUS * PICKUP_RADIUS {
                    continue;
                }
                let space = self.pickup_space(id);
                let amount = space.min(drop.medals);
                if amount == 0 {
                    continue;
                }
                self.pickups.push(PickupVisual {
                    drop_id: drop.id,
                    medals: drop.medals,
                    position: drop.position,
                });
                self.drops[index].medals -= amount;
                let p = self.players.get_mut(&id).unwrap();
                p.medals += amount;
                self.announcements.push(format!(
                    "{} player {id} picked up {amount} medals (holding {}).",
                    p.team.name(),
                    p.medals
                ));
                if self.phase == Phase::SuddenDeath {
                    winner = Some(p.team);
                    break;
                }
            }
            if winner.is_some() {
                break;
            }
        }
        self.drops.retain(|d| d.medals > 0);
        if let Some(team) = winner {
            self.finish(Some(team), now);
            return;
        }
        let (fang, claw) = self.scores();
        if fang >= 600 || claw >= 600 {
            self.finish(Some(if fang >= 600 { Team::Fang } else { Team::Claw }), now);
            return;
        }
        self.clock(now);
    }

    fn clock(&mut self, now: f64) {
        let remaining = (self.deadline - now).max(0.0).ceil() as u32;
        if self.last_clock != remaining && (remaining <= 10 || remaining.is_multiple_of(30)) {
            self.announcements.push(self.status(now));
        }
        self.last_clock = remaining;
    }
}

use super::{
    actor::NetworkedActor,
    instance::Instance,
    network::{DestinationNetwork, NetworkState},
};
use crate::FromServer;
use kawari::{
    common::{DirectorEvent, ObjectId, Position, SharedGroupTimelineState},
    ipc::zone::{ActorControlCategory, ObjectKind, SpawnObject},
};
use parking_lot::Mutex;
use std::{sync::Arc, time::Instant};

#[derive(Debug)]
pub struct Runtime {
    started: Instant,
    pub rules: Match,
    objects: BTreeMap<u32, ObjectId>,
    fading: BTreeMap<u32, (ObjectId, f64)>,
    empty_since: Option<f64>,
    placed: BTreeSet<u32>,
    gate_viewers: BTreeMap<(u32, u32), bool>,
    medal_viewers: BTreeMap<(u32, u32), SharedGroupTimelineState>,
    next_director_snapshot: f64,
    pub supplies: super::feast_supplies::Supplies,
}

// The start_col layer centers the two jails at x = +/-58, y = 0.25.
// Collision and VFX are separate shared groups in the restored layout.
const JAIL_LAYOUTS: [u32; 4] = [7403947, 7403946, 7515404, 7515405];
const JAIL_EOBJS: [u32; 2] = [2008986, 2008987];

fn medal_prefab(medals: u16) -> u32 {
    match medals {
        0..=25 => 2007627,
        26..=50 => 2007628,
        51..=75 => 2007629,
        _ => 2008631,
    }
}

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

pub fn team_spawn(team: Team, slot: usize) -> (Position, f32) {
    let (x, rotation) = match team {
        Team::Fang => (-58.0, std::f32::consts::FRAC_PI_2),
        Team::Claw => (58.0, -std::f32::consts::FRAC_PI_2),
    };
    (
        Position(glam::Vec3::new(x, 0.5, (slot % 4) as f32 * 2.0 - 3.0)),
        rotation,
    )
}

pub fn on_zone_loaded(instance: &mut Instance, actor: ObjectId) {
    if let Some(runtime) = &mut instance.feast_match {
        runtime.placed.remove(&actor.0);
        runtime
            .medal_viewers
            .retain(|(viewer, _), _| *viewer != actor.0);
        runtime
            .gate_viewers
            .retain(|(viewer, _), _| *viewer != actor.0);
    }
}

pub fn begin_respawn_lock(instance: &mut Instance, actor: ObjectId, now: Instant) {
    instance
        .feast_respawns
        .entry(actor)
        .or_insert(now + std::time::Duration::from_secs(10));
}

pub fn respawn_remaining(instance: &Instance, actor: ObjectId, now: Instant) -> u32 {
    instance
        .feast_respawns
        .get(&actor)
        .map(|until| {
            until
                .saturating_duration_since(now)
                .as_millis()
                .min(u32::MAX as u128) as u32
        })
        .unwrap_or(0)
}

pub fn respawn(instance: &Instance, actor: ObjectId) -> (Position, f32) {
    let team = instance
        .feast_match
        .as_ref()
        .and_then(|m| m.rules.players.get(&actor.0))
        .map(|p| p.team)
        .unwrap_or(Team::Fang);
    team_spawn(team, 1)
}

fn sync_jails(
    instance: &mut Instance,
    network: &Arc<Mutex<NetworkState>>,
    runtime: &mut Runtime,
    inputs: &[Input],
) {
    let closed = runtime.rules.phase == Phase::Countdown;
    // Both prefab descriptor tables put ON in slot 1 and OFF in slot 3.
    // These are bit masks, not raw animation sub-IDs.
    let state = if closed {
        SharedGroupTimelineState::TIMELINE_1
    } else {
        SharedGroupTimelineState::TIMELINE_3
    };
    // Layout SGs are not StandObjectManager actors. Bind network EventObjects
    // to all four SGs, including the VFX groups which had no original EObj.
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
                    position: Position(glam::Vec3::new(if west { -58.0 } else { 58.0 }, 0.25, 0.0)),
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
    runtime
        .placed
        .retain(|id| inputs.iter().any(|i| i.id == *id));
    runtime
        .gate_viewers
        .retain(|(id, _), _| inputs.iter().any(|i| i.id == *id));
    for input in inputs {
        let id = ObjectId(input.id);
        if !matches!(instance.find_actor(id), Some(NetworkedActor::Player { .. })) {
            continue;
        }
        if runtime.placed.insert(input.id) {
            if let Some(player) = runtime.rules.players.get(&input.id) {
                let slot = runtime
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
                runtime.rules.players.get_mut(&input.id).unwrap().position = position.0.to_array();
                super::feast::correct_position(instance, network, id, position);
            }
        }
        for controller in &controllers {
            let key = (input.id, controller.0);
            let mut network = network.lock();
            let spawned = network
                .clients
                .values()
                .any(|(handle, state)| handle.actor_id == id && state.has_spawned(*controller));
            if !spawned {
                // Do not consume the update before SpawnObject has been sent.
                runtime.gate_viewers.remove(&key);
                continue;
            }
            if runtime.gate_viewers.get(&key) == Some(&closed) {
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
            runtime.gate_viewers.insert(key, closed);
            tracing::info!(
                viewer = input.id,
                controller = controller.0,
                closed,
                "Feast jail state sent to spawned event object"
            );
        }
    }
}

pub fn can_act(instance: &Instance, actor: ObjectId) -> bool {
    instance.feast_match.as_ref().is_none_or(|m| {
        instance
            .find_actor(actor)
            .is_some_and(|a| m.rules.active(actor.0, a.get_common_spawn().class_job))
    })
}

pub fn tick(instance: &mut Instance, network: &Arc<Mutex<NetworkState>>) {
    if instance.zone.id != 745 {
        return;
    }
    let inputs: Vec<_> = instance
        .actors
        .iter()
        .filter_map(|(id, actor)| {
            if !matches!(actor, NetworkedActor::Player { .. })
                && !instance.feast_dummies.contains_key(id)
            {
                return None;
            }
            let s = actor.get_common_spawn();
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
        .collect();
    if !instance
        .actors
        .values()
        .any(|actor| matches!(actor, NetworkedActor::Player { .. }))
    {
        if let Some(mut m) = instance.feast_match.take() {
            let now = m.started.elapsed().as_secs_f64();
            let empty_since = *m.empty_since.get_or_insert(now);
            m.rules.step(now, &[]);
            if now - empty_since >= 30.0 {
                m.supplies.clear(instance, network);
                for object in m
                    .objects
                    .values()
                    .chain(m.fading.values().map(|(id, _)| id))
                {
                    network.lock().remove_actor(instance, *object);
                }
                instance.feast_whm = None;
                let dummies: Vec<_> = instance.feast_dummies.keys().copied().collect();
                for dummy in dummies {
                    network.lock().remove_actor(instance, dummy);
                }
                instance.feast_dummies.clear();
            } else {
                instance.feast_match = Some(m);
            }
        }
        return;
    }
    let mut runtime = instance.feast_match.take().unwrap_or_else(|| Runtime {
        started: Instant::now(),
        rules: Match::new(0.0),
        objects: BTreeMap::new(),
        fading: BTreeMap::new(),
        empty_since: None,
        placed: BTreeSet::new(),
        gate_viewers: BTreeMap::new(),
        medal_viewers: BTreeMap::new(),
        next_director_snapshot: 0.0,
        supplies: Default::default(),
    });
    instance
        .feast_respawns
        .retain(|id, _| inputs.iter().any(|i| i.id == id.0 && !i.alive));
    for input in &inputs {
        if !input.alive
            && matches!(
                instance.find_actor(ObjectId(input.id)),
                Some(NetworkedActor::Player { .. })
            )
        {
            begin_respawn_lock(instance, ObjectId(input.id), Instant::now());
        }
    }
    let now = runtime.started.elapsed().as_secs_f64();
    runtime.empty_since = None;
    let previous_phase = runtime.rules.phase;
    runtime.rules.step(now, &inputs);
    if previous_phase != runtime.rules.phase
        && matches!(runtime.rules.phase, Phase::Active | Phase::SuddenDeath)
        && let Some(director) = &instance.director
    {
        // Keep native duty warnings on the authoritative match clock, including
        // the sudden-death extension, rather than the earlier entry timestamp.
        let remaining = (runtime.rules.deadline - now).ceil().max(0.0) as u32;
        for input in &inputs {
            let id = ObjectId(input.id);
            if matches!(instance.find_actor(id), Some(NetworkedActor::Player { .. })) {
                network.lock().send_to_by_actor_id(
                    id,
                    FromServer::ActorControlSelf(ActorControlCategory::DirectorEvent {
                        handler_id: director.id,
                        event: DirectorEvent::SetDutyTimeRemaining {
                            arg1: remaining, arg2: 0, arg3: 0, arg4: 0,
                        },
                    }),
                    DestinationNetwork::ZoneClients,
                );
            }
        }
    }
    // Reuse the surviving Marked for Culling status and native stack icon.
    let stacks = runtime.rules.culling_stacks(now);
    for input in &inputs {
        let id = ObjectId(input.id);
        let desired = if input.alive { u16::from(stacks) } else { 0 };
        if let Some(effects) = instance.find_actor_mut(id).and_then(|a| a.status_effects_mut()) {
            if effects.get(994).map_or(0, |s| s.param) != desired {
                effects.remove(994);
                if desired > 0 { effects.add(994, desired, -1.0); }
                super::effect::send_effects_list(network.clone(), instance, id);
            }
        }
    }
    super::feast_supplies::tick(instance, network, &runtime.rules, &mut runtime.supplies, now);
    sync_jails(instance, network, &mut runtime, &inputs);
    let expired: Vec<_> = runtime
        .fading
        .iter()
        .filter(|(_, (_, until))| now >= *until)
        .map(|(id, (actor, _))| (*id, *actor))
        .collect();
    for (id, actor) in expired {
        network.lock().remove_actor(instance, actor);
        runtime.fading.remove(&id);
    }
    for pickup in &runtime.rules.pickups {
        if runtime.fading.contains_key(&pickup.drop_id) {
            continue;
        }
        let actor = runtime.objects.remove(&pickup.drop_id).unwrap_or_else(|| {
            // A close-range KO can create and collect a pile in the same tick.
            // Still spawn its native pickup effect instead of losing the feedback.
            let id = Instance::generate_actor_id();
            instance.insert_object(
                id,
                SpawnObject {
                    kind: ObjectKind::EventObj,
                    base_id: medal_prefab(pickup.medals),
                    entity_id: id,
                    position: Position(glam::Vec3::from_array(pickup.position)),
                    radius: 1.0,
                    not_targetable: true,
                    args1: SharedGroupTimelineState::TIMELINE_2.bits(),
                    ..Default::default()
                },
                String::new(),
            );
            id
        });
        // Slot 2 is dropmedal_off_a: the animated removal, including the native
        // placednpc_PvPpoint_depop sound. Its 50-frame timeline needs time to finish.
        super::set_shared_group_timeline_state(instance, &mut network.lock(), actor, &[2]);
        runtime.fading.insert(pickup.drop_id, (actor, now + 1.0));
    }
    let obsolete: Vec<_> = runtime
        .objects
        .iter()
        .filter(|(id, _)| !runtime.rules.drops.iter().any(|d| d.id == **id))
        .map(|(id, object)| (*id, *object))
        .collect();
    for (id, object) in obsolete {
        network.lock().remove_actor(instance, object);
        runtime.objects.remove(&id);
    }
    for drop in &runtime.rules.drops {
        if runtime.objects.contains_key(&drop.id) || runtime.fading.contains_key(&drop.id) {
            continue;
        }
        let id = Instance::generate_actor_id();
        // Native Feast medal prefab: pvp/002, with dropmedal_f1/c1 timelines.
        // The old 2002056 was a quest's lion medal with generic event-item VFX.
        let object = SpawnObject {
            kind: ObjectKind::EventObj,
            base_id: medal_prefab(drop.medals),
            args1: match drop.team {
                Team::Fang => SharedGroupTimelineState::TIMELINE_3.bits(),
                Team::Claw => SharedGroupTimelineState::TIMELINE_5.bits(),
            },
            entity_id: id,
            position: Position(glam::Vec3::from_array(drop.position)),
            radius: 1.0,
            not_targetable: true,
            ..Default::default()
        };
        instance.insert_object(id, object, String::new());
        runtime.objects.insert(drop.id, id);
    }
    // Timeline state is recipient-specific: teammates may have different capacity.
    runtime.medal_viewers.retain(|(viewer, actor), _| {
        inputs.iter().any(|i| i.id == *viewer) && runtime.objects.values().any(|id| id.0 == *actor)
    });
    for drop in &runtime.rules.drops {
        let Some(&actor) = runtime.objects.get(&drop.id) else {
            continue;
        };
        for input in &inputs {
            let viewer = ObjectId(input.id);
            let state = medal_state(drop.team, !runtime.rules.can_collect(input, drop));
            let key = (input.id, actor.0);
            let mut network = network.lock();
            if !network
                .clients
                .values()
                .any(|(h, s)| h.actor_id == viewer && s.has_spawned(actor))
            {
                runtime.medal_viewers.remove(&key);
                continue;
            }
            if runtime.medal_viewers.get(&key) == Some(&state) {
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
            runtime.medal_viewers.insert(key, state);
        }
    }
    for message in runtime.rules.announcements.drain(..) {
        tracing::info!(%message, "Feast match");
        for input in &inputs {
            super::feast::notice(network, ObjectId(input.id), message.clone());
        }
    }
    if now >= runtime.next_director_snapshot {
        super::feast_director::send(instance, &runtime.rules, now, &mut network.lock());
        runtime.next_director_snapshot = now + 0.25;
    }
    instance.feast_match = Some(runtime);
}

pub fn command(
    instance: &mut Instance,
    network: &Arc<Mutex<NetworkState>>,
    id: ObjectId,
    text: &str,
) {
    if instance.zone.id != 745 {
        super::feast::notice(
            network,
            id,
            "Enter the Feast arena first with !feast.".into(),
        );
        return;
    }
    tick(instance, network);
    let words: Vec<_> = text.split_whitespace().collect();
    if words.first() == Some(&"ground") {
        super::feast::ground_action(instance, network, id, &words);
        return;
    }
    if matches!(words.first(), Some(&"actions") | Some(&"loadout") | Some(&"job")) {
        super::feast::tick(instance, network);
        let command = match words[0] { "actions" => "list", "job" => "status", _ => text };
        super::feast::command(instance, network, id, command);
        return;
    }
    if words.first() == Some(&"director") {
        let enabled = match words.as_slice() {
            ["director", "on"] => true,
            ["director", "off"] => false,
            _ => { super::feast::notice(network, id, "Use !feast director on|off (compatibility test).".into()); return; }
        };
        if enabled {
            super::feast::notice(network, id, "Director testing is disabled while its loading issue is investigated.".into());
            return;
        }
        network.lock().send_to_by_actor_id(id, FromServer::FeastDirectorProbe(enabled), DestinationNetwork::ZoneClients);
        super::feast::notice(network, id, format!("Legacy director probe {}. Existing Feast rules continue; leave the arena or use !feast director off to end the probe.", if enabled { "requested" } else { "disabled" }));
        return;
    }
    if matches!(words.first(), Some(&"dummies") | Some(&"attack") | Some(&"bots")) {
        super::feast_training::command(instance, network, id, &words);
        return;
    }
    if words.len() != 1 {
        super::feast::notice(
            network,
            id,
            "Use !feast status, !feast restart, !feast dummies [1-3], or !feast attack on|off."
                .into(),
        );
        return;
    }
    if text == "hud" {
        super::feast::notice(
            network,
            id,
            format!(
                "KFEASTRESPAWN1|{}",
                respawn_remaining(instance, id, Instant::now())
            ),
        );
        if let Some(m) = &instance.feast_match {
            super::feast::notice(
                network,
                id,
                m.rules
                    .hud_snapshot(m.started.elapsed().as_secs_f64(), id.0),
            );
        }
        return;
    }
    if text == "restart" {
        if instance
            .feast_match
            .as_ref()
            .is_some_and(|m| matches!(m.rules.phase, Phase::Finished(_)))
        {
            if !instance.feast_dummies.is_empty() {
                if instance.feast_dummies.values().any(|d| d.autonomous) {
                    super::feast_training::command(instance, network, id, &["bots"]);
                    return;
                }
                let count = instance.feast_dummies.len().to_string();
                super::feast_training::command(instance, network, id, &["dummies", &count]);
                return;
            }
            if let Some(mut old) = instance.feast_match.take() {
        old.supplies.clear(instance, network);
                for object in old
                    .objects
                    .values()
                    .chain(old.fading.values().map(|(id, _)| id))
                {
                    network.lock().remove_actor(instance, *object);
                }
            }
            instance.feast_whm = None;
            tick(instance, network);
        } else {
            super::feast::notice(
                network,
                id,
                "A match can only restart after its result.".into(),
            );
        }
        return;
    }
    if let Some(m) = &instance.feast_match {
        super::feast::notice(
            network,
            id,
            m.rules.status(m.started.elapsed().as_secs_f64()),
        );
        if let Some(p) = m.rules.players.get(&id.0) {
            super::feast::notice(
                network,
                id,
                format!(
                    "Team {}: {} medals / {} cap. Walk over enemy drops to collect them.",
                    p.team.name(),
                    p.medals,
                    medal_limit(p.job).unwrap()
                ),
            );
        } else {
            super::feast::notice(
                network,
                id,
                "This match's roster is locked. Join the next countdown.".into(),
            );
        }
    }
}

pub fn reset_training(instance: &mut Instance, network: &Arc<Mutex<NetworkState>>) {
    if let Some(mut old) = instance.feast_match.take() {
        old.supplies.clear(instance, network);
        for object in old
            .objects
            .values()
            .chain(old.fading.values().map(|(id, _)| id))
        {
            network.lock().remove_actor(instance, *object);
        }
    }
    instance.feast_whm = None;
    let mut rules = Match::new(0.0);
    for (id, actor) in &instance.actors {
        if !matches!(actor, NetworkedActor::Player { .. })
            && !instance.feast_dummies.contains_key(id)
        {
            continue;
        }
        let s = actor.get_common_spawn();
        rules.players.insert(
            id.0,
            Player {
                team: instance.feast_dummies.get(id).map_or(Team::Fang, |d| d.team),
                job: s.class_job,
                medals: 100,
                position: s.position.0.to_array(),
                alive: s.health_points > 0,
                present: true,
            },
        );
    }
    instance.feast_match = Some(Runtime {
        started: Instant::now(),
        rules,
        objects: BTreeMap::new(),
        fading: BTreeMap::new(),
        empty_since: None,
        placed: BTreeSet::new(),
        gate_viewers: BTreeMap::new(),
        medal_viewers: BTreeMap::new(),
        next_director_snapshot: 0.0,
        supplies: Default::default(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    fn inputs() -> Vec<Input> {
        vec![
            Input {
                id: 1,
                job: 24,
                position: [0.0; 3],
                alive: true,
            },
            Input {
                id: 2,
                job: 24,
                position: [10.0, 0.0, 0.0],
                alive: true,
            },
        ]
    }
    fn active() -> (Match, Vec<Input>) {
        let mut m = Match::new(0.0);
        let p = inputs();
        m.step(0.0, &p);
        m.step(COUNTDOWN, &p);
        (m, p)
    }
    fn total(m: &Match) -> u16 {
        let (f, c) = m.scores();
        f + c + m.drops.iter().map(|d| d.medals).sum::<u16>()
    }

    #[test]
    fn jail_packets_target_spawned_controllers_and_retry_after_loading() {
        let mut instance = Instance::default();
        instance.zone.id = 745;
        let viewer = ObjectId(1);
        instance.insert_empty_actor(viewer);
        let p = instance
            .find_actor_mut(viewer)
            .unwrap()
            .get_common_spawn_mut();
        p.class_job = 24;
        p.max_health_points = 10000;
        p.health_points = 10000;
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
                super::super::ClientState::default(),
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
                instance.feast_match.as_mut().unwrap().rules.deadline = 0.0;
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
        let mut instance = Instance::default();
        instance.zone.id = 745;
        let network = Arc::new(Mutex::new(NetworkState::default()));
        for id in [ObjectId(1), ObjectId(2)] {
            instance.insert_empty_actor(id);
            let p = instance.find_actor_mut(id).unwrap().get_common_spawn_mut();
            p.class_job = 24;
            p.health_points = 10000;
            p.max_health_points = 10000;
        }
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
        let controllers: Vec<_> = instance
            .actors
            .values()
            .filter_map(|actor| match actor {
                NetworkedActor::Object { object, .. }
                    if JAIL_LAYOUTS.contains(&object.bind_layout_id) =>
                {
                    Some(object.bind_layout_id)
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            controllers.len(),
            4,
            "two collision objects plus two VFX controllers"
        );
        let actor_count = instance.actors.len();
        tick(&mut instance, &network);
        assert_eq!(
            instance.actors.len(),
            actor_count,
            "controllers must not duplicate each tick"
        );
        for id in [ObjectId(1), ObjectId(2)] {
            let team = instance.feast_match.as_ref().unwrap().rules.players[&id.0].team;
            assert_eq!(
                instance
                    .find_actor(id)
                    .unwrap()
                    .get_common_spawn()
                    .position
                    .0
                    .x,
                team_spawn(team, 0).0.0.x
            );
        }
        let gate = |instance: &Instance| match instance.find_actor(ObjectId(100)).unwrap() {
            NetworkedActor::Object { object, .. } => object.args1,
            _ => unreachable!(),
        };
        assert_eq!(gate(&instance), SharedGroupTimelineState::TIMELINE_1.bits());
        instance.feast_match.as_mut().unwrap().rules.deadline = 0.0;
        tick(&mut instance, &network);
        assert_eq!(
            instance.feast_match.as_ref().unwrap().rules.phase,
            Phase::Active
        );
        assert_eq!(gate(&instance), SharedGroupTimelineState::TIMELINE_3.bits());
        // Normal movement after the opening must not be snapped back into jail.
        instance
            .find_actor_mut(ObjectId(1))
            .unwrap()
            .get_common_spawn_mut()
            .position = Position(glam::Vec3::ZERO);
        tick(&mut instance, &network);
        assert_eq!(
            instance
                .find_actor(ObjectId(1))
                .unwrap()
                .get_common_spawn()
                .position
                .0,
            glam::Vec3::ZERO
        );
        instance.feast_match.as_mut().unwrap().rules.phase = Phase::Finished(None);
        command(&mut instance, &network, ObjectId(1), "restart");
        assert_eq!(gate(&instance), SharedGroupTimelineState::TIMELINE_1.bits());
        assert_eq!(
            instance
                .find_actor(ObjectId(1))
                .unwrap()
                .get_common_spawn()
                .position
                .0
                .x
                .abs(),
            58.0
        );
    }

    #[test]
    fn pickup_animates_before_cleanup_and_restores_partial_pile() {
        let mut instance = Instance::default();
        instance.zone.id = 745;
        let network = Arc::new(Mutex::new(NetworkState::default()));
        for id in [ObjectId(1), ObjectId(2)] {
            instance.insert_empty_actor(id);
            let p = instance.find_actor_mut(id).unwrap().get_common_spawn_mut();
            p.class_job = 24;
            p.health_points = 10000;
            p.max_health_points = 10000;
        }
        tick(&mut instance, &network);
        instance.feast_match.as_mut().unwrap().rules.deadline = 0.0;
        tick(&mut instance, &network);
        for id in [ObjectId(1), ObjectId(2)] {
            instance
                .find_actor_mut(id)
                .unwrap()
                .get_common_spawn_mut()
                .position = Position(glam::Vec3::ZERO);
        }
        instance
            .find_actor_mut(ObjectId(1))
            .unwrap()
            .get_common_spawn_mut()
            .health_points = 0;
        tick(&mut instance, &network); // KO and partial pickup happen in the same tick.
        let runtime = instance.feast_match.as_ref().unwrap();
        assert_eq!(runtime.rules.players[&2].medals, 125);
        assert_eq!(runtime.rules.drops[0].medals, 25);
        assert_eq!(runtime.rules.pickups.len(), 1);
        let drop_id = runtime.rules.drops[0].id;
        let fading_actor = runtime.fading[&drop_id].0;
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
                .feast_match
                .as_ref()
                .unwrap()
                .rules
                .pickups
                .is_empty()
        );
        instance
            .feast_match
            .as_mut()
            .unwrap()
            .fading
            .get_mut(&drop_id)
            .unwrap()
            .1 = 0.0;
        tick(&mut instance, &network);
        assert!(instance.find_actor(fading_actor).is_none());
        let runtime = instance.feast_match.as_ref().unwrap();
        assert!(
            runtime.objects.contains_key(&drop_id),
            "remaining medals must reappear"
        );
        assert_eq!(runtime.rules.drops[0].medals, 25);
        instance
            .feast_match
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
                .feast_match
                .as_ref()
                .unwrap()
                .rules
                .drops
                .is_empty()
        );
        instance
            .feast_match
            .as_mut()
            .unwrap()
            .fading
            .get_mut(&drop_id)
            .unwrap()
            .1 = 0.0;
        tick(&mut instance, &network);
        let runtime = instance.feast_match.as_ref().unwrap();
        assert!(runtime.objects.is_empty() && runtime.fading.is_empty());
    }

    #[test]
    fn respawn_lock_expires_at_ten_seconds_without_restarting() {
        let mut instance = Instance::default();
        let id = ObjectId(1);
        let start = Instant::now();
        begin_respawn_lock(&mut instance, id, start);
        assert_eq!(respawn_remaining(&instance, id, start), 10000);
        begin_respawn_lock(&mut instance, id, start + std::time::Duration::from_secs(3));
        assert_eq!(
            respawn_remaining(
                &instance,
                id,
                start + std::time::Duration::from_millis(9999)
            ),
            1
        );
        assert_eq!(
            respawn_remaining(&instance, id, start + std::time::Duration::from_secs(10)),
            0
        );
        instance.feast_respawns.remove(&id);
        begin_respawn_lock(
            &mut instance,
            id,
            start + std::time::Duration::from_secs(20),
        );
        assert_eq!(
            respawn_remaining(&instance, id, start + std::time::Duration::from_secs(20)),
            10000
        );
    }

    #[test]
    fn medal_visuals_follow_size_and_personal_eligibility() {
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
        let (mut m, mut inputs) = active();
        let drop = Drop {
            id: 1,
            owner: 2,
            team: Team::Claw,
            medals: 63,
            position: [0.0; 3],
            expires: 100.0,
            abandoned: false,
        };
        assert!(m.can_collect(&inputs[0], &drop));
        assert!(!m.can_collect(&inputs[1], &drop));
        m.drops.push(Drop {
            owner: 1,
            team: Team::Fang,
            medals: 25,
            ..drop.clone()
        });
        assert!(!m.can_collect(&inputs[0], &drop)); // 100 held + 25 reserved
        m.drops.clear();
        assert!(m.can_collect(&inputs[0], &drop));
        inputs[0].job = 19;
        assert!(!m.can_collect(&inputs[0], &drop));
        inputs[0].job = 24;
        m.players.get_mut(&1).unwrap().alive = false;
        assert!(!m.can_collect(&inputs[0], &drop));
        assert_eq!(medal_state(Team::Claw, false).bits(), 1 << 4);
        assert_eq!(medal_state(Team::Claw, true).bits(), 1 << 10);
        assert_eq!(medal_state(Team::Fang, true).bits(), 1 << 8);
    }

    #[test]
    fn hud_tracks_pickups_phase_and_spectators() {
        let (mut m, _) = active();
        assert_eq!(
            m.hud_snapshot(COUNTDOWN + 1.0, 1),
            "KFEASTHUD1|active|359|100|100|fang|100|125|0"
        );
        m.players.get_mut(&1).unwrap().medals = 50;
        assert_eq!(
            m.hud_snapshot(COUNTDOWN + 2.0, 99),
            "KFEASTHUD1|active|358|50|100|spectator|0|0|0"
        );
        m.phase = Phase::Finished(Some(Team::Claw));
        assert!(
            m.hud_snapshot(COUNTDOWN + 3.0, 1)
                .starts_with("KFEASTHUD1|claw|0|50|100|")
        );
    }

    #[test]
    fn culling_resets_on_ko_but_not_during_sudden_death() {
        let (mut m, mut p) = active();
        assert_eq!(m.culling_stacks(COUNTDOWN + 89.999), 0);
        assert_eq!(m.culling_stacks(COUNTDOWN + 90.0), 1);
        assert_eq!(m.culling_stacks(COUNTDOWN + 105.0), 2);
        assert_eq!(m.culling_stacks(COUNTDOWN + 150.0), 5);
        p[1].alive = false;
        m.step(COUNTDOWN + 151.0, &p);
        assert_eq!(m.culling_stacks(COUNTDOWN + 151.0), 0);
        assert_eq!(m.culling_stacks(COUNTDOWN + 241.0), 1);
        // Let unclaimed medals return, so regulation ends tied.
        m.step(COUNTDOWN + 220.0, &p);
        let sudden = COUNTDOWN + MATCH_TIME;
        m.step(sudden, &p);
        assert_eq!(m.phase, Phase::SuddenDeath);
        assert_eq!(m.culling_stacks(sudden), 1);
        p[0].alive = false;
        m.step(sudden + 20.0, &p);
        assert_eq!(m.culling_stacks(sudden + 20.0), 2);
        assert_eq!(m.culling_stacks(sudden + 60.0), 5);
        m.step(sudden + SUDDEN_DEATH, &p);
        assert_eq!(m.culling_stacks(sudden + SUDDEN_DEATH), 0);
    }

    #[test]
    fn disconnect_drop_remains_after_ordinary_expiry() {
        let (mut m, p) = active();
        m.step(COUNTDOWN + 1.0, &p[1..]);
        m.step(COUNTDOWN + DROP_LIFETIME + 2.0, &p[1..]);
        assert_eq!(m.drops.len(),1);
        assert_eq!(m.players[&1].medals,0);
        m.step(COUNTDOWN + DROP_LIFETIME + 3.0, &p);
        assert!(m.drops.is_empty());
        assert_eq!(m.players[&1].medals,100);
    }

    #[test]
    fn six_hundred_medals_ends_regulation_immediately() {
        let (mut m, p) = active();
        m.players.get_mut(&1).unwrap().medals = 599;
        m.step(COUNTDOWN + 1.0, &p);
        assert_eq!(m.phase, Phase::Active);
        m.players.get_mut(&1).unwrap().medals = 600;
        m.step(COUNTDOWN + 2.0, &p);
        assert_eq!(m.phase, Phase::Finished(Some(Team::Fang)));
    }

    #[test]
    fn countdown_balances_and_locks_roster_and_jobs() {
        let mut m = Match::new(0.0);
        let mut p = inputs();
        m.step(0.0, &p);
        assert_eq!(m.scores(), (100, 100));
        assert!(!m.active(1, 24));
        m.step(COUNTDOWN - 1.0, &p);
        assert_eq!(m.phase, Phase::Countdown);
        m.step(COUNTDOWN, &p);
        assert!(m.active(1, 24));
        assert!(!m.active(1, 19));
        p.push(Input {
            id: 3,
            job: 19,
            position: [0.0; 3],
            alive: true,
        });
        m.step(COUNTDOWN + 1.0, &p);
        assert!(!m.players.contains_key(&3));
        assert_eq!(m.deadline, COUNTDOWN + MATCH_TIME);
        assert_eq!(
            (
                medal_limit(19),
                medal_limit(20),
                medal_limit(23),
                medal_limit(24)
            ),
            (Some(200), Some(175), Some(150), Some(125))
        );
    }
    #[test]
    fn death_once_enemy_pickup_cap_overflow_and_expiry() {
        let (mut m, mut p) = active();
        p[0].alive = false;
        m.step(COUNTDOWN + 1.0, &p);
        assert_eq!(m.players[&1].medals, 50);
        assert_eq!(m.drops[0].medals, 50);
        m.step(COUNTDOWN + 2.0, &p);
        assert_eq!(m.drops.len(), 1);
        p[1].position = [0.0; 3];
        m.step(COUNTDOWN + 3.0, &p);
        assert_eq!(m.players[&2].medals, 125);
        assert_eq!(m.drops[0].medals, 25);
        assert_eq!(total(&m), 200);
        m.step(COUNTDOWN + DROP_LIFETIME + 1.0, &p);
        assert!(m.drops.is_empty());
        assert_eq!(m.scores(), (75, 125));
        assert_eq!(total(&m), 200);
    }
    #[test]
    fn own_drops_reserve_capacity_and_cannot_be_reclaimed_by_walking() {
        let (mut m, mut p) = active();
        p[0].alive = false;
        p[1].alive = false;
        m.step(COUNTDOWN + 1.0, &p);
        p[0].alive = true;
        p[1].alive = true;
        m.step(COUNTDOWN + 2.0, &p);
        assert_eq!(m.scores(), (50, 50));
        p[0].position = [10.0, 0.0, 0.0];
        m.step(COUNTDOWN + 3.0, &p);
        // 50 held + 50 reserved leaves only 25 capacity.
        assert_eq!(m.players[&1].medals, 75);
        assert_eq!(m.drops.iter().find(|d| d.owner == 2).unwrap().medals, 25);
        assert_eq!(total(&m), 200);
    }
    #[test]
    fn match_end_freezes_scores_before_late_pickup() {
        let (mut m, mut p) = active();
        p[0].alive = false;
        m.step(COUNTDOWN + MATCH_TIME - 1.0, &p);
        p[1].position = [0.0; 3];
        m.step(COUNTDOWN + MATCH_TIME, &p);
        assert_eq!(m.phase, Phase::Finished(Some(Team::Claw)));
        assert_eq!(m.scores(), (50, 100));
        m.step(COUNTDOWN + MATCH_TIME + 80.0, &p);
        assert_eq!(m.scores(), (50, 100));
    }
    #[test]
    fn sudden_death_pickup_wins_and_timeout_draws() {
        let (mut m, mut p) = active();
        m.step(COUNTDOWN + MATCH_TIME, &p);
        assert_eq!(m.phase, Phase::SuddenDeath);
        p[0].alive = false;
        m.step(COUNTDOWN + MATCH_TIME + 1.0, &p);
        assert_eq!(m.phase, Phase::SuddenDeath);
        p[1].position = [0.0; 3];
        m.step(COUNTDOWN + MATCH_TIME + 2.0, &p);
        assert_eq!(m.phase, Phase::Finished(Some(Team::Claw)));
        let (mut m, p) = active();
        m.step(COUNTDOWN + MATCH_TIME + SUDDEN_DEATH, &p);
        assert_eq!(m.phase, Phase::Finished(None));
    }
    #[test]
    fn disconnect_drops_all_and_reconnect_only_restores_unclaimed_medals() {
        let (mut m, mut p) = active();
        m.step(COUNTDOWN + 1.0, &p[1..]);
        assert_eq!(m.players[&1].medals, 0);
        assert_eq!(m.drops[0].medals, 100);
        p[1].position = [0.0; 3];
        m.step(COUNTDOWN + 2.0, &p[1..]);
        assert_eq!(m.drops[0].medals, 75);
        m.step(COUNTDOWN + 3.0, &p);
        assert_eq!(m.scores(), (75, 125));
        assert!(m.drops.is_empty());
        assert_eq!(total(&m), 200);
    }
}
