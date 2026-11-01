using System.Diagnostics;
using System.Runtime.InteropServices;
using Dalamud.Hooking;
using Dalamud.Plugin.Services;
using Dalamud.Game.Addon.Lifecycle;
using Dalamud.Game.Addon.Lifecycle.AddonArgTypes;
using FFXIVClientStructs.FFXIV.Client.Game.Event;
using FFXIVClientStructs.FFXIV.Client.UI.Agent;

namespace Kawari.FeastHud;

// Addresses come from NativeClient, pinned per verified executable hash;
// unhooked entry points also verify their live prefix.
// Hooked entry points may retain Dalamud dispatch stubs across plugin reloads.
internal sealed unsafe class NativeDirector : IDisposable
{
    private readonly IClientState client;
    private readonly IObjectTable objects;
    private readonly ITargetManager targets;
    private readonly IDataManager data;
    private readonly IAddonLifecycle lifecycle;
    private readonly IPluginLog log;
    private readonly Hook<Refresh> refresh;
    private readonly Hook<SetData> setData;
    private readonly Open open;
    private readonly nint imageBase;
    private readonly nint legacyVtable;
    private readonly DirectorFrames frames = new();
    private readonly NativeRecast recast = new();
    private readonly NativeFeastLayout layout = new();
    private readonly NativeMatchHeader? header;
    private bool replacedRefresh;
    private bool headerFailed;
    private static readonly string[] Names = ["PvPMKSHeader", "PvPMKSPartyList1", "PvPMKSPartyList3"];
    public DirectorFrame? Frame => frames.Current;
    public long Received { get; private set; }
    public bool InArena => client.IsLoggedIn && client.TerritoryType == 745;
    private delegate void Refresh(nint agent, nint director, nint context);
    private delegate void Open(nint agent, nint context);
    private delegate void SetData(EventFramework* framework, EventId id, byte sequence, byte unknown, byte* data, ulong length);

    public NativeDirector(IClientState client, IObjectTable objects, IAddonLifecycle lifecycle,
        IGameInteropProvider interop, IPluginLog log, ITargetManager targets, IDataManager data, string pluginDirectory)
    {
        this.client = client; this.objects = objects; this.lifecycle = lifecycle; this.log = log;
        this.targets = targets; this.data = data;
        var module = Process.GetCurrentProcess().MainModule!;
        imageBase = module.BaseAddress;
        var native = NativeClient.Current;
        legacyVtable = imageBase + native.LegacyDirectorVtable;
        nint Address(int rva, string prefix, bool hook = false)
        {
            var expected = Convert.FromHexString(prefix);
            // Dalamud retains its dispatch stub after a hook is disposed. The
            // verified on-disk executable still pins hooked function addresses;
            // checking their live prologue would incorrectly reject a reload.
            if (!hook && !new ReadOnlySpan<byte>((void*)(imageBase + rva), expected.Length).SequenceEqual(expected))
                throw new InvalidOperationException($"Feast native function check failed at {rva:X}.");
            return imageBase + rva;
        }
        var refreshAddress = Address(native.PvPHeaderRefresh, "48896C24104889742418574883EC20", hook: true);
        open = Marshal.GetDelegateForFunctionPointer<Open>(Address(native.PvPHeaderOpen, "41564883EC70488B01"));
        setData = interop.HookFromAddress<SetData>((nint)EventFramework.MemberFunctionPointers.SetDirectorData, Receive);
        try { refresh = interop.HookFromAddress<Refresh>(refreshAddress, UpdateHeader); }
        catch { setData.Dispose(); throw; }
        foreach (var ev in new[] { AddonEvent.PreRequestedUpdate, AddonEvent.PostRequestedUpdate, AddonEvent.PreDraw, AddonEvent.PreFinalize })
            lifecycle.RegisterListener(ev, Names, OnAddon);
        setData.Enable(); refresh.Enable();
        // Header assets are optional presentation, never a prerequisite for the
        // director, roster, hotbar or suppression of CC's crystal announcements.
        try { header = new NativeMatchHeader(pluginDirectory); }
        catch (Exception ex) { log.Error(ex, "Feast match header assets could not load; remaining Feast features stay enabled."); }
        log.Information("Feast native director presentation enabled for legacy duty 40022.");
    }
    private void UpdateHeader(nint agent, nint director, nint context)
    {
        if (!InArena || director == 0 || *(nint*)director != legacyVtable)
        { refresh.Original(agent, director, context); return; }
        if (!replacedRefresh)
        {
            log.Information("Feast: replaced CC refresh for the active legacy director.");
            replacedRefresh = true;
        }
        // Keep native opening/closing and HUD positioning; replace all CC reads.
        if (!((AgentInterface*)agent)->IsAgentActive()) open(agent, context);
        NativeRoster.Update(Environment.TickCount64 - Received < 2500 ? Frame : null, objects, targets, data);
    }
    private void Receive(EventFramework* framework, EventId id, byte sequence, byte unknown, byte* data, ulong length)
    {
        if (InArena && id.Id == 0x80039C56 && data != null && length >= 10 && length <= 12
            && data[0] is >= 0xF1 and <= 0xF7)
        {
            if (frames.Accept(new ReadOnlySpan<byte>(data, 10)))
            {
                if (Received == 0) log.Information("Feast: received a complete native director frame ({Count} participants).", Frame!.Players.Count);
                Received = Environment.TickCount64;
            }
            // Continuations are fork-owned transport records, not director state.
            if (data[0] != 0xF1) return;
        }
        setData.Original(framework, id, sequence, unknown, data, length);
    }
    public void Update()
    {
        if (!InArena) { header?.Clear(); headerFailed = false; recast.Clear(); layout.Clear(); frames.Clear(); Received = 0; replacedRefresh = false; }
    }
    private void OnAddon(AddonEvent ev, AddonArgs args)
    {
        if (ev == AddonEvent.PreFinalize) { header?.Remove(args.Addon); recast.RemoveAddon(args.Addon); layout.Remove(args.Addon); return; }
        if (InArena && args.AddonName == "PvPMKSHeader")
        {
            if (ev == AddonEvent.PreRequestedUpdate) args.PreventOriginal();
            if (ev == AddonEvent.PreDraw)
            {
                if (header == null || headerFailed) { args.PreventOriginal(); return; }
                try { header.Update(args.Addon, Frame, Environment.TickCount64 - Received); }
                catch (Exception ex)
                {
                    header.Clear(); headerFailed = true; args.PreventOriginal();
                    log.Error(ex,"Feast native match header failed; disabled until next duty entry.");
                }
            }
            return;
        }
        if (InArena && args.AddonName is "PvPMKSPartyList1" or "PvPMKSPartyList3" && Frame is { } frame)
        {
            // These widgets are player/enemy, whereas director rosters are Fang/Claw.
            byte localTeam = frame.Summary.Team == "claw" ? (byte)1 : (byte)0;
            byte team = args.AddonName == "PvPMKSPartyList1" ? localTeam : (byte)(1 - localTeam);
            *((byte*)(nint)args.Addon + 0x239) = team;
            if (ev is AddonEvent.PreDraw or AddonEvent.PostRequestedUpdate)
            {
                layout.Update(args.Addon, frame, team, objects, data);
                recast.Update(args.Addon, frame, team, Environment.TickCount64 - Received);
            }
        }
    }
    public void Dispose()
    {
        refresh.Dispose(); setData.Dispose();
        lifecycle.UnregisterListener(OnAddon);
        header?.Clear();
        recast.Clear();
        layout.Clear();
        frames.Clear();
    }
}
