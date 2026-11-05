using System.Numerics;
using Dalamud.Bindings.ImGui;
using Dalamud.Interface.Textures;
using Dalamud.Interface.Windowing;
using Dalamud.Plugin.Services;
using FFXIVClientStructs.FFXIV.Client.UI;
using FFXIVClientStructs.FFXIV.Client.UI.Misc;
using FFXIVClientStructs.FFXIV.Component.GUI;

namespace Kawari.FeastHud;

// Feast-era PvP Actions window: job, role and additional actions for the
// current job, dragged onto the Wolves' Den PvP hotbars or the saved Feast
// layout. Only opens in the Wolves' Den; the chosen loadout is sent with
// !feast loadout during the next match's preparation, where the server checks it.
internal sealed unsafe class FeastActionsWindow : Window
{
    private const float IconSize = 40;
    private static readonly Vector4 Gold = new(0.93f, 0.82f, 0.55f, 1), Muted = new(0.62f, 0.6f, 0.56f, 1), Frame = new(0.36f, 0.3f, 0.22f, 1);
    private readonly IClientState client;
    private readonly IPlayerState player;
    private readonly IGameGui gameGui;
    private readonly ITextureProvider textures;
    private readonly IDataManager data;
    private readonly FeastHotbars hotbars;
    private readonly List<(uint Bar, uint Slot, Vector2 Min, Vector2 Max)> layoutCells = [];
    private MenuAction? dragging;
    private (uint Job, List<string> Keys)? pending;
    private string status = "";
    public IReadOnlyList<PartyAction> EquippedAdditional { get; set; } = [];
    public bool InArena => client.IsLoggedIn && client.TerritoryType == FeastActionMenu.Arena;
    public bool InDen => client.IsLoggedIn && FeastActionMenu.CanEditActions(client.TerritoryType);

    public FeastActionsWindow(IClientState client, IPlayerState player, IGameGui gameGui, ITextureProvider textures,
        IDataManager data, FeastHotbars hotbars) : base("PvP Actions###KawariFeastActions")
    {
        this.client = client; this.player = player; this.gameGui = gameGui; this.textures = textures;
        this.data = data; this.hotbars = hotbars;
        Size = new Vector2(560, 600); SizeCondition = ImGuiCond.FirstUseEver;
        SizeConstraints = new() { MinimumSize = new(460, 380), MaximumSize = new(900, 1200) };
    }

    private uint Job => player.ClassJob.RowId;
    public List<string> Loadout(uint job)
    {
        // In a match, the server's equipped actions are the truth.
        if (InArena && EquippedAdditional.Count > 0)
        {
            var keys = EquippedAdditional.Where(a => !a.IsAdrenaline).Select(a => FeastJobActions.Find(a.Id)?.Key)
                .Where(k => k != null && FeastActionMenu.RoleActions(job).Skip(1).Contains(k)).Select(k => k!).ToList();
            if (keys.Count == FeastActionMenu.LoadoutSize) return keys;
        }
        return hotbars.Config.Loadouts.TryGetValue(job, out var saved) && FeastActionMenu.ValidLoadout(job, saved)
            ? saved : FeastActionMenu.DefaultLoadout(job).ToList();
    }

    public override void Draw()
    {
        if (!FeastJobActions.Supports(Job))
        {
            ImGui.TextWrapped("Switch to a job that took part in the Feast to see its PvP actions.");
            return;
        }
        var menu = FeastActionMenu.For(Job, FeastActionMenu.MenuIds(Job));
        var jobName = data.GetExcelSheet<Lumina.Excel.Sheets.ClassJob>().GetRowOrDefault(Job)?.Name.ExtractText() ?? "";
        ImGui.TextColored(Gold, System.Globalization.CultureInfo.CurrentCulture.TextInfo.ToTitleCase(jobName));
        ImGui.SameLine();
        ImGui.TextColored(Muted, hotbars.Active ? "Drag an action onto a hotbar." : "Waiting for PvP hotbars...");
        ImGui.Separator();
        layoutCells.Clear();
        if (ImGui.BeginTabBar("##feasttabs"))
        {
            if (ImGui.BeginTabItem("Job Actions"))
            {
                Section("Job Actions", menu.JobActions);
                if (menu.Role != null) Section("Role Action", [menu.Role]);
                Section("Common Actions", menu.Common);
                if (menu.Adrenaline != null) Section("Adrenaline Rush", [menu.Adrenaline]);
                ImGui.EndTabItem();
            }
            if (ImGui.BeginTabItem("Additional Actions"))
            {
                DrawAdditional(menu);
                ImGui.EndTabItem();
            }
            if (ImGui.BeginTabItem("Hotbar Layout"))
            {
                DrawLayout();
                ImGui.EndTabItem();
            }
            ImGui.EndTabBar();
        }
        if (status.Length > 0) { ImGui.Separator(); ImGui.TextColored(Muted, status); }
        DrawDrag();
    }

    private void Section(string title, IReadOnlyList<MenuAction> actions)
    {
        if (actions.Count == 0) return;
        ImGui.Spacing();
        ImGui.TextColored(Gold, title);
        var draw = ImGui.GetWindowDrawList();
        var start = ImGui.GetCursorScreenPos();
        draw.AddLine(start, start + new Vector2(ImGui.GetContentRegionAvail().X, 0), ImGui.GetColorU32(Frame));
        ImGui.Dummy(new(0, 4));
        float column = MathF.Max(200, (ImGui.GetContentRegionAvail().X - 8) / 2);
        int columns = Math.Max(1, (int)(ImGui.GetContentRegionAvail().X / column));
        for (int i = 0; i < actions.Count; i++)
        {
            if (i % columns != 0) ImGui.SameLine(column * (i % columns) + ImGui.GetStyle().WindowPadding.X);
            Entry(actions[i], column - 8, true);
        }
    }

    // One draggable row: native-style icon frame plus the action name and type.
    private void Entry(MenuAction action, float width, bool enabled)
    {
        ImGui.PushID((int)action.HotbarId);
        var pos = ImGui.GetCursorScreenPos();
        ImGui.InvisibleButton("##action", new(width, IconSize + 4));
        bool hovered = ImGui.IsItemHovered(), active = ImGui.IsItemActive();
        if (active && ImGui.IsMouseDragging(ImGuiMouseButton.Left, 4)) dragging = action;
        var draw = ImGui.GetWindowDrawList();
        if (hovered) draw.AddRectFilled(pos, pos + new Vector2(width, IconSize + 4), ImGui.GetColorU32(new Vector4(1, 0.9f, 0.6f, 0.08f)), 4);
        Icon(draw, action.Action.Icon, pos + new Vector2(2, 2), IconSize, enabled ? 1 : 0.4f);
        draw.AddText(pos + new Vector2(IconSize + 10, 4), ImGui.GetColorU32(enabled ? new Vector4(1, 1, 1, 1) : Muted), action.Action.Name);
        draw.AddText(pos + new Vector2(IconSize + 10, 22), ImGui.GetColorU32(Muted), Kind(action.Action));
        if (hovered && dragging == null) Tooltip(action.Action);
        ImGui.PopID();
    }

    private void Icon(ImDrawListPtr draw, int icon, Vector2 min, float size, float alpha)
    {
        var wrap = textures.GetFromGameIcon(new GameIconLookup((uint)icon, false, true)).GetWrapOrEmpty();
        draw.AddImage(wrap.Handle, min, min + new Vector2(size), Vector2.Zero, Vector2.One, ImGui.GetColorU32(new Vector4(1, 1, 1, alpha)));
        draw.AddRect(min - Vector2.One, min + new Vector2(size + 1), ImGui.GetColorU32(Frame), 3, ImDrawFlags.None, 1.5f);
    }

    private static string Kind(JobAction a) => a.Category switch
    {
        "Spell" or "Weaponskill" => a.Cast > 0 ? $"{a.Category}  {a.Cast:0.##}s cast" : $"{a.Category}  Instant",
        "Instant" or "2s" => "Limit Break",
        _ => a.Recast > 0 ? $"Ability  {a.Recast:0.#}s recast" : "Ability",
    };

    private static void Tooltip(JobAction a)
    {
        ImGui.BeginTooltip();
        ImGui.PushTextWrapPos(360);
        ImGui.TextColored(Gold, a.Name);
        ImGui.TextColored(Muted, $"{Kind(a)}   Recast {a.Recast:0.##}s" + (a.Range > 0 ? $"   Range {a.Range}y" : "")
            + (a.Radius > 0 ? $"   Radius {a.Radius}y" : "") + (a.MpCost > 0 ? $"   MP {a.MpCost}" : ""));
        if (a.Description.Length > 0) { ImGui.Separator(); ImGui.TextUnformatted(a.Description); }
        ImGui.PopTextWrapPos();
        ImGui.EndTooltip();
    }

    private void DrawAdditional(ActionMenu menu)
    {
        ImGui.Spacing();
        ImGui.TextWrapped("Set two additional actions. Only set actions can be used in a match; changes are refused during combat.");
        var loadout = pending is { } p && p.Job == menu.Job ? p.Keys : Loadout(menu.Job);
        ImGui.Spacing();
        foreach (var action in menu.Additional)
        {
            bool set = loadout.Contains(action.Action.Key);
            ImGui.PushID(action.Action.Key);
            if (ImGui.Checkbox("##set", ref set))
            {
                var next = FeastActionMenu.Toggle(loadout, action.Action.Key).ToList();
                if (FeastActionMenu.ValidLoadout(menu.Job, next))
                {
                    pending = null; hotbars.Config.Loadouts[menu.Job] = next; hotbars.Save();
                    status = "Additional actions will be set when you enter the Feast.";
                }
                else { pending = (menu.Job, next); status = "Choose two additional actions."; }
                loadout = next;
            }
            ImGui.PopID();
            ImGui.SameLine();
            Entry(action, ImGui.GetContentRegionAvail().X, set);
        }
    }

    // Saved layout for the next match, shown as the first three PvP hotbars.
    private void DrawLayout()
    {
        ImGui.Spacing();
        ImGui.TextWrapped(hotbars.Active ? "This mirrors your PvP hotbars. Dragging here or onto the hotbars both update it."
            : "Actions placed here fill your PvP hotbars when you enter the Feast. Right-click a slot to clear it.");
        var layout = hotbars.Config.Layouts.GetValueOrDefault(Job) ?? [];
        var draw = ImGui.GetWindowDrawList();
        float size = MathF.Min(36, (ImGui.GetContentRegionAvail().X - 60) / 12 - 4);
        for (uint bar = 0; bar < 3; bar++)
        {
            ImGui.TextColored(Muted, $"Hotbar {bar + 1}");
            for (uint i = 0; i < 12; i++)
            {
                if (i > 0) ImGui.SameLine(0, 4);
                ImGui.PushID((int)(bar * 12 + i));
                var pos = ImGui.GetCursorScreenPos();
                ImGui.InvisibleButton("##cell", new(size));
                draw.AddRectFilled(pos, pos + new Vector2(size), ImGui.GetColorU32(new Vector4(0, 0, 0, 0.45f)), 3);
                var placed = layout.FirstOrDefault(s => s.Bar == bar && s.Slot == i);
                var action = placed == null ? null : FeastJobActions.Find(placed.Id);
                if (action != null)
                {
                    Icon(draw, action.Icon, pos, size, 1);
                    if (ImGui.IsItemHovered() && dragging == null) Tooltip(action);
                    if (!hotbars.Active && ImGui.IsItemClicked(ImGuiMouseButton.Right))
                    { hotbars.Config.Layouts[Job] = FeastActionMenu.Clear(layout, bar, i).ToList(); hotbars.Save(); }
                }
                else draw.AddRect(pos, pos + new Vector2(size), ImGui.GetColorU32(Frame), 3);
                layoutCells.Add((bar, i, pos, pos + new Vector2(size)));
                ImGui.PopID();
            }
        }
        ImGui.Spacing();
        if (ImGui.Button("Restore default layout")) { hotbars.ResetLayout(Job); status = "Default Feast hotbars restored."; }
    }

    private void DrawDrag()
    {
        if (dragging == null) return;
        var mouse = ImGui.GetMousePos();
        var fg = ImGui.GetForegroundDrawList();
        Icon(fg, dragging.Action.Icon, mouse - new Vector2(IconSize / 2), IconSize, 0.85f);
        if (!ImGui.IsMouseReleased(ImGuiMouseButton.Left)) return;
        var dropped = dragging; dragging = null;
        foreach (var cell in layoutCells)
            if (mouse.X >= cell.Min.X && mouse.Y >= cell.Min.Y && mouse.X < cell.Max.X && mouse.Y < cell.Max.Y)
            {
                if (hotbars.Active) { status = hotbars.Place(cell.Bar, cell.Slot, dropped.HotbarId) ? $"{dropped.Action.Name} set." : "That slot cannot be changed right now."; return; }
                hotbars.Config.Layouts[Job] = FeastActionMenu.Place(hotbars.Config.Layouts.GetValueOrDefault(Job) ?? [], cell.Bar, cell.Slot, dropped.HotbarId).ToList();
                hotbars.Save(); status = $"{dropped.Action.Name} added to Hotbar {cell.Bar + 1}."; return;
            }
        var target = NativeSlotAt(mouse - ImGui.GetMainViewport().Pos);
        if (target == null) return;
        if (!hotbars.Active) { status = "Wait for your PvP hotbars to load, or use Hotbar Layout."; return; }
        status = hotbars.Place(target.Value.Bar, target.Value.Slot, dropped.HotbarId)
            ? $"{dropped.Action.Name} set on Hotbar {target.Value.Bar + 1}." : "That slot cannot be changed right now.";
    }

    // Finds the visible standard hotbar slot under a point in game screen coordinates.
    private (uint Bar, uint Slot)? NativeSlotAt(Vector2 point)
    {
        var rects = new List<(uint, uint, float, float, float, float)>();
        for (int bar = 0; bar < 10; bar++)
        {
            var addon = (AddonActionBarBase*)gameGui.GetAddonByName(bar == 0 ? "_ActionBar" : $"_ActionBar{bar:00}", 1).Address;
            if (addon == null || !addon->IsVisible || addon->RaptureHotbarId >= 10) continue;
            var slots = addon->ActionBarSlotVector;
            for (int i = 0; i < slots.Count && i < 12; i++)
            {
                // The icon is hidden on empty slots; the drag-drop frame is not.
                var dragDrop = slots.First[i].ComponentDragDrop;
                if (dragDrop == null || dragDrop->OwnerNode == null) continue;
                var res = &dragDrop->OwnerNode->AtkResNode;
                if (!res->IsVisible()) continue;
                float scale = 1;
                for (var n = res; n != null; n = n->ParentNode) scale *= n->ScaleX;
                rects.Add((addon->RaptureHotbarId, (uint)i, res->ScreenX, res->ScreenY, res->Width * scale, res->Height * scale));
            }
        }
        return FeastActionMenu.HitTest(rects, point.X, point.Y);
    }
}
