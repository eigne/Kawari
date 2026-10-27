namespace Kawari.FeastHud;

// Presentation only. The director remains authoritative for medals and phase.
public sealed record HeaderTeam(string Name, int Score, IReadOnlyList<Participant> Players);
public sealed record MatchHeaderState(HeaderTeam Left, HeaderTeam Right, string Clock, string Phase)
{
    public static MatchHeaderState From(DirectorFrame? frame, long age)
    {
        if (frame == null || age > 2500)
            return new(new("", 0, []), new("", 0, []), "--:--", "WAITING FOR MATCH DATA");
        var s = frame.Summary;
        HeaderTeam Team(byte id) => new((id == 1 ? "Fangs" : "Claws") +
            ((id == 1 && s.Team == "fang") || (id == 2 && s.Team == "claw") ? " ★" : ""), id == 1 ? s.Fang : s.Claw,
            frame.Players.Where(p => p.Team == id).Take(4).ToArray());
        var seconds = Math.Max(0, s.Seconds - (s.Phase is "prep" or "active" or "sudden" ? (int)(Math.Max(0, age) / 1000) : 0));
        // Historical header: blue Claws on the left, red Fangs on the right;
        // the star identifies the player's team without swapping medal slots.
        return new(Team(2), Team(1), $"{seconds/60}:{seconds%60:00}", s.Phase switch
        {
            "prep" => "PREPARATION", "sudden" => "SUDDEN DEATH", "fang" => "FANGS WIN",
            "claw" => "CLAWS WIN", "draw" => "DRAW", _ => ""
        });
    }
    public static ushort GaugeWidth(int score) => (ushort)Math.Clamp(score * 340 / 600, 0, 340);
}
