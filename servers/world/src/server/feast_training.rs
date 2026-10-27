//! Solo practice actors: stationary dummies and simple autonomous WHM teams.
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
    pub team: feast_match::Team,
    pub autonomous: bool,
    next_attack: Instant,
    next_move: Instant,
    cast_until: Instant,
    respawn_at: Option<Instant>,
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
                    "{count} practice actors: activity {}.",
                    if enabled { "ON" } else { "OFF" }
                )
            },
        );
        return;
    }
    let autonomous = matches!(words, ["bots"] | ["bots", "on"]);
    let count = if autonomous {
        Some(7)
    } else if words == ["bots", "off"] {
        Some(0)
    } else {
        dummy_count(words)
    };
    let Some(count) = count else {
        super::feast::notice(
            network,
            owner,
            "Use !feast bots [on|off] or !feast dummies [1-3]. This starts a fresh solo practice match.".into(),
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
    let allied_jobs: Vec<_> = [24, 19, 34, 25]
        .into_iter()
        .filter(|job| *job != template.class_job)
        .collect();
    for index in 0..count {
        let team = if autonomous && index < 3 {
            feast_match::Team::Fang
        } else {
            feast_match::Team::Claw
        };
        let id = Instance::generate_actor_id();
        let mut common = template.clone();
        common.object_kind = ObjectKind::BattleNpc(BattleNpcSubKind::Enemy);
        common.base_id = 0;
        common.name_id = 0;
        common.layout_id = 0;
        common.name = if autonomous {
            format!("Feast {} {}", team.name(), index + 1)
        } else {
            format!("Feast Dummy {}", index + 1)
        };
        common.target_id = ObjectTypeId::default();
        common.combat_tagger_id = ObjectTypeId::default();
        common.owner_id = ObjectId::default();
        common.battalion = if team == feast_match::Team::Fang {
            0
        } else {
            4
        };
        common.mode = CharacterMode::Normal;
        common.mode_arg = 0;
        common.health_points = 5000;
        common.max_health_points = 5000;
        common.resource_points = 10000;
        common.max_resource_points = 10000;
        common.class_job = 24;
        if autonomous {
            common.class_job = if index < 3 {
                allied_jobs[index]
            } else {
                [24, 19, 34, 25][index - 3]
            };
        }
        common.status_effects = Default::default();
        common.position =
            Position(template.position.0 + glam::Vec3::new(4.0, 0.0, index as f32 * 3.0));
        if autonomous {
            (common.position, common.rotation) = feast_match::team_spawn(team, index);
        }
        let spawn = SpawnNpc {
            character_data_flags: if team == feast_match::Team::Claw {
                CharacterDataFlag::HOSTILE
            } else {
                CharacterDataFlag::empty()
            },
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
                attacking: autonomous,
                autonomous,
                team,
                next_attack: Instant::now() + ATTACK_INTERVAL,
                next_move: Instant::now(),
                cast_until: Instant::now(),
                respawn_at: None,
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
        if autonomous {
            "Created mixed-job 4v4 practice teams using WHM, PLD, SAM and BLM. Fresh 60-second preparation. !feast attack off pauses bots; !feast bots off removes them.".into()
        } else {
            format!(
                "Created {count} stationary opponents with 5000 HP and 100 medals each. You are Fangs; dummies are Claws. Fresh 60-second preparation. Use !feast attack on|off to control retaliation."
            )
        },
    );
}

pub fn tick(instance: &mut Instance, network: &Arc<Mutex<NetworkState>>) {
    if instance.zone.id != 745 || instance.feast_dummies.is_empty() {
        return;
    }
    let now = Instant::now();
    tick_bots(instance, network, now);
    let candidates: Vec<_> = instance
        .feast_dummies
        .iter()
        .filter(|(_, d)| !d.autonomous && d.attacking && now >= d.next_attack)
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

// Decisions use the same WHM command path as players: no bypass of casts, MP,
// crowd control, cooldowns, damage, healing, or supply ownership.
fn tick_bots(instance: &mut Instance, network: &Arc<Mutex<NetworkState>>, now: Instant) {
    let ids: Vec<_> = instance
        .feast_dummies
        .iter()
        .filter(|(_, d)| d.autonomous)
        .map(|(id, _)| *id)
        .collect();
    for id in ids {
        if !feast_match::can_act(instance, id) {
            continue;
        }
        let Some(actor) = instance.find_actor(id) else {
            continue;
        };
        let common = actor.get_common_spawn();
        let position = common.position.0;
        let job = common.class_job;
        let combat_range = if matches!(job, 19 | 34) { 4.0 } else { 20.0 };
        let dead = common.health_points == 0;
        let bot = instance.feast_dummies.get_mut(&id).unwrap();
        if dead {
            let at = *bot.respawn_at.get_or_insert(now + Duration::from_secs(10));
            if now < at {
                continue;
            }
            bot.respawn_at = None;
            bot.cast_until = now;
            bot.next_attack = now + ATTACK_INTERVAL;
            let (position, rotation) = feast_match::team_spawn(bot.team, 1);
            if let Some(NetworkedActor::Npc {
                state,
                spawn,
                status_effects,
                ..
            }) = instance.find_actor_mut(id)
            {
                *state = super::actor::NpcState::Wander;
                *status_effects = Default::default();
                spawn.common.status_effects = Default::default();
                spawn.common.health_points = spawn.common.max_health_points;
                spawn.common.resource_points = spawn.common.max_resource_points;
                spawn.common.target_id = ObjectTypeId::default();
            }
            super::set_character_mode(instance, &mut network.lock(), id, CharacterMode::Normal, 0);
            move_bot(instance, network, id, position.0, rotation);
            super::action::update_actor_hp_mp(network.clone(), instance, id);
            continue;
        }
        if !bot.attacking || now < bot.cast_until {
            continue;
        }
        let team = bot.team;
        let ready = now >= bot.next_attack;
        let moving = now >= bot.next_move;
        let runtime = instance.feast_match.as_ref().unwrap();
        let roster = &runtime.rules.players;
        let distance = |p: glam::Vec3| position.distance_squared(p);
        let nearest = |points: Vec<glam::Vec3>| {
            points
                .into_iter()
                .min_by(|a, b| distance(*a).total_cmp(&distance(*b)))
        };
        let capacity = runtime.rules.pickup_space(id.0) > 0;
        let pickup = nearest(
            runtime
                .supplies
                .kit_positions(team)
                .map(glam::Vec3::from_array)
                .chain(
                    runtime
                        .rules
                        .drops
                        .iter()
                        .filter(|d| capacity && d.team != team && d.medals > 0)
                        .map(|d| glam::Vec3::from_array(d.position)),
                )
                .collect(),
        );
        let target = runtime
            .supplies
            .boxes
            .keys()
            .copied()
            .chain(
                roster
                    .iter()
                    .filter(|(_, p)| p.team != team && p.present && p.alive)
                    .map(|(id, _)| ObjectId(*id)),
            )
            .filter_map(|target| {
                instance
                    .find_actor(target)
                    .filter(|a| a.get_common_spawn().health_points > 0)
                    .map(|a| (target, a.position().0))
            })
            .min_by(|a, b| distance(a.1).total_cmp(&distance(b.1)));
        // Emergency Cure II only. Normal pressure is Glare; no fabricated job kits.
        let heal = roster
            .iter()
            .filter(|(_, p)| job == 24 && p.team == team && p.present && p.alive)
            .filter_map(|(target, _)| {
                instance
                    .find_actor(ObjectId(*target))
                    .map(|a| (ObjectId(*target), a.get_common_spawn()))
            })
            .filter(|(_, s)| {
                s.health_points * 100 < s.max_health_points * 30
                    && distance(s.position.0) <= 25.0 * 25.0
            })
            .min_by_key(|(_, s)| s.health_points)
            .map(|(id, _)| id);
        if ready
            && (heal.is_some()
                || (pickup.is_none()
                    && target.is_some_and(|(_, p)| distance(p) <= combat_range * combat_range)))
        {
            let Some(engine) = instance.feast_whm.as_ref() else {
                continue;
            };
            let Ok((spell, target, cast_ms)) =
                engine.bot_action(id, target.map_or(id, |t| t.0), heal)
            else {
                continue;
            };
            let target = ObjectId(target);
            instance
                .find_actor_mut(id)
                .unwrap()
                .get_common_spawn_mut()
                .target_id = ObjectTypeId {
                object_id: target,
                ..Default::default()
            };
            super::feast::command(instance, network, id, &spell);
            let bot = instance.feast_dummies.get_mut(&id).unwrap();
            bot.next_attack = now + ATTACK_INTERVAL;
            bot.cast_until = now + Duration::from_millis(cast_ms + 100);
            continue;
        }
        let destination = pickup.or_else(|| target.map(|(_, p)| p));
        if !moving
            || destination.is_none()
            || instance
                .feast_whm
                .as_ref()
                .is_some_and(|f| f.blocked(id, true))
        {
            continue;
        }
        instance.feast_dummies.get_mut(&id).unwrap().next_move = now + Duration::from_millis(200);
        let destination = destination.unwrap();
        if pickup.is_none() && distance(destination) <= combat_range * combat_range {
            continue;
        }
        // Prefer the arena navmesh. The fallback is intentionally a basic direct
        // pursuit for installations without generated navigation data.
        let waypoint = if instance.navmesh.is_available() {
            instance
                .navmesh
                .calculate_path(position, destination)
                .into_iter()
                .find(|p| position.distance_squared(*p) > 0.1)
        } else {
            Some(destination)
        };
        if let Some(waypoint) = waypoint {
            let delta = waypoint - position;
            let step = delta.clamp_length_max(1.2); // 6 yalms/s, 200ms updates.
            let rotation = delta.x.atan2(delta.z);
            move_bot(instance, network, id, position + step, rotation);
        }
    }
}

fn move_bot(
    instance: &mut Instance,
    network: &Arc<Mutex<NetworkState>>,
    id: ObjectId,
    position: glam::Vec3,
    rotation: f32,
) {
    use kawari::common::{JumpState, MoveAnimationState, MoveAnimationType};
    let common = instance.find_actor_mut(id).unwrap().get_common_spawn_mut();
    common.position = Position(position);
    common.rotation = rotation;
    network.lock().send_in_range_inclusive_instance(
        id,
        instance,
        FromServer::ActorMove(
            id,
            Position(position),
            rotation,
            MoveAnimationType::empty(),
            MoveAnimationState::None,
            JumpState::NoneOrFalling,
        ),
        DestinationNetwork::ZoneClients,
    );
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
    fn bots_form_teams_cast_respawn_and_clean_up() {
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
        command(&mut instance, &network, owner, &["bots"]);
        assert_eq!(instance.feast_dummies.len(), 7);
        assert_eq!(
            instance.feast_match.as_ref().unwrap().rules.scores(),
            (400, 400)
        );
        assert_eq!(
            instance
                .feast_dummies
                .values()
                .filter(|d| d.team == feast_match::Team::Fang)
                .count(),
            3
        );
        super::super::feast::tick(&mut instance, &network);
        assert!(
            instance.feast_dummies.keys().all(|id| instance
                .feast_whm
                .as_ref()
                .unwrap()
                .enabled(*id))
        );
        let enemy = *instance
            .feast_dummies
            .iter()
            .find(|(id, d)| {
                d.team == feast_match::Team::Claw
                    && instance
                        .find_actor(**id)
                        .unwrap()
                        .get_common_spawn()
                        .class_job
                        == 24
            })
            .unwrap()
            .0;
        let initial = instance.find_actor(enemy).unwrap().position();
        tick_bots(&mut instance, &network, Instant::now());
        assert_eq!(instance.find_actor(enemy).unwrap().position(), initial); // Preparation gate.
        instance.feast_match.as_mut().unwrap().rules.phase = feast_match::Phase::Active;
        command(&mut instance, &network, owner, &["attack", "off"]);
        tick_bots(&mut instance, &network, Instant::now());
        assert_eq!(instance.find_actor(enemy).unwrap().position(), initial);
        instance.feast_dummies.get_mut(&enemy).unwrap().attacking = true;
        tick_bots(&mut instance, &network, Instant::now());
        assert!(
            instance
                .find_actor(enemy)
                .unwrap()
                .position()
                .0
                .distance(initial.0)
                > 0.5
        );
        // Isolate one bot in casting range of the player; all other bots stay paused.
        instance
            .find_actor_mut(enemy)
            .unwrap()
            .get_common_spawn_mut()
            .position = Position(glam::Vec3::new(10.0, 0.0, 0.0));
        instance.feast_dummies.get_mut(&enemy).unwrap().next_attack = Instant::now();
        tick_bots(&mut instance, &network, Instant::now());
        assert_eq!(
            instance
                .find_actor(enemy)
                .unwrap()
                .get_common_spawn()
                .target_id
                .object_id,
            owner
        );
        assert_eq!(
            instance
                .find_actor(owner)
                .unwrap()
                .get_common_spawn()
                .health_points,
            20000
        );
        std::thread::sleep(Duration::from_millis(1600));
        super::super::feast::tick(&mut instance, &network);
        assert_eq!(
            instance
                .find_actor(owner)
                .unwrap()
                .get_common_spawn()
                .health_points,
            18800
        );
        instance
            .find_actor_mut(enemy)
            .unwrap()
            .get_common_spawn_mut()
            .health_points = 0;
        feast_match::tick(&mut instance, &network);
        let now = Instant::now();
        tick_bots(&mut instance, &network, now);
        assert_eq!(
            instance
                .find_actor(enemy)
                .unwrap()
                .get_common_spawn()
                .health_points,
            0
        );
        tick_bots(&mut instance, &network, now + Duration::from_secs(10));
        assert_eq!(
            instance
                .find_actor(enemy)
                .unwrap()
                .get_common_spawn()
                .health_points,
            5000
        );
        assert_eq!(
            instance.find_actor(enemy).unwrap().position(),
            feast_match::team_spawn(feast_match::Team::Claw, 1).0
        );
        assert_eq!(
            instance.feast_match.as_ref().unwrap().rules.players[&enemy.0].medals,
            50
        );
        let ids: Vec<_> = instance.feast_dummies.keys().copied().collect();
        command(&mut instance, &network, owner, &["bots", "off"]);
        assert!(instance.feast_dummies.is_empty());
        assert!(ids.iter().all(|id| instance.find_actor(*id).is_none()));
        assert_eq!(
            instance.feast_match.as_ref().unwrap().rules.scores(),
            (100, 0)
        );
    }

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
