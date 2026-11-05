# Developer guide: gameplay systems

This chapter follows [the architecture overview](architecture.md). Paths and behavior refer to the same inspected revision. The emphasis is on the systems you will touch when adding an arena and a multiplayer combat mode.

## 1. Learn the ID vocabulary first

Many parameters are integers with different namespaces. Passing a valid number from the wrong namespace can produce a plausible-looking but incorrect result.

| ID | Meaning | Typical use |
| --- | --- | --- |
| `ClientId` | Internal connection identifier | Routing a channel message |
| `ObjectId` / actor ID | Entity identity in the live world/protocol | Targeting and actor lookup |
| Character `content_id` | Persistent character identity | Database relations and social systems |
| TerritoryType ID / zone ID | A territory definition | `Zone::load`, changing territory |
| Map ID | The map associated with a territory | Discovery and map presentation; not the zone ID |
| ContentFinderCondition ID | Duty-finder entry | `!cf`, queueing and instance creation |
| InstanceContent ID | Content definition referenced by a duty | Director setup, content metadata |
| `HandlerId` | Encoded event-handler type and event ID | Dispatching events/director traffic |
| Layout ID | A placed object identity in level/drop-in data | Finding a door, boss placement, or trigger |
| Base ID | A sheet definition, such as BNpcBase or EObj | Which kind of NPC/object to instantiate |
| Spawn index | A slot in one recipient client's entity table | Creating/deleting a visible actor |
| ClassJob ID | A class/job sheet row | Job metadata and packet class fields |
| EXP-array index | Progression slot supplied by ClassJob data | Class/job level and experience arrays |

Two different uses of “content ID” are especially easy to confuse: a character's durable identity and a duty's content definition. Check the type, helper, and sheet at every call site.

## 2. Actors and visibility

[NetworkedActor](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/servers/world/src/server/actor.rs) is an enum with `Player`, `Npc`, `Object`, and `Treasure` variants. Players/NPCs have spawn data containing HP, resources, class, level, position, and other protocol-facing state. Player variants also hold calculated parameters, status effects, combo state, and a dueling opponent. NPCs add navigation, targeting, timelines, and their AI state.

Useful actor methods include `position`, `rotation`, `status_effects_mut`, and the spawn accessors. **`get_common_spawn` is only valid for players/NPCs**: it uses `unreachable!()` for objects and treasure. A generic “target an object” feature cannot blindly call it.

Visibility is managed per receiving client. [SpawnAllocator](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/servers/world/src/zone_connection/spawn_allocator.rs) assigns limited slots; [NetworkState](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/servers/world/src/server/network.rs) builds the correctly indexed spawn packet. A given actor can have different spawn indices on different clients. Player slot zero is reserved for self in the actor allocator setup.

`NetworkedActor::in_range_of` measures horizontal distance, ignoring Y, for player visibility. Spawn tracking is then used by broadcast helpers. This is separate from whether an attack is allowed to reach a target, whether a wall blocks it, or whether a match member should receive the scoreboard.

For an arena, keep all eight participants in the same **Instance**, use actor IDs for identities, and route match-wide state independently of actor visibility where necessary.

There are also three different kinds of actor state: **conditions** describe client/activity restrictions such as being occupied or bound by duty; **CharacterMode** describes modes such as normal, mounted, or dead; **status effects** represent buffs/debuffs. They are not interchangeable. A match transition may need to update more than one through the normal connection/world synchronization paths.

## 3. Territories, maps, and live instances

Three concepts should stay distinct:

* **Territory/zone definition:** which game location and layout to load.
* **Map:** map-sheet identity used for presentation and discovery.
* **Instance:** one live copy of the territory, with its own actors and director.

[WorldServer::ensure_exists](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/servers/world/src/server/mod.rs) reuses a public instance with the same zone ID and no content-finder condition. For territories classified as private, it creates a new instance. `create_instance_for_content` creates a new content instance, associates its ContentFinderCondition ID, initializes a director if a script can be loaded, and identifies its entrance range. Two matches using one territory need two instances; sharing the zone ID is not sufficient isolation.

`cleanup_dead_instances` removes instances without any player variants. Match reconnect/grace-period behavior would have to account for this cleanup policy.

### How Zone::load works

[server/zone.rs](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/servers/world/src/server/zone.rs) contains the map-loading pipeline:

1. Read the TerritoryType row; obtain intended use, Map ID, and `Bg` path.
2. Load the corresponding `bg/<Bg>.lvb` through the game resource resolver.
3. Read relevant referenced LGB layer groups. The server skips `bg.lgb`, `vfx.lgb`, and `sound.lgb` here.
4. Find the layer set associated with the territory, including its navigation path.
5. Locate a matching drop-in and collect its extra placements.
6. Build caches and runtime trigger information from the applicable layers.

The client renders the location using its own files. The world server reads enough level data to model objects, trigger ranges, entrances/exits, and actors. This loader does not send geometry or textures to the client.

[Instance::new](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/servers/world/src/server/instance.rs) wraps the loaded zone with weather, navigation, initial event objects, and NPC spawns. Missing/invalid territory rows can produce warnings and a largely empty zone; reaching that code is not proof that the client can load the location.

### Drop-ins and navigation

[DropInObjectData](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/core/src/common/dropin.rs) supports gathering points, battle NPCs, and event NPCs. The older comment claiming only gathering points are supported is narrower than the actual enum and consumers. Battle NPC placements include base/name IDs, level, optional HP, hostility, link-family settings, and `nonpop` for placements that should be spawned later by content logic.

`Zone::get_npcs` reads these placements because battle-NPC placements are stripped from retail client LGBs. Merely loading the visual dungeon does not populate its encounters. A layout ID identifies a placement and is documented as not necessarily unique across all drop-in objects; it is not a substitute for the live actor ID.

There are two useful loader details when extending content:

* `Zone::load` stops at the first matching drop-in (`break 'outer`); do not assume every file matching the LGB will be merged.
* Drop-ins add the supported placement types. They do not create arbitrary new terrain, client models, or trigger-box formats.

Navigation uses [Navmesh](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/servers/world/src/navmesh.rs) and the [navimesh tool](https://github.com/redstrate/Kawari/tree/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/tools/navimesh). `filesystem.navimesh_path` specifies the cache location, and `world.generate_navmesh` controls generation when a required mesh is absent. The instance records generation state and the server launches the helper. Kawari's `.nvm` format is explicitly its own format despite sharing a filename extension with retail server data.

Navigation is used by NPC movement. Its presence should not be treated as proof of server-side collision enforcement for player movement or line-of-sight checks for combat.

### Trigger ranges and zone transitions

`MapRange` represents a simplified trigger volume, with sanctuary/discovery/entrance flags and optional gimmicks. `MapGimmick` includes generic triggers, jump pads, and same-zone “fake exits.” The periodic world logic checks overlaps and invokes the appropriate behavior. `PopRange` identifies a spawn/arrival region; an `ExitRange` connects travel to another destination.

The transition path is roughly:

```text
ZoneConnection::change_zone
    -> ToServer::ChangeZone
    -> handle_zone_messages / transition helpers
    -> remove old live actor; insert placeholder in destination instance
    -> FromServer::ChangeZone
    -> ZoneConnection::handle_zone_change
    -> client zone initialization and loading acknowledgements
    -> ReadySpawnPlayer / ZoneLoaded handling completes live participation
```

Read [server/zone.rs](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/servers/world/src/server/zone.rs) alongside [zone_connection/zone.rs](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/servers/world/src/zone_connection/zone.rs). The former handles shared membership and destination selection; the latter updates saved position/zone information and sends the client initialization, content, and related packets. `change_zone_warp_to_pop_range` and `change_zone_warp_to_entrance` are useful entry points for content travel.

Editing a saved zone number alone bypasses this lifecycle. For a match, treat “assigned to the instance” and “finished loading and ready to play” as separate states.

## 4. Classes, jobs, equipment, and stats

There is no Rust class hierarchy with a subclass per FFXIV job. Class/jobs are primarily **data-driven identities plus scripts**.

[database::ClassJob](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/servers/world/src/database/models.rs) stores current class, levels, EXP, first class, and rested EXP. [GameData::get_exp_array_index](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/servers/world/src/gamedata.rs) maps ClassJob rows onto progression slots; do not index the level arrays directly by the job ID. Sheet helpers also provide starting levels, class modifiers, soul-crystal mapping, and item compatibility.

### Equipment-driven job changes

[zone_connection/item.rs](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/servers/world/src/zone_connection/item.rs) contains:

* `change_class_based_on_weapon`: inspect the equipped main-hand item's ClassJobCategory and select a compatible class.
* `change_class_based_on_soul_crystal`: map the equipped crystal to its job.
* `remove_incompatible_armor`: reconcile equipment with the new class/job.

The inventory packet handling in `main.rs` coordinates these with client equipment behavior. `update_class_info` sends level/EXP/sync information and an `ActorGauge` packet. `finish_changing_class` emits change-related packets and commits class/job plus inventory data.

**Gauge support is not a completed job engine.** `update_class_info` currently sends `ActorGauge { data: [0; 15], ... }`. A gauge packet definition does not supply gauge generation, spending, job-specific validation, or action replacement logic.

### The stat calculation pipeline

[zone_connection/stats.rs](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/servers/world/src/zone_connection/stats.rs) is the central file:

```text
ClassJob modifiers + Tribe attributes + ParamGrow at effective level
    -> calculate_based_on_level
    -> calculate_stat_across_all_items (optional item-level caps)
    -> calculate_potencies
    -> BaseParameters
```

`base_parameters` accounts for the connection's optional synced level. `unscaled_base_parameters` calculates unsynced values. `send_stats` sends the stat payload and updates the shared actor through `SetNewStatValues`. The latter updates level, class, max HP/MP, and cached parameters, and sends an effects-list packet used by the client for those fields.

The formulas are deliberately approximate. For example, physical damage is derived from strength, and `calc_physical_damage(potency)` is effectively `floor(potency / 100 * physical_damage)`, returned as `u16`. Magic and healing helpers similarly scale their respective potency values. These are not a complete model of retail weapon damage, defense, critical/direct hits, mitigation, or every job's main stat.

For Feast, decide on an explicit PvP stat profile and supported action set. The existing PvE-style level/item calculation should not silently become the balance model for a historical PvP mode.

## 5. Combat: follow one action end to end

The main source is [server/action.rs](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/servers/world/src/server/action.rs), with supporting [effects_builder.rs](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/servers/world/src/lua/effects_builder.rs), [stats.rs](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/servers/world/src/zone_connection/stats.rs), and [instance.rs](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/servers/world/src/server/instance.rs).

### Request and scheduling

1. `process_packet` receives `ClientZoneIpcData::ActionRequest` and forwards it with the connection/actor IDs.
2. `handle_action_messages` looks up cast time via `GameData::get_casttime`.
3. For a nonzero cast it broadcasts `ActorCast`.
4. It inserts `QueuedTaskData::CastAction` with an `Instant` deadline.
5. The interval task eventually calls `execute_action`.

Movement handling in `server/mod.rs` checks and cancels interruptible player cast tasks; other trigger paths also cancel casts. This does not amount to a general rules engine for stun, silence, immunity, line of sight, or target legality.

### Script execution

`execute_action` builds a Lua-facing player value, copying selected data such as the actor's calculated parameters and teleport query. It compares the action's sheet-defined combo predecessor with the stored last combo action.

It dispatches by action type to normal actions, items, or mounts. `execute_normal_action` looks up the action ID in `KawariLuaState.action_scripts`, loads its file, and calls `doAction(player, in_combo)`. A missing script logs a warning. The action-specific LuaPlayer is not a fully populated live actor view: several fields retain defaults. Check what the caller actually fills before using a Lua property in a new action.

This is the pattern in [FastBlade_00009.lua](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/resources/scripts/actions/000/FastBlade_00009.lua), written here with local variables to make its data flow clear:

```lua
local POTENCY = 150

function doAction(player, in_combo)
    local effects = EffectsBuilder()
    effects:damage(
        DAMAGE_KIND_NORMAL,
        DAMAGE_TYPE_SLASHING,
        player.parameters:calc_physical_damage(POTENCY)
    )
    return effects
end
```

The script chooses potency and produces an effect description. It does not itself select recipients, mutate target HP, or validate target range. Those are Rust responsibilities. Combos can select another potency using `in_combo`, but the current combo scheduling needs the fixes described below before relying on it in a match.

### Applying results and sending packets

`EffectsBuilder` collects `ActionEffect`s: damage, healing, gaining/losing statuses, interrupts, VFX, pet summons, and combos. In the player action executor:

* Damage subtracts from the target's live HP with saturation, with a striking-dummy exception.
* NPC invulnerability rewrites damage effects to an invincible effect.
* NPC targets are marked as newly aggroed.
* An interrupt currently cancels all tasks belonging to the target, with a TODO about narrowing this behavior.
* The executor updates HP/MP, constructs an `ActionResult`, applies status gains, constructs relevant `EffectResult`s, and sends queued Lua tasks back to the player's connection.

`ActionResult` carries animation/effect feedback and sequence values; `EffectResult` carries status-related result information; `UpdateHpMpTp` communicates authoritative resource values. Sending attractive combat feedback is distinct from changing the authoritative state.

**Important limits in this exact implementation:**

| Area | Source evidence and implication |
| --- | --- |
| Healing | `EffectsBuilder::heal` and Cure exist, but the player executor's mutation match has no `Heal` arm. The effect can be represented/sent without increasing target HP. |
| PvP action dispatch | `ActionType::PvPAction` exists in the protocol enum, but `execute_action` handles only None/Action/Item/Mount; other types reach `unimplemented!()`. Verify the actual type emitted by the client, then implement the appropriate path. |
| Cooldowns/costs | The executor has a cooldown TODO and a special cooldown-removal debug path, not a general authoritative recast/resource-cost system. |
| Target validation | The shown path does not provide comprehensive same-instance, team, range, line-of-sight, job/level, and alive-state checks before scheduling/applying an action. In particular it looks up the target's instance separately. |
| Multiple targets | The current builder/executor is organized around one requested target and a fixed eight-effect array. It is not a general AoE target-resolution engine. |
| Capacity | The copy into eight effects and status-result arrays is not generically bounded; additional script effects can exceed those capacities. Automatic combo effects also consume space. |
| Combo cleanup | `retain_tasks` directly calls `Vec::retain`. The action executor passes a predicate that keeps the current actor's ResetCombo tasks, despite a comment saying to cancel them. This can discard other queued work. The sequence comparison also occurs after replacing the stored action. Review both before extending combos. |
| Formula fidelity | The damage helpers are simplified and do not consume a complete target mitigation model. |
| Passive recovery | `server_logic_tick` restores 10% of max HP/MP per tick for living players without NPC haters. That NPC-based combat check needs a PvP-aware replacement or override. |

These are source findings, not results from a live Feast test. The proposed first combat milestone should fix and test these foundations before expanding the job roster.

## 6. Statuses, death, and enemy AI

[StatusEffects](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/servers/world/src/status_effects.rs) stores a vector with a dirty flag. Adding/removing statuses marks it dirty; [server/effect.rs](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/servers/world/src/server/effect.rs) sends updates and schedules removal.

`gain_effect_instance` adds a status, optionally emits actor-control/list updates, and queues `LoseStatusEffect` unless duration is zero. `remove_effect` updates the model, sends removal/list packets, and can run the effect's Lua removal behavior. Inspect [resources/scripts/effects](https://github.com/redstrate/Kawari/tree/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/resources/scripts/effects) for examples.

For PvP, extend this deliberately: the store currently finds existing entries by effect ID, does not preserve a full source-aware identity through its `add` API, and refreshes need to reconcile previously scheduled expiry tasks. It is not yet a general multi-caster status/stacking system. A status icon alone does not implement the mechanical effect of crowd control or damage reduction.

`update_actor_hp_mp` broadcasts HP/MP and calls `kill_actor` if HP reaches zero and the actor is not already dead. `kill_actor` sets dead mode, emits the death control, cancels the actor's queued tasks, and schedules NPC fading/despawn. It does not itself force HP to zero; that remains a TODO, so callers must not equate every invocation with a completed resource-state update.

**The director death callback is NPC-specific in this path.** `kill_actor` extracts a BNPC layout ID and calls `on_actor_death` only for NPCs. It does not report a player killer/victim pair. Feast scoring needs a new player-death event with attribution; changing Sastasha's `onActorDeath` alone is insufficient.

[npc_behavior.rs](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/servers/world/src/server/npc_behavior.rs) uses `Wander`, `Follow`, `Hate`, and `Dead` states. It considers battalion hostility from game data, navigation, targeting, linked enemies, and JSON timelines. [execute_enemy_action](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/servers/world/src/server/action.rs) is a separate execution path, so fixing player actions does not automatically fix enemies.

Timelines are defined in [core/src/common/timeline.rs](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/core/src/common/timeline.rs) and [resources/timelines](https://github.com/redstrate/Kawari/tree/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/resources/timelines). NPC timeline positions are explicitly half-second counters. Existing hate/enmity presentation includes placeholder values; do not use it as a complete scoring or threat system.

## 7. Lua and interactive events

[KawariLua::new/init](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/servers/world/src/lua/state.rs) expose enums/flags, load `Global.lua`, discover scripts, set up constructors and registrations, and run `Init.lua`. There are separate Lua contexts for connection-side work, shared-world actions, and individual content directors.

Actions/effects are discovered in a two-level directory layout; the numeric suffix after `_` in each Lua filename is the registration key. Additional resource roots are searched before bundled scripts, and the first entry for an ID wins. Put a new action in the existing directory convention and ensure registration is refreshed; changing a file's contents is distinct from adding a newly discoverable filename.

The scanning code expects the directories it reads to exist. A custom resource root may need empty `scripts/actions`, `scripts/effects`, and `dropins` directories for the relevant loaders. Check actual resolver/scanner behavior rather than assuming all missing overlay directories are ignored.

[LuaPlayer](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/servers/world/src/lua/player.rs) exposes properties and operations. Many operations enqueue [LuaTask](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/servers/world/src/lua/task.rs) values; [zone_connection/lua.rs](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/servers/world/src/zone_connection/lua.rs) applies them to the real connection and persistence/packet paths. A script-side table or copied field is not automatically authoritative.

For talks, cutscenes, shops, crafting, and gathering, [dispatch_event](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/servers/world/src/event.rs) maps `HandlerId` to a Rust or Lua `EventHandler`. Callbacks cover entering territory/triggers, talking, yielding a scene, and returning from a scene. The connection retains active events while the client runs its UI/cutscene sequence. [events/instance_content.rs](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/servers/world/src/events/instance_content.rs) bridges this machinery for duties.

Keep these event handlers separate from the instance director: an interactive event serves a player's client interaction; the director owns shared content progression.

## 8. Duty registration and directors

Queue packets enter `process_packet`, which records duty settings and calls `register_for_content`. Acceptance calls `join_content`, producing `ToServer::JoinContent`. In [server/mod.rs](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/servers/world/src/server/mod.rs), JoinContent gathers online members of the caller's party, or the caller alone, and creates a content instance for them. This is basic admission, not an eight-player matchmaker with two opposing teams and atomic readiness checks.

In particular, `register_for_content` stores the first supplied duty ID and immediately sends update/found packets. A successful duty-finder popup is not evidence that roster matching occurred.

Content lookup uses `ContentFinderCondition` and `InstanceContent`. `HandlerType::from_intended_use` in [common/game.rs](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/core/src/common/game.rs) selects a director family. Some contemporary PvP intended uses map to InstanceContent, but that mapping is not implementation of their rules. Unsupported intended uses can fail during director creation; a legacy arena's classification must be verified.

`create_instance_for_content` derives a short name from game data and loads `resources/scripts/content/<short-name>.lua`. It calls `DirectorData::setup` before zoning players so initial variables can be sent. If the file is absent or fails to load, this path warns and leaves the instance without its scripted director.

[DirectorData](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/servers/world/src/server/director.rs) stores a handler ID, a flag, ten client-facing data bytes, Lua state, queued tasks, boss tracking, and shortcut information. Callbacks include setup, gimmick access, completed event-action casts, NPC death, and trigger entry. Director methods queue tasks; `director_tick` applies tasks such as spawning/hiding objects, changing BGM, and updating client variables. It is a task drain, not an existing general Lua `onTick` callback for arbitrary match clocks.

[s1d1_re.lua](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/resources/scripts/content/s1d1_re.lua) is the Sastasha example. Read `onSetup`, `setSequence`, `onGimmickAccessor`, and `onActorDeath`: together they demonstrate randomized setup, progression variables, interactions, and encounter completion. [m6d1.lua](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/resources/scripts/content/m6d1.lua) supplies another content example. Both contain unfinished behavior; treat them as integration examples rather than fully validated templates.

`DirectorVars`, `DirectorEvent`, `DirectorTrigger`, `MapEffects`, and related actor controls coordinate the client's content UI. The ten data bytes are a wire representation, not a sensible home for an entire eight-player match model. Use explicit server-side state and derive the presentation packets from it.

Continue with [the Feast developer guide](../feast/intro.md) for how the Feast PvP mode was built on these systems.
