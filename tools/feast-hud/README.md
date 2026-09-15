# Feast HUD

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

## Artwork provenance

`assets/header.png` is a lossless PNG decode of the original recovered
`ui/uld/pvpcolosseumheader.tex` (896×104), from local pre-removal patch
`D2018.04.27.0000.0001.patch`. Decoded TEX SHA256:
`1a586a38bc9ca12927676d80d8e5cf15a92e97d04d8f104fcd682c586e353f6d`.
See target/feast-hud-recovery/manifest.json for the complete source inventory.
The banner supplies the original blue/red motif and crest. Labels, numbers and
background are drawn directly as simple text and shapes. No generated artwork.

## Validation

`cargo test -p kawari-world --lib` covers server snapshots and medal rules.
`dotnet run --project tools/feast-hud-tests/Tests.csproj` checks valid and malformed
snapshot parsing. In game: enter `!feast`, compare against `!feast status`, spawn
`!feast dummies 2`, verify KO/pickup updates, result, restart and leaving the map.
