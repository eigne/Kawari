# Feast restoration handoff — start here

## October 27 Feast PvP Actions window (latest)

User asked to restore the Feast-era PvP actions menu with drag-to-hotbar. Added
FeastActionMenu.cs (pure grouping/loadout/layout/hit-test rules, 142 new HUD
checks) and FeastActionsWindow.cs (ImGui window: Job Actions, Additional Actions,
Hotbar Layout tabs). `/feastactions` opens it; it also opens beside PvpProfile.
FeastHotbars now saves a per-job layout (Layouts) and loadout (Loadouts) in plugin
config, applies the layout on arena entry instead of the default kit, and
Place() backs up and sets a native slot. Action lists come from the existing
feast-actions.json catalog built from pre-6.1 guides (historical-job-sources.json);
not re-fetched this pass. No server changes. Release DLL built at the registered
path while FFXIV was closed; previous files backed up in
target/feast-actions-menu/before-install. Not tested in game: native slot
hit-testing, ImGui coordinates under multi-viewport, and loadout auto-send.
Commit d8da6c4c holds the Sept 26-27 work; it was not pushed (permission denied).

## September 27 all-job native gauge audit

NativeResourceGauges now feeds native data readers for all19 jobs/29 addon
layouts, including secondary gauges. Dynamically resolves virtual81 from each
named addon, verifies reader address and initialized payload size, preserves
native data vtable/BRD title pointer, and temporarily overrides array87[25]
only for the matching job refresh. Unknown layouts are left alone and logged.
NativeJobGaugeData contains pure tested mappings and hides unrelated modern
gauges. Summoner manager AetherFlags now converts stack count to bitmask.
BLM Jobs.lua resource byte now carries elemental time in100ms units (0..150),
separately from Polyglot timer; client F7 validator updated. Script read at new
rules-engine creation, so re-enter a fresh duty after plugin reload.
319 HUD checks,29 SDK bounds checks, release build and historical job engine
tests pass. See docs/src/development/feast-job-gauges.md for coverage and limits.
Installed target/feast-all-gauges/hud into registered Release directory, backup
in target/feast-all-gauges/before-install. Reload plugin or restart client to
activate; server left running, no server binary change.
No computer use/job switching: all19 visual appearances are NOT verified.

## September 27 WHM gauge follow-up: live identity mismatch

User reported gauge still absent after visibility fix below. Read-only
ReadProcessMemory probe of running client71816 (target/feast_lily_live.py)
confirmed JobHudWHM0 exists, is initialized, has enabled HealingGaugeData,
but VisibilityFlags remains4. Its vtable is a Dalamud lifecycle clone, not
the original image vtable2295E18: previous RefreshWhm identity guard skipped
the addon. Replaced table-address comparison with exact inline native addon
name JobHudWHM0 (including NUL), using NativeAddonIdentity. No memory writes
or game inputs.129 HUD checks pass, including cloned-vtable/name regressions;
Release build and memory-loaded atlas check pass. Installed candidate
target/feast-lily-identity/hud; backup in before-install sibling. Client was
running at installation, so reload Kawari.FeastHud or restart client before
visual verification. No server restart needed; live visual success not claimed.

## September 27 invisible WHM Healing Gauge

NativeResourceGauges previously only hooked the WHM data reader (RVA15BF5E0).
Pinned client disassembly shows base JobHud RequestedUpdate (RVA15A0210)
also sets VisibilityFlags bit4 when number array87 index25 is zero. Added
a hook scoped to Feast WHM and vtable2295E18 that temporarily enables that
input during the original refresh, restoring it in finally. Also requests
array87 subscriber refreshes while active and once on exit/unload; PvP does
not normally update the PvE lily array. Retains native gauge layout/config,
standard/simple rendering and F7 server-authoritative lily/blood values.
Release build clean;124 HUD checks and memory-loaded header check pass.
Installed target/feast-lily-visibility/hud into the registered Release directory;
previous module backed up in target/feast-lily-visibility/before-install.
Game was absent at installation. No computer use. Native visibility diagnosis
is from static code; in-game visual acceptance still needs user verification.

## September 27 Assize disconnect fix

The logged panic at packet/compression.rs:132 was a connection-task failure,
not an exit of kawari-world. Assize9620 sent AoeEffect8 with only632 payload
bytes while the opcode table requires636 (668 including both headers).
Restored six trailing reserved bytes after packed XYZ instead of two; header,
effect and target offsets are unchanged. The previous standalone layout test
incorrectly asserted632. Added a regression through the actual compress/send
serialization path: reproduced the same panic before the fix.
Original stdout/stderr preserved as target/kawari-server*.log.aoe-crash-*.
Both AoE serialization tests and all57 world tests pass. Rebuilt with
kawari/oodle and restarted supervisor PID93408; world startup confirmed at
2026-09-26T18:44:43Z on21063, fresh stderr empty. No computer use or game launch;
live Assize verification remains pending.

## September 27 all historical PvP jobs (latest)

User explicitly asked for ALL jobs from before April2022, not another small
batch. Implemented the remaining13 kits: MNK,NIN,BRD,MCH,DNC,SMN,RDM,SCH,AST,
DRK,GNB,RPR,SGE. All19 historical PvP jobs enabled. BLU has no Feast kit;
VPR/PCT excluded. No computer use or game launch; user prohibition persists.

All19 official pre-April capture URLs/hashes are recorded in
tools/feast-assets/historical-job-sources.json and linked from
docs/src/development/feast-all-jobs.md. Existing WHM/BLM/SAM/PLD mechanics in
earlier April sources match newly retrieved pre-April pages. GNB's latest
available capture in the searched range is December6,2021; RPR/SGE included
because patch6.0 introduced their PvP actions. Never use current CC job guides.

AdditionalJobs.lua implements214 new job actions plus7 ranged role/LB actions;
missing definitions fail initialization. AdditionalActionData.lua is generated
by tools/feast-assets/import-remaining-jobs.py using cached guides and6.08 CSV.
Shared-role job permissions, MP cost type51, shared/charged recasts, transformations,
combos, buffs, barriers, delayed effects, Dancer steps, RPR/SGE resources and
native control/stealth statuses wired. World loader installs both job modules.

Native hotbars/gauges support the entire roster. F6 now permits64 records (RPR
needs36); F7 job-specific fields include DNC sequence/RPR stacks/RDM two mana
values. Terminal Velocity4249 gets proper adrenaline treatment. Shukuchi8812
uses the ground bridge and authoritative charged recast update. Existing header,
party roster and native adrenaline code retained. No proprietary assets added.

57 world tests pass, including new feast_all_jobs.lua resource/action regressions;
124 HUD checks pass. Release HUD builds with no warnings/errors. Memory-loaded
DLL regression test passes and loads both native header atlases. Native asset
audit:342 distinct action IDs; every historical icon and actual action timeline
dependency exists. Empty Tsubame/Continuation wrappers resolve to real actions.
Compact audit: tools/feast-assets/all-jobs-7.51h2.json.

Installed HUD from target/feast-all-jobs-candidate/hud while FFXIV closed; previous
files backed up under before-install there. Installed DLL SHA256:
559CAD3DFDB2E2A8396C8914C1AEA8BC7E3167ED2D92BD2ECA6506DCD8146AC5.
Oodle world rebuilt; hidden supervisor restarted with RUST_LOG=info. Server logs archived
with before-all-jobs timestamp. No commits this pass; preserve existing changes.

Important: this is gameplay roster coverage, NOT a claim of perfect retail
fidelity/live verification. Fixed potency-to-HP engine, approximate cone/line
geometry/proximity curves, ten-second assist window, collision/LoS gaps and
incomplete native buff/gauge presentation remain documented. Native gauge fields
are populated but live appearance is unverified; removed Huton/SMN layouts not
restored. Default bot teams still use WHM/PLD/SAM/BLM. Tests cannot establish
live animation, native stealth/rendering, or targeting success.

## September 27 Warrior/Dragoon kits (latest)

User asked whether BLM/SAM/PLD were implemented and to add other jobs using
Wayback captures before April2022. Existing three have gameplay/native hotbars
and tests, but broader fidelity/live-validation limits remain. Added WAR21 and
DRG22 from verified CDX captures20220331171726 and20220323203055, respectively.
Full source links, features, limitations and repeatable metadata import are in
docs/src/development/feast-jobs.md. Cached guides under target/feast-more-jobs.
Both sets of historical action timelines survive in the current client.

Jobs.lua contains kits/resources/combos, shared-role permissions, cooldown
reductions, basic bot choices. WhiteMage.lua accepts shared job-role actions and
party LB mapping. Shared action catalog gains51 entries (33 unique new job
actions +18 role/shared records). Native server job gates/F6/F7 and C# hotbars/
gauges now support WAR/DRG; DRG timer validation permits30s, eyes capped2.
Native current DRG Blood stance visualization not restored; no claim it renders
like6.08. Existing status-icon and geometry/stat-formula limitations remain.
Default bot team composition still WHM/PLD/SAM/BLM.

57 world tests, including expanded WAR/DRG Lua regression checks, and77 HUD
checks pass. Release HUD built in target/feast-war-drg-candidate/hud. No computer
use performed; user constraint persists. Native visuals/actions need manual test.
Installed HUD DLL453213B6EA7D9228DE5F3751A30A3EE50512E0DBCAD4E5218CF6B68E7B174864
while FFXIV closed; Oodle world rebuilt/restarted hidden. Logs preserved with
before-war-drg timestamps. Cooldown reductions use the engine's elapsed/total
recast values so native charge counts are preserved. Targeted job test rerun
passed after that correction. No commits this pass.

## September 27 medal tint, Assize flytext, Dia status (latest)

User reports header functional, but rail colors wrong, Assize damage lacks
floating numbers, Dia appears to lack its PvP debuff. No computer use this pass.
NativeMatchHeader now uses additive blue/red rail tint on the recovered grey
fill rather than dark multiply-only tint. Numeric font lacks ':'; clock now
draws a separate separator in the regular native font.

Root cause of Assize: animation event described only selected target/self HP
delta, while damage on other actors updated HP without effect-result entries.
Lua completion now collects all direct damage/heal recipients. Feast adapter
sends a typed AoeEffect8 with per-target effects, historical animation ID and
native request acknowledgement; batches over eight retain all targets.
Replaced the unused, incorrect AoeEffect8 field layout with the shared native
action header / eight effect blocks / target IDs / position layout (636 bytes;
trailing padding corrected in September27 Assize disconnect fix above).
Single-target action presentation remains in place. See action_result.rs and
server/feast.rs. Native GPU/flytext appearance needs manual verification.

Cached April9 WHM PvP guide: Dia800, damage taken +10% for15s, NOT a DoT.
Server modifier already existed and passes expiry tests. Added dia_visual events
for application/expiry and native historical PvP status2035 (not PvE1871), with
caster ownership via StatusEffects.add_from. Archived Status-6.08.csv confirms
2035 description. Tests cover mixed Assize recipients and status add/remove.
Validation:57 world tests, targeted core packet layout test,71 HUD checks pass.
Release HUD installed from target/feast-assize-dia-fix/hud; DLL SHA256
CF7898BBDE1FA4F56BD20851001EC41FDBA06C9043CF8EAA555D8C71CB55CD44.
World rebuilt with Oodle and restarted hidden while client closed. Previous
logs preserved with before-assize-dia timestamps. No live visual test claimed.

## September 26 header layering and party action slots (latest)

User verified native header loads but banner paints over numbers. Changed
NativeMatchHeader.Attach to append nodes in Atk draw traversal order
(ChildNode -> PrevSiblingNode), so banner precedes gauges/text. Prior insertion
reversed paint order. Native rendering still awaits their next manual check.

Party slot0 was incorrectly a normal role action (Purify/Phantom Dart/Smite/
Low Blow). WhiteMage.party_actions now emits job adrenaline first (WHM3362,
BLM3361,SAM3360,PLD3359), then only two equipped additional actions. Loadout
enforcement already allowed two; this was presentation selection. NativeRoster
omits a seconds label on adrenaline; NativeRecast uses authoritative 0..100 fill
for its dimming/radial, leaving actual cooldown timers on slots1/2. Historical
Empyrean Rain icon784 added to PartyAction. Tests cover all four job rosters.

56 world tests passed, followed by updated cross-job test; 71 HUD checks pass.
HUD installed from target/feast-header-layer-fix/hud while FFXIV closed.
World rebuilt with Oodle and server restarted hidden with old logs preserved as
before-party-slots timestamps. No computer use. User requested a brief code map
in final response; keep future maintenance explanations concrete.

## September 26 header startup regression fix (latest)

User reported complete CC UI/hotbar/unbinding regression. Astra dalamud.log
20:48:05 confirms the whole plugin failed construction: ArgumentNullException
path1 in NativeMatchHeader. Dalamud loads the assembly from memory, leaving
Assembly.Location empty. Earlier native-header build was NOT functional.

Fixed: pass pi.AssemblyLocation.DirectoryName explicitly; isolate header asset
construction after director hooks are enabled and catch/log its failure. Missing
header assets now leave roster/hotbars/CC suppression enabled and hide the CC
header. Regression smoke test tools/feast-header-loader-tests loads the actual
DLL from bytes and decodes both atlases using the explicit installed directory.
Passed, along with 67 HUD checks and clean Release build.

Installed corrected DLL SHA256
67F06F8293B4B7E88961131EA43FBCDCF696EAD63AD7620560699DB8B3705C67
from target/feast-native-header-fixed/hud. Log shows game session ended20:52;
no corrected in-game session yet verified. No computer use or game restart.
Continue to honor the user's no-computer-use constraint. Do not claim visual
recovery until a subsequent session supplies evidence.

## September 26 native in-match medal/clock header (latest)

User explicitly requested replacing the TOP IN-MATCH medal/clock bar, not results.
Computer use remains prohibited this turn; do not launch the game for testing.
Implemented NativeMatchHeader using recovered header/gauge textures and ULD
geometry, attached to PvPMKSHeader's native root. Native Atk rendering replaces
the removed ImGui graphite bar; Dalamud still loads the adapter. Blue Claws left,
red Fangs right, local-team star, eight job icons/medals, totals, 600-medal gauges,
native-font clock and phase. Original CC children are hidden and restored on
cleanup; no CC requested update. HUD Layout position/scale uses the existing
Crystalline Conflict Progress Gauge entry. Stale frames clear values; final
clock freezes. Details and asset provenance in feast-native-assets.md.

Build packages only locally recovered textures from ignored target recovery.
Native kernel texture upload avoids assuming missing SqPack paths resolve.
No proprietary asset files committed. No server changes or restart needed.
Release candidate: target/feast-native-header-candidate/hud. 67 HUD checks pass.
Native rendering/lifecycle needs manual in-game verification; no live test claimed.
Installed to tools/feast-hud/bin/Release/net10.0-windows with both atlas files
while FFXIV was closed. DLL SHA256
902F5E0C358994921622D4816C7DAEAAFA9AB08AC4FD73C5D79A8FFA04566D12.
Previous DLL preserved at target/feast-native-header-candidate/before-install.

## September 26 WHM resources and native gauges (latest)

USER OVERRIDE: user is using the computer. Do NOT use computer use or launch
the game; they will manually verify this update. Older autonomous testing
instructions below do not override this latest constraint.

Rechecked cached official April 9, 2022 WHM PvP guide in
target/feast-whm/guide-recheck.html. Server effects already existed; missing
client resource transport and prediction gates made the kit misleading.
WHM now sends lilies, Blood Lily, and 10-second combat timer via director F7.
Preparation grants no lilies; full capacity cannot bank a fourth lily.
Native normal/simple Healing Gauge receives those values. Retained native
_LimitBreak widget receives personal adrenaline, not a custom ImGui bar.
NativeResourceGauges uses the existing exact executable hash guard, WHM reader
RVA15BF5E0 and verified LimitBreak number43/string75 arrays; restores shared
arrays on leave/unload. These native visuals require manual verification.

Hotbar prediction rejects empty lily/blood/adrenaline resources and inactive
matches. WHM uses historical Empyrean Rain3362 rather than PvE Pulse of Life208.
Server tests verify 50% maximum HP healing, cleanse, nearby allies only, no
resurrection, full-gauge consumption and repeated-use rejection. Solace/Rapture
spend lilies; three nourish Blood Lily for one Misery. Adrenaline passive fill
remains the existing provisional 180 seconds, not newly historically verified.

56 world tests and 59 HUD checks pass; release HUD builds without warnings.
Installed candidate DLL SHA256
39E98983AA80F990428F874FCB2D4434305ECC55988B518847A05E22200424FE
while FFXIV was closed. Oodle world rebuilt and server restarted hidden at
2026-09-26T11:29Z. Logs before restart saved with before-whm-gauges timestamps.
No computer use performed and no game launched. No commits this pass.

Manual checks: enter as WHM, inspect both native gauges; gain one lily per10s
combat, spend three through Solace/Rapture, then one Misery; Rain unavailable
until full, heals/cleanses and empties gauge; leave restores ordinary gauges.
Original Feast header/clock ULD and textures are recovered under
target/feast-hud-recovery, but the old controller is gone. Native restoration
needs an adapter; this update leaves the graphite ImGui scoreboard unchanged.

## September 26 autonomous live regression pass (latest)

User explicitly authorizes restarting the sole-player server, Astra login and
full autonomous game testing. Do not stop at builds/restarts or ask them to test.
Computer Use skill (@oai/sky via node_repl) works. Login credentials are not recorded in this file.

Fixed this pass: native historical action tooltips (including MP cost), hotbar
recast groups/charges and icon refresh, original dark party backgrounds, visible
original supply box/kit assets, and data/network lock-order inversions suspected
in the earlier server stall. Native duty timer now includes preparation and is
resynchronized at active/overtime transitions. Three-minute and one-minute
warnings were verified at the corresponding Feast clock times after restart.

Live verified: full six-minute bot match with two KO/respawns, further full solo
matches, repeated queue/entry/exit; original adrenaline barrels and Wolf's Heart
box; five Glare hits destroy a barrel and spawn the original floating kit; native
Glare GCD wedge and Protect darkening/radial/countdown; correct detailed Protect
and Cure II tooltips (Cure II 1.50/2.40 seconds, MP1000). No recurring server stall.
Wolf's Heart takes damage. Pickup rewards and its destruction were NOT completed
in this pass: the first kit expired while approaching, and the match ended during
the Heart test. Those remain live-test work, not claimed verified.

Server rebuilt with Oodle and restarted 2026-09-26T10:33Z. Latest logs are
target/kawari-server.log and target/kawari-server-stderr.log; previous run preserved
as target/kawari-server[-stderr]-before-native-clock.log. 55 world tests and 50 HUD
checks pass. No commits this pass. Release HUD is registered and automatic dev
reload enabled; native hook guards now tolerate Dalamud's retained dispatch stubs
while pinning the complete executable hash. Details in feast-native-assets.md.

Remaining observed defects: preparation/end-of-match client prediction can start
a cast before server rejection; removed action castbar icon is still placeholder.
Bot partial-navmesh stalls, stat normalization, native PvP action menu, and broad
BLM/SAM/PLD playtesting remain unfinished. HP5000/kit lifetime60 are provisional.
Do not claim full Feast recreation. Targeted pickup test should approach BEFORE
destroying the box. KP_5 rotates camera toward target but does not reliably update
server-facing rotation for !nudge; cast first or use normal movement. Sending chat
unfocuses input: click the chat box again before typing each subsequent command.

## September 26 native Feast roster - latest live tests

Recovered PvPColosseumPartyList4 geometry now drives the surviving native
PvPMKSPartyList5 controller. Four compact rows, historical gauge/icon placement,
native target clicks, ten status-image slots, native names/cast labels. CC large
panels/header/fifth row/LB strip removed. Graphite scoreboard and saved HUD
positions retained. HUD Layout entries still have CC names; this is an adapter,
not wholesale loading of the old ULD or a restored old executable controller.

Live verified: all eight rows/names, ally and enemy click targeting (enemy must
be in native target range), Cure II cast name/progress, status icons, cooldown
numbers/dimming. Radial wedge appearance needs a closer check. Native label
allocation must use UISpace.Create<AtkTextNode>() (zeroed before Ctor); the ULD
factory alone left LinkData uninitialized, causing blank names/destructor crash.
The zeroed-label build survived a clean shutdown. Status images also explicitly
initialize PartId=0/type/wrap/flags after an earlier LoadIconTexture crash.

Respawn testing found two server bugs: native auto-return arrives before the
10-second lock, so discarding it leaves the client black; it is now queued once
until the original deadline. Then Some(745) zoning selected a public instance
with content ID 0 and crashed director initialization. Respawn now reinitializes
the actor in the SAME duty, preserving content ID/director/other participants.
Live KO/respawn passed: full HP/MP, correct platform, all eight rows, ongoing
clock and medal state preserved. Same-duty reload also duplicated InitDirector;
it now reuses the existing Feast director. Final KO -> respawn -> leave passed:
Gridania has no PvP header/rosters/alerts and the normal hotbar returns.
Crash evidence:
AppData/Roaming/astra/log/dalamud_appcrash_20260926_062749_497_78064.log.

55 world tests / 47 HUD checks pass; HUD Release and Oodle world builds pass.
Server restarted 2026-09-25T21:45:09Z with fresh online=0 and client absent.
Final live test completed on this build; client exited normally from Gridania.
Earlier logs: target/kawari-server[-stderr]-before-director-reuse.log and
target/kawari-server[-stderr]-before-duty-respawn-fix.log.
No commits. Full notes: docs/src/development/feast-native-assets.md.

User authorizes autonomous Astra login (credentials supplied in chat, not recorded here), normal exits/restarts,
and explicitly exiting the crash handler. That earlier access block is cleared.
Computer Use uses node_repl @oai/sky. Individual press_key characters work in
game chat; type_text does not. Up recalls this process's sent commands.
KP_5 locks onto a selected actor; r toggles autorun. Observe between each action.
Bot navigation still stalls on partial navmesh paths; approaching them triggers
combat. Do not claim full bot-match playability from these HUD checks.

## September 26 BLM/SAM/PLD (latest)

Implemented the user's three archived pre-6.1 job guides in `feast/Jobs.lua`,
extending the WHM engine. Native hotbars use original IDs/icons, server-driven
combo/proc resolution and cast times (F6); existing native job gauges receive
server resources (F7). Role cooldown HUD supports all four roles. Bots now form
mixed WHM/PLD/SAM/BLM teams and use resource-aware basic rotations.
Full sources, choices and limitations: `docs/src/development/feast-jobs.md`.
Asset audit: 70 distinct historical actions, 139 nonempty end/hit timelines and
1301 dependencies present; Tsubame's empty wrapper resolves to valid Kaeshi.
Native ground cursor uses an explicit server-validated chat bridge for Soten/
Cometeor because generic location-action IPC is absent. Ground-impact visuals,
native gauges/icons/click targeting and overall new-job playability need a live
test. Do not claim exact historical stat scaling, cone angle or restored status
HUDs. Normal action routing and server validation remain authoritative.
No client game-file patches. HUD registered Release DLL rebuilt. Validation:
53 world tests, 43 HUD protocol/catalog checks, clean HUD and Oodle world builds.
Server update installed after a fresh DB online=0/client-absent check. Restart
FFXIV through Astra and test supported jobs with `!feast bots`. New commands:
`!feast actions`, `!feast job`, `!feast loadout <first> <second>`.

## September 26 solo bots (latest)

User requested basic bots to test matches. Added `!feast bots` / `on` for solo
4v4 (three allies, four enemies), `bots off` removal/reset, attack on/off pause,
and restart preserving bot mode. Native NPCs reuse server WHM Glare/Cure II,
team rules, medals, supply damage/rewards and ten-second respawn. All bots WHM;
advanced tactics and other job rotations intentionally absent. Navmesh movement
has a direct-pursuit fallback without navigation data. Details in
`docs/src/development/feast-bots.md`. 52 world tests pass; Oodle build passes.
Installed with client absent and fresh DB online=0. Native playtest pending.
Previous broad restoration work below remains incomplete.

Updated 2026-09-21 (Asia/Tokyo). Workspace: `C:\dev\kawaridev\Kawari`.

## September 21 rules audit and candidate (latest)

User requested previous changes committed, then independent research and implementation
of missing Feast rules, prioritizing surviving native logic/assets. Previous accepted
animation/eligibility fixes committed as `17c18861`. New mechanics: `235e6a76`.
Broader task is NOT complete.

Current candidate adds 60s preparation, 600-medal early victory, Culling status994 and
damage/stacks/KO/overtime rules, persistent disconnect drops, timed supply waves,
original layout-bound supply NPCs and native kit EObjects, team aggregate damage
ownership, proximity pickup, adrenaline/Wolf's Heart rewards and native gauge input.
Japanese official guide resolved misleading English wording: KIT OWNERSHIP IS BY
TEAM DAMAGE, not the individual highest contributor. Reward requires ground pickup.

See `docs/src/development/feast-rules-audit.md` for sources, evidence and remaining
work. Box HP5000, kit lifetime60, equal-damage Fangs tie-break and passive180s gauge
are explicit provisional choices; do not claim exact historical fidelity. Original
box/kit resources and status rows verified by new `--supplies` asset audit mode.
Do not claim native targeting, box rendering, pickup effects or speed are verified.

Validation:51 world tests,28 HUD protocol checks; HUD and audit build clean; Oodle
world binary built. Server started 2026-09-21T05:55:02Z after client absent and fresh
DB online0. Runner69908/world23584; all8services started, stderr empty.
User was asked asynchronously to enter Feast and test boxes at4:30, destruction,
kit pickup and adrenaline. Await that live acceptance; no restart while online.

Outstanding broad work includes native supply validation, historical HP/expiry,
Culling weather, native party statuses, job kits beyond WHM, LoS/stat scaling,
full matchmaking/results/rewards and ranked integration. The audit records these
explicitly. User wants independent progress, not a claim that all work is finished.

Additional native research: recovered 6.08 executable read-only from local
D2022.03.01 patch into target/feast-director-research/ffxiv_dx11-6.08.exe.
SHA2564b0a86b3cb5354619a97bc70918492169cf457eaa2bd825cb9e95e17c6c57883.
Never executed/installed/committed. Original PvPColosseum UI references survive.
June row renderer confirms cast percentage n+13 and spell string s+6, plus gauge
n+12 is0..100. Native cast bars now wired and DLL rebuilt at registered path while
client absent. Still awaiting live confirmation. Arena LVB weather91/63 found but
Culling variant unverified, so no weather override. Temporary Rust probe removed.

## New continuation state (supersedes deployed-state notes below)

LATEST PvP eligibility regression fix: User's next test rejected every skill as
unable to use in PvP. Verified June native UseAction RVA8EB0D0: calls spell
resolver8DDBB0, adjusts result8EAA20, passes resolved ID to GetActionStatus8E7CC0
at8EB310. Our override recognized only placeholder button IDs. NativeActions
now includes Actions.Select(FeastSpells.Resolve), allowing all20 historical IDs
through the same WHM/745 scoped override. Casting/recast checks remain intact
using the ID actually presented by native code. DLL rebuilt at registered path;
24existing protocol tests pass. Client restart required, no server restart.
Earlier claim that resolver changes only presentation was too narrow: native
UseAction also uses its result during action adjustment/eligibility. Server
already accepts historical IDs as well as transitional ones. Live usability,
timing and animation/slidecast regressions still require retest.

LATEST CLIENT FIX 2026-09-16: User reports Benison caster animation + Protect
target effect and castbar freezing visually during slidecast after server-only
historical spell_id change. FeastHotbars now hooks native static
ActionManager.GetSpellIdForAction, resolving transitional buttons to identical
historical Feast spell IDs BEFORE local prediction. Scoped to WHMterritory745,
Action type and implemented buttons; ordinary requests/recasts keep button IDs.
New FeastSpells.cs includes all20 actions and upgraded aliases. This removes
local/server spell identity mismatch; no movement/cancellation suppression or
fake castbar is introduced. Release DLL rebuilt at registered path;24existing
protocolchecks pass. Client restart required; no server changes/restart. Both
caster animation and slidecast visual completion need live retest. If freeze
persists, trace native cast cancel/update; do not assume identity fix proves it.

ORIGINAL ANIMATIONS latest: User requests original Feast animations for usable
spells. server/feast.rs now separates button_id from historical animation_id;
ActorCast and ActionResult retain historical spell_id while actual request
action_id/type/sequence are acknowledged. Prior code overwrote spell_id too.
All20 AnimationStart/VFX/AnimationEnd/ActionTimelineHit values compared to cached
6.08 CSV and match exactly. Added all20 acknowledgement regression;45tests pass.
Button metadata/icons/tooltips unchanged. Live original-effect playback pending.
Client/Astra absent and freshDBonline0 confirmed before deployment; original
animation production build passed; runner34724 restarted successfully. Previous logs saved as
target/pre-feast-original-animations-server.log and -stderr.log.

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

## September 26 native asset investigation

- Full findings: `docs/src/development/feast-native-assets.md`.
- Identified original four-player `PvPColosseumPartyList4.uld` from old EXE strings
  and filename CRC ea17456b. Recovered file remains under target/feast-hud-recovery/
  unidentified. Four rows, three action icons and ten status icons per row; all12
  referenced textures survive. NOT redirected into current renderer: its node
  bindings/click callbacks need deliberate adaptation first.
- Also identified old PVPScreenInformation, PvPRankRatingFunction and
  PvPRankPromotionQualifier ULDs. Reproducible resolver in tools/feast-assets.
- Native job/additional/traits/quick-chat menu layouts survive. Current
  PvPActionSort lists modern skills; AgentPvpProfile exposes JobActionIds and count.
  Menu remains unmodified; list population, descriptions and native dragging need
  tracing before a safe Feast adapter. Resource presence is not working behavior.
- Audit:174candidate resources,119texture references; only header/gauge textures
  missing from installed client (already recovered locally). All88unique historical
  action icons present at normal and HR sizes. Evidence:tools/feast-assets/ui-7.51h2.json.
- Applied original WHM hotbar icons from local6.08ActionCSV via
  resources/feast-whm-icons.json and FeastSpells.Icon. Restores Protect, Attunement,
  Rescue etc. No game data patch/server restart. Existing slot ownership/restore
  retained. Release DLL rebuilt at registered path with FFXIV/Astra absent.
- HUD47checks pass; audit+HUD builds0warnings/errors; diff check clean.
  WHM icon visual acceptance remains unconfirmed. User's graphite header retained.

## Autonomous live testing authorized and verified (September 26)

User explicitly authorized launching Astra, signing into the local Kawari account,
selecting Clerie Stance, and entering test commands, and prefers agent-run live
checks over repeated manual-test requests. Login credentials were supplied in the
conversation; do not copy them into committed files. Use this authorization for
routine test launches and normal exits/relaunches of this test character. Still
check for unrelated players before server restarts.

Verified through Computer Use skill (`node_repl` + `@oai/sky`):
- Astra app id/path: C:/dev/kawari/astra-0.8.2-win-x64/bin/Astra.exe.
- Launch with sky.launch_app, then select its fresh returned window.
- Account clerie already selected. Enter supplied password, click Log In.
- Astra closes after launching game. Refresh window inventory and bind the new
  ffxiv_dx11 window. Start -> Clerie Stance -> OK -> New Gridania.
- Optional Playguide prompt appears on login; dismiss No.
- Kawari Feast HUD v0.1.0 loaded was visible in chat.
- For commands, click actual chat edit field, observe caret, type, verify complete
  text, then Return. First typing attempt occasionally produces no text; refocus
  and verify before retrying, never blindly submit/repeat commands.
- A capture can lag transitions; observe again before assuming input failed.
- Activate game before capture: initial occluded capture included foreground app.
  Do not change window size (known separate graphics/cropping issue).
- Verified !feast -> native Commence -> arena and !feast leave -> New Gridania.
  Graphite header/restored WHM hotbar icons visible; departure cleared Feast HUD
  and restored city hotbar. This is an entry/exit smoke check, not combat coverage.
- Normal exit via game title-bar X -> in-game OK works; no forced process kill.
- Final state of this run: Clerie Stance logged into New Gridania, game open.
