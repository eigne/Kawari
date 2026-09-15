namespace Kawari.FeastHud;

public sealed record Snapshot(string Phase, int Seconds, int Fang, int Claw, string Team, int Medals, int Cap, int Ground)
{
    public static bool TryParse(string text, out Snapshot? value)
    {
        value = null; var p = text.Split('|');
        if (p.Length != 9 || p[0] != "KFEASTHUD1" || p[1] is not ("prep" or "active" or "sudden" or "fang" or "claw" or "draw") || p[5] is not ("fang" or "claw" or "spectator")) return false;
        int[] n = new int[6]; int[] indices = [2, 3, 4, 6, 7, 8];
        for (int i = 0; i < n.Length; i++) if (!int.TryParse(p[indices[i]], out n[i]) || n[i] < 0 || n[i] > 10000) return false;
        if (n[0] > 360 || n[4] > 200 || n[3] > n[4]) return false;
        value = new(p[1], n[0], n[1], n[2], p[5], n[3], n[4], n[5]); return true;
    }
}

