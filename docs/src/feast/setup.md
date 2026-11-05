# Setting up

To play or develop the Feast you need four things: a client that knows the
arena exists, the Kawari server from this repository, Dalamud with the Feast
HUD plugin, and a character allowed to use debug commands. Do the steps in
this order the first time.

The supported client is **7.57h1 (game version 2026.10.16.0000.0000)**. The
client data patch and the HUD both check the version and refuse others; see
[Updating to a new client](known-gaps.md#updating-to-a-new-client-version).

## 1. Patch the client's data files

Today's client has an empty row for territory 745 and no Feast duty, so it
cannot load the arena even though the map files are still installed. Two
patches in `tools/feast-map` restore the rows. Both only edit rows inside the
game's existing Excel data pages, keep a byte-for-byte backup, and can be
undone. Run them from the repository root with Python 3.9 or newer, with
the game, its launchers and Kawari closed.

```powershell
$game = "C:\Program Files (x86)\SquareEnix\FINAL FANTASY XIV - A Realm Reborn\game"
# 1a. The territory row (map, weather, place names).
python tools/feast-map/feast_map.py prepare --game $game --output backups/feast-map-7.57h1
powershell -NoProfile -ExecutionPolicy Bypass -File tools/feast-map/Install-FeastMap.ps1
# 1b. The duty: ContentFinderCondition 476, InstanceContent 40022, and the
#     territory's link to them. Needs 1a first.
python tools/feast-map/prepare_feast_duty.py --game $game --output backups/feast-duty-7.57h1
powershell -NoProfile -ExecutionPolicy Bypass -File tools/feast-map/Install-FeastDuty.ps1
```

`prepare` only reads the installation and writes the patch into `backups/`;
the `Install-*.ps1` scripts ask for administrator rights (the game is under
Program Files) and write it. To undo, run the same scripts with `-Restore`,
duty first. A game update or file repair replaces the patched pages, so
re-run both after one.

The server reads the same game files (`filesystem.game_path` in
`config.yaml`), so the patch is all it needs too. Without step 1b, `!feast`
still works in "exploration mode": you can walk the arena and fight, but
there is no duty director, so the HUD's header and team lists stay hidden.

## 2. Run the server

Follow [Local Development](../setup/source.md) for Kawari itself. Then, from
the repository root:

```powershell
cargo run --bin kawari-run
```

This builds and starts every Kawari service. The world server logs Feast
activity at `info` level (`Feast match`, `Feast damage`, ...). The Lua scripts
in `resources/scripts` are read from disk, so script changes apply without a
rebuild: command scripts on next use, the combat engine at the next match.

Practice bots walk the arena with a navigation mesh. Generate it once:

```powershell
cargo run --bin kawari-navimesh -- 745 navimesh/bg/ffxiv/pvp_xx/pvp/l1p2/navimesh/l1p2.nvm
```

The mesh generator leaves out the jail gates, which open and close during
play (`DYNAMIC_SHARED_GROUPS` in `tools/navimesh/src/main.rs`). Without a mesh
the bots walk in straight lines.

## 3. Install Dalamud and the HUD

The HUD is a Dalamud plugin, built against the Dalamud SDK files copied into
`target/dalamud-sdk` (Dalamud.dll, FFXIVClientStructs.dll, Lumina.dll and
friends from the Dalamud build matching the game). Build it with:

```powershell
cd tools/feast-hud
dotnet build -c Release
```

The match header uses two textures from a pre-6.1 client that today's client
no longer has: `pvpcolosseumheader.tex` and `pvpcolosseumgauge.tex`, expected
in `target/feast-hud-recovery/assets/ui/uld/`. They are not distributed. If
they are missing the header is not drawn; everything else still works.

Then register the plugin with your launcher's Dalamud, with the game closed:

- **Astra, first time:** `tools/feast-hud/Setup-Astra.ps1 -ProfileId <guid>
  -AccountId <guid>` copies the matching Dalamud build from XIVLauncher into
  Astra and registers the HUD. The GUIDs are in
  `%LOCALAPPDATA%\State\astrastaterc`.
- **An existing Dalamud config:** `tools/feast-hud/Install.ps1 -ConfigPath
  <dalamud-config.json>` adds the HUD as a dev plugin.

After rebuilding, reload the plugin in game: `/xlplugins` → Dev Tools →
Kawari Feast HUD → reload. Its log is in Dalamud's log
(`%APPDATA%\astra\log\dalamud.log` for Astra); search for `Feast`.

## 4. Play

`!feast` is one of Kawari's [debug commands](../debug_commands.md), so your
character needs the rank those require. In any town or open-world zone:

```text
!feast            queue for the Feast (accept the duty pop)
!feast bots       fill both teams with bots for a solo 4v4
!feast status     clock, scores and your medals
!feast restart    start the next match after a result
!feast leave      leave the arena
!feast den        travel to the Wolves' Den to choose PvP actions
```

Your job must be one of the 19 that existed before 6.1. In the Wolves' Den,
`/feastactions` opens the PvP Actions window, where you choose your two
additional actions and arrange your hotbars. The full list of commands is in
[Commands](commands.md).

To play with others, everyone needs the client patch and the HUD. Party up
and queue with `!feast`; the server splits the party into the two teams once
the match assigns them.
