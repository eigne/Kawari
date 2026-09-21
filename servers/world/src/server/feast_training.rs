//! Stationary, explicitly controlled opponents for solo medal testing.
use super::{
    actor::NetworkedActor,
    feast_match,
    instance::Instance,
    network::{DestinationNetwork, NetworkState},
};
use crate::FromServer;
use kawari::{
    common::{CharacterMode, ObjectId, ObjectTypeId, Position},
    ipc::zone::{
        ActionEffect, ActionResult, ActionType, BattleNpcSubKind, CharacterDataFlag, DamageElement,
        DamageKind, DamageType, EffectKind, ObjectKind, ServerZoneIpcData, ServerZoneIpcSegment,
        SpawnNpc,
    },
};
use parking_lot::Mutex;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

const ATTACK_INTERVAL: Duration = Duration::from_millis(2500);
const ATTACK_RANGE: f32 = 3.0;
const ATTACK_DAMAGE: u16 = 1000;

#[derive(Debug)]
pub struct Dummy {
    pub owner: ObjectId,
    pub attacking: bool,
    next_attack: Instant,
}

fn dummy_count(words: &[&str]) -> Option<usize> {
    match words {
        ["dummies"] => Some(1),
        ["dummies", count] => count.parse::<usize>().ok().filter(|n| (1..=3).contains(n)),
        _ => None,
    }
}

pub fn command(
    instance: &mut Instance,
    network: &Arc<Mutex<NetworkState>>,
    owner: ObjectId,
    words: &[&str],
) {
    if words.first() == Some(&"attack") {
        let enabled = match words {
            ["attack", "on"] => true,
            ["attack", "off"] => false,
            _ => {
                super::feast::notice(
                    network,
                    owner,
                    "Use !feast attack on or !feast attack off.".into(),
                );
                return;
            }
        };
        let mut count = 0;
        for dummy in instance
            .feast_dummies
            .values_mut()
            .filter(|d| d.owner == owner)
        {
            dummy.attacking = enabled;
            dummy.next_attack = Instant::now() + ATTACK_INTERVAL;
            count += 1;
        }
        super::feast::notice(
            network,
            owner,
            if count == 0 {
                "Create opponents first with !feast dummies.".into()
            } else {
                format!(
                    "{count} test opponents: attacks {}. They remain stationary; attack range is 3 yalms.",
                    if enabled { "ON" } else { "OFF" }
                )
            },
        );
        return;
    }
    let Some(count) = dummy_count(words) else {
        super::feast::notice(
            network,
            owner,
            "Use !feast dummies [1-3]. This starts a fresh solo practice match.".into(),
        );
        return;
    };
    if instance
        .actors
        .values()
        .filter(|a| matches!(a, NetworkedActor::Player { .. }))
        .count()
        != 1
    {
        super::feast::notice(
            network,
            owner,
            "Practice setup is only available when you are the only real player in the arena."
                .into(),
        );
        return;
    }
    let Some(NetworkedActor::Player { spawn, .. }) = instance.find_actor(owner) else {
        return;
    };
    if spawn.common.health_points == 0 || feast_match::medal_limit(spawn.common.class_job).is_none()
    {
        super::feast::notice(
            network,
            owner,
            "Return after death and equip a combat job before starting practice.".into(),
        );
        return;
    }
    let template = spawn.common.clone();
    let ids: Vec<_> = instance.feast_dummies.keys().copied().collect();
    for id in ids {
        instance.cancel_actor_tasks(id);
        network.lock().remove_actor(instance, id);
    }
    instance.feast_dummies.clear();
    for index in 0..count {
        let id = Instance::generate_actor_id();
        let mut common = template.clone();
        common.object_kind = ObjectKind::BattleNpc(BattleNpcSubKind::Enemy);
        common.base_id = 0;
        common.name_id = 0;
        common.layout_id = 0;
        common.name = format!("Feast Dummy {}", index + 1);
        common.target_id = ObjectTypeId::default();
        common.combat_tagger_id = ObjectTypeId::default();
        common.owner_id = ObjectId::default();
        common.battalion = 4;
        common.mode = CharacterMode::Normal;
        common.mode_arg = 0;
        common.health_points = 5000;
        common.max_health_points = 5000;
        common.resource_points = 10000;
        common.max_resource_points = 10000;
        common.class_job = 24;
        common.status_effects = Default::default();
        common.position =
            Position(template.position.0 + glam::Vec3::new(4.0, 0.0, index as f32 * 3.0));
        let spawn = SpawnNpc {
            character_data_flags: CharacterDataFlag::HOSTILE,
            common,
            ..Default::default()
        };
        // Training actors have no timeline or autonomous AI resource dependencies.
        instance.actors.insert(
            id,
            NetworkedActor::Npc {
                state: super::actor::NpcState::Wander,
                navmesh_path: Default::default(),
                navmesh_path_lerp: 0.0,
                navmesh_target: None,
                last_position: None,
                spawn,
                timeline: kawari::common::Timeline {
                    autoattack_action_id: 0,
                    timeline_always_plays: false,
                    timepoints: Vec::new(),
                    on_death: Vec::new(),
                },
                timeline_position: 0,
                newly_hated_actor: None,
                currently_invulnerable: false,
                status_effects: Default::default(),
            },
        );
        instance.feast_dummies.insert(
            id,
            Dummy {
                owner,
                attacking: false,
                next_attack: Instant::now() + ATTACK_INTERVAL,
            },
        );
    }
    feast_match::reset_training(instance, network);
    // Clear prior cast/resource state without carrying a previous match into practice.
    instance.cancel_actor_tasks(owner);
    network.lock().send_to_by_actor_id(
        owner,
        FromServer::ActorControlSelf(kawari::ipc::zone::ActorControlCategory::CancelCast {}),
        DestinationNetwork::ZoneClients,
    );
    if let Some(actor) = instance.find_actor_mut(owner) {
        let common = actor.get_common_spawn_mut();
        common.health_points = common.max_health_points;
        common.resource_points = common.max_resource_points;
    }
    super::action::update_actor_hp_mp(network.clone(), instance, owner);
    super::feast::notice(
        network,
        owner,
        format!(
            "Created {count} stationary opponents with 5000 HP and 100 medals each. You are Fangs; dummies are Claws. Fresh 60-second preparation. Use !feast attack on|off to control retaliation."
        ),
    );
}

pub fn tick(instance: &mut Instance, network: &Arc<Mutex<NetworkState>>) {
    if instance.zone.id != 745 || instance.feast_dummies.is_empty() {
        return;
    }
    let now = Instant::now();
    let candidates: Vec<_> = instance
        .feast_dummies
        .iter()
        .filter(|(_, d)| d.attacking && now >= d.next_attack)
        .map(|(id, d)| (*id, d.owner))
        .collect();
    for (source, target) in candidates {
        instance.feast_dummies.get_mut(&source).unwrap().next_attack = now + ATTACK_INTERVAL;
        if !feast_match::can_act(instance, source) || !feast_match::can_act(instance, target) {
            continue;
        }
        let Some(a) = instance.find_actor(source) else {
            continue;
        };
        let Some(b) = instance.find_actor(target) else {
            continue;
        };
        let ac = a.get_common_spawn();
        let bc = b.get_common_spawn();
        if ac.health_points == 0
            || bc.health_points == 0
            || ac.position.0.distance(bc.position.0) > ATTACK_RANGE
        {
            continue;
        }
        if instance
            .feast_whm
            .as_ref()
            .is_some_and(|f| f.blocked(source, false))
        {
            continue;
        }
        strike(instance, network, source, target);
    }
}

fn strike(
    instance: &mut Instance,
    network: &Arc<Mutex<NetworkState>>,
    source: ObjectId,
    target: ObjectId,
) {
    let damage = super::feast::adjust_damage(instance, source, target, ATTACK_DAMAGE);
    let Some(actor) = instance.find_actor_mut(target) else {
        return;
    };
    let common = actor.get_common_spawn_mut();
    let amount = common.health_points.min(damage as u32) as u16;
    common.health_points -= amount as u32;
    let target_id = ObjectTypeId {
        object_id: target,
        ..Default::default()
    };
    let mut effects = [ActionEffect::default(); 8];
    effects[0].kind = EffectKind::Damage {
        damage_kind: DamageKind::Normal,
        damage_type: DamageType::Physical,
        damage_element: DamageElement::Unaspected,
        bonus_percent: 0,
        unk3: 0,
        unk4: 0,
        amount,
    };
    {
        let mut net = network.lock();
        let result = ActionResult {
            action_id: 870,
            spell_id: 870,
            action_type: ActionType::Action,
            animation_target_id: target_id,
            target_id_again: target_id,
            effect_count: 1,
            effects,
            animation_lock: kawari::common::ANIMATION_LOCK_TIME,
            global_sequence: net.global_action_sequence,
            ..Default::default()
        };
        net.global_action_sequence = net.global_action_sequence.wrapping_add(1);
        net.send_in_range_inclusive_instance(
            source,
            instance,
            FromServer::PacketSegment(
                ServerZoneIpcSegment::new(ServerZoneIpcData::ActionResult(result)),
                source,
            ),
            DestinationNetwork::ZoneClients,
        );
    }
    super::action::update_actor_hp_mp(network.clone(), instance, target);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn practice_count_is_bounded() {
        assert_eq!(dummy_count(&["dummies"]), Some(1));
        assert_eq!(dummy_count(&["dummies", "3"]), Some(3));
        for count in ["0", "4", "-1", "no"] {
            assert_eq!(dummy_count(&["dummies", count]), None);
        }
    }

    #[test]
    fn practice_integrates_medals_and_attack_gates() {
        let mut instance = Instance::default();
        instance.zone.id = 745;
        let owner = ObjectId(1);
        instance.insert_empty_actor(owner);
        let p = instance
            .find_actor_mut(owner)
            .unwrap()
            .get_common_spawn_mut();
        p.class_job = 24;
        p.health_points = 20000;
        p.max_health_points = 20000;
        p.resource_points = 10000;
        p.max_resource_points = 10000;
        let network = Arc::new(Mutex::new(NetworkState::default()));
        command(&mut instance, &network, owner, &["dummies"]);
        assert_eq!(instance.feast_dummies.len(), 1);
        assert_eq!(
            instance.feast_match.as_ref().unwrap().rules.scores(),
            (100, 100)
        );
        let dummy = *instance.feast_dummies.keys().next().unwrap();
        instance
            .find_actor_mut(dummy)
            .unwrap()
            .get_common_spawn_mut()
            .position = Position(glam::Vec3::new(2.0, 0.0, 0.0));
        let ready = Instant::now() - Duration::from_secs(1);
        instance.feast_dummies.get_mut(&dummy).unwrap().next_attack = ready;
        tick(&mut instance, &network); // Passive even when its timer is ready.
        assert_eq!(
            instance
                .find_actor(owner)
                .unwrap()
                .get_common_spawn()
                .health_points,
            20000
        );
        command(&mut instance, &network, owner, &["attack", "on"]);
        instance.feast_dummies.get_mut(&dummy).unwrap().next_attack = ready;
        tick(&mut instance, &network); // Countdown prevents attacks.
        assert_eq!(
            instance
                .find_actor(owner)
                .unwrap()
                .get_common_spawn()
                .health_points,
            20000
        );
        instance.feast_match.as_mut().unwrap().rules.phase = feast_match::Phase::Active;
        instance.feast_dummies.get_mut(&dummy).unwrap().next_attack = ready;
        tick(&mut instance, &network);
        assert_eq!(
            instance
                .find_actor(owner)
                .unwrap()
                .get_common_spawn()
                .health_points,
            19000
        );
        command(&mut instance, &network, owner, &["attack", "off"]);
        instance.feast_dummies.get_mut(&dummy).unwrap().next_attack = ready;
        tick(&mut instance, &network);
        assert_eq!(
            instance
                .find_actor(owner)
                .unwrap()
                .get_common_spawn()
                .health_points,
            19000
        );
        instance
            .find_actor_mut(dummy)
            .unwrap()
            .get_common_spawn_mut()
            .health_points = 0;
        feast_match::tick(&mut instance, &network);
        // The living healer picks up 25 of the dropped 50; the remainder stays down.
        let m = &instance.feast_match.as_ref().unwrap().rules;
        assert_eq!(m.scores(), (125, 50));
        assert_eq!(m.drops[0].medals, 25);
    }
}
