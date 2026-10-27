# Feast action asset audit

Read-only inspection of the installed client using the local Dalamud SDK's Lumina
libraries. No game assets are extracted, patched, or committed. Build output is
ignored by this tool's bin/obj rules.

```powershell
dotnet build tools/feast-assets/Audit.csproj
dotnet tools/feast-assets/bin/Debug/net10.0-windows/Audit.dll 'C:\Program Files (x86)\SquareEnix\FINAL FANTASY XIV - A Realm Reborn\game\sqpack' > target/feast-action-assets.json
```

The checked-in `verified-7.51h2.json` records the September 15 inspection of client
2026.06.18.0000.0000: all 40 end/hit timeline references for the 20 Feast WHM
actions resolve, as do all 281 distinct discovered dependency paths.

The audit follows printable absolute chara/vfx/sound references through up to
three levels of timeline/effect files. It does not parse every binary opcode,
resolve all race-specific motion bindings, inspect casting timelines, or prove
live playback. Missing resources produce null sizes. Action IDs come from the
server's historical Feast kit; the client still preserves names and end/hit
references on removed rows, even where Icon=784 and IsPvP=false.


Supply audit: append `--supplies` to the command to verify original box/kit
prefabs, their dependencies and retained statuses. Results for the installed
7.51h2 client are recorded in `supplies-7.51h2.json`. The audit proves resource
presence, not live rendering or historical box HP/expiry behavior.

Job audit: run from the repository root and append `--jobs`. This reads the
shared `resources/feast-actions.json` catalog for BLM/SAM/PLD. The checked-in
`jobs-7.51h2.json` has 70 distinct actions, 139 present end/hit references and
1,301 present dependencies. Tsubame-gaeshi's empty wrapper has no timeline;
its Kaeshi variants have their original assets. The audit also records retained
cast/recast fields and cooldown groups. Icons are restored separately by the HUD.

UI audit: append `--ui` from the repository root to compare current resources with
`target/feast-hud-recovery`, enumerate native menu rows, and verify the historical
icons. The compact `ui-7.51h2.json` records the September 26 findings; generated
reports additionally include node coordinates. This mode emits metadata, not
texture pixels or game binaries. `resolve-ui-names.py` matches unknown recovered
ULD filename hashes against old executable strings without executing that file.
See `docs/src/development/feast-native-assets.md` for findings and integration limits.
