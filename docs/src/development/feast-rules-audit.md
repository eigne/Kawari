# Feast rules audit — final pre-6.1 rules

Researched 2026-09-21. Target: the six-minute Crystal Tower Training Grounds
version with role-based medal limits, not early eight-minute Feast or CC.

## Sources and decisions

- [Official English guide](https://na.finalfantasyxiv.com/lodestone/playguide/contentsguide/wolvesden/thefeast/): 60-second preparation, six-minute match, two-minute overtime; role caps; half-medal KO drops; 90-second no-KO Culling timer, then 15-second stack intervals, maximum five; overtime starts at one stack and ignores KOs. Supply effects: +25% adrenaline and 60-second Wolf's Heart (2% maximum MP per second, +10% movement, not additive with other speed buffs).
- [Official Japanese guide](https://jp.finalfantasyxiv.com/lodestone/playguide/contentsguide/wolvesden/thefeast/): explicitly assigns kit ownership by **team aggregate damage**, with one teammate collecting the dropped kit. The English wording is ambiguous. Disconnect drops remain until collected or reclaimed by reconnection. No supply waves during overtime.
- [Revival's Feast guide](https://www.akhmorning.com/pvp/feast/) and [5.3 player guide](https://jp.finalfantasyxiv.com/lodestone/character/5213171/blog/4567299): two adrenaline boxes at 4:30 and 1:30 remaining; one Wolf's Heart at 3:00.
- [Contemporary player explanation](https://forum.square-enix.com/ffxiv/threads/404142): 600 medals ends a match early.
- [Archived Paladin guide](https://gamerstips.com/ffxiv-ff14-paladin-feast-pvp-guide-2019/): baseline adrenaline fills in approximately three minutes. This is weaker evidence than the official timings; validate against a final-version capture.

## Implemented in this candidate

| Area | Change |
| --- | --- |
| Preparation | 60 seconds, replacing prototype 10 seconds |
| Victory | Early 600-medal victory |
| Culling | Server damage multiplier, KO reset, overtime schedule, native status 994 |
| Disconnects | Abandoned medals no longer expire |
| Supplies | Once-only waves, attackable NPCs bound to original layout, team damage accounting |
| Kits | Original native event objects, team-restricted proximity pickup, single consumption |
| Rewards | Adrenaline cap, passive fill, 60 one-second MP ticks, native status 1149 |
| Presentation | F5 adrenaline director record, native party gauge input and native cast bars |
| Cleanup | Finished/reset/empty match cleanup for boxes, kits and controllers |

Gameplay remains server-side. Client work only consumes presentation data.

## Surviving assets reused

See `tools/feast-assets/supplies-7.51h2.json` for hashes, sizes and dependencies.
No proprietary assets are checked in. Audit command accepts `--supplies`.

- `planmap.lgb`: yellow box layouts 7167385/7167387 at (0,-2.5,+/-10);
  central white box 7167407 at (0,-2.5,0).
- `sgbg_l1p2_a0_gmc04.sgb` and `gmc05`: original models, native pop/break
  sound and VFX. Timeline table: off, animated off, on, animated on.
- Kit EObjects 2007626 and 2007652: original `w_pvp_001_04a/05a` prefabs.
  Their descriptor tables retain off/Fangs/Claws/neutral states and animated variants.
- Status 994 (Marked for Culling), 1149 (Wolf's Heart Kit) survive in 7.51h2.

## Explicit provisional values and missing fidelity

Do not describe this candidate as a complete or exact Feast recreation.

- Supply box HP: **5000 provisional**. Final-version HP has not been established.
- Untouched kit lifetime: **60 seconds provisional**. Untouched boxes persist;
  a later wave does not overlap an existing box. Historical expiry behavior needs a capture.
- Pickup radius: **2 units provisional**, shared with existing medal behavior.
- Exact equal-damage supply tie-break: **Fangs wins provisionally**, not historically verified.
- Ordinary KO medal lifetime remains **60 seconds provisional**; disconnect drops persist.
- Passive adrenaline uses 180 seconds. Confirm final-version rate, combat/KO modifiers,
  death retention against captures. Native gauge scale is verified as 0..100 in the June row renderer.
- Verified in the client on November 1: both box models appear, boxes are targetable and
  take damage, break into the team kit, and walking over an adrenaline kit fills the gauge.
  Wolf's Heart movement and kit effects still need a live check. `!feast supply [heart]`
  spawns a wave at once for testing.
- Culling weather changes are still missing; status and damage are implemented.
- The supported combat kit remains WHM. Other historical jobs, stat normalization,
  damage scaling, full crowd-control resistance and line-of-sight still need work.
- Native party status presentation remains incomplete; native casting, targeting and
  additional cooldowns exist, but cooldown icons currently occupy status slots.
- Full role-based matchmaking, eight-player admission, result/reward packets,
  quick chat, inactivity handling and ranked seasons are not complete. Practice mode
  intentionally permits incomplete teams. These require separate integration work.

## Validation

Rust tests cover match boundaries, supply waves, layout bindings, destruction and
cleanup. Lua tests cover Culling damage, capped adrenaline and Wolf's Heart expiry.
C# protocol tests cover atomic F5 gauge frames and malformed/unknown-owner rejection.
Live acceptance remains required, especially the native bound supply NPCs.

## Original executable recovered for further native-code research

Read-only extraction from the existing local patch archive recovered the complete
March 1, 2022 executable (40,178,584 bytes) into ignored scratch storage:
`target/feast-director-research/ffxiv_dx11-6.08.exe`.
SHA256: `4b0a86b3cb5354619a97bc70918492169cf457eaa2bd825cb9e95e17c6c57883`.
It was not executed, installed or committed. Original `PvPColosseumHeader` and
`PvPColosseumPartyList` references are present and available for continued analysis.

June renderer RVA13C5ED2 reads cast percentage from row+13 (divides by100) and
cast name from string row+6. RVA13C5FA4 reads adrenaline from row+12, compares
against100 and scales the native gauge. These existing controls are now fed.
The arena LVB lists weather variants91/63; identifying the Culling variant still
needs verification, so no guessed weather override was installed.
