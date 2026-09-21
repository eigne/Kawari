using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using Dalamud.Hooking;
using Dalamud.Plugin.Services;
using Dalamud.Game.Addon.Lifecycle;
using Dalamud.Game.Addon.Lifecycle.AddonArgTypes;
using FFXIVClientStructs.FFXIV.Client.Game.Event;
using FFXIVClientStructs.FFXIV.Client.UI.Agent;

namespace Kawari.FeastHud;

// Addresses are from the verified June 7.51h2 binary, not a newer struct dump.
// The executable hash and function prefixes must both match before enabling.
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
    private readonly DirectorFrames frames = new();
    private readonly NativeRecast recast = new();
    private bool replacedRefresh;
    private static readonly string[] Names = ["PvPMKSHeader", "PvPMKSPartyList1", "PvPMKSPartyList3"];
    public DirectorFrame? Frame => frames.Current;
    public long Received { get; private set; }
    public bool InArena => client.IsLoggedIn && client.TerritoryType == 745;
    private delegate void Refresh(nint agent, nint director, nint context);
    private delegate void Open(nint agent, nint context);
    private delegate void SetData(EventFramework* framework, EventId id, byte sequence, byte unknown, byte* data, ulong length);

    public NativeDirector(IClientState client, IObjectTable objects, IAddonLifecycle lifecycle,
        IGameInteropProvider interop, IPluginLog log, ITargetManager targets, IDataManager data)
    {
        this.client = client; this.objects = objects; this.lifecycle = lifecycle; this.log = log;
        this.targets = targets; this.data = data;
        var module = Process.GetCurrentProcess().MainModule!;
        imageBase = module.BaseAddress;
        using var file = File.OpenRead(module.FileName);
        if (Convert.ToHexString(SHA256.HashData(file)) != "4236E770E673150E85F8D10BEAB2FC4834C82F86AAB8A555A9175439FC906A6D")
            throw new InvalidOperationException("Feast native UI requires client 7.51h2 / 2026.06.18.");
        nint Address(int rva, string prefix)
        {
            var expected = Convert.FromHexString(prefix);
            if (!new ReadOnlySpan<byte>((void*)(imageBase + rva), expected.Length).SequenceEqual(expected))
                throw new InvalidOperationException($"Feast native function check failed at {rva:X}.");
            return imageBase + rva;
        }
        var refreshAddress = Address(0xD5C0A0, "48896C24104889742418574883EC20");
        open = Marshal.GetDelegateForFunctionPointer<Open>(Address(0xD5C670, "41564883EC70488B01"));
        setData = interop.HookFromAddress<SetData>((nint)EventFramework.MemberFunctionPointers.SetDirectorData, Receive);
        try { refresh = interop.HookFromAddress<Refresh>(refreshAddress, UpdateHeader); }
        catch { setData.Dispose(); throw; }
        foreach (var ev in new[] { AddonEvent.PreRequestedUpdate, AddonEvent.PreDraw, AddonEvent.PreFinalize })
            lifecycle.RegisterListener(ev, Names, OnAddon);
        setData.Enable(); refresh.Enable();
        log.Information("Feast native director presentation enabled for legacy duty 40022.");
    }
    private void UpdateHeader(nint agent, nint director, nint context)
    {
        if (!InArena || director == 0 || *(nint*)director != imageBase + 0x234E100)
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
            && data[0] is >= 0xF1 and <= 0xF5)
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
        if (!InArena) { recast.Clear(); frames.Clear(); Received = 0; replacedRefresh = false; }
    }
    private void OnAddon(AddonEvent ev, AddonArgs args)
    {
        if (ev == AddonEvent.PreFinalize) { recast.RemoveAddon(args.Addon); return; }
        if (InArena && args.AddonName is "PvPMKSPartyList1" or "PvPMKSPartyList3" && Frame is { } frame)
        {
            // These widgets are player/enemy, whereas director rosters are Fang/Claw.
            byte localTeam = frame.Summary.Team == "claw" ? (byte)1 : (byte)0;
            byte team = args.AddonName == "PvPMKSPartyList1" ? localTeam : (byte)(1 - localTeam);
            *((byte*)(nint)args.Addon + 0x239) = team;
            if (ev == AddonEvent.PreDraw)
                recast.Update(args.Addon, frame, team, Environment.TickCount64 - Received);
        }
        // The graphite header stays custom; party rows now consume our native
        // array data and retain the game's own rendering and click callbacks.
        if (InArena && args.AddonName == "PvPMKSHeader" && ev is AddonEvent.PreRequestedUpdate or AddonEvent.PreDraw)
            args.PreventOriginal();
    }
    public void Dispose()
    {
        refresh.Dispose(); setData.Dispose();
        lifecycle.UnregisterListener(OnAddon);
        recast.Clear();
        frames.Clear();
    }
}
