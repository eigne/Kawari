using System.Text;
using System.Text.Json;
using System.Text.RegularExpressions;
using Lumina;
using Lumina.Data;
using Lumina.Excel.Sheets;

internal static class SupplyAssets
{
    public static void Run(GameData game)
    {
        var records = new List<object>();
        foreach (var path in new[] {
            "bg/ffxiv/pvp_xx/shared/for_bg/sgbg_l1p2_a0_gmc04.sgb",
            "bg/ffxiv/pvp_xx/shared/for_bg/sgbg_l1p2_a0_gmc05.sgb",
            "bgcommon/world/pvp/shared/for_bg/sgbg_w_pvp_001_04a.sgb",
            "bgcommon/world/pvp/shared/for_bg/sgbg_w_pvp_001_05a.sgb" })
        {
            var file = game.GetFile<FileResource>(path) ?? throw new Exception("Missing " + path);
            string text = Encoding.ASCII.GetString(file.Data);
            var dependencies = Regex.Matches(text, @"(?:bgcommon|bg|vfx|sound)/[a-zA-Z0-9_/.-]+")
                .Select(m => m.Value).Where(p => Path.HasExtension(p)).Distinct()
                .Select(p => new { Path = p, Bytes = game.GetFile<FileResource>(p)?.Data.Length }).ToArray();
            records.Add(new { Path = path, Bytes = file.Data.Length,
                Sha256 = Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(file.Data)),
                TimelineNames = Regex.Matches(text, @"(?:box|gem)_[a-z_]+").Select(m => m.Value).ToArray(), Dependencies = dependencies });
        }
        var statuses = new uint[]{994,1149}.Select(id => { var row = (game.GetExcelSheet<Status>() ?? throw new Exception("Missing Status sheet")).GetRow(id); return new {Id=id,Name=row.Name.ToString(),Description=row.Description.ToString()}; });
        Console.WriteLine(JsonSerializer.Serialize(new {Assets=records,Statuses=statuses},new JsonSerializerOptions{WriteIndented=true}));
    }
}
