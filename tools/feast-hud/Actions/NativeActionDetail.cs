using System.Globalization;
using System.Diagnostics;
using Dalamud.Hooking;
using Dalamud.Game.Addon.Lifecycle;
using Dalamud.Game.Addon.Lifecycle.AddonArgTypes;
using Dalamud.Plugin.Services;
using FFXIVClientStructs.FFXIV.Client.UI.Agent;
using FFXIVClientStructs.FFXIV.Component.GUI;

namespace Kawari.FeastHud;

// Feed the surviving native tooltip, preserving its layout, placement and input.
// Array indices verified against AgentActionDetail's 7.51h2 native setters;
// ClientStructs keeps the same ActionDetail array numbering in 7.57.
internal sealed unsafe class NativeActionDetail : IDisposable
{
    private readonly IAddonLifecycle lifecycle;
    private readonly IClientState client;
    private readonly IPlayerState player;
    private readonly Hook<AgentUpdate> update;
    private delegate void AgentUpdate(AgentActionDetail* agent, uint frame);
    public NativeActionDetail(IAddonLifecycle lifecycle, IClientState client, IPlayerState player, IGameInteropProvider interop)
    {
        this.lifecycle = lifecycle; this.client = client; this.player = player;
        // NativeDirector has already verified the executable hash. This agent
        // refreshes arrays without always requesting an addon lifecycle update.
        var address = Process.GetCurrentProcess().MainModule!.BaseAddress + NativeClient.Current.ActionDetailUpdate;
        // A disposed Dalamud hook leaves a dispatch stub at this address.
        // The executable hash pins the original function even across reloads.
        update = interop.HookFromAddress<AgentUpdate>(address, AfterUpdate);
        try
        {
            lifecycle.RegisterListener(AddonEvent.PreRequestedUpdate, "ActionDetail", Update);
            update.Enable();
        }
        catch { lifecycle.UnregisterListener(AddonEvent.PreRequestedUpdate, "ActionDetail", Update); update.Dispose(); throw; }
    }
    private void AfterUpdate(AgentActionDetail* agent, uint frame)
    {
        update.Original(agent, frame);
        Apply(agent, false);
    }
    private void Update(AddonEvent ev, AddonArgs args)
        => Apply(AgentActionDetail.Instance(), true);
    private void Apply(AgentActionDetail* agent, bool suppress)
    {
        if (!client.IsLoggedIn || client.TerritoryType != 745) return;
        if (agent == null || !agent->IsAgentActive() || (int)agent->ActionKind != 30) return;
        uint id = agent->ActionId;
        if (!FeastJobActions.Contains(player.ClassJob.RowId, id) || FeastJobActions.Find(id) is not { } action) return;
        // The agent clears its temporary array pointers before Update returns.
        // Fetch the persistent arrays from AtkStage, as the native agent does.
        var stage = AtkStage.Instance();
        if (stage == null) return;
        var n = stage->GetNumberArrayData((NumberArrayType)32);
        var s = stage->GetStringArrayData((StringArrayType)29);
        if (n == null || s == null || n->Size < 8 || s->Size < 14) return;
        // Cast/recast/range/radius/description. Hide retail unlock and job-level rows.
        n->SetValue(0, 0x3b | (action.MpCost > 0 ? 4 : 0), suppressUpdates: suppress);
        n->SetValue(1, action.Icon, suppressUpdates: suppress);
        n->SetValue(2, -1, suppressUpdates: suppress);
        n->SetValue(7, n->IntArray[7] | 1, suppressUpdates: suppress);
        void Set(int index, string text) => s->SetValue(index, text, managed: true, suppressUpdates: suppress);
        Set(0, action.Name); Set(1, action.Category); Set(2, "");
        Set(3, "Range "); Set(4, action.Range + "y  ");
        Set(5, "Radius "); Set(6, action.Radius + "y");
        Set(7, "MP Cost"); Set(8, action.MpCost.ToString(CultureInfo.InvariantCulture));
        Set(9, "Recast"); Set(10, action.Recast.ToString("0.00", CultureInfo.InvariantCulture) + "s");
        Set(11, "Cast"); Set(12, action.Cast == 0 ? "Instant" : action.Cast.ToString("0.00", CultureInfo.InvariantCulture) + "s");
        Set(13, action.Description);
    }
    public void Dispose() { update.Dispose(); lifecycle.UnregisterListener(AddonEvent.PreRequestedUpdate, "ActionDetail", Update); }
}
