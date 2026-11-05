using FFXIVClientStructs.FFXIV.Client.System.Memory;
using FFXIVClientStructs.FFXIV.Component.GUI;
using FFXIVClientStructs.FFXIV.Client.Graphics.Kernel;
using Lumina.Data;
using Lumina.Data.Files;
using Lumina.Data.Structs;

namespace Kawari.FeastHud;

// Recovered PvPColosseumHeader ULD geometry and original atlases, rendered by Atk.
// The surviving header owns HUD placement/lifecycle; none of its CC data is used.
internal sealed unsafe class NativeMatchHeader
{
    private readonly string assetRoot;
    private readonly Dictionary<string, (int Width, int Height, byte[] Pixels)> atlases = [];
    private nint owner;
    private readonly List<nint> nodes = [];
    private readonly Dictionary<nint, bool> hidden = [];
    private readonly List<nint> textures = [];
    private readonly nint[] names = new nint[2], scores = new nint[2], gauges = new nint[2];
    private readonly nint[] icons = new nint[8], medals = new nint[8];
    private readonly uint[] loadedIcons = new uint[8];
    private nint clock, clockSeconds, phase;
    private ushort width, height;

    public NativeMatchHeader(string pluginDirectory)
    {
        assetRoot = Path.Combine(pluginDirectory, "assets");
        foreach (var file in new[] { "pvpcolosseumheader.tex", "pvpcolosseumgauge.tex" })
        {
            if (!File.Exists(Path.Combine(assetRoot, file)))
                throw new FileNotFoundException("Recovered Feast header asset missing", Path.Combine(assetRoot, file));
            var bytes = File.ReadAllBytes(Path.Combine(assetRoot,file));
            var tex = new TexFile();
            typeof(FileResource).GetProperty(nameof(FileResource.Data))!.SetValue(tex,bytes);
            typeof(FileResource).GetProperty(nameof(FileResource.Reader))!.SetValue(tex,new LuminaBinaryReader(bytes,PlatformId.Win32));
            tex.LoadFile();
            var pixels = tex.ImageData;
            if (pixels.Length != tex.Header.Width * tex.Header.Height * 4)
                throw new InvalidDataException($"Unexpected Feast atlas pixel size: {file}");
            atlases[file] = (tex.Header.Width,tex.Header.Height,pixels);
        }
    }

    private void Attach(AtkResNode* n, float x, float y, ushort w, ushort h)
    {
        var root = ((AtkUnitBase*)owner)->RootNode;
        n->NodeId = (uint)(0xFEA700 + nodes.Count);
        n->NodeFlags = NodeFlags.Enabled;
        n->SetPositionFloat(x,y); n->SetWidth(w); n->SetHeight(h); n->SetScale(1,1);
        n->ToggleVisibility(true);
        // Added in paint order: banner, gauges, then foreground text.
        NativeNodes.AddAtBottom(root, n);
        nodes.Add((nint)n);
    }
    private AtkUldPartsList* Part(string? file, ushort u, ushort v, ushort w, ushort h)
    {
        var p = NativeNodes.CreateParts(u, v, w, h);
        textures.Add((nint)p);
        if (file != null)
        {
            // Upload original BGRA pixels directly. Missing SqPack paths must
            // not go through the game's packed-resource loader.
            var atlas = atlases[file];
            var texture = Texture.CreateTexture2D(atlas.Width,atlas.Height,1,
                TextureFormat.B8G8R8A8_UNORM,TextureFlags.TextureType2D | TextureFlags.Managed,0);
            if (texture == null) throw new InvalidOperationException("Unable to allocate Feast atlas texture");
            fixed (byte* pixels = atlas.Pixels)
                if (!texture->InitializeContents(pixels))
                { texture->DecRef(); throw new InvalidOperationException("Unable to upload Feast atlas texture"); }
            p->Parts->UldAsset->AtkTexture.KernelTexture = texture;
            p->Parts->UldAsset->AtkTexture.TextureType = TextureType.KernelTexture;
        }
        return p;
    }
    private nint Image(string? file, float x, float y, ushort w, ushort h, ushort tw, ushort th)
    {
        var parts = Part(file,0,0,tw,th);
        var n = IMemorySpace.GetUISpace()->Create<AtkImageNode>();
        if (n == null) throw new OutOfMemoryException();
        n->Type = NodeType.Image; n->PartsList = parts;
        n->PartId = 0; n->WrapMode = 1; n->Flags = file == null ? ImageNodeFlags.AutoFit : 0;
        Attach((AtkResNode*)n,x,y,w,h); return (nint)n;
    }
    private nint Gauge(float x, float y, bool border, bool red)
    {
        var parts = Part("pvpcolosseumgauge.tex",1,(ushort)(border?8:1),(ushort)(border?6:102),(ushort)(border?10:6));
        var n = IMemorySpace.GetUISpace()->Create<AtkNineGridNode>();
        if (n == null) throw new OutOfMemoryException();
        n->Type = NodeType.NineGrid; n->PartId = 0;
        n->PartsList = parts;
        n->LeftOffset = n->RightOffset = (short)(border?2:1);
        Attach((AtkResNode*)n,x,y,(ushort)(border?342:340),(ushort)(border?10:6));
        if (!border)
        {
            // The recovered fill is grey. Multiplication alone leaves it dark;
            // native additive tint gives the blue/red illuminated medal rails.
            n->MultiplyRed = n->MultiplyGreen = n->MultiplyBlue = 35;
            n->AddRed = (short)(red ? 210 : 30);
            n->AddGreen = (short)(red ? 35 : 145);
            n->AddBlue = (short)(red ? 35 : 210);
            n->IsDirty = true;
        }
        return (nint)n;
    }
    private nint Text(float x, float y, ushort w, ushort h, byte size, byte alignment = 3, byte font = 0)
    {
        var n = IMemorySpace.GetUISpace()->Create<AtkTextNode>();
        if (n == null) throw new OutOfMemoryException();
        n->Type = NodeType.Text; n->TextFlags = TextFlags.Edge;
        n->TextColor.R = n->TextColor.G = n->TextColor.B = n->TextColor.A = 255;
        n->EdgeColor.A = 255; n->FontSize = size; n->LineSpacing = size;
        n->AlignmentFontType = (byte)(alignment | (font << 4));
        Attach((AtkResNode*)n,x,y,w,h); return (nint)n;
    }
    private static void Set(nint address, string value)
    {
        var n = (AtkTextNode*)address;
        if (n->NodeText.ToString() != value) n->SetText(value);
    }
    private void Build(AtkUnitBase* addon)
    {
        owner = (nint)addon; width = addon->RootNode->Width; height = addon->RootNode->Height;
        // Preserve the original CC tree for teardown. Never change its IDs or free it.
        for (var n = addon->RootNode->ChildNode; n != null; n = n->PrevSiblingNode)
            hidden[(nint)n] = n->IsVisible();
        addon->RootNode->SetWidth(920); addon->RootNode->SetHeight(60);
        Image("pvpcolosseumheader.tex",13,-16,894,104,894,104);
        for (int side=0; side<2; side++)
        {
            var x = side == 0 ? 60 : 520;
            Gauge(x-1,18,true,side==1);
            gauges[side] = Gauge(x,20,false,side==1);
            names[side] = Text(x,0,340,20,16,(byte)(side==0?3:5));
            scores[side] = Text(side==0?344:512,27,64,28,24,4);
            for (int i=0; i<4; i++)
            {
                int slot = side*4+i;
                var px = (side==0?58:584) + i*72;
                icons[slot] = Image(null,px-2,24,32,32,32,32);
                medals[slot] = Text(px+25,28,40,24,18);
            }
        }
        // Native numeric font has no colon glyph; render separator separately.
        clock = Text(405,0,44,52,40,5,5);
        Set(Text(449,0,18,52,32,4), ":");
        clockSeconds = Text(467,0,60,52,40,3,5);
        ((AtkTextNode*)clock)->EdgeColor.R = 130;
        ((AtkTextNode*)clock)->EdgeColor.G = 90;
        ((AtkTextNode*)clock)->EdgeColor.B = 35;
        phase = Text(310,54,300,18,12,4);
        addon->UldManager.UpdateDrawNodeList();
    }
    public void Update(nint address, DirectorFrame? frame, long age)
    {
        var addon = (AtkUnitBase*)address;
        if (addon->RootNode == null) return;
        if (owner != address) { Clear(); Build(addon); }
        foreach (var n in hidden.Keys) ((AtkResNode*)n)->ToggleVisibility(false);
        var state = MatchHeaderState.From(frame,age);
        var clockParts = state.Clock.Split(':');
        Set(clock,clockParts[0]); Set(clockSeconds,clockParts[1]); Set(phase,state.Phase);
        for (int side=0; side<2; side++)
        {
            var team = side==0 ? state.Left : state.Right;
            Set(names[side],team.Name); Set(scores[side],team.Name.Length==0 ? "--" : team.Score.ToString());
            var gauge = (AtkResNode*)gauges[side];
            var fill = MatchHeaderState.GaugeWidth(team.Score);
            gauge->SetWidth(fill); gauge->ToggleVisibility(fill>0);
            gauge->SetPositionFloat(side==0?60:860-fill,20);
            for (int i=0;i<4;i++)
            {
                int slot = side*4+i;
                var p = i<team.Players.Count ? team.Players[i] : null;
                var icon = (AtkImageNode*)icons[slot];
                icon->ToggleVisibility(p!=null);
                Set(medals[slot],p?.Medals.ToString() ?? "");
                uint id = p == null ? 0 : (uint)(62100+p.Job);
                if (id!=0 && loadedIcons[slot]!=id) { icon->LoadIconTexture(id,0); loadedIcons[slot]=id; }
            }
        }
    }
    public void Remove(nint address) { if (owner==address) Clear(); }
    public void Clear()
    {
        if (owner==0) return;
        var addon = (AtkUnitBase*)owner;
        foreach (var address in nodes.AsEnumerable().Reverse())
        {
            var n = (AtkResNode*)address;
            NativeNodes.Detach(n);
            if (n->Type==NodeType.Image) ((AtkImageNode*)n)->PartsList=null;
            if (n->Type==NodeType.NineGrid) ((AtkNineGridNode*)n)->PartsList=null;
            n->Destroy(true);
        }
        foreach (var address in textures) NativeNodes.FreeParts((AtkUldPartsList*)address);
        foreach (var (n,visible) in hidden) ((AtkResNode*)n)->ToggleVisibility(visible);
        addon->RootNode->SetWidth(width); addon->RootNode->SetHeight(height);
        addon->UldManager.UpdateDrawNodeList();
        nodes.Clear(); textures.Clear(); hidden.Clear(); Array.Clear(loadedIcons); owner=0;
    }
}
