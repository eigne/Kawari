# Feast HUD

## Local spell presentation alignment

The native GetSpellIdForAction hook resolves WHM Feast button IDs to historical
spell IDs before the client starts prediction and its cast display. This matches
the server's ActorCast/ActionResult spell_id while leaving button request/recast
IDs intact. It addresses the mixed placeholder-caster/Feast-target animation
and the identity mismatch associated with the reported slidecast-bar regression.
Requires a client restart. Live slidecast completion still needs verification;
movement and genuine cancellation handling are unchanged.

## Native party lists (current candidate)

Follow-up: player/enemy widgets explicitly use the recipient's Feast team,
independent of the legacy director's team field. Additional-action icons now
dim during cooldown and have native image nodes using the built-in hotbar recast
atlas. The overlay has no mouse events; native row targeting remains unchanged.
Restart the client to load this DLL. No server restart is needed for this fix.

The ally/enemy lists now use the game's existing PvPMKSPartyList controls.
Only the native score header remains suppressed. The graphite clock/medals
panel is unchanged. HUD Layout still calls the lists Crystalline Conflict
Ally/Enemy Info; their normal positions and scaling apply.

The module supplies roster data to native number array 117 / string array 105.
Native row callbacks resolve actor IDs and select targets for both teams,
without ImGui hitboxes or regular-party membership. Loaded actors supply names,
job icons and HP/MP; missing actors, departed players and stale snapshots hide
their rows. Both teams use the native unassigned-party sentinel (-1) to avoid
the normal party-slot targeting path. Native target highlighting is updated.

Server F4 director records carry WHM Purify and the selected two additional
actions, including cooldowns in tenths of a second. Native icon slots display
the archived action icons and remaining whole seconds. Other jobs do not yet
have server action kits to provide cooldowns. This reuses current native widgets,
not the historical Feast artwork; action tooltip restoration is not included.
Requires a client restart and updated world server. Live row rendering and
click targeting must still be checked in game.

## Compact team footer and cancelled casts

The graphite panel's footer now shows each team's participant job abbreviations
and medals (up to four entries per side), replacing Carried/OnField. Both native
side panels are suppressed along with the native header. Native text injection
has been removed; the director still supplies scores, roster and timer data.

The server refunds the provisional GCD when a cast is actually interrupted or
fails completion, and resets client spell cooldown group 57. Successful casts
retain existing timing; rejected attempts cannot clear a valid GCD. Requires the
updated world binary and client DLL, followed by a server/client restart.

## Follow-up: graphite header and cast timing

The user confirmed native entry/presentation, then requested the previous
EVE-like graphite clock and medal panel back. That ImGui panel now displays
director-frame data; native CC header drawing is suppressed. The native director
and its CC producer replacement remain in use. Team text widgets remain native.

The action-status override now preserves the native casting and recast gates
(and respects the caller's check flags for queueing). Previously it returned
success during these gates, starting requests that the server then cancelled.
Server GCD remains 2.4 seconds from cast start; Glare takes 1.5 seconds. A Lua
regression test verifies cast completion does not restart that cooldown.
Release DLL rebuilt; restart the client through Astra to load it. No server
restart is necessary for this follow-up. Live timing verification is pending.

## Native director candidate (2026-09-15)

The user approved extending this existing Dalamud module for a Feast-only fork.
This section supersedes the earlier display-only description below.

- Requires the exact June 7.51h2 executable; SHA-256 and native function prefixes
  are checked before enabling hooks. No executable or SqPack writes are made.
- Replaces PvPHeader's CC refresh at RVA `D5C0A0` when territory 745 has the
  legacy director vtable `234E100`. Native opening and closing remain intact.
  The replaced routine reads beyond the old director allocation and feeds the
  CC progress and unbinding notification.
- Native `PvPMKSHeader` and team addons host module-owned text nodes. Stock
  requested updates are suppressed; original nodes are hidden and restored at
  addon finalization/unload. This is native text presentation, **not restored
  historical Feast artwork**, HP bars, status icons, or target selection.
- Normal DirectorVars for handler `0x80039C56` carry ordered 10-byte records:
  `F1` summary, `F2` participants, `F3` commit/ground medals, at 4 Hz.
  Only complete frames are published. Continuations are intercepted before the
  stock director; F1 remains in UnionData. Data becomes stale after 2.5 seconds.
  Chat polling remains a fallback until the first native frame. The former
  ImGui display remains available as a preview.
- WHM's live PvP bars receive the 20-action Feast prototype kit. Recovery is
  saved before replacing slots. Owned slots are restored on unload; PvP slots
  are never restored over newly loaded PvE bars. Icons/tooltips remain current
  placeholders, and cross hotbars are not populated. Only implemented WHM
  actions in territory 745 bypass retail action-status gates. Casts use native
  ActionRequest rather than chat commands; server Feast rules remain authoritative.

Validation: Release build succeeds, 11 chat parser and 8 native frame tests pass,
and all 44 world tests pass. Native rendering, notification suppression, slot
restoration and cast acceptance still require an in-game test. Keep the separate
Kawari Feast combat plugin disabled. Leave the duty before unloading this module.

Display-only Dalamud API 15 plugin for the current Kawari client. Combat, casts,
cooldowns and medal rules remain on the server. Keep the old **Kawari Feast**
combat plugin disabled; enable **Kawari Feast HUD** instead.

In territory 745 the HUD automatically shows team totals, preparation/match/
sudden-death time, winner, personal medals/cap and medals on the ground. It is
centered at the top, scales with screen width, and does not intercept mouse input.
Outside the arena it hides and stops requesting state. `/feasthud preview` toggles
sample values for visual checks; `/feasthud reset` returns to live data.

## Build and install

```powershell
dotnet build tools/feast-hud/Kawari.FeastHud.csproj -c Release
# Close FFXIV first; supply the existing Astra profile's Dalamud config path.
./tools/feast-hud/Install.ps1 -ConfigPath '<existing dalamud-config.json>'
```

The installer backs up the config, preserves other settings and plugin entries,
registers this plugin at its build path, and disables the old combat plugin.
Restart the game through the existing Dalamud-enabled Astra profile afterward.
Revert by restoring the config backup while the game is closed.

## Data and limitations

The plugin requests `!feast hud` once per second inside the arena. The server
returns a private, versioned system notice with authoritative state. The plugin
consumes valid notices and hides the internal command echo. Other players do not
receive the snapshot. Clients without the plugin incur no periodic HUD traffic.
A four-second timeout replaces stale numbers with a waiting state; phase changes
and pickups appear on the next snapshot. The displayed clock interpolates whole
seconds between updates. This is a prototype transport over existing server
notices, not restoration of the historical native Feast director.

No hooks, action interception, hotbar replacement or cast/VFX modifications.
Requires client-side Dalamud for drawing; recovered native ULD compatibility is
still unverified. Live-game rendering and notice delivery require user validation.

## Visual design

Graphite panels, thin dividers, neutral score rails, and Windows Bahnschrift
(DIN-derived) typography. The player's team label has a subtle desaturated accent.
The old blue/red banner is no longer loaded or included. UI shapes are drawn
as vectors; no generated raster artwork is needed. Two managed font sizes are
baked at higher resolution for crisp labels and numerals instead of scaling the
small default ImGui font. This Windows client requires bahnschrift.ttf from the
system Fonts directory; the font is not redistributed with the plugin.

## Validation

`cargo test -p kawari-world --lib` covers server snapshots and medal rules.
`dotnet run --project tools/feast-hud-tests/Tests.csproj` checks valid and malformed
snapshot parsing. In game: enter `!feast`, compare against `!feast status`, spawn
`!feast dummies 2`, verify KO/pickup updates, result, restart and leaving the map.

## Respawn confirmation

In the Feast arena, the plugin adds a countdown to the native revive confirmation
and disables its OK button until the server's ten-second respawn delay expires.
Only AgentRevive's SelectYesno dialog is changed. Private KFEASTRESPAWN1 notices
accompany HUD snapshots; stale data keeps confirmation disabled. The server checks
the deadline independently. Unloading restores the original button; no combat
hooks or automatic confirmation are used. Reload the plugin after updating.
