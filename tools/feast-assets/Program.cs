using System.Text;
using System.Text.Json;
using System.Text.RegularExpressions;
using Lumina;
using Lumina.Data;
using Lumina.Excel.Sheets;

// Read-only audit: emits metadata, never extracts or modifies game assets.
if (args.Length is < 1 or > 2) throw new ArgumentException("Usage: dotnet run --project tools/feast-assets -- <game/sqpack>");
using var game = new GameData(args[0]);
if (args.Length == 2 && args[1] == "--supplies") { SupplyAssets.Run(game); return; }
uint[] ids = [8895,8896,18945,17789,17790,17791,18946,17793,13975,9620,17794,1584,18942,18943,17690,17832,17688,18957,17689,3362];
var actions = game.GetExcelSheet<Lumina.Excel.Sheets.Action>() ?? throw new InvalidOperationException("Action sheet missing");
var timelineSheet = game.GetExcelSheet<ActionTimeline>() ?? throw new InvalidOperationException("ActionTimeline sheet missing");
var records = new List<object>();
foreach (uint id in ids)
{
    var action = actions.GetRow(id);
    var timelines = new List<object>();
    foreach (uint timelineId in new[] { action.AnimationEnd.RowId, action.ActionTimelineHit.RowId }.Distinct())
    {
        string key = timelineSheet.GetRow(timelineId).Key.ToString();
        string path = $"chara/action/{key}.tmb";
        var file = game.GetFile<FileResource>(path);
        var dependencies = new SortedDictionary<string, int?>();
        void Scan(byte[] data, int depth)
        {
            foreach (Match m in Regex.Matches(Encoding.ASCII.GetString(data), @"(?:vfx|sound|chara)/[a-zA-Z0-9_/.-]+"))
            {
                string dependency = m.Value;
                if (!Path.HasExtension(dependency)) dependency += ".tmb";
                if (dependencies.ContainsKey(dependency)) continue;
                var resource = game.GetFile<FileResource>(dependency);
                dependencies[dependency] = resource?.Data.Length;
                if (depth < 2 && resource != null && (dependency.EndsWith(".avfx") || dependency.EndsWith(".tmb"))) Scan(resource.Data, depth + 1);
            }
        }
        if (file != null) Scan(file.Data, 0);
        timelines.Add(new { timelineId, path, bytes = file?.Data.Length, dependencies });
    }
    records.Add(new { id, name = action.Name.ToString(), action.Icon, action.IsPvP, timelines });
}
Console.WriteLine(JsonSerializer.Serialize(records, new JsonSerializerOptions { WriteIndented = true }));
