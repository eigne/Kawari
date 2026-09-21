using System.Numerics;
using Dalamud.Bindings.ImGui;
using Dalamud.Game.Chat;
using Dalamud.Game.Text;
using Dalamud.Game.Text.SeStringHandling;
using Dalamud.Game.Command;
using Dalamud.Plugin;
using Dalamud.Plugin.Services;
using Dalamud.Interface.ManagedFontAtlas;
using FFXIVClientStructs.FFXIV.Client.UI;
using FFXIVClientStructs.FFXIV.Client.UI.Agent;
using FFXIVClientStructs.FFXIV.Client.System.String;
namespace Kawari.FeastHud;

public sealed unsafe class Plugin : IDalamudPlugin
{
    private readonly IDalamudPluginInterface pi;
    private readonly IClientState client;
    private readonly IFramework framework;
    private readonly IChatGui chat;
    private readonly ICommandManager commands;
    private readonly IGameGui gameGui;
    private readonly IObjectTable objects;
    private readonly NativeDirector native;
    private readonly FeastHotbars hotbars;
    private int respawnMs = -1;
    private long respawnReceived, deadSince;
    private bool wasDead;
    private uint modifiedDialog;
    private string? originalYes;
    private readonly IFontHandle labels;
    private readonly IFontHandle numerals;
    private Snapshot? state;
    private long received, nextPoll;
    private bool preview;
    private bool InArena => client.IsLoggedIn && client.TerritoryType == 745;
    public Plugin(IDalamudPluginInterface pi, IClientState client, IFramework framework,
        IChatGui chat, ICommandManager commands, IGameGui gameGui, IObjectTable objects,
        IGameInteropProvider interop, IAddonLifecycle lifecycle, IPluginLog log, IPlayerState player, ITargetManager targets, IDataManager data)
    {
        this.gameGui = gameGui; this.objects = objects;
        this.pi = pi; this.client = client; this.framework = framework; this.chat = chat; this.commands = commands;
        native = new NativeDirector(client, objects, lifecycle, interop, log, targets, data);
        try { hotbars = new FeastHotbars(pi, client, player, objects, interop); }
        catch { native.Dispose(); throw; }
        // Bahnschrift is Windows' DIN-derived UI face. Bake real font glyphs rather
        // than magnifying ImGui's small default font.
        var fontPath = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.Fonts), "bahnschrift.ttf");
        labels = pi.UiBuilder.FontAtlas.NewDelegateFontHandle(e => e.OnPreBuild(tk =>
            tk.AddFontFromFile(fontPath, new SafeFontConfig { SizePx = 24, OversampleH = 2, OversampleV = 2 })));
        numerals = pi.UiBuilder.FontAtlas.NewDelegateFontHandle(e => e.OnPreBuild(tk =>
            tk.AddFontFromFile(fontPath, new SafeFontConfig { SizePx = 54, OversampleH = 2, OversampleV = 2 })));
        pi.UiBuilder.Draw += Draw;
        framework.Update += Update;
        chat.ChatMessage += OnChat;
        commands.AddHandler("/feasthud", new CommandInfo(Control) { HelpMessage = "preview: toggle sample HUD; reset: resume live HUD." });
    }
    private void Control(string command, string args) { preview = args.Trim() == "preview" && !preview; state = null; nextPoll = 0; }
    private void Update(IFramework _)
    {
        native.Update();
        hotbars.Update();
        if (native.Frame is { } frame)
        {
            var participant = frame.Players.FirstOrDefault(p => p.Id == objects.LocalPlayer?.EntityId);
            state = frame.Summary with { Cap = participant?.MedalCapacity ?? 0 }; received = native.Received;
            respawnMs = frame.RespawnSeconds * 1000; respawnReceived = native.Received;
        }
        UpdateRespawnPrompt();
        if (!InArena) { state = null; nextPoll = 0; return; }
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
        if (Snapshot.TryParse(text, out var snapshot)) { state = snapshot; received = Environment.TickCount64; message.PreventOriginal(); }
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

    private void Draw()
    {
        if (!InArena && !preview) return;
        var s = preview ? new Snapshot("active", 247, 350, 250, "fang", 125, 125, 50) : state;
        var stale = !preview && (s == null || Environment.TickCount64 - received > 4000);
        var scale = Math.Clamp(ImGui.GetIO().DisplaySize.X / 1920f, 0.8f, 1.5f);
        ImGui.SetNextWindowPos(new Vector2(ImGui.GetIO().DisplaySize.X / 2, 24 * scale), ImGuiCond.Always, new Vector2(.5f, 0));
        ImGui.SetNextWindowSize(new Vector2(620, 108) * scale, ImGuiCond.Always);
        ImGui.SetNextWindowBgAlpha(0);
        if (ImGui.Begin("Feast match HUD", ImGuiWindowFlags.NoDecoration | ImGuiWindowFlags.NoInputs | ImGuiWindowFlags.NoSavedSettings | ImGuiWindowFlags.NoFocusOnAppearing | ImGuiWindowFlags.NoNav))
        {
            var d = ImGui.GetWindowDrawList();
            var origin = ImGui.GetWindowPos();
            Vector2 P(float x, float y) => origin + new Vector2(x, y) * scale;
            const uint ink = 0xffebe9e2, muted = 0xffa99f90, edge = 0x805d5549, accent = 0xffc4d0ac;
            d.AddRectFilled(P(0, 0), P(620, 104), 0xe6191511);
            d.AddLine(P(0, 0), P(620, 0), edge, scale);
            d.AddLine(P(0, 104), P(620, 104), edge, scale);
            foreach (var x in new float[] { 0, 620 })
            {
                d.AddLine(P(x, 0), P(x, 9), ink, scale);
                d.AddLine(P(x, 95), P(x, 104), edge, scale);
            }
            d.AddLine(P(210, 14), P(210, 63), edge, scale);
            d.AddLine(P(410, 14), P(410, 63), edge, scale);
            d.AddLine(P(20, 77), P(600, 77), edge, scale);

            void Text(string value, float x, float y, float size, uint color, bool number = false, float tracking = 0)
            {
                using var fontScope = (number ? numerals : labels).Push();
                var font = ImGui.GetFont();
                float Measure(string t) => ImGui.CalcTextSize(t).X * size * scale / ImGui.GetFontSize();
                var width = Measure(value) + Math.Max(0, value.Length - 1) * tracking * scale;
                var cursor = P(x, y) - new Vector2(width / 2, 0);
                if (tracking == 0) d.AddText(font, size * scale, cursor, color, value);
                else foreach (var c in value)
                {
                    var glyph = c.ToString();
                    d.AddText(font, size * scale, cursor, color, glyph);
                    cursor.X += Measure(glyph) + tracking * scale;
                }
            }
            var phase = stale ? "NO SIGNAL" : s!.Phase switch
            {
                "prep" => "PREPARATION",
                "sudden" => "SUDDEN DEATH",
                "fang" => "FANGS WIN",
                "claw" => "CLAWS WIN",
                "draw" => "DRAW",
                _ => "MATCH"
            };
            Text("FANGS", 105, 12, 12, s?.Team == "fang" ? accent : muted, tracking: 2);
            Text("CLAWS", 515, 12, 12, s?.Team == "claw" ? accent : muted, tracking: 2);
            Text(preview ? "PREVIEW" : phase, 310, 10, 10, s?.Phase == "sudden" ? 0xff8fb5dc : muted, tracking: 1.7f);
            Text(stale ? "--" : s!.Fang.ToString(), 105, 29, 32, ink, true);
            Text(stale ? "--" : s!.Claw.ToString(), 515, 29, 32, ink, true);
            var seconds = s?.Seconds ?? 0;
            if (!preview && !stale && s!.Phase is "prep" or "active" or "sudden")
                seconds = Math.Max(0, seconds - (int)((Environment.TickCount64 - received) / 1000));
            Text(stale ? "--:--" : $"{seconds / 60:00}:{seconds % 60:00}", 310, 28, 34, ink, true);
            // Each rail is that team's share of the medals currently being carried.
            if (!stale)
            {
                var total = Math.Max(1, s!.Fang + s.Claw);
                d.AddRectFilled(P(30, 68), P(180, 70), 0xff39332b);
                d.AddRectFilled(P(440, 68), P(590, 70), 0xff39332b);
                d.AddRectFilled(P(30, 68), P(30 + 150f * s.Fang / total, 70), ink);
                d.AddRectFilled(P(590 - 150f * s.Claw / total, 68), P(590, 70), ink);
            }
            string Roster(byte team) => string.Join("   ", (native.Frame?.Players ?? [])
                .Where(p => p.Team == team).Take(4).Select(p => $"{p.JobAbbreviation} {p.Medals}"));
            Text(stale ? "WAITING FOR MATCH DATA" : preview ? "WHM 125   PLD 100   MNK 50   SMN 75" : Roster(1),
                155, 87, 11, muted);
            Text(stale ? "" : preview ? "WHM 100   PLD 125   SAM 100   BRD 75" : Roster(2),
                465, 87, 11, muted);
        }
        ImGui.End();
    }
    public void Dispose() {
        native.Dispose();
        hotbars.Dispose();
        var addon = (AddonSelectYesno*)gameGui.GetAddonByName("SelectYesno", 1).Address;
        var agent = AgentRevive.Instance();
        if (addon != null && agent != null && modifiedDialog != 0 && addon->Id == modifiedDialog
            && addon->Id == agent->GetAddonId() && addon->YesButton != null)
        { addon->YesButton->SetText(originalYes ?? "OK"); addon->YesButton->SetEnabledState(true); }
        pi.UiBuilder.Draw -= Draw; framework.Update -= Update; chat.ChatMessage -= OnChat; commands.RemoveHandler("/feasthud"); labels.Dispose(); numerals.Dispose(); }
}


