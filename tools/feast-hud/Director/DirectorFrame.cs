using System.Buffers.Binary;

namespace Kawari.FeastHud;

// The match state the server sends through the duty director, decoded.
// Record layouts are documented in servers/world/src/server/feast/director.rs.

// The F1 summary: the match as this player sees it. Phase is prep, active,
// sudden, fang, claw or draw (the last three are results); Team is fang,
// claw or spectator; Ground is medals on the ground (from F3).
public sealed record MatchSummary(string Phase, int Seconds, int Fang, int Claw, string Team, int Medals, int Ground);
// F2 (plus F5 adrenaline and F4 party actions): one match participant.
public sealed record Participant(uint Id, byte Team, ushort Medals, byte Job, bool Alive)
{
    public byte Adrenaline { get; init; }
    public IReadOnlyList<PartyAction> Actions { get; init; } = [];
    public string JobAbbreviation => Job switch
    {
        19 => "PLD", 20 => "MNK", 21 => "WAR", 22 => "DRG", 23 => "BRD",
        24 => "WHM", 25 => "BLM", 27 => "SMN", 28 => "SCH", 30 => "NIN",
        31 => "MCH", 32 => "DRK", 33 => "AST", 34 => "SAM", 35 => "RDM",
        37 => "GNB", 38 => "DNC", 39 => "RPR", 40 => "SGE", 41 => "VPR",
        42 => "PCT", _ => "?",
    };
    public int MedalCapacity => Job switch
    {
        19 or 21 or 32 or 37 => 200,
        20 or 22 or 30 or 34 or 39 or 41 => 175,
        23 or 25 or 27 or 31 or 35 or 38 or 42 => 150,
        24 or 28 or 33 or 40 => 125,
        _ => 0,
    };
}
// F4: one of a participant's three party list action slots.
public sealed record PartyAction(byte Slot, ushort Id, ushort RemainingTenths)
{
    public bool IsAdrenaline => Id is >= 3359 and <= 3362 or 4249;
    public int RecastTenths => FeastJobActions.Find(Id) is { } action ? (int)(action.Recast * 10) : Id is 17832 or 18957 ? 450 : 300;
    // Verified against the archived 6.08 Action sheet.
    public int Icon => Id switch { 3362 => 784, 1584 => 9112, 17690 => 9339, 17832 => 9208,
        17688 => 9294, 18957 => 9526, 17689 => 9113, _ => FeastJobActions.Find(Id)?.Icon ?? 0 };
}
// F6: what a hotbar button currently becomes, with its cast and recast.
public sealed record HotbarAction(ushort BaseId, ushort ResolvedId, ushort CastMs, ushort RecastMs);
// F7: the player's own job gauge. Field meanings depend on the job (M.job_gauge in Lua).
public sealed record JobGauge(byte Job, byte Resource, byte Meditation, byte Polyglot, byte Element, bool Enochian, ushort Timer);
public sealed record DirectorFrame(MatchSummary Summary, IReadOnlyList<Participant> Players, int RespawnSeconds)
{
    public IReadOnlyList<HotbarAction> Hotbar { get; init; } = [];
    public JobGauge? Gauge { get; init; }
}

// Assembles frames from records. A frame starts with F1 and is published
// only when its F3 commit arrives, so the HUD never shows half an update.
// Any malformed or out-of-range record discards the whole frame.
public sealed class DirectorFrames
{
    private MatchSummary? pending;
    private readonly List<Participant> players = [];
    private readonly List<HotbarAction> hotbar = [];
    private JobGauge? jobGauge;
    private int respawn;
    public DirectorFrame? Current { get; private set; }
    public void Clear() { pending = null; players.Clear(); Current = null; }
    public bool Accept(ReadOnlySpan<byte> data)
    {
        if (data.Length != 10) return false;
        
        switch (data[0])
        {
            case 0xF1:
                pending = null; players.Clear(); hotbar.Clear(); jobGauge = null;
                int phase = data[1] & 7, team = data[1] >> 3;
                if (phase > 5 || team > 2 || BinaryPrimitives.ReadUInt16LittleEndian(data.Slice(2, 2)) > 360 || data[8] > 200 || data[9] > 10) return false;
                pending = new(new[] { "prep", "active", "sudden", "fang", "claw", "draw" }[phase],
                    BinaryPrimitives.ReadUInt16LittleEndian(data.Slice(2, 2)), BinaryPrimitives.ReadUInt16LittleEndian(data.Slice(4, 2)), BinaryPrimitives.ReadUInt16LittleEndian(data.Slice(6, 2)), new[] { "spectator", "fang", "claw" }[team], data[8], 0);
                respawn = data[9];
                return false;
            case 0xF2 when pending != null:
                uint id = BinaryPrimitives.ReadUInt32LittleEndian(data.Slice(2, 4));
                if (players.Count >= 8 || data[1] is not (1 or 2) || data[9] > 1 || BinaryPrimitives.ReadUInt16LittleEndian(data.Slice(6, 2)) > 200
                    || id == 0 || id == 0xE0000000 || players.Any(p => p.Id == id))
                { pending = null; return false; }
                players.Add(new(id, data[1], BinaryPrimitives.ReadUInt16LittleEndian(data.Slice(6, 2)), data[8], data[9] == 1));
                return false;
            case 0xF4 when pending != null:
                uint owner = BinaryPrimitives.ReadUInt32LittleEndian(data.Slice(1, 4));
                int index = players.FindIndex(p => p.Id == owner);
                var action = new PartyAction(data[5], BinaryPrimitives.ReadUInt16LittleEndian(data.Slice(6, 2)), BinaryPrimitives.ReadUInt16LittleEndian(data.Slice(8, 2)));
                if (index < 0 || action.Slot > 2 || action.Icon == 0 || action.RemainingTenths > action.RecastTenths
                    || players[index].Actions.Any(a => a.Slot == action.Slot))
                { pending = null; return false; }
                players[index] = players[index] with { Actions = players[index].Actions.Append(action).OrderBy(a => a.Slot).ToArray() };
                return false;
            case 0xF5 when pending != null:
                uint gaugeOwner = BinaryPrimitives.ReadUInt32LittleEndian(data.Slice(1, 4));
                int gaugeIndex = players.FindIndex(p => p.Id == gaugeOwner);
                if (gaugeIndex < 0 || data[5] > 100) { pending = null; return false; }
                players[gaugeIndex] = players[gaugeIndex] with { Adrenaline = data[5] };
                return false;
            case 0xF6 when pending != null:
                var slot = new HotbarAction(BinaryPrimitives.ReadUInt16LittleEndian(data.Slice(2,2)), BinaryPrimitives.ReadUInt16LittleEndian(data.Slice(4,2)),
                    BinaryPrimitives.ReadUInt16LittleEndian(data.Slice(6,2)), BinaryPrimitives.ReadUInt16LittleEndian(data.Slice(8,2)));
                if (hotbar.Count >= 64 || data[1] != 0 || FeastJobActions.Find(slot.BaseId) == null || FeastJobActions.Find(slot.ResolvedId) == null
                    || slot.CastMs > 10000 || slot.RecastMs > 60000 || hotbar.Any(a => a.BaseId == slot.BaseId))
                { pending = null; return false; }
                hotbar.Add(slot);
                return false;
            case 0xF7 when pending != null:
                if (jobGauge != null || !FeastJobActions.Supports(data[1]) || data[2] > (data[1] == 25 ? 150 : 100)
                    || data[3] > (data[1] is 35 or 39 ? 100 : data[1] is 23 or 38 ? 4 : 3)
                    || data[4] > (data[1] is 38 or 39 ? 5 : 2) || data[5] > (data[1] == 39 ? 5 : 2) || data[6] > 1
                    || (data[1] == 24 && (data[2] > 3 || BinaryPrimitives.ReadUInt16LittleEndian(data.Slice(7,2)) >= 10000))
                    || (data[1] == 22 && data[2] > 2)
                    || (data[1] == 20 && data[2] > 5)
                    || (data[1] is 27 or 37 && data[2] > 2)
                    || (data[1] == 40 && (data[2] > 3 || BinaryPrimitives.ReadUInt16LittleEndian(data.Slice(7,2)) >= 10000))
                    || (data[1] == 33 && data[2] is not (0 or 7 or 8))
                    || (data[1] == 38 && (data[4] > 4 || (data[7] & 15) > 4 || (data[7] >> 4) > 4 || (data[8] & 15) > 4 || (data[8] >> 4) > 4))
                    || BinaryPrimitives.ReadUInt16LittleEndian(data.Slice(7,2)) > (data[1] is 22 or 23 or 32 or 38 ? 30000 : 15000)) { pending = null; return false; }
                jobGauge = new(data[1],data[2],data[3],data[4],data[5],data[6] == 1,BinaryPrimitives.ReadUInt16LittleEndian(data.Slice(7,2)));
                return false;
            case 0xF3 when pending != null:
                Current = new(pending with { Ground = BinaryPrimitives.ReadUInt16LittleEndian(data.Slice(1, 2)) }, players.ToArray(), respawn) { Hotbar = hotbar.ToArray(), Gauge = jobGauge };
                pending = null;
                return true;
            default: pending = null; return false;
        }
    }
}

