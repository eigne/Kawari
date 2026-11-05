//! Bridge to the Lua combat engine in `resources/scripts/feast/`.
//!
//! All Feast combat rules (damage, healing, casts, cooldowns, job gauges,
//! crowd control) live in Lua. This file feeds the engine and runs its
//! replies. Every call follows the same pattern:
//!
//! 1. `sync`: send a snapshot of every actor (HP, MP, position, job, team).
//! 2. Call one engine function (`command`, `advance`, `supply`, ...).
//! 3. `drain`: take the events it produced and apply them (`effects.rs`).
//!
//! The engine is created per arena instance when first needed, and dropped
//! when the match ends, so script edits apply to the next match without a
//! rebuild.
use std::{collections::HashMap, sync::Arc, time::Instant};

use kawari::{
    common::{CharacterMode, ObjectId, Position},
    ipc::zone::{ActionRequest, ActionType, ActorControlCategory},
};
use mlua::{FromLuaMulti, Function, IntoLuaMulti, Lua, LuaSerdeExt, Table};
use parking_lot::Mutex;
use serde::Serialize;

use super::{
    actions::{self, GCD_GROUP},
    effects::{Event, apply},
};
use crate::{
    FromServer,
    server::{
        actor::{NetworkedActor, NpcTarget},
        instance::Instance,
        network::{DestinationNetwork, NetworkState},
    },
};

/// The engine scripts, in load order. Engine.lua returns the engine table
/// (with White Mage built in); each job file returns a function that adds
/// its jobs to it.
const SCRIPTS: [&str; 3] = ["Engine.lua", "Jobs.lua", "AdditionalJobs.lua"];
/// Native action metadata for AdditionalJobs.lua, generated from the catalog.
const ACTION_DATA: &str = "AdditionalActionData.lua";
/// ActionRequest's target when nothing is targeted.
const NO_TARGET: u32 = 0xE000_0000;

#[derive(Debug)]
pub struct CombatEngine {
    lua: Lua,
    started: Instant,
    /// The hotbar request behind each actor's cast in progress. Its action
    /// ID and sequence are echoed back so the client accepts the result.
    pub(super) native_requests: HashMap<ObjectId, ActionRequest>,
}

/// What the engine knows about one actor, sent before every call.
#[derive(Serialize)]
struct ActorSnapshot {
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
    /// An enemy NPC is targeting this actor.
    combat: bool,
    invulnerable: bool,
    native_statuses: Vec<u16>,
    match_team: Option<&'static str>,
    /// False before the match starts or when the player's job changed.
    match_active: bool,
    match_culling: u8,
}

fn read_script(name: &str) -> mlua::Result<Vec<u8>> {
    #[cfg(test)]
    let path = std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../resources/scripts/feast/"
    ))
    .join(name);
    #[cfg(not(test))]
    let path = std::path::PathBuf::from(
        kawari::config::get_config()
            .filesystem
            .locate_script_file(&format!("feast/{name}")),
    );
    std::fs::read(&path)
        .map_err(|error| mlua::Error::external(format!("{}: {error}", path.display())))
}

impl CombatEngine {
    pub fn new() -> mlua::Result<Self> {
        let lua = Lua::new();
        let mut engine: Option<Table> = None;
        for name in SCRIPTS {
            let chunk = lua
                .load(read_script(name)?)
                .set_name(format!("@feast/{name}"));
            match &engine {
                None => engine = Some(chunk.eval()?),
                Some(engine) if name == "AdditionalJobs.lua" => {
                    let data: Table = lua.load(read_script(ACTION_DATA)?).eval()?;
                    chunk
                        .eval::<Function>()?
                        .call::<()>((engine.clone(), data))?
                }
                Some(engine) => chunk.eval::<Function>()?.call::<()>(engine.clone())?,
            }
        }
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

    /// The engine's clock: seconds since it was created.
    fn now(&self) -> f64 {
        self.started.elapsed().as_secs_f64()
    }

    fn sync(&self, instance: &Instance) -> mlua::Result<()> {
        self.function("sync")?
            .call((self.lua.to_value(&actor_snapshots(instance))?, self.now()))
    }

    fn drain(&self) -> mlua::Result<Vec<Event>> {
        self.lua.from_value(self.function("drain")?.call(())?)
    }

    /// Syncs the world, calls the engine function `name`, and returns its
    /// result together with the events it produced.
    fn run<R: FromLuaMulti>(
        &self,
        instance: &Instance,
        name: &str,
        args: impl IntoLuaMulti,
    ) -> mlua::Result<(R, Vec<Event>)> {
        self.sync(instance)?;
        let result = self.function(name)?.call(args)?;
        Ok((result, self.drain()?))
    }

    /// Job gauge for the HUD: `[job, resource, ...]`, see `director.rs`.
    pub fn job_gauge(&self, id: u32) -> mlua::Result<Vec<u16>> {
        self.function("job_gauge")?.call(id)
    }

    /// A bot's next action: (key, target, cast time in ms).
    pub fn bot_action(
        &self,
        id: ObjectId,
        target: ObjectId,
        heal: Option<ObjectId>,
    ) -> mlua::Result<(String, u32, u64)> {
        self.function("bot_action")?
            .call((id.0, target.0, heal.map_or(0, |id| id.0)))
    }

    /// Each hotbar action as (base ID, ID it currently becomes, cast ms, recast ms).
    pub fn hotbar(&self, id: u32) -> mlua::Result<Vec<(u16, u16, u16, u16)>> {
        self.lua.from_value(self.function("hotbar")?.call(id)?)
    }

    /// The party list's three action slots: (action ID, tenths of a second left).
    pub fn party_actions(&self, id: u32) -> mlua::Result<Vec<(u16, u16)>> {
        let rows: Table = self.function("party_actions")?.call((id, self.now()))?;
        rows.sequence_values::<Table>()
            .map(|row| {
                let row = row?;
                Ok((row.get(1)?, row.get(2)?))
            })
            .collect()
    }

    /// An action's recast as (elapsed, total) in hundredths of a second.
    pub fn recast(&self, id: ObjectId, key: &str) -> mlua::Result<(u32, u32)> {
        self.function("recast")?.call((id.0, key, self.now()))
    }

    pub fn adrenaline(&self, id: u32) -> u8 {
        self.function("adrenaline")
            .and_then(|f| f.call(id))
            .unwrap_or(0)
    }

    /// Whether this actor has Feast rules enabled (players get them on entry).
    pub fn enabled(&self, id: ObjectId) -> bool {
        self.function("enabled")
            .and_then(|f| f.call(id.0))
            .unwrap_or(false)
    }

    /// Records a movement packet; moving early in a cast interrupts it.
    pub fn movement(&self, id: ObjectId, position: Position) {
        if let Err(error) = self.function("movement").and_then(|f| {
            f.call::<()>((id.0, self.now(), position.0.x, position.0.y, position.0.z))
        }) {
            tracing::error!(%error, "Feast cast movement check failed");
        }
    }

    /// Whether crowd control stops this actor acting (or moving, if `movement`).
    pub fn blocked(&self, id: ObjectId, movement: bool) -> bool {
        self.function("blocked")
            .and_then(|f| f.call((id.0, movement)))
            .unwrap_or(false)
    }
}

fn actor_snapshots(instance: &Instance) -> Vec<ActorSnapshot> {
    let arena = instance.feast.arena.as_ref();
    instance
        .actors
        .iter()
        .filter_map(|(id, actor)| {
            let (player, invulnerable) = match actor {
                NetworkedActor::Player { .. } => (true, false),
                // Bots count as players so the engine gives them a job kit.
                NetworkedActor::Npc {
                    currently_invulnerable,
                    ..
                } => (
                    instance.feast.practice.get(id).is_some_and(|d| d.autonomous),
                    *currently_invulnerable,
                ),
                _ => return None,
            };
            let s = actor.get_common_spawn();
            let combat = instance.actors.values().any(|enemy| {
                matches!(enemy,
                    NetworkedActor::Npc { navmesh_target: Some(NpcTarget::Actor(target)), spawn, .. }
                    if target == id && spawn.common.health_points > 0)
            });
            Some(ActorSnapshot {
                id: id.0,
                // The GM kill command can leave HP on a dead actor.
                hp: if s.mode == CharacterMode::Dead { 0 } else { s.health_points },
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
                match_team: arena
                    .and_then(|a| a.rules.players.get(&id.0))
                    .map(|p| p.team.name()),
                match_active: !player || super::arena::can_act(instance, *id),
                match_culling: actor
                    .status_effects()
                    .and_then(|s| s.get(994))
                    .map_or(0, |s| s.param as u8),
                native_statuses: actor
                    .status_effects()
                    .map(|effects| effects.data().iter().map(|s| s.effect_id).collect())
                    .unwrap_or_default(),
            })
        })
        .collect()
}

/// Creates the instance's engine if it has none yet.
fn ensure_engine(instance: &mut Instance) -> mlua::Result<()> {
    if instance.feast.combat.is_none() {
        instance.feast.combat = Some(CombatEngine::new()?);
    }
    Ok(())
}

fn cancel_native_cast(network: &Arc<Mutex<NetworkState>>, actor: ObjectId, action_id: u32) {
    network.lock().send_to_by_actor_id(
        actor,
        FromServer::ActorControlSelf(super::cancel_cast(action_id)),
        DestinationNetwork::ZoneClients,
    );
}

/// Whether the engine accepted `actor`'s action (it started a cast or landed).
fn accepted(events: &[Event], actor: ObjectId) -> bool {
    events
        .iter()
        .any(|event| event.actor == actor.0 && matches!(event.kind.as_str(), "cast" | "animation"))
}

/// Handles an ActionRequest from a player's hotbar inside the arena. The
/// request names the historical action ID; the engine decides everything else.
pub fn native_action(
    instance: &mut Instance,
    network: &Arc<Mutex<NetworkState>>,
    actor: ObjectId,
    request: &ActionRequest,
) {
    if !super::arena::can_act(instance, actor) {
        super::notice(
            network,
            actor,
            "Feast combat is closed: wait for the countdown, or join the next match. Jobs are locked during a match.".into(),
        );
        cancel_native_cast(network, actor, request.action_id);
        return;
    }
    // Bring time forward first, so a cast that just ended is not "already casting".
    tick(instance, network);
    let action = (request.action_type == ActionType::Action)
        .then(|| actions::by_id(request.action_id))
        .flatten();
    let target = match request.target.object_id.0 {
        NO_TARGET => 0,
        id => id,
    };
    let result = (|| -> mlua::Result<(String, Vec<Event>)> {
        let engine = instance
            .feast
            .combat
            .as_ref()
            .ok_or_else(|| mlua::Error::external("Feast job rules are not loaded."))?;
        let action =
            action.ok_or_else(|| mlua::Error::external("This action is not in the Feast kit."))?;
        let job = instance
            .find_actor(actor)
            .map_or(0, |a| a.get_common_spawn().class_job);
        if !super::is_feast_job(job) || !engine.enabled(actor) {
            return Err(mlua::Error::external(
                "Equip a job that existed in the Feast to use its PvP actions.",
            ));
        }
        engine.run(
            instance,
            "command",
            (actor.0, action.key.as_str(), target, engine.now()),
        )
    })();
    let (response, events) = match result {
        Ok(reply) => reply,
        Err(error) => {
            super::notice(network, actor, error.to_string());
            cancel_native_cast(network, actor, request.action_id);
            return;
        }
    };
    if accepted(&events, actor) {
        instance
            .feast
            .combat
            .as_mut()
            .unwrap()
            .native_requests
            .insert(actor, request.clone());
        // Weaponskills with their own recast (Drill, Air Anchor) also start
        // the GCD, which the client does not do for them on its own.
        if action.is_some_and(|a| a.secondary == GCD_GROUP as i32) {
            network.lock().send_to_by_actor_id(
                actor,
                FromServer::ActorControlSelf(ActorControlCategory::SetCooldownTimer {
                    cooldown_group: GCD_GROUP,
                    unk1: 0,
                    unk2: 240,
                }),
                DestinationNetwork::ZoneClients,
            );
        }
        // The client already started this recast when it sent the request, using the
        // HUD's Feast recast and charges. Resending it here arrived a round trip late and
        // visibly rolled the hotbar sweep back; refusals and refunds still resync it.
    } else {
        cancel_native_cast(network, actor, request.action_id);
    }
    apply(instance, network, events);
    super::notice(network, actor, response);
}

/// Runs a text command in the engine for `actor`, against their current
/// target: an action key (`"glare"`), or `on`, `list`, `status`, `loadout a b`.
/// Used by bots, `!feast actions/job/loadout`, `!whm`, and tests.
pub fn command(
    instance: &mut Instance,
    network: &Arc<Mutex<NetworkState>>,
    actor: ObjectId,
    text: &str,
) {
    if !super::in_arena(instance) {
        super::notice(
            network,
            actor,
            "Feast rules are only available in the Feast arena.".into(),
        );
        return;
    }
    if text.trim().eq_ignore_ascii_case("off") {
        super::notice(
            network,
            actor,
            "Feast rules apply automatically in this arena. Leave the arena for normal actions."
                .into(),
        );
        return;
    }
    let target = instance
        .find_actor(actor)
        .map_or(0, |a| a.get_common_spawn().target_id.object_id.0);
    let result = ensure_engine(instance).and_then(|()| {
        let engine = instance.feast.combat.as_ref().unwrap();
        engine.run::<String>(instance, "command", (actor.0, text, target, engine.now()))
    });
    match result {
        Ok((response, events)) => {
            apply(instance, network, events);
            super::notice(network, actor, response);
        }
        Err(error) => {
            tracing::error!(%error, "Feast command failed");
            super::notice(network, actor, format!("Feast command failed: {error}"));
        }
    }
}

/// Ground-targeted actions (Soten, Shukuchi, Cometeor). The HUD catches the
/// client's ground cursor and sends `!feast ground <id> <x> <y> <z>`, since
/// the location-action packet is not implemented. The engine checks range.
pub fn ground_action(
    instance: &mut Instance,
    network: &Arc<Mutex<NetworkState>>,
    actor: ObjectId,
    action_id: u32,
    position: [f32; 3],
) {
    if !super::arena::can_act(instance, actor) || !position.iter().all(|v| v.is_finite()) {
        return;
    }
    let Some(action) = actions::by_id(action_id) else {
        return;
    };
    tick(instance, network);
    let Some(engine) = instance.feast.combat.as_ref() else {
        return;
    };
    let [x, y, z] = position;
    let result = engine.run::<String>(
        instance,
        "ground_command",
        (actor.0, action.key.as_str(), x, y, z, engine.now()),
    );
    let (response, events) = match result {
        Ok(reply) => reply,
        Err(error) => {
            tracing::warn!(%error, "Feast ground action failed");
            return;
        }
    };
    // Unlike hotbar actions, the client did not start this recast itself.
    if accepted(&events, actor)
        && let Some(group) = actions::cooldown_group(&action.key)
        && let Ok((elapsed, total)) = engine.recast(actor, &action.key)
    {
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
    apply(instance, network, events);
    super::notice(network, actor, response);
}

/// Advances the engine to now: finishes casts, ticks statuses and gauges.
/// Also turns Feast rules on for players and bots who just arrived.
pub fn tick(instance: &mut Instance, network: &Arc<Mutex<NetworkState>>) {
    if !super::in_arena(instance) {
        return;
    }
    let arrivals: Vec<_> = instance
        .actors
        .iter()
        .filter(|(id, actor)| {
            let player = matches!(actor, NetworkedActor::Player { .. })
                || instance
                    .feast
                    .practice
                    .get(id)
                    .is_some_and(|d| d.autonomous);
            player
                && super::is_feast_job(actor.get_common_spawn().class_job)
                && !instance
                    .feast
                    .combat
                    .as_ref()
                    .is_some_and(|engine| engine.enabled(**id))
        })
        .map(|(id, _)| *id)
        .collect();
    for actor in arrivals {
        command(instance, network, actor, "on");
    }
    let Some(engine) = instance.feast.combat.as_ref() else {
        return;
    };
    match engine.run::<()>(instance, "advance", engine.now()) {
        Ok(((), events)) => apply(instance, network, events),
        Err(error) => tracing::error!(%error, "Feast tick failed"),
    }
}

/// Lets the engine modify damage from outside it (scripted NPC actions,
/// training dummies): buffs, debuffs, shields and invulnerability.
pub fn adjust_damage(instance: &Instance, source: ObjectId, target: ObjectId, amount: u16) -> u16 {
    let Some(engine) = instance.feast.combat.as_ref() else {
        return amount;
    };
    engine
        .function("modify_damage")
        .and_then(|f| f.call((source.0, target.0, amount, engine.now())))
        .unwrap_or_else(|error| {
            tracing::error!(%error, "Feast damage modifier failed");
            amount
        })
}

/// Gives a player a supply kit's reward. The engine owns the effect; the
/// native status is only its icon.
pub fn grant_supply(
    instance: &mut Instance,
    network: &Arc<Mutex<NetworkState>>,
    id: ObjectId,
    kind: super::supplies::Kind,
) {
    let Some(engine) = &instance.feast.combat else {
        return;
    };
    let wolfs_heart = kind == super::supplies::Kind::WolfsHeart;
    match engine.run::<()>(instance, "supply", (id.0, wolfs_heart, engine.now())) {
        Ok(((), events)) => apply(instance, network, events),
        Err(error) => tracing::error!(%error, "Feast supply reward failed"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::feast::{
        TERRITORY,
        effects::{Hit, presentation},
    };
    use kawari::ipc::zone::{ServerZoneIpcData, TargetEffectKind};

    /// A Lua state with the test helpers the scripts in resources/tests use:
    /// `make_engine()` (a White Mage-only engine) and `install_jobs(engine)`.
    fn lua_test_harness() -> Lua {
        let lua = Lua::new();
        let load = |name: &str| {
            lua.load(read_script(name).unwrap())
                .into_function()
                .unwrap()
        };
        lua.globals()
            .set("make_engine", load("Engine.lua"))
            .unwrap();
        lua.globals().set("load_jobs", load("Jobs.lua")).unwrap();
        lua.globals()
            .set("load_additional_jobs", load("AdditionalJobs.lua"))
            .unwrap();
        lua.globals()
            .set("load_action_data", load(ACTION_DATA))
            .unwrap();
        lua.load("function install_jobs(m) load_jobs()(m); load_additional_jobs()(m, load_action_data()) end")
        .exec()
        .unwrap();
        lua
    }

    fn run_lua_test(name: &str) -> u32 {
        let path = std::path::PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../resources/tests/"
        ))
        .join(name);
        lua_test_harness()
            .load(std::fs::read(path).unwrap())
            .set_name(name)
            .eval()
            .unwrap()
    }

    /// An arena instance with a full engine and one WHM per id.
    fn arena(ids: &[u32]) -> (Instance, Arc<Mutex<NetworkState>>) {
        let mut instance = Instance::default();
        instance.zone.id = TERRITORY;
        instance.feast.combat = Some(CombatEngine::new().unwrap());
        for &id in ids {
            instance.insert_empty_actor(ObjectId(id));
            let common = instance
                .find_actor_mut(ObjectId(id))
                .unwrap()
                .get_common_spawn_mut();
            common.class_job = 24;
            common.health_points = 1000;
            common.max_health_points = 20000;
            common.resource_points = 10000;
            common.max_resource_points = 10000;
        }
        (instance, Arc::new(Mutex::new(NetworkState::default())))
    }

    fn hp(instance: &Instance, id: u32) -> u32 {
        instance
            .find_actor(ObjectId(id))
            .unwrap()
            .get_common_spawn()
            .health_points
    }

    #[test]
    fn white_mage_lua_rules() {
        assert!(run_lua_test("feast_white_mage.lua") >= 25);
    }

    #[test]
    fn every_job_lua_rules() {
        assert!(run_lua_test("feast_jobs.lua") > 100);
        assert!(run_lua_test("feast_all_jobs.lua") > 500);
    }

    #[test]
    fn engine_actions_match_the_catalog() {
        let engine = CombatEngine::new().unwrap();
        let definitions: Table = engine
            .lua
            .globals()
            .get::<Table>("feast")
            .unwrap()
            .get("actions")
            .unwrap();
        for action in actions::all() {
            let definition: Table = definitions.get(action.key.as_str()).unwrap();
            assert_eq!(
                definition.get::<u32>("id").unwrap(),
                action.id,
                "{}",
                action.key
            );
        }
        engine
            .lua
            .load("feast.sync({{id=1,job=25,player=true,hp=10000,max_hp=10000,mp=10000,x=0,y=0,z=0}},0); feast.command(1,'on',0,0)")
            .exec()
            .unwrap();
        assert!(engine.hotbar(1).unwrap().contains(&(8862, 8862, 0, 2400)));
        assert_eq!(engine.job_gauge(1).unwrap()[0], 25);
        engine
            .lua
            .load("feast.sync({{id=1,job=24,player=true,hp=10000,max_hp=10000,mp=10000,x=0,y=0,z=0,combat=true,match_active=true}},0); feast.advance(12.5)")
            .exec()
            .unwrap();
        assert_eq!(engine.job_gauge(1).unwrap(), vec![24, 1, 0, 0, 0, 0, 2500]);
    }

    #[test]
    fn area_actions_list_every_hit_and_statuses_get_native_icons() {
        let (mut instance, network) = arena(&[1, 2]);
        let event = Event {
            kind: "animation".into(),
            message: "assize".into(),
            actor: 1,
            target: 1,
            hits: vec![
                Hit {
                    target: 2,
                    amount: 1000,
                    healing: false,
                },
                Hit {
                    target: 1,
                    amount: 2000,
                    healing: true,
                },
            ],
            ..Default::default()
        };
        let Some(ServerZoneIpcData::AoeEffect8(result)) = presentation(&instance, &event) else {
            panic!("missing area result")
        };
        assert_eq!(result.effect_count, 2);
        assert_eq!(result.spell_id, 9620);
        assert_eq!(result.target_ids[0].object_id, ObjectId(2));
        assert!(matches!(
            result.effects[0][0].0,
            TargetEffectKind::Damage { amount: 1000, .. }
        ));
        assert!(matches!(
            result.effects[1][0].0,
            TargetEffectKind::Heal { amount: 2000, .. }
        ));
        let status = |instance: &Instance, id: u16| {
            instance
                .find_actor(ObjectId(2))
                .unwrap()
                .status_effects()
                .unwrap()
                .get(id)
        };
        apply(
            &mut instance,
            &network,
            vec![Event {
                kind: "dia_visual".into(),
                actor: 1,
                target: 2,
                amount: 15,
                ..Default::default()
            }],
        );
        let dia = status(&instance, 2035).unwrap();
        assert_eq!(dia.duration, 15.0);
        assert_eq!(dia.source_actor_id, ObjectId(1));
        apply(
            &mut instance,
            &network,
            vec![Event {
                kind: "dia_visual".into(),
                actor: 1,
                target: 2,
                amount: 0,
                ..Default::default()
            }],
        );
        assert!(status(&instance, 2035).is_none());
        apply(
            &mut instance,
            &network,
            vec![Event {
                kind: "control_visual".into(),
                actor: 1,
                target: 2,
                amount: 5000,
                message: "1344".into(),
                ..Default::default()
            }],
        );
        let heavy = status(&instance, 1344).unwrap();
        assert_eq!(
            (heavy.duration, heavy.param, heavy.source_actor_id),
            (5.0, 50, ObjectId(1))
        );
        apply(
            &mut instance,
            &network,
            vec![Event {
                kind: "control_visual".into(),
                actor: 1,
                target: 2,
                message: "1344".into(),
                ..Default::default()
            }],
        );
        assert!(status(&instance, 1344).is_none());
    }

    #[test]
    fn empyrean_rain_needs_full_adrenaline_and_consumes_it() {
        let (mut instance, network) = arena(&[1]);
        let id = ObjectId(1);
        let request = ActionRequest {
            action_id: 3362,
            action_type: ActionType::Action,
            sequence: 1,
            target: kawari::common::ObjectTypeId {
                object_id: id,
                ..Default::default()
            },
            ..Default::default()
        };
        native_action(&mut instance, &network, id, &request);
        assert_eq!(hp(&instance, 1), 1000);
        let engine = instance.feast.combat.as_ref().unwrap();
        engine
            .lua
            .load("feast.players[1].adrenaline=100")
            .exec()
            .unwrap();
        native_action(&mut instance, &network, id, &request);
        assert_eq!(hp(&instance, 1), 11000);
        assert_eq!(instance.feast.combat.as_ref().unwrap().adrenaline(1), 0);
        native_action(&mut instance, &network, id, &request);
        assert_eq!(hp(&instance, 1), 11000);
    }

    #[test]
    fn hotbar_casts_enable_rules_and_keep_their_request_until_they_land() {
        let (mut instance, network) = arena(&[1]);
        let id = ObjectId(1);
        let request = ActionRequest {
            action_id: 8896, // Cure II, a 1.5s cast
            action_type: ActionType::Action,
            sequence: 42,
            target: kawari::common::ObjectTypeId {
                object_id: id,
                ..Default::default()
            },
            ..Default::default()
        };
        native_action(&mut instance, &network, id, &request);
        let engine = instance.feast.combat.as_ref().unwrap();
        assert!(engine.enabled(id));
        assert_eq!(
            engine.party_actions(id.0).unwrap(),
            vec![(3362, 0), (17832, 0), (18957, 0)]
        );
        assert_eq!(engine.native_requests[&id].sequence, 42);
        assert_eq!(hp(&instance, 1), 1000);
        // A second request while casting is refused and keeps the first.
        let mut denied = request.clone();
        denied.sequence = 43;
        native_action(&mut instance, &network, id, &denied);
        let engine = instance.feast.combat.as_ref().unwrap();
        assert_eq!(engine.native_requests[&id].sequence, 42);
        engine.function("advance").unwrap().call::<()>(2.0).unwrap();
        let events = engine.drain().unwrap();
        apply(&mut instance, &network, events);
        assert!(
            instance
                .feast
                .combat
                .as_ref()
                .unwrap()
                .native_requests
                .is_empty()
        );
        assert!(hp(&instance, 1) > 1000);
        assert_eq!(actions::by_id(17789).map(|a| a.key.as_str()), Some("glare"));
        assert!(actions::by_id(u32::MAX).is_none());
    }

    #[test]
    fn presentation_uses_historical_actions_without_changing_hp() {
        let (instance, _) = arena(&[1]);
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
        assert_eq!(hp(&instance, 1), 1000);
        event.message = "unknown".into();
        assert!(presentation(&instance, &event).is_none());
    }

    #[test]
    fn engine_applies_hp_mp_and_damage_modifiers() {
        let (mut instance, network) = arena(&[1, 2]);
        instance
            .find_actor_mut(ObjectId(1))
            .unwrap()
            .get_common_spawn_mut()
            .target_id
            .object_id = ObjectId(2);
        command(&mut instance, &network, ObjectId(1), "on");
        command(&mut instance, &network, ObjectId(2), "on");
        command(&mut instance, &network, ObjectId(1), "tetra");
        assert_eq!(hp(&instance, 2), 5000);
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
        assert_eq!(hp(&instance, 2), 5000);
    }
}
