using Dalamud.Hooking;
using Dalamud.Plugin;
using Dalamud.Plugin.Services;
using FFXIVClientStructs.FFXIV.Client.Game;
using FFXIVClientStructs.FFXIV.Client.UI.Misc;
using FFXIVClientStructs.FFXIV.Client.UI;
using FFXIVClientStructs.FFXIV.Component.GUI;
using static FFXIVClientStructs.FFXIV.Client.UI.Misc.RaptureHotbarModule;

namespace Kawari.FeastHud;

public sealed record HotbarBackup(uint Bar, uint Slot, HotbarSlotType Type, uint Id, uint Replacement);
public sealed class FeastHotbarConfig : Dalamud.Configuration.IPluginConfiguration
{
    public int Version { get; set; } = 1;
    public ulong Character { get; set; }
    public uint ClassJob { get; set; } = 24;
    // Original contents of the PvP hotbar slots we replaced, restored on leaving.
    public List<HotbarBackup> Slots { get; set; } = [];
    // Per-job Feast hotbar layout from the PvP Actions window; empty means the default kit.
    public Dictionary<uint, List<LayoutSlot>> Layouts { get; set; } = [];
    // Per-job additional actions chosen in the PvP Actions window.
    public Dictionary<uint, List<string>> Loadouts { get; set; } = [];
}

// Makes the historical Feast actions usable from the normal hotbars.
//
// In the Wolves' Den and the arena, the job's PvP hotbars are filled with its
// Feast kit (the historical Action sheet rows from feast-actions.json); the
// original slots are backed up and restored on leaving. Pressing a button
// goes through the game's own UseAction, so the client sends a normal
// ActionRequest, shows its cast bar and starts its recast, and the server's
// combat engine decides what happens.
//
// Those Action rows were blanked when Feast was removed (icon 784, no cast or
// recast data, not "PvP"), so the client would refuse or mistime them. The
// hooks below answer the client's questions about these actions from the
// catalog and the server's live hotbar records instead:
//   GetActionStatus     can it be used now (resources from the director)
//   GetSpellIdForAction what it becomes now (combos, procs)
//   GetAdjustedCastTime / GetAdjustedRecastTime, GetMaxCharges
//   GetRecastGroup / GetAdditionalRecastGroup, so the recast sweeps show
//   GetIconIdForSlot, LoadIconId, UpdateHotbarSlot: the historical icons
//   UseActionLocation   ground-targeted actions, forwarded as a chat command
// Every hook falls through to the original outside the Feast or for other actions.
internal sealed unsafe class FeastHotbars : IDisposable
{
    private readonly IDalamudPluginInterface pi;
    private readonly IClientState client;
    private readonly IPlayerState player;
    private readonly IObjectTable objects;
    private readonly Hook<ActionStatus> status;
    private readonly Hook<SpellId> spell;
    private readonly Hook<CastTime>? castTime;
    private readonly Hook<RecastTime>? recastTime;
    private readonly Hook<UseLocation>? location;
    private readonly Hook<RecastGroup>? recastGroup;
    private readonly Hook<RecastGroup>? additionalGroup;
    private readonly Hook<IconId>? iconId;
    private readonly Hook<MaxCharges>? maxCharges;
    private readonly Hook<LoadIcon>? loadIcon;
    private readonly Hook<UpdateSlot>? updateSlot;
    private delegate void UpdateSlot(AddonActionBarBase* addon, ActionBarSlot* slot, NumberArrayData* numbers, StringArrayData* strings, int numberIndex, int stringIndex);
    [return: System.Runtime.InteropServices.MarshalAs(System.Runtime.InteropServices.UnmanagedType.I1)]
    private delegate bool LoadIcon(HotbarSlot* slot);
    private delegate ushort MaxCharges(uint id, uint level);
    private delegate int RecastGroup(ActionManager* manager, int type, uint id);
    private delegate int IconId(HotbarSlot* slot, HotbarSlotType type, uint id);
    [return: System.Runtime.InteropServices.MarshalAs(System.Runtime.InteropServices.UnmanagedType.I1)]
    private delegate bool UseLocation(ActionManager* manager, ActionType type, uint id, ulong target, System.Numerics.Vector3* position, uint extra, byte route);
    private delegate int CastTime(ActionType type, uint id, [System.Runtime.InteropServices.MarshalAs(System.Runtime.InteropServices.UnmanagedType.I1)] bool procs, ActionManager.CastTimeProc* proc);
    private delegate int RecastTime(ActionType type, uint id, [System.Runtime.InteropServices.MarshalAs(System.Runtime.InteropServices.UnmanagedType.I1)] bool mechanics);
    private delegate uint SpellId(ActionType type, uint id);
    private delegate uint ActionStatus(ActionManager* manager, ActionType type, uint id, ulong target,
        [System.Runtime.InteropServices.MarshalAs(System.Runtime.InteropServices.UnmanagedType.I1)] bool recast,
        [System.Runtime.InteropServices.MarshalAs(System.Runtime.InteropServices.UnmanagedType.I1)] bool casting, uint* extra);
    // The server's hotbar records (F6): what each button currently becomes,
    // with its current cast and recast. Empty when no frame is fresh.
    private IReadOnlyList<HotbarAction> live = [];
    private DirectorFrame? resources;
    private readonly FeastHotbarConfig config;
    private bool applied;
    private long nextScan;
    public bool Active => applied && Eligible;
    public FeastHotbarConfig Config => config;
    public void Save() => pi.SavePluginConfig(config);
    private bool Eligible => client.IsLoggedIn && FeastActionMenu.UsesFeastHotbars(client.TerritoryType) && FeastJobActions.Supports(player.ClassJob.RowId);
    // In the Wolves' Den the layout is shown but the native checks keep it unusable.
    private bool Castable => Eligible && FeastActionMenu.CanCast(client.TerritoryType);
    // Only the current job's Feast actions are changed; everything else is native.
    private bool Allowed(uint id) => FeastJobActions.Contains(player.ClassJob.RowId, id);
    private HotbarAction? Live(uint id) => live.FirstOrDefault(a => a.BaseId == id);

    public FeastHotbars(IDalamudPluginInterface pi, IClientState client, IPlayerState player, IObjectTable objects, IGameInteropProvider interop)
    {
        this.pi = pi; this.client = client; this.player = player; this.objects = objects;
        config = pi.GetPluginConfig() as FeastHotbarConfig ?? new();
        status = interop.HookFromAddress<ActionStatus>((nint)ActionManager.MemberFunctionPointers.GetActionStatus, GetStatus);
        try { spell = interop.HookFromAddress<SpellId>((nint)ActionManager.MemberFunctionPointers.GetSpellIdForAction, GetSpell); }
        catch { status.Dispose(); throw; }
        try
        {
            castTime = interop.HookFromAddress<CastTime>((nint)ActionManager.MemberFunctionPointers.GetAdjustedCastTime, GetCastTime);
            recastTime = interop.HookFromAddress<RecastTime>((nint)ActionManager.MemberFunctionPointers.GetAdjustedRecastTime, GetRecastTime);
            location = interop.HookFromAddress<UseLocation>((nint)ActionManager.MemberFunctionPointers.UseActionLocation, UseGround);
            recastGroup = interop.HookFromAddress<RecastGroup>((nint)ActionManager.MemberFunctionPointers.GetRecastGroup, GetGroup);
            additionalGroup = interop.HookFromAddress<RecastGroup>((nint)ActionManager.MemberFunctionPointers.GetAdditionalRecastGroup, GetAdditionalGroup);
            iconId = interop.HookFromAddress<IconId>((nint)HotbarSlot.MemberFunctionPointers.GetIconIdForSlot, GetIcon);
            maxCharges = interop.HookFromAddress<MaxCharges>((nint)ActionManager.MemberFunctionPointers.GetMaxCharges, GetCharges);
            loadIcon = interop.HookFromAddress<LoadIcon>((nint)HotbarSlot.MemberFunctionPointers.LoadIconId, LoadHistoricalIcon);
            updateSlot = interop.HookFromAddress<UpdateSlot>((nint)AddonActionBarBase.MemberFunctionPointers.UpdateHotbarSlot, UpdateNativeSlot);
            status.Enable(); spell.Enable(); castTime.Enable(); recastTime.Enable(); location.Enable();
            recastGroup.Enable(); additionalGroup.Enable(); iconId.Enable();
            maxCharges.Enable(); loadIcon.Enable(); updateSlot.Enable();
        }
        catch { DisposeHooks(); throw; }
    }

    // Hotbar addons cache their icon; reload the historical one after each native refresh.
    private void UpdateNativeSlot(AddonActionBarBase* addon, ActionBarSlot* slot, NumberArrayData* numbers, StringArrayData* strings, int numberIndex, int stringIndex)
    {
        updateSlot!.Original(addon, slot, numbers, strings, numberIndex, stringIndex);
        if (!Eligible || slot == null || slot->ComponentDragDrop == null) return;
        uint actionId = (uint)slot->ActionId;
        var saved = config.Slots.FirstOrDefault(s => s.Bar == slot->HotbarId && s.Replacement == actionId);
        if (saved == null || !Allowed(actionId)) return;
        var module = RaptureHotbarModule.Instance();
        if (module == null) return;
        var command = module->GetSlotById(saved.Bar, saved.Slot);
        if (command == null || command->CommandType != HotbarSlotType.Action || command->CommandId != actionId) return;
        var icon = slot->ComponentDragDrop->AtkComponentIcon;
        if (icon == null) return;
        uint historical = (uint)GetIcon(null, HotbarSlotType.Action, actionId);
        if (icon->IconId != historical) icon->LoadIcon(historical);
    }
    private bool LoadHistoricalIcon(HotbarSlot* slot)
    {
        var result = loadIcon!.Original(slot);
        if (Eligible && slot != null && slot->CommandType == HotbarSlotType.Action && Allowed(slot->CommandId))
        {
            slot->IconId = (uint)GetIcon(slot, HotbarSlotType.Action, slot->CommandId);
            return false;
        }
        return result;
    }
    private ushort GetCharges(uint id, uint level)
        => Eligible && Allowed(id) && FeastJobActions.Find(id) is { } a ? (ushort)Math.Max(1, a.Charges) : maxCharges!.Original(id, level);
    private int GetGroup(ActionManager* manager, int type, uint id)
        => Eligible && type == (int)ActionType.Action && Allowed(id) && FeastJobActions.Find(id) is { } a
            ? a.Group : recastGroup!.Original(manager,type,id);
    private int GetAdditionalGroup(ActionManager* manager, int type, uint id)
        => Eligible && type == (int)ActionType.Action && Allowed(id) && FeastJobActions.Find(id) is { } a
            ? a.Secondary : additionalGroup!.Original(manager,type,id);
    // The icon of what the button currently becomes (e.g. the next combo step).
    private int GetIcon(HotbarSlot* slot, HotbarSlotType type, uint id)
    {
        if (Eligible && type == HotbarSlotType.Action && Allowed(id)
            && FeastJobActions.Find(Live(id)?.ResolvedId ?? id) is { } action)
            return action.Icon;
        return iconId!.Original(slot,type,id);
    }
    // The client casts and animates the resolved action, as the server does.
    private uint GetSpell(ActionType type, uint id)
        => Castable && type == ActionType.Action && Allowed(id) ? Live(id)?.ResolvedId ?? id : spell.Original(type, id);
    // Ground-targeted actions: the location packet is not implemented on the
    // server, so the chosen point is sent as `!feast ground <id> <x> <y> <z>`.
    private bool UseGround(ActionManager* manager, ActionType type, uint id, ulong target, System.Numerics.Vector3* position, uint extra, byte route)
    {
        if (!Castable || type != ActionType.Action || !Allowed(id) || !FeastJobActions.IsGroundTargeted(id) || position == null)
            return location!.Original(manager,type,id,target,position,extra,route);
        if (objects.LocalPlayer is { IsCasting: true }) return false;
        var p = *position;
        if (!float.IsFinite(p.X) || !float.IsFinite(p.Y) || !float.IsFinite(p.Z)) return false;
        Chat.Send(FormattableString.Invariant($"!feast ground {id} {p.X:R} {p.Y:R} {p.Z:R}"));
        return true;
    }
    private int GetCastTime(ActionType type, uint id, bool procs, ActionManager.CastTimeProc* proc)
    {
        if (Eligible && type == ActionType.Action && Allowed(id))
        {
            if (proc != null) *proc = default;
            return Live(id)?.CastMs ?? (int)((FeastJobActions.Find(id)?.Cast ?? 0) * 1000);
        }
        return castTime!.Original(type,id,procs,proc);
    }
    private int GetRecastTime(ActionType type, uint id, bool mechanics)
        => Eligible && type == ActionType.Action && Allowed(id)
            ? Live(id)?.RecastMs ?? (int)((FeastJobActions.Find(id)?.Recast ?? 0) * 1000)
            : recastTime!.Original(type,id,mechanics);
    private uint GetStatus(ActionManager* manager, ActionType type, uint id, ulong target, bool recast, bool casting, uint* extra)
    {
        // Retail's PvE/PvP, unlock and resource gates describe a different ruleset.
        // Never relax checks for another territory, job, or unimplemented action.
        if (Castable && type == ActionType.Action && Allowed(id))
        {
            if (extra != null) *extra = 0;
            if (!FeastResources.CanUse(id, resources, objects.LocalPlayer?.EntityId ?? 0)) return 0x246;
            // Retain native timing gates and their normal status codes. Returning
            // success during these gates starts requests the server must cancel.
            // Honor the flags: UseAction also probes without these gates to queue.
            if (casting && objects.LocalPlayer is { IsCasting: true }) return 0x244;
            if (recast && !RecastReady(manager, type, id)) return 0x246;
            return 0;
        }
        return status.Original(manager, type, id, target, recast, casting, extra);
    }
    // IsRecastTimerActive ignores charges: it reports a running timer after the first of
    // several charges is spent. Read the charged group directly; the GCD (group 57) is the
    // only additional group the server enforces, so it stays a hard gate.
    private bool RecastReady(ActionManager* manager, ActionType type, uint id)
    {
        if (!manager->IsRecastTimerActive(type, id)) return true;
        if (FeastJobActions.Find(id) is not { Charges: > 1 } action) return false;
        var gcd = action.Secondary == 57 ? manager->GetRecastGroupDetail(57) : null;
        if (gcd != null && !FeastJobActions.ChargeReady(1, gcd->IsActive, gcd->Elapsed, gcd->Total)) return false;
        var detail = manager->GetRecastGroupDetail(action.Group);
        return detail == null || FeastJobActions.ChargeReady(action.Charges, detail->IsActive, detail->Elapsed, detail->Total);
    }

    // Called every frame with the latest fresh director frame (or null).
    public void Update(DirectorFrame? frame)
    {
        resources = Eligible ? frame : null;
        live = Eligible ? frame?.Hotbar ?? [] : [];
        if (Eligible && frame?.Gauge is { } gauge && gauge.Job == player.ClassJob.RowId)
            JobGaugeManagerSync.Apply(gauge);
        var module = RaptureHotbarModule.Instance();
        if (module == null || !module->ModuleReady) return;
        if (applied && config.ClassJob != module->ActiveHotbarClassJobId)
        { applied = false; config.Slots.Clear(); pi.SavePluginConfig(config); }
        if (!Eligible || !module->PvPHotbarsActive || module->ActiveHotbarClassJobId != player.ClassJob.RowId)
        {
            if (applied)
            {
                // Once PvE bars have loaded, discard recovery without touching them.
                if (module->PvPHotbarsActive && module->ActiveHotbarClassJobId == config.ClassJob) Restore(module);
                applied = false; config.Slots.Clear(); pi.SavePluginConfig(config);
            }
            return;
        }
        if (applied)
        {
            UpdateIcons(module);
            if (Environment.TickCount64 >= nextScan) { nextScan = Environment.TickCount64 + 1000; RecordLayout(module); }
            return;
        }
        Apply(module);
    }

    // Fills the PvP hotbars with the saved layout, or the default kit, after
    // saving what was there. The backup is written before the first change.
    private void Apply(RaptureHotbarModule* module)
    {
        if (config.Character == player.ContentId && config.Slots.Count != 0) Restore(module);
        config.Character = player.ContentId; config.ClassJob = player.ClassJob.RowId; config.Slots.Clear();
        var layout = config.Layouts.GetValueOrDefault(player.ClassJob.RowId)?.Where(s => s.Bar < 10 && s.Slot < 12 && Allowed(s.Id)).ToList();
        if (layout is not { Count: > 0 })
            layout = FeastActionMenu.MenuIds(player.ClassJob.RowId).Select((id, i) => new LayoutSlot((uint)i / 12, (uint)i % 12, id)).ToList();
        foreach (var placed in layout)
        {
            var slot = module->GetSlotById(placed.Bar, placed.Slot);
            config.Slots.Add(new(placed.Bar, placed.Slot, slot->CommandType, slot->CommandId, placed.Id));
        }
        pi.SavePluginConfig(config);
        // Set() changes live slots only; SetAndSaveSlot would overwrite HOTBAR.DAT.
        foreach (var saved in config.Slots) module->GetSlotById(saved.Bar, saved.Slot)->Set(HotbarSlotType.Action, saved.Replacement);
        applied = true;
        UpdateIcons(module);
    }
    private void UpdateIcons(RaptureHotbarModule* module)
    {
        foreach (var saved in config.Slots)
        {
            var slot = module->GetSlotById(saved.Bar,saved.Slot);
            if (slot == null || slot->CommandType != HotbarSlotType.Action || slot->CommandId != saved.Replacement) continue;
            if (FeastJobActions.Find(saved.Replacement) is { } metadata)
                slot->PopUpHelp.SetString(metadata.Name + slot->PopUpKeybindHintString);
            if (FeastJobActions.Find(Live(saved.Replacement)?.ResolvedId ?? saved.Replacement) is { } action)
                slot->IconId = (uint)action.Icon;
        }
    }
    // Places a menu action on a live PvP hotbar slot. The slot's original
    // content is backed up once and restored when the player leaves the arena.
    public bool Place(uint bar, uint slotId, uint id)
    {
        var module = RaptureHotbarModule.Instance();
        if (!Active || module == null || bar >= 10 || slotId >= 12 || !Allowed(id)) return false;
        var slot = module->GetSlotById(bar, slotId);
        if (slot == null) return false;
        int index = config.Slots.FindIndex(s => s.Bar == bar && s.Slot == slotId);
        if (index < 0) config.Slots.Add(new(bar, slotId, slot->CommandType, slot->CommandId, id));
        else config.Slots[index] = config.Slots[index] with { Replacement = id };
        slot->Set(HotbarSlotType.Action, id);
        RecordLayout(module);
        UpdateIcons(module);
        return true;
    }
    // Restores the default kit for the current job on the next update.
    public void ResetLayout(uint job)
    {
        config.Layouts.Remove(job);
        var module = RaptureHotbarModule.Instance();
        if (Active && module != null && config.ClassJob == job)
        { Restore(module); config.Slots.Clear(); applied = false; }
        pi.SavePluginConfig(config);
    }
    // Remembers where Feast actions sit, including native moves and removals.
    private void RecordLayout(RaptureHotbarModule* module)
    {
        var current = new List<LayoutSlot>();
        for (uint bar = 0; bar < 10; bar++)
            for (uint i = 0; i < 12; i++)
            {
                var slot = module->GetSlotById(bar, i);
                if (slot != null && slot->CommandType == HotbarSlotType.Action && Allowed(slot->CommandId)) current.Add(new(bar, i, slot->CommandId));
            }
        var saved = config.Layouts.GetValueOrDefault(config.ClassJob);
        if (saved != null && saved.SequenceEqual(current)) return;
        config.Layouts[config.ClassJob] = current;
        pi.SavePluginConfig(config);
    }
    // Puts back the slots we replaced, unless the player changed them since.
    private void Restore(RaptureHotbarModule* module)
    {
        if (config.Character != player.ContentId || config.ClassJob != module->ActiveHotbarClassJobId) return;
        foreach (var saved in config.Slots)
        {
            var slot = module->GetSlotById(saved.Bar, saved.Slot);
            if (slot != null && slot->CommandType == HotbarSlotType.Action && slot->CommandId == saved.Replacement)
                slot->Set(saved.Type, saved.Id);
        }
    }
    private void DisposeHooks()
    {
        updateSlot?.Dispose(); loadIcon?.Dispose(); maxCharges?.Dispose(); iconId?.Dispose(); additionalGroup?.Dispose(); recastGroup?.Dispose();
        location?.Dispose(); recastTime?.Dispose(); castTime?.Dispose(); spell.Dispose(); status.Dispose();
    }
    public void Dispose()
    {
        DisposeHooks();
        var module = RaptureHotbarModule.Instance();
        if (module != null && module->ModuleReady && module->PvPHotbarsActive && module->ActiveHotbarClassJobId == config.ClassJob)
        { Restore(module); config.Slots.Clear(); pi.SavePluginConfig(config); }
    }
}
