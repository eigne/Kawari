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

#[derive(Debug)]
pub struct Match {
    pub phase: Phase,
    pub deadline: f64,
    pub players: BTreeMap<u32, Player>,
    pub drops: Vec<Drop>,
    next_drop: u32,
    last_clock: u32,
    pub announcements: Vec<String>,
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

    pub fn active(&self, id: u32, job: u8) -> bool {
        matches!(self.phase, Phase::Active | Phase::SuddenDeath)
            && self
                .players
                .get(&id)
                .is_some_and(|p| p.present && p.job == job)
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
            if now >= drop.expires {
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
                let reserved: u16 = self
                    .drops
                    .iter()
                    .filter(|d| d.owner == id)
                    .map(|d| d.medals)
                    .sum();
                let space = medal_limit(p.job)
                    .unwrap()
                    .saturating_sub(p.medals + reserved);
                let amount = space.min(drop.medals);
                if amount == 0 {
                    continue;
                }
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
    common::{ObjectId, Position, SharedGroupTimelineState},
    ipc::zone::{
        ActorControlCategory, ObjectKind, ServerZoneIpcData, ServerZoneIpcSegment, SpawnObject,
    },
};
use parking_lot::Mutex;
use std::{sync::Arc, time::Instant};

#[derive(Debug)]
pub struct Runtime {
    started: Instant,
    pub rules: Match,
    objects: BTreeMap<u32, ObjectId>,
    empty_since: Option<f64>,
    placed: BTreeSet<u32>,
    gate_viewers: BTreeMap<u32, bool>,
}

// The start_col layer centers the two jails at x = +/-58, y = 0.25.
// Collision and VFX are separate shared groups in the restored layout.
const JAIL_LAYOUTS: [u32; 4] = [7403947, 7403946, 7515404, 7515405];
const JAIL_EOBJS: [u32; 2] = [2008986, 2008987];

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
        runtime.gate_viewers.remove(&actor.0);
    }
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
    for base in JAIL_EOBJS {
        if let Some(id) = instance.find_object_by_eobj_id(base) {
            if let Some(NetworkedActor::Object { object, .. }) = instance.find_actor_mut(id) {
                object.args1 = state.bits();
            }
        }
    }
    runtime
        .placed
        .retain(|id| inputs.iter().any(|i| i.id == *id));
    runtime
        .gate_viewers
        .retain(|id, _| inputs.iter().any(|i| i.id == *id));
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
        if runtime.gate_viewers.get(&input.id) != Some(&closed) {
            for layout_id in JAIL_LAYOUTS {
                network.lock().send_to_by_actor_id(
                    id,
                    FromServer::ActorControl(
                        id,
                        ActorControlCategory::SetSharedGroupTimelineState {
                            state,
                            arg2: 0,
                            object_type: 1,
                            layout_id,
                        },
                    ),
                    DestinationNetwork::ZoneClients,
                );
            }
            runtime.gate_viewers.insert(input.id, closed);
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
                for object in m.objects.values() {
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
        empty_since: None,
        placed: BTreeSet::new(),
        gate_viewers: BTreeMap::new(),
    });
    let now = runtime.started.elapsed().as_secs_f64();
    runtime.empty_since = None;
    let previous: Vec<_> = runtime.rules.players.keys().copied().collect();
    runtime.rules.step(now, &inputs);
    sync_jails(instance, network, &mut runtime, &inputs);
    // Normal client countdown UI; match clock/scores are announced in chat for now.
    if runtime.rules.phase == Phase::Countdown {
        for input in &inputs {
            if previous.contains(&input.id) || !runtime.rules.players.contains_key(&input.id) {
                continue;
            }
            let id = ObjectId(input.id);
            if let Some(NetworkedActor::Player { spawn, .. }) = instance.find_actor(id) {
                let packet = ServerZoneIpcData::Countdown {
                    account_id: spawn.account_id,
                    content_id: spawn.content_id,
                    starter_actor_id: id,
                    starter_name: "Feast".into(),
                    unk: 0x5B,
                    duration: (runtime.rules.deadline - now).ceil().max(1.0) as u16,
                };
                network.lock().send_to_by_actor_id(
                    id,
                    FromServer::PacketSegment(ServerZoneIpcSegment::new(packet), id),
                    DestinationNetwork::ZoneClients,
                );
            }
        }
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
        if runtime.objects.contains_key(&drop.id) {
            continue;
        }
        let id = Instance::generate_actor_id();
        // Native Feast medal prefab: pvp/002, with dropmedal_f1/c1 timelines.
        // The old 2002056 was a quest's lion medal with generic event-item VFX.
        let object = SpawnObject {
            kind: ObjectKind::EventObj,
            base_id: 2007627,
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
    for message in runtime.rules.announcements.drain(..) {
        tracing::info!(%message, "Feast match");
        for input in &inputs {
            super::feast::notice(network, ObjectId(input.id), message.clone());
        }
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
    if matches!(words.first(), Some(&"dummies") | Some(&"attack")) {
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
                let count = instance.feast_dummies.len().to_string();
                super::feast_training::command(instance, network, id, &["dummies", &count]);
                return;
            }
            instance.feast_match = None;
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
    if let Some(old) = instance.feast_match.take() {
        for object in old.objects.values() {
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
                team: if instance.feast_dummies.contains_key(id) {
                    Team::Claw
                } else {
                    Team::Fang
                },
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
        empty_since: None,
        placed: BTreeSet::new(),
        gate_viewers: BTreeMap::new(),
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
        m.step(60.0, &p);
        (m, p)
    }
    fn total(m: &Match) -> u16 {
        let (f, c) = m.scores();
        f + c + m.drops.iter().map(|d| d.medals).sum::<u16>()
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
                    ..Default::default()
                },
                String::new(),
            );
        }
        tick(&mut instance, &network);
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
    fn hud_tracks_pickups_phase_and_spectators() {
        let (mut m, _) = active();
        assert_eq!(
            m.hud_snapshot(61.0, 1),
            "KFEASTHUD1|active|359|100|100|fang|100|125|0"
        );
        m.players.get_mut(&1).unwrap().medals = 50;
        assert_eq!(
            m.hud_snapshot(62.0, 99),
            "KFEASTHUD1|active|358|50|100|spectator|0|0|0"
        );
        m.phase = Phase::Finished(Some(Team::Claw));
        assert!(
            m.hud_snapshot(63.0, 1)
                .starts_with("KFEASTHUD1|claw|0|50|100|")
        );
    }

    #[test]
    fn countdown_balances_and_locks_roster_and_jobs() {
        let mut m = Match::new(0.0);
        let mut p = inputs();
        m.step(0.0, &p);
        assert_eq!(m.scores(), (100, 100));
        assert!(!m.active(1, 24));
        m.step(59.0, &p);
        assert_eq!(m.phase, Phase::Countdown);
        m.step(60.0, &p);
        assert!(m.active(1, 24));
        assert!(!m.active(1, 19));
        p.push(Input {
            id: 3,
            job: 19,
            position: [0.0; 3],
            alive: true,
        });
        m.step(61.0, &p);
        assert!(!m.players.contains_key(&3));
        assert_eq!(m.deadline, 420.0);
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
        m.step(61.0, &p);
        assert_eq!(m.players[&1].medals, 50);
        assert_eq!(m.drops[0].medals, 50);
        m.step(62.0, &p);
        assert_eq!(m.drops.len(), 1);
        p[1].position = [0.0; 3];
        m.step(63.0, &p);
        assert_eq!(m.players[&2].medals, 125);
        assert_eq!(m.drops[0].medals, 25);
        assert_eq!(total(&m), 200);
        m.step(121.0, &p);
        assert!(m.drops.is_empty());
        assert_eq!(m.scores(), (75, 125));
        assert_eq!(total(&m), 200);
    }
    #[test]
    fn own_drops_reserve_capacity_and_cannot_be_reclaimed_by_walking() {
        let (mut m, mut p) = active();
        p[0].alive = false;
        p[1].alive = false;
        m.step(61.0, &p);
        p[0].alive = true;
        p[1].alive = true;
        m.step(62.0, &p);
        assert_eq!(m.scores(), (50, 50));
        p[0].position = [10.0, 0.0, 0.0];
        m.step(63.0, &p);
        // 50 held + 50 reserved leaves only 25 capacity.
        assert_eq!(m.players[&1].medals, 75);
        assert_eq!(m.drops.iter().find(|d| d.owner == 2).unwrap().medals, 25);
        assert_eq!(total(&m), 200);
    }
    #[test]
    fn match_end_freezes_scores_before_late_pickup() {
        let (mut m, mut p) = active();
        p[0].alive = false;
        m.step(419.0, &p);
        p[1].position = [0.0; 3];
        m.step(420.0, &p);
        assert_eq!(m.phase, Phase::Finished(Some(Team::Claw)));
        assert_eq!(m.scores(), (50, 100));
        m.step(500.0, &p);
        assert_eq!(m.scores(), (50, 100));
    }
    #[test]
    fn sudden_death_pickup_wins_and_timeout_draws() {
        let (mut m, mut p) = active();
        m.step(420.0, &p);
        assert_eq!(m.phase, Phase::SuddenDeath);
        p[0].alive = false;
        m.step(421.0, &p);
        assert_eq!(m.phase, Phase::SuddenDeath);
        p[1].position = [0.0; 3];
        m.step(422.0, &p);
        assert_eq!(m.phase, Phase::Finished(Some(Team::Claw)));
        let (mut m, p) = active();
        m.step(540.0, &p);
        assert_eq!(m.phase, Phase::Finished(None));
    }
    #[test]
    fn disconnect_drops_all_and_reconnect_only_restores_unclaimed_medals() {
        let (mut m, mut p) = active();
        m.step(61.0, &p[1..]);
        assert_eq!(m.players[&1].medals, 0);
        assert_eq!(m.drops[0].medals, 100);
        p[1].position = [0.0; 3];
        m.step(62.0, &p[1..]);
        assert_eq!(m.drops[0].medals, 75);
        m.step(63.0, &p);
        assert_eq!(m.scores(), (75, 125));
        assert!(m.drops.is_empty());
        assert_eq!(total(&m), 200);
    }
}
