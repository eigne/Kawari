# Developer guide: architecture and contribution workflow

Kawari is a Rust workspace that emulates the services expected by the retail FFXIV client. Most gameplay lives in one world-server process, with Lua scripts supplying selected actions and content behavior. Game assets and Excel sheets supply definitions; SQLite stores characters and accounts; live Rust structures hold actors and instances.

This guide describes source revision `8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856`, inspected on 2026-09-14. The checkout's README identifies its target as **Windows FFXIV 7.51h2**, build `2026.06.18.0000.0000`. This is a statement about this checkout, not a claim about the latest upstream release. The analysis is based on source inspection, not a verified live multiplayer session.

Read these chapters in order:

1. **Architecture and contribution workflow** — this chapter.
2. [Gameplay systems](gameplay.md) — actors, maps, combat, jobs, scripts, and duties.
3. [Building toward Feast 4v4](feast.md) — existing foundations, missing systems, and implementation milestones.

Throughout the guide, **existing behavior** means code is present; it does not imply retail accuracy or complete integration. Proposed designs are explicitly identified as proposals.

Source links point to the inspected GitHub revision so they also work in the published book. The paths in the text identify the corresponding files in a local checkout.

## 1. Repository map

| Location | Responsibility | When to open it |
| --- | --- | --- |
| [Cargo.toml](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/Cargo.toml) | Workspace members, shared dependencies, edition and profiles | Building, dependency changes, version alignment |
| [core/src](https://github.com/redstrate/Kawari/tree/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/core/src) | Shared `kawari` library: wire protocol, common types, configuration | Packets, IDs, flags, shared helpers |
| [servers/world/src](https://github.com/redstrate/Kawari/tree/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/servers/world/src) | Gameplay, connection dispatch, runtime world, persistence, Lua bindings | Almost all map/combat work |
| [servers/lobby/src](https://github.com/redstrate/Kawari/tree/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/servers/lobby/src) | Binary lobby connection and character-selection flow | Login reaches lobby but not the world |
| [servers/login/src](https://github.com/redstrate/Kawari/tree/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/servers/login/src) | Authentication, accounts, sessions, account pages | Registration and authentication |
| [resources/scripts](https://github.com/redstrate/Kawari/tree/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/resources/scripts) | Lua commands, events, actions, effects, content | Implementing supported scripted behavior |
| [resources/dropins](https://github.com/redstrate/Kawari/tree/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/resources/dropins) | JSON placements absent from client layouts | NPC and gathering-point populations |
| [resources/timelines](https://github.com/redstrate/Kawari/tree/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/resources/timelines) | NPC behavior sequences | Enemy casts and scripted behavior |
| [resources/data/tests](https://github.com/redstrate/Kawari/tree/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/resources/data/tests) | Binary captures and other test fixtures | Protocol regression tests |
| [tools/navimesh](https://github.com/redstrate/Kawari/tree/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/tools/navimesh) | Navigation-mesh generation | NPC pathfinding in a new territory |
| [tools/run/src/main.rs](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/tools/run/src/main.rs) | Builds and launches the service executables | Local development |
| [docs/src](..) | mdBook source | Contributor documentation |

The main dependencies explain much of the design: Tokio for asynchronous I/O and tasks, Axum for HTTP services, `binrw` for binary layouts, Diesel/SQLite for persistence, `mlua` for Lua 5.4, Physis for game-file parsing, Icarus for typed Excel sheet readers, and Recast/Detour for navigation. These are the dependencies selected by this checkout, not independent compatibility recommendations.

## 2. Processes and the login path

These are separate executables, not independent gameplay shards:

| Executable | Role |
| --- | --- |
| `kawari-frontier` | Status/news endpoints and launcher/support-facing responses |
| `kawari-launcher` | Launcher HTML and static resources |
| `kawari-login` | Account registration, authentication, session checks, character-backup upload flow |
| `kawari-patch` | Boot/game patch checks and patch responses |
| `kawari-lobby` | Lobby protocol, service-account and character-selection operations |
| `kawari-world` | Zone/chat connections, world simulation, characters and content |
| `kawari-web` | Landing/setup pages and launcher autoconfiguration |
| `kawari-admin` | Administrative pages and configuration operations |

Conceptually, the launcher interacts with the HTTP services, authentication provides a session, the game uses the lobby for selection, and the selected character connects to the world server. The world listener distinguishes zone, chat, and Kawari-internal connections. This is a conceptual flow rather than an exhaustive packet order.

```text
Launcher / browser ---- HTTP ---- login, frontier, launcher, patch, web
Game client ---------- binary --- lobby
Game client ---------- binary --- world
                                   |-- zone connections
                                   |-- chat connections
Other Kawari services - custom IPC -|-- internal requests
```

[World `initial_setup`](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/servers/world/src/main.rs) reads the connection type and chooses the handler. [Custom IPC](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/servers/world/src/custom_ipc_connection.rs) and [send_custom_world_packet](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/core/src/packet/send_helpers.rs) support internal server-to-server operations. Do not confuse this custom wire protocol with the in-process `ToServer`/`FromServer` channels below. Some inter-service calls also use HTTP.

## 3. The world server's two layers

The most useful distinction is between **one player's connection state** and **the shared live world**.

### Connection layer

[ZoneConnection and PlayerData](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/servers/world/src/zone_connection/mod.rs) hold the socket/protocol state, character data, inventory, conditions, current content information, and references to the database, game data, and Lua state.

[main.rs](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/servers/world/src/main.rs) is large because it dispatches many protocol variants:

* `initial_setup` establishes the connection kind and initializes state.
* `client_loop` uses `tokio::select!` to receive socket data or internal world messages. Its selection is biased toward incoming client data.
* `process_packet` interprets client packets. Some operations are handled on the connection; others become `ToServer` messages.
* `process_server_msg` consumes `FromServer` replies/notifications and calls connection methods to update local state or emit packets.

Feature-specific connection methods are under `zone_connection/`: `zone.rs`, `stats.rs`, `item.rs`, `actor.rs`, `event.rs`, `party.rs`, and the social modules. Start with the packet arm, then follow its method or message, rather than reading all of `main.rs` sequentially.

### Shared world layer

[server/mod.rs](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/servers/world/src/server/mod.rs) defines `WorldServer`, creates the runtime, dispatches shared-world messages, and runs periodic logic. Its private modules implement the subsystems.

| Type | What it owns |
| --- | --- |
| `WorldServer` | A vector of live `Instance`s and periodic counters |
| `Instance` | Actors, a loaded `Zone`, navmesh, weather, queued tasks, and optional content director |
| `NetworkedActor` | A player, NPC, event object, or treasure object |
| `NetworkState` | Connected clients, their spawn allocations, parties, linkshell membership, action sequence counter |
| `GameData` | Resolved game resources and sheet readers used to look up definitions |

```text
client packet
    -> process_packet / ZoneConnection
    -> ToServer message
    -> server_main_loop / subsystem handler
    -> mutate Instance and NetworkedActor
    -> NetworkState chooses recipients
    -> FromServer message
    -> process_server_msg / ZoneConnection
    -> serialized client packet
```

[common.rs](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/servers/world/src/common.rs) defines these message enums plus `ServerHandle`, `ClientHandle`, and `ClientId`. They are the central navigation aid for cross-layer behavior: search for a variant's construction and its match arm.

**State is duplicated in places.** A character's connection data, live actor spawn data, and Lua-facing snapshot can all contain related information. Changing only one can produce correct UI for the owner but incorrect data for observers, combat, or the next login. For example, `send_stats` sends a player-stat packet and also sends `ToServer::SetNewStatValues` so the live actor gets updated. Follow that pattern when adding state.

### Concurrency and time

`server_main_loop` creates shared `Arc<parking_lot::Mutex<...>>` values and starts a separate Tokio task with a **500 ms interval**. That task calls `server_logic_tick` and executes due `Instance` tasks. The message-receive loop also accesses the same shared structures. This is not an ECS, nor one dedicated process/thread per territory.

The periodic work includes NPC behavior, trigger-range checks, visibility, director tasks, and other updates. Cast completion and status expiration use `Instant` deadlines but are collected by the interval task. Thus even a zero-cast-time action queued by the action handler waits for a later task scan. Under normal scheduling that can add up to roughly one tick; it is not a strict latency bound under load.

When extending this code:

* Keep mutex scopes small; do not hold synchronous guards across asynchronous I/O.
* Check lock acquisition order across both message and tick paths. Shared `Arc<Mutex<_>>` values do not eliminate deadlock risks.
* Prefer a typed `ToServer` operation for shared-world mutations and a typed queued task for delayed work.
* Do not change the 500 ms interval in isolation: NPC timeline counters explicitly use half-seconds, and other counters depend on the interval.

## 4. Protocol boundaries

[core/src/packet](https://github.com/redstrate/Kawari/tree/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/core/src/packet) handles packet headers, segments, compression, encryption, and scrambling. A packet contains segments; gameplay IPC is one segment kind. [core/src/ipc/zone](https://github.com/redstrate/Kawari/tree/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/core/src/ipc/zone) supplies typed client/server payloads. [opcodes.rs](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/core/src/opcodes.rs) maps protocol message identities, sizes, and numeric opcodes.

The layers are roughly:

```text
TCP bytes -> PacketHeader -> decompressed segments -> IPC header/opcode
          -> ClientZoneIpcData / ServerZoneIpcData -> gameplay dispatch
```

`ConnectionState` carries the state needed for decoding/encoding. The zone setup initializes separate inbound/outbound Oodle state; lobby encryption and zone scrambling have their own implementations. Avoid hand-building raw bytes in gameplay code when a typed variant exists.

Important helpers:

| Helper | Purpose and caveat |
| --- | --- |
| `ServerZoneIpcSegment::new(...)` | Wraps a typed server payload for transmission |
| `ZoneConnection::send_ipc_self(...)` | Sends a payload to that player; it does not broadcast shared state |
| Actor-control helpers | Send compact state changes such as mode, effects, or duty controls; self/target/observer forms differ |
| `NetworkState::send_to_by_actor_id(...)` | Routes a message to a particular actor's connection |
| `send_in_range_inclusive_instance(...)` | Includes the source actor and eligible observers in the instance |
| `send_in_range_instance(...)` | Excludes the source actor |
| `spawn_existing_actor_message(...)` | Allocates the receiving client's spawn slot and constructs the spawn message |

The broadcast helpers' implementation checks instance membership and, for range-based variants, whether the recipient has spawned the actor. They are not action-range validators. `send_to_instance` also excludes the supplied actor; a match-wide broadcast must account for that deliberately. See [network.rs](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/servers/world/src/server/network.rs).

A packet enum or a field named `PvP` proves that a representation exists, not that gameplay is implemented. Unknown fields and hardcoded payloads are common during reverse engineering. For a new packet, preserve layout/size evidence and add a fixture-backed test where possible. The existing [packet guide](../packets/intro.md) and [opcode guide](../opcodes.md) describe the tooling workflow.

## 5. Definitions, saved data, and live data

| Kind of data | Source | Example |
| --- | --- | --- |
| Game definitions | Client resources via Physis/Icarus | `Action`, `ClassJob`, `TerritoryType`, `InstanceContent` |
| Server-authored content | Resources and overlays | Lua behavior, NPC placement JSON, timelines |
| Durable player/account data | SQLite through Diesel | Characters, class experience, inventory, unlocks, sessions |
| Transient world data | Rust structures in memory | Current actors, cast tasks, NPC state, director state |

[GameData::new](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/servers/world/src/gamedata.rs) constructs resource access and loads sheets. Many lookups require `&mut self` even when logically reading data. Its helpers are preferable to scattering raw sheet traversal through handlers.

[WorldDatabase](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/servers/world/src/database/mod.rs), [models.rs](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/servers/world/src/database/models.rs), [schema.rs](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/servers/world/src/database/schema.rs), and [character.rs](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/servers/world/src/database/character.rs) connect character models to SQLite. `commit_player_data` writes the broader character state; narrower helpers include `commit_volatile` and `commit_classjob_and_inventory`. Logout commits player data, and some operations commit earlier, such as class changes and territory transitions. There is no automatic persistence of arbitrary new fields added to `NetworkedActor` or a Lua table.

The world and login services have separate migrations under their respective server directories. If a contribution adds durable data, change models/schema/migrations together. This project's contribution document explicitly says database stability and migration versioning are not currently guaranteed. Use disposable development characters and preserve any local data you care about before schema experiments.

For Feast, temporary match membership and score belong in live match state; durable results would need an explicit persistence design. Avoid using an existing unrelated character field simply because it already reaches SQLite.

## 6. Running and contributing

Follow [local development](../setup/source.md), [Windows setup](../setup/windows.md), and [configuration](../setup/configuration.md) for the exact machine setup. You need the supported game files, Rust, native build dependencies, and the Oodle setup described there. Run from the repository root because many resources are addressed relative to it.

Useful explicit commands from the workspace root:

```powershell
# Build and launch the service suite through the repository's runner.
cargo run --bin kawari-run

# Build the workspace with the runtime compression feature.
cargo build --features oodle

# Run just the world service once its companion services are available.
cargo run --bin kawari-world --features oodle

# Contributor checks for code changes.
cargo fmt --check
cargo clippy --features oodle
cargo test

# Build the documentation if mdBook is installed.
mdbook build docs
```

`kawari-run` invokes a workspace build with `oodle` when started through Cargo, then launches all eight server executables. A world-only process is not a replacement for the login/lobby stack.

The [contribution checklist](../contributing.md) also requires an in-game world login. The [CI workflow](https://github.com/redstrate/Kawari/tree/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/.github/workflows) shows the project's actual build/test matrix. Protocol tests alone cannot verify that a client accepts the packet sequence or displays a duty correctly.

For each new feature, use three levels of evidence:

1. **Logic:** deterministic tests of the state change and its failure paths.
2. **Protocol:** fixture/serialization tests of new or changed wire representations.
3. **Client integration:** observe the owner and another client, then test zoning, disconnecting, and rejoining as appropriate.

For exploration, useful existing commands include `!inspect`, `!cf <ContentFinderCondition ID>`, `!spawnmonster <id>`, `!ai_disable`, and `!finishevent`. See [debug commands](../debug_commands.md) and [GM commands](../gm_commands.md). Commands that bypass restrictions are helpful for setup but are not evidence that normal gameplay validation works.

**Upstream scope matters:** the checked-in [scope policy](../scope.md) currently limits accepted instanced-content work to Sastasha and A Merchant's Tale and asks contributors to contact maintainers before investing in other content. A Feast experiment can be developed locally, with reusable engine fixes proposed separately; upstream acceptance of the mode should be discussed. Also review the contribution page's linked project policies before submitting.

## 7. Efficient reading and debugging

Read in this order: `common.rs` message types; `ZoneConnection`/`PlayerData`; `WorldServer`/`Instance`/`NetworkedActor`; then the feature-specific files in the next chapter.

These searches give a useful cross-section without reading thousands of dispatch lines:

```powershell
rg -n 'ActionRequest|execute_action|CastAction' servers/world/src
rg -n 'ChangeZone|ZoneLoaded|ReadySpawnPlayer' servers/world/src
rg -n 'SetNewStatValues|send_stats|finish_changing_class' servers/world/src
rg -n 'JoinContent|register_for_content|create_instance_for_content' servers/world/src
rg -n 'TODO|unimplemented!' servers/world/src/server/action.rs
```

When something appears to work only on one client, trace both the state mutation and the recipient selection. When something resets on login, trace the database commit path. When a script never runs, inspect its ID-to-filename registration or content short-name lookup before changing its logic.
