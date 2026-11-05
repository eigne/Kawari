

using Dalamud.Game.Chat;
using Dalamud.Game.Text;
using Dalamud.Game.Text.SeStringHandling;
using Dalamud.Game.Command;
using Dalamud.Game.Addon.Lifecycle;
using Dalamud.Game.Addon.Lifecycle.AddonArgTypes;
using Dalamud.Interface.Windowing;
using Dalamud.Plugin;
using Dalamud.Plugin.Services;

using FFXIVClientStructs.FFXIV.Client.UI;
using FFXIVClientStructs.FFXIV.Client.UI.Agent;
using FFXIVClientStructs.FFXIV.Client.System.String;
using FFXIVClientStructs.FFXIV.Component.GUI;
namespace Kawari.FeastHud;

public sealed unsafe class Plugin : IDalamudPlugin
{
    private readonly IClientState client;
    private readonly IFramework framework;
    private readonly IChatGui chat;
    private readonly ICommandManager commands;
    private readonly IGameGui gameGui;
    private readonly IObjectTable objects;
    private readonly NativeDirector native;
    private readonly FeastHotbars hotbars;
    private readonly NativeActionDetail actionDetail;
    private readonly NativeResourceGauges resourceGauges;
    private readonly NativeNamePlateMedals namePlateMedals;
    private readonly IDalamudPluginInterface pi;
    private readonly IAddonLifecycle lifecycle;
    private readonly IPlayerState player;
    private readonly WindowSystem windows = new("Kawari.FeastHud");
    private readonly FeastActionsWindow actionsWindow;
    private bool openedByProfile, loadoutSynced;
    private int respawnMs = -1;
    private long respawnReceived, deadSince;
    private bool wasDead;
    private uint modifiedDialog;
    private string? originalYes;


    private long nextPoll;
    private bool InArena => client.IsLoggedIn && client.TerritoryType == 745;
    public Plugin(IDalamudPluginInterface pi, IClientState client, IFramework framework,
        IChatGui chat, ICommandManager commands, IGameGui gameGui, IObjectTable objects,
        IGameInteropProvider interop, IAddonLifecycle lifecycle, IPluginLog log, IPlayerState player, ITargetManager targets, IDataManager data,
        ITextureProvider textures, INamePlateGui namePlates)
    {
        this.pi = pi; this.lifecycle = lifecycle; this.player = player;
        this.gameGui = gameGui; this.objects = objects;
        this.client = client; this.framework = framework; this.chat = chat; this.commands = commands;
        native = new NativeDirector(client, objects, lifecycle, interop, log, targets, data,
            pi.AssemblyLocation.DirectoryName!);
        try { hotbars = new FeastHotbars(pi, client, player, objects, interop, data); }
        catch { native.Dispose(); throw; }
        try { actionDetail = new NativeActionDetail(lifecycle, client, player, interop); }
        catch { hotbars.Dispose(); native.Dispose(); throw; }
        try { resourceGauges = new NativeResourceGauges(native, objects, lifecycle, interop, log); }
        catch { actionDetail.Dispose(); hotbars.Dispose(); native.Dispose(); throw; }
        namePlateMedals = new NativeNamePlateMedals(namePlates, native);
        actionsWindow = new FeastActionsWindow(client, player, gameGui, textures, data, hotbars);
        windows.AddWindow(actionsWindow);
        pi.UiBuilder.Draw += windows.Draw;
        pi.UiBuilder.OpenMainUi += ToggleActions;
        // The CC-era PvP Profile replaced the Feast PvP Actions window; open it alongside.
        lifecycle.RegisterListener(AddonEvent.PostSetup, "PvpProfile", OnProfile);
        lifecycle.RegisterListener(AddonEvent.PreFinalize, "PvpProfile", OnProfile);
        framework.Update += Update;
        chat.ChatMessage += OnChat;
        commands.AddHandler("/feastactions", new CommandInfo((_, _) => ToggleActions()) { HelpMessage = "Open the Feast PvP Actions window (Wolves' Den only)." });
        commands.AddHandler("/feasthud", new CommandInfo(Control) { HelpMessage = "Native Feast header follows the Crystalline Conflict Progress Gauge HUD position." });
    }
    private const string DenOnly = "PvP actions can only be changed in the Wolves' Den. Use !feast den to travel there.";
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
    private void Send(string text)
    {
        var ui = UIModule.Instance(); if (ui == null) return;
        var message = Utf8String.FromString(text);
        try { ui->ProcessChatBoxEntry(message); } finally { message->Dtor(true); }
    }
    // Applies the loadout chosen outside the arena once per entry, before combat.
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
        if (!current.Order().SequenceEqual(wanted.Order())) Send(FeastActionMenu.LoadoutCommand(wanted));
    }
    private void Control(string command, string args) => chat.Print("Feast uses the native match header. Move or scale it with Crystalline Conflict Progress Gauge in HUD Layout.");
    private void Update(IFramework _)
    {
        native.Update();
        namePlateMedals.Update();
        resourceGauges.Update();
        if (actionsWindow.IsOpen && !actionsWindow.InDen) { actionsWindow.IsOpen = false; openedByProfile = false; }
        CloseProfileOutsideDen();
        hotbars.Update(Environment.TickCount64 - native.Received < 2500 ? native.Frame : null);
        SyncLoadout(Environment.TickCount64 - native.Received < 2500 ? native.Frame : null);
        if (native.Frame is { } frame)
        {
            respawnMs = frame.RespawnSeconds * 1000; respawnReceived = native.Received;
        }
        UpdateRespawnPrompt();
        if (!InArena) { nextPoll = 0; return; }
        if (native.Frame != null) return;
        if (Environment.TickCount64 < nextPoll) return;
        nextPoll = Environment.TickCount64 + 1000;
        Send("!feast hud");
    }
    private void OnChat(IHandleableChatMessage message)
    {
        if (!InArena) return;
        var text = message.Message.TextValue;
        if (text == "!feast hud") { message.PreventOriginal(); return; }
        // Only accept system notices, never player-authored chat or arbitrary embedded strings.
        if (message.Sender.TextValue.Length != 0) return;
        if (text.StartsWith("KFEASTRESPAWN1|") && int.TryParse(text[15..], out var ms) && ms >= 0 && ms <= 10000)
        { respawnMs = ms; respawnReceived = Environment.TickCount64; message.PreventOriginal(); return; }
        if (Snapshot.TryParse(text, out _)) message.PreventOriginal();
    }
    private void UpdateRespawnPrompt()
    {
        var dead = InArena && objects.LocalPlayer is { CurrentHp: 0 };
        if (dead && !wasDead) { deadSince = Environment.TickCount64; respawnMs = -1; nextPoll = 0; }
        wasDead = dead;
        var agent = AgentRevive.Instance();
        var addon = (AddonSelectYesno*)gameGui.GetAddonByName("SelectYesno", 1).Address;
        // Only alter the confirmation owned by the revive agent, never another Yes/No dialog.
        if (addon == null || agent == null || addon->Id != agent->GetAddonId())
        { modifiedDialog = 0; originalYes = null; if (!dead) respawnMs = -1; return; }
        if (!dead)
        {
            if (modifiedDialog == addon->Id && addon->YesButton != null)
            { addon->YesButton->SetText(originalYes ?? "OK"); addon->YesButton->SetEnabledState(true); }
            modifiedDialog = 0; originalYes = null; respawnMs = -1; return;
        }
        if (addon->YesButton == null) return;
        if (modifiedDialog != addon->Id)
        { modifiedDialog = addon->Id; originalYes = addon->YesButton->ButtonTextNode == null ? "OK" : addon->YesButton->ButtonTextNode->NodeText.ToString(); }
        var age = Environment.TickCount64 - respawnReceived;
        // Only the server's zero releases the button; interpolation is display-only.
        var ready = respawnMs == 0 && age < 2500 && Environment.TickCount64 - deadSince >= 10000;
        var seconds = respawnMs < 0 ? 10 : Math.Max(1, (int)Math.Ceiling((respawnMs - age) / 1000.0));
        addon->YesButton->SetEnabledState(ready);
        addon->YesButton->SetText(ready ? originalYes ?? "OK" : $"OK ({seconds}s)");
    }

    public void Dispose() {
        lifecycle.UnregisterListener(AddonEvent.PostSetup, "PvpProfile", OnProfile);
        lifecycle.UnregisterListener(AddonEvent.PreFinalize, "PvpProfile", OnProfile);
        pi.UiBuilder.Draw -= windows.Draw; pi.UiBuilder.OpenMainUi -= ToggleActions;
        windows.RemoveAllWindows(); commands.RemoveHandler("/feastactions");
        namePlateMedals.Dispose();
        resourceGauges.Dispose();
        actionDetail.Dispose();
        native.Dispose();
        hotbars.Dispose();
        var addon = (AddonSelectYesno*)gameGui.GetAddonByName("SelectYesno", 1).Address;
        var agent = AgentRevive.Instance();
        if (addon != null && agent != null && modifiedDialog != 0 && addon->Id == modifiedDialog
            && addon->Id == agent->GetAddonId() && addon->YesButton != null)
        { addon->YesButton->SetText(originalYes ?? "OK"); addon->YesButton->SetEnabledState(true); }
        framework.Update -= Update; chat.ChatMessage -= OnChat; commands.RemoveHandler("/feasthud"); }
}


