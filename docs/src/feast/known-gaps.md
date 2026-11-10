# Known gaps and next steps

What is missing, what is a guess, and what would make the Feast more
faithful. Values marked provisional are constants you can change in one
place once better evidence turns up (for example a packet capture from
patch 6.08).

## Provisional values

| Value | Now | Where |
|---|---|---|
| Damage | potency as HP | the Lua engine |
| Supply box HP | 5,000 | `BOX_HP`, `feast/supplies.rs` |
| Untouched kit lifetime | 60 s | `KIT_LIFETIME`, `feast/supplies.rs` |
| Medal pile lifetime | 60 s | `DROP_LIFETIME`, `feast/rules.rs` |
| Pickup radius (medals and kits) | 2 yalms | `PICKUP_RADIUS`, `KIT_PICKUP_RADIUS` |
| Supply tie-break | Fangs | `feast/supplies.rs` |
| Passive adrenaline | full in 180 s | `M.advance` in `Engine.lua` |
| Preparation | 5 s | `COUNTDOWN`, `feast/rules.rs` |
| Cone angle, line width, proximity curves | 90°, 2 yalms, linear | job files; see [Jobs](jobs.md#how-faithful-the-kits-are) |

The real preparation was longer; 5 seconds is a development convenience.

## Not implemented

- **Matchmaking and ranks**: no queue that forms teams of four from separate
  players, no ranked tiers, seasons, rewards or result screen. Teams are formed
  from whoever is in the instance during preparation.
- **The location-action packet**: ground-targeted actions go through a chat
  command sent by the HUD.
- **Line of sight and collision** for actions and dashes.
- **Native icons for a few statuses** that have no matching Status row
  (internal ones such as Riddle of Fire's weaponskill bonus, Scorch Ready,
  Verfinisher, Astral Fire and the Dragoon gauge states). They work but are
  invisible; the job gauge shows the gauge states.
- **Weather** changes that came with Marked for Culling.
- **Original UI windows** other than the header: the Feast's PvP Actions
  window and result screen were rebuilt or left out.

## Updating to a new client version

The fork supports one client build at a time (currently 7.57h1). After a game
update:

1. Re-run the client data patch's `prepare` steps; update the expected build
   in `tools/feast-map` if they refuse. Reinstall the patch.
2. Update the Dalamud SDK in `target/dalamud-sdk` and the HUD's
   `NativeClient.cs`; see [When the client updates](hud-plugin.md#when-the-client-updates).
3. Rebuild the server (Kawari's own update steps apply: opcodes, sheets).
4. Run the automated tests, then the in-game checklist in [Testing](testing.md).
