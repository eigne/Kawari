using Dalamud.Game.Gui.NamePlate;
using Dalamud.Game.Text.SeStringHandling;
using Dalamud.Game.Text.SeStringHandling.Payloads;
using Dalamud.Plugin.Services;

namespace Kawari.FeastHud;

// Feast showed each player's carried medals in their nameplate's «» slot,
// where a Free Company tag normally sits. The current client hides FC tags in
// PvP and has no medal source, so fill the slot from the director frame.
internal sealed class NativeNamePlateMedals : IDisposable
{
    private readonly INamePlateGui namePlates;
    private readonly NativeDirector director;
    private string drawn = "";

    public NativeNamePlateMedals(INamePlateGui namePlates, NativeDirector director)
    {
        this.namePlates = namePlates; this.director = director;
        namePlates.OnNamePlateUpdate += OnUpdate;
    }

    // Nameplates only rebuild their text when the game asks; redraw when medals change.
    public void Update()
    {
        var frame = director.InArena ? director.Frame : null;
        var state = frame == null ? "" : string.Join(',', frame.Players.Select(p => $"{p.Id}:{p.Medals}"));
        if (state == drawn) return;
        drawn = state;
        namePlates.RequestRedraw();
    }

    private void OnUpdate(INamePlateUpdateContext context, IReadOnlyList<INamePlateUpdateHandler> handlers)
    {
        if (!director.InArena || director.Frame is not { } frame) return;
        foreach (var handler in handlers)
        {
            if (handler.PlayerCharacter is not { } player) continue;
            var participant = frame.Players.FirstOrDefault(p => p.Id == player.EntityId);
            if (participant == null) continue;
            handler.FreeCompanyTag = new SeString(new TextPayload($" «{participant.Medals}»"));
        }
    }

    public void Dispose()
    {
        namePlates.OnNamePlateUpdate -= OnUpdate;
        namePlates.RequestRedraw();
    }
}
