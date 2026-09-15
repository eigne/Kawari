# Kawari Feast restoration — handoff

Saved September 15, 2026. Read this before continuing in a new Codex chat.

## Latest update — Feast buttons and animations (September 15, 14:12 JST)

Follow-up: user confirmed the plugin broadly works, but casted spells showed only
the release animation. A plugin-only fix now hooks `ActionManager.OpenCastBar`
for accepted local Feast casts and plays `Action.AnimationStart.Value.Name` via
`Timeline.PlayActionTimeline`. Cast cleanup runs each frame for movement/native
cancellation/deadline and clears only slots still holding the casting timeline,
preserving a release animation that has replaced it. Uses APIs in the installed
15.0.2.3 DLLs (which do NOT expose the newer `ActionManager.OnCastCancelled`).
The plugin rebuilt successfully with no warnings. User must reload Kawari Feast
in Dalamud to test this change; no server restart is needed. Test stationary cast,
movement midway, movement after completion, and Cure as well as Glare. This
specific casting-pose fix is not yet live-confirmed.

Next report: restarting with that fix still showed no casting pose. Logs confirmed
plugin loaded, but contained no cast-pose trace. Replaced the OpenCastBar hook with
the installed API's `PacketDispatcher.HandleActorCastPacket`, resolving the same
cast timeline and using `TimelineSequencer.PlayTimeline`. Removed cleanup based
on local native CastInfo flags because command casts do not initialize native
UseAction state; movement/deadline/teardown still clear only the owned pose.
Added information-level load/cast/timeline-slot diagnostics. This revision builds
cleanly. Asked user to reload plugin and cast once so logs can confirm the path.
Live log confirmed v2 loaded at 14:51:02 JST; Glare packets at 14:51:11 and
14:51:15 resolved ActionCastTimeline row 33 to animation 108. After playback,
native base timeline was 108, upper-body timeline 0. Awaiting the user's visual
confirmation; this proves hook/playback request, not sustained visible rendering.

User then confirmed the twirl/pose appears, but casting spell effects are missing;
the release remains correct. Added CastEffects.cs (v3) to start the separate
Action.AnimationStart.VFX and Action.VFX.VFX actor-attached assets. Current game
data resolves Cure/Cure II/Medica to cmcn_castx0c + cmrv_wndct0c, Glare to
cmwh_castx0c + cmat_ligct0c. All four AVFX files were read successfully. Native
actor-VFX create/remove API signatures from VFXEditor each match once in this
client executable. Owned handles are tracked; a removal hook discards handles
already freed by the game. Movement/deadline/teardown removes preparation effects
only. Built without warnings; user was asked to reload and test Glare/Cure again.
Look for "Feast casting presentation v3" and "Feast casting VFX ... created=true"
in Astra's dalamud.log. Visual validation is pending.
Live log: v3 loaded at 14:56:12 JST. Both Glare effects and both Cure effects
reported created=true during tests between 14:56:19 and 14:56:43. Awaiting the
user's confirmation of appearance and interruption behavior.

Next report: intro and release work, but the sustained/levitating cast pose is
missing. Verified game ActionTimeline rows: Glare 108 battle/magic_wht_start,
109 battle/magic_wht_loop; Cure family 98 battle/magic_cnj_start, 99
battle/magic_cnj_loop. V4 replaces PlayTimeline(intro) with
Timeline.PlayActionTimeline(intro, loop), dynamically finding the matching _loop
key. Cleanup recognizes both owned timelines and preserves unrelated release
timelines. Per-frame transition diagnostics log base/upper slot changes while
casting. V4 builds cleanly; user asked to reload and test Glare again. Awaiting
live logs and visual confirmation.

V4 live test failed: start 108 returned to normal/idle 3 at ~0.63s, never loop
109. V5 now observes that return to idle, explicitly plays loop109 (or99) and
holds it with Timeline.BaseOverride. Cleanup restores the previous override only
if it still owns it. ActionEffectHandler.Receive hook clears preparation before
the matching release packet is processed, avoiding any override of the release.
Death/movement/deadline/teardown also clear the hold. V5 built cleanly and user was
asked to reload/cast Glare; awaiting diagnostics and visual confirmation.

V5 live logs show Glare 108->109 and Cure 98->99; user confirmed sustained pose
works. User reported a brief idle gap before release. V6 keeps the pose/hold/VFX
past the local cast deadline until ActionEffectHandler.Receive clears it directly
before release. Movement cancels only before the cast deadline; a +2s grace
timeout prevents a stranded pose on a missing completion. V6 builds cleanly;
user asked to reload and check the gap. These V5/V6 changes are plugin-only; the
recovered background server remains running.

User confirmed V6 character animation is correct, but casting VFX end abruptly.
V7 completes casting effects using VfxObject.SomeFlags bit0x40 / FadeOutFrames=15
(0.25 seconds), allowing overlap with the native release. Interrupted casts still
remove active VFX immediately. Separate fading-handle tracking survives a new
cast; removal hook untracks native deletions, a 750ms fallback cleans leftovers,
and unload clears both active/fading sets. Installed API exposes the verified
fields. V7 built without warnings, user asked to reload and check the blend.
V7 live log confirms loading at 15:15:13 JST and completion fades for Cure120,
Glare16533, and Medica124 between 15:15:16 and 15:15:41. User's visual feedback
on the 0.25s blend remains pending.

User confirmed V7 casting transition looks good. Next request is Temperance's
persisting wings; user estimated 20s and asked to check the archive. Cached
guide.html PvP Temperance row (line7280) says15s; PvE row says20s. Communicated
that difference and implemented15s. V8 listens for accepted local ActionResult
16536 and resolves current-client Status1872->StatusLoopVFX107->cnj17wing_c0h;
the same wing asset is also referenced by historical PvP status2037. Asset read
verified. Persistent effect handles are separate from active casting/fading sets,
so movement/other casts don't remove them. Expiry15s fades, death/exit/restore/
unload removes; reapplication replaces. No server/buff rules changed. V8 built
cleanly, user asked to reload/activate Temperance and confirm15s persistence.
V8 loaded at15:19:16 JST; live log shows cnj17wing_c0h created=true at15:19:20
and expiry/fade at15:19:34.994 (about15s). Visual confirmation still pending.

User confirmed wings appear, but their expiry was abrupt. V9 replaces the
persistent wing's short native fade/fallback with a separate 1-second smoothstep
color+alpha ramp after15s. RGB is faded too for additive glow. Handles are kept
until opacity reaches0, then removed; native deletion untracks handles. Death,
exit,reapplication,unload clear active and fading wings immediately. Casting
particle fades are unchanged. V9 builds cleanly; user asked to reload/test.

This section supersedes the older prototype-only state below.

The user confirmed Glare damage and Cure II healing work in game. They requested
automatic hotbar spells and animations, and authorized using a client plugin.
They use plain Astra but already had matching Dalamud 15.0.2.3 installed under
`%APPDATA%/XIVLauncher/addon/Hooks/15.0.2.3`; its version.json explicitly supports
game `2026.06.18.0000.0000`.

Implemented `tools/feast-plugin`: Dalamud API 15 plugin, built to
`tools/feast-plugin/bin/Release/net10.0-windows/Kawari.Feast.dll`. On territory
745 / job 24 it fills hotbars 1–2, saves original slots for recovery, enables WHM,
and intercepts its own slot clicks/keybinds to send `!whm` internally. No manual
per-cast typing. It restores on leaving/job change/unload; `/kfeast restore`
restores and pauses, `/kfeast on` resumes, `/kfeast status` prints the mapping.
Native ActionRequest handling remains blocked when opted in; this plugin uses
the command transport. Cooldown UI/gauges/cross hotbars are not implemented.

The Lua rules now emit cast, animation, and cancellation events. The server
sends ActorCast/ActionResult with current-client animation IDs and primary-target
damage/heal presentation, without applying HP changes twice. All prior 25 world
library tests passed; three focused Feast tests passed after adding presentation
coverage (26 total library tests now). Plugin compiled with zero warnings/errors.
**Plugin loading, hotbar swapping/restoration and animations await live testing.**

Ran `tools/feast-plugin/Install-Astra.ps1`: copied runtime/assets into
`%APPDATA%/astra/dalamud`, set profile `{55f83ad4-b533-4684-a456-77398c222c24}` to
DalamudEnabled=true / DalamudChannel=local, registered the DLL for account
`{1a7f787d-aaec-4bee-95c7-ad37c5863230}` in
`%LOCALAPPDATA%/astra/dalamud/{1a7f787d-aaec-4bee-95c7-ad37c5863230}/dalamud-config.json`.
Original state backup is `%LOCALAPPDATA%/State/astrastaterc.before-feast-20260915-140724`.
Astra was closed during setup. Local channel pins the matching build.

User said "logged out"; read-only DB check confirmed zero online. Restarted the
verified repo server with final changes; all eight services started, world at
`2026-09-15T05:10:48Z`. Managed exec session `50165`; recheck process IDs as usual.
Live log remains `target/kawari-server.log`. The game process was still open at
last check, Astra had not relaunched. User was told to fully exit the game, reopen
Astra, launch normally, and test arena entry / Glare / Cure II / leaving restores
hotbars. Do not assume injection has been tested; inspect Astra/Dalamud logs.

## Goal and current situation

The user wants to recreate **the old Feast 4v4 PvP mode**, starting with Crystal
Tower Training Grounds, then its historical White Mage kit. Do not substitute
current PvE White Mage or Crystalline Conflict PvP actions.

The map is restored and the user confirmed that entering it works. Basic combat,
death-return handling, and a command-driven Feast WHM prototype were implemented.
The latest feature is tested in code but has not yet been confirmed working by the
user in game. The immediate interruption is a Codex UI problem: after opening a
terminal panel, the user cannot see this chat's history. They requested this handoff.

Do not automatically open another Codex terminal panel. Reading the server's log
file and controlling processes through shell tools is sufficient. The association
between opening the panel and the UI failure is reported by the user; the exact
root cause is unconfirmed.

## Workspace and user preferences

* Repository: `C:\dev\kawaridev\Kawari`, branch `master`.
* User wants work committed in logical commits and wants the assistant to manage
  server starts/rebuilds/restarts and read logs itself.
* Avoid unnecessary permission questions. Do not restart an online character's
  server without arranging a normal logout, because full autosave does not exist.
* No subagents were requested or used.
* No AGENTS.md was found in earlier inspection; recheck if the environment changes.
* Source documentation is under `docs/src/development/`.
* `oodle-network-shared.dll` is an intentional untracked runtime dependency; do not
  delete it or accidentally commit it.

## Commits already made

| Commit | Work |
| --- | --- |
| `1160816f` | Architecture, gameplay, and Feast implementation-plan documentation. |
| `c6bd1031` | Territory 745 restoration tools and `!feast` exploration command. |
| `41b1c04a` | Windows enemy auto-attack script alias fix and non-panicking action script errors. |
| `3a740ecf` | Handle return-to-homepoint requests after death. |
| `b521ab65` | Override death return inside territory 745 to respawn inside the arena. |
| `f4673a58` | Isolated, command-driven Feast-era White Mage ruleset, adapter, tests, docs. |

HEAD was `f4673a58` when this handoff was written. Before this document was added,
only the Oodle DLL was untracked and there were no uncommitted code changes.

## Running server and logs

Launch from the repository root (working directory matters for resources/SQLite):

```powershell
$env:RUST_LOG = 'info'
cargo run --bin kawari-run 2>&1 | Tee-Object -FilePath target/kawari-server.log
```

`kawari-run` builds with the Oodle feature and starts eight services. The separate
prebuilt installation at `C:\dev\kawari\Kawari-Windows` does not contain our edits.

At handoff:

* Latest managed exec session ID: `92852` (may not be accessible from a new chat).
* Most recently observed `kawari-run` PID: `74900`, executable
  `C:\dev\kawaridev\Kawari\target\debug\kawari-run.exe`.
* **Recheck process IDs before using them.** Never kill a stale/reused PID.
* Server was rebuilt and all eight services started at approximately
  `2026-09-15T04:44:22Z`; world started at `04:44:28Z`.
* A client attempted login at `04:46:24Z`; do not assume nobody is online now.
* Live log: `C:\dev\kawaridev\Kawari\target\kawari-server.log`.
* Previous startup log: `target/kawari-server-before-feast-whm.log`.
* Services listen on ports 21057–21065; world uses 21063 and 21064.

Managed PTY Ctrl+C did not reliably terminate all child processes. To restart,
first confirm online character count and arrange logout when needed. Then verify
the exact repo's `kawari-run` process and terminate that process tree. Previously
`taskkill /PID <verified-pid> /T /F` required a sandbox escalation and succeeded.
No UAC elevation was needed for ordinary server operation. Do not kill all Rust,
PowerShell, or FFXIV processes indiscriminately.

Read-only online-count check: connect to `file:world.db?mode=ro` with Python
SQLite and query `select count(*) from volatile where is_online != 0`.

Bundled Python:
`C:\Users\oracle\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe`.
Cargo is available on PATH; user installation is under `C:\Users\oracle\.cargo\bin`.

## Map recovery and native installation

Client/game directory:
`C:\Program Files (x86)\SquareEnix\FINAL FANTASY XIV - A Realm Reborn\game`.
Supported build: **2026.06.18.0000.0000 (7.51h2)**.

Historical patches are in `C:\dev\kawari\ffxiv-patches(1)`. The user removed a ZIP
to free space. Existing tools were also found under `C:\dev\kawari` (Astra,
FFXIV Explorer, Godbert, SaintCoinach, Oodle, prebuilt Kawari).

The arena is internal map **l1p2**, native LVB:
`bg/ffxiv/pvp_xx/pvp/l1p2/level/l1p2.lvb`.
Historical territories: 745, 765, 766, 767; map row 440; place name 2383;
content-finder rows 476, 478, 479, 480.

Selective recovery scanned 164 historical patches through March 2022, avoiding a
full installation rebuild. Recovered 388 named assets (~106 MiB), six unresolved
collision assets by hash, manifests, and source tools. The first original map LVB
was in `game/4e9a232b/D2018.01.12.0000.0001.patch`.

Recovery output: `C:\dev\kawari\ffxiv-patches(1)\feast-recovery`.
Read its README and manifests if continuing asset restoration. 117 native asset
parse checks passed. The current client already retained 383/388 recovered paths
and all 59 shared dependencies. The real blocker was blank territory metadata.

**An actual reversible patch is installed in the user's game data.** It populates
only territory 745 as an ordinary, non-PvP exploration area, with no duty binding.
Only one 39,936-byte allocation in `sqpack/ffxiv/0a0000.win32.dat0` at offset
311689344 was changed. Other 1,289 territory rows were verified unchanged.
The original allocation is backed up. Do not reapply blindly after a client update.

* Tool/docs: `tools/feast-map/README.md`, `feast_map.py`, `Install-FeastMap.ps1`.
* Backup: `backups/feast-map-7.51h2/patch.json`, `original-slot.bin`, `patched-slot.bin`.
* Seven installer tests passed; native Physis reads of installed metadata passed.
* Program Files patch installation required a UAC-elevated PowerShell helper.
* Historical recovered geometry was not copied over the current client; surviving
  client assets are used by the prototype.

Commands:

```text
!feast
!feast leave
```

Entry is `(-37, 1, 0)`, rotation `1.570451`, above a PC arrival found in the map's
planmap.lgb. `leave` returns to Limsa (128). The debug Lua command requires DEBUG GM rank.

## Saving, XP, and basic combat findings

`world.db` stores characters in the server working directory. Creation persists
the character. Full mutable state is committed during normal logout or a handled
network disconnect (`ZoneConnection::begin_log_out`). Zone transitions commit
volatile/location data; class changes commit class/inventory. No periodic full
character autosave was found. A crash can lose tutorial/quest progress while the
created character survives. Full saves write several tables, not one explicit
transaction wrapping the entire character state.

The user previously crashed by granting excessive XP. The XP loop repeatedly uses
the starting level's XP requirement and has unchecked arithmetic. **That XP bug
was identified but not fixed.** Prefer `//gm lv <level>` for setup.

To become WHM: `!classjob 24`, equip Soul of the White Mage from the Armoury Chest,
then `//gm lv 100` if the unlock reset the shared class/job level.

`!spawnmonster 1` spawns a level-1 enemy with 1,500 HP at the player's position.
`!ai_disable` disables NPC AI in the instance. Invalid IDs still use an unwrap in
the spawn command; do not guess random IDs without validation.

The no-damage/spawn failure was proven by a user log: enemy auto-attack 870 loaded
`EnemyAutoAttack_00870.lua`, a Git symlink represented as plain text on Windows.
Lua syntax error killed the detached world tick task while zoning/login continued.
Fix: explicitly register action 870 to the real 872 script in `Init.lua`; action
script IO/syntax/runtime errors now log instead of panicking. Tests cover recovery
after malformed/missing scripts and Stone/enemy damage.

PvE Stone exists; Glare/Dia/upgraded Stone scripts generally do not. Native healing
in the old generic action executor lacks authoritative Heal mutation. The new
Feast adapter applies its own authoritative healing; do not claim general PvE
healing or all retail job actions are now complete.

## Death and respawn

The death Return button sends client trigger category **200**, kind **8**.
This had no handler and caused an infinite loading screen. It now maps to
`ToServer::ReturnAfterDeath`, validates the live actor is dead, and performs a full
resurrection zone initialization. Kind 5 (accepting a raise) remains unsupported.

Outside the arena it uses the saved homepoint. Inside territory **745** it now
respawns at the arena entry with full HP/MP. This was a user correction: they do
not want death to send them to Gridania. The confirmation can still name the
saved homepoint; client UI text is not replaced. Same-zone death return forces a
full respawn rather than moving a dead actor. Saved homepoint remains unchanged.

## Feast White Mage — latest feature

User's source:
https://web.archive.org/web/20220409040536/https://na.finalfantasyxiv.com/jobguide/whitemage/#pvp

Direct download of the archive worked with the `20220409040536id_` Wayback URL
using an escalated network shell. Web tool opening alone failed. Local references:
`target/feast-whm/guide.html`, `Action-6.08.csv`, `Status-6.08.csv`.
Historical CSV commit: `bc1a1de27e7bf00adba0a0cc362402459c55b7c0` in xivapi/ffxiv-datamining.

Read **`docs/src/development/feast-white-mage.md`** for the full action table,
controls, and important prototype limitations.

Code:

* `resources/scripts/feast/WhiteMage.lua`: all 20 definitions and timed rules.
* `servers/world/src/server/feast.rs`: isolated Lua runtime per instance, actor
  snapshots, command dispatch, HP/MP effects, damage modifiers, movement correction.
* `resources/tests/feast_white_mage.lua`: behavioral scenarios covering all actions.
* Integration hooks: server `action.rs`, `chat.rs`, `instance.rs`, `mod.rs`, `npc_behavior.rs`.

Start with WHM equipped:

```text
!feast
!whm on
!whm list
!whm glare
!whm cure2 self
!whm status
```

Actions: `cure`, `cure2`, `medica`, `glare`, `dia`, `solace`, `rapture`, `misery`,
`tetra`, `assize`, `temperance`, `purify`, `bolt`, `kit`, `repose`, `protect`,
`rescue`, `largesse`, `attunement`, `rain`.

Target selected actor for single-target actions. `self` overrides it. Heals with no
selection default to self. Area actions choose recipients. For PvP testing each
WHM opts in and picks `!whm team fang` or `!whm team claw`. Same-team participants
act as a test party. Two additional abilities are selected with
`!whm loadout repose rescue`; default is Protect/Largesse. `!whm adrenaline` fills
the test gauge for `!whm rain`. `!whm off` returns to ordinary action handling.

Implemented: fixed potency damage/heals, target/range/life checks, MP spending,
cast timers/interruption, shared spell GCD, sequential charge recharge, lilies
every ten combat seconds, Blood Lily consumption, regen, team AoE effects,
mitigation/outgoing bonuses, moving Temperance aura, cleanse, sleep/wake on damage,
Rescue, immobilization, and adrenaline consumption. Generic native hotbar actions
are rejected while opted in to avoid mixing modern skills with Feast mechanics.
Generic 10%-per-tick HP/MP regeneration is disabled for opted-in players.

**Not a fully restored retail job:** no historical hotbar, icons, animations,
cooldown UI, gauge UI, match-level adrenaline generation, normalized PvP stats,
historical passive regeneration, full line-of-sight/animation-lock rules, or CC
resistance system. Most effects use internal state and text feedback. Bolt uses
current Sprint status for movement. Medical Kit/Attunement use a documented linear
missing-HP interpolation because the guide specifies endpoints but not a curve.
Regen snapshots modifiers and ticks every three seconds; these timing/stacking
assumptions need historical verification. `!whm` is a development prototype, not
a rated-match security/validation layer.

All 25 world library tests passed before the final dead-target refinement. The
two Feast tests were rerun successfully after that refinement. Full build and all
service startup messages succeeded. **No user-confirmed in-game test of the new
Feast WHM kit yet.** Do not present successful code tests as live client validation.

## Suggested next steps

1. Resolve or work around the Codex history display issue without opening another
   terminal panel. The new agent can manage the existing server via logs/processes.
2. Check current server/online state and read fresh logs; do not start a duplicate.
3. Help the user test `!whm on`, single-target damage/healing, lilies, party effects,
   and arena respawn; fix observed runtime issues.
4. Confirm which next layer the user wants: historical client hotbar/gauge
   restoration, more faithful combat mechanics, or full Feast match/team rules.
   Do not assume the command prototype completed the whole Feast project.

## Original Codex task and UI recovery evidence

Task title: **Import Crystal Towers map**.
Task ID: `01a09fba-2771-7be1-ab73-4bcf13c8bfea`.
The app task listing still returns this task. Local transcript files exist:

* `C:\Users\oracle\.codex\sessions\2026\09\14\rollout-2026-09-14T20-42-52-01a09fba-2771-7be1-ab73-4bcf13c8bfea.jsonl`
* `C:\Users\oracle\.codex\sessions\2026\09\15\rollout-2026-09-15T12-51-21-01a09fba-2771-7be1-ab73-4bcf13c8bfea_01a0a330-d607-79d2-a5f0-dee481c7c755.jsonl`

Their existence confirms records remain, not that every UI message can be rendered.
Do not delete/reset the Codex profile as a first troubleshooting step.
Official Windows shortcuts: Ctrl+J toggles the bottom panel, Ctrl+backtick toggles
the terminal. Hiding the panel, reselecting the task, and restarting the app after
active work finishes are reasonable recovery steps. No confirmed fix for this
specific missing-history issue was found.

Official references consulted:
https://learn.chatgpt.com/docs/reference/commands
https://learn.chatgpt.com/docs/reference/troubleshooting

Before restarting Codex, log out of FFXIV normally if connected: managed server
process survival across app shutdown has not been verified.

## Native Feast migration, 2026-09-15 (current work)
User requested server-native actions instead of expanding plugin, and specified territory 745 always applies Feast rules.
Implemented native ActionRequest routing in 745, automatic WHM enable in tick, prevention of !whm off bypass, request sequence preservation through casts/results, upgraded spell aliases, recast-details packets from Lua timers, and native Temperance status1872 for15s with expiry/death cleanup.
All 27 world library tests passed, including new native cast/sequence test and Lua recast/visual tests.
Live validation pending: disable Kawari Feast plugin and use /hotbar set for Glare, Cure II, Temperance. Verify cast loop/release/interruption, GCD 2.4s and Temperance60s recast, wings15s and native fade. Current client CureII cast2s differs from Feast1.5s. Full kit still has client lily/resource/charge restrictions; reconnect recast sync outstanding.
Build linked successfully but cargo cannot replace running target/debug/kawari-world.exe (Windows file lock). Building Oodle-enabled variant for deployment; do not deploy non-Oodle build. One player online; require normal logout before stopping server. Existing server still running old build.
Deployment complete: commit eed0e921. User closed game; database online=0. Oodle-enabled cargo build completed after stopping old services. Started hidden kawari-run PID73768, world72628; all eight services started, world listening21063 at2026-09-15T06:44:23Z. No startup stderr. Plugin-free live test still pending; user must keep Kawari Feast disabled. Previous handoff's Astra config path no longer exists, so startup setting was not changed. Do not assume plugin is disabled.

## Feast medal match implementation, 2026-09-15
User requested medals, KO half drops, role caps, starting countdown and timed team winner; explicitly approved small test teams.
Implemented servers/world/src/server/feast_match.rs: automatic60s preparation, balanced up-to4/side (solo allowed),100medals each,6min match,half KO drops rounded up, opposing living walk-over pickup2y,200/175/150/125 caps including owned ground reserves, overflow retained,60s return, score result plus2min sudden death next pickup win, disconnect all drops/rejoin reclaim,30s empty reset. Countdown native packet; clocks/scores/results chat and !feast status; !feast restart after result. EObj2002056 lion medal placeholder. Lua/native combat gated and server teams authoritative.
33 world lib tests pass. Oodle build linked; cargo replacement blocked by running executable. Staged target/debug/kawari-world-medals-staged.exe. Waiting for normal logout from current player before restart. Old server still running.
Important scope: current territory is open-world metadata. Native opposing-player targeting and medal model appearance require in-game multiplayer validation; historical PvP HUD not restored. WHM only action kit. docs/src/development/feast-match.md describes exact assumptions and limits.
Medal deployment complete:845e2fa4. User normal logout confirmed online0. Oodle cargo build succeeded. Hidden kawari-run PID77028; all8 startup lines good, world21063 at2026-09-15T07:01:34Z, stderr empty. Live match/pickup validation pending. Native combat/cast/gcd from previous version was user-confirmed good; do not conflate that with new match validation.

## Status routing fix and solo dummies (current turn)
User confirmed countdown works but !feast status reached Lua usage; requested stationary dummy players and optional retaliation for medal/death testing.
Root cause main.rs executes registered Lua command before world routing. Added shared public feast_commands::is_match_command used in main Lua precedence filter and server chat dispatch, whitespace-aware, test-covered. Match subcommands status/restart/dummies/attack now bypass Lua arena entry script.
Added server/feast_training.rs and Instance.feast_dummies: !feast dummies [1-3] (default1) resets solo practice/countdown, clones player's appearance as hostile enemy NPCs,5000HP100medals healer cap, allClaws vsplayerFang. Generic NPC AI skips dummies; corpses don't fade/despawn until nextsetup. !feast attack on/off controls fixed1000damage/2.5s/range3y strikes using action870 with normal HP/death pipeline and Feast mitigation. No chase. Setup only if sole realplayer, alive,combatjob. Match tick includes dummy inputs for drops/pickups; empty-arena cleanup still based on realplayers. Restart afterresult recreates dummies.
All36 libtests pass, including commandclassifier and integrated setup/attackgates/dummydeath/medaloverflow. Oodle binary linked but cannot replace running image. Awaiting user normal logout confirmation; old server running. Documents updated docs/src/development/feast-match.md. Visual/runtime test pending afterdeploy.
Deployment complete for0bb3d094: user logout confirmed online0, Oodle build successful, hidden kawari-run54972/world75408. All8 services started; world21063 at2026-09-15T07:31:29Z; stderr empty. Ready for live !feast status, !feast dummies2, !feast attack on/off tests.

## Feast HUD asset audit (read-only)
User asked why native medal counter/clock missing and whether assets remain. Current mode still open-world restored metadata; server only native countdown + chat scores, no Feast director/HUD hookup.
Verified installed7.51h2 executable strings: PvPColosseumHeader, PvPColosseumPartyList0/1, PVPColosseumResult, Feast nameplate configs. ClientStructs Revive retains FeastMedalsPrevious/Current (source evidence, not proof working HUD).
Lumina verified ui/uld/pvpcolosseumresult.uld(33548bytes), pvpcolosseum.uld(32288bytes), corresponding PVPColosseum.tex/ColosseumResult.tex; profile textures feastParty.tex/feastSolo.tex referenced. Guessed pvpcolosseumheader/partylist and colosseumtacticalinfo ULD paths absent. Enumerated1253 ULD layouts in ui/uld via060000.win32.index (folderCRC A80C432B); found results/profile Feast references but did not positively identify original live scoreboard layout. Do not claim native medal HUD is fully preserved or ready to activate. No game/server files changed; no restart needed. Temporary audit program target/feast-api-inspect/Program.cs scans ULDs by folder hash; latest filters Feast textures. More work needed to trace surviving addon layout registration/director support or recover old ULDs.

## Historical Feast HUD recovery, 2026-09-15
User requested thorough inspection of C:\dev\kawari\ffxiv-patches(1), correctly distinguishing pre-6.1 archive from installed client. Reused 164-patch scan through 2022.03.01; scanned 964 historical ULD layouts. Recovered 290 named UI/dependency files (41,625,928 bytes) plus 7 unnamed ULD candidates into target/feast-hud-recovery. Found original pvpcolosseumheader.uld/.tex/_hr1.tex and pvpcolosseumgauge.tex/_hr1.tex, all five missing in installed client. Visually verified original blue/red banner/crest atlas and gauge. Named files parsed with Lumina; all SHA256 rechecked. 209 identical to current,76 changed,5 absent. Sixteen shared dependency payloads predate archive; seven ULD names unresolved. Not all recovered assets Feast-exclusive. Existing388arena recovery retained. Full report target/feast-hud-recovery/README.md with manifest.json,validation.json,SHA256SUMS.txt,preview/feast-hud-atlases.png. Scripts target/feast-recovery-tools/recover_hud.py and preview_hud.py. No live client/server modifications. Native HUD activation/modern ULD compatibility remain untested; missing client resources need a client installation mechanism even with server-native gameplay.

## Feast display HUD implemented and deployed, 2026-09-15
User requested a usable HUD from recovered assets and confirmed logout then game closed. Added standalone tools/feast-hud (Kawari.FeastHud API15), display-only, no hooks/hotbars/actions. Original recovered header PNG + simple text/shapes show scores, clock, phase/winner, personal medals/cap, ground medals. Automatic territory745-only once/sec !feast hud requests; server private KFEASTHUD1 snapshots; hides command echo and consumes valid system notices. 4s stale timeout. /feasthud preview toggles sample view. Server authoritative snapshot and shared command routing added;37 world tests pass;11 parser cases pass;plugin Release build clean. Static layout reference target/feast-hud-recovery/preview/hud-layout-preview.png visually checked, not an actual game capture. Live rendering/notice delivery remain pending user test.
Online0 before restart. Oodle world build passed; hidden kawari-run8144 restarted services at08:01:49UTC, world started08:01:55UTC, all8 healthy stderr empty. Registered new plugin in LocalAppData/astra/dalamud/{1a7f787d-aaec-4bee-95c7-ad37c5863230}/dalamud-config.json with backup suffix.before-feast-hud-20260915-170400. Disabled old Kawari.Feast.dll DevPluginLoadLocations and StartOnBoot; new HUD starts on boot. User should launch Astra and !feast. No game SqPack files changed. Install.ps1 safely backs up/preserves config, requires closed game. Native old addon compatibility remains unverified; this is an optional display plugin while combat remains server-native.

## HUD redesign and real medal prefab, 2026-09-15
User rejected red/blue HUD artwork/default font and requested EVE-like minimal industrial styling, plus original dropped medal model. Found current questlion2002056 points to warriorsmedal01_vfx01, generic event item. Scanned ExportedSG/EObj and parsed native PvP002 SGB with Physis: EObj2007627 (SG12640) sg bg_w_pvp_002_01a.sgb has explicit dropmedal_f1(3),c1(5),neutral_n(25) timeline names. Models w_pvp_002_01a/02a/03a and b0815coin1..4 effects. Archive confirms SGB2019.05.29,model2016.12.17,VFX2018.01.12;current paths still readable. Switched drops to2007627 and args1=3/5 by owner team. Live animation/scale need user check.
HUD now620x108,graphite flat panels,thin dividers,neutral score-share rails,subtle ownteam accent. Removed banner dependency/art. Managed Bahnschrift font handles baked24px labels/54px numerals for crisp render; actual sizes10-34px, letterspacing. Preview target/medal-research/hud-redesign.png is static layout reference,not game screenshot.37world tests pass,Release plugin build no warnings/errors. online0 verified before Oodle server restart;hidden kawari-run75300. User must reload Kawari Feast HUD or restart game to load new DLL. Old combat plugin remains disabled. No client SqPack replacement. Docs updated. Research dumps in target/medal-research, old cache unchanged.

## Team jail spawn and native barrier lifecycle, 2026-09-15
User requested team-based jail starts and opening barriers at match begin. Inspected current original LGBs with Physis (target/feast-gates.txt). planmap EObj2008986/2008987 bound to start_col collision groups7403947/7403946 at(-58,.25,0)/(58,.25,0). Separate VFX groups7515404/7515405. Collision SGB gmc16 parsed jail_r_on sub2/off3; descriptor slots[2,2,3,3]. Blue VFX b1418 jail_b_on6/off7/off2on8, slots[6,6,7,8]. Native AC SetSharedGroupTimelineState object_type1 + layout_id controls both pairs: bit1 ON,bit4 OFF. Runtime persists EObj.args1 for late spawns and tracks per-client gate state; ZoneLoaded invalidates tracking. Countdown closes,active opens,SD/results open,restart closes. New realplayers placed at team x±58,y.5,z spaced4slots,facingcenter. Respawn uses ownteam. !feast initial staging now-58 instead of-37, repeatedinsidecommand no longer warps towrongteam. Runtime sync avoids repeated playerteleport. Dummies retain stationary testplacement.
Also corrected medal SpawnObject.args1 from raw3/5 to slotmask0x4/0x10 after confirming packet's bitfield semantics while inspecting jail control. Native medal prefab unchanged.38worldtests pass including integrated jailplacement,startopen,normalmovementnotreset,restartclose. Oodlebuild passes. Online0 checked immediately before stopping old server;hiddenkawari-run74116 restarted. Live jailfloor/collision/VFX removal remains user validation. No plugin changes or client restart needed; enter/reconnect and test countdown. Docs updated docs/src/development/feast-match.md.

## Barrier routing correction + graphics resize diagnosis (2026-09-15)
User reported barriers unchanged, whole3Dscene cropped on resize,chattextmissing,and fullscreen onlyworks1280x720. User disabledHUD and reproduced. Barrier root: prior AC object_type1 looked up StandObjectManager ambient objects; actual mapSGs aren't there. Changed to network EObj controllers bound to each of four jailSGs; reuse collisionactors bybind_layout_id, create twoVFXcontrollers usingpop1gatebase2008986/7,SGBPath0. Send ActorControl against actualcontrolleractor ID object_type0/layout0, persistargs1, wait recipient has_spawned before marking sent, invalidate ondespawn/reload. Added realchannel packet test (waitspawn,4actorsclosed/open,correctrecipientlookup,noreplay).39worldtests pass,Oodlebuildpass. User closedafterrequest;online0verified;hiddenkawari-run80368 deployed. Live barrier checkpending.
Resize diagnosis: readonlyprocessgraphicsprobe target/display_diagnostic.py reportsDevice1280x720,SwapChain1280x720,window1920x1080,DPI96,pending0. Astra's actualconfig is AppData/Roaming/astra/user/{1a7f787d-aaec-4bee-95c7-ad37c5863230}/FFXIV.cfg, NOT retailDocumentscfg (initialborderlessstatement camefromretailcfg;actualAstraScreenMode1fullscreenandFullScreen1280x720 atinspection). Dalamudlog AppData/Roaming/astra/log/dalamud.log repeatedly logs '[InterfaceManager] invalid call to resizeBuffers' inclafterHUDunload17:34:46. Hooked Present atDiscordHook64.dll+0x1CEB0; live modules DiscordHook64.dll plusNVIDIA nvspcap64.dll. Suspectedoverlayconflict,NOT confirmed. Asked user disableDiscordoverlayforFFXIV+close;useransweredclosed. No displaysettingschanged;HUDremainsuserdisabledfortest. Next: relaunch andtryresolutionchange; checknewlogerrorsandprocessdimensions. Ifstillfails, isolateNVIDIAoverlay thenlaunchwithoutDalamudentirely. Do notclaimgraphicsfixedyet. Serverdeploymentindependentofgraphics. Needuserin-gamevalidationforboth.

## Preparation, medal pickup feedback, and slidecasting (2026-09-15)
User confirmed jail barriers work. Preparation now 10 seconds. Medal pickups trigger native prefab timeline slot 2 dropmedal_off_a, containing placednpc_PvPpoint_depop.scd; retain object one second before cleanup, then respawn leftover partial piles. Same-tick death/pickup creates a temporary actor. Fade actors are cleaned on resets/empty arena. Lua WHM movement permits final 250ms, effects resolve once at full deadline, early movement is latched by actor movement handler even across delayed ticks. Completion uses original caster position for range. Other validity checks remain. 40 world tests pass including Lua boundary/late movement, exactly-once damage/healing and native pickup lifecycle. Docs updated. User normal logout confirmed online0; Oodle build passed and hidden kawari-run PID81452 restarted at09:04:25UTC. Live pickup animation/audio and slidecast validation pending. No plugin changes. Graphics resize issue still unconfirmed.

## Follow-up: 500ms grace window (pending deployment)
User reports occasional Glare cancellations and requests 500ms. Changed Lua SLIDECAST_WINDOW to0.500, moved early cancellation regression to0.999s for1.5s cast, added1.0s boundary acceptance and Cure/range boundary coverage. All40world tests pass. Docs updated. Online1; async normal logout request pending. Do not restart until logout. Medal question explained: original official Feast rules count own unclaimed ground medals toward cap. Their100held + inferred25own-ground =125; cannot collect remaining13. Official source https://na.finalfantasyxiv.com/lodestone/playguide/contentsguide/wolvesden/thefeast/ explicitly confirms this; no medal rule changes made.
500ms deployment complete: 04001456. User logout confirmed and online0 verified. Oodle build succeeded, hidden kawari-run PID66176; all8 services started, world ready2026-09-15T09:20:21Z, stderr empty. Ready for live slidecast check.

## Native medal variants and redundant countdown removal (pending deployment)
User supplied screenshot of four medal tiers and blocked overlay. Found four installed EObj/ExportedSG prefabs:2007627 pvp002_01a,2007628 _02a,2007629 _03a,2008631 _04a. Dumped SGBs target/medal-research/{id}.sgb/.txt. All expose normal Fangs/Claws slots3/5 and marked/max variants slots9/11 (dropmedal_f1m/c1m through f4m/c4m); descriptor sub IDs differ but slot positions stable. Runtime selects prefab by1-25/26-50/51-75/76+ count, including same-tick pickup temporary actor, leftover respawn picks remaining size. Per-viewer medal_viewers cache sends native actor control to spawned object only, normal vs blocked based on active roster/job/alive/team/cap including own reserved drops. Cache invalidated on zone load/despawn and removed actors/viewers. Shared pickup_space helper keeps capacity rule consistent. In-game marked visual appearance pending.
Removed regular Feast Countdown packet plus dummy-practice hardcoded60s Countdown packet (the reported obsolete timer); corrected practice text to10s. HUD is sole preparation display. No plugin edits. Added size/cap/team/job/death visual selection regression;41world tests pass, no compiler warnings;diff check passes. Online1 and async normal logout request pending; do not restart until logout. Files docs/src/development/feast-match.md,servers/world/src/server/feast_match.rs,feast_training.rs. Previous deployed04001456 remains live.

## Ten-second respawn (2026-09-15)
User requested10s respawn delay and disabled countdown confirmation. Added Instance.feast_respawns monotonic deadlines, set from kill_actor (realplayers745 only) and fallback dead observation in match tick. Repeated kills preserve deadline; living/absent input clears stale. ReturnAfterDeath745 denies before deadline (fallback starts10s if missing), then removes deadline and existing teamrespawn. Private KFEASTRESPAWN1|milliseconds accompanies !feast hud responses. HUD plugin injects IGameGui/IObjectTable, identifies SelectYesno owned by AgentRevive.GetAddonId, changes native YesButton label OK(Ns), disables until freshserver0 plus local10s minimum. Restores label/state at life/unload; no automatic clicks/combat hooks. Requires plugin reload or game restart. Live button validation pending.42worldtests pass including deadline9999ms/10000ms,repeateddeath,newdeath;Release plugin build clean.
Read-only native research: target/re-tools capstone/pefile, target/inspect_revive.py,revive_refs.py,revive_dispatch.py and disasm txt. Installed exe native UIState=0x142ab6f90,Revive=+0x1360=0x142ab82f0. ActorControl category0x358 (856) target0x140b38a45 jumpentry0x140b429ec table0x140b41c8c: param1playerid,param2timer*0.1,param3threshold*0.1,param4oldmedals,param5newmedals. Setter0x140c0b290 writes+18timer,+1cthreshold,+20/+24medals. Native generic openworld prompt doesn't use timer for disabledbutton; PvPdirector choosesdialog6/8 (can auto-return at0), so did NOT add guessed packet or altermetadata. AgentRevive open0x140cc5fd0 calls0x140cc6910 and storesaddonID+20; validatesplugin ownership check. No client file modification.
Online0 detected and rechecked before authorized hiddenserver restart. Build/restart pending unifiedexec session34723. Also deploys precedingb306dc11medalvariant/countdown removal. No user logout reply required as zeroonlineverified. Need finish serviceverification,commitandhandoff. PluginDLL built atregisteredpath; user reload required.
Deployment complete:3c62d85f plusb306dc11. Oodlebuildpassed,hiddenkawari-run80856,all8serviceshealthy,worldready2026-09-15T09:35:36Z,stderr empty. User must reloadKawari Feast HUD or restartgame for nativebutton countdown. Ingame medalvariants and respawnbutton stillunverified.

## Native director investigation and probe (2026-09-15, pending deployment)
User authorized investigate/migrate ifnatural. MAJOR finding: native legacyPvPfactory DOES survive. Initialmetadata-only assumption couldhave suggestedallremoved but wrong. Read livePID81068 EventFramework signature and StdMap DirectorFactories:13entries,0x8003factory141aa4180. ActiveContentDirector0 atprobe. FactoryIDs40001-40026 ALLbranch141aa7619 allocate20f8 ctor1419ad440 ->141a378c0, derivedvt14234e100. CC40027-47 branch141aa764d allocate23c0 ctor141a3acb0. Sharedbasevt1423f1c90;legacyoverrides slots0,381(RVA19ac840),404(19ac9f0), nonemptyhandlers. Same type5 getters can still assumeCC downstream => needruntimevalidation,don'tclaimfullnative restored. InstalledIC40001type5CFC121blank;40002..26metadata0. OldCFC recovered476/478/479/480/580 (CrystalTower) frompre6.1archive2022.03.01. target/feast-director-research containsEXH/EXD forInstanceContent andCFC; schemasdifferentfrommodern. Territory745exploration intended1 CFC0.
Added opt-in!feast director on|off compatibilityprobe tosharedrouting,FromServer::FeastDirectorProbe(bool),mainhandler,andZoneConnection::feast_director_probe. Usesexisting40001flags0 InitDirector,trackscontent_handler/current_instance_id forautomaticzonechange/respawntermination,offexplicitTerminateDirector. Doesnotchangeinstalledmetadata,doesnotreplaceexistingrules orsendnative scores. Needsliveusercommand afterdeploy.42worldtestsandcargo checkpass. online0 lastcheck. Must buildOodle/restartandchecklogs thenuser !feast director on; run target/director_live_probe.py PID toinspectactiveclass. Research tools requireescalation because readprocess/capstone ACL; no writes toprocess. Detailedreportdocs/src/development/feast-director-investigation.md. Neednotrequestplugin; noHUD changes.
Probe deployment complete895de8b6:online0beforestop,Oodlebuildpassed,hiddenkawari-run78888;all8healthyworldready2026-09-15T09:47:46Zstderr empty. Asyncuserquestionpending: login,!feast,!feast director on,replyenabledandstayinarena. Next inspectliveEventFrameworkDirectorModuleActiveContentDirectorandvtable viaread-onlyprobe,thenprotocol/migrationwork. Do NOT treattheprobeascompletedmigration. User goal stillnativeapproachifcompatible.

## Probe failed in live use; restart and duty-path comparison
User!feastdirectoron at10:15:03UTC sawnochange,relogged invisible,queuedTripleTriadBattlehallanddisconnected. Processremainedalive,no recordedpanic;timeoutsandworldmessagesstalledafterzoneinit. Don'tclaimcausalityproven. Logs preservedtarget/director-failure-server.log/stderr.log. Userexplicitlyrequestedrestart;onlineflag1apparentlystale,restartauthorizedevenwithoutlogout. Firsttoolabortedbeforeexecution;secondcompleted. Disabledprobeactivation inbothservercommandandZoneConnection;offretained. Oodlebuildpassed,hiddenkawari-run74536;all8healthyworldready10:19:49UTCstderr empty. Fullgame restartrecommendedtoclearnativedirectorstate.
UseraskswhethercommandvsDutyFindercausesissue,whetherSastashasupported/director/queueable. VerifiedSastasharesources/scripts/content/s1d1_re.lua onSetup119,onActorDeath231,complete_duty269 (partialmechanicsTODO). QueueDuties main2345 ->register_for_content zone_connection/mod344 ->join_content zone.rs storesoldzone/position ->ToServer::JoinContent server/mod2104 ->create_instance_for_content139,loadsLua,onSetup,setscontentfinderid,entrance;zoneconnection setsBoundByDuty andInitDirector duringzoneload. Existing!feastteleport bypasses allthis. Actualgap islifecycleandmetadata,nottypingcommanditself;commandcanusequeuepath. Probe40001 attachedafterzoneinit withoutdutycontext too incomplete. NextnativeexperimentshouldusepropercontentlifecycleandvalidrestoredCFC/ICmetadata,Sastashacontroltestfirst. TripleTriadhangmightalsobeexistingnetwork/data lockinversion;unproven,no fixclaimed.
