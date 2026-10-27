local checks=0
local function eq(a,b) checks=checks+1; assert(a==b,tostring(a).." ~= "..tostring(b)) end
local function setup(job)
 local m=make_engine(); install_jobs(m)
 local function actor(id,j,team,x) return {id=id,job=j,player=true,hp=50000,max_hp=50000,mp=10000,x=x,y=0,z=0,match_team=team,match_active=true} end
 m.sync({actor(1,job,"fang",0),actor(2,24,"claw",3),actor(3,19,"fang",1)},0)
 m.command(1,"on",0,0); m.command(2,"on",0,0); m.command(3,"on",0,0)
 return m,m.actors[1],m.actors[2],m.players[1]
end
for _,case in ipairs({{24,3362},{25,3361},{34,3360},{19,3359}}) do
 local engine=setup(case[1]); local slots=engine.party_actions(1,0)
 eq(#slots,3); eq(slots[1][1],case[2]); eq(slots[1][2],0)
 for i=2,3 do
  local found=false
  for name,action in pairs(engine.actions) do
   if action.id==slots[i][1] then eq(action.additional,true); eq(engine.players[1].loadout[name],true); found=true end
  end
  eq(found,true)
 end
end
local m,a,b,p=setup(25)
m.command(1,"fire",2,0); eq(b.hp,48800); eq(a.mp,9000)
m.command(1,"enochian",1,2.4); eq(p.enochian,true)
m.command(1,"enochian",2,4.8); m.advance(6.8); eq(b.hp,45920); eq(a.mp,7000)
m.command(1,"blizzard",2,7.3); eq(a.mp,9000)
m.command(1,"enochian",2,9.7); m.advance(11.3); eq(a.mp,10000)
m.command(1,"blizzard",2,14); m.advance(17.4); eq(p.polyglot,1)
m.command(1,"xenoglossy",2,18); eq(p.polyglot,0)
m,a,b,p=setup(25)
m.command(1,"thunder",2,0); m.advance(3); eq(b.hp,49200)
m.internal.damage(a,b,8001); eq(m.internal.has(1,"thundercloud"),true)
m.command(1,"thunder",2,3.1); eq(m.internal.has(1,"thundercloud"),nil)
eq(b.hp,38799)
m,a,b,p=setup(25)
m.command(1,"loadout swiftcast manaward",0,0)
a.mp=5000; m.command(1,"manaward",1,0); m.internal.damage(b,a,2500); eq(a.hp,50000); eq(a.mp,7500)
m.command(1,"swiftcast",1,1); m.command(1,"flare",2,1); eq(p.pending,nil); eq(a.mp,3500)
eq(m.command(1,"hallowed_ground",1,2),"This action belongs to another job.")
m,a,b,p=setup(34)
m.command(1,"yukikaze",2,0); eq(p.kenki,15)
m.command(1,"yukikaze",2,2.4); eq(p.kenki,25)
m.command(1,"yukikaze",2,4.8); eq(p.kenki,35); eq(b.hp,45400)
m.command(1,"midare_setsugekka",2,7.2); m.advance(8.5); eq(p.meditation,1); eq(p.kenki,55)
m.command(1,"tsubame_gaeshi",2,8.6); eq(p.meditation,2); eq(p.kenki,75)
m.command(1,"shoha",2,8.7); eq(p.meditation,0)
m.command(1,"chiten",1,9); eq(p.kenki,50)
m.internal.damage(b,a,1000); eq(a.hp,49500); eq(p.kenki,55)
eq(m.command(1,"tenka_goken",2,10),"Action is on cooldown.")
m,a,b,p=setup(34)
m.command(1,"loadout fetter_ward bloodbath",0,0); m.command(1,"fetter_ward",1,0)
m.internal.buff(b,a,"stun",2); eq(m.blocked(1,false),false)
a.hp=40000; m.command(1,"bloodbath",1,0); m.command(1,"yukikaze",2,0); eq(a.hp,41400)
m,a,b,p=setup(19)
m.command(1,"fast_blade",2,0); m.command(1,"fast_blade",2,2.4); a.mp=9000
m.command(1,"fast_blade",2,4.8); eq(p.oath,15); eq(b.hp,46400)
m.command(1,"intervene",2,5); eq(p.sword,3)
m.command(1,"fast_blade",2,7.2); eq(p.sword,2); eq(p.oath,25); eq(a.mp,9500)
a.mp=0; a.hp=40000; m.command(1,"requiescat",2,8); eq(b.hp,41800)
m.command(1,"holy_spirit",2,9.6); eq(p.pending,nil); eq(a.hp,41800); eq(a.mp,0)
m.command(1,"testudo",1,10); m.internal.damage(b,a,3500); eq(a.hp,41300)
m.command(1,"hallowed_ground",1,11); m.internal.damage(b,a,10000); eq(a.hp,41300)
m,a,b,p=setup(19)
m.command(1,"loadout rampart retaliation",0,0); m.command(1,"rampart",1,0)
m.command(1,"intervention",3,0); m.internal.damage(b,m.actors[3],1000); eq(m.actors[3].hp,49300)
m.command(1,"retaliation",1,0); m.internal.damage(b,a,1000); eq(a.hp,49200); eq(b.hp,49200)
-- Every registered action has a native ID and enforces its job restriction.
m,a,b,p=setup(34)
p.kenki=50
eq(m.ground_command(1,"soten",30,0,0,0),"Target is out of range."); eq(p.kenki,50)
m.ground_command(1,"soten",10,0,0,0); eq(a.x,10); eq(p.kenki,25); eq(b.hp,48400)
m,a,b,p=setup(25)
p.adrenaline=100; m.ground_command(1,"cometeor",3,0,0,0); m.advance(2)
eq(b.hp,46000); eq(p.adrenaline,0)
m,a,b,p=setup(34)
p.adrenaline=100; m.internal.buff(b,b,"rampart",10)
m.internal.buff(b,b,"testudo",10); b.statuses.testudo.shield=3000
m.command(1,"raw_destruction",2,0); m.advance(2)
eq(b.hp,43000) -- Damage ignores reduction, but barriers still absorb it.
m,a,b,p=setup(34)
m.command(1,"midare_setsugekka",2,0); m.movement(1,0.2,1,0,0); m.advance(0.2)
eq(p.gcd,0); eq(#p.cooldowns.iaijutsu,0)
m.command(1,"midare_setsugekka",2,0.3); m.advance(1.6); eq(p.meditation,1)
m,a,b,p=setup(25)
m.command(1,"blizzard",2,0); m.command(1,"fire",2,2.4)
local elapsed,total=m.recast(1,"fire",2.4); eq(total,192)
local rows=m.hotbar(1); eq(#rows>20,true)
local gauge=m.job_gauge(1); eq(gauge[1],25); eq(gauge[5],1)
for _,job in ipairs({19,25,34}) do
 local engine=setup(job)
 local n,target,cast=engine.bot_action(1,2,0)
 eq(engine.actions[n].job,job); eq(target,2); eq(cast>=0,true)
end
for _,job in ipairs({19,25,34}) do
 local engine=setup(job)
 for name,d in pairs(engine.actions) do
  if d.job==job then eq(type(d.id),"number"); eq(d.id>0,true) end
 end
end
-- March 2022 DRG: combo transformations, extension cap, charges and eye spends.
m,a,b,p=setup(22)
eq(m.command(1,"geirskogul",2,0):find("two eyes")~=nil,true)
m.command(1,"raiden_thrust",2,0);eq(b.hp,48800);eq(m.resolve(a,"raiden_thrust"),"full_thrust")
m.command(1,"raiden_thrust",2,2.4);eq(b.hp,47400)
m.command(1,"raiden_thrust",2,4.8);eq(b.hp,45800);eq(a.statuses.blood_dragon.expires,25)
m.command(1,"raiden_thrust",2,7.2);eq(b.hp,44200);eq(a.statuses.blood_dragon.expires,35)
m.command(1,"mirage_dive",2,7.3);m.command(1,"mirage_dive",2,7.4);eq(p.eyes,2)
eq(m.command(1,"mirage_dive",2,7.5),"Action is on cooldown.")
m.command(1,"geirskogul",2,7.6);eq(p.eyes,0);eq(m.resolve(a,"geirskogul"),"nastrond")
m.command(1,"high_jump",2,7.7);eq(p.cooldowns.mirage_dive[1]<=7.7,true)
m.command(1,"mirage_dive",2,7.8);m.command(1,"spineshatter_dive",2,7.85);m.command(1,"mirage_dive",2,7.9);eq(p.eyes,2)
m.command(1,"geirskogul",2,8.6);eq(p.eyes,0);eq(m.internal.has(1,"blood_dragon"),true)
m,a,b,p=setup(22)
m.command(1,"life_surge",1,0);m.command(1,"raiden_thrust",2,0);eq(b.hp,48200);eq(a.statuses.life_surge,nil)
m.command(1,"dragon_sight",3,1);eq(m.modify_damage(1,2,1000,1),1100);eq(m.modify_damage(3,2,1000,1),1100)
m.internal.buff(b,a,"bind",10);eq(m.command(1,"high_jump",2,2):find("bound")~=nil,true)
m.command(1,"elusive_jump",1,2);eq(a.statuses.bind,nil);eq(a.z,-15)
local drg_slots=m.party_actions(1,2);eq(#drg_slots,3);eq(drg_slots[1][1],3360)
-- WAR: combo resources, chaos transformation, free-spend window, cooldown reduction.
m,a,b,p=setup(21);a.hp=40000
m.command(1,"heavy_swing",2,0);m.command(1,"heavy_swing",2,2.4);m.command(1,"heavy_swing",2,4.8)
eq(p.beast,40);eq(b.hp,46400);eq(a.hp,41400)
eq(m.command(1,"fell_cleave",2,7.2),"Not enough Beast Gauge.")
m.command(1,"infuriate",1,7.2);eq(m.resolve(a,"fell_cleave"),"inner_chaos")
m.command(1,"fell_cleave",2,7.2);eq(p.beast,40);eq(b.hp,44400);eq(a.hp,43400)
eq(m.modify_damage(1,2,1000,8),1100);eq(m.modify_damage(1,2,1000,17.2),1000)
m.command(1,"inner_release",1,9.6);m.command(1,"fell_cleave",2,9.6);eq(p.beast,40)
eq(math.abs(p.cooldowns.infuriate[1]-27.2)<0.00001,true)
m.internal.buff(b,a,"stun",2);eq(a.statuses.stun,nil)
m,a,b,p=setup(21);m.command(1,"holmgang",2,0);m.internal.damage(b,a,99999);eq(a.hp,1)
m.advance(3);m.internal.damage(b,a,10);eq(a.hp,0)
m,a,b,p=setup(21);a.hp=40000;m.actors[3].hp=40000
m.command(1,"nascent_flash",3,0);m.command(1,"heavy_swing",2,0);eq(a.hp,41000);eq(m.actors[3].hp,41000)
eq(m.modify_damage(2,1,1000,1),900);eq(m.modify_damage(2,3,1000,1),900)
m.command(1,"loadout rampart retaliation",0,7) -- combat prevents changes: use fresh engine
m,a,b,p=setup(21);m.command(1,"loadout rampart retaliation",0,0);m.command(1,"rampart",1,0);m.command(1,"shake_it_off",1,0)
eq(a.statuses.rampart,nil);eq(a.statuses.shake_it_off.shield,4000)
m.internal.damage(b,a,4500);eq(a.hp,49500)
eq(m.party_actions(1,1)[1][1],3359);eq(m.job_gauge(1)[1],21)
for _,job in ipairs({21,22}) do
 local engine=setup(job);eq(#engine.hotbar(1)>20,true)
 local slots=engine.party_actions(1,0);eq(#slots,3)
 eq(engine.command(1,job==21 and "feint" or "rampart",2,0):find("another job")~=nil,true)
end
m,a,b,p=setup(25)
m.command(1,"fire",2,0)
eq(m.job_gauge(1)[2],150)
m.advance(3)
eq(m.job_gauge(1)[2],120)
m.advance(15.1)
eq(m.job_gauge(1)[2],0)
return checks
