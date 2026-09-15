using System.Buffers.Binary;

namespace Kawari.FeastHud;

public sealed record Participant(uint Id, byte Team, ushort Medals, byte Job, bool Alive)
{
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
public sealed record PartyAction(byte Slot, ushort Id, ushort RemainingTenths)
{
    public int RecastTenths => Id is 17832 or 18957 ? 450 : 300;
    // Verified against the archived 6.08 Action sheet.
    public int Icon => Id switch { 1584 => 9112, 17690 => 9339, 17832 => 9208,
        17688 => 9294, 18957 => 9526, 17689 => 9113, _ => 0 };
}
public sealed record DirectorFrame(Snapshot Summary, IReadOnlyList<Participant> Players, int RespawnSeconds);

// An ordered director frame is published only after its commit record arrives.
public sealed class DirectorFrames
{
    private Snapshot? pending;
    private readonly List<Participant> players = [];
    private int respawn;
    public DirectorFrame? Current { get; private set; }
    public void Clear() { pending = null; players.Clear(); Current = null; }
    public bool Accept(ReadOnlySpan<byte> data)
    {
        if (data.Length != 10) return false;
        
        switch (data[0])
        {
            case 0xF1:
                pending = null; players.Clear();
                int phase = data[1] & 7, team = data[1] >> 3;
                if (phase > 5 || team > 2 || BinaryPrimitives.ReadUInt16LittleEndian(data.Slice(2, 2)) > 360 || data[8] > 200 || data[9] > 10) return false;
                pending = new(new[] { "prep", "active", "sudden", "fang", "claw", "draw" }[phase],
                    BinaryPrimitives.ReadUInt16LittleEndian(data.Slice(2, 2)), BinaryPrimitives.ReadUInt16LittleEndian(data.Slice(4, 2)), BinaryPrimitives.ReadUInt16LittleEndian(data.Slice(6, 2)), new[] { "spectator", "fang", "claw" }[team], data[8], 0, 0);
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
                if (index < 0 || action.Slot > 2 || action.Icon == 0 || action.RemainingTenths > 450
                    || players[index].Actions.Any(a => a.Slot == action.Slot))
                { pending = null; return false; }
                players[index] = players[index] with { Actions = players[index].Actions.Append(action).OrderBy(a => a.Slot).ToArray() };
                return false;
            case 0xF3 when pending != null:
                Current = new(pending with { Ground = BinaryPrimitives.ReadUInt16LittleEndian(data.Slice(1, 2)) }, players.ToArray(), respawn);
                pending = null;
                return true;
            default: pending = null; return false;
        }
    }
}

