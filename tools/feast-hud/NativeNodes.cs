using System.Runtime.InteropServices;
using FFXIVClientStructs.FFXIV.Component.GUI;

namespace Kawari.FeastHud;

// Adding our own nodes to the game's UI trees, and removing them again.
//
// Atk keeps a node's children as a linked list: the parent's ChildNode is the
// last child drawn (on top), and PrevSiblingNode walks towards the first.
// Every node we insert must be detached and destroyed before its addon is
// finalized or the plugin unloads; the classes that add nodes keep a list.
internal static unsafe class NativeNodes
{
    // Adds node as the parent's top child (drawn last).
    public static void AddOnTop(AtkResNode* parent, AtkResNode* node)
    {
        node->ParentNode = parent;
        node->PrevSiblingNode = parent->ChildNode;
        if (parent->ChildNode != null) parent->ChildNode->NextSiblingNode = node;
        parent->ChildNode = node;
        parent->ChildCount++;
    }

    // Adds node as the parent's bottom child (drawn first).
    public static void AddAtBottom(AtkResNode* parent, AtkResNode* node)
    {
        node->ParentNode = parent;
        var bottom = parent->ChildNode;
        if (bottom == null) parent->ChildNode = node;
        else
        {
            while (bottom->PrevSiblingNode != null) bottom = bottom->PrevSiblingNode;
            bottom->PrevSiblingNode = node;
            node->NextSiblingNode = bottom;
        }
        parent->ChildCount++;
    }

    // Adds node directly above sibling, under the same parent.
    public static void AddAbove(AtkResNode* sibling, AtkResNode* node)
    {
        node->ParentNode = sibling->ParentNode;
        node->PrevSiblingNode = sibling;
        node->NextSiblingNode = sibling->NextSiblingNode;
        if (sibling->NextSiblingNode != null) sibling->NextSiblingNode->PrevSiblingNode = node;
        sibling->NextSiblingNode = node;
        if (node->ParentNode != null)
        {
            if (node->ParentNode->ChildNode == sibling) node->ParentNode->ChildNode = node;
            node->ParentNode->ChildCount++;
        }
    }

    // Unlinks node from its parent and siblings. The caller destroys it.
    public static void Detach(AtkResNode* node)
    {
        if (node->PrevSiblingNode != null) node->PrevSiblingNode->NextSiblingNode = node->NextSiblingNode;
        if (node->NextSiblingNode != null) node->NextSiblingNode->PrevSiblingNode = node->PrevSiblingNode;
        if (node->ParentNode != null)
        {
            if (node->ParentNode->ChildNode == node) node->ParentNode->ChildNode = node->PrevSiblingNode;
            node->ParentNode->ChildCount--;
        }
        node->ParentNode = node->PrevSiblingNode = node->NextSiblingNode = null;
    }

    // A one-part parts list with its own texture slot, for an image or nine-grid node.
    public static AtkUldPartsList* CreateParts(ushort u = 0, ushort v = 0, ushort width = 0, ushort height = 0)
    {
        var parts = (AtkUldPartsList*)NativeMemory.AllocZeroed((nuint)sizeof(AtkUldPartsList));
        parts->PartCount = 1;
        parts->Parts = (AtkUldPart*)NativeMemory.AllocZeroed((nuint)sizeof(AtkUldPart));
        parts->Parts->U = u; parts->Parts->V = v; parts->Parts->Width = width; parts->Parts->Height = height;
        parts->Parts->UldAsset = (AtkUldAsset*)NativeMemory.AllocZeroed((nuint)sizeof(AtkUldAsset));
        parts->Parts->UldAsset->AtkTexture.Ctor();
        return parts;
    }

    // Releases a parts list from CreateParts, including its texture.
    public static void FreeParts(AtkUldPartsList* parts)
    {
        parts->Parts->UldAsset->AtkTexture.Destroy(false);
        NativeMemory.Free(parts->Parts->UldAsset);
        NativeMemory.Free(parts->Parts);
        NativeMemory.Free(parts);
    }
}
