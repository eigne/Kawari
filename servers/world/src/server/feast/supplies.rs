//! Supply boxes: breakable crates that spawn at 1:30, 3:00 and 4:30.
//!
//! A box is a hostile battle NPC that anyone can attack, standing on an
//! invisible controller object bound to the original l1p2 layout prefab
//! (which supplies the real model, break animation and sound). When the box
//! breaks, the team that dealt the most total damage to it gets a kit on the
//! ground; any living member of that team can walk over it to collect:
//! Adrenaline (+25 adrenaline) or Wolf's Heart (MP regeneration).
use super::rules::{MATCH_TIME, Match, Phase, Team};
use crate::server::{actor::NetworkedActor, instance::Instance, network::NetworkState};
use kawari::{
    common::{ObjectId, Position, SharedGroupTimelineState},
    ipc::zone::{BattleNpcSubKind, CharacterDataFlag, ObjectKind, SpawnNpc, SpawnObject},
};
use parking_lot::Mutex;
use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

// Exact box HP is not published in the final guide. Isolated tuning pending a 6.08 capture.
const BOX_HP: u32 = 5000;
// Untouched-kit lifetime is likewise provisional until a historical capture is found.
const KIT_LIFETIME: f64 = 60.0;
const KIT_PICKUP_RADIUS: f32 = 2.0;
/// Seconds into the match when each wave spawns.
const WAVES: [(f64, Kind); 3] = [
    (90.0, Kind::Adrenaline),
    (180.0, Kind::WolfsHeart),
    (270.0, Kind::Adrenaline),
];
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Adrenaline,
    WolfsHeart,
}
#[derive(Debug)]
pub struct BoxState {
    pub kind: Kind,
    controller: ObjectId,
    damage: BTreeMap<u32, u64>,
}
#[derive(Debug)]
struct Kit {
    kind: Kind,
    team: Team,
    position: [f32; 3],
    expires: f64,
}
#[derive(Debug, Default)]
pub struct Supplies {
    waves: u8,
    pub boxes: HashMap<ObjectId, BoxState>,
    controllers: Vec<ObjectId>,
    kits: HashMap<ObjectId, Kit>,
    fading: HashMap<ObjectId, f64>,
}
impl Supplies {
    pub fn record_damage(&mut self, source: u32, target: ObjectId, amount: u32) {
        if let Some(b) = self.boxes.get_mut(&target) {
            *b.damage.entry(source).or_default() += u64::from(amount);
        }
    }
    fn due(&mut self, phase: Phase, elapsed: f64) -> Vec<Kind> {
        if phase != Phase::Active {
            return Vec::new();
        }
        let mut result = Vec::new();
        for (i, (at, kind)) in WAVES.into_iter().enumerate() {
            if elapsed >= at && self.waves & (1 << i) == 0 {
                self.waves |= 1 << i;
                result.push(kind);
            }
        }
        result
    }
    pub fn clear(&mut self, instance: &mut Instance, network: &Arc<Mutex<NetworkState>>) {
        for id in std::mem::take(&mut self.boxes).into_keys() {
            network.lock().remove_actor(instance, id);
        }
        for id in std::mem::take(&mut self.kits)
            .into_keys()
            .chain(std::mem::take(&mut self.fading).into_keys())
        {
            network.lock().remove_actor(instance, id);
        }
        for id in std::mem::take(&mut self.controllers) {
            crate::server::actor::set_shared_group_timeline_state(
                instance,
                &mut network.lock(),
                id,
                &[1],
            );
            network.lock().remove_actor(instance, id);
        }
    }
}

fn spawn(
    instance: &mut Instance,
    network: &Arc<Mutex<NetworkState>>,
    supplies: &mut Supplies,
    kind: Kind,
    layout: u32,
    position: [f32; 3],
) {
    let controller = Instance::generate_actor_id();
    // Bound layout objects supply the real model, sound and animation. No replacement art.
    instance.insert_object(
        controller,
        SpawnObject {
            entity_id: controller,
            kind: ObjectKind::EventObj,
            // A valid bound-object row is required even when the model comes
            // from bind_layout_id. Use the same controller row as Feast jails;
            // the bound supply SG supplies its own model and timelines.
            base_id: 2008986,
            bind_layout_id: layout,
            radius: 1.0,
            position: Position(glam::Vec3A::from_array(position)),
            not_targetable: true,
            args1: SharedGroupTimelineState::TIMELINE_4.bits(),
            ..Default::default()
        },
        String::new(),
    );
    supplies.controllers.push(controller);
    let id = Instance::generate_actor_id();
    let mut spawn = SpawnNpc {
        gimmick_id: layout,
        character_data_flags: CharacterDataFlag::HOSTILE,
        ..Default::default()
    };
    spawn.common.object_kind = ObjectKind::BattleNpc(BattleNpcSubKind::Enemy);
    spawn.common.name_id = if kind == Kind::Adrenaline { 4686 } else { 5476 };
    spawn.common.name = if kind == Kind::Adrenaline {
        "Adrenaline"
    } else {
        "Wolf's Heart"
    }
    .into();
    spawn.common.position = Position(glam::Vec3A::from_array(position));
    spawn.common.health_points = BOX_HP;
    spawn.common.max_health_points = BOX_HP;
    spawn.common.battalion = 4;
    super::insert_npc(instance, id, spawn);
    supplies.boxes.insert(
        id,
        BoxState {
            kind,
            controller,
            damage: BTreeMap::new(),
        },
    );
    // Native timeline table: 1 off, 2 break, 3 visible, 4 spawn.
    crate::server::actor::set_shared_group_timeline_state(
        instance,
        &mut network.lock(),
        controller,
        &[4],
    );
    tracing::info!(
        ?kind,
        actor = id.0,
        controller = controller.0,
        layout,
        ?position,
        "Feast supply box spawned"
    );
}

pub fn tick(
    instance: &mut Instance,
    network: &Arc<Mutex<NetworkState>>,
    rules: &Match,
    supplies: &mut Supplies,
    now: f64,
) {
    if matches!(rules.phase, Phase::Finished(_)) {
        supplies.clear(instance, network);
        return;
    }
    let broken: Vec<_> = supplies
        .boxes
        .keys()
        .copied()
        .filter(|id| {
            instance
                .find_actor(*id)
                .is_none_or(|a| a.get_common_spawn().health_points == 0)
        })
        .collect();
    for id in broken {
        let b = supplies.boxes.remove(&id).unwrap();
        // Japanese official guide: aggregate damage claims the kit for a TEAM.
        // Any one living member of that team can then collect it from the ground.
        let (mut fang, mut claw) = (0u64, 0u64);
        for (id, damage) in &b.damage {
            if let Some(player) = rules.players.get(id) {
                match player.team {
                    Team::Fang => fang += damage,
                    Team::Claw => claw += damage,
                }
            }
        }
        if fang + claw > 0 {
            // Equal contribution has no documented tie-break; stable Fangs tie-break for now.
            let team = if fang >= claw { Team::Fang } else { Team::Claw };
            if let Some(actor) = instance.find_actor(id) {
                let position = actor.get_common_spawn().position.0.to_array();
                let kit = Instance::generate_actor_id();
                instance.insert_object(
                    kit,
                    SpawnObject {
                        entity_id: kit,
                        radius: 1.0,
                        kind: ObjectKind::EventObj,
                        base_id: if b.kind == Kind::Adrenaline {
                            2007626
                        } else {
                            2007652
                        },
                        position: Position(glam::Vec3A::from_array(position)),
                        not_targetable: true,
                        args1: if team == Team::Fang {
                            SharedGroupTimelineState::TIMELINE_3.bits()
                        } else {
                            SharedGroupTimelineState::TIMELINE_5.bits()
                        },
                        ..Default::default()
                    },
                    String::new(),
                );
                supplies.kits.insert(
                    kit,
                    Kit {
                        kind: b.kind,
                        team,
                        position,
                        expires: now + KIT_LIFETIME,
                    },
                );
            }
        }
        crate::server::actor::set_shared_group_timeline_state(
            instance,
            &mut network.lock(),
            b.controller,
            &[2],
        );
        network.lock().remove_actor(instance, id);
    }
    let mut collected = Vec::new();
    for (&id, kit) in &supplies.kits {
        if now >= kit.expires {
            collected.push(id);
            continue;
        }
        if let Some((&winner, _)) = rules.players.iter().find(|(_, p)| {
            p.present
                && p.alive
                && p.team == kit.team
                && p.position
                    .iter()
                    .zip(kit.position)
                    .map(|(a, b)| (a - b).powi(2))
                    .sum::<f32>()
                    <= KIT_PICKUP_RADIUS * KIT_PICKUP_RADIUS
        }) {
            tracing::info!(kind = ?kit.kind, player = winner, "Feast supply kit collected");
            super::combat::grant_supply(instance, network, ObjectId(winner), kit.kind);
            collected.push(id);
        }
    }
    for id in collected {
        supplies.kits.remove(&id);
        crate::server::actor::set_shared_group_timeline_state(
            instance,
            &mut network.lock(),
            id,
            &[2],
        );
        supplies.fading.insert(id, now + 0.5);
    }
    let faded: Vec<_> = supplies
        .fading
        .iter()
        .filter(|(_, until)| now >= **until)
        .map(|(id, _)| *id)
        .collect();
    for id in faded {
        supplies.fading.remove(&id);
        network.lock().remove_actor(instance, id);
    }
    for kind in supplies.due(rules.phase, now - (rules.deadline - MATCH_TIME)) {
        spawn_wave(instance, network, supplies, kind);
    }
}

/// Spawns one supply wave now. Also used by `!feast supply` for testing.
pub fn spawn_wave(
    instance: &mut Instance,
    network: &Arc<Mutex<NetworkState>>,
    supplies: &mut Supplies,
    kind: Kind,
) {
    match kind {
        Kind::Adrenaline => {
            // Original planmap yellow supply placements, north/south of the centre.
            for (layout, z) in [(7167385, 10.0), (7167387, -10.0)] {
                // Do not stack a later wave over an unbroken earlier box.
                let occupied = supplies.boxes.keys().any(|id| matches!(instance.find_actor(*id), Some(NetworkedActor::Npc {spawn, ..}) if spawn.gimmick_id == layout));
                if !occupied {
                    spawn(instance, network, supplies, kind, layout, [0.0, -2.5, z]);
                }
            }
        }
        Kind::WolfsHeart => spawn(instance, network, supplies, kind, 7167407, [0.0, -2.5, 0.0]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_boxes_bind_original_layouts_and_cleanup_after_destruction() {
        let mut instance = Instance::default();
        let network = Arc::new(Mutex::new(NetworkState::default()));
        let mut rules = Match::new(0.0);
        rules.phase = Phase::Active;
        rules.deadline = 420.0;
        let mut supplies = Supplies::default();
        tick(&mut instance, &network, &rules, &mut supplies, 150.0);
        assert_eq!(supplies.boxes.len(), 2);
        for controller in &supplies.controllers {
            match instance.find_actor(*controller).unwrap() {
                NetworkedActor::Object { object, .. } => {
                    assert_eq!(object.base_id, 2008986);
                    assert_eq!(object.radius, 1.0);
                    assert!([7167385, 7167387].contains(&object.bind_layout_id));
                }
                _ => panic!("Supply model requires a bound native event object"),
            }
        }
        let id = *supplies.boxes.keys().next().unwrap();
        match instance.find_actor(id).unwrap() {
            NetworkedActor::Npc { spawn, .. } => {
                assert!([7167385, 7167387].contains(&spawn.gimmick_id));
                assert_eq!(spawn.common.name_id, 4686);
            }
            _ => panic!("Supply must be a native targetable battle NPC"),
        }
        supplies.record_damage(1, id, 4000);
        supplies.record_damage(2, id, 1000);
        assert_eq!(supplies.boxes[&id].damage[&1], 4000);
        instance
            .find_actor_mut(id)
            .unwrap()
            .get_common_spawn_mut()
            .health_points = 0;
        tick(&mut instance, &network, &rules, &mut supplies, 151.0);
        assert_eq!(supplies.boxes.len(), 1);
        assert!(instance.find_actor(id).is_none());
        tick(&mut instance, &network, &rules, &mut supplies, 240.0);
        assert_eq!(supplies.boxes.len(), 2); // remaining adrenaline and centre Wolf's Heart
        rules.phase = Phase::Finished(None);
        tick(&mut instance, &network, &rules, &mut supplies, 241.0);
        assert!(supplies.boxes.is_empty());
        assert!(instance.actors.is_empty());
    }

    #[test]
    fn team_aggregate_wins_kit_and_enemy_cannot_pick_it_up() {
        let mut instance = Instance::default();
        let network = Arc::new(Mutex::new(NetworkState::default()));
        let mut rules = Match::new(0.0);
        let inputs: Vec<_> = (1..=3)
            .map(|id| super::super::rules::Input {
                id,
                job: 24,
                alive: true,
                position: [50.0, 0.0, 0.0],
                team: None,
            })
            .collect();
        rules.step(0.0, &inputs);
        rules.step(60.0, &inputs);
        let mut s = Supplies::default();
        tick(&mut instance, &network, &rules, &mut s, 150.0);
        let id = *s.boxes.keys().next().unwrap();
        let position = instance
            .find_actor(id)
            .unwrap()
            .get_common_spawn()
            .position
            .0
            .to_array();
        s.record_damage(1, id, 1500);
        s.record_damage(3, id, 1500);
        s.record_damage(2, id, 2500);
        instance
            .find_actor_mut(id)
            .unwrap()
            .get_common_spawn_mut()
            .health_points = 0;
        rules.players.get_mut(&2).unwrap().position = position;
        tick(&mut instance, &network, &rules, &mut s, 151.0);
        assert_eq!(s.kits.len(), 1);
        assert_eq!(s.kits.values().next().unwrap().team, Team::Fang);
        // The Claw who dealt the largest individual hit cannot take the Fangs' kit.
        tick(&mut instance, &network, &rules, &mut s, 152.0);
        assert_eq!(s.kits.len(), 1);
        rules.players.get_mut(&1).unwrap().position = position;
        tick(&mut instance, &network, &rules, &mut s, 153.0);
        assert!(s.kits.is_empty());
        assert_eq!(s.fading.len(), 1);
        tick(&mut instance, &network, &rules, &mut s, 154.0);
        assert!(s.fading.is_empty());
    }

    #[test]
    fn waves_are_once_only_at_exact_boundaries_and_never_in_overtime() {
        let mut s = Supplies::default();
        assert!(s.due(Phase::Countdown, 400.0).is_empty());
        assert!(s.due(Phase::Active, 89.999).is_empty());
        assert_eq!(s.due(Phase::Active, 90.0), vec![Kind::Adrenaline]);
        assert!(s.due(Phase::Active, 90.1).is_empty());
        assert_eq!(s.due(Phase::Active, 180.0), vec![Kind::WolfsHeart]);
        assert_eq!(s.due(Phase::Active, 270.0), vec![Kind::Adrenaline]);
        assert!(s.due(Phase::SuddenDeath, 400.0).is_empty());
    }
}
