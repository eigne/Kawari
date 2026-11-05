# Jobs: finishing and adding them

All 19 jobs that existed before patch 6.1 are playable: Paladin, Monk,
Warrior, Dragoon, Bard, White Mage, Black Mage, Summoner, Scholar, Ninja,
Machinist, Dark Knight, Astrologian, Samurai, Red Mage, Gunbreaker, Dancer,
Reaper and Sage. Every action in their Feast kits has an implementation.
"Implemented" does not mean "exact", though: see [How faithful the kits
are](#how-faithful-the-kits-are) before trusting a number.

## Where a job is defined

| What | Where |
|---|---|
| Action metadata (ID, name, icon, cast, recast, groups, range, MP) | `resources/feast-actions.json` (all jobs); also `resources/scripts/feast/AdditionalActionData.lua` for the 13 jobs in `AdditionalJobs.lua` |
| Action effects | `Engine.lua` (WHM), `Jobs.lua` (BLM, SAM, PLD, WAR, DRG), `AdditionalJobs.lua` (the rest) |
| Combos and procs | `M.resolve` in `Jobs.lua` / `AdditionalJobs.lua` |
| Role actions | `roles` in `Jobs.lua` (tank, melee, caster, healer) and `AdditionalJobs.lua` (job → role, plus physical ranged); mirrored in the HUD's `FeastActionMenu.RoleActions` |
| Job gauge | player fields, `M.job_gauge`, and the HUD's `NativeJobGaugeData` / `JobGaugeManagerSync` |
| Medal limit | `rules::medal_limit` (server) and `Participant.MedalCapacity` (HUD) |
| Bot rotation | `M.bot_action` |

## How faithful the kits are

The kits follow the official job guides archived before 6.1, but those guides
describe actions in words, so some details had to be chosen:

- **Damage is potency as HP.** A 1200-potency Glare deals 1200 HP before
  modifiers. The real game scaled potency with stats; since every Feast player
  had the same stats, this keeps the ratios right but the absolute numbers are
  a guess, and so is the shared maximum HP (`FEAST_MAX_HP`).
- **Shapes.** Cones are 90 degrees and lines two yalms wide; the guides give
  only the range.
- **Scaling curves.** Proximity bonuses (dance finishes, Pneuma) and
  missing-HP bonuses are linear; the guides give only the end points.
- **Assists.** Ninja and Dark Knight cooldown resets count anyone who damaged
  the target in the last ten seconds.
- **No line of sight or collision** for dashes and knockbacks.
- **Presentation.** Only crowd control, Dia, Temperance, Wolf's Heart and
  Bolt show native status icons. Other buffs and debuffs work but are
  invisible, and many animations have not been checked in game.
- **Bots** play White Mage, Paladin, Samurai and Black Mage with real
  rotations; other jobs only use their basic combo.

Search the Lua for "approximation", "provisional" and "guide specifies" to
find each of these choices.

## Implementing or fixing an action

Say Sidewinder should deal more damage to targets below half HP and you want
to change it.

1. **Find it.** Search the Lua for its key (`'sidewinder'`). Keys are the
   action's English name in lower case with underscores; the catalog has the
   key for every ID.
2. **Read the guide text.** The catalog's `description` is the archived
   tooltip. Older tooltips are in `tools/feast-assets/historical-job-sources.json`.
3. **Change the effect function.** Use the helpers: the engine's are in
   `M.internal` (`damage`, `heal`, `buff`, `has`, `allies`, `enemies`, ...),
   and each job file defines a few more at its top (`gain`, `shape`, `chain`,
   `require_resource`, ...):

   | Helper | Does |
   |---|---|
   | `damage(a, b, n)` | Deals `n` through every modifier; returns the amount dealt. |
   | `heal(a, b, n)` | Heals through healing modifiers. |
   | `buff(a, b, name, seconds, potency)` | Adds a status (a heal-over-time if `potency`). |
   | `has(id, name)` | Whether a status is active. |
   | `area(a, b, radius, n, extra)` / `shape(...)` / `cone(...)` | Damage every enemy in an area, line or cone. |
   | `allies(a, center, r)`, `enemies(a, center, r)` | Living actors around a point. |
   | `gain(a, key, n, cap)` | Change a job resource. |
   | `move(a, x, y, z)`, `step(a, n)` | Move an actor. |
   | `combo(a, key)`, `setcombo(a, key)` | Combo tracking. |
   | `require_resource(key, n)`, `require_buff(key)` | Ready-made `check` functions. |

4. **Make buffs matter.** A new status only does something if a hook reads
   it: add it to `M.outgoing` (damage dealt), `M.incoming` (damage taken),
   `M.effective` (cast/recast/MP), `M.healing_received` or `M.immune`.
5. **Show it.** For a native status icon, emit an event the server turns into
   a status (see how `dia_visual` works in `Engine.lua` and `effects.rs`). The
   6.08 PvP status IDs still exist in the client.
6. **Test it** in `resources/tests/feast_all_jobs.lua` (or `feast_jobs.lua`
   for the five jobs in `Jobs.lua`): set up actors, use the action, check HP
   and statuses. Then `cargo test -p kawari-world feast`.
7. **Try it in game.** Script changes apply at the next match: `!feast
   restart` after a result, or leave and re-enter.

If you change an action's numbers (cast time, recast, range), change them in
**both** `feast-actions.json` and the Lua definition (for the 13 jobs in
`AdditionalJobs.lua`, in `AdditionalActionData.lua` instead). The HUD tells the
client the catalog's timings; if they disagree with the server's, the
client's recast sweep will not match what the server enforces. Rebuild the
HUD after changing the JSON.

## Adding a combo step or proc

A button that turns into something else (Raiden Thrust → Full Thrust) is two
actions plus a rule:

1. Define both actions; the second usually has `combo=true` and a `check`
   requiring the first (`chain` in `AdditionalJobs.lua` does this for you).
2. In `M.resolve`, return the second key when the first is the player's last
   combo action.
3. Add the second key to `Derived` in the HUD's `FeastJobActions.cs`, so it
   gets no button of its own. The server's hotbar records swap the icon.

## Adding a job

The Feast ended before Viper and Pictomancer, so adding one means designing
a kit, but the steps are the same for any job.

1. **Choose action rows.** The client can only animate actions it has rows
   for. Use rows whose animations exist (a job's PvE or Crystalline Conflict
   actions), and check them with the `tools/feast-assets` audit.
2. **Catalog.** Add a record per action to `resources/feast-actions.json`,
   with `job`, a unique `key`, the row `id`, and the timing fields. `group`
   and `secondary` are zero-based cooldown groups (57 is the GCD).
3. **Engine.** Add the actions in a job file (follow `AdditionalJobs.lua`:
   `action(key, effect, extra)` with metadata in `AdditionalActionData.lua`),
   resources in `M.reset_job`, ticking in `M.advance_job`, combos in
   `M.resolve`, a gauge in `M.job_gauge`, a basic rotation in
   `M.bot_action`, and the job's role in the `roles` table. Add its adrenaline
   rush to the role's list and to the table in `M.party_actions`.
4. **Server.** Add the job ID to `feast::JOBS` (`server/feast/mod.rs`) and to
   `rules::medal_limit`.
5. **HUD.** Add the job to `FeastJobActions.Supports`, `Participant`
   (abbreviation and medal capacity), `FeastActionMenu.RoleActions`, the `F7`
   bounds in `DirectorFrames.Accept`, and its gauge to `NativeJobGaugeData`
   and `JobGaugeManagerSync`.
6. **Tests.** Add the job to the loops in `resources/tests/feast_all_jobs.lua`
   and `tools/feast-hud-tests/Program.cs`, which check every job has a kit,
   icons, a gauge and a valid default loadout.

The job ID lists in steps 4 and 5 are deliberately simple copies rather than
one shared table across three languages; the tests in step 6 catch most
omissions.
