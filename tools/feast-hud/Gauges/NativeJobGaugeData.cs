using System.Buffers.Binary;
using System.Text;

namespace Kawari.FeastHud;

// Payloads of the retained native JobHud data readers (not JobGaugeManager).
// Offsets match the client structs; the first eight bytes are a vtable and
// must never be cleared. BRD's borrowed SongTitle pointer is also untouched.
internal static class NativeJobGaugeData
{
    internal sealed record Layout(string Name, byte Job, int Size, bool Visible = true)
    {
        internal byte[] NameBytes { get; } = Encoding.ASCII.GetBytes(Name);
    }
    internal static readonly Layout[] Layouts = [
        new("JobHudPLD0",19,0x18), new("JobHudMNK1",20,0x18), new("JobHudMNK0",20,0x38,false),
        new("JobHudWAR0",21,0x18), new("JobHudDRG0",22,0x20), new("JobHudBRD0",23,0x38),
        new("JobHudWHM0",24,0x20), new("JobHudBLM0",25,0x38), new("JobHudBLM1",25,0x10,false),
        new("JobHudSMN0",27,0x10), new("JobHudSMN1",27,0x38),
        new("JobHudSCH0",28,0x20), new("JobHudACN0",28,0x10,false),
        new("JobHudNIN0",30,0x18), new("JobHudNIN1v70",30,0x10,false),
        new("JobHudMCH0",31,0x38), new("JobHudDRK0",32,0x18), new("JobHudDRK1",32,0x18),
        new("JobHudAST0",33,0x20), new("JobHudSAM0",34,0x18), new("JobHudSAM1",34,0x10,false),
        new("JobHudRDM0",35,0x28), new("JobHudGNB0",37,0x18),
        new("JobHudDNC0",38,0x30), new("JobHudDNC1",38,0x20),
        new("JobHudRRP0",39,0x28), new("JobHudRRP1",39,0x18),
        new("JobHudGFF0",40,0x10), new("JobHudGFF1",40,0x20),
    ];

    internal static Layout? Identify(ReadOnlySpan<byte> name, uint job)
    {
        int end = name.IndexOf((byte)0);
        if (end < 0) return null;
        foreach (var layout in Layouts)
            if (layout.Job == job && name[..end].SequenceEqual(layout.NameBytes)) return layout;
        return null;
    }

    internal static bool Write(Layout layout, JobGauge gauge, Span<byte> destination)
    {
        if (gauge.Job != layout.Job || destination.Length < layout.Size) return false;
        destination.Slice(8, layout.Size - 8).Clear();
        // Stack-local writer keeps these mappings testable without a game process.
        var w = new Writer(destination);
        if (!layout.Visible) return true;
        int r = gauge.Resource, m = gauge.Meditation, t = gauge.Timer, e = gauge.Element;
        w.Flag(8, true);
        switch (layout.Name)
        {
            case "JobHudPLD0": w.Flag(8,false); w.Flag(9,true); w.Flag(10,true); w.Int(12,r); w.Int(16,100); break;
            case "JobHudWAR0": w.Flag(9,true); w.Int(12,r); w.Int(16,100); break;
            case "JobHudMNK1": w.Int(12,r); w.Flag(16,r==5); break;
            case "JobHudNIN0": w.Int(12,r); w.Int(16,100); w.Int(20,50); break;
            case "JobHudDRG0": w.Flag(9,true); w.Int(12,e); w.Int(16,t); w.Int(20,30000); w.Int(24,r); break;
            case "JobHudBRD0":
                w.Flag(9,e!=0); w.Flag(11,true); w.Int(16,r); w.Int(20,50); w.Int(28,100);
                w.Int(32,e==1?3:e==2?2:0); w.Int(36,t); w.Int(40,30000); w.Int(44,m); break;
            case "JobHudWHM0": w.Flag(9,true); w.Int(12,r); w.Int(16,m); w.Int(20,t); w.Int(24,10000); break;
            case "JobHudBLM0":
                w.Flag(9,e!=0); w.Flag(10,gauge.Enochian); w.Flag(11,true);
                w.Int(16,e==1?1:e==2?-1:0); w.Int(20,1); w.Int(24,e==0?0:r*100); w.Int(28,15000);
                w.Int(36,t); w.Int(40,15000); w.Int(44,gauge.Polyglot); w.Int(48,2); break;
            case "JobHudSMN0": w.Int(8,r); w.Flag(12,true); break;
            case "JobHudSMN1": w.Flag(9,true); w.Int(12,e); w.Int(16,t); w.Int(20,15000); break;
            case "JobHudSCH0": w.Int(8,r); w.Int(12,100); w.Flag(17,true); w.Flag(18,true); w.Int(20,50); w.Int(28,20000); break;
            case "JobHudMCH0": w.Int(12,r); w.Int(16,100); w.Int(20,50); w.Flag(24,e!=0); w.Int(28,t); w.Int(40,100); w.Int(44,50); break;
            case "JobHudDRK0": w.Flag(9,true); w.Int(12,r); w.Int(16,100); w.Int(20,50); break;
            case "JobHudDRK1": w.Flag(9,true); w.Flag(10,e!=0); w.Int(12,t); w.Int(16,30000); break;
            case "JobHudAST0": w.Flag(9,true); w.Int(24,r); break;
            case "JobHudSAM0": w.Flag(10,true); w.Int(12,m); w.Int(16,r); w.Int(20,100); break;
            case "JobHudRDM0": w.Int(12,r); w.Int(16,m); w.Int(20,100); w.Int(24,50); w.Int(28,50); break;
            case "JobHudGNB0": w.Flag(9,true); w.Int(16,r); break; // Feast has two cartridges.
            case "JobHudDNC0":
                w.Flag(9,e!=0); w.Int(12,e); w.Int(16,gauge.Polyglot);
                for(int i=0;i<4;i++) w.Int(20+4*i,(t>>(4*i))&15);
                break;
            case "JobHudDNC1": w.Flag(9,true); w.Int(12,m); w.Int(16,r); w.Int(20,100); w.Int(24,50); break;
            case "JobHudRRP0": w.Flag(9,true); w.Int(12,r); w.Int(16,m); w.Int(20,100); w.Int(24,100); w.Int(28,50); w.Int(32,50); w.Flag(36,t>0); break;
            case "JobHudRRP1": w.Flag(9,t>0); w.Int(12,gauge.Polyglot); w.Int(16,e); w.Int(20,t); break;
            case "JobHudGFF0": w.Flag(9,e!=0); break;
            case "JobHudGFF1": w.Int(12,r); w.Int(20,t); w.Int(24,10000); w.Flag(28,true); break;
            default: return false;
        }
        return true;
    }

    private readonly ref struct Writer
    {
        private readonly Span<byte> data;
        internal Writer(Span<byte> data) { this.data = data; }
        internal void Int(int offset,int value) => BinaryPrimitives.WriteInt32LittleEndian(data.Slice(offset,4),value);
        internal void Flag(int offset,bool value) => data[offset]=(byte)(value?1:0);
    }
}
