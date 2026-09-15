using Dalamud.Configuration;
using Dalamud.Game.Command;
using Dalamud.Hooking;
using Dalamud.Plugin;
using Dalamud.Plugin.Services;
using FFXIVClientStructs.FFXIV.Client.System.String;
using FFXIVClientStructs.FFXIV.Client.UI;
using FFXIVClientStructs.FFXIV.Client.UI.Misc;
using static FFXIVClientStructs.FFXIV.Client.UI.Misc.RaptureHotbarModule;

namespace Kawari.Feast;

public sealed class Configuration : IPluginConfiguration
{
    public int Version { get; set; } = 1;
    public ulong Character { get; set; }
    public List<SavedSlot> Backup { get; set; } = [];
}

public sealed record SavedSlot(uint Bar, uint Slot, HotbarSlotType Type, uint Id);

public sealed unsafe class Plugin : IDalamudPlugin
{
    // Current-client icons only. The server executes the historical Feast action.
    private static readonly (uint IconAction, string Command)[] Actions = [
        (16533, "glare"), (16532, "dia"), (120, "cure"), (135, "cure2"),
        (124, "medica"), (16531, "solace"), (16534, "rapture"), (16535, "misery"),
        (3570, "tetra"), (3571, "assize"), (16536, "temperance"), (7568, "purify"),
        (3, "bolt"), (140, "kit"), (7432, "protect"), (25861, "largesse"),
        (16560, "repose"), (7571, "rescue"), (7433, "attunement"), (208, "rain")
    ];
    private readonly IDalamudPluginInterface pi;
    private readonly IClientState client;
    private readonly IObjectTable objects;
    private readonly IPlayerState player;
    private readonly IFramework framework;
    private readonly IChatGui chat;
    private readonly ICommandManager commands;
    private readonly IPluginLog log;
    private readonly Configuration config;
    private readonly Hook<ExecuteSlotDelegate> execute;
    private bool active;
    private bool paused;
    private ulong character;
    private long nextCheck;
    private long nextClick;
    private delegate byte ExecuteSlotDelegate(RaptureHotbarModule* module, HotbarSlot* slot);

    public Plugin(IDalamudPluginInterface pi, IClientState client, IObjectTable objects,
        IPlayerState player, IFramework framework, IChatGui chat, ICommandManager commands,
        IGameInteropProvider interop, IPluginLog log)
    {
        this.pi = pi; this.client = client; this.objects = objects; this.player = player;
        this.framework = framework; this.chat = chat; this.commands = commands; this.log = log;
        config = pi.GetPluginConfig() as Configuration ?? new Configuration();
        execute = interop.HookFromAddress<ExecuteSlotDelegate>(
            (nint)MemberFunctionPointers.ExecuteSlot, ExecuteSlot);
        execute.Enable();
        framework.Update += Update;
        commands.AddHandler("/kfeast", new CommandInfo(Control) {
            HelpMessage = "status | restore (pause) | on. Feast slots automatically load in territory 745 as WHM."
        });
    }

    private bool Eligible => client.IsLoggedIn && client.TerritoryType == 745
        && objects.LocalPlayer?.ClassJob.RowId == 24 && player.ContentId != 0;

    private void Update(IFramework _)
    {
        if (Environment.TickCount64 < nextCheck) return;
        nextCheck = Environment.TickCount64 + 250;
        try
        {
            if (active && (!Eligible || character != player.ContentId || paused)) Restore();
            if (!active && Eligible && !paused) Enter();
        }
        catch (Exception error)
        {
            paused = true;
            log.Error(error, "Feast hotbar update failed; paused to preserve the original layout.");
            chat.PrintError("Kawari Feast paused. Use /kfeast restore to restore your hotbars.");
        }
    }

    private void Enter()
    {
        var module = RaptureHotbarModule.Instance();
        if (module == null || !module->ModuleReady || module->ActiveHotbarClassJobId != 24) return;
        // A previous session can leave a recovery record. Restore before taking a new snapshot.
        Recover(module);
        var backup = new List<SavedSlot>();
        for (var i = 0; i < Actions.Length; i++)
        {
            var slot = module->GetSlotById((uint)i / 12, (uint)i % 12);
            if (slot == null) return;
            backup.Add(new((uint)i / 12, (uint)i % 12, slot->CommandType, slot->CommandId));
        }
        config.Character = character = player.ContentId;
        config.Backup = backup;
        pi.SavePluginConfig(config); // Recovery exists before the first mutation.
        active = true;
        for (var i = 0; i < Actions.Length; i++)
            module->GetSlotById((uint)i / 12, (uint)i % 12)->Set(HotbarSlotType.Action, Actions[i].IconAction);
        // Set() changes live slots only; SetAndSaveSlot would overwrite HOTBAR.DAT.
        Send("!whm on");
        chat.Print("Kawari Feast: hotbars 1–2 loaded. Buttons/keybinds cast Feast spells. /kfeast status shows the mapping; /kfeast restore restores your layout.");
    }

    private void Recover(RaptureHotbarModule* module)
    {
        if (config.Character != player.ContentId || objects.LocalPlayer?.ClassJob.RowId != 24) return;
        for (var i = 0; i < config.Backup.Count && i < Actions.Length; i++)
        {
            var old = config.Backup[i];
            // Only restore owned slots, never overwrite a newly loaded job or user-edited slot.
            if (old.Bar != (uint)i / 12 || old.Slot != (uint)i % 12) continue;
            var slot = module->GetSlotById(old.Bar, old.Slot);
            if (slot != null && slot->CommandType == HotbarSlotType.Action && slot->CommandId == Actions[i].IconAction)
                slot->Set(old.Type, old.Id);
        }
        config.Backup.Clear();
        pi.SavePluginConfig(config);
    }

    private void Restore()
    {
        active = false;
        var module = RaptureHotbarModule.Instance();
        if (module != null && client.IsLoggedIn) Recover(module);
        // After logout or a class change the client loads its saved layout itself.
    }

    private byte ExecuteSlot(RaptureHotbarModule* module, HotbarSlot* slot)
    {
        if (!active || !Eligible || character != player.ContentId || slot == null)
            return execute.Original(module, slot);
        for (var i = 0; i < Actions.Length; i++)
        {
            // Intercept owned buttons only. Other bars and non-Feast actions retain normal handling.
            if (slot != module->GetSlotById((uint)i / 12, (uint)i % 12)
                || slot->CommandType != HotbarSlotType.Action || slot->CommandId != Actions[i].IconAction) continue;
            try
            {
                if (Environment.TickCount64 >= nextClick)
                {
                    nextClick = Environment.TickCount64 + 100;
                    Send("!whm " + Actions[i].Command);
                }
            }
            catch (Exception error) { log.Error(error, "Feast button failed"); }
            return 1;
        }
        return execute.Original(module, slot);
    }

    private static void Send(string text)
    {
        var ui = UIModule.Instance();
        if (ui == null) return;
        var message = Utf8String.FromString(text);
        try { ui->ProcessChatBoxEntry(message); }
        finally { message->Dtor(true); }
    }

    private void Control(string _, string args)
    {
        switch (args.Trim().ToLowerInvariant())
        {
            case "restore": paused = true; Restore(); if (Eligible) Send("!whm off"); break;
            case "on": paused = false; nextCheck = 0; break;
            default:
                for (var i = 0; i < Actions.Length; i++)
                    chat.Print($"Feast bar {i / 12 + 1}, slot {i % 12 + 1}: {Actions[i].Command}");
                if (Eligible) Send("!whm status");
                break;
        }
    }

    public void Dispose()
    {
        framework.Update -= Update;
        commands.RemoveHandler("/kfeast");
        execute.Disable();
        Restore();
        execute.Dispose();
    }
}
