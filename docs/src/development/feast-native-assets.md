# Native Feast UI asset investigation

## September 27 presentation fixes

Medal rail tint and separate clock-colon rendering live in `NativeMatchHeader`.
The atlas fill is grey; its native additive RGB tint now supplies the team color.
This is a rendering adjustment, not a change to medals or the win threshold.

Combat flytext is server-driven. `WhiteMage.lua` records direct damage/heal
recipients during action completion; `server/feast.rs` translates them to native
per-target effect blocks in `AoeEffect8`. The old adapter only described the
selected target's HP difference, losing Assize's enemy effects on self casts.
The typed packet in `core/src/ipc/zone/server/action_result.rs` shares the native
action header, has eight 64-byte target effect blocks, and serializes to636 bytes.
Over-eight-target casts are batched; only the first batch acknowledges the input.
Packet serialization and mixed healing/damage recipient tests cover this path.

Dia's PvP vulnerability (+10% damage taken,15s) already existed in the rules.
Its presentation now uses retained PvP status2035 with caster ownership and
expiry removal. PvE Dia1871 is a different DoT status and must not be substituted.
Native appearance is pending manual verification; no computer use was performed.

September 26, 2026. Read-only comparison of the installed 7.51h2 resources,
the recovered 6.08 executable, and pre-6.1 patch resources. Resource presence
does not establish that the corresponding native controller still works.

## Findings

| Resource | Evidence | Practical use |
| --- | --- | --- |
| `PvPColosseumPartyList4.uld` | Recovered layout, 22,924 bytes; absent from current SqPack | Original four-player Feast party layout, including action and status slots |
| `PvpActionJob.uld` | Current and old versions both available | Native job-action list with existing icon/list components |
| `PvpActionAdditional.uld`, `PvpActionTraits.uld`, `PvpActionQuickChat.uld` | All still present | Original menu families can be reused; presence does not imply old selection rules work |
| `PvpCharacter.uld`, `PvpColosseum.uld`, `PvpColosseumResult.uld` | Old and current versions available | Profile, Colosseum panel and result-screen resources |
| `PVPScreenInformationHotBar.uld` | Current bytes identical to recovered copy | Retained native layout; its runtime purpose/callbacks need tracing |
| `PVPScreenInformation.uld` | Newly identified recovered resource, 20,732 bytes | Candidate historical screen-information layout |
| `PvPRankRatingFunction.uld`, `PvPRankPromotionQualifier.uld` | Newly identified recovered resources | Historical rating/promotion presentation |
| `JobHudBLM0/1`, `JobHudSAM0`, `JobHudPLD0`, `JobHudWHM0` | Current layouts present | Native gauge artwork; current controllers may differ from Feast-era gauges |
| Historical action icons | 88 unique icons, normal and high-resolution textures present | Original WHM/BLM/SAM/PLD and shared action icons |

The earlier recovered Feast header/gauge textures remain available locally.
The graphite header has now been replaced by a native match-header adapter.

## Native in-match medal header (September 26)

`NativeMatchHeader` adds native Atk image, nine-grid and text nodes to the surviving
`PvPMKSHeader` host. The host retains HUD Layout placement and scale. Its CC
requested-update handler is suppressed and its original children are hidden;
the header's native draw runs normally. All added nodes/textures are detached
and released before addon finalization, on leaving the arena, and on unload.
The original children and root dimensions are restored. No ImGui header remains.

Original `pvpcolosseumheader.tex` and `pvpcolosseumgauge.tex` are copied from the
local recovery into the build output's `assets` directory. They are decoded by
Lumina and uploaded as native kernel textures, avoiding nonexistent SqPack paths.
No game archives are modified and proprietary textures are not committed.
The recovered ULD supplies the 920x60 root, 894x104 banner, 340-pixel gauges,
four 72-pixel medal slots per team, font choices and team/score placement.
This is a replacement controller using historical artwork/layout, not loading
the removed executable controller unchanged. Match phase uses an additional
small native text line below the clock.

Director frames supply totals and individual job/medal values. Blue Claws stay
left and red Fangs right, with a star on the local team. Gauges reach full at
the server's 600-medal victory threshold. Active clocks interpolate only fresh
frames; finished clocks freeze and stale data clears the slots and shows dashes.
The HUD Layout entry remains **Crystalline Conflict Progress Gauge** because
the retained host still owns its saved settings.

Reproduce layout inspection with `dotnet run --project tools/feast-assets
--no-restore -- <game/sqpack> --header-layout`. This emits typed node metadata,
including native fonts, atlas parts and component positions.

Release build and 67 HUD checks pass. The user requested no computer use, so
native rendering, moving/scaling, and entry/exit cleanup await manual verification.

Startup regression: the first deployed header used `Assembly.Location`, empty
under Dalamud's memory loader, so constructor failure disabled the entire plugin.
It now uses `IDalamudPluginInterface.AssemblyLocation.DirectoryName`. Header
construction is isolated from director/roster initialization; asset failure cannot
disable the other Feast hooks. `tools/feast-header-loader-tests` reproduces the
memory-loaded assembly and loads both real atlases without invoking game APIs.
Run with the built HUD directory as its single argument. This validates startup
asset decoding, not native GPU upload or rendering.

## Original party list structure

The 6.08 executable supplies the name `PvPColosseumPartyList4`. Its lowercase
filename CRC matches the previously unnamed `uld_ea17456b.uld`. This is stronger
evidence than a guessed filename; the decoded layout also has exactly four rows.

The root is 384 x 330. Widget nodes 6–9 instantiate component 1006 at Y=0,78,156,234.
That component contains the player name, HP/MP-style gauge components, cast
presentation, three component-1002 action icons (nodes 13–15), and ten
component-1001 status icons (nodes 16–25). The icon component includes the original
charge/recast artwork. These positions are layout coordinates before scaling.

All twelve referenced textures survive: IconTextRelease, IconA_ChargeIcon,
IconA_Frame, IconA_Recast, IconA_Recast2, PartyList_GaugeHP, PartyList_Bg,
PartyList_GaugeCast, PartyListTargetBase, PVPHudMove, PVPPartyTarget,
PVPSimulationDisplay (all under `ui/uld`, with `.tex`).

`NativeFeastLayout` now maps this geometry onto the retained native controller.
The current resource is `PvPMKSPartyList5.uld`, used by the player/enemy addons
`PvPMKSPartyList1/3`. Its gauges, cast labels and target highlights share the old
textures. Keeping its node IDs and event handlers preserves native clicks;
blindly redirecting the whole ULD would invalidate its controller bindings.

The adapter removes the CC headers, large panels, fifth row and LB strip; uses
the original four-row spacing, gauge positions, square action icons and native
collision bounds; and adds ten native status-image slots per row. The graphite
scoreboard remains unchanged. HUD Layout's saved positions remain authoritative.
The HUD configuration entries still have the surviving CC widget names.

### Live validation and outstanding work (September 26)

- All eight rows/names, original spacing, square action icons and removed CC
  panels observed in a mixed WHM/PLD/SAM/BLM match. Both ally and enemy clicks
  select the real actor through native callbacks. Distant enemies must be within
  the client's normal targeting range. The server retains all present Feast
  participants across territory 745 instead of culling opposite-gate actors.
- Native cast labels/progress observed for Cure II, Holy Spirit and Glare;
  character names return when casting ends. Blank native CC name rendering is
  avoided with an owned AtkTextNode child; it has no collision/event handlers.
- Ten native status slots per row reuse current Status sheet icon resources.
  Culling icons appeared on all eight rows without the earlier image-loader crash.
- Cooldown numbers, dimming and recovery were observed on Protect/Largesse.
  The recast atlas wedge is implemented but its small-scale appearance needs a
  closer visual check before claiming exact historical reproduction.
- A zero-initialized label build survived clean shutdown; later testing also
  survived full same-duty player reinitialization after a KO. Final regression
  passed: enter, mixed bots, KO, respawn, leave to Gridania. All PvP widgets
  disappeared, the normal hotbar returned, and the client exited normally.

#### Native allocation corrections

`AtkUldManager.CreateAtkImageNode()` does not initialize PartId for a standalone
node. Explicit PartId=0/type/wrap/flags prevents LoadIconTexture from indexing
past the one allocated part. The text factory similarly leaves text-specific
state uninitialized: nonzero garbage in LinkData caused invisible names and a
crash during destruction. Allocate labels with
`IMemorySpace.GetUISpace()->Create<AtkTextNode>()`, which zeros the complete
structure before calling Ctor. Unlink owned children before Destroy and rebuild
the component draw list; do not add event/collision nodes.

#### Respawn lifecycle corrections

The native PvP client fades out and submits ReturnAfterDeath before the server's
10-second KO lock expires. Retain that request once at the original deadline;
rejecting it leaves a permanent black screen. The previous territory-based warp
then selected a public arena (content ID 0), crashing native director creation.
Reinitialize the player in the same Instance with its existing content ID and
match runtime. Live: KO dropped 50 medals, opponent collected them, player
respawned at the correct platform with full HP/MP, all eight rows, score 350-450
and the match clock continuing. Server logs confirm director ID 40022.

This exposed a duplicate InitDirector on the same-duty reload: an extra native
CC director remained after leaving. The connection now reuses the already active
Feast director on respawn; its normal departure termination remains responsible
for closing the HUD. The deployed guard passed the combined KO/respawn/leave
regression: no CC header, party panels or unbinding alert remained in Gridania.
A cosmetic native Fight announcement replayed on respawn; the authoritative
match timer continued correctly. Bot navigation can still stall on partial
navmesh paths, so approaching the opponents was needed for this combat test.

55 world tests and 47 HUD protocol/catalog checks pass; Release HUD and Oodle
world builds pass. These checks do not substitute for native rendering/lifetime
validation. User has authorized exiting crash handlers and autonomous testing.
The earlier crash-handler access timeout is resolved. No game resource files
are patched by these layout adaptations.

## Native action menu

The old panels survive, but the installed `PvPActionSort` subrows point at modern
actions. For example, job 24 begins with 29256, 29259, 29260, 29258, 29261,
29262, 29264, 29266. The old Feast WHM IDs are absent from that list.

The local FFXIVClientStructs SDK exposes `AgentPvpProfile.JobActionIds`,
`JobActionIdCount`, `ActionComboRouteId`, `SelectedRoleActionIndex`, and
`SelectedTab`. This identifies a native producer worth tracing; it does **not**
prove that overwriting the array safely rebuilds the menu. The native menu also
needs correct descriptions, combo wrappers, drag-to-hotbar IDs, action validation
and additional-action selection. Some old Action rows now carry placeholder icon
784 and `IsPvP=false`, so merely restoring the list is insufficient.

October 27: a Feast-scoped ImGui PvP Actions window now exists (see
`tools/feast-hud/README.md`), opened by `/feastactions` and alongside the native
`PvpProfile` addon. Its drag target is the live native hotbar slots, written
through `HotbarSlot.Set` and restored on exit. This is a stopgap; a native
adapter remains preferable once list population is traced.

Prefer a Feast-scoped adapter around the retained agent and panels over a new
ImGui menu. Trace list population and drag events before installing hooks.
Names such as `PvpActionSet` and `PvpProfileAction` appear in the old executable,
but have not been verified as same-named ULDs: agent names and resource names
are not interchangeable.

## Historical hotbar icons

### September 26 live correction pass

The current adapter also replaces the native ActionDetail tooltip arrays (number
32/string 29), including historical description, icon, cast/recast, range/radius
and MP cost. AgentActionDetail clears its temporary array pointers before its
update returns; the adapter obtains the persistent arrays from AtkStage instead.
The native agent must be active, otherwise writing the arrays reopens a stale
tooltip. Full native Protect and Cure II tooltips were verified in-game.

Hotbar recast-group and maximum-charge hooks now use the historical catalog.
Live Glare GCD wedges and Protect's darkened icon, radial wedge and countdown
were observed. Native party rows retain their own cooldown indicators. Original
PartyList_Bg geometry restores the dark background and four-row footprint.

The original adrenaline barrels and Wolf's Heart box render after giving their
layout controller a valid EventObject base row (2008986) and unit radius. The
bound layout selects the actual box model; the base row is only its controller.
Destroying an adrenaline barrel visibly produced the original floating kit.

A full six-minute bot match, two KO/respawns, additional solo matches and repeated
entry/exit completed without the previous stall. This supports the data-before-
network lock-order correction, but no historical hang stack proved its cause.
Logs before the clock correction are target/kawari-server-before-native-clock.log
and its stderr counterpart. Subsequent checks use the normal server logs.

Remaining observed issues: preparation/end-of-match requests can briefly predict
a cast before server rejection; removed-action castbar icons can still show the
placeholder. Bot navigation, historical stat normalization, native PvP menu
population, and full BLM/SAM/PLD visual validation remain unfinished. Box HP and
untouched-kit lifetime remain provisional values, not historical measurements.

`resources/feast-whm-icons.json` records the twenty original WHM/shared icon IDs
from the local 6.08 Action CSV. `FeastSpells.Icon` now supplies them to the existing
Feast-owned native hotbar slots. This includes Protect, Attunement, Largesse,
Rescue, Repose and Empyrean Rain. Existing slot restoration on departure remains
in place. This icon change adds no game-resource patches. The native roster
adapter described above is a separate client-side change, hosted by the existing
Kawari.FeastHud loader.

## Reproduction and evidence

From the repository root:

```powershell
dotnet build tools/feast-assets/Audit.csproj --no-restore
dotnet tools/feast-assets/bin/Debug/net10.0-windows/Audit.dll '<game>/sqpack' --ui > target/feast-ui-audit.json
python tools/feast-assets/resolve-ui-names.py target/feast-director-research/ffxiv_dx11-6.08.exe target/feast-hud-recovery/unidentified
```

`tools/feast-assets/ui-7.51h2.json` is the compact metadata-only evidence: sizes,
hashes, texture references, component/node counts, icon availability and native
menu rows. The generated target report additionally retains node coordinates.
The audit covers 174 candidate resources and 119 referenced texture paths; only
the two already recovered Feast header/gauge textures are missing from the
installed client. Not every resource in the inventory belongs to Feast.

Resolved unknown filename hashes:

- `ea17456b`: `pvpcolosseumpartylist4.uld`
- `8f1241b0`: `pvpscreeninformation.uld`
- `330bbd06`: `pvprankratingfunction.uld`
- `ee23982c`: `pvprankpromotionqualifier.uld`

Three other recovered ULD filenames remain unidentified. Old game binaries and
recovered proprietary UI files remain in ignored local research directories.
