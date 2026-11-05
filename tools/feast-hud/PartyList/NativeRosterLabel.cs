using FFXIVClientStructs.FFXIV.Component.GUI;
using FFXIVClientStructs.FFXIV.Client.System.Memory;

namespace Kawari.FeastHud;

// A native text child, independent of the CC name/spectator timeline. The
// existing row still owns all collision nodes and target-selection events.
internal sealed unsafe class NativeRosterLabel
{
    private sealed record Entry(nint Addon, nint Component, nint Text);
    private readonly Dictionary<nint, Entry> entries = [];

    public void Update(nint addon, AtkComponentBase* component, string? label, bool casting = false)
    {
        if (!entries.TryGetValue((nint)component, out var entry))
        {
            if (label == null) return;
            // The ULD factory leaves text-specific padding/state uninitialized;
            // its loader normally fills that in. These are standalone nodes.
            var text = IMemorySpace.GetUISpace()->Create<AtkTextNode>();
            if (text == null) return;
            text->Type = NodeType.Text;
            text->NodeId = 0xFEA610;
            text->NodeFlags = NodeFlags.Enabled;
            text->TextId = 0;
            text->TextFlags = TextFlags.Edge;
            text->TextColor.R = text->TextColor.G = text->TextColor.B = text->TextColor.A = 255;
            text->EdgeColor.R = text->EdgeColor.G = text->EdgeColor.B = 0;
            text->EdgeColor.A = 255;
            text->BackgroundColor = default;
            text->FontSize = 14; text->LineSpacing = 14; text->CharSpacing = 0;
            text->AlignmentFontType = 0; text->SheetType = 0;
            text->FontCacheHandle = 0;
            text->Width = 200; text->Height = 24;
            text->SetPositionFloat(74,22);
            NativeNodes.AddOnTop(component->UldManager.RootNode, (AtkResNode*)text);
            entries[(nint)component] = entry = new(addon,(nint)component,(nint)text);
            component->UldManager.UpdateDrawNodeList();
        }
        var node = (AtkTextNode*)entry.Text;
        node->ToggleVisibility(label != null);
        // A cast name sits right-aligned just below the cast bar, as in native PvP.
        node->AlignmentFontType = (byte)((node->AlignmentFontType & 0xF0) | (casting ? 2 : 0));
        node->SetPositionFloat(74, casting ? 24 : 22);
        if (label != null && node->NodeText.ToString() != label) node->SetText(label);
    }

    public void Remove(nint addon)
    {
        foreach (var key in entries.Where(p => p.Value.Addon == addon).Select(p => p.Key).ToArray())
        {
            var entry = entries[key]; entries.Remove(key);
            var text = (AtkTextNode*)entry.Text;
            NativeNodes.Detach((AtkResNode*)text);
            text->Destroy(true);
            ((AtkComponentBase*)entry.Component)->UldManager.UpdateDrawNodeList();
        }
    }
}
