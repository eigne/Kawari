using Dalamud.Game.Chat;
using Dalamud.Plugin.Services;
using FFXIVClientStructs.FFXIV.Client.UI;
using FFXIVClientStructs.FFXIV.Client.UI.Agent;

namespace Kawari.FeastHud;

// A KO'd Feast player must wait ten seconds before returning. The server
// refuses early returns on its own; this only shows the wait on the native
// "Return" confirmation as "OK (7s)" with the button disabled until then.
//
// The remaining time comes from the director summary (F1). Without director
// frames (exploration mode) it polls `!feast hud` once a second, whose
// private KFEASTRESPAWN1|<ms> reply is hidden from chat.
internal sealed unsafe class RespawnPrompt : IDisposable
{
    private const string Reply = "KFEASTRESPAWN1|";
    private readonly IGameGui gameGui;
    private readonly IObjectTable objects;
    private readonly IChatGui chat;
    private readonly NativeDirector director;
    private int respawnMs = -1;
    private long respawnReceived, deadSince, nextPoll;
    private bool wasDead;
    private uint modifiedDialog;
    private string? originalYes;

    public RespawnPrompt(IGameGui gameGui, IObjectTable objects, IChatGui chat, NativeDirector director)
    {
        this.gameGui = gameGui; this.objects = objects; this.chat = chat; this.director = director;
        chat.ChatMessage += OnChat;
    }

    public void Update()
    {
        if (director.Frame is { } frame)
        {
            respawnMs = frame.RespawnSeconds * 1000;
            respawnReceived = director.Received;
        }
        else if (director.InArena && Environment.TickCount64 >= nextPoll)
        {
            nextPoll = Environment.TickCount64 + 1000;
            Chat.Send("!feast hud");
        }
        UpdateDialog();
    }

    private void OnChat(IHandleableChatMessage message)
    {
        if (!director.InArena) return;
        var text = message.Message.TextValue;
        if (text == "!feast hud") { message.PreventOriginal(); return; }
        // Only accept system notices, never player-authored chat.
        if (message.Sender.TextValue.Length != 0 || !text.StartsWith(Reply)) return;
        if (int.TryParse(text[Reply.Length..], out var ms) && ms is >= 0 and <= 10000)
        { respawnMs = ms; respawnReceived = Environment.TickCount64; }
        message.PreventOriginal();
    }

    private void UpdateDialog()
    {
        var dead = director.InArena && objects.LocalPlayer is { CurrentHp: 0 };
        if (dead && !wasDead) { deadSince = Environment.TickCount64; respawnMs = -1; nextPoll = 0; }
        wasDead = dead;
        var agent = AgentRevive.Instance();
        var addon = (AddonSelectYesno*)gameGui.GetAddonByName("SelectYesno", 1).Address;
        // Only alter the confirmation owned by the revive agent, never another Yes/No dialog.
        if (addon == null || agent == null || addon->Id != agent->GetAddonId())
        { modifiedDialog = 0; originalYes = null; if (!dead) respawnMs = -1; return; }
        if (!dead) { RestoreDialog(addon); respawnMs = -1; return; }
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

    private void RestoreDialog(AddonSelectYesno* addon)
    {
        if (modifiedDialog == addon->Id && addon->YesButton != null)
        { addon->YesButton->SetText(originalYes ?? "OK"); addon->YesButton->SetEnabledState(true); }
        modifiedDialog = 0; originalYes = null;
    }

    public void Dispose()
    {
        chat.ChatMessage -= OnChat;
        var addon = (AddonSelectYesno*)gameGui.GetAddonByName("SelectYesno", 1).Address;
        var agent = AgentRevive.Instance();
        if (addon != null && agent != null && addon->Id == agent->GetAddonId()) RestoreDialog(addon);
    }
}
