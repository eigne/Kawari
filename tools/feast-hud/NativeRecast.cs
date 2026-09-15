using System.Runtime.InteropServices;
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
        public nint Addon, Component, Image, Node, Parts, Part, Asset;
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
            float remaining = action == null ? 0 : Math.Max(0, action.RemainingTenths - ageMs / 100f);
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
            int cell = Math.Clamp((int)((1 - remaining / action!.RecastTenths) * 79), 0, 79);
            var part = (AtkUldPart*)entry.Part;
            part->U = (ushort)(cell % 9 * 44); part->V = (ushort)(cell / 9 * 48);
            node->SetPositionFloat(image->X, image->Y);
            node->SetScale(image->Width * image->ScaleX / 44f, image->Height * image->ScaleY / 46f);
            node->IsDirty = true;
        }
    }
    private static Entry Create(nint addon, AtkComponentBase* component, AtkImageNode* image)
    {
        var node = AtkUldManager.CreateAtkImageNode();
        var parts = (AtkUldPartsList*)NativeMemory.AllocZeroed((nuint)sizeof(AtkUldPartsList));
        var part = (AtkUldPart*)NativeMemory.AllocZeroed((nuint)sizeof(AtkUldPart));
        var asset = (AtkUldAsset*)NativeMemory.AllocZeroed((nuint)sizeof(AtkUldAsset));
        asset->AtkTexture.Ctor(); asset->AtkTexture.LoadTexture("ui/uld/icona_recast.tex", 1);
        part->UldAsset = asset; part->Width = 44; part->Height = 46;
        parts->PartCount = 1; parts->Parts = part;
        node->PartsList = parts; node->PartId = 0; node->Width = 44; node->Height = 46;
        node->NodeId = 0xFEA500;
        node->Color.A = 110;
        node->NodeFlags = NodeFlags.Visible | NodeFlags.Enabled;
        // Insert immediately above the existing image, below timer text. No
        // collision/event nodes are added, so native row targeting is unchanged.
        node->ParentNode = image->ParentNode;
        node->PrevSiblingNode = (AtkResNode*)image;
        node->NextSiblingNode = image->NextSiblingNode;
        if (image->NextSiblingNode != null) image->NextSiblingNode->PrevSiblingNode = (AtkResNode*)node;
        image->NextSiblingNode = (AtkResNode*)node;
        if (node->ParentNode != null)
        {
            if (node->ParentNode->ChildNode == (AtkResNode*)image) node->ParentNode->ChildNode = (AtkResNode*)node;
            node->ParentNode->ChildCount++;
        }
        component->UldManager.UpdateDrawNodeList();
        return new() { Addon = addon, Component = (nint)component, Image = (nint)image, Node = (nint)node,
            Parts = (nint)parts, Part = (nint)part, Asset = (nint)asset };
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
        if (node->PrevSiblingNode != null) node->PrevSiblingNode->NextSiblingNode = node->NextSiblingNode;
        if (node->NextSiblingNode != null) node->NextSiblingNode->PrevSiblingNode = node->PrevSiblingNode;
        if (node->ParentNode != null)
        {
            if (node->ParentNode->ChildNode == (AtkResNode*)node) node->ParentNode->ChildNode = node->PrevSiblingNode;
            node->ParentNode->ChildCount--;
        }
        node->ParentNode = node->PrevSiblingNode = node->NextSiblingNode = null;
        ((AtkComponentBase*)e.Component)->UldManager.UpdateDrawNodeList();
        node->PartsList = null; node->Destroy(true);
        ((AtkUldAsset*)e.Asset)->AtkTexture.Destroy(false);
        NativeMemory.Free((void*)e.Asset); NativeMemory.Free((void*)e.Part); NativeMemory.Free((void*)e.Parts);
    }
}
