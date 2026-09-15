using Kawari.FeastHud;
var valid=new[]{"KFEASTHUD1|prep|60|100|100|fang|100|125|0","KFEASTHUD1|sudden|120|200|200|spectator|0|0|50","KFEASTHUD1|claw|0|50|150|claw|150|200|0"};
foreach(var text in valid)if(!Snapshot.TryParse(text,out _))throw new Exception("Rejected valid snapshot: "+text);
var invalid=new[]{"someone says KFEASTHUD1|prep|60|100|100|fang|100|125|0","KFEASTHUD2|prep|60|100|100|fang|100|125|0","KFEASTHUD1|bad|60|100|100|fang|100|125|0","KFEASTHUD1|active|-1|100|100|fang|100|125|0","KFEASTHUD1|active|361|100|100|fang|100|125|0","KFEASTHUD1|active|10|100|100|fang|126|125|0","KFEASTHUD1|active|10|100|100|fang|100|125|999999999999","KFEASTHUD1|active|10|100|100|fang|100|125"};
foreach(var text in invalid)if(Snapshot.TryParse(text,out _))throw new Exception("Accepted malformed snapshot: "+text);
Console.WriteLine("11 HUD protocol cases passed.");
var frames = new DirectorFrames();
byte[] summary = [0xF1, 8, 9, 0, 100, 0, 100, 0, 100, 2];
byte[] fang = [0xF2, 1, 1, 0, 0, 0, 100, 0, 24, 1];
byte[] claw = [0xF2, 2, 2, 0, 0, 0, 100, 0, 24, 0];
byte[] commit = [0xF3, 25, 0, 0, 0, 0, 0, 0, 0, 0];
void Check(bool condition, string name) { if (!condition) throw new Exception(name); }
Check(!frames.Accept(summary) && frames.Current == null, "Summary must wait for commit");
Check(!frames.Accept(fang) && !frames.Accept(claw), "Roster must wait for commit");
Check(frames.Accept(commit), "Complete frame accepted");
Check(frames.Current!.Summary is { Seconds: 9, Fang: 100, Claw: 100, Ground: 25, Team: "fang" }
    && frames.Current.Players.Count == 2 && frames.Current.RespawnSeconds == 2, "Cross-language byte fixture");
frames.Accept(summary); frames.Accept(fang); frames.Accept(fang);
Check(!frames.Accept(commit), "Duplicate participant invalidates frame");
frames.Accept(summary); frames.Accept(claw); frames.Accept(commit);
Check(frames.Current!.Players.Count == 1 && frames.Current.Players[0].Id == 2, "Departed participants removed");
frames.Accept(summary); frames.Accept(new byte[10]);
Check(!frames.Accept(commit), "Unknown record invalidates pending frame");
frames.Clear();
Check(frames.Current == null && !frames.Accept(commit), "Duty exit clears frame");
Console.WriteLine("8 director framing cases passed.");
byte[] cooldown = [0xF4, 1, 0, 0, 0, 0, 0x30, 6, 0x2B, 1]; // Purify, 29.9s
frames.Accept(summary); frames.Accept(fang); frames.Accept(cooldown);
Check(frames.Current == null, "Cooldowns wait for atomic commit");
frames.Accept(commit);
Check(frames.Current!.Players[0].Actions.Single() is { Id: 1584, RemainingTenths: 299, Icon: 9112 }, "Cooldown bytes decoded");
frames.Accept(summary); frames.Accept(fang); frames.Accept(cooldown); frames.Accept(cooldown);
Check(!frames.Accept(commit), "Duplicate cooldown slot rejects frame");
frames.Accept(summary); frames.Accept(cooldown);
Check(!frames.Accept(commit), "Unknown cooldown owner rejects frame");
frames.Accept(summary); frames.Accept(fang); frames.Accept(commit);
Check(frames.Current!.Players[0].Actions.Count == 0, "Loadout cooldowns cannot leak between frames");
Console.WriteLine("5 cooldown framing cases passed.");
