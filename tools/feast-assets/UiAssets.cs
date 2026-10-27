using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using System.Text.RegularExpressions;
using Lumina;
using Lumina.Data;
using Lumina.Data.Files;
using Lumina.Data.Structs;
using Lumina.Excel.Sheets;

// Read-only comparison of surviving UI resources and locally recovered 6.08 ULDs.
internal static class UiAssets
{
    internal static UldFile ReadLayout(byte[] bytes)
    {
        var result = new UldFile();
        // Lumina normally initializes these internal setters from a SqPack file.
        // Recovered patch resources are already decompressed; use the same parser.
        typeof(FileResource).GetProperty(nameof(FileResource.Data))!.SetValue(result, bytes);
        typeof(FileResource).GetProperty(nameof(FileResource.Reader))!.SetValue(result, new LuminaBinaryReader(bytes, PlatformId.Win32));
        result.LoadFile();
        return result;
    }
    internal static object Layout(byte[] bytes)
    {
        var uld = ReadLayout(bytes);
        object Node(Lumina.Data.Parsing.Uld.UldRoot.NodeData n) => new { n.NodeId, n.ParentId, n.NodeType, n.X, n.Y, n.W, n.H, n.Visible };
        return new {
            assets = uld.AssetData.Select(a => new { a.Id, path = new string(a.Path).TrimEnd('\0').ToLowerInvariant(), a.IconId }),
            components = uld.Components.Select(c => new { c.Id, c.Type, nodes = c.Nodes.Select(Node) }),
            widget = new { uld.WidgetData.NodeCount, nodes = uld.WidgetData.Nodes.Select(Node) },
        };
    }
    internal static void Run(GameData game)
    {
        var root = Path.GetFullPath("target/feast-hud-recovery");
        var candidates = new SortedSet<string>(StringComparer.Ordinal);
        foreach (var name in new[] { "pvpactionjob", "pvpactionadditional", "pvpactiontraits", "pvpactionquickchat", "pvpactionset", "pvpactionsetview",
            "pvpprofile", "pvpprofileaction", "pvpprofileactionadditional", "pvpprofileadditional", "pvpcolosseumheader", "pvpcolosseumheaderspec",
            "pvpcolosseumpartylist0", "pvpcolosseumpartylist1", "pvpcolosseumpartylist4", "pvpcolosseumpartylistspec1", "pvpcolosseumpartylistspec2",
            "pvpmkspartylist1", "pvpmkspartylist3", "pvpmksheader", "pvpscreeninformation", "pvpscreeninformationhotbar", "colosseumresult",
            "pvpcolosseumresult", "pvprankratingfunction", "pvprankpromotionqualifier", "pvpcalendar", "pvpcalender", "actionmenu", "actioncontents",
            "jobhudblm0", "jobhudblm1", "jobhudsam0", "jobhudpld0", "jobhudwhm0", "jobhudwhm1" })
            candidates.Add($"ui/uld/{name}.uld");
        var historical = new Dictionary<string, byte[]>(StringComparer.Ordinal);
        if (Directory.Exists(Path.Combine(root,"assets")))
            foreach (var path in Directory.EnumerateFiles(Path.Combine(root,"assets"),"*",SearchOption.AllDirectories))
            {
                var relative=Path.GetRelativePath(Path.Combine(root,"assets"),path).Replace('\\','/').ToLowerInvariant();
                if (!Regex.IsMatch(relative,"pvp|colosseum|feast|actionmenu|actioncontents|icona_")) continue;
                historical[relative]=File.ReadAllBytes(path); candidates.Add(relative);
            }
        // These names were recovered from the 6.08 executable and matched against
        // the SqPack filename CRC; see the reproducible name resolver.
        foreach (var (hash,name) in new[] { ("ea17456b","pvpcolosseumpartylist4"), ("8f1241b0","pvpscreeninformation"),
            ("330bbd06","pvprankratingfunction"), ("ee23982c","pvprankpromotionqualifier") })
        {
            var path=Path.Combine(root,"unidentified",$"uld_{hash}.uld");
            if (File.Exists(path)) historical[$"ui/uld/{name}.uld"]=File.ReadAllBytes(path);
        }
        var resources = new List<object>();
        var dependencies = new SortedDictionary<string, int?>();
        foreach (var path in candidates)
        {
            var current=game.GetFile<FileResource>(path)?.Data;
            historical.TryGetValue(path,out var old);
            foreach (var bytes in new[] { current, old })
                if (path.EndsWith(".uld") && bytes != null)
                    foreach (var asset in ReadLayout(bytes).AssetData)
                    {
                        var texture = new string(asset.Path).TrimEnd('\0').ToLowerInvariant();
                        if (texture.Length != 0 && !dependencies.ContainsKey(texture))
                            dependencies[texture] = game.GetFile<FileResource>(texture)?.Data.Length;
                    }
            resources.Add(new { path, currentBytes=current?.Length, historicalBytes=old?.Length,
                identical=current!=null && old!=null ? (bool?)current.SequenceEqual(old) : null,
                currentHash=current==null ? null : Convert.ToHexString(SHA256.HashData(current)),
                historicalHash=old==null ? null : Convert.ToHexString(SHA256.HashData(old)),
                currentLayout=path.EndsWith(".uld") && current!=null ? Layout(current) : null,
                historicalLayout=path.EndsWith(".uld") && old!=null ? Layout(old) : null });
        }
        var icons = new List<object>();
        var catalog=JsonDocument.Parse(File.ReadAllText("resources/feast-actions.json"));
        using var whm = JsonDocument.Parse(File.ReadAllText("resources/feast-whm-icons.json"));
        foreach (var action in catalog.RootElement.EnumerateArray().Concat(whm.RootElement.EnumerateArray()))
        {
            int id=action.GetProperty("icon").GetInt32();
            string folder=$"ui/icon/{id/1000*1000:000000}/{id:000000}";
            var variants=new[]{folder+".tex",folder+"_hr1.tex"}.Select(path => {
                var tex=game.GetFile<TexFile>(path);
                return new { path, exists=tex!=null, width=tex?.Header.Width, height=tex?.Header.Height };
            }).ToArray();
            icons.Add(new { action=action.GetProperty("id").GetUInt32(), id, variants });
        }
        var rootList=game.GetFile<FileResource>("exd/root.exl");
        var sheets=rootList==null ? [] : Encoding.UTF8.GetString(rootList.Data).Split('\n')
            .Where(s=>Regex.IsMatch(s,"pvp|colosseum|feast",RegexOptions.IgnoreCase)).Select(s=>s.Trim()).ToArray();
        var menuActions = (game.GetSubrowExcelSheet<PvPActionSort>() ?? throw new InvalidOperationException("PvPActionSort missing")).SelectMany(group => group)
            .Where(row => row.RowId is 19 or 24 or 25 or 34)
            .Select(row => new { job = row.RowId, index = row.SubrowId, action = row.Action.RowId, type = row.ActionType }).ToArray();
        Console.WriteLine(JsonSerializer.Serialize(new { resources, icons, sheets, dependencies, menuActions },new JsonSerializerOptions { WriteIndented=true }));
    }
}
