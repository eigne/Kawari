# Architecture

This chapter maps every Feast file and follows the two flows that matter
most: one world tick, and one button press. If you only read one chapter
after the introduction, read this one.

## Where everything is

### World server: `servers/world/src/server/feast/`

All Feast server code is in this one module. Everything Feast-specific about
an instance is stored in one field, `Instance::feast` (a `FeastState`), which
stays empty outside the arena.

| File | What it does |
|---|---|
| `mod.rs` | Constants (`TERRITORY` 745, `CONTENT_FINDER_CONDITION` 476, the 19 `JOBS`), `FeastState`, `tick`, and small helpers (`notice`, `correct_position`, `cancel_cast`). |
| `rules.rs` | The medal match as pure logic: phases, teams, medals, drops, pickups, culling, winner. No networking, so it is easy to test. |
| `arena.rs` | Runs `rules` in a world instance: places players in their jails, opens and closes the gates, spawns medal piles, makes the teams enemies, applies Marked for Culling, sends announcements and director frames. |
| `combat.rs` | The bridge to the Lua combat engine: snapshots actors into it, calls it, and applies what it returns. Handles hotbar actions (`native_action`), text commands, ground-targeted actions and the engine's tick. |
| `effects.rs` | Turns the engine's events into packets and actor changes: cast bars, action results, HP/MP, statuses, movement. |
| `actions.rs` | The historical action catalog, read from `resources/feast-actions.json`. |
| `supplies.rs` | Supply boxes, the kits they drop, and who gets them. |
| `director.rs` | Per-player match frames for the HUD, sent through the duty director. |
| `practice.rs` | `!feast dummies`. |
| `lobby.rs` | Custom matches: the teams from party order, the host, `!feast teams` / `swap` / `start`, and the team parties. |
| `commands.rs` | The `!feast` match commands (`status`, `restart`, `dummies`, ...). |

The rest of the server calls into this module from a handful of places.
Search for `feast::` to find them all:

| File | Hook |
|---|---|
| `server/mod.rs` | `feast::tick` every world tick; keeps all participants in each other's view; no natural HP regeneration in the arena; crowd control undoes movement; records movement for interrupting casts; restarts the duty timer length. |
| `server/action.rs` | In the arena, every `ActionRequest` goes to `combat::native_action` instead of the normal action scripts. Damage from scripted actions passes through `combat::adjust_damage`. |
| `server/zone.rs` | Entering stages players in the west jail; `ReturnAfterDeath` respawns them in their own jail after the ten-second wait; zone loads reset per-viewer state (`arena::on_zone_loaded`). |
| `server/actor.rs` | A KO starts the respawn wait; practice dummies stay as corpses. |
| `server/director.rs` | The duty director is initialized only once per player; the duty timer follows the match clock. |
| `server/npc_behavior.rs` | Practice dummies and supply boxes have no AI; crowd control stops NPCs too. |
| `server/chat.rs` | Routes `!feast <match command>` and `!whm` here. |
| `server/party.rs` | `disband_party` and `form_party`, used to give each team its own party and to reunite the group after the result. |
| `zone_connection/actor.rs`, `main.rs` | The player's own spawn carries their team battalion (`SetFeastBattalion`). |
| `zone_connection/stats.rs` | Maximum HP in the arena depends only on role (`feast_max_hp`). |
| `zone_connection/zone.rs`, `zone_connection/mod.rs`, `gamedata.rs` | Refuse the Feast duty if the client data patch is missing (`feast_duty_metadata_ready`). |

### Combat engine: `resources/scripts/feast/`

| File | What it does |
|---|---|
| `Engine.lua` | The engine: state, damage/healing, statuses, casts, cooldowns, the public functions the server calls, and the White Mage kit. Its header comment is the reference for the engine's API. |
| `Jobs.lua` | Black Mage, Samurai, Paladin, Warrior, Dragoon, plus the hooks every job shares (roles, damage modifiers, combos, job gauges, hotbars). |
| `AdditionalJobs.lua` | The other 13 jobs. Wraps the hooks from `Jobs.lua`. |
| `AdditionalActionData.lua` | Generated metadata (IDs, cast and recast times, ranges, cooldown groups) for `AdditionalJobs.lua`. |

Tests for the engine are Lua files in `resources/tests/feast_*.lua`, run by
`cargo test` (see [Testing](testing.md)). `resources/scripts/commands/debug/Feast.lua`
is the `!feast` command that moves players in and out of the arena.

### Client: `tools/`

| Directory | What it is |
|---|---|
| `tools/feast-hud` | The Kawari Feast HUD Dalamud plugin. See [The HUD plugin](hud-plugin.md). |
| `tools/feast-hud-tests` | Unit tests for the HUD's pure logic. |
| `tools/feast-header-loader-tests` | Checks the built HUD loads its header textures and matches the SDK's gauge layouts. |
| `tools/feast-map` | The client data patch (territory and duty rows). |
| `tools/feast-assets` | Read-only audits of which Feast assets survive in the client, and the scripts that built `feast-actions.json`. The `*.json` files there are recorded audit results. |

### Shared data: `resources/feast-actions.json`

One record per historical action of every job: ID, Lua key, name, icon, cast,
recast, cooldown groups, charges, range, radius, MP cost and description. The
server uses it to map IDs to Lua keys; the HUD embeds it for icons, tooltips
and the client's timing questions. See [Actions and casting](actions-and-casting.md).

## One world tick

The world server ticks every instance about twice a second
(`server_logic_tick` in `server/mod.rs`). For the arena, `feast::tick` runs:

1. **`arena::tick`**: let the lobby give new arrivals a team (`lobby::plan`),
   collect every participant (players and practice dummies)
   as a `rules::Input`, then `Match::step` the rules. Afterwards make the world
   match the rules:
   - restart the native duty timer if the phase changed;
   - set Marked for Culling stacks;
   - tick supply boxes;
   - give each player their team's battalion and make the parties match the
     teams (`lobby::sync_parties`);
   - open or close the jail gates and place new players in their jail;
   - spawn, animate and remove medal piles, dimmed per viewer;
   - send announcements, and director frames four times a second.
2. **`combat::tick`**: enable Feast rules for newly arrived players,
   then advance the Lua engine to now (finishing casts, ticking statuses) and
   apply its events.
3. **`practice::tick`**: dummies hit back if `!feast attack on`.

The arena takes itself out of the instance while it works
(`instance.feast.arena.take()`) and puts itself back at the end. That lets it
mutate the instance freely; it is not a concurrency mechanism.

## One button press

What happens when a player presses Glare in the arena:

1. **Client.** The hotbar slot holds historical action 17789. The HUD's hooks
   answer the client's questions about it from the catalog (cast 1.5 s, recast
   2.4 s, its icon, whether it is usable now). The client starts its cast bar
   and its GCD and sends a normal `ActionRequest`.
2. **Server routing.** `action.rs` sees the request is in the arena and calls
   `combat::native_action`.
3. **Checks.** `native_action` refuses if the match has not started or the
   player's job changed (`arena::can_act`), then advances the engine so that
   any cast finishing now finishes first.
4. **Engine.** It snapshots every actor into Lua (`sync`) and calls
   `M.command(actor, "glare", target, now)`. The engine checks range, MP,
   cooldowns, crowd control and so on. If it accepts, it starts the GCD and,
   since Glare has a cast time, records a pending cast and emits a `cast`
   event. Then `drain` returns the events.
5. **Reply.** If a `cast` or `animation` event came back, the server
   remembers the request (to acknowledge it later); otherwise it cancels the
   client's cast. It applies the events: the `cast` event becomes an
   `ActorCast` packet so everyone sees the cast bar.
6. **Completion.** A later tick's `M.advance` finds the cast finished (or
   interrupted, if the player moved too early). It applies damage, emitting
   `hp`, `damage` and an `animation` event. The server turns the animation into
   an `ActionEffect` that acknowledges the original request; the client ends
   its cast and plays the hit.

[Actions and casting](actions-and-casting.md) covers each step in detail.

## Design choices

- **Server authority.** Everything is decided on the server. The HUD only
  predicts (greying out a button, showing a recast) so the client feels
  native; the server checks everything again.
- **Pure rules, thin glue.** `rules.rs` and the Lua engine have no networking,
  so most behaviour is testable without a client. The glue (`arena.rs`,
  `combat.rs`, `effects.rs`) only translates.
- **Lua for combat.** There are hundreds of actions, each with its own rules.
  Lua keeps each one a few lines long, can be changed without rebuilding, and
  can be tested directly.
- **Reuse what survived.** The arena, animations, models, statuses and UI all
  come from today's client. New things are drawn with the client's own UI
  nodes rather than custom windows, except for the PvP Actions window.
- **Locks.** The world server holds two locks: the world data and the
  network state. Always take the data lock first; the reverse order can
  deadlock against the world tick. The Feast code runs inside the data lock
  and takes the network lock briefly per message.
