using Kawari.FeastHud;
var valid=new[]{"KFEASTHUD1|prep|60|100|100|fang|100|125|0","KFEASTHUD1|sudden|120|200|200|spectator|0|0|50","KFEASTHUD1|claw|0|50|150|claw|150|200|0"};
foreach(var text in valid)if(!Snapshot.TryParse(text,out _))throw new Exception("Rejected valid snapshot: "+text);
var invalid=new[]{"someone says KFEASTHUD1|prep|60|100|100|fang|100|125|0","KFEASTHUD2|prep|60|100|100|fang|100|125|0","KFEASTHUD1|bad|60|100|100|fang|100|125|0","KFEASTHUD1|active|-1|100|100|fang|100|125|0","KFEASTHUD1|active|361|100|100|fang|100|125|0","KFEASTHUD1|active|10|100|100|fang|126|125|0","KFEASTHUD1|active|10|100|100|fang|100|125|999999999999","KFEASTHUD1|active|10|100|100|fang|100|125"};
foreach(var text in invalid)if(Snapshot.TryParse(text,out _))throw new Exception("Accepted malformed snapshot: "+text);
Console.WriteLine("11 HUD protocol cases passed.");
