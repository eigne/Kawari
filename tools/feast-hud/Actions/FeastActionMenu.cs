namespace Kawari.FeastHud;

public sealed record MenuAction(uint HotbarId, JobAction Action);
public sealed record ActionMenu(uint Job, IReadOnlyList<MenuAction> JobActions, MenuAction? Role,
    IReadOnlyList<MenuAction> Additional, IReadOnlyList<MenuAction> Common, MenuAction? Adrenaline);
public sealed record LayoutSlot(uint Bar, uint Slot, uint Id);

// Groups the Feast-era PvP Actions window from the same catalog the hotbars use.
// Role lists mirror resources/scripts/feast/Jobs.lua and AdditionalJobs.lua: the
// first entry is always available, the other five are chosen two at a time.
public static class FeastActionMenu
{
    private static readonly string[] Healer = ["purify", "repose", "protect", "rescue", "largesse", "attunement"];
    private static readonly string[] Caster = ["phantom_dart", "drain", "addle", "aetheric_burst", "swiftcast", "manaward"];
    private static readonly string[] Melee = ["smite", "feint", "bloodbath", "arms_length", "fetter_ward", "backstep"];
    private static readonly string[] Tank = ["low_blow", "full_swing", "weapon_throw", "reprisal", "rampart", "retaliation"];
    private static readonly string[] Ranged = ["head_graze", "leg_graze", "foot_graze", "concentrate", "recuperate", "peloton"];
    public const int LoadoutSize = 2;
    public const uint Arena = 745, WolvesDen = 250;
    // As before 6.1: actions are chosen in the Wolves' Den, whose PvP hotbars
    // carry the Feast layout; the arena uses that layout but cannot change it.
    public static bool CanEditActions(uint territory) => territory == WolvesDen;
    public static bool UsesFeastHotbars(uint territory) => territory is Arena or WolvesDen;
    public static bool CanCast(uint territory) => territory == Arena;
    public static uint[] MenuIds(uint job) => FeastJobActions.Hotbar(job);
    public static string[] RoleActions(uint job) => job switch
    {
        19 or 21 or 32 or 37 => Tank,
        20 or 22 or 30 or 34 or 39 => Melee,
        23 or 31 or 38 => Ranged,
        25 or 27 or 35 => Caster,
        24 or 28 or 33 or 40 => Healer,
        _ => [],
    };
    // Server defaults: White Mage starts with Protect/Largesse, everyone else the first two choices.
    public static string[] DefaultLoadout(uint job) => job == 24 ? ["protect", "largesse"] : RoleActions(job) is { Length: 6 } r ? [r[1], r[2]] : [];
    public static bool IsAdrenaline(uint id) => id is >= 3359 and <= 3362 or 4249;
    public static ActionMenu For(uint job, IEnumerable<uint> hotbarIds)
    {
        var roles = RoleActions(job);
        var entries = hotbarIds.Select(id => FeastJobActions.Find(id) is { } a ? new MenuAction(id, a) : null)
            .OfType<MenuAction>().ToArray();
        bool IsRole(MenuAction m) => roles.Contains(m.Action.Key);
        bool IsCommon(MenuAction m) => m.Action.Key is "bolt" or "kit";
        return new(job,
            entries.Where(m => !IsRole(m) && !IsCommon(m) && !IsAdrenaline(m.Action.Id)).ToArray(),
            entries.FirstOrDefault(m => roles.Length > 0 && m.Action.Key == roles[0]),
            roles.Skip(1).Select(key => entries.FirstOrDefault(m => m.Action.Key == key)).OfType<MenuAction>().ToArray(),
            entries.Where(IsCommon).ToArray(),
            entries.FirstOrDefault(m => IsAdrenaline(m.Action.Id)));
    }
    // Only two distinct choices from this job's own additional actions can be sent.
    public static bool ValidLoadout(uint job, IReadOnlyList<string> keys)
        => keys.Count == LoadoutSize && keys.Distinct().Count() == LoadoutSize && keys.All(k => RoleActions(job).Skip(1).Contains(k));
    public static string LoadoutCommand(IReadOnlyList<string> keys) => $"!feast loadout {keys[0]} {keys[1]}";
    // Selecting a third action replaces the oldest choice, like the original two-slot set.
    public static string[] Toggle(IReadOnlyList<string> current, string key)
    {
        if (current.Contains(key)) return current.Where(k => k != key).ToArray();
        var next = current.Append(key).ToArray();
        return next.Length > LoadoutSize ? next[^LoadoutSize..] : next;
    }
    public static IReadOnlyList<LayoutSlot> Place(IReadOnlyList<LayoutSlot> layout, uint bar, uint slot, uint id)
        => bar >= 10 || slot >= 12 ? layout
            : layout.Where(s => s.Bar != bar || s.Slot != slot).Append(new(bar, slot, id)).OrderBy(s => s.Bar).ThenBy(s => s.Slot).ToArray();
    public static IReadOnlyList<LayoutSlot> Clear(IReadOnlyList<LayoutSlot> layout, uint bar, uint slot)
        => layout.Where(s => s.Bar != bar || s.Slot != slot).ToArray();
    // Returns the native hotbar slot under a point, given each visible slot's screen rectangle.
    public static (uint Bar, uint Slot)? HitTest(IEnumerable<(uint Bar, uint Slot, float X, float Y, float W, float H)> slots, float x, float y)
    {
        foreach (var s in slots)
            if (s.Bar < 10 && s.Slot < 12 && x >= s.X && y >= s.Y && x < s.X + s.W && y < s.Y + s.H) return (s.Bar, s.Slot);
        return null;
    }
}
