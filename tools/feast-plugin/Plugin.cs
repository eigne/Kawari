using Dalamud.Configuration;
using Dalamud.Game.Command;
using Dalamud.Hooking;
using Dalamud.Plugin;
using Dalamud.Plugin.Services;
using FFXIVClientStructs.FFXIV.Client.Game;
using FFXIVClientStructs.FFXIV.Client.Game.Character;
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
    private readonly IDataManager data;
    private readonly Configuration config;
    private readonly Hook<ExecuteSlotDelegate> execute;
    private readonly Hook<OpenCastBarDelegate> openCastBar;
    private bool nativeCastingSeen;
    private nint castingActor;
    private ushort castingTimeline;
    private long castingEnds;
    private System.Numerics.Vector3 castingOrigin;
    private bool active;
    private bool paused;
    private ulong character;
    private long nextCheck;
    private long nextClick;
    private delegate byte ExecuteSlotDelegate(RaptureHotbarModule* module, HotbarSlot* slot);
    private delegate void OpenCastBarDelegate(ActionManager* manager, BattleChara* actor,
        ActionType type, uint actionId, uint spellId, uint extraParam, float elapsed, float total);

    public Plugin(IDalamudPluginInterface pi, IClientState client, IObjectTable objects,
        IPlayerState player, IFramework framework, IChatGui chat, ICommandManager commands,
        IGameInteropProvider interop, IPluginLog log, IDataManager data)
    {
        this.pi = pi; this.client = client; this.objects = objects; this.player = player;
        this.framework = framework; this.chat = chat; this.commands = commands; this.log = log;
        this.data = data;
        config = pi.GetPluginConfig() as Configuration ?? new Configuration();
        execute = interop.HookFromAddress<ExecuteSlotDelegate>(
            (nint)MemberFunctionPointers.ExecuteSlot, ExecuteSlot);
        openCastBar = interop.HookFromAddress<OpenCastBarDelegate>(
            (nint)ActionManager.MemberFunctionPointers.OpenCastBar, OpenCastBar);
        execute.Enable();
        openCastBar.Enable();
        framework.Update += Update;
        commands.AddHandler("/kfeast", new CommandInfo(Control) {
            HelpMessage = "status | restore (pause) | on. Feast slots automatically load in territory 745 as WHM."
        });
    }

    private bool Eligible => client.IsLoggedIn && client.TerritoryType == 745
        && objects.LocalPlayer?.ClassJob.RowId == 24 && player.ContentId != 0;

    private void Update(IFramework _)
    {
        // Pose cleanup runs every frame, independently of the hotbar polling interval.
        if (castingActor != 0)
        {
            var local = objects.LocalPlayer;
            var sameActor = local != null && local.Address == castingActor;
            var nativeCasting = sameActor && ((BattleChara*)castingActor)->CastInfo.IsCasting;
            if (!active || !Eligible || local == null || local.Address != castingActor
                || Environment.TickCount64 >= castingEnds
                || (nativeCastingSeen && !nativeCasting)
                || System.Numerics.Vector3.DistanceSquared(local.Position, castingOrigin) > 0.01f)
                StopCastingPose();
            else nativeCastingSeen |= nativeCasting;
        }
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

    private void OpenCastBar(ActionManager* manager, BattleChara* actor, ActionType type,
        uint actionId, uint spellId, uint extraParam, float elapsed, float total)
    {
        openCastBar.Original(manager, actor, type, actionId, spellId, extraParam, elapsed, total);
        // ActorCast opens the bar, but command-driven local casts never ran UseAction's
        // animation-start path. Repair presentation only after the server accepted a cast.
        if (!active || !Eligible || actor == null || (nint)actor != objects.LocalPlayer?.Address
            || type != ActionType.Action || total <= elapsed
            || actionId is not (120 or 135 or 124 or 16533)) return;
        try
        {
            var action = data.GetExcelSheet<Lumina.Excel.Sheets.Action>().GetRow(actionId);
            var timeline = checked((ushort)action.AnimationStart.Value.Name.RowId);
            if (timeline == 0) return;
            StopCastingPose();
            castingActor = (nint)actor;
            castingTimeline = timeline;
            castingOrigin = objects.LocalPlayer!.Position;
            nativeCastingSeen = actor->CastInfo.IsCasting;
            castingEnds = Environment.TickCount64 + (long)Math.Ceiling((total - elapsed) * 1000);
            actor->Timeline.PlayActionTimeline(timeline);
            log.Debug("Feast cast pose: action {Action}, timeline {Timeline}, duration {Duration}",
                actionId, timeline, total - elapsed);
        }
        catch (Exception error) { log.Error(error, "Could not start Feast casting pose"); }
    }

    private void StopCastingPose()
    {
        var address = castingActor;
        var timeline = castingTimeline;
        castingActor = 0;
        castingTimeline = 0;
        nativeCastingSeen = false;
        if (address == 0 || objects.LocalPlayer?.Address != address) return;
        var actor = (BattleChara*)address;
        // A completed ActionResult replaces the casting timeline with the release.
        // Never reset that release, even if movement/cancellation follows completion.
        for (uint slot = 0; slot < 14; slot++)
            if (actor->Timeline.TimelineSequencer.GetSlotTimeline(slot) == timeline)
                actor->Timeline.TimelineSequencer.SetSlotTimeline(slot, 0);
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
        StopCastingPose();
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
        openCastBar.Disable();
        Restore();
        execute.Dispose();
        openCastBar.Dispose();
    }
}
