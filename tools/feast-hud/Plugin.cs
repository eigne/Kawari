using Dalamud.Game.Command;
using Dalamud.Game.Addon.Lifecycle;
using Dalamud.Game.Addon.Lifecycle.AddonArgTypes;
using Dalamud.Interface.Windowing;
using Dalamud.Plugin;
using Dalamud.Plugin.Services;
using FFXIVClientStructs.FFXIV.Component.GUI;

namespace Kawari.FeastHud;

// Kawari Feast HUD: makes today's client look and behave like the Feast.
// The server owns every rule; this plugin only presents its state and lets
// the client use the historical actions. See docs/src/feast/hud-plugin.md.
//
//   NativeDirector       receives the server's match frames; drives the CC
//                        header, party lists and target bars in the arena
//   FeastHotbars         historical actions on the native hotbars
//   NativeActionDetail   their tooltips
//   NativeResourceGauges job gauges and the adrenaline (limit break) bar
//   NativeNamePlateMedals medals on nameplates
//   RespawnPrompt        the ten-second return countdown
//   FeastActionsWindow   the PvP Actions window in the Wolves' Den
public sealed unsafe class Plugin : IDalamudPlugin
{
    private const string DenOnly = "PvP actions can only be changed in the Wolves' Den. Use !feast den to travel there.";
    // Director frames older than this are treated as missing.
    private const long FrameLifetimeMs = 2500;
    private readonly IClientState client;
    private readonly IFramework framework;
    private readonly IChatGui chat;
    private readonly ICommandManager commands;
    private readonly IGameGui gameGui;
    private readonly IObjectTable objects;
    private readonly IDalamudPluginInterface pi;
    private readonly IAddonLifecycle lifecycle;
    private readonly IPlayerState player;
    private readonly NativeDirector native;
    private readonly FeastHotbars hotbars;
    private readonly NativeActionDetail actionDetail;
    private readonly NativeResourceGauges resourceGauges;
    private readonly NativeNamePlateMedals namePlateMedals;
    private readonly RespawnPrompt respawnPrompt;
    private readonly WindowSystem windows = new("Kawari.FeastHud");
    private readonly FeastActionsWindow actionsWindow;
    private bool openedByProfile, loadoutSynced;
    private bool InArena => client.IsLoggedIn && client.TerritoryType == FeastActionMenu.Arena;
    private DirectorFrame? FreshFrame => Environment.TickCount64 - native.Received < FrameLifetimeMs ? native.Frame : null;

    public Plugin(IDalamudPluginInterface pi, IClientState client, IFramework framework,
        IChatGui chat, ICommandManager commands, IGameGui gameGui, IObjectTable objects,
        IGameInteropProvider interop, IAddonLifecycle lifecycle, IPluginLog log, IPlayerState player, ITargetManager targets, IDataManager data,
        ITextureProvider textures, INamePlateGui namePlates)
    {
        this.pi = pi; this.lifecycle = lifecycle; this.player = player;
        this.gameGui = gameGui; this.objects = objects;
        this.client = client; this.framework = framework; this.chat = chat; this.commands = commands;
        // Each part hooks the game; if a later one fails, undo the earlier ones.
        native = new NativeDirector(client, objects, lifecycle, interop, log, targets, data,
            pi.AssemblyLocation.DirectoryName!);
        try { hotbars = new FeastHotbars(pi, client, player, objects, interop); }
        catch { native.Dispose(); throw; }
        try { actionDetail = new NativeActionDetail(lifecycle, client, player, interop); }
        catch { hotbars.Dispose(); native.Dispose(); throw; }
        try { resourceGauges = new NativeResourceGauges(native, objects, lifecycle, interop, log); }
        catch { actionDetail.Dispose(); hotbars.Dispose(); native.Dispose(); throw; }
        namePlateMedals = new NativeNamePlateMedals(namePlates, native);
        respawnPrompt = new RespawnPrompt(gameGui, objects, chat, native);
        actionsWindow = new FeastActionsWindow(client, player, gameGui, textures, data, hotbars);
        windows.AddWindow(actionsWindow);
        pi.UiBuilder.Draw += windows.Draw;
        pi.UiBuilder.OpenMainUi += ToggleActions;
        // The CC-era PvP Profile replaced the Feast PvP Actions window; open it alongside.
        lifecycle.RegisterListener(AddonEvent.PostSetup, "PvpProfile", OnProfile);
        lifecycle.RegisterListener(AddonEvent.PreFinalize, "PvpProfile", OnProfile);
        framework.Update += Update;
        commands.AddHandler("/feastactions", new CommandInfo((_, _) => ToggleActions()) { HelpMessage = "Open the Feast PvP Actions window (Wolves' Den only)." });
        commands.AddHandler("/feasthud", new CommandInfo((_, _) => chat.Print("Feast uses the native match header. Move or scale it with Crystalline Conflict Progress Gauge in HUD Layout.")) { HelpMessage = "Native Feast header follows the Crystalline Conflict Progress Gauge HUD position." });
    }

    private void Update(IFramework _)
    {
        native.Update();
        namePlateMedals.Update();
        resourceGauges.Update();
        hotbars.Update(FreshFrame);
        SyncLoadout(FreshFrame);
        respawnPrompt.Update();
        if (actionsWindow.IsOpen && !actionsWindow.InDen) { actionsWindow.IsOpen = false; openedByProfile = false; }
        CloseProfileOutsideDen();
    }

    private void ToggleActions()
    {
        if (actionsWindow.InDen) actionsWindow.Toggle();
        else chat.PrintError(DenOnly);
    }

    // Outside the Wolves' Den the PvP Profile closes on its first frame.
    private void CloseProfileOutsideDen()
    {
        if (actionsWindow.InDen) return;
        var profile = (AtkUnitBase*)gameGui.GetAddonByName("PvpProfile", 1).Address;
        if (profile == null || !profile->IsVisible) return;
        profile->Close(true);
        chat.PrintError(DenOnly);
    }

    private void OnProfile(AddonEvent type, AddonArgs args)
    {
        if (type == AddonEvent.PostSetup && actionsWindow.InDen && !actionsWindow.IsOpen && FeastJobActions.Supports(player.ClassJob.RowId))
        { actionsWindow.IsOpen = true; openedByProfile = true; }
        else if (type == AddonEvent.PreFinalize && openedByProfile) { actionsWindow.IsOpen = false; openedByProfile = false; }
    }

    // Sends the loadout chosen in the Wolves' Den once per arena entry,
    // during preparation (the server refuses changes after that).
    private void SyncLoadout(DirectorFrame? frame)
    {
        var self = frame?.Players.FirstOrDefault(p => p.Id == objects.LocalPlayer?.EntityId);
        actionsWindow.EquippedAdditional = self?.Actions ?? [];
        if (!InArena) { loadoutSynced = false; return; }
        if (loadoutSynced || self == null || frame!.Summary.Phase != "prep") return;
        loadoutSynced = true;
        uint job = player.ClassJob.RowId;
        if (!hotbars.Config.Loadouts.TryGetValue(job, out var wanted) || !FeastActionMenu.ValidLoadout(job, wanted)) return;
        var current = actionsWindow.Loadout(job);
        if (!current.Order().SequenceEqual(wanted.Order())) Chat.Send(FeastActionMenu.LoadoutCommand(wanted));
    }

    public void Dispose()
    {
        framework.Update -= Update;
        lifecycle.UnregisterListener(AddonEvent.PostSetup, "PvpProfile", OnProfile);
        lifecycle.UnregisterListener(AddonEvent.PreFinalize, "PvpProfile", OnProfile);
        pi.UiBuilder.Draw -= windows.Draw; pi.UiBuilder.OpenMainUi -= ToggleActions;
        windows.RemoveAllWindows();
        commands.RemoveHandler("/feastactions");
        commands.RemoveHandler("/feasthud");
        respawnPrompt.Dispose();
        namePlateMedals.Dispose();
        resourceGauges.Dispose();
        actionDetail.Dispose();
        native.Dispose();
        hotbars.Dispose();
    }
}
