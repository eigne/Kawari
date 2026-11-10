//! The Feast: restored 4v4 medal PvP (patch 6.08 rules) in the Crystal Tower
//! Training Grounds. The developer guide (docs/src/feast/) explains it all.
//!
//! - `rules`: the medal match itself (phases, teams, medals, drops). Pure logic.
//! - `arena`: runs a match in an instance: jails, piles, hostility, culling.
//! - `combat`: bridge to the Lua combat engine (resources/scripts/feast/).
//! - `effects`: turns combat engine events into packets.
//! - `actions`: the historical action catalog (resources/feast-actions.json).
//! - `supplies`: supply boxes and the kits they drop.
//! - `director`: per-player match snapshots for the HUD plugin.
//! - `lobby`: custom matches: teams, the host and the team parties.
//! - `practice`: training dummies.
//! - `commands`: the `!feast` match commands.
//!
//! Everything Feast-specific about an instance lives in `Instance::feast`.
//! Code elsewhere in the server calls in here through the `pub` functions
//! below and in `arena`/`combat`; search for `feast::` to find those hooks.
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Instant,
};

use kawari::{
    common::{ObjectId, Position, WarpType},
    ipc::zone::{
        ActorControlCategory, ActorSetPos, ServerNoticeMessage, ServerZoneIpcData,
        ServerZoneIpcSegment, SpawnNpc,
    },
};
use parking_lot::Mutex;

use super::{
    actor::{NetworkedActor, NpcState},
    instance::Instance,
    network::{DestinationNetwork, NetworkState},
};
use crate::FromServer;

mod actions;
pub mod arena;
pub mod combat;
mod commands;
mod director;
mod effects;
mod lobby;
mod practice;
pub mod rules;
mod supplies;

pub use commands::{command, is_match_command};

/// Crystal Tower Training Grounds (bg l1p2), the Feast arena.
pub const TERRITORY: u16 = 745;
/// The arena's duty (ContentFinderCondition), restored by tools/feast-map.
pub const CONTENT_FINDER_CONDITION: u16 = 476;
/// Native duty length: preparation plus the match.
pub const DUTY_DURATION: u16 = (rules::COUNTDOWN + rules::MATCH_TIME) as u16;
/// The 19 jobs that existed before patch 6.1, when the Feast ended.
pub const JOBS: [u8; 19] = [
    19, 20, 21, 22, 23, 24, 25, 27, 28, 30, 31, 32, 33, 34, 35, 37, 38, 39, 40,
];

pub fn is_feast_job(job: u8) -> bool {
    JOBS.contains(&job)
}

/// Whether this instance is the Feast arena (as a duty or for exploration).
pub fn in_arena(instance: &Instance) -> bool {
    instance.zone.id == TERRITORY
}

/// Everything Feast-specific about one instance. Empty outside the arena.
#[derive(Debug, Default)]
pub struct FeastState {
    /// The Lua combat engine, created when the first player arrives.
    pub combat: Option<combat::CombatEngine>,
    /// The current match, created when the first player arrives.
    pub arena: Option<arena::Arena>,
    /// Practice dummies, by actor.
    pub practice: HashMap<ObjectId, practice::PracticeActor>,
    /// Teams and host, kept across the matches of this instance.
    pub lobby: lobby::Lobby,
    /// When each KO'd player may return.
    pub respawns: HashMap<ObjectId, Instant>,
    /// Players whose native duty director was already initialized. A respawn
    /// reloads the zone, and initializing the director twice leaves a second
    /// Crystalline Conflict HUD behind.
    pub directors_sent: HashSet<ObjectId>,
}

/// Runs one world tick of the arena: match, combat engine, then practice dummies.
pub fn tick(instance: &mut Instance, network: &Arc<Mutex<NetworkState>>) {
    arena::tick(instance, network);
    combat::tick(instance, network);
    practice::tick(instance, network);
}

/// Sends `message` to one player as a system notice (ignores empty messages).
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

/// Moves a player's own client to `position` (it predicts its own movement,
/// so a server-side move alone is not enough).
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

/// Cancels the client's cast of `action_id`, as the retail server does.
pub fn cancel_cast(action_id: u32) -> ActorControlCategory {
    ActorControlCategory::CancelCast {
        unk1: 538,
        unk2: 1,
        action_id,
    }
}

/// Whether `viewer`'s client has spawned `actor` yet. Updates sent before
/// that are lost, so they must wait.
fn has_spawned_for(network: &NetworkState, viewer: ObjectId, actor: ObjectId) -> bool {
    network
        .clients
        .values()
        .any(|(handle, state)| handle.actor_id == viewer && state.has_spawned(actor))
}

/// Adds a battle NPC with no AI script (supply boxes, practice dummies).
fn insert_npc(instance: &mut Instance, id: ObjectId, spawn: SpawnNpc) {
    instance.actors.insert(
        id,
        NetworkedActor::Npc {
            state: NpcState::Wander,
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
            last_wander_timestamp: Instant::now(),
            parameters: Default::default(),
            casting: false,
        },
    );
}
