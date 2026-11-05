using Dalamud.Game.ClientState.Objects.Types;
using Dalamud.Plugin.Services;
using FFXIVClientStructs.FFXIV.Component.GUI;

namespace Kawari.FeastHud;

// Native image nodes at the ten status positions from the recovered Feast ULD.
// They are visual children of the native row, with no independent mouse handler.
internal sealed unsafe class NativeStatusStrip
{
    private sealed class Entry
    {
        public nint Addon, Component;
        public readonly List<nint> Images = [];
        public readonly uint[] Icons = new uint[10];
    }
    private readonly Dictionary<nint, Entry> entries = [];

    public void Update(nint addon, AtkComponentBase* component, IBattleChara? actor, IDataManager data)
    {
        if (!entries.TryGetValue((nint)component, out var entry))
        {
            if (actor == null) return;
            entries[(nint)component] = entry = new() { Addon=addon, Component=(nint)component };
            for (int i=0; i<10; i++)
            {
                var image = AtkUldManager.CreateAtkImageNode();
                image->PartsList = NativeNodes.CreateParts(); image->PartId = 0;
                image->Type = NodeType.Image; image->NodeId = (uint)(0xFEA600+i);
                image->WrapMode = 1; image->Flags = ImageNodeFlags.AutoFit;
                image->Width = 24; image->Height = 32;
                image->SetPositionFloat(138+i*24,60);
                image->NodeFlags = NodeFlags.Enabled;
                // On top of the row's native children. No event/collision node added.
                NativeNodes.AddOnTop(component->UldManager.RootNode, (AtkResNode*)image);
                entry.Images.Add((nint)image);
            }
            component->UldManager.UpdateDrawNodeList();
        }
        var statuses = actor?.StatusList.Where(s => s.StatusId != 0).Take(10).ToArray() ?? [];
        var sheet = data.GetExcelSheet<Lumina.Excel.Sheets.Status>();
        for (int i=0; i<10; i++)
        {
            var image = (AtkImageNode*)entry.Images[i];
            uint icon = i < statuses.Length ? sheet?.GetRowOrDefault(statuses[i].StatusId)?.Icon ?? 0 : 0;
            image->ToggleVisibility(icon != 0);
            if (icon == 0 || entry.Icons[i] == icon) continue;
            image->LoadIconTexture(icon,0);
            entry.Icons[i] = icon;
        }
    }
    public void Remove(nint addon)
    {
        foreach (var key in entries.Where(p=>p.Value.Addon==addon).Select(p=>p.Key).ToArray())
        {
            var entry = entries[key]; entries.Remove(key);
            foreach (var address in entry.Images)
            {
                var image = (AtkImageNode*)address;
                var parts = image->PartsList;
                NativeNodes.Detach((AtkResNode*)image);
                image->PartsList = null; image->Destroy(true);
                NativeNodes.FreeParts(parts);
            }
            ((AtkComponentBase*)entry.Component)->UldManager.UpdateDrawNodeList();
        }
    }
    public void Clear() { foreach (var addon in entries.Values.Select(e=>e.Addon).Distinct().ToArray()) Remove(addon); }
}
