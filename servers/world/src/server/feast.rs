//! Territory-scoped Feast White Mage rules and native action routing.
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_requests_enable_arena_rules_and_keep_cast_sequence() {
        let lua = Lua::new();
        let table: Table = lua
            .load(include_str!(
                "../../../../resources/scripts/feast/WhiteMage.lua"
            ))
            .eval()
            .unwrap();
        lua.globals().set("feast", table).unwrap();
        let mut instance = Instance::default();
        instance.zone.id = 745;
        instance.feast_whm = Some(FeastWhiteMage {
            lua,
            started: Instant::now(),
            native_requests: HashMap::new(),
        });
        let id = ObjectId(1);
        instance.insert_empty_actor(id);
        let common = instance.find_actor_mut(id).unwrap().get_common_spawn_mut();
        common.class_job = 24;
        common.health_points = 1000;
        common.max_health_points = 20000;
        common.resource_points = 10000;
        common.max_resource_points = 10000;
        let network = Arc::new(Mutex::new(NetworkState::default()));
        let request = kawari::ipc::zone::ActionRequest {
            action_id: 135,
            action_type: ActionType::Action,
            sequence: 42,
            target: kawari::common::ObjectTypeId {
                object_id: id,
                ..Default::default()
            },
            ..Default::default()
        };
        native_action(&mut instance, &network, id, &request, Some(57));
        let engine = instance.feast_whm.as_ref().unwrap();
        assert!(engine.enabled(id));
        assert_eq!(engine.party_actions(id.0).unwrap(), vec![(1584, 0), (17832, 0), (18957, 0)]);
        assert_eq!(engine.native_requests[&id].sequence, 42);
        assert_eq!(
            instance
                .find_actor(id)
                .unwrap()
                .get_common_spawn()
                .health_points,
            1000
        );
        let mut denied = request.clone();
        denied.sequence = 43;
        native_action(&mut instance, &network, id, &denied, Some(57));
        let engine = instance.feast_whm.as_ref().unwrap();
        assert_eq!(engine.native_requests[&id].sequence, 42);
        engine.function("advance").unwrap().call::<()>(2.0).unwrap();
        let events = engine.drain().unwrap();
        apply(&mut instance, &network, events);
        assert!(
            instance
                .feast_whm
                .as_ref()
                .unwrap()
                .native_requests
                .is_empty()
        );
        assert!(
            instance
                .find_actor(id)
                .unwrap()
                .get_common_spawn()
                .health_points
                > 1000
        );
        assert_eq!(native_action_name(25859), Some("glare"));
        assert_eq!(native_action_name(u32::MAX), None);
    }
    #[test]
    fn feast_presentation_uses_current_actions_without_mutating_hp() {
        let mut instance = Instance::default();
        instance.insert_empty_actor(ObjectId(1));
        let hp = instance
            .find_actor(ObjectId(1))
            .unwrap()
            .get_common_spawn()
            .health_points;
        let mut event = Event {
            kind: "cast".into(),
            actor: 1,
            target: 2,
            amount: 1500,
            message: "glare".into(),
            ..Default::default()
        };
        assert!(matches!(
            presentation(&instance, &event),
            Some(ServerZoneIpcData::ActorCast {
                action_id: 16533,
                cast_time: 1.5,
                target: ObjectId(2),
                ..
            })
        ));
        event.kind = "animation".into();
        event.message = "cure2".into();
        event.amount = 4000;
        event.healing = true;
        let Some(ServerZoneIpcData::ActionResult(result)) = presentation(&instance, &event) else {
            panic!("Expected action result")
        };
        assert_eq!(result.action_id, 135);
        assert_eq!(result.source_sequence, 0);
        assert_eq!(result.target_id_again.object_id, ObjectId(2));
        assert!(matches!(
            result.effects[0].kind,
            EffectKind::Heal { amount: 4000, .. }
        ));
        assert_eq!(
            instance
                .find_actor(ObjectId(1))
                .unwrap()
                .get_common_spawn()
                .health_points,
            hp
        );
        event.message = "unknown".into();
        assert!(presentation(&instance, &event).is_none());
    }
    #[test]
    fn feast_white_mage_rules() {
        let lua = mlua::Lua::new();
        lua.globals()
            .set(
                "make_engine",
                lua.load(include_str!(
                    "../../../../resources/scripts/feast/WhiteMage.lua"
                ))
                .into_function()
                .unwrap(),
            )
            .unwrap();
        let count: u32 = lua
            .load(include_str!(
                "../../../../resources/tests/feast_white_mage.lua"
            ))
            .eval()
            .unwrap();
        assert!(count >= 25);
    }

    #[test]
    fn feast_adapter_applies_hp_mp_and_damage_modifiers() {
        let lua = Lua::new();
        let table: Table = lua
            .load(include_str!(
                "../../../../resources/scripts/feast/WhiteMage.lua"
            ))
            .eval()
            .unwrap();
        lua.globals().set("feast", table).unwrap();
        let mut instance = Instance::default();
        instance.zone.id = 745;
        instance.feast_whm = Some(FeastWhiteMage {
            lua,
            started: Instant::now(),
            native_requests: HashMap::new(),
        });
        for id in [ObjectId(1), ObjectId(2)] {
            instance.insert_empty_actor(id);
            let common = instance.find_actor_mut(id).unwrap().get_common_spawn_mut();
            common.class_job = 24;
            common.health_points = 1000;
            common.max_health_points = 20000;
            common.resource_points = 10000;
            common.max_resource_points = 10000;
        }
        instance
            .find_actor_mut(ObjectId(1))
            .unwrap()
            .get_common_spawn_mut()
            .target_id
            .object_id = ObjectId(2);
        let network = Arc::new(Mutex::new(NetworkState::default()));
        command(&mut instance, &network, ObjectId(1), "on");
        command(&mut instance, &network, ObjectId(2), "on");
        command(&mut instance, &network, ObjectId(1), "tetra");
        assert_eq!(
            instance
                .find_actor(ObjectId(2))
                .unwrap()
                .get_common_spawn()
                .health_points,
            5000
        );
        command(&mut instance, &network, ObjectId(1), "protect");
        assert_eq!(
            adjust_damage(&instance, ObjectId(3), ObjectId(2), 1000),
            800
        );
        apply(
            &mut instance,
            &network,
            vec![Event {
                kind: "mp".into(),
                actor: 1,
                target: 1,
                amount: 9000,
                ..Default::default()
            }],
        );
        assert_eq!(
            instance
                .find_actor(ObjectId(1))
                .unwrap()
                .get_common_spawn()
                .resource_points,
            9000
        );
        // GM death can leave positive HP in the underlying actor. It must not
        // make a corpse a legal healing target in the Feast engine.
        instance
            .find_actor_mut(ObjectId(2))
            .unwrap()
            .get_common_spawn_mut()
            .mode = CharacterMode::Dead;
        command(&mut instance, &network, ObjectId(1), "tetra");
        assert_eq!(
            instance
                .find_actor(ObjectId(2))
                .unwrap()
                .get_common_spawn()
                .health_points,
            5000
        );
    }
}

use std::{collections::HashMap, sync::Arc, time::Instant};

use kawari::{
    common::{CharacterMode, ObjectId, Position},
    config::get_config,
    ipc::zone::{
        ActionEffect, ActionResult, ActionType, ActorControlCategory, ActorSetPos, DamageElement,
        DamageKind, DamageType, EffectKind, ServerNoticeMessage, ServerZoneIpcData,
        ServerZoneIpcSegment, WarpType,
    },
};
use mlua::{Function, Lua, LuaSerdeExt, Table};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use super::{
    actor::NetworkedActor,
    instance::Instance,
    network::{DestinationNetwork, NetworkState},
};
use crate::FromServer;

#[derive(Debug)]
pub struct FeastWhiteMage {
    lua: Lua,
    started: Instant,
    native_requests: HashMap<ObjectId, kawari::ipc::zone::ActionRequest>,
}

#[derive(Serialize)]
struct Snapshot {
    id: u32,
    hp: u32,
    max_hp: u32,
    mp: u16,
    x: f32,
    y: f32,
    z: f32,
    player: bool,
    job: u8,
    combat: bool,
    invulnerable: bool,
    native_statuses: Vec<u16>,
    match_team: Option<&'static str>,
    match_active: bool,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Event {
    kind: String,
    actor: u32,
    target: u32,
    amount: u32,
    healing: bool,
    message: String,
    x: f32,
    y: f32,
    z: f32,
}

impl FeastWhiteMage {
    fn new() -> mlua::Result<Self> {
        let lua = Lua::new();
        let path = get_config()
            .filesystem
            .locate_script_file("feast/WhiteMage.lua");
        let source = std::fs::read(&path).map_err(mlua::Error::external)?;
        let engine: Table = lua.load(source).set_name(path).eval()?;
        lua.globals().set("feast", engine)?;
        Ok(Self {
            lua,
            started: Instant::now(),
            native_requests: HashMap::new(),
        })
    }

    fn function(&self, name: &str) -> mlua::Result<Function> {
        self.lua.globals().get::<Table>("feast")?.get(name)
    }

    fn now(&self) -> f64 {
        self.started.elapsed().as_secs_f64()
    }

    fn sync(&self, snapshots: &[Snapshot]) -> mlua::Result<()> {
        self.function("sync")?
            .call((self.lua.to_value(snapshots)?, self.now()))
    }

    fn drain(&self) -> mlua::Result<Vec<Event>> {
        self.lua.from_value(self.function("drain")?.call(())?)
    }

    pub fn party_actions(&self, id: u32) -> mlua::Result<Vec<(u16, u16)>> {
        let rows: Table = self.function("party_actions")?.call((id, self.now()))?;
        rows.sequence_values::<Table>().map(|row| {
            let row = row?;
            Ok((row.get(1)?, row.get(2)?))
        }).collect()
    }

    pub fn enabled(&self, id: ObjectId) -> bool {
        self.function("enabled")
            .and_then(|f| f.call(id.0))
            .unwrap_or(false)
    }

    pub fn movement(&self, id: ObjectId, position: Position) {
        if let Err(error) = self.function("movement").and_then(|f| {
            f.call::<()>((id.0, self.now(), position.0.x, position.0.y, position.0.z))
        }) {
            tracing::error!(%error, "Feast cast movement check failed");
        }
    }

    pub fn blocked(&self, id: ObjectId, movement: bool) -> bool {
        self.function("blocked")
            .and_then(|f| f.call((id.0, movement)))
            .unwrap_or(false)
    }
}

fn snapshots(instance: &Instance) -> Vec<Snapshot> {
    instance
        .actors
        .iter()
        .filter_map(|(id, actor)| {
            let (player, invulnerable) = match actor {
                NetworkedActor::Player { .. } => (true, false),
                NetworkedActor::Npc {
                    currently_invulnerable,
                    ..
                } => (false, *currently_invulnerable),
                _ => return None,
            };
            let s = actor.get_common_spawn();
            let combat = instance.actors.values().any(|enemy| {
                matches!(enemy,
            NetworkedActor::Npc { navmesh_target: Some(target), spawn, .. }
                if target == id && spawn.common.health_points > 0)
            });
            Some(Snapshot {
                id: id.0,
                hp: if s.mode == CharacterMode::Dead {
                    0
                } else {
                    s.health_points
                },
                max_hp: s.max_health_points,
                mp: s.resource_points,
                x: s.position.0.x,
                y: s.position.0.y,
                z: s.position.0.z,
                player,
                job: s.class_job,
                combat,
                invulnerable,
                match_team: instance
                    .feast_match
                    .as_ref()
                    .and_then(|m| m.rules.players.get(&id.0))
                    .map(|p| p.team.name()),
                match_active: !player || super::feast_match::can_act(instance, *id),
                native_statuses: actor
                    .status_effects()
                    .map(|effects| effects.data().iter().map(|s| s.effect_id).collect())
                    .unwrap_or_default(),
            })
        })
        .collect()
}

pub fn notice(network: &Arc<Mutex<NetworkState>>, id: ObjectId, message: String) {
    if message.is_empty() {
        return;
    }
    network.lock().send_to_by_actor_id(
        id,
        FromServer::PacketSegment(
            ServerZoneIpcSegment::new(ServerZoneIpcData::ServerNoticeMessage(
                ServerNoticeMessage {
                    message,
                    ..Default::default()
                },
            )),
            id,
        ),
        DestinationNetwork::ZoneClients,
    );
}

fn apply(instance: &mut Instance, network: &Arc<Mutex<NetworkState>>, events: Vec<Event>) {
    for event in events {
        let target = ObjectId(event.target);
        match event.kind.as_str() {
            "cast" | "animation" => {
                let source = ObjectId(event.actor);
                if let Some(packet) = presentation(instance, &event) {
                    let mut network = network.lock();
                    let mut packet = packet;
                    if let ServerZoneIpcData::ActorCast {
                        action_id,
                        spell_id,
                        ..
                    } = &mut packet
                        && let Some(request) = instance
                            .feast_whm
                            .as_ref()
                            .and_then(|engine| engine.native_requests.get(&source))
                    {
                        *action_id = request.action_id;
                        *spell_id = request.action_id as u16;
                    }
                    if let ServerZoneIpcData::ActionResult(ref mut result) = packet {
                        if let Some(request) = instance
                            .feast_whm
                            .as_mut()
                            .and_then(|engine| engine.native_requests.remove(&source))
                        {
                            result.source_sequence = request.sequence;
                            result.action_id = request.action_id;
                            result.spell_id = request.action_id as u16;
                            result.action_type = request.action_type;
                        }
                        result.global_sequence = network.global_action_sequence;
                        network.global_action_sequence =
                            network.global_action_sequence.wrapping_add(1);
                    }
                    network.send_in_range_inclusive_instance(
                        source,
                        instance,
                        FromServer::PacketSegment(ServerZoneIpcSegment::new(packet), source),
                        DestinationNetwork::ZoneClients,
                    );
                }
            }
            "reset_gcd" => {
                // Zero-based normal spell GCD group (Action cooldown group 58).
                // Only genuine interrupted casts emit this, not rejected inputs.
                network.lock().send_to_by_actor_id(
                    target,
                    FromServer::ActorControlSelf(ActorControlCategory::SetCooldownTimer {
                        cooldown_group: 57, unk1: 0, unk2: 0,
                    }),
                    DestinationNetwork::ZoneClients,
                );
            }
            "cancel" => {
                if let Some(engine) = instance.feast_whm.as_mut() {
                    engine.native_requests.remove(&target);
                }
                network.lock().send_to_by_actor_id(
                    target,
                    FromServer::ActorControlSelf(ActorControlCategory::CancelCast {}),
                    DestinationNetwork::ZoneClients,
                );
            }
            "interrupt" => instance.cancel_actor_tasks(target),
            "temperance_visual" => {
                if let Some(effects) = instance
                    .find_actor_mut(target)
                    .and_then(|actor| actor.status_effects_mut())
                {
                    if event.amount > 0 {
                        effects.add(1872, 0, event.amount as f32);
                    } else {
                        effects.remove(1872);
                    }
                }
                super::effect::send_effects_list(network.clone(), instance, target);
            }
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
                super::action::update_actor_hp_mp(network.clone(), instance, target);
            }
            "damage" => {
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
            "notice" => notice(network, ObjectId(event.actor), event.message),
            "move" => {
                let position = Position(glam::Vec3::new(event.x, event.y, event.z));
                if let Some(actor) = instance.find_actor_mut(target) {
                    actor.get_common_spawn_mut().position = position;
                }
                correct_position(instance, network, target, position);
            }
            "speed" | "cleanse" | "remove_holmgang" => {
                if let Some(effects) = instance
                    .find_actor_mut(target)
                    .and_then(|a| a.status_effects_mut())
                {
                    match event.kind.as_str() {
                        // Use the client's Sprint movement effect for Bolt's ten-second buff.
                        "speed" if event.amount > 100 => effects.add(50, 30, 10.0),
                        "speed" => effects.remove(50),
                        "cleanse" => effects.remove(event.amount as u16),
                        "remove_holmgang" => {
                            // Historical PvP caster/target effects, plus the original bind.
                            for id in [88, 1304, 1305] {
                                effects.remove(id);
                            }
                        }
                        _ => {}
                    }
                }
                super::effect::send_effects_list(network.clone(), instance, target);
            }
            _ => tracing::warn!(?event, "Unknown Feast effect"),
        }
    }
}

/// Existing client actions supply presentation only; Lua remains authoritative.
fn animation_id(name: &str) -> Option<u32> {
    Some(match name {
        "cure" => 120,
        "cure2" => 135,
        "medica" => 124,
        "glare" => 16533,
        "dia" => 16532,
        "solace" => 16531,
        "rapture" => 16534,
        "misery" => 16535,
        "tetra" => 3570,
        "assize" => 3571,
        "temperance" => 16536,
        "purify" => 7568,
        "bolt" => 3,
        "kit" => 140,
        "repose" => 16560,
        "protect" => 7432,
        "rescue" => 7571,
        "largesse" => 25861,
        "attunement" => 7433,
        "rain" => 208,
        _ => return None,
    })
}

fn presentation(instance: &Instance, event: &Event) -> Option<ServerZoneIpcData> {
    let action_id = animation_id(&event.message)?;
    let actor = instance.find_actor(ObjectId(event.actor))?;
    let common = actor.get_common_spawn();
    let target = kawari::common::ObjectTypeId {
        object_id: ObjectId(event.target),
        ..Default::default()
    };
    Some(if event.kind == "cast" {
        ServerZoneIpcData::ActorCast {
            spell_id: action_id as u16,
            action_id,
            action_type: ActionType::Action,
            omen_delay: 0,
            cast_time: event.amount as f32 / 1000.0,
            target: target.object_id,
            rotation: common.rotation,
            interruptible: false,
            ballista_entity_id: ObjectId::default(),
            position: actor.position(),
        }
    } else {
        let mut effects = [ActionEffect::default(); 8];
        let amount = event.amount.min(u16::MAX as u32) as u16;
        if amount > 0 {
            effects[0].kind = if event.healing {
                EffectKind::Heal {
                    unk1: [0; 5],
                    amount,
                }
            } else {
                EffectKind::Damage {
                    damage_kind: DamageKind::Normal,
                    damage_type: DamageType::Magic,
                    damage_element: DamageElement::Unaspected,
                    bonus_percent: 0,
                    unk3: 0,
                    unk4: 0,
                    amount,
                }
            };
        }
        ServerZoneIpcData::ActionResult(ActionResult {
            animation_target_id: target,
            target_id_again: target,
            action_id,
            spell_id: action_id as u16,
            action_type: ActionType::Action,
            animation_lock: kawari::common::ANIMATION_LOCK_TIME,
            rotation: common.rotation,
            effect_count: 1,
            effects,
            // Presentation only: HP/MP were already applied by the rules engine.
            // Commands/plugin casts have no native ActionRequest sequence to acknowledge.
            ..Default::default()
        })
    })
}

pub(super) fn native_action_name(id: u32) -> Option<&'static str> {
    // Include upgraded actions selected automatically by the current client.
    match id {
        119 | 127 | 3568 | 7431 | 25859 => Some("glare"),
        121 | 132 => Some("dia"),
        133 | 37010 => Some("medica"),
        _ => [
            "cure",
            "cure2",
            "medica",
            "glare",
            "dia",
            "solace",
            "rapture",
            "misery",
            "tetra",
            "assize",
            "temperance",
            "purify",
            "bolt",
            "kit",
            "repose",
            "protect",
            "rescue",
            "largesse",
            "attunement",
            "rain",
        ]
        .into_iter()
        .find(|name| animation_id(name) == Some(id)),
    }
}

pub fn native_action(
    instance: &mut Instance,
    network: &Arc<Mutex<NetworkState>>,
    actor: ObjectId,
    request: &kawari::ipc::zone::ActionRequest,
    cooldown_group: Option<u32>,
) {
    if instance.zone.id != 745 {
        return;
    }
    if !super::feast_match::can_act(instance, actor) {
        notice(network, actor, "Feast combat is closed: wait for the countdown, or join the next match. Jobs are locked during a match.".into());
        network.lock().send_to_by_actor_id(
            actor,
            FromServer::ActorControlSelf(ActorControlCategory::CancelCast {}),
            DestinationNetwork::ZoneClients,
        );
        return;
    }
    tick(instance, network);
    let name = (request.action_type == ActionType::Action)
        .then(|| native_action_name(request.action_id))
        .flatten();
    let result = (|| -> mlua::Result<(String, Vec<Event>)> {
        let engine = instance
            .feast_whm
            .as_ref()
            .ok_or_else(|| mlua::Error::external("Feast currently supports White Mage only."))?;
        let name =
            name.ok_or_else(|| mlua::Error::external("This action is not in the Feast kit."))?;
        if !engine.enabled(actor) {
            return Err(mlua::Error::external(
                "Equip White Mage to use the Feast kit.",
            ));
        }
        if instance
            .find_actor(actor)
            .is_none_or(|a| a.get_common_spawn().class_job != 24)
        {
            return Err(mlua::Error::external(
                "Feast currently supports White Mage only.",
            ));
        }
        engine.sync(&snapshots(instance))?;
        let target = if request.target.object_id.0 == 0xE0000000 {
            0
        } else {
            request.target.object_id.0
        };
        let response = engine
            .function("command")?
            .call((actor.0, name, target, engine.now()))?;
        Ok((response, engine.drain()?))
    })();
    match result {
        Ok((response, events)) => {
            let accepted = events.iter().any(|event| {
                event.actor == actor.0 && matches!(event.kind.as_str(), "cast" | "animation")
            });
            if accepted {
                instance
                    .feast_whm
                    .as_mut()
                    .unwrap()
                    .native_requests
                    .insert(actor, request.clone());
                if let Some(group) = cooldown_group {
                    let engine = instance.feast_whm.as_ref().unwrap();
                    match engine
                        .function("recast")
                        .and_then(|f| f.call::<(u32, u32)>((actor.0, name.unwrap(), engine.now())))
                    {
                        Ok((elapsed, total)) => network.lock().send_to_by_actor_id(
                            actor,
                            FromServer::ActorControlSelf(ActorControlCategory::SetCooldownTimer {
                                cooldown_group: group,
                                unk1: elapsed,
                                unk2: total,
                            }),
                            DestinationNetwork::ZoneClients,
                        ),
                        Err(error) => {
                            tracing::error!(%error, "Feast recast synchronization failed")
                        }
                    }
                }
            } else {
                network.lock().send_to_by_actor_id(
                    actor,
                    FromServer::ActorControlSelf(ActorControlCategory::CancelCast {}),
                    DestinationNetwork::ZoneClients,
                );
            }
            apply(instance, network, events);
            notice(network, actor, response);
        }
        Err(error) => {
            notice(network, actor, error.to_string());
            network.lock().send_to_by_actor_id(
                actor,
                FromServer::ActorControlSelf(ActorControlCategory::CancelCast {}),
                DestinationNetwork::ZoneClients,
            );
        }
    }
}

pub fn command(
    instance: &mut Instance,
    network: &Arc<Mutex<NetworkState>>,
    actor: ObjectId,
    text: &str,
) {
    if instance.zone.id != 745 {
        notice(
            network,
            actor,
            "Feast White Mage is only available in territory 745.".into(),
        );
        return;
    }
    if text.trim().eq_ignore_ascii_case("off") {
        notice(
            network,
            actor,
            "Feast rules apply automatically in this arena. Leave the arena for normal actions."
                .into(),
        );
        return;
    }
    if instance.feast_whm.is_none() {
        match FeastWhiteMage::new() {
            Ok(engine) => instance.feast_whm = Some(engine),
            Err(error) => {
                notice(
                    network,
                    actor,
                    format!("Could not load Feast rules: {error}"),
                );
                return;
            }
        }
    }
    let target = instance
        .find_actor(actor)
        .map(|a| a.get_common_spawn().target_id.object_id.0)
        .unwrap_or(0);
    let result = (|| -> mlua::Result<(String, Vec<Event>)> {
        let engine = instance.feast_whm.as_ref().unwrap();
        engine.sync(&snapshots(instance))?;
        let response = engine
            .function("command")?
            .call((actor.0, text, target, engine.now()))?;
        Ok((response, engine.drain()?))
    })();
    match result {
        Ok((response, events)) => {
            apply(instance, network, events);
            notice(network, actor, response);
        }
        Err(error) => {
            tracing::error!(%error,"Feast command failed");
            notice(network, actor, format!("Feast command failed: {error}"));
        }
    }
}

pub fn tick(instance: &mut Instance, network: &Arc<Mutex<NetworkState>>) {
    if instance.zone.id != 745 {
        return;
    }
    let players: Vec<_> = snapshots(instance)
        .into_iter()
        .filter(|actor| actor.player && actor.job == 24)
        .map(|actor| ObjectId(actor.id))
        .collect();
    for actor in players {
        if !instance
            .feast_whm
            .as_ref()
            .is_some_and(|engine| engine.enabled(actor))
        {
            command(instance, network, actor, "on");
        }
    }
    let Some(engine) = instance.feast_whm.as_ref() else {
        return;
    };
    let result = (|| -> mlua::Result<Vec<Event>> {
        engine.sync(&snapshots(instance))?;
        engine.function("advance")?.call::<()>(engine.now())?;
        engine.drain()
    })();
    match result {
        Ok(events) => apply(instance, network, events),
        Err(error) => tracing::error!(%error,"Feast tick failed"),
    }
}

pub fn adjust_damage(instance: &Instance, source: ObjectId, target: ObjectId, amount: u16) -> u16 {
    let Some(engine) = instance.feast_whm.as_ref() else {
        return amount;
    };
    engine
        .function("modify_damage")
        .and_then(|f| f.call((source.0, target.0, amount, engine.now())))
        .unwrap_or_else(|error| {
            tracing::error!(%error,"Feast damage modifier failed");
            amount
        })
}

pub fn correct_position(
    instance: &Instance,
    network: &Arc<Mutex<NetworkState>>,
    id: ObjectId,
    position: Position,
) {
    let rotation = instance
        .find_actor(id)
        .map(|a| a.get_common_spawn().rotation)
        .unwrap_or_default();
    network.lock().send_to_by_actor_id(
        id,
        FromServer::PacketSegment(
            ServerZoneIpcSegment::new(ServerZoneIpcData::ActorSetPos(ActorSetPos {
                position,
                rotation,
                warp_type: WarpType::None,
                ..Default::default()
            })),
            id,
        ),
        DestinationNetwork::ZoneClients,
    );
}
