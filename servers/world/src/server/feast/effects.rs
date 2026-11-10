//! Applies the combat engine's events to the world.
//!
//! The Lua engine never touches packets. It records what happened as a list
//! of events (`emit(kind, actor, target, amount, message)` in Engine.lua) and
//! the server drains and applies them here, in order. HP and MP changes arrive
//! as `hp`/`mp` events with the new value; `cast`/`animation` events are only
//! presentation and never change HP themselves.
use std::sync::Arc;

use kawari::{
    common::{JumpState, MoveAnimationState, MoveAnimationType, ObjectId, ObjectTypeId, Position},
    ipc::zone::{
        ActionEffect, ActionType, ActorControlCategory, AoeActionResult, DamageElement, DamageKind,
        DamageType, ServerZoneIpcData, ServerZoneIpcSegment, TargetEffect, TargetEffectKind,
    },
};
use parking_lot::Mutex;
use serde::Deserialize;

use super::actions::{self, GCD_GROUP};
use crate::{
    FromServer,
    server::{
        actor::NetworkedActor,
        effect::send_effects_list,
        instance::Instance,
        network::{DestinationNetwork, NetworkState},
    },
};

/// One thing the combat engine wants the world to do. The meaning of
/// `amount` and `message` depends on `kind`; see `apply`.
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default)]
pub struct Event {
    pub kind: String,
    pub actor: u32,
    pub target: u32,
    pub amount: u32,
    pub healing: bool,
    pub message: String,
    /// Every target an `animation` hit, for area actions.
    pub hits: Vec<Hit>,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct Hit {
    pub target: u32,
    pub amount: u32,
    pub healing: bool,
}

const SPRINT: u16 = 50;
/// Holmgang's caster/target effects and the original bind.
const HOLMGANG_STATUSES: [u16; 3] = [88, 1304, 1305];
/// The client's action result packet holds at most eight targets.
const MAX_TARGETS: usize = 8;

pub fn apply(instance: &mut Instance, network: &Arc<Mutex<NetworkState>>, events: Vec<Event>) {
    for event in events.into_iter().flat_map(split_large_area_hits) {
        let target = ObjectId(event.target);
        match event.kind.as_str() {
            // A cast bar starting (amount = cast time in ms) or an action
            // landing (amount = total HP change, hits = every target).
            "cast" | "animation" => send_presentation(instance, network, &event),
            // An interrupted cast refunds the GCD it reserved when it started.
            "reset_gcd" => send_cooldown(network, target, GCD_GROUP, 0, 0),
            // A cooldown changed outside the normal flow (e.g. reduced by
            // another action): resend its recast to the client.
            "job_cooldown" => {
                let Some(engine) = instance.feast.combat.as_ref() else {
                    continue;
                };
                if let Some(group) = actions::cooldown_group(&event.message) {
                    match engine.recast(target, &event.message) {
                        Ok((elapsed, total)) => {
                            send_cooldown(network, target, group, elapsed, total)
                        }
                        Err(error) => {
                            tracing::warn!(%error, "Feast reduced cooldown snapshot failed")
                        }
                    }
                }
            }
            // A refused or interrupted action gives back its cooldown.
            "reset_action_cooldown" => {
                if let Some(group) = actions::cooldown_group(&event.message) {
                    send_cooldown(network, target, group, 0, 0);
                }
            }
            // Cancels the target's cast bar.
            "cancel" => {
                let action_id = instance
                    .feast
                    .combat
                    .as_mut()
                    .and_then(|engine| engine.native_requests.remove(&target))
                    .map_or(0, |request| request.action_id);
                network.lock().send_to_by_actor_id(
                    target,
                    FromServer::ActorControlSelf(super::cancel_cast(action_id)),
                    DestinationNetwork::ZoneClients,
                );
            }
            // Stops queued server-side actions (e.g. scripted NPC casts).
            "interrupt" => instance.cancel_actor_tasks(target),
            // The native icon of a Lua status (see `M.status_icons`):
            // message = status ID or "ID:param", amount = ms left (0 removes it).
            "status_visual" => {
                let (status, param) = event
                    .message
                    .split_once(':')
                    .unwrap_or((event.message.as_str(), "0"));
                let (Ok(status), Ok(param)) = (status.parse::<u16>(), param.parse::<u16>()) else {
                    continue;
                };
                if let Some(effects) = instance
                    .find_actor_mut(target)
                    .and_then(|a| a.status_effects_mut())
                {
                    if event.amount > 0 {
                        let seconds = event.amount as f32 / 1000.0;
                        effects.add(status, param, seconds);
                        effects.add_from(status, seconds, ObjectId(event.actor));
                    } else {
                        effects.remove(status);
                    }
                }
                send_effects_list(network.clone(), instance, target);
            }
            // New HP or MP value for the target.
            "hp" | "mp" => {
                if let Some(actor) = instance.find_actor_mut(target) {
                    let common = actor.get_common_spawn_mut();
                    if event.kind == "hp" {
                        common.health_points = event.amount.min(common.max_health_points);
                    } else {
                        common.resource_points =
                            event.amount.min(common.max_resource_points as u32) as u16;
                    }
                }
                crate::server::actor::update_actor_hp_mp(network.clone(), instance, target);
            }
            // Damage dealt (amount); credits supply boxes and draws NPC aggro.
            "damage" => {
                if let Some(arena) = &mut instance.feast.arena
                    && arena
                        .rules
                        .players
                        .get(&event.actor)
                        .is_some_and(|p| p.present && p.alive)
                {
                    arena
                        .supplies
                        .record_damage(event.actor, target, event.amount);
                }
                if let Some(NetworkedActor::Npc {
                    newly_hated_actor, ..
                }) = instance.find_actor_mut(target)
                {
                    *newly_hated_actor = Some(ObjectId(event.actor));
                }
                tracing::info!(
                    source = event.actor,
                    target = event.target,
                    amount = event.amount,
                    "Feast damage"
                );
            }
            "heal" => tracing::info!(
                source = event.actor,
                target = event.target,
                amount = event.amount,
                "Feast healing"
            ),
            // A chat notice for `actor`.
            "notice" => super::notice(network, ObjectId(event.actor), event.message),
            // Teleports the target to x/y/z (dashes, knockbacks, Rescue).
            "move" => {
                let position = Position(glam::Vec3A::new(event.x, event.y, event.z));
                let mut rotation = 0.0;
                if let Some(actor) = instance.find_actor_mut(target) {
                    let common = actor.get_common_spawn_mut();
                    common.position = position;
                    rotation = common.rotation;
                }
                network.lock().send_in_range_inclusive_instance(
                    target,
                    instance,
                    FromServer::ActorMove(
                        target,
                        position,
                        rotation,
                        MoveAnimationType::empty(),
                        MoveAnimationState::None,
                        JumpState::NoneOrFalling,
                    ),
                    DestinationNetwork::ZoneClients,
                );
                super::correct_position(instance, network, target, position);
            }
            // Bolt/Peloton: amount above 100 grants the client's Sprint
            // movement speed for ten seconds, 100 ends it.
            "speed" => {
                if let Some(effects) = instance
                    .find_actor_mut(target)
                    .and_then(|a| a.status_effects_mut())
                {
                    if event.amount > 100 {
                        // Param 30 is Sprint's speed bonus.
                        effects.add(SPRINT, 30, 10.0);
                    } else {
                        effects.remove(SPRINT);
                    }
                }
                send_effects_list(network.clone(), instance, target);
            }
            // Removes one native status (amount = status ID), e.g. after Purify.
            "cleanse" => remove_statuses(instance, network, target, &[event.amount as u16]),
            "remove_holmgang" => remove_statuses(instance, network, target, &HOLMGANG_STATUSES),
            _ => tracing::warn!(?event, "Unknown Feast effect"),
        }
    }
}

/// Supplies may bring an area action above the packet's eight targets;
/// send those as several animations instead of dropping hits.
fn split_large_area_hits(event: Event) -> Vec<Event> {
    if event.kind == "animation" && event.hits.len() > MAX_TARGETS {
        event
            .hits
            .chunks(MAX_TARGETS)
            .map(|hits| Event {
                hits: hits.to_vec(),
                ..event.clone()
            })
            .collect()
    } else {
        vec![event]
    }
}

fn send_cooldown(
    network: &Arc<Mutex<NetworkState>>,
    actor: ObjectId,
    group: u32,
    elapsed: u32,
    total: u32,
) {
    network.lock().send_to_by_actor_id(
        actor,
        FromServer::ActorControlSelf(ActorControlCategory::SetCooldownTimer {
            cooldown_group: group,
            unk1: elapsed,
            unk2: total,
        }),
        DestinationNetwork::ZoneClients,
    );
}

fn remove_statuses(
    instance: &mut Instance,
    network: &Arc<Mutex<NetworkState>>,
    target: ObjectId,
    statuses: &[u16],
) {
    if let Some(effects) = instance
        .find_actor_mut(target)
        .and_then(|a| a.status_effects_mut())
    {
        for status in statuses {
            effects.remove(*status);
        }
    }
    send_effects_list(network.clone(), instance, target);
}

/// Sends a cast bar or an action result to everyone nearby.
///
/// If the action came from the player's own hotbar, the result also
/// acknowledges that request (same action ID and sequence), so the client
/// ends its cast and starts the right recast instead of rolling it back.
fn send_presentation(instance: &mut Instance, network: &Arc<Mutex<NetworkState>>, event: &Event) {
    let source = ObjectId(event.actor);
    let Some(mut packet) = presentation(instance, event) else {
        return;
    };
    let mut network = network.lock();
    let requests = instance
        .feast
        .combat
        .as_mut()
        .map(|engine| &mut engine.native_requests);
    match &mut packet {
        ServerZoneIpcData::ActorCast { action_id, .. } => {
            if let Some(request) = requests.and_then(|r| r.get(&source)) {
                *action_id = request.action_id;
            }
        }
        ServerZoneIpcData::ActionEffect1(result) => {
            if let Some(request) = requests.and_then(|r| r.remove(&source)) {
                result.source_sequence = request.sequence;
                result.action_id = request.action_id;
                result.action_type = request.action_type;
            }
            result.global_sequence = network.global_action_sequence;
            network.global_action_sequence = network.global_action_sequence.wrapping_add(1);
        }
        ServerZoneIpcData::AoeEffect8(result) => {
            if let Some(request) = requests.and_then(|r| r.remove(&source)) {
                result.source_sequence = request.sequence;
                result.action_id = request.action_id;
                result.action_type = request.action_type;
            }
            result.global_sequence = network.global_action_sequence;
            network.global_action_sequence = network.global_action_sequence.wrapping_add(1);
        }
        _ => {}
    }
    network.send_in_range_inclusive_instance(
        source,
        instance,
        FromServer::PacketSegment(ServerZoneIpcSegment::new(packet), source),
        DestinationNetwork::ZoneClients,
    );
}

/// Builds the packet for a `cast` or `animation` event, or `None` if the
/// action or its caster is unknown.
///
/// `spell_id` always names the historical action, which selects the client's
/// cast/hit animation and VFX. `action_id` starts the same and is replaced by
/// the hotbar request's ID when one exists (see `send_presentation`), e.g.
/// Raiden Thrust's button for a resolved Full Thrust.
pub(super) fn presentation(instance: &Instance, event: &Event) -> Option<ServerZoneIpcData> {
    let action_id = actions::by_key(&event.message)?.id;
    let actor = instance.find_actor(ObjectId(event.actor))?;
    let common = actor.get_common_spawn();
    let target = ObjectTypeId {
        object_id: ObjectId(event.target),
        ..Default::default()
    };
    if event.kind == "cast" {
        // Ground-targeted casts aim at their point rather than the caster.
        let position = if matches!(event.message.as_str(), "soten" | "cometeor") {
            Position(glam::Vec3A::new(event.x, event.y, event.z))
        } else {
            actor.position()
        };
        return Some(ServerZoneIpcData::ActorCast {
            spell_id: action_id as u16,
            action_id,
            action_type: ActionType::Action,
            omen_delay: 0,
            cast_time: event.amount as f32 / 1000.0,
            target: target.object_id,
            rotation: common.rotation,
            interruptible: false,
            ballista_entity_id: ObjectId::default(),
            position,
        });
    }
    let is_area =
        event.hits.len() > 1 || event.hits.first().is_some_and(|h| h.target != event.target);
    if is_area {
        let mut result = AoeActionResult {
            animation_target_id: target,
            action_id,
            spell_id: action_id as u16,
            action_type: ActionType::Action,
            animation_lock: kawari::common::ANIMATION_LOCK_TIME,
            rotation: common.rotation,
            position: actor.position(),
            effect_count: event.hits.len().min(MAX_TARGETS) as u8,
            ..Default::default()
        };
        for (i, hit) in event.hits.iter().take(MAX_TARGETS).enumerate() {
            result.effects[i] = target_effects(hit.amount, hit.healing);
            result.target_ids[i] = ObjectTypeId {
                object_id: ObjectId(hit.target),
                ..Default::default()
            };
        }
        return Some(ServerZoneIpcData::AoeEffect8(result));
    }
    Some(ServerZoneIpcData::ActionEffect1(ActionEffect {
        animation_target_id: target,
        targets: vec![target],
        action_id,
        spell_id: action_id as u16,
        action_type: ActionType::Action,
        animation_lock: kawari::common::ANIMATION_LOCK_TIME,
        rotation: common.rotation,
        effects: target_effects(event.amount, event.healing),
        // Presentation only: HP/MP were already applied by the rules engine.
        ..Default::default()
    }))
}

/// The flytext for one target: a heal or magic damage number.
fn target_effects(amount: u32, healing: bool) -> [TargetEffect; 8] {
    let mut effects = [TargetEffect::default(); 8];
    let amount = amount.min(u16::MAX as u32) as u16;
    if amount > 0 {
        effects[0].0 = if healing {
            TargetEffectKind::Heal {
                unk1: [0; 5],
                amount,
            }
        } else {
            TargetEffectKind::Damage {
                damage_kind: DamageKind::default(),
                damage_type: DamageType::Magic,
                damage_element: DamageElement::Unaspected,
                bonus_percent: 0,
                unk3: 0,
                unk4: 0,
                amount,
            }
        };
    }
    effects
}
