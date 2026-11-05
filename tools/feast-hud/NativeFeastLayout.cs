using Dalamud.Game.ClientState.Objects.Types;
using Dalamud.Plugin.Services;
using FFXIVClientStructs.FFXIV.Component.GUI;

namespace Kawari.FeastHud;

// Adapt the surviving native row controller to PvPColosseumPartyList4's geometry.
// Gauge, cast, target-highlight and collision nodes are the original shared
// components. Keep their IDs/events: replacing the whole ULD breaks the controller.
internal sealed unsafe class NativeFeastLayout
{
    private sealed record Saved(nint Node, float X, float Y, ushort W, ushort H,
        float ScaleX, float ScaleY, bool Visible);
    private readonly Dictionary<nint, Dictionary<nint, Saved>> originals = [];
    private readonly NativeStatusStrip statuses = new();
    private readonly NativeRosterLabel labels = new();

    public void Update(nint address, DirectorFrame frame, byte team, IObjectTable objects, IDataManager data)
    {
        var addon = (AtkUnitBase*)address;
        if (addon->RootNode == null) return;
        var first = addon->GetComponentByNodeId(6);
        if (first == null || first->GetNodeById(37) == null) return;
        if (!originals.TryGetValue(address, out var saved)) originals[address] = saved = [];
        void Remember(AtkResNode* n)
        {
            if (n != null && !saved.ContainsKey((nint)n))
                saved.Add((nint)n, new((nint)n,n->X,n->Y,n->Width,n->Height,n->ScaleX,n->ScaleY,n->IsVisible()));
        }
        void Place(AtkResNode* n, float x, float y, ushort w, ushort h, float scale = 1)
        {
            if (n == null) return;
            Remember(n); n->SetPositionFloat(x,y); n->SetWidth(w); n->SetHeight(h); n->SetScale(scale,scale);
        }
        void Hide(AtkResNode* n) { if (n != null) { Remember(n); n->ToggleVisibility(false); } }
        // Large CC team panels/header are not part of the compact Feast roster.
        Hide(addon->GetNodeById(11));
        // Both ULDs retain PartyList_Bg.tex; only its native nine-grid height differs.
        var background = addon->GetNodeById(15);
        var header = addon->GetNodeById(2);
        Place(background,28,0,288,334);
        if (background != null) background->ToggleVisibility(true);
        Place(header,12,-9,366,32);
        if (header != null) header->ToggleVisibility(true);
        // The root position is owned by HUD Layout, unlike ULD-local child positions.
        Remember(addon->RootNode); addon->RootNode->SetWidth(384); addon->RootNode->SetHeight(330);
        Place(addon->GetNodeById(5),0,0,384,330);
        Place(addon->GetNodeById(17),0,0,384,330);
        Place(addon->GetNodeById(18),0,0,384,330);
        Hide(addon->GetNodeById(10)); // Feast is four versus four.
        var players = frame.Players.Where(p => p.Team == team + 1).Take(4).ToArray();
        for (int row = 0; row < 4; row++)
        {
            var owner = addon->GetNodeById((uint)(6 + row));
            var c = addon->GetComponentByNodeId((uint)(6 + row));
            if (owner == null || c == null) continue;
            Place(owner,0,row * 78,384,96);
            Place(c->GetNodeById(1),0,0,384,96);
            Place(c->GetNodeById(6),0,0,384,96);
            Hide(c->GetNodeById(2)); Hide(c->GetNodeById(7)); // CC-specific KO banner/LB strip.
            // CC node -> original Feast node: 10->2, 15->7, 16->8, 17->9,
            // 18->10, 32->26, 33->27, 34->28, 35->29, 36->30, 37->31.
            Place(c->GetNodeById(10),71,12,204,32);
            Place(c->GetNodeById(15),36,18,36,36);
            Place(c->GetNodeById(16),72,20,96,48);
            Place(c->GetNodeById(17),175,20,96,48);
            Place(c->GetNodeById(18),74,22,200,24);
            Place(c->GetNodeById(32),17,-3,367,100);
            Place(c->GetNodeById(33),3,5,68,68);
            Place(c->GetNodeById(34),50,15,317,85);
            Place(c->GetNodeById(35),50,15,317,85);
            Place(c->GetNodeById(36),35,92,334,4);
            Place(c->GetNodeById(37),28,16,350,76);
            for (int i = 0; i < 10; i++)
            {
                var icon = c->GetNodeById((uint)(22+i));
                if (i < 3)
                {
                    Place(icon,38+i*28,59,44,48,0.65f);
                    var actionIcon = c->GetComponentById((uint)(22+i));
                    if (actionIcon != null)
                    {
                        Place(actionIcon->GetNodeById(3),2,3,40,40);
                        Place(actionIcon->GetNodeById(2),0,34,44,18);
                    }
                }
                else Hide(icon);
            }
            var actor = row < players.Length ? objects.SearchByEntityId(players[row].Id) as IBattleChara : null;
            statuses.Update(address,c,actor,data);
            Hide(c->GetNodeById(21));
            Hide(c->GetNodeById(11));
            string? label = actor == null ? null : NativeRoster.DisplayName(players[row],actor,row);
            bool casting = false;
            if (actor is { IsCasting: true })
            {
                var castName = data.GetExcelSheet<Lumina.Excel.Sheets.Action>()?
                    .GetRowOrDefault(actor.CastActionId)?.Name.ToString();
                if (!string.IsNullOrWhiteSpace(castName)) { label = castName; casting = true; }
            }
            labels.Update(address,c,label,casting);
        }
        addon->UpdateCollisionNodeList(false);
    }

    public void Remove(nint addon)
    {
        labels.Remove(addon);
        statuses.Remove(addon);
        if (!originals.Remove(addon, out var saved)) return;
        foreach (var s in saved.Values)
        {
            var n = (AtkResNode*)s.Node;
            n->SetPositionFloat(s.X,s.Y); n->SetWidth(s.W); n->SetHeight(s.H);
            n->SetScale(s.ScaleX,s.ScaleY); n->ToggleVisibility(s.Visible);
        }
        ((AtkUnitBase*)addon)->UpdateCollisionNodeList(false);
    }
    public void Clear() { foreach (var addon in originals.Keys.ToArray()) Remove(addon); }
}
