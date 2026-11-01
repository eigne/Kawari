# BLM, SAM and PLD — final Feast kits

Current roster: [all 19 historical PvP jobs](feast-all-jobs.md). This page retains
the earlier implementation notes for the first six jobs.

## September 27: Warrior and Dragoon added

Verified CDX captures strictly before April 2022:

- [Dragoon, March 23 2022](https://web.archive.org/web/20220323203055/https://na.finalfantasyxiv.com/jobguide/dragoon/#pvp)
- [Warrior, March 31 2022](https://web.archive.org/web/20220331171726/https://na.finalfantasyxiv.com/jobguide/warrior/#pvp)

Cached pages: `target/feast-more-jobs/{dragoon,warrior}.html`. Reproduce catalog
extraction with `tools/feast-assets/import-war-drg.py` against those pages and
the recovered 6.08 Action CSV. The shared catalog adds51 records:33 job actions
and18 shared/role actions. `tools/feast-assets/war-drg-7.51h2.json` records the
installed-client audit; all new job action timelines are present. No fabricated
replacement animations or edits to game archives.

DRG implements four-step and line combos, Blood/Life duration extensions, two
eyes, Geirskogul/Nastrond, charged Mirage Dive and its jump-triggered recast
reductions, jumping attacks/escape, Life Surge and Dragon Sight. WAR implements
combos, Beast generation/spending, charged Infuriate, Chaos substitutions,
cooldown reductions, Onslaught, Holmgang's bind/HP floor, Inner Release immunity,
Nascent Flash/Glint and Rampart-enhanced Shake It Off. Shared melee/tank role
actions and adrenaline are available to both jobs with role/job validation.

Native hotbars use original IDs/icons, exclude combo follow-up buttons, and use
the existing F6 substitution mechanism. F7 carries WAR Beast or DRG eyes/stance/
remaining timer (up to30s); writes target the retained current-client gauges.
Modern DRG's removed Blood stance renderer is not restored here; exact gauge
appearance still requires manual verification. Basic rotation choices exist,
but the default four-job practice-team composition remains WHM/PLD/SAM/BLM.

Tests exercise combos, resources, charge exhaustion/recovery, eye spends, stance
switches, Life Surge consumption, bind restrictions, Chaos/free spending,
Holmgang survival, shared heals, shields, role loadouts and HUD records.
57 world tests and77 HUD checks pass. No computer use/live validation performed.

Additional assumptions: line width is a provisional two-yalm corridor; Piercing
Talon interpolates800–1600 over0–15y. The archived guide states shape and potency
range but not those exact geometries/formulae. Native buff icon replication,
collision-aware displacement and historical stat scaling remain broader gaps.
These two jobs have implemented gameplay kits, not a claim of perfect fidelity.

## Full roster update

All 19 historical PvP jobs are now enabled. See [the full roster implementation,
pre-April sources and remaining fidelity limits](feast-all-jobs.md). The sections
below describe the earlier six-job implementation and its history.

## Sources and reused client data

Rules are from the user-specified official guides:

- [Black Mage, April 6 2022](https://web.archive.org/web/20220406205457/https://na.finalfantasyxiv.com/jobguide/blackmage/#pvp)
- [Samurai, April 6 2022](https://web.archive.org/web/20220406205457/https://na.finalfantasyxiv.com/jobguide/samurai/#pvp)
- [Paladin, April 7 2022](https://web.archive.org/web/20220407184056/https://na.finalfantasyxiv.com/jobguide/paladin/#pvp)

Original action IDs, icons, recast groups, charges and timings were cross-checked
against the recovered 6.08 Action sheet. `resources/feast-actions.json` is shared
by the server and HUD module. Run the asset auditor with `--jobs` to repeat the
installed-client check. `tools/feast-assets/jobs-7.51h2.json` records 70 distinct
actions and 1,301 surviving dependencies. The only absent timeline is Tsubame's
empty wrapper; it resolves to the original Kaeshi actions, whose assets exist.
The game files are unchanged.

The BLM page contradicts itself: Enochian's action description specifies 15s
for Polyglot, while the gauge prose says 30s. This implementation uses the
action description's 15s. Cone actions use a provisional 90-degree sector;
the guide provides range and shape but no angle. Soten uses a one-yalm corridor.

## Implemented

- BLM: Astral Fire/Umbral Ice, Enochian, Polyglot, Thunder damage-over-time and
  damage-triggered Thundercloud, spell transformations, sleep, ally teleport,
  MP recovery, role actions, barriers, and Cometeor.
- SAM: single-target/area combos, Kenki, Meditation, shared Iaijutsu timer,
  charged Tsubame/Kaeshi follow-ups, counters, movement, role actions, and
  Raw Destruction.
- PLD: combos, Oath, Sword Oath substitutions, spell lifesteal, Requiescat,
  barriers, Intervention/Rampart interaction, Hallowed Ground, knockback/pull,
  role actions, and Aegis Boon.
- Shared engine validates job/role, targets, team, range, resources, cast
  interruptions and cooldowns. Existing WHM behavior remains covered by tests.
- Native hotbars use historical IDs/icons for these jobs. F6 director records
  drive transformations and cast/recast timing; F7 drives the existing native
  Oath, Kenki/Meditation and BLM gauges. Both are part of the atomic frame.
- Party action icons/cooldowns include the new roles. Mixed practice teams use
  WHM/PLD/SAM/BLM with simple resource-aware rotations.

## Use

Equip a supported job before queuing. `!feast bots` fills each team with the
supported jobs, omitting the player's job from the three allied bots.
Hotbars populate automatically inside territory 745.

- `!feast actions`: action names for the current job.
- `!feast job`: current job resources.
- `!feast loadout <first> <second>`: select two additional role actions during
  the preparation phase only. Names use underscores, e.g. `swiftcast manaward` or
  `rampart retaliation`. Players normally choose them in the Wolves' Den with the
  HUD's PvP Actions window, which sends this command for them.

Defaults are Drain/Addle, Feint/Bloodbath, and Full Swing/Weapon Throw.
The first combo button progresses through the combo; Enochian and Tsubame
transform according to server state. Three normal bars may be populated.

## Boundaries and live acceptance

Potencies still use the existing prototype's fixed HP amounts rather than a
reconstructed historical stat formula. LoS, collision-aware displacement and
complete native status-icon presentation remain broader restoration work.
Current-client gauges are reused, not the original 6.08 gauge layouts.

Kawari has no generic ground-location action IPC handler. For Soten and Cometeor,
the client module retains the native cursor and sends the selected coordinates
through a territory-scoped `!feast ground` bridge. The server validates them and
executes the same rules engine. This needs live validation, especially Cometeor's
ground-impact visual. Other actions continue through native ActionRequest.

No live playback, ground selection, gauge rendering or targeting success is
claimed from unit tests. Client restart through Astra is needed for the new HUD
module. Regression checks cover resource transitions, combos, cast interruption,
shared cooldowns, Thundercloud, defenses, ground range/path damage, bot choices,
native IDs and atomic HUD records. Cross hotbars remain unsupported.
