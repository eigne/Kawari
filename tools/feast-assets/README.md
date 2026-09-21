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
