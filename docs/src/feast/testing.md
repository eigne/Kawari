# Testing and debugging

Most Feast behaviour can be tested without the game. Run the automated tests
after every change; then check in game what they cannot cover.

## Automated tests

```powershell
# Server: match rules, arena, supplies, practice, director frames, and the
# Lua engine tests below.
cargo test -p kawari-world --lib

# Only the Feast tests.
cargo test -p kawari-world --lib feast

# HUD: frame decoding, menus, catalog, gauge mapping, recast charges.
dotnet run --project tools/feast-hud-tests/Tests.csproj

# HUD: the built plugin loads its header textures; gauge layouts match the SDK.
dotnet run --project tools/feast-header-loader-tests -- tools/feast-hud/bin/Release/net10.0-windows

# Client data patch tools, against synthetic archives.
python -m unittest discover -s tools/feast-map -v
```

The Lua engine tests are plain Lua files in `resources/tests/`, run by Rust
tests in `server/feast/combat.rs`. Each returns how many checks it ran:

| File | Covers |
|---|---|
| `feast_white_mage.lua` | Every White Mage action, lilies, casting, slidecasting, leeway, interruption, statuses. Uses `make_engine()`: an engine with only `Engine.lua`. |
| `feast_jobs.lua` | Black Mage, Samurai, Paladin, Warrior, Dragoon, and the shared hooks. Uses `install_jobs(engine)` to add the job files. |
| `feast_all_jobs.lua` | Every job: kits, party actions, hotbars, gauges, and specific mechanics. |

The pattern in all three: build an engine, `sync` a few actors, `command` an
action, `advance` time, and check HP, MP, statuses and players' resources.

`navmesh.rs` has a test that the jails connect to the centre of the arena;
it only runs if you have generated the navmesh.

## In game

Things only a client can confirm. After a change, check the ones it touches:

- `!feast` pops the duty; you arrive in a jail; the header and team lists
  appear; the gates open after the countdown and close for the next match.
- Opponents' names are red, Tab targets them, enemy-only actions work on
  them; a party that queued together is split.
- Casting: the cast bar and animation play; moving early cancels, moving in
  the last half second does not; the recast sweep matches the server (no
  rollback when an action is accepted); queued actions are not refused.
- Hotbar icons, tooltips, combo icons and charges; the job gauge moves.
- Medals: KO drops a pile, enemies collect it with the pickup animation,
  the header, nameplates and target bar show the counts.
- Respawn: the confirmation counts down from ten and then returns you to
  your jail at full HP.
- Leaving the arena restores your normal hotbars and UI.

`!feast bots` gives you a full match alone; `!feast supply` tests supply
boxes without waiting.

## Logs

- **Server**: the console of `kawari-run`. Feast lines include `Feast match`
  (announcements), `Feast damage` / `Feast healing`, `Feast team battalion
  changed`, `Disbanding a party split across Feast teams`, and errors from the
  Lua engine (`Feast tick failed`, `Feast command failed`) with the Lua stack.
- **HUD**: Dalamud's log (`%APPDATA%\astra\log\dalamud.log` with Astra).
  Every line from the plugin starts with "Feast".
- **Engine replies**: the engine's refusals ("Global cooldown is active.",
  "Target is out of range.") are sent to the player as chat notices.

## Common problems

| Symptom | Likely cause |
|---|---|
| `!feast` says the duty metadata is missing | The client data patch is not installed, or a game update replaced it. |
| No Feast header or team lists | Exploration mode (no duty patch), the HUD is not loaded, or the HUD refused an unknown client build (check its log). |
| Opponents are blue / untargetable | The HUD (BattalionMode) is not loaded, or you are still in a party with them. |
| Actions are greyed out | Preparation phase, wrong job, a missing resource, or no fresh director frames. |
| Every action is refused with "not in the Feast kit" | The hotbar holds a non-Feast action; restore the default layout in the PvP Actions window. |
| A Lua change has no effect | The engine is per match: `!feast restart` after a result, or leave and re-enter. |
