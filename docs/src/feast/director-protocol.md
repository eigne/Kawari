# The director protocol

The HUD needs match data the client has no packets for any more: scores,
the roster, medals, cooldowns, gauges. The server sends it through a packet
the client still handles: `DirectorVars`, the generic "director variables"
update for the arena duty's director (handler `0x80039C56`). Its ten data
bytes carry one record of ours.

Server: `server/feast/director.rs`. HUD: `Director/DirectorFrame.cs` (decoding)
and `Director/NativeDirector.cs` (receiving).

## Frames and records

Four times a second the server sends every player in the arena their own
**frame**: an `F1` summary, any number of other records, and an `F3` commit.
The first byte of each record says what it is; numbers are little-endian.

| Byte 0 | Record | Bytes 1–9 | Who |
|---|---|---|---|
| `F1` | summary | phase (bits 0–2) and own team (bits 3–4), seconds left (u16), Fang score (u16), Claw score (u16), own medals, respawn seconds | per recipient |
| `F2` | participant | team (1 Fang, 2 Claw), actor ID (u32), medals (u16), job, alive | each present participant |
| `F5` | adrenaline | actor ID (u32), adrenaline 0–100 | each participant |
| `F4` | party action | actor ID (u32), slot 0–2, action ID (u16), tenths of a second left (u16) | three per participant |
| `F7` | job gauge | job, six gauge bytes, timer (u16) | the recipient only |
| `F6` | hotbar action | 0, button ID (u16), current ID (u16), cast ms (u16), recast ms (u16) | the recipient's buttons |
| `F3` | commit | medals on the ground (u16) | last |

Phase values: 0 preparation, 1 match, 2 sudden death, 3 Fangs won, 4 Claws
won, 5 draw. Team values: 0 spectator, 1 Fangs, 2 Claws.

The gauge bytes' meaning depends on the job and is defined by `M.job_gauge`
in the Lua engine; the HUD maps them onto the game's gauge widgets.

## Rules for the receiver

- A frame is used only when its `F3` arrives (`DirectorFrames.Accept`), so the
  HUD never shows a mix of two updates.
- Every record is range-checked. One bad or unexpected record discards the
  whole frame: the HUD keeps showing the previous one.
- A frame older than 2.5 seconds counts as missing; the HUD then shows
  "waiting" instead of stale numbers.
- The HUD consumes `F2`–`F7` itself and only passes `F1` on to the game, whose
  director keeps it as normal director data. The game never reads our other
  records.

## Changing the protocol

Add a new record type rather than changing an existing one, and update both
ends in the same change: the builder in `director.rs` and the decoder in
`DirectorFrames.Accept`, plus a test on each side. The summary test in
`director.rs` and the first test in `tools/feast-hud-tests/Program.cs` use
the same bytes, so a change to `F1` must update both.

## Exploration mode

Without the duty there is no director, so no frames. The HUD then only needs
the respawn countdown, which it polls with `!feast hud` (the server replies
with a hidden `KFEASTRESPAWN1|<milliseconds>` chat notice).
