# Developer guide: building toward Feast 4v4

This is a proposed implementation roadmap, grounded in the [architecture](architecture.md) and [gameplay](gameplay.md) chapters. An exploration prototype is available below; full Feast gameplay is not implemented.

**Restoring Feast is a new multiplayer game mode, not just adding a territory.** Kawari has useful foundations for loading locations, instancing, actors, actions, and content scripts, but it does not currently provide a complete competitive PvP engine.

## Exploration prototype

The `tools/feast-map/` helper restores territory **745** for exploration on the supported Windows **7.51h2** client. Its arena files survive under `bg/ffxiv/pvp_xx/pvp/l1p2`, while its TerritoryType row was blanked. The helper edits only that row in the current sheet, using an ordinary non-duty, non-PvP definition, and saves a reversible backup. Both client and server need this definition; a server-only teleport is insufficient.

Follow [the setup and rollback instructions](../../../tools/feast-map/README.md), restart the client and the server from this checkout, then use `!feast` to enter and `!feast leave` to return to Limsa. The command requires debug GM access. This is a map-loading prototype, not a Feast match. Native data parsing has been checked; in-game exploration and multiplayer acceptance checks below remain necessary.

## 1. Establish the target before choosing packet IDs

There are two separate compatibility questions:

1. Which version of Feast's rules and job kits do you want to reproduce?
2. Which parts can the supported current client still represent?

A historical map, action ID, or capture does not establish that the current client has the corresponding files, action behavior, UI, or opcode layout. The source inspection for this guide did not inspect a client installation for Feast assets or validate historical Feast captures. No arena ID, medal threshold, match length, or historical balance value is asserted here.

Write a short rules specification covering roster/roles, supported jobs/actions, scoring, pickups, death/respawn, match timing, victory/ties, disconnects, and rewards. Record the historical version and evidence for each restored behavior. This separates “faithful to a chosen Feast version” from “adapted to fit current client capabilities.”

Begin with a small asset/compatibility worksheet:

| Question | Evidence to collect |
| --- | --- |
| Does the arena still have a TerritoryType row? | Row ID, Bg path, intended use, Map ID |
| Are its level files present? | LVB, relevant LGBs, layer set and arrival ranges |
| Can the client render the arena? | Successful load on the supported client build |
| Is there usable duty metadata? | ContentFinderCondition and InstanceContent linkage, script short name, entrance |
| Can the client expose the intended action set? | Current Action rows, action request IDs/types, UI behavior |
| What PvP/team/result presentation survives? | Current packet behavior and any usable client interfaces |

Historical captures can help recover semantics, but numeric opcodes and layouts must be checked against the target build. The repository's [packet documentation](../packets/intro.md) is the starting workflow.

## 2. What already exists

| Foundation | Reuse | Missing work for Feast |
| --- | --- | --- |
| Territory loader and overlays | Load known locations or modified definitions | Confirm arena assets and client compatibility |
| `Instance` | Isolate actors and a director per live copy | Match identity, lifecycle, admission and reconnect policy |
| Party system | Group membership and related packet flows | Two teams with independent match membership |
| Action scripts and task queue | A place to define selected abilities and schedule work | Validation, cooldowns, costs, reliable healing/statuses and PvP dispatch |
| Actor/status packets | Communicate effects and state | Complete authoritative rules and team-aware targeting |
| Director infrastructure | Content setup, objects, variables and events | Match clock, scoring, player-death attribution and result lifecycle |
| Duel handling | Examples of PvP state/countdown packets | Reliable arena/team PvP implementation |
| Database and migrations | Durable character data | Optional match records/results and idempotent rewards |

The duel scaffolding is in [server/mod.rs](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/servers/world/src/server/mod.rs): `RequestDuel`, `RequestDuelResponse`, and `DuelDecision` send `DuelInformation`, `SetPvPState`, and countdown controls, with `dueling_opponent_id` stored on players. However, [Zone::load](https://github.com/redstrate/Kawari/blob/8cdadde59a1e1dd66c8c28ae30f22cc73eeb6856/servers/world/src/server/zone.rs) currently creates the relevant event ranges with `duel: false` and a TODO to restore detection. These paths do not prove a working duel or transferable 4v4 team mode.

The protocol includes `PvPAction`, but the executor does not handle it. Sending it through the current match can reach `unimplemented!()`. Contemporary PvP intended-use mappings and unlock arrays are likewise not evidence of implemented Feast rules.

## 3. Add or restore the arena in stages

### Case A: use a territory already present in the supported client

Start with the existing TerritoryType row. Verify its Bg/LVB/LGB paths, intended use, spawn coordinates, exits, and map data. Exercise the normal zone-transition path with two clients and confirm they enter the same intended instance.

For a duty, also establish the correct ContentFinderCondition/InstanceContent mapping, entrance range, and content short name. `!cf` accepts a ContentFinderCondition ID; it does not accept an arbitrary territory ID. If no suitable duty definition exists, adding one is an additional metadata/client compatibility task.

A successful visual load is the first milestone. Next add only the minimum server placements needed for testing. Use drop-ins for supported NPC placements and content/director APIs for existing event objects. Generate navigation if NPCs need it.

### Case B: restore or introduce territory definitions/assets

[Extensibility](../extensibility.md) supports two different mechanisms:

```yaml
filesystem:
  additional_resource_paths:
    - C:/dev/kawari-feast/resources
  additional_search_paths:
    - C:/dev/kawari-feast/game-overlay
```

These are example paths, not files created by this guide. Preserve your existing `game_path` and other settings when incorporating them.

* `additional_resource_paths` overlays Kawari resources such as scripts, drop-ins, and timelines. Match their expected directory structure and check the scanner caveats in the gameplay chapter.
* `additional_search_paths` overlays game-file paths for server-side resolution, such as modified `exd/TerritoryType...` files.

**The server overlay does not install files into players' clients.** New/restored visuals and changed client sheet definitions require a compatible client-side arrangement too. Plan and verify that separately. The [existing hidden-zone notes](../tips.md) explain the repository's approach to locations without a normal territory definition.

Do not expect a JSON drop-in to restore missing geometry. Also do not assume an old intended-use value has a current `HandlerType::from_intended_use` mapping: content initialization can fail before the director script runs.

### Arena acceptance checklist

Before implementing scores, verify:

1. Two clients enter the same arena instance and observe movement correctly.
2. Two separate test instances of the same territory cannot see or affect one another.
3. Each side has valid arrival/respawn coordinates and a usable exit path.
4. Relevant event objects spawn and interact on both clients.
5. Disconnecting or leaving does not strand the other participant or corrupt future admissions.
6. NPC navigation works if the prototype needs NPCs; player combat reach/line of sight is tested separately.

## 4. Introduce explicit match state

**Proposed design:** add an instance-owned `FeastMatch` (or a more general match controller if another mode genuinely needs it). Keep it authoritative in Rust. Let Lua configure rules/content where useful and let a presentation adapter produce supported client packets.

An illustrative model, not an existing API:

```text
FeastMatch
  match_id
  phase: Forming | Loading | Countdown | Active | Finished | Closing
  rules_version
  participants: actor_id -> team, readiness, life state, score data
  team totals
  phase/match deadlines
  pending respawns and world pickups
  result / completion marker
```

The exact score fields depend on the selected historical rules. Distinguish match membership from ordinary party membership, and use a stable match identity rather than a mutable position in `WorldServer.instances`.

```text
Forming -> Loading -> Countdown -> Active -> Finished -> Closing
               |          |          |
               +----------+----------+-> explicit abort/forfeit policy
```

Define every transition with a trigger and guard. For example, Countdown begins only when the selected roster has loaded; damage is allowed only in Active; completion happens once; Closing returns remaining players and releases match resources.

The existing JoinContent handler creates an instance from the caller's party. It does not merge two parties into one match. For an early prototype, implement explicit assignment of eight test participants to one instance. Then add queueing/admission with role/team constraints and readiness. Do not call the existing party admission path twice and assume both parties will share one match.

All clients should receive the same authoritative snapshot after entry/re-entry: phase, remaining time, teams, player state, and score. Existing director variables can carry only the client representation their protocol allows; keep richer state in Rust.

## 5. Build a reliable combat subset

Start with two supported jobs and a tiny action set that exercises damage, healing, a status, and a cast. Expand only after those behave consistently for both teams.

**Proposed validation pipeline:**

```text
resolve actor and match
 -> validate phase, life state, allowed job/action and resource cost
 -> resolve target in the same instance
 -> validate team relation, range and any required line of sight
 -> check/reserve recast and casting state
 -> schedule or execute
 -> revalidate relevant state at cast completion
 -> apply authoritative effects
 -> emit result/resource/status packets
```

At minimum, address the [combat findings](gameplay.md#5-combat-follow-one-action-end-to-end): the unsupported action type, missing authoritative healing, incomplete cooldown/cost checks, fixed result capacities, and the combo task-retention predicate. The latter can affect unrelated scheduled work, making it particularly important before adding respawns or match deadlines to the same queue.

Represent recast groups and resources on authoritative actors/match participants. Client animation locks and action feedback are presentation; they do not replace enforcement. Implement a shared eligibility rule for allies/enemies rather than using UI flags or NPC battalion data as the entire PvP policy.

Override the existing passive recovery rules for match participants as needed: the world tick currently restores 10% of max HP/MP to living players without NPC haters. A player-versus-player fight may not populate that NPC-based combat check, so leaving it unchanged can invalidate both combat balance and healing tests.

Similarly, implement mechanical statuses, not just icons. Clarify duration, refresh, source attribution, stacking, cleansing, death cleanup, and immunity rules. A refresh should invalidate obsolete expiry tasks. If an interrupt should cancel a cast, it must not cancel unrelated status expiry or match work.

Before changing scheduling granularity, convert tick-count assumptions to elapsed time or otherwise preserve their units. Measure end-to-end combat timing with multiple clients; a new nominal interval alone is not evidence of responsiveness.

## 6. Add death, scoring, respawn, and completion

The existing NPC death callback cannot award player kills. Add an explicit authoritative event along the lines of `PlayerDefeated { victim, source, cause, match_id }`, with a policy for damage over time, environmental death, assists, and disconnects if relevant to the chosen rules.

Process defeat once, then apply the specified scoring/pickup behavior and schedule a respawn if permitted. On respawn, reconcile HP/resources, conditions, statuses, casts, cooldown policy, position, and client mode. Use the existing movement/actor packet machinery, but make the lifecycle explicit. A `Kill` animation by itself neither settles a score nor implements a respawn.

If the selected Feast rules use world pickups, implement their authoritative ownership and lifetime. Validate proximity, active phase, and single consumption. Two near-simultaneous pickup requests must not duplicate value. Verify any score-conservation invariant required by those rules.

Use server deadlines for the match clock, and derive UI updates from them. Generic duty countdown/result packets may be useful, but do not assume a dungeon completion event produces Feast's historical result UI. Isolate presentation code so an initial debug display can later be replaced by verified PvP packets without rewriting scoring.

Completion should freeze the result, prevent further score changes, inform all participants, apply any rewards once, and schedule exit/cleanup. If durable results are needed, add explicit database records and migrations; transient Lua variables will not survive a restart.

## 7. Suggested contribution sequence

| Milestone | Deliverable | Evidence required |
| --- | --- | --- |
| 1. Compatibility investigation | Arena/action/UI worksheet and chosen rules version | Reproducible current-client observations |
| 2. Empty arena | Correct loading, entry, exit and instance isolation | Two clients plus two separate instances |
| 3. Combat foundations | Safe targeting, damage/healing, recasts, statuses, corrected task handling | Deterministic tests and opposing-client observations |
| 4. Minimal match | Explicit rosters, loading barrier, countdown and active/end phases | Eight clients or a documented integration harness plus client checks |
| 5. Scoring and respawn | Attributed defeats, specified scoring/pickups, timed return | Duplicate/reordered event and race tests |
| 6. Client presentation | Team, timer, score, results and cleanup | All participants agree throughout the match |
| 7. Broader jobs and admission | Additional kits, role rules, queueing, disconnect policy | Full-match and failure-path tests |

Favor small engine fixes with independent value: bounded effects, proper healing, safe task cancellation, source-aware status expiration, and authoritative target validation are useful outside Feast. The [upstream scope policy](../scope.md) still applies to submitting the mode itself.

## 8. Tests that catch the expensive mistakes

| Scenario | Expected invariant |
| --- | --- |
| Target belongs to another arena instance | No damage, healing, statuses, or information leakage |
| Ally/enemy/self targeting | Allowed relation follows the action rules |
| Action during loading/countdown/finished phase | No unauthorized combat state change |
| Repeated requests inside a cooldown | At most the permitted execution; costs charged correctly |
| Target dies or leaves while a cast is pending | Completion is safely rejected or resolved by explicit rules |
| Combo executes with other tasks pending | Other actors' casts, expirations, and respawns remain intact |
| Heal near maximum HP | Authoritative HP clamps correctly and all observers agree |
| Status refreshed before its original deadline | Old expiry cannot remove the refreshed application |
| Player defeat is processed twice | Score and death bookkeeping change once |
| Two players request one pickup | Only one valid consumption occurs |
| Last-second score and timeout coincide | Deterministic ordering and one result |
| Player disconnects/rejoins | Correct membership and snapshot; no duplicated participant |
| Last player leaves | Cleanup follows the chosen grace/abort policy |
| Match ends | No late task changes its result or affects a later match |

A good first vertical slice is: **two players enter an isolated arena, damage and heal correctly, one defeats the other, the server records that once, and the victim respawns consistently on both clients**. That proves the critical engine path before historical UI, the complete job roster, or matchmaking adds complexity.
