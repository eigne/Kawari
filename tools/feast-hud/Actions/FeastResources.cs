namespace Kawari.FeastHud;

// UI prediction only; the server validates and consumes the real resources.
public static class FeastResources
{
    public static bool CanUse(uint action, DirectorFrame? frame, uint actor)
    {
        if (frame == null || frame.Summary.Phase is not ("active" or "sudden")) return false;
        var self = frame.Players.FirstOrDefault(p => p.Id == actor);
        if (self is not { Alive: true }) return false;
        if (action is 3359 or 3360 or 3361 or 3362 or 4249) return self.Adrenaline == 100;
        if (action is 17791 or 18946) return frame.Gauge is { Job: 24, Resource: > 0 };
        if (action == 17793) return frame.Gauge is { Job: 24, Meditation: 3 };
        return true;
    }
}
