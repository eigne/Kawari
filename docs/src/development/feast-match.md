# Feast match prototype

Territory 745 starts a 60-second preparation countdown when its first player
arrives. The server balances arrivals between Fangs and Claws, up to four on each
side. Small teams (including solo testing) are allowed. The roster locks when the
six-minute match begins. New arrivals wait for the next match; changing jobs during
a match blocks actions and pickups until returning to the original job.

Commands:

* `!feast`: enter the arena.
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

The countdown uses the normal client countdown packet. The running clock and
score changes use chat notices (every 30 seconds and each of the last 10 seconds)
and `!feast status`; the historical Feast HUD is not restored. Drops use the
installed lion-medal EObj 2002056 as a visible placeholder. Its exact appearance
and native opposing-player targeting require multiplayer client validation.
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
and `dropmedal_n` (25). Spawn args1 selects 3/5 from the drop owner's team.
Historical index verification places the SGB in D2019.05.29, the first model
in D2016.12.17 and the first coin VFX in D2018.01.12. All are still present
in the installed client. Native appearance and timeline activation need live
validation; no client asset replacement or invented medal artwork is required.
