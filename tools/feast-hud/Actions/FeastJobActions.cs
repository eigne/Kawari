using System.Text.Json;

namespace Kawari.FeastHud;

public sealed record JobAction(uint Job, string Key, uint Id, int Icon, string Name,
    float Cast, float Recast, int Group, int Secondary, int Charges, int Range, int Radius, string Description = "", string Category = "Ability", int MpCost = 0);

// The historical action catalog, resources/feast-actions.json (embedded).
// The server reads the same file; see servers/world/src/server/feast/actions.rs.
public static class FeastJobActions
{
    private static readonly JobAction[] All = JsonSerializer.Deserialize<JobAction[]>(
        typeof(FeastJobActions).Assembly.GetManifestResourceStream("FeastActions.json")!,
        new JsonSerializerOptions { PropertyNameCaseInsensitive = true })!;
    // Actions a button turns into (combo steps, procs, stance swaps) rather
    // than buttons of their own. The server's hotbar records (F6) say which
    // one a button currently is; Lua's M.resolve decides.
    private static readonly HashSet<string> Derived = [
        "riot_blade", "royal_authority", "prominence", "atonement", "glory_slash",
        "gekko", "kasha", "oka", "kaeshi_setsugekka", "kaeshi_goken", "thunder_iii", "thunder_iv", "fire_iv", "blizzard_iv",
        "full_thrust", "fang_and_claw", "wheeling_thrust", "sonic_thrust", "coerthan_torment", "nastrond",
        "maim", "storms_path", "steel_cyclone", "inner_chaos", "chaotic_cyclone",
        "true_strike", "snap_punch", "rockbreaker", "gust_slash", "aeolian_edge", "hakke_mujinsatsu", "hyosho_ranryu", "goka_mekkyaku", "trick_attack",
        "heated_slug_shot", "heated_clean_shot", "heat_blast", "auto_crossbow",
        "fountain", "bladeshower", "reverse_cascade", "fountainfall", "rising_windmill", "bloodshower",
        "emboite", "entrechat", "jete", "pirouette", "standard_finish", "technical_finish", "ending",
        "fountain_of_fire", "brand_of_purgatory", "firebird_trance",
        "enchanted_zwerchhau", "enchanted_redoublement", "veraero", "verthunder", "verholy", "verflare", "impact", "scorch",
        "aspected_benefic", "aspected_helios", "lord_of_crowns", "lady_of_crowns",
        "syphon_strike", "souleater", "stalwart_soul", "brutal_shell", "solid_barrel", "demon_slaughter", "savage_claw", "wicked_talon",
        "jugular_rip", "abdomen_tear", "eye_gouge",
        "waxing_slice", "infernal_slice", "nightmare_scythe", "gibbet", "gallows", "void_reaping", "cross_reaping", "guillotine", "grim_reaping",
        "communio", "unveiled_gibbet", "unveiled_gallows", "lemures_slice", "lemures_scythe", "regress",
        "eukrasian_dosis_iii", "eukrasian_diagnosis", "eukrasian_prognosis"];
    public static JobAction? Find(uint id) => All.FirstOrDefault(a => a.Id == id);
    // Soten, Cometeor and Shukuchi: cast at a point chosen with the ground cursor.
    public static bool IsGroundTargeted(uint id) => id is 7857 or 3361 or 8812;
    // The 19 jobs that existed before patch 6.1, when the Feast ended.
    public static bool Supports(uint job) => job is 19 or 20 or 21 or 22 or 23 or 24 or 25 or 27 or 28 or 30 or 31 or 32 or 33 or 34 or 35 or 37 or 38 or 39 or 40;
    public static bool Contains(uint job, uint id) => All.Any(a => a.Job == job && a.Id == id);
    // The server reports a charged recast as elapsed/total over all charges, recharging
    // one at a time, so a charge is ready once a full per-charge recast has elapsed.
    public static bool ChargeReady(int charges, bool active, float elapsed, float total)
        => !active || total <= 0 || elapsed >= total / Math.Max(1, charges);
    // The job's buttons for the default hotbars and the PvP Actions window.
    public static uint[] Hotbar(uint job) => All.Where(a => a.Job == job && !Derived.Contains(a.Key)).Select(a => a.Id).ToArray();
}
