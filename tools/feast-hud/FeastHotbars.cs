using Dalamud.Hooking;
using Dalamud.Plugin;
using Dalamud.Plugin.Services;
using FFXIVClientStructs.FFXIV.Client.Game;
using FFXIVClientStructs.FFXIV.Client.UI.Misc;
using static FFXIVClientStructs.FFXIV.Client.UI.Misc.RaptureHotbarModule;

namespace Kawari.FeastHud;

public sealed record HotbarBackup(uint Bar, uint Slot, HotbarSlotType Type, uint Id, uint Replacement);
public sealed class FeastHotbarConfig : Dalamud.Configuration.IPluginConfiguration
{
    public int Version { get; set; } = 1;
    public ulong Character { get; set; }
    public List<HotbarBackup> Slots { get; set; } = [];
}

// Temporary live PvP slots. Casts still travel through native ActionRequest,
// ActionCast and ActionResult; the server's Feast engine validates each request.
internal sealed unsafe class FeastHotbars : IDisposable
{
    private static readonly uint[] Actions = [16533, 16532, 120, 135, 124, 16531, 16534, 16535,
        3570, 3571, 16536, 7568, 3, 140, 7432, 25861, 16560, 7571, 7433, 208];
    // UseAction resolves the spell, adjusts it, then calls GetActionStatus with
    // that resolved ID. Recognize both stages, not just the original hotbar ID.
    private static readonly HashSet<uint> NativeActions = [..Actions, ..Actions.Select(FeastSpells.Resolve),
        119, 127, 3568, 7431, 25859, 121, 132, 133, 37010];
    private readonly IDalamudPluginInterface pi;
    private readonly IClientState client;
    private readonly IPlayerState player;
    private readonly IObjectTable objects;
    private readonly Hook<ActionStatus> status;
    private readonly Hook<SpellId> spell;
    private readonly FeastHotbarConfig config;
    private bool applied;
    private delegate uint SpellId(ActionType type, uint id);
    private delegate uint ActionStatus(ActionManager* manager, ActionType type, uint id, ulong target,
        [System.Runtime.InteropServices.MarshalAs(System.Runtime.InteropServices.UnmanagedType.I1)] bool recast,
        [System.Runtime.InteropServices.MarshalAs(System.Runtime.InteropServices.UnmanagedType.I1)] bool casting, uint* extra);
    private bool Eligible => client.IsLoggedIn && client.TerritoryType == 745 && player.ClassJob.RowId == 24;
    public FeastHotbars(IDalamudPluginInterface pi, IClientState client, IPlayerState player, IObjectTable objects, IGameInteropProvider interop)
    {
        this.pi = pi; this.client = client; this.player = player; this.objects = objects;
        config = pi.GetPluginConfig() as FeastHotbarConfig ?? new();
        status = interop.HookFromAddress<ActionStatus>((nint)ActionManager.MemberFunctionPointers.GetActionStatus, GetStatus);
        try { spell = interop.HookFromAddress<SpellId>((nint)ActionManager.MemberFunctionPointers.GetSpellIdForAction, GetSpell); }
        catch { status.Dispose(); throw; }
        status.Enable(); spell.Enable();
    }
    private uint GetSpell(ActionType type, uint id)
        => Eligible && type == ActionType.Action && NativeActions.Contains(id)
            ? FeastSpells.Resolve(id) : spell.Original(type, id);
    private uint GetStatus(ActionManager* manager, ActionType type, uint id, ulong target, bool recast, bool casting, uint* extra)
    {
        // Retail's PvE/PvP, lily and unlock gates describe a different ruleset.
        // Never relax checks for another territory, job, or unimplemented action.
        if (Eligible && type == ActionType.Action && NativeActions.Contains(id))
        {
            if (extra != null) *extra = 0;
            // Retain native timing gates and their normal status codes. Returning
            // success during these gates starts requests the server must cancel.
            // Honor the flags: UseAction also probes without these gates to queue.
            if (casting && objects.LocalPlayer is { IsCasting: true }) return 0x244;
            if (recast && manager->IsRecastTimerActive(type, id)) return 0x246;
            return 0;
        }
        return status.Original(manager, type, id, target, recast, casting, extra);
    }
    public void Update()
    {
        var module = RaptureHotbarModule.Instance();
        if (module == null || !module->ModuleReady) return;
        if (!Eligible || !module->PvPHotbarsActive || module->ActiveHotbarClassJobId != 24)
        {
            if (applied)
            {
                // Once PvE bars have loaded, discard recovery without touching them.
                if (module->PvPHotbarsActive && module->ActiveHotbarClassJobId == 24) Restore(module);
                applied = false; config.Slots.Clear(); pi.SavePluginConfig(config);
            }
            return;
        }
        if (applied) return;
        if (config.Character == player.ContentId && config.Slots.Count != 0) Restore(module);
        config.Character = player.ContentId; config.Slots.Clear();
        for (uint i = 0; i < Actions.Length; i++)
        {
            var slot = module->GetSlotById(i / 12, i % 12);
            config.Slots.Add(new(i / 12, i % 12, slot->CommandType, slot->CommandId, Actions[i]));
        }
        pi.SavePluginConfig(config);
        foreach (var saved in config.Slots) module->GetSlotById(saved.Bar, saved.Slot)->Set(HotbarSlotType.Action, saved.Replacement);
        applied = true;
    }
    private void Restore(RaptureHotbarModule* module)
    {
        if (config.Character != player.ContentId) return;
        foreach (var saved in config.Slots)
        {
            var slot = module->GetSlotById(saved.Bar, saved.Slot);
            if (slot != null && slot->CommandType == HotbarSlotType.Action && slot->CommandId == saved.Replacement)
                slot->Set(saved.Type, saved.Id);
        }
    }
    public void Dispose()
    {
        spell.Dispose(); status.Dispose();
        var module = RaptureHotbarModule.Instance();
        if (module != null && module->ModuleReady && module->PvPHotbarsActive && module->ActiveHotbarClassJobId == 24)
        { Restore(module); config.Slots.Clear(); pi.SavePluginConfig(config); }
    }
}
