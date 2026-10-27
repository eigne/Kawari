# Complete historical Feast job roster

## Scope

The server now enables all 19 jobs that had PvP kits before April 2022:
PLD, WAR, DRK, GNB, WHM, SCH, AST, SGE, MNK, DRG, NIN, SAM, RPR,
BRD, MCH, DNC, BLM, SMN and RDM. Blue Mage had no Feast PvP kit;
Viper and Pictomancer were introduced after the cutoff and remain excluded.

The additional 13 kits define 214 job actions, plus the seven ranged role/LB
actions. Each action has an explicit gameplay implementation; missing handlers
fail engine initialization. Existing role actions are shared with job permissions
and the two-action loadout limit. Party slot zero remains role adrenaline.

## Verified official archive captures

These are actual CDX capture timestamps, not guessed dates. Cached HTML is under
ignored `target/feast-more-jobs`; SHA-256 hashes and URLs are recorded in
`tools/feast-assets/historical-job-sources.json`.

| Job | Capture |
| --- | --- |
| PLD | [2022-03-31](https://web.archive.org/web/20220331171153/https://na.finalfantasyxiv.com/jobguide/paladin/#pvp) |
| WAR | [2022-03-31](https://web.archive.org/web/20220331171726/https://na.finalfantasyxiv.com/jobguide/warrior/#pvp) |
| DRK | [2022-01-20](https://web.archive.org/web/20220120041635/https://na.finalfantasyxiv.com/jobguide/darkknight/#pvp) |
| GNB | [2021-12-06](https://web.archive.org/web/20211206201441/https://na.finalfantasyxiv.com/jobguide/gunbreaker/#pvp) |
| WHM | [2022-03-16](https://web.archive.org/web/20220316120017/https://na.finalfantasyxiv.com/jobguide/whitemage/#pvp) |
| SCH | [2022-02-12](https://web.archive.org/web/20220212051122/https://na.finalfantasyxiv.com/jobguide/scholar/#pvp) |
| AST | [2022-01-23](https://web.archive.org/web/20220123050826/https://na.finalfantasyxiv.com/jobguide/astrologian/#pvp) |
| SGE | [2022-01-22](https://web.archive.org/web/20220122081244/https://na.finalfantasyxiv.com/jobguide/sage/#pvp) |
| MNK | [2022-02-19](https://web.archive.org/web/20220219223313/https://na.finalfantasyxiv.com/jobguide/monk/#pvp) |
| DRG | [2022-03-23](https://web.archive.org/web/20220323203055/https://na.finalfantasyxiv.com/jobguide/dragoon/#pvp) |
| NIN | [2022-02-08](https://web.archive.org/web/20220208042258/https://na.finalfantasyxiv.com/jobguide/ninja/#pvp) |
| SAM | [2022-03-19](https://web.archive.org/web/20220319115951/https://na.finalfantasyxiv.com/jobguide/samurai/#pvp) |
| RPR | [2022-02-19](https://web.archive.org/web/20220219223422/https://na.finalfantasyxiv.com/jobguide/reaper/#pvp) |
| BRD | [2022-02-12](https://web.archive.org/web/20220212040940/https://na.finalfantasyxiv.com/jobguide/bard/#pvp) |
| MCH | [2022-01-26](https://web.archive.org/web/20220126103602/https://na.finalfantasyxiv.com/jobguide/machinist/#pvp) |
| DNC | [2022-03-01](https://web.archive.org/web/20220301211525/https://na.finalfantasyxiv.com/jobguide/dancer/#pvp) |
| BLM | [2022-02-19](https://web.archive.org/web/20220219223519/https://na.finalfantasyxiv.com/jobguide/blackmage/#pvp) |
| SMN | [2022-03-06](https://web.archive.org/web/20220306170309/https://na.finalfantasyxiv.com/jobguide/summoner/#pvp) |
| RDM | [2022-03-26](https://web.archive.org/web/20220326173253/https://na.finalfantasyxiv.com/jobguide/redmage/#pvp) |

WHM/BLM/SAM/PLD action descriptions were compared with the earlier April captures:
the mechanics match (some archive copies have a damaged annotation marker).
Reaper and Sage are intentionally included: their PvP actions were introduced
in [patch 6.0](https://na.finalfantasyxiv.com/lodestone/topics/detail/bdd208b52ddababad086dc9679e96a8412962edf).

## Code map

- `resources/scripts/feast/AdditionalJobs.lua`: new job rules, shared effects,
  combos, resources, charged/shared recasts, transformations, delayed healing,
  Wildfire, dancing, Kardia, Haima, native crowd-control/stealth synchronization.
  `Jobs.lua` retains the six earlier kits; `WhiteMage.lua` remains the shared
  action validator, cast lifecycle, damage/healing and status tick engine.
- `AdditionalActionData.lua`: generated native metadata used to register new
  actions. `tools/feast-assets/import-remaining-jobs.py` imports it and the shared
  `resources/feast-actions.json` from cached guides and the recovered 6.08 sheet.
  Fixed PvP MP costs use sheet cost type 51, not the PvE percentage cost type.
- `servers/world/src/server/feast.rs`: loads both rule modules, enables the
  historical jobs, routes Shukuchi ground coordinates and native control effects.
  Existing native action/effect packets carry original animation IDs and hits.
- `FeastJobActions.cs`: eligibility and base hotbar buttons. Derived actions
  replace their base buttons instead of occupying extra slots.
- `FeastHotbars.cs`: historical icons/tooltips/cooldowns and native gauge fields.
  `DirectorFrame.cs`: atomic frames accept up to 64 action records (Reaper needs
  more than the previous 32); bounded gauge fields cover the larger job roster.
  `FeastResources.cs` recognizes Terminal Velocity as adrenaline.
- `resources/tests/feast_all_jobs.lua`: all-job entry, metadata/hotbar bounds,
  resource/charge consumption, combos, transformations, barriers, partner heals,
  expiration, cast interruption, native controls and adrenaline regression tests.

## Native asset reuse

The installed-client audit covers **342 distinct historical actions** and confirms
all 342 historical icon textures are present. Their
action timelines and referenced dependencies are present. Only the empty wrapper
timelines for Tsubame-gaeshi and Continuation are absent; neither executes as an
attack, and the actual Kaeshi/Rip/Tear/Gouge timelines exist. The compact report is
`tools/feast-assets/all-jobs-7.51h2.json`. No game binaries or asset payloads are
copied into source control.

Native gauge storage is updated for the supported jobs, including Dancer's
randomized step sequence and Reaper's two resources/Enshroud stacks. The existing
WHM renderer hook and native adrenaline display remain in place. The new jobs
reuse current native gauge renderers where compatible; their in-game display
is not yet visually verified. Removed historical gauge layouts (notably Huton
and the older SMN trance presentation) have not been reconstructed here.

## Validation and remaining fidelity limits

All 57 world tests and 124 HUD checks pass; Release HUD compilation passes.
No computer use or live playback was performed, per the user's instruction.

The following existing prototype limits are **not** resolved by adding jobs:

- Damage uses fixed potency-as-HP values rather than the historical stat formula.
- Cone angle (90 degrees), line width (two yalms), proximity scaling for dance
  finishers/Pneuma and missing-HP scaling are explicit approximations where the
  guide specifies endpoints but not the exact function.
- Movement has no collision/line-of-sight reconstruction. Assist cooldown resets
  currently count contributors in a ten-second window, pending historical packet
  evidence for the exact assist window.
- Native crowd-control and Ninja stealth statuses are sent, but actual visual
  stealth, every retained animation, new gauge rendering and ground targeting
  still need in-game verification. Complete native presentation of every job
  buff/debuff is still broader restoration work.
- Modern gauge layouts are not evidence that the 6.08 layout has been restored.
- Training bots retain their existing four-job team composition. Other supported
  jobs have a basic fallback rotation if used by an actor; advanced bot tactics
  are not included in this change.

Equip the desired job before entering Feast. `!feast actions`, `!feast job` and
`!feast loadout <first> <second>` use the same shared rules for all 19 jobs.
