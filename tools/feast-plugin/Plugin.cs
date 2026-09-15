using Dalamud.Configuration;
using Dalamud.Game.Command;
using Dalamud.Hooking;
using Dalamud.Plugin;
using Dalamud.Plugin.Services;
using FFXIVClientStructs.FFXIV.Client.Game;
using FFXIVClientStructs.FFXIV.Client.Game.Character;
using FFXIVClientStructs.FFXIV.Client.Game.Network;
using FFXIVClientStructs.FFXIV.Client.Network;
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
    private readonly CastEffects castEffects;
    private readonly Configuration config;
    private readonly Hook<ExecuteSlotDelegate> execute;
    private readonly Hook<ActorCastDelegate> actorCast;
    private readonly Hook<ActionEffectDelegate> actionEffect;
    private nint castingActor;
    private ushort castingTimeline;
    private ushort castingLoop;
    private bool loopStarted;
    private ushort previousBaseOverride;
    private uint castingAction;
    private readonly Dictionary<ushort, ushort> castLoops = [];
    private uint lastCastingSlots;
    private long castingEnds;
    private System.Numerics.Vector3 castingOrigin;
    private bool active;
    private bool paused;
    private ulong character;
    private long nextCheck;
    private long nextClick;
    private delegate byte ExecuteSlotDelegate(RaptureHotbarModule* module, HotbarSlot* slot);
    private delegate void ActorCastDelegate(uint entityId, ActorCastPacket* packet);
    private delegate void ActionEffectDelegate(uint entityId, Character* actor,
        System.Numerics.Vector3* position, ActionEffectHandler.Header* header,
        ActionEffectHandler.TargetEffects* effects,
        FFXIVClientStructs.FFXIV.Client.Game.Object.GameObjectId* targets);

    public Plugin(IDalamudPluginInterface pi, IClientState client, IObjectTable objects,
        IPlayerState player, IFramework framework, IChatGui chat, ICommandManager commands,
        IGameInteropProvider interop, IPluginLog log, IDataManager data, ISigScanner scanner)
    {
        this.pi = pi; this.client = client; this.objects = objects; this.player = player;
        this.framework = framework; this.chat = chat; this.commands = commands; this.log = log;
        this.data = data;
        config = pi.GetPluginConfig() as Configuration ?? new Configuration();
        castEffects = new CastEffects(scanner, interop, log);
        execute = interop.HookFromAddress<ExecuteSlotDelegate>(
            (nint)MemberFunctionPointers.ExecuteSlot, ExecuteSlot);
        actorCast = interop.HookFromAddress<ActorCastDelegate>(
            (nint)PacketDispatcher.MemberFunctionPointers.HandleActorCastPacket, ActorCast);
        actionEffect = interop.HookFromAddress<ActionEffectDelegate>(
            (nint)ActionEffectHandler.MemberFunctionPointers.Receive, ActionEffect);
        execute.Enable();
        actorCast.Enable();
        actionEffect.Enable();
        log.Information("Feast casting presentation v7 loaded: casting VFX fade smoothly into release.");
        framework.Update += Update;
        commands.AddHandler("/kfeast", new CommandInfo(Control) {
            HelpMessage = "status | restore (pause) | on. Feast slots automatically load in territory 745 as WHM."
        });
    }

    private bool Eligible => client.IsLoggedIn && client.TerritoryType == 745
        && objects.LocalPlayer?.ClassJob.RowId == 24 && player.ContentId != 0;

    private void Update(IFramework _)
    {
        castEffects.Update();
        // Pose cleanup runs every frame, independently of the hotbar polling interval.
        if (castingActor != 0)
        {
            var local = objects.LocalPlayer;
            var now = Environment.TickCount64;
            if (!active || !Eligible || local == null || local.Address != castingActor
                || local.CurrentHp == 0 || now >= castingEnds + 2000
                // After the cast bar completes, wait for the authoritative release.
                // Ending the pose at the local deadline exposes a frame of idle while
                // the server tick/network response is still in flight.
                || (now < castingEnds && System.Numerics.Vector3.DistanceSquared(local.Position, castingOrigin) > 0.01f))
                StopCastingPose();
            else
            {
                var actor = (BattleChara*)castingActor;
                var baseId = actor->Timeline.TimelineSequencer.GetSlotTimeline(0);
                var upperId = actor->Timeline.TimelineSequencer.GetSlotTimeline(1);
                // On this client command casts finish their intro into normal/idle (row 3).
                // They do not have UseAction's native casting state to sustain the loop.
                if (!loopStarted && baseId == 3 && upperId == 0)
                {
                    previousBaseOverride = actor->Timeline.BaseOverride;
                    loopStarted = true;
                    actor->Timeline.BaseOverride = castingLoop;
                    actor->Timeline.TimelineSequencer.PlayTimeline(castingLoop);
                    baseId = actor->Timeline.TimelineSequencer.GetSlotTimeline(0);
                    log.Information("Feast sustained loop started: loop={Loop}, base={Base}", castingLoop, baseId);
                }
                var slots = (uint)baseId | ((uint)upperId << 16);
                if (slots != lastCastingSlots)
                {
                    lastCastingSlots = slots;
                    log.Information("Feast casting transition: base={Base}, upper={Upper}, expected loop={Loop}",
                        baseId, upperId, castingLoop);
                }
            }
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

    private void ActorCast(uint entityId, ActorCastPacket* packet)
    {
        actorCast.Original(entityId, packet);
        var local = objects.LocalPlayer;
        if (!active || !Eligible || local == null || local.EntityId != entityId || packet == null
            || packet->ActionType != (byte)ActionType.Action || packet->CastTime <= 0
            || packet->ActionId is not (120 or 135 or 124 or 16533)) return;
        try
        {
            var actionId = packet->ActionId;
            var action = data.GetExcelSheet<Lumina.Excel.Sheets.Action>().GetRow(actionId);
            var timeline = checked((ushort)action.AnimationStart.Value.Name.RowId);
            var loop = FindCastLoop(timeline);
            log.Information("Feast cast received: action {Action}, cast timeline row {Row}, animation {Timeline}, duration {Duration}",
                actionId, action.AnimationStart.RowId, timeline, packet->CastTime);
            if (timeline == 0) { log.Warning("Feast cast has no preparation timeline"); return; }
            StopCastingPose();
            var actor = (BattleChara*)local.Address;
            castingActor = (nint)actor;
            castingTimeline = timeline;
            castingLoop = loop;
            castingAction = actionId;
            loopStarted = false;
            lastCastingSlots = uint.MaxValue;
            castingOrigin = local.Position;
            castingEnds = Environment.TickCount64 + (long)Math.Ceiling(packet->CastTime * 1000);
            actor->Timeline.TimelineSequencer.PlayTimeline(timeline);
            log.Information("Feast casting motion: start={Start}, loop={Loop}", timeline, loop);
            castEffects.Start(local.Address, [
                action.AnimationStart.Value.VFX.Value.Location.ToString(),
                action.VFX.Value.VFX.Value.Location.ToString()
            ]);
            log.Information("Feast cast animation requested; base={Base}, upper={Upper}",
                actor->Timeline.TimelineSequencer.GetSlotTimeline(0),
                actor->Timeline.TimelineSequencer.GetSlotTimeline(1));
        }
        catch (Exception error) { log.Error(error, "Could not start Feast casting pose"); }
    }

    private void ActionEffect(uint entityId, Character* actor, System.Numerics.Vector3* position,
        ActionEffectHandler.Header* header, ActionEffectHandler.TargetEffects* effects,
        FFXIVClientStructs.FFXIV.Client.Game.Object.GameObjectId* targets)
    {
        // Remove our sustained base override BEFORE the native release handler runs.
        // A later movement/deadline cleanup can therefore never replace the release.
        if (castingActor != 0 && (nint)actor == castingActor && header != null
            && header->ActionId == castingAction && header->ActionType == (byte)ActionType.Action)
        {
            StopCastingPose(fadeEffects: true);
            log.Information("Feast cast completed: pose cleared and effects fading before release action {Action}", header->ActionId);
        }
        actionEffect.Original(entityId, actor, position, header, effects, targets);
    }

    private ushort FindCastLoop(ushort start)
    {
        if (castLoops.TryGetValue(start, out var loop)) return loop;
        var sheet = data.GetExcelSheet<Lumina.Excel.Sheets.ActionTimeline>();
        var key = sheet.GetRow(start).Key.ToString();
        if (!key.EndsWith("_start", StringComparison.Ordinal))
            throw new InvalidOperationException($"Unexpected casting start timeline: {key}");
        var loopKey = key[..^6] + "_loop";
        loop = checked((ushort)sheet.FirstOrDefault(row => row.Key.ToString() == loopKey).RowId);
        if (loop == 0) throw new InvalidOperationException($"Missing casting loop: {loopKey}");
        castLoops[start] = loop;
        return loop;
    }

    private void StopCastingPose(bool fadeEffects = false)
    {
        if (fadeEffects) castEffects.Complete();
        else castEffects.Stop();
        var address = castingActor;
        var timeline = castingTimeline;
        var loop = castingLoop;
        var restoreOverride = loopStarted;
        castingActor = 0;
        castingTimeline = 0;
        castingLoop = 0;
        castingAction = 0;
        loopStarted = false;
        if (address == 0 || objects.LocalPlayer?.Address != address) return;
        var actor = (BattleChara*)address;
        if (restoreOverride && actor->Timeline.BaseOverride == loop)
            actor->Timeline.BaseOverride = previousBaseOverride;
        // A completed ActionResult replaces the casting timeline with the release.
        // Never reset that release, even if movement/cancellation follows completion.
        for (uint slot = 0; slot < 14; slot++)
            if (actor->Timeline.TimelineSequencer.GetSlotTimeline(slot) is var current
                && (current == timeline || (loop != 0 && current == loop)))
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
        actorCast.Disable();
        actionEffect.Disable();
        Restore();
        execute.Dispose();
        actorCast.Dispose();
        actionEffect.Dispose();
        castEffects.Dispose();
    }
}
