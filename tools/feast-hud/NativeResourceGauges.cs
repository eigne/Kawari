using System.Diagnostics;
using Dalamud.Hooking;
using Dalamud.Plugin.Services;
using Dalamud.Game.Addon.Lifecycle;
using Dalamud.Game.Addon.Lifecycle.AddonArgTypes;
using FFXIVClientStructs.FFXIV.Client.UI;
using FFXIVClientStructs.FFXIV.Component.GUI;

namespace Kawari.FeastHud;

// Feed retained native gauge renderers, including normal/simple layouts for all Feast jobs.
// Addresses and array offsets verified in the hash-pinned 7.51h2 executable.
internal sealed unsafe class NativeResourceGauges : IDisposable
{
    private readonly NativeDirector director;
    private readonly IObjectTable objects;
    private readonly IAddonLifecycle lifecycle;
    private readonly IGameInteropProvider interop;
    private readonly IPluginLog log;
    private readonly Dictionary<nint, Hook<ReadGaugeData>> readers = new();
    private readonly HashSet<string> reported = new();
    private readonly nint imageBase;
    private readonly int imageSize;
    private readonly Hook<RefreshJobHud> refreshJob;
    private bool wasActive;
    private int[]? savedLimit;
    private string? savedLabel;
    private delegate byte ReadGaugeData(nint addon, nint destination,
        NumberArrayData* numbers, nint strings);
    private delegate void RefreshJobHud(nint addon, NumberArrayData** numbers, StringArrayData** strings);
    public NativeResourceGauges(NativeDirector director, IObjectTable objects,
        IAddonLifecycle lifecycle, IGameInteropProvider interop, IPluginLog log)
    {
        this.director = director; this.objects = objects; this.lifecycle = lifecycle;
        this.interop = interop; this.log = log;
        var module = Process.GetCurrentProcess().MainModule!;
        imageBase = module.BaseAddress; imageSize = module.ModuleMemorySize;
        refreshJob = interop.HookFromAddress<RefreshJobHud>(imageBase + NativeClient.Current.JobHudRequestedUpdate, RefreshJob);
        try
        {
            lifecycle.RegisterListener(AddonEvent.PreRequestedUpdate, "_LimitBreak", BeforeLimit);
            refreshJob.Enable();
        }
        catch { lifecycle.UnregisterListener(BeforeLimit); refreshJob.Dispose(); throw; }
    }
    private uint ActiveJob => director.InArena ? objects.LocalPlayer?.ClassJob.RowId ?? 0 : 0;
    private NativeJobGaugeData.Layout? Identify(nint addon) => addon == 0 ? null
        : NativeJobGaugeData.Identify(new ReadOnlySpan<byte>((void*)(addon + 8), 32), ActiveJob);
    private DirectorFrame? Current => director.InArena && Environment.TickCount64 - director.Received < 2500 ? director.Frame : null;
    private void RefreshJob(nint addon, NumberArrayData** numbers, StringArrayData** strings)
    {
        var layout = Identify(addon);
        var n = numbers == null ? null : numbers[87];
        if (layout == null || n == null || n->Size <= 25 || !EnsureReader(addon, layout))
        { refreshJob.Original(addon, numbers, strings); return; }
        int enabled = n->IntArray[25];
        n->IntArray[25] = layout.Visible ? 1 : 0;
        try { refreshJob.Original(addon, numbers, strings); }
        finally { n->IntArray[25] = enabled; }
    }
    private bool EnsureReader(nint addon, NativeJobGaugeData.Layout layout)
    {
        // Base JobHud RequestedUpdate calls virtual slot81 to read gauge data,
        // before rendering either the standard or simplified layout. Resolve it
        // from the actual addon; lifecycle may clone the surrounding vtable.
        var hud = (AddonJobHud*)addon;
        long size = (byte*)hud->DataCurrentPointer - (byte*)hud->DataPreviousPointer;
        nint address = (*(nint**)addon)[81];
        if (size < layout.Size || size > 128 || address < imageBase || address >= imageBase + imageSize)
        {
            if (reported.Add(layout.Name)) log.Warning("Feast gauge {Name}: native reader/layout mismatch; left untouched.", layout.Name);
            return false;
        }
        if (readers.ContainsKey(address)) return true;
        Hook<ReadGaugeData>? reader = null;
        try
        {
            reader = interop.HookFromAddress<ReadGaugeData>(address, (unit, destination, data, text) =>
            {
                byte result = reader!.Original(unit, destination, data, text);
                var currentLayout = Identify(unit);
                if (destination == 0 || currentLayout == null) return result;
                var gauge = Current?.Gauge;
                if (gauge == null || gauge.Job != currentLayout.Job)
                    gauge = new(currentLayout.Job,0,0,0,0,false,0);
                return NativeJobGaugeData.Write(currentLayout, gauge,
                    new Span<byte>((void*)destination,currentLayout.Size)) ? (byte)1 : result;
            });
            reader.Enable(); readers.Add(address,reader);
            log.Information("Feast native gauge reader enabled: {Name}, RVA {Address:X}.", layout.Name, address-imageBase);
            return true;
        }
        catch (Exception ex)
        {
            reader?.Dispose();
            if (reported.Add(layout.Name)) log.Error(ex,"Feast gauge reader could not be enabled for {Name}.",layout.Name);
            return false;
        }
    }
    private void BeforeLimit(AddonEvent ev, AddonArgs args) => UpdateLimit(true);
    public void Update()
    {
        UpdateLimit(false);
        bool active = FeastJobActions.Supports(ActiveJob);
        if (active || wasActive)
        {
            // The current PvP client does not change its PvE job number array.
            // Invalidate subscribers so our readers receive each F7 update,
            // including empty gauges and the first refresh after leaving Feast.
            var stage = AtkStage.Instance();
            var n = stage == null ? null : stage->GetNumberArrayData((NumberArrayType)87);
            if (n != null && n->UpdateState == 0) n->UpdateState = 1;
        }
        wasActive = active;
    }
    private void UpdateLimit(bool suppress)
    {
        var stage = AtkStage.Instance();
        if (stage == null) return;
        // _LimitBreak RequestedUpdate RVA1599C10: number[43], string[75].
        var n = stage->GetNumberArrayData((NumberArrayType)43);
        var s = stage->GetStringArrayData((StringArrayType)75);
        if (n == null || n->Size < 7 || s == null || s->Size < 1) return;
        if (!director.InArena)
        {
            if (savedLimit == null) return;
            for (int i = 0; i < 7; i++) n->SetValue(i, savedLimit[i]);
            s->SetValue(0, savedLabel ?? "", managed: true);
            savedLimit = null; savedLabel = null;
            return;
        }
        if (savedLimit == null)
        {
            savedLimit = new ReadOnlySpan<int>(n->IntArray, 7).ToArray();
            savedLabel = s->StringArray[0].ToString();
        }
        var self = Current?.Players.FirstOrDefault(p => p.Id == objects.LocalPlayer?.EntityId);
        // Normal refresh, PvP skin, one 10,000-unit bar, authoritative fill.
        int[] values = [2, 1, 1, (self?.Adrenaline ?? 0) * 100, 0, 0, 0];
        for (int i = 0; i < values.Length; i++) n->SetValue(i, values[i], suppressUpdates: suppress);
        s->SetValue(0, "ADRENALINE", managed: true, suppressUpdates: suppress);
    }
    public void Dispose()
    {
        refreshJob.Dispose();
        foreach (var reader in readers.Values) reader.Dispose();
        readers.Clear(); lifecycle.UnregisterListener(BeforeLimit);
        // Restore shared arrays even if unloaded while still in the arena.
        var stage = AtkStage.Instance();
        if (stage != null && wasActive)
        {
            var job = stage->GetNumberArrayData((NumberArrayType)87);
            if (job != null && job->UpdateState == 0) job->UpdateState = 1;
        }
        if (savedLimit != null && stage != null)
        {
            var n = stage->GetNumberArrayData((NumberArrayType)43);
            var s = stage->GetStringArrayData((StringArrayType)75);
            if (n != null && n->Size >= 7) for (int i = 0; i < 7; i++) n->SetValue(i, savedLimit[i]);
            if (s != null && s->Size >= 1) s->SetValue(0, savedLabel ?? "", managed: true);
        }
    }
}
