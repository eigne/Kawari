# Kawari Feast client plugin

Dalamud API 15 / game build `2026.06.18.0000.0000`. Built against the user's
matching Dalamud 15.0.2.3 runtime. Requires the Kawari Feast WHM server rules.

## Use

Launch through Astra with Dalamud enabled, log in as White Mage and enter `!feast`.
The plugin enables Feast rules and temporarily fills hotbar 1 and the first eight
slots of hotbar 2. Click buttons or use their existing keybinds. Select an enemy
for attacks or a teammate/yourself for heals; deselect to heal yourself.

`/kfeast status` prints the mapping and requests server gauge/status information.
`/kfeast restore` restores the layout, disables Feast rules, and pauses automatic
activation. `/kfeast on` resumes automatic activation. Leaving territory 745,
changing jobs, logging out, or unloading the plugin removes the temporary layout.

| Bar | Slots, in order |
| --- | --- |
| 1 | Glare, Dia, Cure, Cure II, Medica, Solace, Rapture, Misery, Tetragrammaton, Assize, Temperance, Purify |
| 2 (1–8) | Bolt, Medical Kit, Protect, Largesse, Repose, Rescue, Attunement, Empyrean Rain |

The default additional abilities are Protect/Largesse. The other additional
buttons require choosing a loadout with `!whm loadout repose rescue` (or another
pair). Rain still requires `!whm adrenaline` to fill the prototype test gauge.

Icons/tooltips use current-client placeholders: Esuna = Purify, Sprint = Bolt,
Benediction = Medical Kit, Divine Benison = Protect, Aquaveil = Largesse,
Plenary Indulgence = Attunement, Pulse of Life = Empyrean Rain.

## How it works and limits

The plugin intercepts `RaptureHotbarModule.ExecuteSlot` only for its owned slots.
It sends the existing `!whm` commands internally, so no typing or user macros are
needed for individual casts. The server remains authoritative. This avoids modern
PvE lily/MP/unlock/recast checks preventing a Feast ability from reaching the
server. It does not convert generic native ActionRequest packets into Feast casts.

The live slot `Set` API does not save the replacements to HOTBAR.DAT. Before
changing any slot, the plugin saves a recovery record in its Dalamud config. It
restores matching owned slots for the same character/job; it does not restore
an old job's buttons over a newly loaded job. Avoid rearranging these temporary
slots. The plugin does not force hidden bars visible, alter keybindings, or fill
controller cross hotbars. Native cooldown shading, tooltips, gauges and action
queueing do not represent the historical rules yet; server notices explain gates.

The server sends current-client cast and action-result packets at the historical
cast start/completion times. Primary-target hit/heal presentation reflects actual
HP changes. AoE secondary-target floating numbers and historical exact VFX remain
unfinished. These are presentation packets, not a second application of damage.

For the local player's casted spells, the plugin hooks
`PacketDispatcher.HandleActorCastPacket` and plays the action's
`AnimationStart -> ActionCastTimeline.Name` through `TimelineSequencer.PlayTimeline`. The
command transport bypasses normal `UseAction` startup, so the bar and completion
packet alone were not starting the local casting pose. Movement, the cast deadline,
and plugin teardown clear only the owned casting timeline;
they never reset a different timeline installed by the completion packet. The
hook covers the four casted Feast spells (Glare, Cure, Cure II, Medica) and does
not execute a native spell or change the server's damage/healing/cast rules.
The earlier `OpenCastBar` hook did not fix the user's missing preparation pose.
The packet hook logs accepted action IDs, resolved cast timelines, and the active
base/upper-body timeline IDs to Astra's `dalamud.log` for live diagnosis.

After updating the DLL, disable/re-enable Kawari Feast in Dalamud's plugin list.
Verify each cast shows its preparation pose, moving midway stops it without a
release, and a completed cast still plays its release when moving afterward.

## Build and install

Install .NET 10 SDK. Put reference DLLs from a matching Dalamud runtime in
`target/dalamud-sdk`, or supply `-p:DalamudLibPath=C:/path/to/Dalamud`.

```powershell
dotnet build tools/feast-plugin -c Release --configfile tools/feast-plugin/NuGet.Config
```

`Install-Astra.ps1` is a local setup helper whose default IDs identify this
workspace's existing private-server profile/account. With Astra closed, it copies
the matching installed XIVLauncher runtime/assets into Astra, registers the built
DLL as a developer plugin, backs up `astrastaterc`, and selects Astra's local
Dalamud channel. The local channel pins the compatible version; do not use it with
a different game build. Existing Astra Dalamud configuration needs a manual merge.

To install manually: enable Dalamud in Astra's profile settings, then add the
absolute built `Kawari.Feast.dll` path to Dalamud's developer plugin load locations
and enable startup loading. Restore/pause with `/kfeast restore` before disabling
the plugin. To disable injection, turn off Enable Dalamud in Astra profile settings.

References: [Astra launcher](https://github.com/redstrate/Astra),
[client hotbar API](https://github.com/aers/FFXIVClientStructs/blob/main/FFXIVClientStructs/FFXIV/Client/UI/Misc/RaptureHotbarModule.cs).

Validation: plugin builds without warnings against the matching local runtime;
server tests cover cast timing, cancelled casts, packet selection and authoritative
HP changes. Automatic loading, hotbar interaction, restoration and VFX still need
live confirmation on the user's client.
