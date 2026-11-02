using System.Text.Json;

namespace Kawari.FeastHud;

public sealed record JobAction(uint Job, string Key, uint Id, int Icon, string Name,
    float Cast, float Recast, int Group, int Secondary, int Charges, int Range, int Radius, string Description = "", string Category = "Ability", int MpCost = 0);
public static class FeastJobActions
{
    private static readonly JobAction[] All = JsonSerializer.Deserialize<JobAction[]>(
        typeof(FeastJobActions).Assembly.GetManifestResourceStream("FeastActions.json")!,
        new JsonSerializerOptions { PropertyNameCaseInsensitive = true })!;
    private static readonly HashSet<string> Derived = ["riot_blade", "royal_authority", "prominence", "atonement", "glory_slash",
        "gekko", "kasha", "oka", "kaeshi_setsugekka", "kaeshi_goken", "thunder_iii", "thunder_iv", "fire_iv", "blizzard_iv",
        "true_strike", "snap_punch", "rockbreaker", "gust_slash", "aeolian_edge", "hakke_mujinsatsu", "hyosho_ranryu", "goka_mekkyaku", "trick_attack",
        "heated_slug_shot", "heated_clean_shot", "heat_blast", "auto_crossbow", "fountain", "bladeshower", "reverse_cascade", "fountainfall", "rising_windmill", "bloodshower", "emboite", "entrechat", "jete", "pirouette", "standard_finish", "technical_finish", "ending",
        "fountain_of_fire", "brand_of_purgatory", "firebird_trance", "enchanted_zwerchhau", "enchanted_redoublement", "veraero", "verthunder", "verholy", "verflare", "impact", "scorch", "aspected_benefic", "aspected_helios", "lord_of_crowns", "lady_of_crowns",
        "syphon_strike", "souleater", "stalwart_soul", "brutal_shell", "solid_barrel", "demon_slaughter", "savage_claw", "wicked_talon", "jugular_rip", "abdomen_tear", "eye_gouge",
        "waxing_slice", "infernal_slice", "nightmare_scythe", "gibbet", "gallows", "void_reaping", "cross_reaping", "guillotine", "grim_reaping", "communio", "unveiled_gibbet", "unveiled_gallows", "lemures_slice", "lemures_scythe", "regress", "eukrasian_dosis_iii", "eukrasian_diagnosis", "eukrasian_prognosis"];
    public static JobAction? Find(uint id) => All.FirstOrDefault(a => a.Id == id);
    public static bool Supports(uint job) => job is 19 or 20 or 21 or 22 or 23 or 24 or 25 or 27 or 28 or 30 or 31 or 32 or 33 or 34 or 35 or 37 or 38 or 39 or 40;
    public static bool Contains(uint job, uint id) => All.Any(a => a.Job == job && a.Id == id);
    // The server reports a charged recast as elapsed/total over all charges, recharging
    // one at a time, so a charge is ready once a full per-charge recast has elapsed.
    public static bool ChargeReady(int charges, bool active, float elapsed, float total)
        => !active || total <= 0 || elapsed >= total / Math.Max(1, charges);
    public static uint[] Hotbar(uint job) => All.Where(a => a.Job == job && !Derived.Contains(a.Key)
        && a.Key is not ("full_thrust" or "fang_and_claw" or "wheeling_thrust" or "sonic_thrust" or "coerthan_torment" or "nastrond" or "maim" or "storms_path" or "steel_cyclone" or "inner_chaos" or "chaotic_cyclone")).Select(a => a.Id).ToArray();
}
