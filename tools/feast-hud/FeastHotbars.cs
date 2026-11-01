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
    public List<HotbarBackup> Slots { get; set; } = [];
    // Per-job Feast hotbar layout from the PvP Actions window; empty means the default kit.
    public Dictionary<uint, List<LayoutSlot>> Layouts { get; set; } = [];
    public Dictionary<uint, List<string>> Loadouts { get; set; } = [];
}

// Temporary live PvP slots. Casts still travel through native ActionRequest,
// ActionCast and ActionResult; the server's Feast engine validates each request.
internal sealed unsafe class FeastHotbars : IDisposable
{
    private static readonly uint[] WhmActions = FeastActionMenu.WhmButtons;
    // UseAction resolves the spell, adjusts it, then calls GetActionStatus with
    // that resolved ID. Recognize both stages, not just the original hotbar ID.
    private static readonly HashSet<uint> NativeActions = [..WhmActions, ..WhmActions.Select(FeastSpells.Resolve),
        119, 127, 3568, 7431, 25859, 121, 132, 133, 37010];
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
    private readonly Hook<LoadComponentIcon>? componentIcon;
    private readonly Dictionary<uint, uint> whmIconReplacements = [];
    [return: System.Runtime.InteropServices.MarshalAs(System.Runtime.InteropServices.UnmanagedType.I1)]
    private delegate bool LoadComponentIcon(AtkComponentIcon* icon, uint id);
    private delegate void UpdateSlot(AddonActionBarBase* addon, ActionBarSlot* slot, NumberArrayData* numbers, StringArrayData* strings, int numberIndex, int stringIndex);
    [return: System.Runtime.InteropServices.MarshalAs(System.Runtime.InteropServices.UnmanagedType.I1)]
    private delegate bool LoadIcon(HotbarSlot* slot);
    private delegate ushort MaxCharges(uint id, uint level);
    private delegate int RecastGroup(ActionManager* manager, int type, uint id);
    private delegate int IconId(HotbarSlot* slot, HotbarSlotType type, uint id);
    private JobAction? Metadata(uint id) => FeastJobActions.Find(player.ClassJob.RowId == 24 ? FeastSpells.Resolve(id) : id);
    [return: System.Runtime.InteropServices.MarshalAs(System.Runtime.InteropServices.UnmanagedType.I1)]
    private delegate bool UseLocation(ActionManager* manager, ActionType type, uint id, ulong target, System.Numerics.Vector3* position, uint extra, byte route);
    private IReadOnlyList<HotbarAction> live = [];
    private DirectorFrame? resources;
    private delegate int CastTime(ActionType type, uint id, [System.Runtime.InteropServices.MarshalAs(System.Runtime.InteropServices.UnmanagedType.I1)] bool procs, ActionManager.CastTimeProc* proc);
    private delegate int RecastTime(ActionType type, uint id, [System.Runtime.InteropServices.MarshalAs(System.Runtime.InteropServices.UnmanagedType.I1)] bool mechanics);
    private readonly FeastHotbarConfig config;
    private bool applied;
    private long nextScan;
    public bool Active => applied && Eligible;
    public FeastHotbarConfig Config => config;
    public void Save() => pi.SavePluginConfig(config);
    private delegate uint SpellId(ActionType type, uint id);
    private delegate uint ActionStatus(ActionManager* manager, ActionType type, uint id, ulong target,
        [System.Runtime.InteropServices.MarshalAs(System.Runtime.InteropServices.UnmanagedType.I1)] bool recast,
        [System.Runtime.InteropServices.MarshalAs(System.Runtime.InteropServices.UnmanagedType.I1)] bool casting, uint* extra);
    private bool Eligible => client.IsLoggedIn && FeastActionMenu.UsesFeastHotbars(client.TerritoryType) && FeastJobActions.Supports(player.ClassJob.RowId);
    // In the Wolves' Den the layout is shown but the native checks keep it unusable.
    private bool Castable => Eligible && FeastActionMenu.CanCast(client.TerritoryType);
    private bool Allowed(uint id) => player.ClassJob.RowId == 24 ? NativeActions.Contains(id) : FeastJobActions.Contains(player.ClassJob.RowId,id);
    public FeastHotbars(IDalamudPluginInterface pi, IClientState client, IPlayerState player, IObjectTable objects, IGameInteropProvider interop, IDataManager data)
    {
        this.pi = pi; this.client = client; this.player = player; this.objects = objects;
        var actions = data.GetExcelSheet<Lumina.Excel.Sheets.Action>();
        foreach (uint id in WhmActions)
        {
            uint current = actions.GetRow(id).Icon;
            uint historical = FeastSpells.Icon(id);
            if (current > 0 && current != 784 && historical > 0)
                whmIconReplacements[current] = historical;
        }
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
            componentIcon = interop.HookFromAddress<LoadComponentIcon>((nint)AtkComponentIcon.MemberFunctionPointers.LoadIcon, LoadFeastComponentIcon);
            status.Enable(); spell.Enable(); castTime.Enable(); recastTime.Enable(); location.Enable();
            recastGroup.Enable(); additionalGroup.Enable(); iconId.Enable();
            maxCharges.Enable();
            loadIcon.Enable();
            updateSlot.Enable();
            componentIcon.Enable();
        }
        catch { componentIcon?.Dispose(); updateSlot?.Dispose(); loadIcon?.Dispose(); maxCharges?.Dispose(); iconId?.Dispose(); additionalGroup?.Dispose(); recastGroup?.Dispose(); location?.Dispose(); recastTime?.Dispose(); castTime?.Dispose(); spell.Dispose(); status.Dispose(); throw; }
    }
    private bool LoadFeastComponentIcon(AtkComponentIcon* icon, uint id)
        => componentIcon!.Original(icon, Eligible && player.ClassJob.RowId == 24 ? whmIconReplacements.GetValueOrDefault(id, id) : id);
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
        => Eligible && Allowed(id) && Metadata(id) is { } a ? (ushort)Math.Max(1, a.Charges) : maxCharges!.Original(id, level);
    private int GetGroup(ActionManager* manager, int type, uint id)
        => Eligible && type == (int)ActionType.Action && Allowed(id) && Metadata(id) is { } a
            ? a.Group : recastGroup!.Original(manager,type,id);
    private int GetAdditionalGroup(ActionManager* manager, int type, uint id)
        => Eligible && type == (int)ActionType.Action && Allowed(id) && Metadata(id) is { } a
            ? a.Secondary : additionalGroup!.Original(manager,type,id);
    private int GetIcon(HotbarSlot* slot, HotbarSlotType type, uint id)
    {
        if (Eligible && type == HotbarSlotType.Action && Allowed(id))
        {
            uint resolved = player.ClassJob.RowId == 24 ? FeastSpells.Resolve(id) : live.FirstOrDefault(a => a.BaseId == id)?.ResolvedId ?? id;
            if (FeastJobActions.Find(resolved) is { } action) return action.Icon;
        }
        return iconId!.Original(slot,type,id);
    }
    private uint GetSpell(ActionType type, uint id)
        => Castable && type == ActionType.Action && Allowed(id)
            ? player.ClassJob.RowId == 24 ? FeastSpells.Resolve(id) : live.FirstOrDefault(a => a.BaseId == id)?.ResolvedId ?? id : spell.Original(type, id);
    private bool UseGround(ActionManager* manager, ActionType type, uint id, ulong target, System.Numerics.Vector3* position, uint extra, byte route)
    {
        if (!Castable || type != ActionType.Action || !Allowed(id) || id is not (7857 or 3361 or 8812) || position == null)
            return location!.Original(manager,type,id,target,position,extra,route);
        if (objects.LocalPlayer is { IsCasting: true }) return false;
        var p = *position;
        if (!float.IsFinite(p.X) || !float.IsFinite(p.Y) || !float.IsFinite(p.Z)) return false;
        var ui = FFXIVClientStructs.FFXIV.Client.UI.UIModule.Instance();
        if (ui == null) return false;
        var message = FFXIVClientStructs.FFXIV.Client.System.String.Utf8String.FromString(
            FormattableString.Invariant($"!feast ground {id} {p.X:R} {p.Y:R} {p.Z:R}"));
        try { ui->ProcessChatBoxEntry(message); } finally { message->Dtor(true); }
        return true;
    }
    private int GetCastTime(ActionType type, uint id, bool procs, ActionManager.CastTimeProc* proc)
    {
        if (Eligible && type == ActionType.Action && Allowed(id))
        {
            if (proc != null) *proc = default;
            return live.FirstOrDefault(a => a.BaseId == id)?.CastMs ?? (int)((Metadata(id)?.Cast ?? 0) * 1000);
        }
        return castTime!.Original(type,id,procs,proc);
    }
    private int GetRecastTime(ActionType type, uint id, bool mechanics)
        => Eligible && type == ActionType.Action && Allowed(id)
            ? live.FirstOrDefault(a => a.BaseId == id)?.RecastMs ?? (int)((Metadata(id)?.Recast ?? 0) * 1000)
            : recastTime!.Original(type,id,mechanics);
    private uint GetStatus(ActionManager* manager, ActionType type, uint id, ulong target, bool recast, bool casting, uint* extra)
    {
        // Retail's PvE/PvP, lily and unlock gates describe a different ruleset.
        // Never relax checks for another territory, job, or unimplemented action.
        if (Castable && type == ActionType.Action && Allowed(id))
        {
            if (extra != null) *extra = 0;
            if (!FeastResources.CanUse(player.ClassJob.RowId == 24 ? FeastSpells.Resolve(id) : id,
                    resources, objects.LocalPlayer?.EntityId ?? 0)) return 0x246;
            // Retain native timing gates and their normal status codes. Returning
            // success during these gates starts requests the server must cancel.
            // Honor the flags: UseAction also probes without these gates to queue.
            if (casting && objects.LocalPlayer is { IsCasting: true }) return 0x244;
            if (recast && manager->IsRecastTimerActive(type, id)) return 0x246;
            return 0;
        }
        return status.Original(manager, type, id, target, recast, casting, extra);
    }
    public void Update(DirectorFrame? frame)
    {
        resources = Eligible ? frame : null;
        live = Eligible ? frame?.Hotbar ?? [] : [];
        if (Eligible && frame?.Gauge is { } gauge && gauge.Job == player.ClassJob.RowId)
        {
            var manager = JobGaugeManager.Instance();
            if (manager != null && manager->ClassJobId == gauge.Job)
            {
                if (gauge.Job == 24)
                {
                    manager->WhiteMage.Lily = gauge.Resource;
                    manager->WhiteMage.BloodLily = gauge.Meditation;
                    manager->WhiteMage.LilyTimer = (short)gauge.Timer;
                }
                if (gauge.Job == 19) manager->Paladin.OathGauge = gauge.Resource;
                if (gauge.Job == 21) manager->Warrior.BeastGauge = gauge.Resource;
                if (gauge.Job == 20) manager->Monk.Chakra = gauge.Resource;
                if (gauge.Job == 30) manager->Ninja.Ninki = gauge.Resource;
                if (gauge.Job == 23)
                {
                    manager->Bard.SoulVoice = gauge.Resource;
                    manager->Bard.Repertoire = gauge.Meditation;
                    manager->Bard.SongTimer = gauge.Timer;
                    manager->Bard.SongFlags = gauge.Element == 1 ? FFXIVClientStructs.FFXIV.Client.Game.Gauge.SongFlags.WanderersMinuet
                        : gauge.Element == 2 ? FFXIVClientStructs.FFXIV.Client.Game.Gauge.SongFlags.ArmysPaeon : 0;
                }
                if (gauge.Job == 31)
                {
                    manager->Machinist.Heat = gauge.Resource;
                    manager->Machinist.OverheatTimeRemaining = (short)gauge.Timer;
                    manager->Machinist.TimerActive = gauge.Element;
                }
                if (gauge.Job == 27)
                {
                    // Retail stores one bit per stack; the director sends a count.
                    manager->Summoner.AetherFlags = (FFXIVClientStructs.FFXIV.Client.Game.Gauge.AetherFlags)((1 << gauge.Resource) - 1);
                    manager->Summoner.SummonTimer = gauge.Timer;
                }
                if (gauge.Job == 28) manager->Scholar.FairyGauge = gauge.Resource;
                // CurrentArcana reads the upper nibble of Cards in this SDK.
                if (gauge.Job == 33) manager->Astrologian.Cards = (short)(gauge.Resource << 12);
                if (gauge.Job == 35) { manager->RedMage.BlackMana = gauge.Resource; manager->RedMage.WhiteMana = gauge.Meditation; }
                if (gauge.Job == 32)
                {
                    manager->DarkKnight.Blood = gauge.Resource;
                    manager->DarkKnight.DarksideTimer = gauge.Timer;
                    manager->DarkKnight.DarkArtsState = gauge.Element;
                }
                if (gauge.Job == 37) manager->Gunbreaker.Ammo = gauge.Resource;
                if (gauge.Job == 38)
                {
                    manager->Dancer.Esprit = gauge.Resource; manager->Dancer.Feathers = gauge.Meditation;
                    manager->Dancer.StepIndex = gauge.Polyglot;
                    for (int i = 0; i < 4; i++) manager->Dancer.DanceSteps[i] = (byte)((gauge.Timer >> (4*i)) & 15);
                }
                if (gauge.Job == 39)
                {
                    manager->Reaper.Soul = gauge.Resource; manager->Reaper.Shroud = gauge.Meditation;
                    manager->Reaper.LemureShroud = gauge.Polyglot; manager->Reaper.VoidShroud = gauge.Element;
                    manager->Reaper.EnshroudedTimeRemaining = gauge.Timer;
                }
                if (gauge.Job == 40)
                {
                    manager->Sage.Addersgall = gauge.Resource; manager->Sage.AddersgallTimer = (short)gauge.Timer;
                    manager->Sage.Eukrasia = gauge.Element;
                }
                if (gauge.Job == 22)
                {
                    manager->Dragoon.EyeCount = gauge.Resource;
                    manager->Dragoon.LotdTimer = (short)gauge.Timer;
                    manager->Dragoon.LotdState = gauge.Element;
                }
                if (gauge.Job == 34) { manager->Samurai.Kenki = gauge.Resource; manager->Samurai.MeditationStacks = gauge.Meditation; }
                if (gauge.Job == 25)
                {
                    manager->BlackMage.ElementStance = (sbyte)(gauge.Element == 1 ? 1 : gauge.Element == 2 ? -1 : 0);
                    manager->BlackMage.PolyglotStacks = gauge.Polyglot;
                    manager->BlackMage.EnochianTimer = (short)gauge.Timer;
                    manager->BlackMage.EnochianFlags = gauge.Enochian ? FFXIVClientStructs.FFXIV.Client.Game.Gauge.EnochianFlags.Enochian : 0;
                }
            }
        }
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
        if (config.Character == player.ContentId && config.Slots.Count != 0) Restore(module);
        config.Character = player.ContentId; config.ClassJob = player.ClassJob.RowId; config.Slots.Clear();
        var layout = config.Layouts.GetValueOrDefault(player.ClassJob.RowId)?.Where(s => s.Bar < 10 && s.Slot < 12 && Allowed(s.Id)).ToList();
        if (layout is not { Count: > 0 })
        {
            var Actions = FeastActionMenu.MenuIds(player.ClassJob.RowId);
            layout = Actions.Select((id, i) => new LayoutSlot((uint)i / 12, (uint)i % 12, id)).ToList();
        }
        foreach (var placed in layout)
        {
            var slot = module->GetSlotById(placed.Bar, placed.Slot);
            config.Slots.Add(new(placed.Bar, placed.Slot, slot->CommandType, slot->CommandId, placed.Id));
        }
        pi.SavePluginConfig(config);
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
            if (Metadata(saved.Replacement) is { } metadata)
                slot->PopUpHelp.SetString(metadata.Name + slot->PopUpKeybindHintString);
            if (player.ClassJob.RowId == 24)
            {
                uint icon = FeastSpells.Icon(saved.Replacement);
                if (icon != 0) slot->IconId = icon;
                continue;
            }
            uint resolved = live.FirstOrDefault(a => a.BaseId == saved.Replacement)?.ResolvedId ?? saved.Replacement;
            if (FeastJobActions.Find(resolved) is { } action) slot->IconId = (uint)action.Icon;
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
    public void Dispose()
    {
        componentIcon?.Dispose(); updateSlot?.Dispose(); loadIcon?.Dispose(); maxCharges?.Dispose(); iconId?.Dispose(); additionalGroup?.Dispose(); recastGroup?.Dispose();
        location?.Dispose(); recastTime?.Dispose(); castTime?.Dispose(); spell.Dispose(); status.Dispose();
        var module = RaptureHotbarModule.Instance();
        if (module != null && module->ModuleReady && module->PvPHotbarsActive && module->ActiveHotbarClassJobId == config.ClassJob)
        { Restore(module); config.Slots.Clear(); pi.SavePluginConfig(config); }
    }
}
