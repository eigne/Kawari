using FFXIVClientStructs.FFXIV.Component.GUI;

namespace Kawari.FeastHud;

// Native status icons have no recast timeline. Add a non-interactive native image
// using the same built-in recast atlas as the hotbar, with independently owned
// texture references. Never cast an IconText component to AtkComponentIcon.
internal sealed unsafe class NativeRecast
{
    private readonly Dictionary<nint, Entry> entries = [];
    private sealed class Entry
    {
        public nint Addon, Component, Image, Node, Parts;
    }
    public void Update(nint addon, DirectorFrame frame, byte team, long ageMs)
    {
        var players = frame.Players.Where(p => p.Team == team + 1).Take(4).ToArray();
        for (int row = 0; row < 5; row++)
        for (int slot = 0; slot < 3; slot++)
        {
            var component = *(AtkComponentBase**)((byte*)addon + 0x2D8 + row * 256 + slot * 8);
            if (component == null) continue;
            var action = row < players.Length && slot < players[row].Actions.Count && ageMs < 2500
                ? players[row].Actions[slot] : null;
            float remaining = action == null ? 0 : action.IsAdrenaline
                ? 100 - players[row].Adrenaline : Math.Max(0, action.RemainingTenths - ageMs / 100f);
            bool active = remaining > 0;
            var image = *(AtkImageNode**)((byte*)component + 0xC0);
            if (image == null || image->Type != NodeType.Image) continue;
            if (!entries.TryGetValue((nint)component, out var entry))
            {
                if (!active) continue;
                entry = Create(addon, component, image);
                entries.Add((nint)component, entry);
            }
            image->MultiplyRed = image->MultiplyGreen = image->MultiplyBlue = active ? (byte)50 : (byte)100;
            image->IsDirty = true;
            var node = (AtkImageNode*)entry.Node;
            node->ToggleVisibility(active);
            if (!active) continue;
            // ActionBar.uld part-list4: 9 columns, 44x46 parts, 48px row pitch.
            // Last atlas cell is the completion flash; use only progress cells.
            int cell = Math.Clamp((int)((1 - remaining / (action!.IsAdrenaline ? 100 : action.RecastTenths)) * 79), 0, 79);
            var part = ((AtkUldPartsList*)entry.Parts)->Parts;
            part->U = (ushort)(cell % 9 * 44); part->V = (ushort)(cell / 9 * 48);
            node->SetPositionFloat(image->X, image->Y);
            node->SetScale(image->Width * image->ScaleX / 44f, image->Height * image->ScaleY / 46f);
            node->IsDirty = true;
        }
    }
    private static Entry Create(nint addon, AtkComponentBase* component, AtkImageNode* image)
    {
        var node = AtkUldManager.CreateAtkImageNode();
        var parts = NativeNodes.CreateParts(width: 44, height: 46);
        parts->Parts->UldAsset->AtkTexture.LoadTexture("ui/uld/icona_recast.tex", 1);
        node->PartsList = parts; node->PartId = 0; node->Width = 44; node->Height = 46;
        node->NodeId = 0xFEA500;
        node->Type = NodeType.Image;
        node->WrapMode = 1; node->Flags = 0;
        node->Color.A = 110;
        node->NodeFlags = NodeFlags.Visible | NodeFlags.Enabled;
        // Insert immediately above the existing image, below timer text. No
        // collision/event nodes are added, so native row targeting is unchanged.
        NativeNodes.AddAbove((AtkResNode*)image, (AtkResNode*)node);
        component->UldManager.UpdateDrawNodeList();
        return new() { Addon = addon, Component = (nint)component, Image = (nint)image, Node = (nint)node, Parts = (nint)parts };
    }
    public void RemoveAddon(nint addon)
    {
        foreach (var key in entries.Where(e => e.Value.Addon == addon).Select(e => e.Key).ToArray()) Remove(key);
    }
    public void Clear() { foreach (var key in entries.Keys.ToArray()) Remove(key); }
    private void Remove(nint key)
    {
        var e = entries[key]; entries.Remove(key);
        var node = (AtkImageNode*)e.Node;
        var image = (AtkImageNode*)e.Image;
        image->MultiplyRed = image->MultiplyGreen = image->MultiplyBlue = 100;
        image->IsDirty = true;
        NativeNodes.Detach((AtkResNode*)node);
        ((AtkComponentBase*)e.Component)->UldManager.UpdateDrawNodeList();
        node->PartsList = null; node->Destroy(true);
        NativeNodes.FreeParts((AtkUldPartsList*)e.Parts);
    }
}
