//! Territory-scoped Feast White Mage rules and native action routing.
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn assize_packet_contains_each_hit_and_dia_has_native_status() {
        let mut instance=Instance::default();
        instance.insert_empty_actor(ObjectId(1)); instance.insert_empty_actor(ObjectId(2));
        let event=Event { kind:"animation".into(), message:"assize".into(),actor:1,target:1,
            hits:vec![Hit{target:2,amount:1000,healing:false},Hit{target:1,amount:2000,healing:true}],..Default::default() };
        let Some(ServerZoneIpcData::AoeEffect8(result))=presentation(&instance,&event) else { panic!("missing area result") };
        assert_eq!(result.effect_count,2); assert_eq!(result.spell_id,9620);
        assert_eq!(result.target_ids[0].object_id,ObjectId(2));
        assert!(matches!(result.effects[0][0].0,TargetEffectKind::Damage{amount:1000,..}));
        assert!(matches!(result.effects[1][0].0,TargetEffectKind::Heal{amount:2000,..}));
        let network=Arc::new(Mutex::new(NetworkState::default()));
        apply(&mut instance,&network,vec![Event{kind:"dia_visual".into(),actor:1,target:2,amount:15,..Default::default()}]);
        let effect=instance.find_actor(ObjectId(2)).unwrap().status_effects().unwrap().get(2035).unwrap();
        assert_eq!(effect.duration,15.0); assert_eq!(effect.source_actor_id,ObjectId(1));
        apply(&mut instance,&network,vec![Event{kind:"dia_visual".into(),actor:1,target:2,amount:0,..Default::default()}]);
        assert!(instance.find_actor(ObjectId(2)).unwrap().status_effects().unwrap().get(2035).is_none());
        apply(&mut instance,&network,vec![Event{kind:"control_visual".into(),actor:1,target:2,amount:5000,message:"1344".into(),..Default::default()}]);
        let heavy=instance.find_actor(ObjectId(2)).unwrap().status_effects().unwrap().get(1344).unwrap();
        assert_eq!(heavy.duration,5.0); assert_eq!(heavy.param,50); assert_eq!(heavy.source_actor_id,ObjectId(1));
        apply(&mut instance,&network,vec![Event{kind:"control_visual".into(),actor:1,target:2,message:"1344".into(),..Default::default()}]);
        assert!(instance.find_actor(ObjectId(2)).unwrap().status_effects().unwrap().get(1344).is_none());
    }
    #[test]
    fn historical_jobs_rules_and_native_ids() {
        let lua = Lua::new();
        lua.globals().set("make_engine", lua.load(include_str!("../../../../resources/scripts/feast/WhiteMage.lua")).into_function().unwrap()).unwrap();
        lua.globals().set("base_jobs", lua.load(include_str!("../../../../resources/scripts/feast/Jobs.lua")).eval::<Function>().unwrap()).unwrap();
        lua.globals().set("extra_jobs", lua.load(include_str!("../../../../resources/scripts/feast/AdditionalJobs.lua")).eval::<Function>().unwrap()).unwrap();
        lua.globals().set("extra_data", lua.load(include_str!("../../../../resources/scripts/feast/AdditionalActionData.lua")).eval::<Table>().unwrap()).unwrap();
        lua.load("function install_jobs(m) base_jobs(m); extra_jobs(m,extra_data) end").exec().unwrap();
        let checks: u32 = lua.load(include_str!("../../../../resources/tests/feast_jobs.lua")).eval().unwrap();
        assert!(checks > 100);
        let all_checks: u32 = lua.load(include_str!("../../../../resources/tests/feast_all_jobs.lua")).eval().unwrap();
        assert!(all_checks > 500);
        let engine = FeastWhiteMage::new().unwrap();
        let actions: Table = engine.lua.globals().get::<Table>("feast").unwrap().get("actions").unwrap();
        for action in job_actions() {
            let definition: Table = actions.get(action.key.as_str()).unwrap();
            assert_eq!(definition.get::<u32>("id").unwrap(), action.id);
            assert_eq!(animation_id(&action.key), Some(action.id));
        }
        engine.lua.load("feast.sync({{id=1,job=25,player=true,hp=10000,max_hp=10000,mp=10000,x=0,y=0,z=0}},0); feast.command(1,'on',0,0)").exec().unwrap();
        assert!(engine.hotbar(1).unwrap().iter().any(|row| *row == (8862,8862,0,2400)));
        assert_eq!(engine.job_gauge(1).unwrap()[0],25);
        engine.lua.load("feast.sync({{id=1,job=24,player=true,hp=10000,max_hp=10000,mp=10000,x=0,y=0,z=0,combat=true,match_active=true}},0); feast.advance(12.5)").exec().unwrap();
        assert_eq!(engine.job_gauge(1).unwrap(), vec![24,1,0,0,0,0,2500]);
    }
    #[test]
    fn native_rain_heals_once_and_consumes_the_server_gauge() {
        let mut instance = Instance::default();
        instance.zone.id = 745;
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
            action_id: 3362, action_type: ActionType::Action, sequence: 1,
            target: kawari::common::ObjectTypeId { object_id: id, ..Default::default() },
            ..Default::default()
        };
        native_action(&mut instance, &network, id, &request);
        assert_eq!(instance.find_actor(id).unwrap().get_common_spawn().health_points, 1000);
        instance.feast_whm.as_ref().unwrap().lua.load("feast.players[1].adrenaline=100").exec().unwrap();
        native_action(&mut instance, &network, id, &request);
        assert_eq!(instance.find_actor(id).unwrap().get_common_spawn().health_points, 11000);
        assert_eq!(instance.feast_whm.as_ref().unwrap().adrenaline(1), 0);
        native_action(&mut instance, &network, id, &request);
        assert_eq!(instance.find_actor(id).unwrap().get_common_spawn().health_points, 11000);
    }
    #[test]
    fn historical_animation_survives_placeholder_acknowledgement() {
        let lua = Lua::new();
        let engine: Table = lua.load(include_str!("../../../../resources/scripts/feast/WhiteMage.lua")).eval().unwrap();
        let actions: Table = engine.get("actions").unwrap();
        let mut count = 0;
        for pair in actions.pairs::<String, Table>() {
            let (name, definition) = pair.unwrap();
            let historical: u32 = definition.get("id").unwrap();
            assert_eq!(animation_id(&name), Some(historical));
            let request = kawari::ipc::zone::ActionRequest {
                action_id: button_id(&name).unwrap(), action_type: ActionType::Action,
                sequence: 123, ..Default::default()
            };
            let mut result = ActionEffect { action_id: historical, spell_id: historical as u16, ..Default::default() };
            acknowledge_result(&mut result, &request);
            assert_eq!(result.action_id, request.action_id, "{name} button acknowledgement");
            assert_eq!(result.source_sequence, 123);
            assert_eq!(result.spell_id, historical as u16, "{name} historical animation");
            assert_eq!(native_action_name(historical), Some(name.as_str()));
            assert_eq!(native_action_name(request.action_id), Some(name.as_str()));
            count += 1;
        }
        assert_eq!(count, 20);
    }
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
        native_action(&mut instance, &network, id, &request);
        let engine = instance.feast_whm.as_ref().unwrap();
        assert!(engine.enabled(id));
        assert_eq!(engine.party_actions(id.0).unwrap(), vec![(3362, 0), (17832, 0), (18957, 0)]);
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
        native_action(&mut instance, &network, id, &denied);
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
    fn feast_presentation_uses_historical_actions_without_mutating_hp() {
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
                action_id: 17789,
                cast_time: 1.5,
                target: ObjectId(2),
                ..
            })
        ));
        event.kind = "animation".into();
        event.message = "cure2".into();
        event.amount = 4000;
        event.healing = true;
        let Some(ServerZoneIpcData::ActionEffect1(result)) = presentation(&instance, &event) else {
            panic!("Expected action result")
        };
        assert_eq!(result.action_id, 8896);
        assert_eq!(result.source_sequence, 0);
        assert_eq!(result.targets[0].object_id, ObjectId(2));
        assert!(matches!(
            result.effects[0].0,
            TargetEffectKind::Heal { amount: 4000, .. }
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
    common::{CharacterMode, ObjectId, Position, WarpType},
    config::get_config,
    ipc::zone::{
        ActionEffect, ActionType, ActorControlCategory, ActorSetPos, DamageElement, DamageKind,
        DamageType, ServerNoticeMessage, ServerZoneIpcData, ServerZoneIpcSegment, TargetEffect,
        TargetEffectKind,
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
    rotation: f32,
    player: bool,
    job: u8,
    combat: bool,
    invulnerable: bool,
    native_statuses: Vec<u16>,
    match_team: Option<&'static str>,
    match_active: bool,
    match_culling: u8,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default)]
struct Event {
    hits: Vec<Hit>,
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

#[derive(Debug, Default, Clone, Deserialize)]
struct Hit { target: u32, amount: u32, healing: bool }

impl FeastWhiteMage {
    pub fn job_gauge(&self, id: u32) -> mlua::Result<Vec<u16>> {
        self.function("job_gauge")?.call(id)
    }
    pub fn bot_action(&self, id: ObjectId, target: ObjectId, heal: Option<ObjectId>) -> mlua::Result<(String,u32,u64)> {
        self.function("bot_action")?.call((id.0,target.0,heal.map_or(0,|id|id.0)))
    }
    pub fn hotbar(&self, id: u32) -> mlua::Result<Vec<(u16,u16,u16,u16)>> {
        self.lua.from_value(self.function("hotbar")?.call(id)?)
    }
    fn new() -> mlua::Result<Self> {
        let lua = Lua::new();
        let path = get_config()
            .filesystem
            .locate_script_file("feast/WhiteMage.lua");
        #[cfg(not(test))]
        let source = std::fs::read(&path).map_err(mlua::Error::external)?;
        #[cfg(test)]
        let source: &[u8] = include_bytes!("../../../../resources/scripts/feast/WhiteMage.lua");
        let engine: Table = lua.load(source).set_name(path).eval()?;
        #[cfg(not(test))]
        let jobs = std::fs::read(get_config().filesystem.locate_script_file("feast/Jobs.lua")).map_err(mlua::Error::external)?;
        #[cfg(test)]
        let jobs: &[u8] = include_bytes!("../../../../resources/scripts/feast/Jobs.lua");
        lua.load(jobs).eval::<Function>()?.call::<()>(engine.clone())?;
        #[cfg(not(test))]
        let extra = std::fs::read(get_config().filesystem.locate_script_file("feast/AdditionalJobs.lua")).map_err(mlua::Error::external)?;
        #[cfg(test)]
        let extra: &[u8] = include_bytes!("../../../../resources/scripts/feast/AdditionalJobs.lua");
        #[cfg(not(test))]
        let data = std::fs::read(get_config().filesystem.locate_script_file("feast/AdditionalActionData.lua")).map_err(mlua::Error::external)?;
        #[cfg(test)]
        let data: &[u8] = include_bytes!("../../../../resources/scripts/feast/AdditionalActionData.lua");
        lua.load(extra).eval::<Function>()?.call::<()>((engine.clone(),lua.load(data).eval::<Table>()?))?;
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

    pub fn adrenaline(&self, id: u32) -> u8 {
        self.function("adrenaline").and_then(|f| f.call(id)).unwrap_or(0)
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
                } => (instance.feast_dummies.get(id).is_some_and(|d| d.autonomous), *currently_invulnerable),
                _ => return None,
            };
            let s = actor.get_common_spawn();
            let combat = instance.actors.values().any(|enemy| {
                matches!(enemy,
            NetworkedActor::Npc { navmesh_target: Some(super::actor::NpcTarget::Actor(target)), spawn, .. }
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
                rotation: s.rotation,
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
                match_culling: actor.status_effects().and_then(|s| s.get(994)).map_or(0, |s| s.param as u8),
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
    let events = events.into_iter().flat_map(|event| {
        // Supplies may bring an arena cast above eight targets; never drop effects.
        if event.kind == "animation" && event.hits.len() > 8 {
            event.hits.chunks(8).map(|hits| Event { hits: hits.to_vec(), ..event.clone() }).collect::<Vec<_>>()
        } else { vec![event] }
    });
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
                        ..
                    } = &mut packet
                        && let Some(request) = instance
                            .feast_whm
                            .as_ref()
                            .and_then(|engine| engine.native_requests.get(&source))
                    {
                        *action_id = request.action_id;
                    }
                    if let ServerZoneIpcData::ActionEffect1(ref mut result) = packet {
                        if let Some(request) = instance
                            .feast_whm
                            .as_mut()
                            .and_then(|engine| engine.native_requests.remove(&source))
                        {
                            acknowledge_result(result, &request);
                        }
                        result.global_sequence = network.global_action_sequence;
                        network.global_action_sequence =
                            network.global_action_sequence.wrapping_add(1);
                    }
                    if let ServerZoneIpcData::AoeEffect8(ref mut result) = packet {
                        if let Some(request) = instance.feast_whm.as_mut().and_then(|engine| engine.native_requests.remove(&source)) {
                            result.source_sequence = request.sequence;
                            result.action_id = request.action_id;
                            result.action_type = request.action_type;
                        }
                        result.global_sequence = network.global_action_sequence;
                        network.global_action_sequence = network.global_action_sequence.wrapping_add(1);
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
            "job_cooldown" => {
                if let Some(action) = job_actions().iter().find(|a| a.key == event.message && a.group >= 0) {
                    if let Some(engine) = instance.feast_whm.as_ref() {
                        match engine.function("recast").and_then(|f| f.call::<(u32,u32)>((target.0,event.message.as_str(),engine.now()))) {
                            Ok((elapsed,total)) => network.lock().send_to_by_actor_id(target, FromServer::ActorControlSelf(ActorControlCategory::SetCooldownTimer {
                                cooldown_group: action.group as u32, unk1: elapsed, unk2: total,
                            }), DestinationNetwork::ZoneClients),
                            Err(error) => tracing::warn!(%error,"Feast reduced cooldown snapshot failed"),
                        }
                    }
                }
            }
            "reset_action_cooldown" => {
                if let Some(action) = job_actions().iter().find(|a| a.key == event.message && a.group >= 0) {
                    network.lock().send_to_by_actor_id(target, FromServer::ActorControlSelf(ActorControlCategory::SetCooldownTimer {
                        cooldown_group: action.group as u32, unk1: 0, unk2: 0,
                    }), DestinationNetwork::ZoneClients);
                }
            }
            "cancel" => {
                let action_id = instance
                    .feast_whm
                    .as_mut()
                    .and_then(|engine| engine.native_requests.remove(&target))
                    .map_or(0, |request| request.action_id);
                network.lock().send_to_by_actor_id(
                    target,
                    FromServer::ActorControlSelf(cancel_cast(action_id)),
                    DestinationNetwork::ZoneClients,
                );
            }
            "interrupt" => instance.cancel_actor_tasks(target),
            "wolfs_heart" => {
                if let Some(effects) = instance.find_actor_mut(target).and_then(|a| a.status_effects_mut()) {
                    if event.amount > 0 { effects.add(1149, 0, event.amount as f32); }
                    else { effects.remove(1149); }
                }
                super::effect::send_effects_list(network.clone(), instance, target);
            }
            "control_visual" => {
                let Ok(status) = event.message.parse::<u16>() else { continue };
                if !matches!(status, 1343 | 1344 | 1345 | 1347 | 1348 | 1315 | 1316) { continue; }
                if let Some(effects) = instance.find_actor_mut(target).and_then(|a| a.status_effects_mut()) {
                    if event.amount > 0 {
                        effects.add(status, if status == 1344 { 50 } else { 0 }, event.amount as f32 / 1000.0);
                        effects.add_from(status, event.amount as f32 / 1000.0, ObjectId(event.actor));
                    } else { effects.remove(status); }
                }
                super::effect::send_effects_list(network.clone(), instance, target);
            }
            "dia_visual" | "temperance_visual" => {
                let status = if event.kind == "dia_visual" { 2035 } else { 1872 };
                if let Some(effects) = instance
                    .find_actor_mut(target)
                    .and_then(|actor| actor.status_effects_mut())
                {
                    if event.amount > 0 {
                        effects.add_from(status, event.amount as f32, ObjectId(event.actor));
                    } else {
                        effects.remove(status);
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
                super::actor::update_actor_hp_mp(network.clone(), instance, target);
            }
            "damage" => {
                if let Some(m) = &mut instance.feast_match {
                    if m.rules.players.get(&event.actor).is_some_and(|p| p.present && p.alive) {
                        m.supplies.record_damage(event.actor, target, event.amount);
                    }
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
            "notice" => notice(network, ObjectId(event.actor), event.message),
            "move" => {
                use kawari::common::{JumpState, MoveAnimationState, MoveAnimationType};
                let position = Position(glam::Vec3A::new(event.x, event.y, event.z));
                if let Some(actor) = instance.find_actor_mut(target) {
                    actor.get_common_spawn_mut().position = position;
                }
                let rotation = instance.find_actor(target).map_or(0.0,|a|a.get_common_spawn().rotation);
                network.lock().send_in_range_inclusive_instance(target,instance,
                    FromServer::ActorMove(target,position,rotation,MoveAnimationType::empty(),MoveAnimationState::None,JumpState::NoneOrFalling),
                    DestinationNetwork::ZoneClients);
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

#[derive(Deserialize)]
struct JobAction { id: u32, key: String, group: i32, secondary: i32 }
fn job_actions() -> &'static [JobAction] {
    static ACTIONS: std::sync::LazyLock<Vec<JobAction>> = std::sync::LazyLock::new(||
        serde_json::from_str(include_str!("../../../../resources/feast-actions.json")).expect("checked Feast action catalog"));
    &ACTIONS
}

/// Transitional button IDs used by the current client hotbars.
fn button_id(name: &str) -> Option<u32> {
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
        _ => return job_actions().iter().find(|a| a.key == name).map(|a| a.id),
    })
}

// Original 6.08 Feast rows retain identical cast/VFX/end/hit references in
// June 7.51h2. The wire spell_id selects presentation independently of the
// requested button action_id and its sequence/cooldown acknowledgement.
fn animation_id(name: &str) -> Option<u32> {
    Some(match name {
        "cure" => 8895,
        "cure2" => 8896,
        "medica" => 18945,
        "glare" => 17789,
        "dia" => 17790,
        "solace" => 17791,
        "rapture" => 18946,
        "misery" => 17793,
        "tetra" => 13975,
        "assize" => 9620,
        "temperance" => 17794,
        "purify" => 1584,
        "bolt" => 18942,
        "kit" => 18943,
        "repose" => 17690,
        "protect" => 17832,
        "rescue" => 17688,
        "largesse" => 18957,
        "attunement" => 17689,
        "rain" => 3362,
        _ => return job_actions().iter().find(|a| a.key == name).map(|a| a.id),
    })
}

fn acknowledge_result(result: &mut ActionEffect, request: &kawari::ipc::zone::ActionRequest) {
    result.source_sequence = request.sequence;
    result.action_id = request.action_id;
    result.action_type = request.action_type;
    // Preserve spell_id: this is the original Feast animation, not the button.
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
            position: if matches!(event.message.as_str(), "soten" | "cometeor") { Position(glam::Vec3A::new(event.x,event.y,event.z)) } else { actor.position() },
        }
    } else if event.hits.len() > 1 || event.hits.first().is_some_and(|h| h.target != event.target) {
        let mut result = kawari::ipc::zone::AoeActionResult {
            animation_target_id: target, action_id, spell_id: action_id as u16,
            action_type: ActionType::Action, animation_lock: kawari::common::ANIMATION_LOCK_TIME,
            rotation: common.rotation, position: actor.position(),
            effect_count: event.hits.len().min(8) as u8, ..Default::default()
        };
        for (i, hit) in event.hits.iter().take(8).enumerate() {
            let visual = Event { kind: "animation".into(), actor: event.actor, target: hit.target,
                amount: hit.amount, healing: hit.healing, message: event.message.clone(), ..Default::default() };
            if let Some(ServerZoneIpcData::ActionEffect1(single)) = presentation(instance, &visual) {
                result.effects[i] = single.effects;
                result.target_ids[i] = single.targets[0];
            }
        }
        ServerZoneIpcData::AoeEffect8(result)
    } else {
        let mut effects = [TargetEffect::default(); 8];
        let amount = event.amount.min(u16::MAX as u32) as u16;
        if amount > 0 {
            effects[0].0 = if event.healing {
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
        ServerZoneIpcData::ActionEffect1(ActionEffect {
            animation_target_id: target,
            targets: vec![target],
            action_id,
            spell_id: action_id as u16,
            action_type: ActionType::Action,
            animation_lock: kawari::common::ANIMATION_LOCK_TIME,
            rotation: common.rotation,
            effects,
            // Presentation only: HP/MP were already applied by the rules engine.
            // Commands/plugin casts have no native ActionRequest sequence to acknowledge.
            ..Default::default()
        })
    })
}

/// Cancels the client's cast of `action_id`, as the retail server does.
pub(super) fn cancel_cast(action_id: u32) -> ActorControlCategory {
    ActorControlCategory::CancelCast {
        unk1: 538,
        unk2: 1,
        action_id,
    }
}

pub(super) fn native_action_name(id: u32) -> Option<&'static str> {
    if let Some(action) = job_actions().iter().find(|a| a.id == id) { return Some(&action.key); }
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
        .find(|name| button_id(name) == Some(id) || animation_id(name) == Some(id)),
    }
}

pub fn native_action(
    instance: &mut Instance,
    network: &Arc<Mutex<NetworkState>>,
    actor: ObjectId,
    request: &kawari::ipc::zone::ActionRequest,
) {
    let historical = native_action_name(request.action_id)
        .and_then(|name| job_actions().iter().find(|a| a.key == name));
    if instance.zone.id != 745 {
        return;
    }
    if !super::feast_match::can_act(instance, actor) {
        notice(network, actor, "Feast combat is closed: wait for the countdown, or join the next match. Jobs are locked during a match.".into());
        network.lock().send_to_by_actor_id(
            actor,
            FromServer::ActorControlSelf(cancel_cast(request.action_id)),
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
            .ok_or_else(|| mlua::Error::external("Feast job rules are not loaded."))?;
        let name =
            name.ok_or_else(|| mlua::Error::external("This action is not in the Feast kit."))?;
        if !engine.enabled(actor) {
            return Err(mlua::Error::external(
                "Equip WHM, BLM, SAM, or PLD to use the Feast kit.",
            ));
        }
        if instance
            .find_actor(actor)
            .is_none_or(|a| !matches!(a.get_common_spawn().class_job, 19 | 20 | 21 | 22 | 23 | 24 | 25 | 27 | 28 | 30 | 31 | 32 | 33 | 34 | 35 | 37 | 38 | 39 | 40))
        {
            return Err(mlua::Error::external(
                "Feast supports WHM, BLM, SAM, and PLD.",
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
                if historical.is_some_and(|a| a.secondary == 57) {
                    network.lock().send_to_by_actor_id(actor,
                        FromServer::ActorControlSelf(ActorControlCategory::SetCooldownTimer {
                            cooldown_group: 57, unk1: 0, unk2: 240,
                        }), DestinationNetwork::ZoneClients);
                }
                // The client already started this recast when it sent the request, using the
                // HUD's Feast recast and charges. Resending it here arrived a round trip late and
                // visibly rolled the hotbar sweep back; refusals and refunds still resync it.
            } else {
                network.lock().send_to_by_actor_id(
                    actor,
                    FromServer::ActorControlSelf(cancel_cast(request.action_id)),
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
                FromServer::ActorControlSelf(cancel_cast(request.action_id)),
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

/// Bridge from the native ground-target cursor until the generic location-action
/// IPC is implemented. Coordinates and all combat rules are validated here/Lua.
pub fn ground_action(instance: &mut Instance, network: &Arc<Mutex<NetworkState>>, actor: ObjectId, words: &[&str]) {
    if instance.zone.id != 745 || !super::feast_match::can_act(instance,actor) { return; }
    let ["ground", action, x, y, z] = words else { return; };
    let (Ok(action),Ok(x),Ok(y),Ok(z)) = (action.parse::<u32>(),x.parse::<f32>(),y.parse::<f32>(),z.parse::<f32>()) else { return; };
    if ![x,y,z].into_iter().all(f32::is_finite) || !matches!(action,7857|3361|8812) { return; }
    tick(instance,network);
    let Some(engine)=instance.feast_whm.as_ref() else { return; };
    let name=match action {7857=>"soten",8812=>"shukuchi",_=>"cometeor"};
    let result=(|| -> mlua::Result<_> {
        engine.sync(&snapshots(instance))?;
        let response: String=engine.function("ground_command")?.call((actor.0,name,x,y,z,engine.now()))?;
        Ok((response,engine.drain()?))
    })();
    match result {
        Ok((response,events)) => {
            if events.iter().any(|e| e.actor==actor.0 && matches!(e.kind.as_str(),"cast"|"animation")) {
                if let Some(metadata)=job_actions().iter().find(|a| a.id==action && a.group>=0) {
                    if let Ok((elapsed,total))=engine.function("recast").and_then(|f| f.call::<(u32,u32)>((actor.0,name,engine.now()))) {
                        network.lock().send_to_by_actor_id(actor,FromServer::ActorControlSelf(ActorControlCategory::SetCooldownTimer {
                            cooldown_group:metadata.group as u32,unk1:elapsed,unk2:total,
                        }),DestinationNetwork::ZoneClients);
                    }
                }
            }
            apply(instance,network,events); notice(network,actor,response);
        },
        Err(error) => tracing::warn!(%error,"Feast ground action failed"),
    }
}

pub fn tick(instance: &mut Instance, network: &Arc<Mutex<NetworkState>>) {
    if instance.zone.id != 745 {
        return;
    }
    let players: Vec<_> = snapshots(instance)
        .into_iter()
        .filter(|actor| actor.player && matches!(actor.job, 19 | 20 | 21 | 22 | 23 | 24 | 25 | 27 | 28 | 30 | 31 | 32 | 33 | 34 | 35 | 37 | 38 | 39 | 40))
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

/// Supply effects remain authoritative in the combat engine; native statuses provide visuals.
pub fn grant_supply(instance: &mut Instance, network: &Arc<Mutex<NetworkState>>, id: ObjectId, kind: super::feast_supplies::Kind) {
    let Some(engine) = &instance.feast_whm else { return; };
    let result = (|| -> mlua::Result<Vec<Event>> {
        engine.sync(&snapshots(instance))?;
        engine.function("supply")?.call::<()>((id.0, kind == super::feast_supplies::Kind::WolfsHeart, engine.now()))?;
        engine.drain()
    })();
    match result {
        Ok(events) => apply(instance, network, events),
        Err(error) => tracing::error!(%error, "Feast supply reward failed"),
    }
}
