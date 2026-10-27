

using Dalamud.Game.Chat;
using Dalamud.Game.Text;
using Dalamud.Game.Text.SeStringHandling;
using Dalamud.Game.Command;
using Dalamud.Plugin;
using Dalamud.Plugin.Services;

using FFXIVClientStructs.FFXIV.Client.UI;
using FFXIVClientStructs.FFXIV.Client.UI.Agent;
using FFXIVClientStructs.FFXIV.Client.System.String;
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
    private int respawnMs = -1;
    private long respawnReceived, deadSince;
    private bool wasDead;
    private uint modifiedDialog;
    private string? originalYes;


    private long nextPoll;
    private bool InArena => client.IsLoggedIn && client.TerritoryType == 745;
    public Plugin(IDalamudPluginInterface pi, IClientState client, IFramework framework,
        IChatGui chat, ICommandManager commands, IGameGui gameGui, IObjectTable objects,
        IGameInteropProvider interop, IAddonLifecycle lifecycle, IPluginLog log, IPlayerState player, ITargetManager targets, IDataManager data)
    {
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
        framework.Update += Update;
        chat.ChatMessage += OnChat;
        commands.AddHandler("/feasthud", new CommandInfo(Control) { HelpMessage = "Native Feast header follows the Crystalline Conflict Progress Gauge HUD position." });
    }
    private void Control(string command, string args) => chat.Print("Feast uses the native match header. Move or scale it with Crystalline Conflict Progress Gauge in HUD Layout.");
    private void Update(IFramework _)
    {
        native.Update();
        resourceGauges.Update();
        hotbars.Update(Environment.TickCount64 - native.Received < 2500 ? native.Frame : null);
        if (native.Frame is { } frame)
        {
            respawnMs = frame.RespawnSeconds * 1000; respawnReceived = native.Received;
        }
        UpdateRespawnPrompt();
        if (!InArena) { nextPoll = 0; return; }
        if (native.Frame != null) return;
        if (Environment.TickCount64 < nextPoll) return;
        nextPoll = Environment.TickCount64 + 1000;
        var ui = UIModule.Instance(); if (ui == null) return;
        var message = Utf8String.FromString("!feast hud");
        try { ui->ProcessChatBoxEntry(message); } finally { message->Dtor(true); }
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


