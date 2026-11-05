-- Black Mage, Samurai, Paladin, Warrior and Dragoon (the first jobs after
-- White Mage), plus the hooks every job shares: roles and additional
-- actions, damage modifiers, shields, combos (M.resolve), job gauges for the
-- HUD (M.job_gauge), hotbars (M.hotbar).
-- AdditionalJobs.lua wraps these hooks for the remaining jobs.
-- Sources: the official job guides archived before patch 6.1.
return function(M)
local I=M.internal
local add,state,has,buff,damage,heal,emit=I.add,I.state,I.has,I.buff,I.damage,I.heal,I.emit
local enemies,allies,valid,distance=I.enemies,I.allies,I.valid,I.distance
M.jobs={[19]=true,[21]=true,[22]=true,[24]=true,[25]=true,[34]=true}
local A=M.actions
for name,d in pairs(A) do if name~="bolt" and name~="kit" then d.job=24 end end
-- Each role's six PvP role actions. The first is always equipped; players
-- choose two of the other five in the Wolves' Den (their "loadout").
local roles={
 [24]={"purify","repose","protect","rescue","largesse","attunement"},
 [25]={"phantom_dart","drain","addle","aetheric_burst","swiftcast","manaward"},
 [34]={"smite","feint","bloodbath","arms_length","fetter_ward","backstep"},
 [19]={"low_blow","full_swing","weapon_throw","reprisal","rampart","retaliation"}}
function M.party_list(job) return roles[job] or {} end
roles[21]=roles[19]; roles[22]=roles[34]
function M.job_status(a,p)
 if a.job==21 then return string.format("Feast WAR: Beast %d/100",p.beast) end
 if a.job==22 then return string.format("Feast DRG: Eyes %d/2, Blood %s, Life %s",p.eyes,tostring(has(a.id,"blood_dragon") or false),tostring(has(a.id,"life_dragon") or false)) end
 if a.job==19 then return string.format("Feast PLD: Oath %d/100, Sword Oath %d, MP %d",p.oath,has(a.id,"sword_oath") and p.sword or 0,a.mp) end
 if a.job==34 then return string.format("Feast SAM: Kenki %d/100, Meditation %d/3",p.kenki,p.meditation) end
 if a.job==25 then return string.format("Feast BLM: %s, Enochian %s, Polyglot %d/2, MP %d",has(a.id,"astral_fire") and "Astral Fire" or has(a.id,"umbral_ice") and "Umbral Ice" or "neutral",tostring(p.enochian or false),p.polyglot,a.mp) end
end
function M.allowed(a,d)
 if not d.additional then return true end
 for _,name in ipairs(roles[a.job] or {}) do if name==d.name then return true end end
 return false
end
function M.reset_job(p)
 p.lilies=0; p.blood=0; p.lily_time=0
 p.beast=0; p.eyes=0; p.surge_active=false
 p.kenki=0; p.meditation=0; p.oath=0; p.sword=0; p.polyglot=0
 p.enochian=nil; p.poly_time=0; p.combo=nil; p.repeat_action=nil
end
function M.initialize(a,p)
 M.reset_job(p)
 if a.job~=24 then local r=roles[a.job]; p.loadout={[r[2]]=true,[r[3]]=true} end
end
local function player(a) return M.players[a.id] end
local function gauge(a,key,amount,cap) local p=player(a); p[key]=math.min(cap or 100,math.max(0,(p[key] or 0)+amount)) end
local function mp(a,amount) a.mp=math.min(10000,math.max(0,a.mp+amount)); emit("mp",a.id,a.id,a.mp) end
local function move(a,x,y,z)
 a.x,a.y,a.z=x,y,z
 table.insert(M.events,{kind="move",actor=a.id,target=a.id,x=x,y=y,z=z})
end
local function bound(a) if M.blocked(a.id,true) then return "Cannot move while bound or incapacitated." end end
local function away(a,b,amount)
 if has(b.id,"fetter_ward") or has(b.id,"inner_release") then return end
 local dx,dz=b.x-a.x,b.z-a.z; local length=math.sqrt(dx*dx+dz*dz)
 if length>0 then move(b,b.x+dx/length*amount,b.y,b.z+dz/length*amount) end
end
local function area(a,b,r,potency,extra)
 for _,t in ipairs(enemies(a,b,r)) do local hit=damage(a,t,potency); if extra then extra(t,hit) end end
end
local function cone(a,b,r,potency,extra)
 local dx,dz=b.x-a.x,b.z-a.z; local length=math.sqrt(dx*dx+dz*dz)
 if length==0 then return end
 for _,t in ipairs(enemies(a,a,r)) do
  local tx,tz=t.x-a.x,t.z-a.z; local tl=math.sqrt(tx*tx+tz*tz)
  -- 90-degree forward cone; guide specifies shape/range, not exact angle.
  if tl==0 or (dx*tx+dz*tz)/(length*tl)>=math.sqrt(0.5) then
   local hit=damage(a,t,potency); if extra then extra(t,hit) end
  end
 end
end
local function action(job,name,id,target,cast,recast,cost,range,run,extra)
 extra=extra or {}; extra.job=job
 add(name,id,target,cast,recast,cost,range,run,extra)
end
local function spell(name,id,cast,cost,run,extra)
 extra=extra or {}; extra.gcd=true; extra.spell=true
 action(25,name,id,"enemy",cast,2.4,cost,25,run,extra)
end
local function element(a,name)
 a.statuses.astral_fire=nil; a.statuses.umbral_ice=nil; buff(a,a,name,15)
end
local function thunder(a,b,potency,threshold)
 local key="thunder_"..a.id
 buff(a,b,key,15,potency)
 b.statuses[key].dot=true; b.statuses[key].threshold=threshold; b.statuses[key].received=0
end
spell("fire",8858,0,1000,function(a,b) damage(a,b,1200); element(a,"astral_fire") end)
spell("blizzard",8859,0,0,function(a,b) damage(a,b,800); mp(a,2000); element(a,"umbral_ice") end)
spell("thunder",8860,0,0,function(a,b) damage(a,b,400); thunder(a,b,400,8000) end)
spell("thunder_ii",18935,0,0,function(a,b) area(a,b,5,200,function(t) thunder(a,t,200,4000) end) end)
spell("flare",8866,2,4000,function(a,b) area(a,b,5,1800); element(a,"astral_fire") end)
spell("freeze",8867,2,0,function(a,b) area(a,b,5,1200,function() mp(a,2000) end); element(a,"umbral_ice") end)
action(25,"enochian",8862,"self",0,2.4,0,0,function(a) player(a).enochian=true; player(a).poly_time=0 end,
 {gcd=true,check=function(a) if not has(a.id,"astral_fire") and not has(a.id,"umbral_ice") then return "Astral Fire or Umbral Ice required." end end})
local function poly(a) if player(a).polyglot<1 then return "Polyglot required." end end
spell("foul",8865,0,0,function(a,b) gauge(a,"polyglot",-1,2); area(a,b,5,1800) end,{check=poly})
spell("xenoglossy",17774,0,0,function(a,b) gauge(a,"polyglot",-1,2); damage(a,b,2400) end,{check=poly})
action(25,"night_wing",17775,"enemy",0,45,0,25,function(a,b)
 for _,t in ipairs(enemies(a,b,5)) do buff(a,t,"sleep",3); emit("interrupt",a.id,t.id) end
end)
action(25,"manipulation",8869,"other_ally",0,15,0,25,function(a,b) move(a,b.x,b.y,b.z) end,{check=bound})
local function cloud(a) if not has(a.id,"thundercloud") then return "Thundercloud required." end end
spell("thunder_iii",8861,0,0,function(a,b) a.statuses.thundercloud=nil; damage(a,b,2400); thunder(a,b,400,8000) end,{check=cloud})
spell("thunder_iv",18936,0,0,function(a,b) a.statuses.thundercloud=nil; area(a,b,5,1200,function(t) thunder(a,t,200,4000) end) end,{check=cloud})
spell("fire_iv",8863,2,2000,function(a,b) damage(a,b,2400) end,
 {check=function(a) if not player(a).enochian or not has(a.id,"astral_fire") then return "Enochian and Astral Fire required." end end})
spell("blizzard_iv",8864,2,0,function(a,b) damage(a,b,1600); mp(a,4000) end,
 {check=function(a) if not player(a).enochian or not has(a.id,"umbral_ice") then return "Enochian and Umbral Ice required." end end})
action(25,"phantom_dart",17684,"enemy",0,30,0,25,function(a,b) damage(a,b,1000); buff(a,b,"phantom_dart",6) end)
action(25,"drain",17683,"enemy",0,30,0,25,function(a,b) heal(a,a,damage(a,b,600+1800*(1-a.hp/a.max_hp))*2,true) end,{additional=true})
action(25,"addle",17686,"enemy",0,30,0,25,function(a,b) buff(a,b,"addle",6) end,{additional=true})
action(25,"aetheric_burst",18956,"self",0,45,0,0,function(a) area(a,a,5,1600,function(t) buff(a,t,"aetheric_burst",10) end) end,{additional=true})
action(25,"swiftcast",17685,"self",0,15,0,0,function(a) buff(a,a,"swiftcast",5) end,{additional=true})
action(25,"manaward",17687,"self",0,30,0,0,function(a) buff(a,a,"manaward",10); a.statuses.manaward.shield=2500 end,{additional=true})

local function weaponskill(job,name,id,target,range,run,extra)
 extra=extra or {}; extra.gcd=true; extra.weapon=true
 action(job,name,id,target,0,2.4,0,range,run,extra)
end
local function combo(a,previous) local p=player(a); return p.combo==previous and (p.combo_until or 0)>M.now end
local function setcombo(a,name) player(a).combo=name; player(a).combo_until=M.now+15 end
weaponskill(34,"yukikaze",8824,"enemy",5,function(a,b) damage(a,b,1400); gauge(a,"kenki",15); setcombo(a,"yukikaze") end,{combo=true})
weaponskill(34,"gekko",8825,"enemy",5,function(a,b) damage(a,b,1600); gauge(a,"kenki",10); setcombo(a,"gekko") end,
 {combo=true,check=function(a) if not combo(a,"yukikaze") then return "Yukikaze combo required." end end})
weaponskill(34,"kasha",8826,"enemy",5,function(a,b) damage(a,b,1600); gauge(a,"kenki",10); setcombo(a,nil) end,
 {combo=true,check=function(a) if not combo(a,"gekko") then return "Gekko combo required." end end})
weaponskill(34,"mangetsu",18926,"self",0,function(a) area(a,a,5,1000); gauge(a,"kenki",10); setcombo(a,"mangetsu") end,{combo=true})
weaponskill(34,"oka",18927,"self",0,function(a) area(a,a,5,1000); gauge(a,"kenki",10); setcombo(a,nil) end,
 {combo=true,check=function(a) if not combo(a,"mangetsu") then return "Mangetsu combo required." end end})
weaponskill(34,"enpi",8827,"enemy",15,function(a,b) damage(a,b,1200); gauge(a,"kenki",10) end)
local function iaijutsu(a,b,wide)
 if wide then cone(a,b,8,1600,function() gauge(a,"kenki",10) end) else damage(a,b,2600); gauge(a,"kenki",20) end
 gauge(a,"meditation",1,3)
end
for _,d in ipairs({{"midare_setsugekka",8831,false},{"tenka_goken",8830,true}}) do
 action(34,d[1],d[2],"enemy",1.3,2.4,0,8,function(a,b) iaijutsu(a,b,d[3]) end,
  {weapon=true,gcd=true,cooldown="iaijutsu",cooldown_recast=15})
end
local function repeatcheck(expected)
 return function(a) local p=player(a); if not p.repeat_action or (expected and p.repeat_action~=expected) then return "Execute the corresponding Iaijutsu immediately beforehand." end end
end
action(34,"tsubame_gaeshi",17740,"enemy",0,30,0,8,function() end,{weapon=true,charges=2,cooldown="tsubame",check=repeatcheck()})
for _,d in ipairs({{"kaeshi_setsugekka",17743,"midare_setsugekka",false},{"kaeshi_goken",17742,"tenka_goken",true}}) do
 action(34,d[1],d[2],"enemy",0,30,0,8,function(a,b) iaijutsu(a,b,d[4]) end,
  {weapon=true,charges=2,cooldown="tsubame",check=repeatcheck(d[3])})
end
local function kenki(a) if player(a).kenki<25 then return "25 Kenki required." end end
action(34,"shinten",8832,"enemy",0,1,0,5,function(a,b) gauge(a,"kenki",-25); damage(a,b,1200) end,{check=kenki})
action(34,"kyuten",18988,"self",0,1,0,0,function(a) gauge(a,"kenki",-25); area(a,a,5,800) end,{check=kenki})
action(34,"soten",7857,"ground",0,15,0,20,function(a,b)
 gauge(a,"kenki",-25)
 local dx,dz=b.x-a.x,b.z-a.z; local length=dx*dx+dz*dz
 for _,t in ipairs(enemies(a,a,21)) do
  local f=length>0 and ((t.x-a.x)*dx+(t.z-a.z)*dz)/length or 0
  if f>=0 and f<=1 and (t.x-a.x-f*dx)^2+(t.z-a.z-f*dz)^2<=1 then damage(a,t,1600) end
 end
 move(a,b.x,b.y,b.z)
end,{check=function(a) return bound(a) or kenki(a) end})
action(34,"chiten",7855,"self",0,30,0,0,function(a) gauge(a,"kenki",-25); buff(a,a,"chiten",6); a.statuses.chiten.counters=5 end,{check=kenki})
action(34,"shoha",18929,"enemy",0,1,0,5,function(a,b) damage(a,b,800*player(a).meditation); player(a).meditation=0 end,
 {check=function(a) if player(a).meditation<1 then return "Meditation required." end end})
action(34,"smite",18992,"enemy",0,30,0,5,function(a,b) damage(a,b,600+2400*(1-b.hp/b.max_hp)) end)
action(34,"feint",18954,"enemy",0,45,0,15,function(a,b) buff(a,b,"feint_"..a.id,6) end,{additional=true})
action(34,"bloodbath",17676,"self",0,30,0,0,function(a) buff(a,a,"bloodbath",6) end,{additional=true})
action(34,"arms_length",17681,"self",0,45,0,0,function(a) buff(a,a,"arms_length",6) end,{additional=true})
action(34,"fetter_ward",17677,"self",0,60,0,0,function(a) buff(a,a,"fetter_ward",10) end,{additional=true})
action(34,"backstep",17674,"self",0,15,0,0,function(a) move(a,a.x-math.sin(a.rotation or 0)*10,a.y,a.z-math.cos(a.rotation or 0)*10) end,{additional=true,check=bound})

weaponskill(19,"fast_blade",8746,"enemy",5,function(a,b) damage(a,b,1000); gauge(a,"oath",5); setcombo(a,"fast_blade") end,{combo=true})
weaponskill(19,"riot_blade",8749,"enemy",5,function(a,b) damage(a,b,1200); mp(a,500); gauge(a,"oath",5); setcombo(a,"riot_blade") end,
 {combo=true,check=function(a) if not combo(a,"fast_blade") then return "Fast Blade combo required." end end})
weaponskill(19,"royal_authority",8750,"enemy",5,function(a,b) damage(a,b,1400); gauge(a,"oath",5); setcombo(a,nil) end,
 {combo=true,check=function(a) if not combo(a,"riot_blade") then return "Riot Blade combo required." end end})
weaponskill(19,"total_eclipse",18900,"self",0,function(a) area(a,a,5,600); gauge(a,"oath",5); setcombo(a,"total_eclipse") end,{combo=true})
weaponskill(19,"prominence",18901,"self",0,function(a) area(a,a,5,800,function() mp(a,250) end); gauge(a,"oath",5); setcombo(a,nil) end,
 {combo=true,check=function(a) if not combo(a,"total_eclipse") then return "Total Eclipse combo required." end end})
weaponskill(19,"shield_lob",18902,"enemy",15,function(a,b) damage(a,b,800); gauge(a,"oath",5) end)
action(19,"holy_spirit",8752,"enemy",1.5,2.4,5000,25,function(a,b) heal(a,a,damage(a,b,1800),true); gauge(a,"oath",10) end,{gcd=true,spell=true})
action(19,"holy_circle",17693,"self",1.5,2.4,5000,0,function(a) area(a,a,5,1000,function(_,hit) heal(a,a,hit,true) end); gauge(a,"oath",10) end,{gcd=true,spell=true})
action(19,"confiteor",17692,"enemy",0,2.4,0,25,function(a,b) gauge(a,"oath",-50); area(a,b,5,2000,function(t) buff(a,t,"confiteor",10) end) end,
 {gcd=true,spell=true,check=function(a) if player(a).oath<50 then return "50 Oath required." end end})
action(19,"intervene",17694,"enemy",0,30,0,15,function(a,b) move(a,b.x,b.y,b.z); damage(a,b,600); player(a).sword=3; buff(a,a,"sword_oath",10) end,{check=bound})
action(19,"requiescat",8754,"enemy",0,30,0,5,function(a,b) damage(a,b,1000+1000*(1-a.mp/10000)); buff(a,a,"requiescat",6) end)
action(19,"hallowed_ground",8755,"self",0,60,0,0,function(a) buff(a,a,"hallowed_ground",5) end)
action(19,"intervention",19085,"other_ally",0,15,0,30,function(a,b) buff(a,b,"intervention",6); b.statuses.intervention.reduction=has(a.id,"rampart") and 0.3 or 0.2 end)
action(19,"testudo",19086,"self",0,45,0,0,function(a) for _,b in ipairs(allies(a,a,15)) do buff(a,b,"testudo",10); b.statuses.testudo.shield=3000 end end)
local function sword(a) if not has(a.id,"sword_oath") or player(a).sword<1 then return "Sword Oath required." end end
weaponskill(19,"atonement",17691,"enemy",5,function(a,b) damage(a,b,2000); mp(a,500); gauge(a,"oath",10); player(a).sword=player(a).sword-1 end,{check=sword})
weaponskill(19,"glory_slash",18899,"enemy",6,function(a,b) cone(a,b,6,1200,function() mp(a,250) end); gauge(a,"oath",10); player(a).sword=player(a).sword-1 end,{check=sword})
action(19,"low_blow",17669,"enemy",0,30,0,5,function(a,b) damage(a,b,600); buff(a,b,"stun",2); emit("interrupt",a.id,b.id) end)
action(19,"full_swing",17671,"enemy",0,30,0,5,function(a,b) damage(a,b,1200); away(a,b,15) end,{additional=true})
action(19,"weapon_throw",18952,"enemy",0,30,0,15,function(a,b) damage(a,b,400); away(a,b,-math.min(10,distance(a,b))) end,{additional=true})
action(19,"reprisal",17866,"self",0,45,0,0,function(a) for _,b in ipairs(enemies(a,a,5)) do buff(a,b,"reprisal",10) end end,{additional=true})
action(19,"rampart",17672,"self",0,45,0,0,function(a) buff(a,a,"rampart",10) end,{additional=true})
action(19,"retaliation",18990,"self",0,45,0,0,function(a) buff(a,a,"retaliation",10) end,{additional=true})

-- Verified March 2022 official guides: DRG and WAR additions.
local function reduce(a,name,seconds)
 local times=player(a).cooldowns[name] or {}
 for i,t in ipairs(times) do times[i]=math.max(M.now,t-seconds) end
 emit("job_cooldown",a.id,a.id,math.ceil(math.max(0,(times[1] or M.now)-M.now)*100),name)
end
local function line(a,b,r,potency,extra)
 local dx,dz=b.x-a.x,b.z-a.z; local length=math.sqrt(dx*dx+dz*dz)
 if length==0 then return end
 dx,dz=dx/length,dz/length
 for _,t in ipairs(enemies(a,a,r)) do
  local tx,tz=t.x-a.x,t.z-a.z; local forward=tx*dx+tz*dz
  -- Guide specifies a straight line, not width: provisional 2-yalm corridor.
  if forward>=0 and forward<=r and math.abs(tx*dz-tz*dx)<=1 then
   local hit=damage(a,t,potency); if extra then extra(t,hit) end
  end
 end
end
local function dragon(a,extend)
 local name=has(a.id,"life_dragon") and "life_dragon" or "blood_dragon"
 if extend then
  if has(a.id,name) then a.statuses[name].expires=math.min(M.now+30,a.statuses[name].expires+10) end
 elseif name=="blood_dragon" then buff(a,a,name,15) end
end
local function dragon_check(a) if not has(a.id,"blood_dragon") and not has(a.id,"life_dragon") then return "Blood or Life of the Dragon required." end end
weaponskill(22,"raiden_thrust",18916,"enemy",5,function(a,b) damage(a,b,1200); dragon(a,false); setcombo(a,"raiden_thrust") end,{combo=true})
for _,d in ipairs({{"full_thrust",8793,"raiden_thrust",1400},{"fang_and_claw",8794,"full_thrust",1600},{"wheeling_thrust",8798,"fang_and_claw",1600}}) do
 local name,id,previous,potency=table.unpack(d)
 weaponskill(22,name,id,"enemy",5,function(a,b) damage(a,b,potency); if name~="full_thrust" then dragon(a,true) end; setcombo(a,name=="wheeling_thrust" and nil or name) end,
  {combo=true,check=function(a) if not combo(a,previous) then return "Dragoon combo required." end end})
end
weaponskill(22,"doom_spike",18917,"enemy",10,function(a,b) line(a,b,10,800); dragon(a,false); setcombo(a,"doom_spike") end,{combo=true})
for _,d in ipairs({{"sonic_thrust",18918,"doom_spike",1000},{"coerthan_torment",18919,"sonic_thrust",1200}}) do
 local name,id,previous,potency=table.unpack(d)
 weaponskill(22,name,id,"enemy",10,function(a,b) line(a,b,10,potency); dragon(a,true); setcombo(a,name=="coerthan_torment" and nil or name) end,
  {combo=true,check=function(a) if not combo(a,previous) then return "Dragoon area combo required." end end})
end
weaponskill(22,"piercing_talon",8799,"enemy",15,function(a,b) damage(a,b,800+800*math.min(1,distance(a,b)/15)); dragon(a,true) end)
action(22,"high_jump",17728,"enemy",0,15,0,20,function(a,b) damage(a,b,2000); reduce(a,"mirage_dive",15) end,{check=bound})
action(22,"spineshatter_dive",8802,"enemy",0,30,0,20,function(a,b) move(a,b.x,b.y,b.z); damage(a,b,800); reduce(a,"mirage_dive",15) end,{check=bound})
action(22,"stardiver",17729,"enemy",0,45,0,20,function(a,b) move(a,b.x,b.y,b.z); area(a,b,5,2400) end,{check=function(a) return bound(a) or dragon_check(a) end})
action(22,"elusive_jump",8803,"self",0,30,0,0,function(a)
 a.statuses.heavy=nil; a.statuses.bind=nil; a.statuses.holmgang=nil
 for _,id in ipairs({13,14,1344,1345}) do emit("cleanse",a.id,a.id,id) end
 emit("remove_holmgang",a.id,a.id); move(a,a.x-math.sin(a.rotation or 0)*15,a.y,a.z-math.cos(a.rotation or 0)*15)
end)
action(22,"mirage_dive",17730,"enemy",0,15,0,20,function(a,b) damage(a,b,1000); gauge(a,"eyes",1,2) end,{charges=2,check=dragon_check})
for _,d in ipairs({{"geirskogul",8805,"blood_dragon","life_dragon",1600},{"nastrond",8806,"life_dragon","blood_dragon",2000}}) do
 local name,id,old,new,potency=table.unpack(d)
 action(22,name,id,"enemy",0,1,0,15,function(a,b)
  local duration=math.max(15,a.statuses[old].expires-M.now); a.statuses[old]=nil
  buff(a,a,new,duration); gauge(a,"eyes",-2,2); line(a,b,15,potency)
 end,{cooldown="geirskogul",check=function(a) if not has(a.id,old) or player(a).eyes<2 then return "Dragon stance and two eyes required." end end})
end
action(22,"life_surge",18920,"self",0,30,0,0,function(a) buff(a,a,"life_surge",5) end,{charges=2})
action(22,"dragon_sight",10032,"other_ally",0,60,0,15,function(a,b) buff(a,a,"right_eye",10); buff(a,b,"left_eye",10) end)

local function beast_check(cost)
 return function(a) if not has(a.id,"inner_release") and player(a).beast<cost then return "Not enough Beast Gauge." end end
end
local function spend_beast(a,cost) if not has(a.id,"inner_release") then gauge(a,"beast",-cost) end end
weaponskill(21,"heavy_swing",8758,"enemy",5,function(a,b) damage(a,b,1000); gauge(a,"beast",10); setcombo(a,"heavy_swing") end,{combo=true})
weaponskill(21,"maim",8761,"enemy",5,function(a,b) damage(a,b,1200); gauge(a,"beast",10); setcombo(a,"maim") end,{combo=true,check=function(a) if not combo(a,"heavy_swing") then return "Heavy Swing combo required." end end})
weaponskill(21,"storms_path",8762,"enemy",5,function(a,b) heal(a,a,damage(a,b,1400),true); gauge(a,"beast",20); setcombo(a,nil) end,{combo=true,check=function(a) if not combo(a,"maim") then return "Maim combo required." end end})
weaponskill(21,"mythril_tempest",18904,"self",0,function(a) area(a,a,5,600); gauge(a,"beast",10); setcombo(a,"mythril_tempest") end,{combo=true})
weaponskill(21,"steel_cyclone",18905,"self",0,function(a) area(a,a,5,800,function(_,hit) heal(a,a,hit,true) end); gauge(a,"beast",20); setcombo(a,nil) end,{combo=true,check=function(a) if not combo(a,"mythril_tempest") then return "Mythril Tempest combo required." end end})
weaponskill(21,"tomahawk",8764,"enemy",15,function(a,b) damage(a,b,800); gauge(a,"beast",10) end)
weaponskill(21,"fell_cleave",8763,"enemy",5,function(a,b) spend_beast(a,50); damage(a,b,2000); reduce(a,"infuriate",10) end,{check=beast_check(50)})
weaponskill(21,"decimate",17695,"self",0,function(a) spend_beast(a,50); area(a,a,5,1200,function() reduce(a,"infuriate",5) end) end,{check=beast_check(50)})
action(21,"onslaught",8765,"enemy",0,15,0,15,function(a,b) spend_beast(a,20); move(a,b.x,b.y,b.z); damage(a,b,1200) end,{check=function(a) return bound(a) or beast_check(20)(a) end})
action(21,"holmgang",8767,"enemy",0,45,0,10,function(a,b)
 move(b,a.x,a.y,a.z); buff(a,a,"holmgang",3)
 -- Explicitly bypass bind immunity for this action only.
 a.statuses.bind={source=a.id,expires=M.now+3}; b.statuses.bind={source=a.id,expires=M.now+3}
end)
action(21,"infuriate",17698,"self",0,30,0,0,function(a) buff(a,a,"nascent_chaos",5) end,{charges=2})
action(21,"inner_release",8768,"self",0,45,0,0,function(a) buff(a,a,"inner_release",6) end)
action(21,"nascent_flash",17889,"other_ally",0,15,0,30,function(a,b)
 buff(a,a,"nascent_flash",6); a.statuses.nascent_flash.partner=b.id; buff(a,b,"nascent_glint",6)
end)
action(21,"shake_it_off",17699,"self",0,45,0,0,function(a)
 local shield=has(a.id,"rampart") and 4000 or 2000; a.statuses.rampart=nil
 for _,b in ipairs(allies(a,a,15)) do buff(a,b,"shake_it_off",10); b.statuses.shake_it_off.shield=shield end
end)
for _,d in ipairs({{"inner_chaos",17696,false},{"chaotic_cyclone",17697,true}}) do
 local name,id,wide=table.unpack(d)
 weaponskill(21,name,id,wide and "self" or "enemy",wide and 0 or 5,function(a,b)
  a.statuses.nascent_chaos=nil
  local function hit(t) heal(a,a,damage(a,t,wide and 1200 or 2000),true); buff(a,t,"chaos_vulnerability",10) end
  if wide then for _,t in ipairs(enemies(a,a,5)) do hit(t) end else hit(b) end
 end,{check=function(a) if not has(a.id,"nascent_chaos") then return "Nascent Chaos required." end end})
end

local function absorb(b,amount)
 for _,name in ipairs({"testudo","manaward","shake_it_off"}) do
  if has(b.id,name) then
   local s=b.statuses[name]; local absorbed=math.min(amount,s.shield)
   s.shield=s.shield-absorbed; amount=amount-absorbed
   if s.shield==0 then b.statuses[name]=nil; if name=="manaward" then mp(b,2500) end end
  end
 end
 return amount
end
local function raw_damage(a,b,amount)
 if not valid(b) or b.invulnerable or has(b.id,"hallowed_ground") then return end
 if M.absorb_extra then amount=M.absorb_extra(a,b,amount) end
 local hit=math.min(b.hp,absorb(b,amount)); if has(b.id,"holmgang") then hit=math.min(hit,math.max(0,b.hp-1)) end; b.hp=b.hp-hit
 emit("hp",a.id,b.id,b.hp); emit("damage",a.id,b.id,hit)
 if hit>0 then b.statuses.sleep=nil end
 if M.players[a.id] then M.players[a.id].combat_until=M.now+10 end
 if M.players[b.id] then M.players[b.id].combat_until=M.now+10 end
 if M.after_damage then M.after_damage(a,b,hit) end
end
M.raw_adrenaline_damage=raw_damage
action(25,"cometeor",3361,"ground",2,0,0,25,function(a,b)
 for _,t in ipairs(enemies(a,b,8)) do raw_damage(a,t,4000); buff(a,t,"cometeor",10) end
end,{adrenaline=true,spell=true})
action(34,"raw_destruction",3360,"enemy",2,0,0,8,function(a,b) raw_damage(a,b,10000) end,{adrenaline=true,weapon=true})
action(19,"aegis_boon",3359,"self",0,0,0,0,function(a) for _,b in ipairs(allies(a,a,30)) do buff(a,b,"aegis_boon",10) end end,{adrenaline=true})
for _,name in ipairs(roles[22]) do A[name].jobs={[22]=true} end
for _,name in ipairs(roles[21]) do A[name].jobs={[21]=true} end
A.raw_destruction.jobs={[22]=true}; A.aegis_boon.jobs={[21]=true}

function M.immune(a,name)
 return (has(a.id,"fetter_ward") or has(a.id,"inner_release")) and (name=="stun" or name=="sleep" or name=="bind" or name=="heavy" or name=="silence")
end
function M.outgoing(id,healing)
 local factor=has(id,"confiteor") and 0.9 or 1
 if not healing then
  if has(id,"right_eye") or has(id,"left_eye") then factor=factor*1.1 end
  if M.players[id] and M.players[id].surge_active then factor=factor*1.5 end
  if has(id,"astral_fire") then factor=factor*1.2 end
  if has(id,"addle") then factor=factor*0.6 end
  if has(id,"reprisal") then factor=factor*0.8 end
  if has(id,"arms_debuff") then factor=factor*0.9 end
 end
 return factor
end
local reflecting=false
function M.incoming(a,b,amount)
 if has(b.id,"hallowed_ground") then return 0 end
 for _,entry in ipairs({{"phantom_dart",1.2},{"cometeor",1.25},{"rampart",0.8},{"arms_length",0.9},{"chiten",0.5},{"aegis_boon",0.75}}) do
  if has(b.id,entry[1]) then amount=amount*entry[2] end
 end
 if a and has(b.id,"feint_"..a.id) then amount=amount*1.2 end
 if has(b.id,"intervention") then amount=amount*(1-b.statuses.intervention.reduction) end
 if has(b.id,"nascent_flash") or has(b.id,"nascent_glint") then amount=amount*0.9 end
 if has(b.id,"chaos_vulnerability") then amount=amount*1.1 end
 amount=math.floor(amount)
 amount=absorb(b,amount)
 if has(b.id,"holmgang") then amount=math.min(amount,math.max(0,b.hp-1)) end
 return amount
end
function M.after_damage(a,b,amount)
 if has(a.id,"bloodbath") and (a.job==34 or a.job==22) then heal(a,a,amount,true) end
 if has(a.id,"nascent_flash") then
  heal(a,a,amount,true)
  local partner=state(a.statuses.nascent_flash.partner)
  if valid(partner) and has(partner.id,"nascent_glint") then heal(a,partner,amount,true) end
 end
 for key,s in pairs(b.statuses) do
  if s.threshold then
   s.received=s.received+amount
   if s.received>s.threshold then
    local caster=state(s.source); if valid(caster) then buff(caster,caster,"thundercloud",10) end
    b.statuses[key]=nil
   end
  end
 end
 if amount<=0 or reflecting or not valid(a) or not valid(b) then return end
 if has(b.id,"arms_length") then buff(b,a,"arms_debuff",6) end
 local potency=has(b.id,"retaliation") and 800 or 0
 if has(b.id,"chiten") and b.statuses.chiten.counters>0 then
  b.statuses.chiten.counters=b.statuses.chiten.counters-1; potency=1200; gauge(b,"kenki",5)
 end
 if potency>0 then reflecting=true; damage(b,a,potency); reflecting=false end
end
function M.effective(a,d)
 local result={}; for k,v in pairs(d) do result[k]=v end
 if a then
  if d.target=="ground" then result.point=player(a) and player(a).ground end
  if d.spell and has(a.id,"umbral_ice") then result.cast=d.cast*0.8; result.recast=d.recast*0.8 end
  if (d.spell or d.weapon) and has(a.id,"aetheric_burst") then result.cast=result.cast*1.2; result.recast=result.recast*1.2 end
  if d.spell and has(a.id,"swiftcast") then result.cast=0 end
  if a.job==19 and d.spell and has(a.id,"requiescat") then result.cast=0; result.mp=0 end
 end
 return result
end
function M.ground_command(id,name,x,y,z,now)
 local a=state(id); local p=M.players[id]
 if not a or not p or (name~="soten" and name~="cometeor") then return "Invalid Feast ground action." end
 if x~=x or y~=y or z~=z or math.abs(x)>1000 or math.abs(y)>1000 or math.abs(z)>1000 then return "Invalid ground position." end
 p.ground={id=id,x=x,y=y,z=z,hp=1,max_hp=1,mp=0,statuses={}}
 local result=M.command(id,name,id,now)
 p.ground=nil
 return result
end
function M.resolve(a,name)
 local p=player(a)
 if name=="raiden_thrust" then return combo(a,"raiden_thrust") and "full_thrust" or combo(a,"full_thrust") and "fang_and_claw" or combo(a,"fang_and_claw") and "wheeling_thrust" or name end
 if name=="doom_spike" then return combo(a,"doom_spike") and "sonic_thrust" or combo(a,"sonic_thrust") and "coerthan_torment" or name end
 if name=="geirskogul" and has(a.id,"life_dragon") then return "nastrond" end
 if name=="heavy_swing" then return combo(a,"heavy_swing") and "maim" or combo(a,"maim") and "storms_path" or name end
 if name=="mythril_tempest" and combo(a,"mythril_tempest") then return "steel_cyclone" end
 if name=="fell_cleave" and has(a.id,"nascent_chaos") then return "inner_chaos" end
 if name=="decimate" and has(a.id,"nascent_chaos") then return "chaotic_cyclone" end
 if name=="fire" or name=="blizzard" then return name end
 if name=="enochian" and p.enochian then return has(a.id,"astral_fire") and "fire_iv" or "blizzard_iv" end
 if name=="thunder" and has(a.id,"thundercloud") then return "thunder_iii" end
 if name=="thunder_ii" and has(a.id,"thundercloud") then return "thunder_iv" end
 if name=="tsubame_gaeshi" and p.repeat_action then return p.repeat_action=="midare_setsugekka" and "kaeshi_setsugekka" or "kaeshi_goken" end
 if name=="yukikaze" then return combo(a,"yukikaze") and "gekko" or combo(a,"gekko") and "kasha" or name end
 if name=="mangetsu" and combo(a,"mangetsu") then return "oka" end
 if name=="fast_blade" then
  if has(a.id,"sword_oath") and p.sword>0 then return "atonement" end
  return combo(a,"fast_blade") and "riot_blade" or combo(a,"riot_blade") and "royal_authority" or name
 end
 if name=="total_eclipse" then
  if has(a.id,"sword_oath") and p.sword>0 then return "glory_slash" end
  if combo(a,"total_eclipse") then return "prominence" end
 end
 return name
end
function M.before_action(a,d)
 if a.job==22 and d.weapon and d.gcd and has(a.id,"life_surge") then player(a).surge_active=true end
 if d.spell and A[d.name].cast>0 then a.statuses.swiftcast=nil end
end
function M.after_action(a,d)
 local p=player(a)
 if p.surge_active then p.surge_active=false; a.statuses.life_surge=nil end
 if d.weapon and not d.combo then p.combo=nil end
 if d.weapon then
  p.repeat_action=(d.name=="midare_setsugekka" or d.name=="tenka_goken") and d.name or nil
 end
end
function M.advance_job(a,p,elapsed)
 if a.job==22 and not has(a.id,"blood_dragon") and not has(a.id,"life_dragon") then p.eyes=0 end
 if a.job==25 then
  if not has(a.id,"astral_fire") and not has(a.id,"umbral_ice") then p.enochian=nil; p.poly_time=0
  elseif p.enochian and a.match_active~=false then
   p.poly_time=p.poly_time+elapsed
   while p.poly_time>=15 do gauge(a,"polyglot",1,2); p.poly_time=p.poly_time-15 end
  end
 end
end
function M.hotbar(id)
 local a=state(id); local p=M.players[id]; local result={}
 if not a or not p or not M.jobs[a.job] then return result end
 for name,d in pairs(A) do
  if d.job==a.job or (d.jobs and d.jobs[a.job]) or name=="bolt" or name=="kit" then
   local resolved=M.resolve(a,name); local current=M.effective(a,A[resolved])
   table.insert(result,{d.id,current.id,math.floor(current.cast*1000),math.floor((current.cooldown_recast or current.recast)*1000)})
  end
 end
 table.sort(result,function(a,b) return a[1]<b[1] end)
 return result
end
function M.job_gauge(id)
 local a=state(id); local p=M.players[id]
 if a and p and a.job==24 then
  return {24,p.lilies,p.blood,0,0,0,math.floor(p.lily_time*1000)}
 end
 if not a or not p or not M.jobs[a.job] then return {0,0,0,0,0,0,0} end
 if a.job==21 then return {21,p.beast,0,0,0,0,0} end
 if a.job==22 then
  local s=a.statuses[has(a.id,"life_dragon") and "life_dragon" or "blood_dragon"]
  return {22,p.eyes,0,0,has(a.id,"life_dragon") and 2 or has(a.id,"blood_dragon") and 1 or 0,0,math.floor(math.max(0,s and s.expires-M.now or 0)*1000)}
 end
 -- BLM has no Kenki: use this otherwise-unused byte for element time in
 -- 100ms units, separately from the Polyglot timer at the end of F7.
 local resource=a.job==19 and p.oath or p.kenki
 if a.job==25 then
  local effect=a.statuses[has(id,"astral_fire") and "astral_fire" or "umbral_ice"]
  resource=math.min(150,math.ceil(math.max(0,effect and effect.expires-M.now or 0)*10))
 end
 return {a.job,resource,p.meditation,p.polyglot,
     has(id,"astral_fire") and 1 or has(id,"umbral_ice") and 2 or 0,
     p.enochian and 1 or 0,math.floor(math.max(0,15-p.poly_time)*1000)}
end
end
