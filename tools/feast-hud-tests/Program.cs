using Kawari.FeastHud;
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
byte[] gauge = [0xF5, 1, 0, 0, 0, 75, 0, 0, 0, 0];
frames.Clear(); frames.Accept(summary); frames.Accept(fang); frames.Accept(gauge);
Check(frames.Current == null, "Gauge waits for commit");
frames.Accept(commit); Check(frames.Current!.Players[0].Adrenaline == 75, "Gauge decoded");
frames.Accept(summary); frames.Accept(gauge); Check(!frames.Accept(commit), "Unknown gauge owner rejected");
frames.Accept(summary); frames.Accept(fang); gauge[5] = 101; frames.Accept(gauge);
Check(!frames.Accept(commit), "Invalid gauge rejected");
Console.WriteLine("4 adrenaline framing cases passed.");
byte[] jobAction = [0xF6, 0, 0x9e, 0x22, 0x9f, 0x22, 0xd0, 0x07, 0x60, 0x09]; // Enochian -> Fire IV
frames.Clear(); frames.Accept(summary); frames.Accept(fang); frames.Accept(jobAction);
Check(frames.Current == null, "Hotbar changes wait for commit");
frames.Accept(commit);
Check(frames.Current!.Hotbar.Single() is { BaseId: 8862, ResolvedId: 8863, CastMs: 2000, RecastMs: 2400 }, "Historical action resolution decoded");
frames.Accept(summary); frames.Accept(jobAction); frames.Accept(jobAction);
Check(!frames.Accept(commit), "Duplicate hotbar action invalidates frame");
frames.Accept(summary); frames.Accept(commit);
Check(frames.Current!.Hotbar.Count == 0, "Hotbar state cannot leak into next job");
foreach (uint job in new uint[] {19,25,34})
{
    var bar = FeastJobActions.Hotbar(job);
    Check(bar.Length is > 15 and <= 36, "Full native kit fits three bars");
    Check(bar.All(id => FeastJobActions.Contains(job,id) && FeastJobActions.Find(id)!.Icon != 784), "Historical icons and job whitelist");
}
Check(!FeastJobActions.Contains(25,8755), "Black Mage cannot bypass Paladin action gates");
Console.WriteLine("11 job hotbar/catalog cases passed.");
byte[] jobGauge = [0xF7,34,75,2,0,0,0,0x98,0x3a,0];
frames.Clear(); frames.Accept(summary); frames.Accept(jobGauge);
Check(frames.Current == null,"Job gauge waits for commit");
frames.Accept(commit); Check(frames.Current!.Gauge is { Job:34,Resource:75,Meditation:2 },"Native gauge fields decoded");
frames.Accept(summary); jobGauge[2]=101; frames.Accept(jobGauge); Check(!frames.Accept(commit),"Invalid Kenki rejected");
frames.Accept(summary); frames.Accept(commit); Check(frames.Current!.Gauge==null,"Job gauge cleared between jobs");
Console.WriteLine("4 native job gauge cases passed.");
Check(FeastJobActions.Find(17832) is { Name: "Protect", Recast: 45, Icon: 9208, MpCost: 0 }, "Historical Protect metadata and icon");
Check(FeastJobActions.Find(17789) is { Name: "Glare", Icon: 9439 }, "Historical Glare icon");
Check(FeastJobActions.Hotbar(24).Length == 20 && FeastJobActions.Hotbar(24).All(id => FeastJobActions.Contains(24, id)), "White Mage uses its 20 historical actions");
Check(FeastJobActions.Find(8896) is { Name: "Cure II", MpCost: 1000, Cast: 1.5f }, "Historical Cure II tooltip cost and cast time");
Check(FeastJobActions.Find(8866) is { MpCost: 4000 }, "Historical Flare tooltip cost");
Console.WriteLine("5 historical metadata cases passed.");
byte[] lilies = [0xF7,24,2,3,0,0,0,0xc4,9,0]; // 2 lilies, full blood, 2500ms
frames.Clear(); frames.Accept(summary); frames.Accept(fang); frames.Accept(lilies); frames.Accept(commit);
Check(frames.Current!.Gauge is { Job:24, Resource:2, Meditation:3, Timer:2500 }, "WHM gauge decoded atomically");
frames.Accept(summary); lilies[2]=4; frames.Accept(lilies);
Check(!frames.Accept(commit), "Reject fourth lily");
lilies[2]=2; lilies[7]=0x10; lilies[8]=0x27;
frames.Accept(summary); frames.Accept(lilies);
Check(!frames.Accept(commit), "Reject unbounded lily timer");
var resourceFrame = new DirectorFrame(new("active",60,100,0,"fang",100,0),
    [new Participant(1,1,100,24,true) { Adrenaline=100 }],0) { Gauge=new(24,1,3,0,0,false,0) };
Check(FeastResources.CanUse(17791,resourceFrame,1) && FeastResources.CanUse(18946,resourceFrame,1), "Lily enables Solace and Rapture");
Check(FeastResources.CanUse(17793,resourceFrame,1), "Full blood enables Misery");
Check(FeastResources.CanUse(3362,resourceFrame,1), "Full adrenaline enables Rain");
var emptyFrame = resourceFrame with { Gauge=new(24,0,0,0,0,false,0), Players=[new Participant(1,1,100,24,true)] };
Check(!FeastResources.CanUse(17791,emptyFrame,1) && !FeastResources.CanUse(18946,emptyFrame,1)
    && !FeastResources.CanUse(17793,emptyFrame,1) && !FeastResources.CanUse(3362,emptyFrame,1), "Empty resources disable predictions");
Check(!FeastResources.CanUse(3362,resourceFrame with { Summary=resourceFrame.Summary with { Phase="prep" } },1), "Preparation blocks actions");
Check(!FeastResources.CanUse(3362,null,1) && !FeastResources.CanUse(3362,resourceFrame,99), "Missing state or participant fails closed");
Console.WriteLine("9 WHM/adrenaline protocol and eligibility cases passed.");
var headerFrame = new DirectorFrame(new("active",298,300,500,"claw",100,0),
    [new(10,1,50,34,true),new(20,2,200,19,true),new(21,2,100,24,true)],0);
var header = MatchHeaderState.From(headerFrame,0);
Check(header.Left.Name=="Claws ★" && header.Left.Score==500 && header.Right.Name=="Fangs" && header.Right.Score==300, "Header preserves historical team colors and marks local team");
Check(header.Left.Players.Select(p=>p.Id).SequenceEqual(new uint[]{20,21}) && header.Right.Players.Single().Medals==50, "Header medal slots keep team and roster ordering");
Check(header.Clock=="4:58" && MatchHeaderState.From(headerFrame,1200).Clock=="4:57", "Header clock interpolates live frame");
Check(MatchHeaderState.From(headerFrame,2501).Left.Players.Count==0 && MatchHeaderState.From(headerFrame,2501).Clock=="--:--", "Stale header hides old scores and roster");
Check(MatchHeaderState.From(headerFrame with { Summary=headerFrame.Summary with { Team="fang" } },0).Right.Name=="Fangs ★", "Fang player marker stays on red side");
Check(MatchHeaderState.From(headerFrame with { Summary=headerFrame.Summary with { Phase="claw" } },2000).Clock=="4:58", "Finished clock stops interpolating");
Check(MatchHeaderState.From(headerFrame with { Summary=headerFrame.Summary with { Phase="sudden", Seconds=1 } },2000) is { Clock:"0:00", Phase:"SUDDEN DEATH" }, "Overtime clock clamps at zero");
Check(MatchHeaderState.GaugeWidth(0)==0 && MatchHeaderState.GaugeWidth(300)==170 && MatchHeaderState.GaugeWidth(900)==340, "Medal gauge reflects 600 medal victory threshold");
Console.WriteLine("8 native match header cases passed.");
foreach (ushort id in new ushort[]{3359,3360,3361,3362})
    Check(new PartyAction(0,id,0) is { IsAdrenaline:true, Icon:>0 }, "Adrenaline party slot has a historical icon");
Console.WriteLine("4 party adrenaline icon cases passed.");
Check(FeastJobActions.Supports(21) && FeastJobActions.Supports(22), "WAR and DRG enable native Feast hotbars");
Check(FeastJobActions.Hotbar(21).Contains(8758u) && !FeastJobActions.Hotbar(21).Contains(8761u), "WAR base combo button excludes derived Maim");
Check(FeastJobActions.Hotbar(22).Contains(18916u) && !FeastJobActions.Hotbar(22).Contains(8793u), "DRG base combo button excludes derived Full Thrust");
Check(FeastJobActions.Contains(21,3359) && FeastJobActions.Contains(22,3360), "WAR and DRG retain role adrenaline actions");
byte[] dragonGauge=[0xF7,22,2,0,0,2,0,0x30,0x75,0];
frames.Clear();frames.Accept(summary);frames.Accept(dragonGauge);frames.Accept(commit);
Check(frames.Current?.Gauge is { Job:22,Resource:2,Element:2,Timer:30000 }, "Dragon gauge accepts capped 30-second timer");
dragonGauge[2]=3;frames.Accept(summary);frames.Accept(dragonGauge);
Check(!frames.Accept(commit), "Dragon gauge rejects third eye");
Console.WriteLine("6 WAR/DRG HUD cases passed.");
int expandedChecks=0;
foreach (uint job in new uint[]{20,23,27,28,30,31,32,33,35,37,38,39,40})
{
    Check(FeastJobActions.Supports(job), $"Historical job {job} enabled"); expandedChecks++;
    var actions=FeastJobActions.Hotbar(job);
    Check(actions.Length is > 8 and <= 36 && actions.Distinct().Count()==actions.Length, $"Job {job} has unique bounded hotbar"); expandedChecks++;
    Check(actions.All(id=>FeastJobActions.Find(id) is { Icon:>0 }), $"Job {job} has original icons"); expandedChecks++;
}
Check(!FeastJobActions.Supports(36) && !FeastJobActions.Supports(41) && !FeastJobActions.Supports(42), "BLU and post-Feast jobs excluded"); expandedChecks++;
Check(new PartyAction(0,4249,0).IsAdrenaline && !FeastResources.CanUse(4249,emptyFrame,1), "Terminal Velocity requires adrenaline and uses the LB slot"); expandedChecks++;
Check(FeastJobActions.Find(17701) is { MpCost:2500 } && FeastJobActions.Find(8905) is { MpCost:1000 }, "Recovered fixed PvP MP costs"); expandedChecks++;
// Reaper's complete atomic action frame exceeds the previous 32-record limit.
frames.Clear(); frames.Accept(summary);
foreach (var id in new ushort[]{27795,27796,27797,27802,27803,27806,27808,27809,27810,27813,27814,27817,27818,27820,27821,27798,27799,27800,27801,27804,27805,27807,27811,27812,27815,27816,27819,18992,18954,17676,17681,17677,17674,3360,18942,18943})
{
    byte[] row=[0xF6,0,(byte)id,(byte)(id>>8),(byte)id,(byte)(id>>8),0,0,0,0]; frames.Accept(row);
}
Check(frames.Accept(commit) && frames.Current!.Hotbar.Count==36, "Full Reaper frame is accepted atomically"); expandedChecks++;
foreach (byte[] row in new byte[][]{
    [0xF7,35,100,100,0,0,0,0,0,0], [0xF7,39,100,100,5,4,0,0x98,0x3a,0],
    [0xF7,38,100,4,4,2,0,0x21,0x43,0], [0xF7,23,100,4,0,2,0,0x30,0x75,0]})
{
    frames.Clear(); frames.Accept(summary); frames.Accept(row); Check(frames.Accept(commit), $"Job {row[1]} gauge decodes"); expandedChecks++;
}
Console.WriteLine($"{expandedChecks} all-job HUD cases passed.");


// Exercise every renderer with nonzero data, then zero/expiry, while guarding
// the native vtable and bytes beyond the payload (including BRD's string ptr).
int gaugeChecks = 0;
foreach (var layout in NativeJobGaugeData.Layouts)
{
    var name = System.Text.Encoding.ASCII.GetBytes(layout.Name + "\0");
    Check(NativeJobGaugeData.Identify(name, layout.Job) == layout, "Exact native gauge name/owner");
    Check(NativeJobGaugeData.Identify(name, 0) == null, "Outside Feast does not match");
    var buffer = Enumerable.Repeat((byte)0xA5,layout.Size+16).ToArray();
    var state = new JobGauge(layout.Job,2,1,2,1,true,5000);
    Check(NativeJobGaugeData.Write(layout,state,buffer), "Native projection exists for " + layout.Name);
    Check(buffer.Take(8).All(x=>x==0xA5) && buffer.Skip(layout.Size).All(x=>x==0xA5), "Native pointers and adjacent fields preserved");
    var before = buffer.ToArray();
    Check(!NativeJobGaugeData.Write(layout,state with { Job=0 },buffer) && buffer.SequenceEqual(before), "Wrong-job frame cannot overwrite gauge");
    Check(!NativeJobGaugeData.Write(layout,state,buffer.AsSpan(0,layout.Size-1)), "Short native payload rejected");
    NativeJobGaugeData.Write(layout,state with {Resource=0,Meditation=0,Polyglot=0,Element=0,Enochian=false,Timer=0},buffer);
    if (!layout.Visible) Check(buffer.Skip(8).Take(layout.Size-8).All(x=>x==0), "Non-Feast gauge stays disabled");
    gaugeChecks += 6 + (layout.Visible?0:1);
}
Check(NativeJobGaugeData.Layouts.Select(x=>x.Job).Distinct().Count()==19,"All 19 jobs have native gauge mappings");
int GaugeInt(string name, JobGauge state, int offset)
{
    var layout=NativeJobGaugeData.Layouts.Single(x=>x.Name==name);
    var buffer=new byte[layout.Size]; NativeJobGaugeData.Write(layout,state,buffer);
    return System.Buffers.Binary.BinaryPrimitives.ReadInt32LittleEndian(buffer.AsSpan(offset,4));
}
Check(GaugeInt("JobHudWHM0",new(24,3,2,0,0,false,9999),12)==3,"WHM lilies");
Check(GaugeInt("JobHudSAM0",new(34,100,3,0,0,false,0),12)==3,"SAM meditation distinct from Kenki");
Check(GaugeInt("JobHudRDM0",new(35,80,60,0,0,false,0),16)==60,"RDM white mana distinct from black");
Check(GaugeInt("JobHudGFF1",new(40,2,0,0,0,false,7000),24)==10000,"SGE ten-second generation");
Check(GaugeInt("JobHudDNC0",new(38,0,0,2,2,false,0x4321),32)==4,"DNC fourth packed step");
Check(GaugeInt("JobHudRRP1",new(39,50,50,5,2,false,12000),12)==5,"RPR Lemure distinct from Void");
Check(GaugeInt("JobHudAST0",new(33,8,0,0,0,false,0),24)==8,"AST Lady crown");
Check(GaugeInt("JobHudBLM0",new(25,120,0,2,1,true,9000),24)==12000,"BLM element timer separate from Enochian");
byte[] blmTimer=[0xF7,25,150,0,2,1,1,0x98,0x3a,0];
frames.Clear();frames.Accept(summary);frames.Accept(blmTimer);Check(frames.Accept(commit),"BLM fifteen-second element timer accepted");
blmTimer[2]=151;frames.Accept(summary);frames.Accept(blmTimer);Check(!frames.Accept(commit),"BLM element timer bounded");
Console.WriteLine($"{gaugeChecks+11} native gauge projection cases passed.");
int menuChecks=0;
foreach (uint job in new uint[] {19,20,21,22,23,24,25,27,28,30,31,32,33,34,35,37,38,39,40})
{
    var menu=FeastActionMenu.For(job,FeastActionMenu.MenuIds(job));
    Check(menu.JobActions.Count>=8,$"Job {job} lists its job actions");
    Check(menu.Role?.Action.Key==FeastActionMenu.RoleActions(job)[0],$"Job {job} fixed role action");
    Check(menu.Additional.Count==5,$"Job {job} offers five additional actions");
    Check(menu.Common.Select(a=>a.Action.Key).Order().SequenceEqual(new[]{"bolt","kit"}),$"Job {job} common actions");
    Check(menu.Adrenaline!=null&&FeastActionMenu.IsAdrenaline(menu.Adrenaline.Action.Id),$"Job {job} adrenaline rush");
    var all=menu.JobActions.Concat(menu.Additional).Concat(menu.Common).Append(menu.Role!).Append(menu.Adrenaline!).ToArray();
    Check(all.Length==FeastActionMenu.MenuIds(job).Length&&all.Select(a=>a.HotbarId).Distinct().Count()==all.Length,$"Job {job} menu covers the hotbar kit once");
    Check(FeastActionMenu.ValidLoadout(job,FeastActionMenu.DefaultLoadout(job)),$"Job {job} default loadout valid");
    menuChecks+=7;
}
var whmMenu=FeastActionMenu.For(24,FeastActionMenu.MenuIds(24));
Check(whmMenu.JobActions.Any(a=>a.HotbarId==17789&&a.Action.Name=="Glare"),"WHM drags its historical Glare");
Check(whmMenu.Additional.Select(a=>a.Action.Key).SequenceEqual(new[]{"repose","protect","rescue","largesse","attunement"}),"WHM additional order");
Check(FeastActionMenu.Toggle(["protect","largesse"],"rescue").SequenceEqual(new[]{"largesse","rescue"}),"Third choice replaces the oldest");
Check(FeastActionMenu.Toggle(["protect","largesse"],"protect").SequenceEqual(new[]{"largesse"}),"Unset leaves one choice");
Check(!FeastActionMenu.ValidLoadout(24,["protect","protect"])&&!FeastActionMenu.ValidLoadout(24,["purify","protect"])&&!FeastActionMenu.ValidLoadout(25,["protect","largesse"]),"Invalid loadouts rejected");
Check(FeastActionMenu.LoadoutCommand(["arms_length","feint"])=="!feast loadout arms_length feint","Loadout command matches the server parser");
var placed=FeastActionMenu.Place([new(0,0,8858)],0,0,8859);
Check(placed.Single() is {Bar:0,Slot:0,Id:8859},"Dropping replaces a layout slot");
Check(FeastActionMenu.Place(placed,10,0,8858)==placed&&FeastActionMenu.Clear(placed,0,0).Count==0,"Cross bars ignored and clear works");
var rects=new[]{(0u,0u,100f,500f,40f,40f),(1u,3u,300f,450f,40f,40f)};
Check(FeastActionMenu.HitTest(rects,120,520)==(0u,0u)&&FeastActionMenu.HitTest(rects,339,489)==(1u,3u)&&FeastActionMenu.HitTest(rects,140,520)==null,"Native slot hit test");
Check(FeastActionMenu.CanEditActions(250)&&!FeastActionMenu.CanEditActions(745)&&!FeastActionMenu.CanEditActions(132),"Actions are edited in the Wolves' Den only");
Check(FeastActionMenu.UsesFeastHotbars(250)&&FeastActionMenu.UsesFeastHotbars(745)&&!FeastActionMenu.UsesFeastHotbars(132),"Feast hotbars in the Den and arena");
Check(FeastActionMenu.CanCast(745)&&!FeastActionMenu.CanCast(250),"Feast actions are cast in the arena only");
Console.WriteLine($"{menuChecks+12} PvP actions menu cases passed.");
var client751=NativeClient.ForHash("4236E770E673150E85F8D10BEAB2FC4834C82F86AAB8A555A9175439FC906A6D");
var client757=NativeClient.ForHash("46A52AB1DBE80B9A8D6C4D4531BF66693904200C5135FACACD053EF28A5DE14A");
Check(client751 is {PvPHeaderRefresh:0xD5C0A0,LegacyDirectorVtable:0x234E100,ActionDetailUpdate:0xEE2530,JobHudRequestedUpdate:0x15A0210},"7.51h2 addresses unchanged");
// Same constructor-relative offsets in both builds; vtable moved with its neighbours.
Check(client757 is {PvPHeaderRefresh:0xD62790,PvPHeaderOpen:0xD62D60,LegacyDirectorVtable:0x23780E0,ActionDetailUpdate:0xEE93D0,JobHudRequestedUpdate:0x15AB770},"7.57h1 addresses");
Check(client757!.PvPHeaderOpen-client757.PvPHeaderRefresh==client751!.PvPHeaderOpen-client751.PvPHeaderRefresh,"PvP header functions keep their spacing");
Check(NativeClient.ForHash("00")==null,"Unknown executables get no native hooks");
Console.WriteLine("4 native client version cases passed.");
// Server recast for a charged action: total = recast * charges, elapsed = total - (last deadline - now).
(bool active, float elapsed, float total) Recast(float recast, int charges, float[] deadlines, float now)
{
    float total = recast * charges, remaining = Math.Max(0, (deadlines.Length > 0 ? deadlines[^1] : now) - now);
    return (remaining > 0, total - remaining, total);
}
bool Ready(int charges, (bool active, float elapsed, float total) r) => FeastJobActions.ChargeReady(charges, r.active, r.elapsed, r.total);
Check(Ready(2, Recast(15, 2, [], 0)), "Unused charges ready");
Check(Ready(2, Recast(15, 2, [15], 0)), "Second of two charges ready after one use");
Check(!Ready(2, Recast(15, 2, [15, 30], 0)), "No charge after both are spent");
Check(Ready(2, Recast(15, 2, [15, 30], 15)), "One charge back after one recast");
Check(Ready(3, Recast(15, 3, [15, 30], 0)) && !Ready(3, Recast(15, 3, [15, 30, 45], 1)), "Three-charge Medical Kit");
Check(!Ready(1, Recast(15, 1, [15], 0)) && Ready(1, Recast(15, 1, [15], 15)), "Single charge unchanged");
Check(FeastJobActions.Find(13975) is { Charges: 2 } && FeastJobActions.Find(18943) is { Charges: 3 }, "Tetragrammaton and Medical Kit charges");
Console.WriteLine("7 charged recast cases passed.");
