using System.Numerics;
using Dalamud.Bindings.ImGui;
using Dalamud.Game.Chat;
using Dalamud.Game.Text;
using Dalamud.Game.Text.SeStringHandling;
using Dalamud.Game.Command;
using Dalamud.Plugin;
using Dalamud.Plugin.Services;
using Dalamud.Interface.Textures;
using FFXIVClientStructs.FFXIV.Client.UI;
using FFXIVClientStructs.FFXIV.Client.System.String;
namespace Kawari.FeastHud;

public sealed unsafe class Plugin : IDalamudPlugin
{
    private readonly IDalamudPluginInterface pi;
    private readonly IClientState client;
    private readonly IFramework framework;
    private readonly IChatGui chat;
    private readonly ICommandManager commands;
    private readonly ISharedImmediateTexture banner;
    private Snapshot? state;
    private long received, nextPoll;
    private bool preview;
    private bool InArena => client.IsLoggedIn && client.TerritoryType == 745;
    public Plugin(IDalamudPluginInterface pi, IClientState client, IFramework framework,
        IChatGui chat, ICommandManager commands, ITextureProvider textures)
    {
        this.pi = pi; this.client = client; this.framework = framework; this.chat = chat; this.commands = commands;
        banner = textures.GetFromFile(Path.Combine(pi.AssemblyLocation.DirectoryName!, "assets", "header.png"));
        pi.UiBuilder.Draw += Draw;
        framework.Update += Update;
        chat.ChatMessage += OnChat;
        commands.AddHandler("/feasthud", new CommandInfo(Control) { HelpMessage = "preview: toggle sample HUD; reset: resume live HUD." });
    }
    private void Control(string command, string args) { preview = args.Trim() == "preview" && !preview; state = null; nextPoll = 0; }
    private void Update(IFramework _)
    {
        if (!InArena) { state = null; nextPoll = 0; return; }
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
        if (Snapshot.TryParse(text, out var snapshot)) { state = snapshot; received = Environment.TickCount64; message.PreventOriginal(); }
    }
    private void Draw()
    {
        if (!InArena && !preview) return;
        var s = preview ? new Snapshot("active", 247, 350, 250, "fang", 125, 125, 50) : state;
        var stale = !preview && (s == null || Environment.TickCount64 - received > 4000);
        var scale = Math.Clamp(ImGui.GetIO().DisplaySize.X / 1920f, 0.65f, 1.5f);
        var size = new Vector2(700, 132) * scale;
        ImGui.SetNextWindowPos(new Vector2(ImGui.GetIO().DisplaySize.X / 2, 30 * scale), ImGuiCond.Always, new Vector2(.5f, 0));
        ImGui.SetNextWindowSize(size, ImGuiCond.Always);
        ImGui.SetNextWindowBgAlpha(0);
        if (ImGui.Begin("Feast match HUD", ImGuiWindowFlags.NoDecoration | ImGuiWindowFlags.NoInputs | ImGuiWindowFlags.NoSavedSettings | ImGuiWindowFlags.NoFocusOnAppearing | ImGuiWindowFlags.NoNav))
        {
            var d = ImGui.GetWindowDrawList(); var origin = ImGui.GetWindowPos();
            Vector2 P(float x, float y) => origin + new Vector2(x, y) * scale;
            var white = 0xffeeeeeeu; var muted = 0xffb4b0a8u;
            d.AddRectFilled(P(5, 5), P(695, 125), 0xd91a1713, 4 * scale);
            if (banner.TryGetWrap(out var tex, out _)) d.AddImage(tex.Handle, P(0, 10), P(700, 91));
            d.AddRectFilled(P(287, 22), P(413, 62), 0xe51a1713, 3 * scale);
            d.AddLine(P(40, 94), P(660, 94), 0x405d5a54, scale);
            void Text(string value, float x, float y, float font, uint color) { var sz = font * scale; var width = ImGui.CalcTextSize(value).X * (sz / ImGui.GetFontSize()); d.AddText(ImGui.GetFont(), sz, P(x, y) - new Vector2(width / 2, 0), color, value); }
            Text("THE FANGS", 145, 16, 14, white); Text("THE CLAWS", 555, 16, 14, white);
            Text(stale ? "—" : s!.Fang.ToString(), 145, 37, 32, white); Text(stale ? "—" : s!.Claw.ToString(), 555, 37, 32, white);
            var phase = stale ? "CONNECTING" : s!.Phase switch { "prep" => "PREPARATION", "sudden" => "SUDDEN DEATH", "fang" => "FANGS WIN", "claw" => "CLAWS WIN", "draw" => "DRAW", _ => "THE FEAST" };
            var seconds = s?.Seconds ?? 0;
            if (!preview && !stale && s!.Phase is "prep" or "active" or "sudden") seconds = Math.Max(0, seconds - (int)((Environment.TickCount64 - received) / 1000));
            Text(stale ? "--:--" : $"{seconds / 60:00}:{seconds % 60:00}", 350, 26, 30, white);
            Text(preview ? "PREVIEW  •  " + phase : phase, 350, 67, 13, s?.Phase == "sudden" ? 0xff7bc3ed : muted);
            var detail = stale ? "Waiting for server match state" : s!.Team == "spectator" ? "SPECTATING" : $"{s.Team.ToUpperInvariant()}  ·  YOUR MEDALS  {s.Medals} / {s.Cap}";
            Text(detail, 245, 101, 14, white);
            Text(stale ? "" : $"ON GROUND  {s!.Ground}", 560, 101, 14, muted);
        }
        ImGui.End();
    }
    public void Dispose() { pi.UiBuilder.Draw -= Draw; framework.Update -= Update; chat.ChatMessage -= OnChat; commands.RemoveHandler("/feasthud"); }
}


