# The HUD plugin

`tools/feast-hud` is the **Kawari Feast HUD**, a Dalamud plugin. The Feast's
UI code was removed from the client, but much of its UI survives in other
forms: Crystalline Conflict's header and team lists, the job gauges, the
hotbars, and some textures. The plugin feeds those with Feast data from the
server and changes a few client decisions. It never decides game rules; the
server checks everything again.

## Principles

- **Only in the Feast.** Every change applies only in the arena (territory
  745), or the Wolves' Den for hotbars, and only for the 19 Feast jobs. Hooks
  call the game's original function in every other case.
- **Native UI first.** Data goes into the game's own UI arrays and nodes, so
  HUD Layout positions, scaling and mouse targeting keep working. The PvP
  Actions window is the only ImGui window.
- **Leave no trace.** Every node the plugin adds is removed and every value
  it changed is restored when the addon closes, the player leaves the arena,
  or the plugin unloads. Replaced hotbar slots are backed up to the plugin's
  config before the first change.
- **Pinned to a client build.** Functions ClientStructs does not name are
  found by address for a known executable hash (`NativeClient.cs`); an
  unknown executable disables them instead of guessing.

## Files

`Plugin.cs` creates every part and calls them each frame. Its header lists
them; by folder:

| Folder | Files | Job |
|---|---|---|
| `Director/` | `NativeDirector.cs`, `DirectorFrame.cs`, `NativeClient.cs` | Receive the server's match frames ([protocol](director-protocol.md)); drive the arena UI; client addresses. |
| `Header/` | `NativeMatchHeader.cs`, `MatchHeaderState.cs` | The Feast match header (clock, scores, medals). |
| `PartyList/` | `NativeRoster.cs`, `NativeFeastLayout.cs`, `NativeRosterLabel.cs`, `NativeStatusStrip.cs`, `NativeRecast.cs` | The two team lists. |
| `Gauges/` | `NativeResourceGauges.cs`, `NativeJobGaugeData.cs`, `JobGaugeManagerSync.cs` | Job gauges and the adrenaline bar. |
| `Actions/` | `FeastHotbars.cs`, `FeastJobActions.cs`, `FeastResources.cs`, `NativeActionDetail.cs`, `FeastActionMenu.cs`, `FeastActionsWindow.cs` | Historical actions on the hotbars, tooltips, and the PvP Actions window. |
| `Arena/` | `NativeBattalionMode.cs`, `NativeTargetMedals.cs`, `NativeNamePlateMedals.cs`, `RespawnPrompt.cs` | Hostility, medals on target bars and nameplates, the respawn countdown. |
| (root) | `NativeNodes.cs`, `Chat.cs` | Adding/removing UI nodes; sending chat commands. |

Classes named `Native*` change the game's own UI or memory; `Feast*` classes
are mostly plugin logic. Pure logic (decoding, menus, gauge mapping) is kept
free of game pointers so `tools/feast-hud-tests` can test it.

## The match header and team lists

Inside the arena the duty director makes the client open Crystalline
Conflict's header (`PvPMKSHeader`) and team lists (`PvPMKSPartyList1` for the
player's team, `PvPMKSPartyList3` for the enemy). `NativeDirector` takes them
over:

- **Header.** CC's own refresh would read CC match data the director does not
  have, so `NativeDirector` hooks it (`PvPHeaderRefresh`) and only keeps it
  open. `NativeMatchHeader` hides CC's nodes and builds the Feast header from
  the recovered ULD geometry and textures (`pvpcolosseumheader.tex`,
  `pvpcolosseumgauge.tex`): team names, medal gauges filling towards 600, each
  player's job icon and medals, and a clock that counts down between frames
  (`MatchHeaderState`). Fangs are on the right in red, Claws on the left in
  blue, as in the original, with a star on the player's own team.
- **Team lists.** `NativeRoster` writes each team's four players into the
  lists' number array 117 and string array 105, the same arrays the client
  fills for CC: name, job icon, HP, MP, adrenaline, cast progress and three
  action slots. Writing actor IDs there keeps the client's own click
  targeting. `NativeFeastLayout` moves the CC row nodes into the Feast's
  compact layout and hides CC-only parts; `NativeStatusStrip` adds status
  icons, `NativeRosterLabel` the name or current cast, and `NativeRecast` the
  hotbar-style cooldown sweep over action slots.

## Job gauges and adrenaline

The server sends the player's job gauge in record `F7`. Two things consume it:

- `JobGaugeManagerSync` writes it into the client's `JobGaugeManager`, which
  the client reads for its own checks and highlights.
- `NativeResourceGauges` makes the gauge widgets show it. Each job gauge addon
  (`JobHudWHM0`, ...) reads its data through a virtual function before
  drawing; the plugin hooks that reader on each addon it sees and overwrites
  the result with the Feast values (`NativeJobGaugeData` maps each job's
  bytes onto each layout's fields). It also enables the gauge for jobs whose
  modern gauge would otherwise be hidden.

The limit break bar (`_LimitBreak`, arrays 43/75) shows adrenaline as one bar
labelled "ADRENALINE".

## Hotbars and actions

See [Actions and casting](actions-and-casting.md) for why and how
`FeastHotbars` hooks the client's action functions. The hotbar part:

- In the Wolves' Den and the arena, the job's PvP hotbars get the Feast kit:
  the player's saved layout, or the default (the job's buttons in catalog
  order). The original slots are saved to the plugin config first and put back
  when leaving. Slots the player changed since are left alone.
- Every second the plugin records where Feast actions sit on bars 1–10, so
  rearranging them with the game's own drag and drop is remembered.
- In the Den the actions are shown but cannot be used.

**The PvP Actions window** (`/feastactions`, or opening the PvP Profile in
the Den) replaces the Feast's, whose controller code is gone. It lists the
job's actions by group, lets the player choose two additional actions, and
drag actions onto the native hotbars or a layout grid. The choice is saved in
the plugin config and sent once per arena entry, during preparation, as
`!feast loadout <first> <second>`; the server refuses changes after that.
The rules for grouping, loadouts and hit-testing are in `FeastActionMenu`, so
they are testable.

## Hostility, medals and the respawn countdown

- **`NativeBattalionMode`** changes the arena's BattalionMode from 1 to 4 in
  the client's loaded TerritoryType row (`GameMain.CurrentTerritoryTypeRow`,
  byte `0x2E`) and puts it back on unload. Without it players can never be
  enemies in this territory; see [Hostility](arena.md#hostility-who-is-an-enemy).
- **`NativeTargetMedals`**: in PvP the target bars show a player's job as
  «WHM». The Feast showed their medals there instead, but that code path is
  gone, so the plugin rewrites the text inside «» with the medal count.
- **`NativeNamePlateMedals`** puts « medals » in the nameplate's Free Company
  tag slot, through Dalamud's nameplate API, and asks for a redraw when any
  count changes.
- **`RespawnPrompt`**: a KO'd player must wait ten seconds. The plugin shows
  "OK (7s)" on the native return confirmation and disables it until the
  server says the wait is over. Only the dialog owned by the revive agent is
  touched. The server enforces the wait regardless.

## Building, loading and debugging

```powershell
cd tools/feast-hud
dotnet build -c Release
```

Reload the dev plugin in game (`/xlplugins` → Dev Tools). Most parts can be
reloaded inside the arena, but leaving the duty first is safest. Logs go to
Dalamud's log; every message starts with "Feast". Run the tests with:

```powershell
dotnet run --project tools/feast-hud-tests/Tests.csproj
dotnet run --project tools/feast-header-loader-tests -- tools/feast-hud/bin/Release/net10.0-windows
```

## When the client updates

A game patch can move functions and change structures. In order of
likelihood:

1. **`NativeClient.cs`**: add the new executable's SHA-256 and the five
   addresses. They are found relative to functions ClientStructs knows (the
   agent and addon constructors); the comment in the file explains how the
   7.57 values were derived from 7.51.
2. **ClientStructs/Dalamud**: update `target/dalamud-sdk` to the Dalamud build
   for the new game version and fix compile errors.
3. **Raw offsets** not covered by ClientStructs: `PvPMKSPartyList` + `0x239`
   (team) and + `0x2D8` (action slot components), the JobHud data reader's
   virtual slot (81), the UI array numbers (117/105 team lists, 87 job gauges,
   43/75 limit break, 32/29 tooltips) and BattalionMode's `0x2E`. Each is
   commented where it is used. The header loader test checks the gauge layouts
   against the SDK.
4. **The data patch** in `tools/feast-map` refuses new builds until its
   expected build string is updated and its checks pass.
