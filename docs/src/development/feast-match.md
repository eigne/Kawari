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
