# Crystal Tower Training Grounds exploration

This prototype restores **territory 745** as an ordinary exploration area. It adds
`!feast` to enter and `!feast leave` to return to Limsa. Accepting Return after death
in territory 745 respawns you at the arena entry point with full HP/MP. The client
may still name your homepoint in the confirmation; your saved homepoint is unchanged.
An optional [Feast White Mage combat prototype](../../docs/src/development/feast-white-mage.md)
is available through native hotbar actions. The server now implements a
[small-team match prototype](../../docs/src/development/feast-match.md) with a
countdown, medals and results. Matchmaking and the historical client HUD are not restored.

The supported Windows client build is **2026.06.18.0000.0000 (7.51h2)**. The arena's
`bg/ffxiv/pvp_xx/pvp/l1p2/level/l1p2.lvb` and its main assets still exist in this
build, but the territory row is blank. The ZoneInit packet supplies a territory ID,
so **the client and server both need the restored definition**. A server-only
teleport command cannot supply the missing client map path.

## Setup

Run from the repository root with Python 3.9 or newer. The tool uses only the Python
standard library. Substitute your Python executable and game path where needed.

1. Prepare the patch. This reads the installed files and writes the proposed change
   and original-byte backup; it does not change the installation.

   ```powershell
   python tools/feast-map/feast_map.py prepare --game "C:\Program Files (x86)\SquareEnix\FINAL FANTASY XIV - A Realm Reborn\game" --output backups/feast-map-7.51h2
   ```

2. Close FFXIV, its launchers, and Kawari. Apply the prepared patch:

   ```powershell
   powershell -NoProfile -ExecutionPolicy Bypass -File tools/feast-map/Install-FeastMap.ps1
   ```

   The helper requests normal Windows UAC elevation for the installation under
   Program Files. It can find the bundled Codex Python runtime, or use `python` on
   PATH. Pass `-Python C:\path\to\python.exe` to choose another interpreter.
   The result is recorded in `backups/feast-map-7.51h2/install.log`.

3. Start the server from **this checkout**, using the same `filesystem.game_path`,
   then start the client and log in with a debug GM character. In ordinary chat:

   ```text
   !feast
   !feast leave
   ```

   The entry position is `(-37, 1, 0)`, just above a PC arrival in the surviving
   `planmap.lgb`. Repeating `!feast` while inside returns to that position. Leaving
   uses the same Limsa coordinates as the existing `!skipintro` command.

The chat command is Lua, so it requires no server binary rebuild. A separately
downloaded Kawari distribution will not automatically load this checkout's scripts.
Each participating client needs the metadata patch. When client and server share
the configured game installation, the native patch supplies the definition to both;
no `additional_search_paths` change is needed.

## Undo

Leave the arena, close FFXIV/launchers/Kawari, and run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools/feast-map/Install-FeastMap.ps1 -Restore
```

Keep the backup folder until restoring. The tool refuses a stale backup after a
client update or an unrelated modification. A normal game repair/update may also
replace this local modification. Recheck compatibility before reapplying it.

For a writable installation without elevation, the equivalent commands are
`python tools/feast-map/feast_map.py install --output backups/feast-map-7.51h2`
and the same command with `restore` in place of `install`.

## Scope and checks

The preparation step edits only row 745 in the **current** TerritoryType page.
It restores `Name`, `Bg`, place/map IDs, weather, fixed time, and basic access flags,
using OpenWorld intended use, no duty link, and no PvP flag. It does not restore the
other three Feast rows or old duty definitions. It does not replace the current
map visuals with recovered pre-6.1 models or materials.

Every other EXD row is checked byte-for-byte. The new page must fit within its
existing SqPack allocation; neither archive indexes nor file lengths are changed.
The original allocation is backed up with SHA-256 hashes before installation.
Install/restore recheck the build, schema, index, and source data, refuse conflicting
changes, and verify the bytes after writing. On a write/verification exception the
tool attempts to roll back that write. The loose page in `overlay/exd/` can also be
used as a server overlay or packaged with a compatible client mod loader.

Validation performed on the prepared page: Physis parsed the restored row, confirmed
**1,289 other territory rows unchanged**, and parsed all six current arena layout
files and the collision list. This is not an in-game loading test; arrival, lighting,
exits, and multiplayer behavior still need to be exercised with a connected client.

Run the synthetic archive tests without touching a game installation:

```powershell
python -m unittest discover -s tools/feast-map -v
```

Format references: the checkout's cached [Physis](https://github.com/redstrate/physis)
revision `2c4a9a3` and [Icarus](https://github.com/redstrate/Icarus) TerritoryType schema.
