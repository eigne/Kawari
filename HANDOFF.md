# Feast restoration handoff — start here

Updated 2026-09-15 (Asia/Tokyo). Workspace: `C:\dev\kawaridev\Kawari`.

## New continuation state (supersedes deployed-state notes below)

COMMITTED through current native HUD work at user's request: c7c41846 duty entry,
5daa44ae rules/director transport,6cab7e56 client HUD,1b84350c documentation.
Oodle binary intentionally left untracked. Latest asset investigation finds all
40 end/hit references for20 historical WHM actions and281 discovered dependency
paths still present in June7.51h2. Protect17832 retains timeline451/1886, original
protect_hit AVFX and Protes sound. Old rows keep names/timelines but Icon784 and
IsPvP=false. New read-only tools/feast-assets audit +manifest records findings;
details atop feast-white-mage.md. No action IDs or assets restored by this audit.
Next action work: restore metadata/icons/tooltips and native original-ID routing,
starting with live Protect caster/target playback; verify cast/recast/charge
groups before swapping the entire kit. Native party follow-up still awaits test.

LATEST CLIENT-ONLY FIX: User saw player/enemy lists reversed and cooldown numbers
without dimming/radial animation. NativeDirector now explicitly assigns addon
PartyList1 local Feast team and PartyList3 opposing team through verified +239
before RequestedUpdate/Draw. Actor data and click lookup share that orientation.
NativeRecast.cs dims IconText's image at +C0, adds a noninteractive native image
using built-in ui/uld/icona_recast.tex (ActionBar.uld partlist4 layout: 9columns,
44x46 parts, 48px pitch, progress0..79 excluding completion flash80). Uses
authoritative cooldown duration30/45s with interpolation between snapshots.
Owned texture/parts/node detached and freed on addon PreFinalize, territoryexit,
and module Dispose. Does NOT cast IconText to Icon (different layouts).
Release DLL rebuilt at registered path;24protocolchecks pass. No server changes
or restart needed. Client was absent at last process check; user should launch
through Astra to load it. Team orientation and circle visuals await live test.

DEPLOYED NATIVE PARTY LIST CANDIDATE (latest request): User wants real ally/enemy
party rows and additional-action cooldowns with reliable click targeting.
Implemented NativeRoster.cs feeding native arrays117/105. Party addon drawing
and RequestedUpdate restored; CC producer still bypassed, graphite header kept.
Native click callbacks use actor-ID path for BOTH teams via number[3]=-1;
no managed click handler/ImGui list. Names/jobicons/HP/MP from loaded actors,
stale/departed rows hidden, current target highlight fed. F4 atomic records
add WHM Purify+selected2 cooldowns using Lua authoritative deadlines. Historical
icons from cached6.08ActionCSV. Other jobs' kits and action tooltips not included.
Native contracts/addresses recorded atop feast-director-investigation.md.
HUD DLL built at registered path;44worldtests+24clientprotocolchecks pass.
User confirmed "exited"; client/Astra absent and fresh DB online count0 verified.
Oodle production world build passed and services restarted 2026-09-15T12:22:21Z;
world ready12:22:27Z PID82472, runner77368. All8services started, stderr empty.
Prior logs saved target/pre-feast-native-party-server.log and -stderr.log.
Registered HUD DLL includes candidate. Live native row rendering, both-team
click targeting, cooldown icons and leave cleanup still need user verification.

CURRENT DEPLOYED CANDIDATE: user requested removing the left/right medal widgets
and replacing graphite footer Carried/OnField with each team's JOB medals list.
Done in HUD DLL: all three native PvP addons have drawing/CC updates suppressed;
native text node injection removed. Graphite footer shows up to4 job+medal pairs
per team, using the director roster, including KO players' carried medals.
User also requested cancelled casts leave no GCD. Lua now refunds provisional
cast-start GCD on actual interruption/failed completion/death and emits reset_gcd.
Server translates it to SetCooldownTimer group57 elapsed0 total0. Successful
casts retain existing start-to-start timing; rejected inputs cannot reset GCD.
Movement/interruption retry and invalid completion regression tests pass. Earlier
test expecting cancellation to retain GCD updated to the new requested behavior.
HUDRelease/19protocoltests pass; world43tests plus corrected WHM suite pass.
User exited and confirmed. Server restarted at11:45:19Z, world ready11:45:25Z;
all8services started and stderr empty. No new SqPack changes. Prior logs saved
as target/pre-feast-cancel-refund-server.log and -stderr.log. HUD DLL is at the
registered path. Live compact-footer/cancel-refund retest remains pending.
User additionally asked to audit WHM spells against their April9,2022 Wayback
guide. Exactpage fetched again to target/feast-whm/guide-recheck.html; SHA256
28CDAE962E8D958BB2CE86E1784374F375B69A0B166DA019DB96068E4A1577C4 matches cache.
All20server action definitions checked against PvP rows. Core values/effects match;
client still has PvE placeholder IDs/names/tooltips/icons/gauges. Full mapping and
known limitations now at top of docs/src/development/feast-white-mage.md.

LATEST FOLLOW-UP: User confirmed native director candidate generally works,
but reported repeated cast cancellations/GCD notices and rejected native text
clock/score visuals, asking to restore the earlier EVE-like graphite HUD.
Updated HUD DLL has been built at the registered path; requires client restart.
No server restart/change needed for this follow-up. Plugin now always draws the
original graphite ImGui clock/medal panel from director frames; native header
PreDraw is suppressed, while CC refresh suppression/team widgets remain. Personal
medal cap is derived from the participant's server-supplied job (WHM125).
FeastHotbars GetActionStatus now respects casting/recast flags, returning native
0x244 for active casting and 0x246 for active recast. Previous unconditional0
incorrectly bypassed these timing gates and let rejected requests start locally.
Native status branch/return codes verified in June binary at RVA8E82E8/8E87B3.
Server GCD is unchanged:2.4s from cast start, Glare1.5s. New Lua regression test
proves completion does not restart GCD and next Glare is accepted at2.4s.
HUD build,19protocoltests and targeted WHM Lua suite pass. Live retest pending.

LATEST: User approved extending Kawari.FeastHud (the existing Dalamud plugin) for
native client hooks. No need to preserve CC in this Feast-only fork. Native UI
candidate built and deployed after user said "exited"; no game/Astra processes
and DB online count zero were verified first. Services restarted with new world
binary. No further SqPack edits. Existing Astra profile loads the updated DLL.
All44worldtests, 19clientprotocolchecks, HUDRelease and Oodleworldbuild passed.

New files: server/feast_director.rs (native snapshot/roster transport), HUD
NativeDirector.cs (verified CC producer replacement and native addon text nodes),
DirectorFrame.cs (atomic parser), FeastHotbars.cs (temporary WHM PvP slots and
native action-status override). Read docs/src/development/feast-director-investigation.md
and tools/feast-hud/README.md newest sections for format, verified addresses and
limitations. This is NOT restored historical art or complete HP/status/target
panels; it is a native text presentation candidate. Live rendering, unbinding
suppression, hotbar swaps and native casts remain to be user-tested. Old separate
Kawari.Feast combat plugin stays disabled. Previous logs saved as
target/pre-feast-native-ui-server.log and target/pre-feast-native-ui-stderr.log.

Sastasha duty entry is now user-confirmed after `!unlockcontent all` and relog.
`!unlock all` did not unlock duties. Enemies appeared; Stone action 127 was logged
as unscripted. The normal InstanceContent 4 entrance event returned successfully.

Feast duty-entry candidate installed with user authorization after closing FFXIV.
World restarted at 2026-09-15T10:40:01Z; all eight services started, stderr empty.
Working tree now contains uncommitted duty-entry changes; do not discard them.
Current metadata is CFC 476 / InstanceContent 40022 / territory 745 / script l1p2,
the recovered historical pairing. Intended use is now 28 and PvP is enabled.
`!feast` queues through Duty Finder acceptance; `!feast leave` uses saved duty return.
The user tested entry and exit successfully. Character is visible. Read-only live
probe confirmed active legacy vtable RVA 0x234E100. However client presentation is
CC: Astra/Umbra, invalid negative percentages, empty team panels, CC hotbars and
repeated Unbinding in 0s. Do NOT treat this as working native Feast presentation.
User confirmed !feast leave returns to New Gridania and clears all PvP UI/alert,
restoring normal hotbars. Town queue guard was fixed live to accept intended use0
as well as OpenWorld1. Current remaining issue: shared client PvP/CC consumers.
No native score/team protocol is implemented yet; l1p2.lua is lifecycle scaffolding.
The old attach-after-teleport director-on command remains disabled.

Backup: backups/feast-duty-7.51h2. Restore via
tools/feast-map/Install-FeastDuty.ps1 -Restore with game/launchers/server closed.
This restores the prior exploration metadata. Never restore the older exploration
patch on top of an active duty patch. Pre-install logs: target/pre-feast-duty-server.log
and target/pre-feast-duty-stderr.log. Validation: 42 world tests, cargo check,
Oodle build and three installer transaction tests passed.
Full current details: docs/src/development/feast-director-investigation.md.

## Immediate objective and user direction

Restore Feast PvP on the current FFXIV client. User wants native server/client
integration where possible, avoiding additional plugin workarounds. They suggested
our failed director test might be caused by teleporting into the arena instead of
using the proper Duty Finder entry path. Investigation confirms this is plausible,
but the exact failure cause has NOT been established.

Next: verify Sastasha through Duty Finder as a baseline, then investigate restoring
Feast duty metadata and using Kawari's normal JoinContent/zone/director lifecycle.
Do not reactivate the old attach-after-teleport probe as if it were a working solution.
The user requested this handoff to continue in a new chat; no new test is running.

## Current deployed state

- Branch `master`, latest commit **d102e2f9**: disables experimental director activation.
- Server restarted successfully: hidden kawari-run PID **74536**. All eight services
  started; world ready at **2026-09-15T10:19:49Z**, startup stderr empty.
- Working tree clean except intentional untracked `HANDOFF_FEAST.md` and
  `oodle-network-shared.dll`. Preserve both. This new HANDOFF.md is also intentional.
- User was told to fully restart FFXIV to clear experimental client director state.
  They have not confirmed that restart or tested the recovered server yet.
- `!feast director on` is disabled in BOTH command and connection handlers.
  `off` remains available. Existing working Feast rules remain deployed.

## Latest failure — distinguish observations from hypotheses

User ran `!feast director on`, saw no visible change, relogged and had an invisible
character, then queued for Triple Triad Battlehall and disconnected.

Logs confirm probe activation at 10:15:03 UTC, a new zone load around 10:15:26,
then timeouts. The server process remained alive, with no recorded panic. Possible
server stall/deadlock or incomplete client duty state; neither is proven. One online
DB flag remained set after the reported disconnect. User explicitly requested the
restart, so it was performed despite that stale-looking flag.

Preserved failure logs:
- `target/director-failure-server.log`
- `target/director-failure-stderr.log`

Do not tell the user the crash cause has been fixed. Only the experimental command
was disabled and services restarted. The test's missing duty lifecycle is a strong
reason to redesign the next experiment, not a confirmed diagnosis of the disconnect.

## Confirmed director findings

Read **docs/src/development/feast-director-investigation.md** for details.

Installed client: Windows **7.51h2**, build `2026.06.18.0000.0000`.
Client location:
`C:\Program Files (x86)\SquareEnix\FINAL FANTASY XIV - A Realm Reborn\game`.

- Territory **745**, Crystal Tower Training Grounds (`l1p2`), is restored as an
  exploration territory: intended use 1, ContentFinderCondition (CFC) 0.
- `!feast` currently teleports there, bypassing real duty creation.
- Current InstanceContent 40001 retains type 5 and CFC 121, but CFC 121 is blank.
  Former 40002–40026 rows have cleared metadata.
- IMPORTANT: cleared metadata does NOT mean the old native director is gone.
  Reading the running client's factory map and installed executable proved that
  **40001–40026 use a surviving legacy constructor**, distinct from CC 40027–40047.
- Factory 0x8003: RVA 0x1AA4180. Legacy branch RVA 0x1AA7619 allocates 0x20F8,
  ctor RVA 0x19AD440 -> RVA 0x1A378C0, derived vtable RVA 0x234E100.
  CC branch RVA 0x1AA764D allocates 0x23C0, ctor RVA 0x1A3ACB0.
- Legacy overrides include nonempty content-update handlers (vtable slots 381/404).
  This proves executable support remains, NOT full runtime compatibility.
- Type-5 getters are shared and may allow downstream CC assumptions. Check this.
- Old pre-6.1 CFCs recovered: 476 team ranked Crystal Tower, 478 ranked, 479 training,
  480 custom, 580 team custom. Old/current Excel schemas differ: do not copy old
  row bytes directly into current sheets.
- Prior test used InitDirector 40001 flags0 after loading exploration map, tracked
  connection content_handler_id/current_instance_id, and terminated on zone change.
  It did NOT restore metadata, create a duty, or send native scores/team state.

Read-only investigation tools/results (ignored target directory):
- `target/director_live_probe.py <FFXIV PID>`: reads EventFramework factory map and
  ActiveContentDirector using ReadProcessMemory; no client state changes.
- `target/inspect_revive.py <preferred-base address hex> [bytecount]`: static disassembly.
- `target/director_factory_ids.py`, `director_virtuals.py`, `director_dispatch`/factory
  research outputs, `director-current-sheets.txt`, `director-legacy-overrides.txt`.
- `target/feast-director-research`: recovered InstanceContent/CFC EXH/EXD files.
- `target/recover_director_metadata.py`: selective historical recovery script.
- Python libraries capstone/pefile in `target/re-tools`; installed with escalation,
  so tools importing them may require escalation due to Windows ACLs.
- `target/client-structs` has newer (Sept 2026) symbolic data: do NOT use its absolute
  addresses on this June build without signature verification. Inspected RVAs above
  are for the actual installed build. Shared field offsets were checked where used.

## Sastasha and proper duty path

YES: repo has Sastasha content and Duty Finder entry, though some mechanics are TODOs.
Queue implementation is basic: immediately sends ContentFinderFound for the first
selected duty; it is not full matchmaking.

Relevant code:
- `resources/scripts/content/s1d1_re.lua`: Sastasha. onSetup near119,
  onActorDeath near231, complete_duty near269; randomized coral, doors, bosses,
  shortcuts, completion, unfinished mechanics.
- `servers/world/src/main.rs`: QueueDuties -> register_for_content (around2345).
- `servers/world/src/zone_connection/mod.rs`: register_for_content around344.
- `servers/world/src/zone_connection/zone.rs`: join_content stores return location.
- `servers/world/src/server/mod.rs`: ToServer::JoinContent around2104 creates a new
  instance; create_instance_for_content around139 loads content Lua, calls setup,
  assigns CFC and entrance. Director is only created if the content script loads.
- ZoneConnection::handle_zone_change sets duty conditions and initializes the client
  director during zone loading.
- `servers/world/src/server/director.rs`: server DirectorData, Lua events/variables.

The key is the initialization sequence, NOT clicking UI versus typing a command.
A command can call the normal queue/duty path. Merely renaming our Runtime as a
Director or sending InitDirector after teleport is insufficient.

Suggested next steps:
1. Have user restart client; verify ordinary login and Sastasha Duty Finder entry.
2. Inspect baseline duty state, director initialization/variables, exit and reconnect.
3. Prepare backed-up current-schema Feast CFC/InstanceContent/TerritoryType metadata
   restoration and a content script/entry path modeled on that working baseline.
4. Validate native director initialization before connecting match updates.
5. Check PvP hotbar/action restrictions before switching intended use: current Feast
   hotbars use native PvE action placeholders routed to server Feast rules.
6. Replace plugin UI workarounds only when native equivalents work in-game.

## Working Feast implementation to preserve

Map745 is the only Feast arena. Small teams/solo testing allowed.

Commands: `!feast`, `!feast leave`, `!feast status`, `!feast restart`,
`!feast dummies [1-3]`, `!feast attack on|off`; private `!feast hud` feeds HUD.

- Native WHM hotbar casts, castbars, animations and GCD/recasts work. Old combat
  plugin Kawari.Feast stays DISABLED. `!unlock all` + normal relog unlocks actions.
- Glare/Cure slidecast grace is **500ms**, effects resolve once at full cast deadline.
  Early movement is latched immediately; cast-origin range preserves late movement.
  Lua `resources/scripts/feast/WhiteMage.lua`, adapter `server/feast.rs`.
- Preparation **10s**, match6min, 2min sudden death, next pickup wins tie-break.
- Medals100/player, KO drops half rounded up, enemy pickup2y, role caps200tank /
  175melee /150ranged-caster /125healer. OWN unclaimed dropped medals count toward
  cap (official Feast rule). Example100held+25own-ground blocks further pickup.
  Prototype ground expiry60s returns to owner. Overflow stays on ground.
- Dummies copy appearance,5000HP100medals, stationary; optional1000damage every2.5s
  within3y, no chase. Corpses persist until reset. Dummy teamClaws vsuserFangs.
- Team spawns inside jails x=-58/+58,y.5,z=-3/-1/1/3. Gates USER-CONFIRMED working.
  CollisionSG7403947/7403946 plusVFX7515404/7515405 controlled via network EObjs
  and SetSharedGroupTimelineState object_type0,actorID (NOT StandObjecttype1).
  Slots1closed/3open; wait recipient has_spawned before sending.
- Medal native prefabs:2007627(1–25),2007628(26–50),2007629(51–75),2008631(76+).
  NormalFang/Claw timeline slots3/5; blocked slots9/11, perviewer eligibility.
  Pickup slot2 dropmedal_off_a includes native pickup sound;retain1s thenremove;
  partial pile respawns at new size. Pickup animation/sound user-confirmed before
  size/blocked variants; latest variants still need live confirmation.
- Separate old battle countdown overlay REMOVED, including hardcoded60s practice
  timer. HUD shows preparation. This was deployed along with respawn changes.
- Server respawn minimum **10s**, enforced through Instance.feast_respawns deadlines,
  kill_actor and fallback dead observation. ReturnAfterDeath rejects early requests.
  HUD plugin modifies native AgentRevive-owned SelectYesno YesButton to OK(Ns),
  disables until fresh server0 + local10s. Private KFEASTRESPAWN1|ms snapshots.
  Native button visual still NOT user-confirmed. Requires HUD reload/client restart.

Docs: `docs/src/development/feast-match.md`, `feast-white-mage.md`.
Last validation before probe deployment:42worldtests pass, cargo check passed,
HUD Release build clean. Disabling probe rebuilt Oodle binary successfully.

## HUD and client launch

- `tools/feast-hud` = **Kawari.FeastHud**, API15, display/native-confirmation UI only.
- `tools/feast-plugin` = old combat plugin, keep disabled.
- HUD: graphite industrial style, Bahnschrift typography,620x108; user rejected
  original red/blue banner. `/feasthud preview`, `/feasthud reset`.
- Registered DLL: `tools/feast-hud/bin/Release/net10.0-windows/Kawari.FeastHud.dll`.
- Astra Dalamud config:
  `C:/Users/oracle/AppData/Local/astra/dalamud/{1a7f787d-aaec-4bee-95c7-ad37c5863230}/dalamud-config.json`
  Preserve typed JSON metadata; installer backs up safely.
- Actual Astra FFXIV config is under AppData/Roaming/astra/user/same-GUID/FFXIV.cfg,
  not retail Documents config.

Earlier graphics bug remains unresolved/unconfirmed: resizing cropped whole3Dscene
and hid chattext even withHUD disabled. Read-only diagnostic found Device/SwapChain
1280x720 whilewindow1920x1080. Dalamud logged invalid resizeBuffers; DiscordHook64
and NVIDIA nvspcap64 present. User was asked to disable Discord overlay and close.
Do not claim this issue was fixed or caused by our HUD. `target/display_diagnostic.py`.

## Assets and history

Oldpatcharchive: `C:\dev\kawari\ffxiv-patches(1)\game`, cutoff2022.03.01 (before6.1).
164patches,1,130,570writes scan cache in target/feast-recovery-tools/recovered.
HUD recovery: `target/feast-hud-recovery` (290named files,41.6MB); missing modern
header/gauge resources recovered. Arena388files recovered. No wholesale oldclient
installation. Tools in target/feast-recovery-tools; Archive class recover_arena.py.
`tools/feast-map` manages original reversible territory restoration.

`HANDOFF_FEAST.md` contains the full chronological history. Its early entries include
superseded implementations; use this HANDOFF.md and latest code as current truth.

## Operations

Use hidden processes; user dislikes visible terminal panels and needless confirmations.
Do not restart while a player is online unless explicitly authorized to interrupt
(as in the last recovery). Normal logout saves progress. Check fresh DB state:

```powershell
& 'C:/Users/oracle/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/python.exe' -c "import sqlite3; print(sqlite3.connect('file:world.db?mode=ro',uri=True).execute('select count(*) from volatile where is_online != 0').fetchone()[0])"
```

Builds: `cargo test -p kawari-world --lib`, `cargo check -p kawari-world`,
`cargo build -p kawari-world --features kawari/oodle` (running EXE locks replacement).
HUD: `dotnet build tools/feast-hud/Kawari.FeastHud.csproj -c Release --no-restore`.
Python use `-X utf8` and explicit UTF8 file encodings on this Windows system.

Stop only Kawari executables under this workspace's target/debug, build, then:

```powershell
$env:RUST_LOG='info'
Start-Process -FilePath 'C:\dev\kawaridev\Kawari\target\debug\kawari-run.exe' -WorkingDirectory 'C:\dev\kawaridev\Kawari' -WindowStyle Hidden -RedirectStandardOutput 'C:\dev\kawaridev\Kawari\target\kawari-server.log' -RedirectStandardError 'C:\dev\kawaridev\Kawari\target\kawari-server-stderr.log'
```

Verify all8startup lines and stderr. Escalation generally needed for process control,
git writes, ProgramFiles/AppData and some tool dependency ACLs. Preserve failure logs
before restart. Never delete/commit oodle-network-shared.dll.
