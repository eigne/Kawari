# Feast match prototype

Territory 745 starts a 10-second preparation countdown when its first player
arrives. The server balances arrivals between Fangs and Claws, up to four on each
side. Small teams (including solo testing) are allowed. The roster locks when the
six-minute match begins. New arrivals wait for the next match; changing jobs during
a match blocks actions and pickups until returning to the original job.

Commands:

* `!feast`: enter the arena.
* `!feast den`: travel to the Wolves' Den Pier, where PvP actions are chosen.
* `!feast status`: time, team scores, your medals and cap.
* `!feast restart`: start another countdown after a result.
* `!feast leave`: leave the arena.
* `!feast dummies [1-3]`: start fresh solo practice with stationary opponents.
* `!feast attack on` / `!feast attack off`: enable/disable their retaliation.

## Solo practice

`!feast dummies` creates one opponent by default; use `!feast dummies 2` or `3`
for more. Setup requires you to be the only real player in the arena and alive.
It replaces prior test opponents, resets medals and the countdown, and restores
your HP/MP. You are Fangs; all dummies are Claws. Multiple dummies deliberately
make the teams uneven for testing.

Dummies copy your appearance but are enemy NPCs so native hotbar targeting works
without a PvP client mode. Each has 5,000 HP, 100 medals and the healer medal cap.
They remain stationary and passive even when hit; generic NPC AI skips them.
`!feast attack on` enables a 1,000-damage attack every 2.5 seconds within three
yalms, subject to Feast mitigation. Countdown, result, death and range checks
prevent attacks. `off` disables them immediately. They do not chase you.

Kill an opponent and walk over its drop. As WHM, your initial capacity only allows
25 of its 50 dropped medals. To test your own death, enable attacks and stand
near a living opponent after the countdown. Use Return after death. Create new
dummies to reset practice, or use `!feast restart` after a result. Dead dummies
remain as corpses until the next setup; they do not auto-respawn.

The shared `feast_commands::is_match_command` classifier bypasses the registered
Lua `!feast` entry script for match/training subcommands. Without that bypass,
Lua consumes `!feast status` before the world server sees it.

Every participant starts with 100 medals. A KO drops half, rounded up. Only living
opponents can collect a drop by walking within two yalms. Role caps are tank 200,
melee 175, ranged/caster 150, healer 125. Unclaimed medals dropped by a player still
reserve space under that player's cap. Overflow stays on the ground. After 60
seconds unclaimed medals return to their owner. The two-yalm pickup radius,
60-second return time and odd-medal rounding are prototype choices; the official
guide does not specify their exact values.

At the deadline the team holding more medals wins; ground drops are not held
medals. A tie starts two minutes of sudden death: the next successful pickup wins,
otherwise the result is a draw. Combat and scoring stop at the result. Early
600-medal victory, culling, heavy/light medals, supply kits, ratings, rewards,
matchmaking and full role kits are outside this iteration.

Disconnecting drops all held medals at the last observed position. Rejoining the
same instance with the same actor ID returns any unclaimed disconnect drops.
The arena resets after it has been empty for 30 seconds. Match state is in memory
and is not resumed after a server restart. This is a shared test arena, not isolated
matchmaking instances.

## Presentation and validation

Preparation uses the display HUD timer; the separate battle countdown overlay is disabled. The running clock and
score changes use chat notices (every 30 seconds and each of the last 10 seconds)
and `!feast status`; the optional display HUD is described below. Drops use the
native Feast medal prefab EObj 2007627. Native opposing-player targeting still
requires multiplayer client validation.
Server rules enforce opposing teams independently of client presentation. The
existing arena is still restored with open-world metadata, so this change does
not claim to restore the full native PvP client mode or team UI.

The native WHM implementation remains the only implemented role kit. No plugin is
required for match state, pickup detection, timer enforcement or scoring.
Keep the old Kawari Feast plugin disabled.

`cargo test -p kawari-world --lib` covers countdown/roster locking, role caps,
single KO processing, overflow and conservation, reserved capacity, expiry,
disconnect/reconnect, scoring cutoff, sudden death, and action gating.

References: [official Feast guide](https://na.finalfantasyxiv.com/lodestone/playguide/contentsguide/wolvesden/thefeast/)
and [original starting medal rules](https://na.finalfantasyxiv.com/lodestone/topics/detail/c755d756aece79cc9707f5444ad0980ac2b5a39c).

## Display HUD

The optional display-only `tools/feast-hud` Dalamud plugin shows live team medals,
match time/phase, personal medals/cap, and ground medals using a compact graphite HUD with Bahnschrift typography.
It requests private `!feast hud` snapshots once per second in territory 745; no
periodic traffic is sent for clients without it. The old combat plugin stays disabled.
Native historical director/ULD activation is still not implemented. See the HUD
README for installation and validation.


### Native medal pickup visuals

Drops use EObj 2007627 / ExportedSG 12640:
`bgcommon/world/pvp/shared/for_bg/sgbg_w_pvp_002_01a.sgb`.
This is the actual Feast prefab, not quest EObj 2002056 (lion medal).
Its models are `bgcommon/world/pvp/002/bgparts/w_pvp_002_01a.mdl`
through `_03a.mdl`, with `b0815coin1_o.avfx` through `b0815coin4_o.avfx`.
The parsed SGB names its timelines `dropmedal_f1` (3), `dropmedal_c1` (5),
and `dropmedal_n` (25). Spawn args1 selects timeline-slot masks 0x4/0x10 from the drop owner's team.
Historical index verification places the SGB in D2019.05.29, the first model
in D2016.12.17 and the first coin VFX in D2018.01.12. All are still present
in the installed client. Native appearance and timeline activation need live
validation; no client asset replacement or invented medal artwork is required.

Successful pickups play timeline slot 2 (`dropmedal_off_a`), including its native
`placednpc_PvPpoint_depop.scd` sound. The server retains the object for one second
to finish that animation before deleting it. If a role cap leaves medals behind,
the remaining pile reappears after this animation. Expiry does not play pickup
feedback. Animation and audio still require an in-game check.

### Cast completion and slidecasting

Native Feast WHM casts allow movement during their final 500 ms. Effects resolve
once at the full cast deadline; movement during the grace window or after that
deadline cannot interrupt them. Earlier movement is recorded immediately, so a
delayed server tick cannot accidentally accept it. Completion range checks use
the caster's original position so sliding does not move the cast out of range.
Other validity checks, including target state and crowd control, still apply.


### Team jails and match opening

Players stage at x=-58 (Fangs) or x=58 (Claws), y=0.5, with four spaced
positions across each holding area. Jail centers come from the original start_col
layout. Respawns use the same team side. Every new match places the players again;
normal movement during a match is not overridden. Zone loads invalidate the
client's cached gate state so reconnects receive the current state.

The countdown closes both collision shared groups (7403947/7403946, bound EObjs
2008986/2008987) and the separate VFX groups (7515404/7515405). All four use network EventObject controllers bound to their layout shared groups.
The two visual groups receive synthetic controllers because the map supplies none.
SetSharedGroupTimelineState targets each controller actor (object_type=0): timeline
slot 1 closes, slot 3 opens. StandObjectManager lookup is not valid for these groups.
Parsed collision animations are jail_r_on/off; the visual prefab has jail_b_on/off.
The server also stores state in SpawnObject.args1 for subsequent spawns, and
waits for each recipient's object spawn allocation before sending an update.
The active phase opens the gates; they stay open through sudden death and results,
and close for the next preparation. This uses native layout control, not a plugin.

Medal piles select native EObjs 2007627/2007628/2007629/2008631 for 1–25,
26–50, 51–75, and 76+ medals. Partial pickups respawn with the remaining size.
Per-viewer timeline slots 9/11 show the native blocked variants when the viewer
cannot collect (own team, death, job lock, inactive match, or held plus reserved
medals at cap). Slots 3/5 restore eligible visuals. Updates wait for object spawn
and are resent after reconnect; range alone does not mark a pile blocked.

### Respawn delay

Feast players must wait ten seconds after death before Return is accepted. The
server records a monotonic deadline at death; repeated death notifications do not
restart it. Living players clear the previous deadline. The display plugin adds
an OK countdown to the native revive confirmation using private KFEASTRESPAWN1
snapshots. It only alters the dialog owned by AgentRevive, and waits for a fresh
server-ready reply before enabling confirmation. Without the HUD plugin the server
still rejects early returns, but the native button has no custom countdown.
