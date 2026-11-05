using Dalamud.Plugin.Services;
using FFXIVClientStructs.FFXIV.Client.Game;

namespace Kawari.FeastHud;

// The client only compares player battalions when the territory's BattalionMode
// is 4 or 6 (mode 1 limits it to battle NPCs). Every live PvP arena uses 4, but
// the retired Crystal Tower Training Grounds row now ships with 1, so the
// server's team battalions are ignored there. Restore the PvP mode on the loaded
// row; hostility itself still comes from the server's spawn battalions.
internal sealed unsafe class NativeBattalionMode(IPluginLog log)
{
    private const uint FeastTerritory = 745;
    private const int BattalionModeOffset = 0x2E;
    private const byte NpcOnly = 1;
    private const byte PvP = 4;
    private byte* patched;

    public void Update()
    {
        var game = GameMain.Instance();
        if (game == null || game->CurrentTerritoryTypeId != FeastTerritory || game->CurrentTerritoryTypeRow == 0) return;
        var mode = (byte*)game->CurrentTerritoryTypeRow + BattalionModeOffset;
        if (*mode != NpcOnly) return;
        *mode = PvP;
        patched = mode;
        log.Information("Feast: territory BattalionMode {Old} -> {New}.", NpcOnly, PvP);
    }

    public void Dispose()
    {
        if (patched != null && *patched == PvP) *patched = NpcOnly;
        patched = null;
    }
}
