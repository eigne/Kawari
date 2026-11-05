-- The other 13 Feast jobs: Monk, Ninja, Bard, Machinist, Summoner, Red Mage,
-- Scholar, Astrologian, Dark Knight, Gunbreaker, Reaper, Sage and Dancer.
--
-- D is AdditionalActionData.lua: each action's native 6.08 metadata (ID,
-- cast, recast, range, cooldown groups), generated from the Action sheet.
-- action(key, run, extra) combines that metadata with the effect written
-- here, so only gameplay is hand-written. Every key in D must get an
-- implementation (checked at the bottom), so a missing action fails at
-- startup instead of silently doing nothing.
--
-- Each hook defined in Jobs.lua is wrapped, not replaced: the old version is
-- kept in a local and called first (see M.outgoing, M.resolve, ...).
return function(M,D)
local I=M.internal
local add,state,has,buff,damage,heal,emit=I.add,I.state,I.has,I.buff,I.damage,I.heal,I.emit
local allies,enemies,valid,distance=I.allies,I.enemies,I.valid,I.distance
local A=M.actions
-- Retained 6.08 statuses drive native movement restrictions/stealth as well as
-- party-list icons. Lua remains authoritative for action eligibility.
local native_controls={stun=1343,heavy=1344,bind=1345,silence=1347,sleep=1348,kakuremi=1315,hidden=1316}
local sent_controls={}
function M.owns_native_status(id,status) return sent_controls[id] and sent_controls[id][status]~=nil end
local function sync_controls()
 for id,a in pairs(M.actors) do
  local sent=sent_controls[id] or {}; sent_controls[id]=sent
  for key,status in pairs(native_controls) do
   local s=a.statuses[key]; local deadline=s and s.expires>M.now and s.expires or 0
   if deadline~=(sent[status] or 0) then
    emit('control_visual',s and s.source or id,id,math.ceil(math.max(0,deadline-M.now)*1000),tostring(status))
    sent[status]=deadline
   end
  end
 end
 for id in pairs(sent_controls) do if not M.actors[id] then sent_controls[id]=nil end end
end
local function p(a) return M.players[a.id] end
local function gain(a,key,n,cap) p(a)[key]=math.max(0,math.min(cap or 100,(p(a)[key] or 0)+n)) end
local function mp(a,n) a.mp=math.max(0,math.min(10000,a.mp+n)); emit('mp',a.id,a.id,a.mp) end
local function move(a,x,y,z) a.x,a.y,a.z=x,y,z; table.insert(M.events,{kind='move',actor=a.id,target=a.id,x=x,y=y,z=z}) end
local function step(a,n) move(a,a.x+math.sin(a.rotation or 0)*n,a.y,a.z+math.cos(a.rotation or 0)*n) end
local function bound(a) if M.blocked(a.id,true) then return 'Cannot move while bound or incapacitated.' end end
local function require_resource(key,n) return function(a) if (p(a)[key] or 0)<n then return key..' required.' end end end
local function require_buff(key) return function(a) if not has(a.id,key) then return key..' required.' end end end
local function shield(a,b,key,n,duration) buff(a,b,key,duration); b.statuses[key].shield=n end
local function dot(a,b,key,n,duration) key=key..'_'..a.id; buff(a,b,key,duration,n); b.statuses[key].dot=true end
local function regen(a,b,key,n,duration) buff(a,b,key,duration,n*I.outgoing(a.id,true)) end
local function strip(b) for key,s in pairs(b.statuses) do if s.shield or (s.potency and not s.dot) then b.statuses[key]=nil end end end
local function area(a,b,r,n,extra) for _,t in ipairs(enemies(a,b,r)) do local hit=damage(a,t,n); if extra then extra(t,hit) end end end
local function shape(a,b,r,n,cone,extra)
 local dx,dz=b.x-a.x,b.z-a.z; local len=math.sqrt(dx*dx+dz*dz); if len==0 then dx,dz,len=math.sin(a.rotation or 0),math.cos(a.rotation or 0),1 end
 for _,t in ipairs(enemies(a,a,r)) do
  local x,z=t.x-a.x,t.z-a.z; local along=(x*dx+z*dz)/len; local across=math.abs(x*dz-z*dx)/len
  if along>=0 and (cone and across<=along or not cone and across<=1) then local hit=damage(a,t,n); if extra then extra(t,hit) end end
 end
end
local function reduce(a,name,n)
 local d=A[name]; local times=p(a).cooldowns[d.cooldown or name] or {}
 for i=1,#times do times[i]=times[i]-n end
 emit('job_cooldown',a.id,a.id,0,name)
end
local function partybuff(a,key,duration,radius) for _,b in ipairs(allies(a,a,radius or 30)) do buff(a,b,key,duration) end end
local function combo(a,key) return p(a).combo==key and (p(a).combo_until or 0)>M.now end
local function setcombo(a,key) p(a).combo=key; p(a).combo_until=M.now+15 end
local function action(key,run,extra)
 local d=assert(D[key],key); extra=extra or {}; extra.job=d.job; extra.charges=d.charges
 extra.weapon=d.category=='Weaponskill'; extra.spell=d.category=='Spell'
 extra.gcd=d.group==57 or d.secondary==57
 if not extra.gcd and not extra.cooldown then extra.cooldown='job_'..d.job..'_group_'..d.group end
 -- Separate recast groups on weaponskills (Drill, dances, etc.) still reserve
 -- their own cooldown in addition to the common weaponskill timer.
 local recast=d.recast
 if d.secondary==57 and d.group~=57 then extra.cooldown_recast=d.recast; recast=2.4 end
 add(key,d.id,extra.target or d.target,d.cast,recast,d.mpCost,d.range,run,extra)
end
local function hit(key,n,extra) action(key,function(a,b) damage(a,b,n) end,extra) end
local function chain(keys,potencies,extra,wide)
 for i,key in ipairs(keys) do action(key,function(a,b)
  local function apply(t) local n=damage(a,t,potencies[i]); if extra then extra(a,t,i,n) end end
  if wide then for _,t in ipairs(enemies(a,a,D[key].radius)) do apply(t) end else apply(b) end
  setcombo(a,i==#keys and nil or key)
 end,{combo=true,check=i>1 and function(a) if not combo(a,keys[i-1]) then return 'Previous combo action required.' end end or nil}) end
end
local roles={ [20]=34,[30]=34,[32]=19,[37]=19,[27]=25,[35]=25,[28]=24,[33]=24,[23]=23,[31]=23,[38]=23,[39]=34,[40]=24 }
local oldparty=M.party_list
local ranged={'head_graze','leg_graze','foot_graze','concentrate','recuperate','peloton'}
function M.party_list(job) return roles[job]==23 and ranged or oldparty(roles[job] or job) end
for job,role in pairs(roles) do
 M.jobs[job]=true
 if role~=23 then
  for _,name in ipairs(oldparty(role)) do A[name].jobs=A[name].jobs or {}; A[name].jobs[job]=true end
  local name=({[34]='raw_destruction',[19]='aegis_boon',[25]='cometeor',[24]='rain'})[role]
  A[name].jobs=A[name].jobs or {}; A[name].jobs[job]=true
 end
end
local reset=M.reset_job
function M.reset_job(s)
 reset(s)
 for _,key in ipairs({'chakra','ninki','soul_voice','repertoire','heat','esprit','feathers','aetherflow','black_mana','white_mana','faerie','dark_blood','cartridges','soul','shroud','lemure','addersgall','addersting'}) do s[key]=0 end
 s.dance=nil; s.partner=nil; s.card=nil; s.firebird_next=false; s.kakuremi_until=nil; s.trance_until=nil; s.contributions={}; s.gall_time=0; s.kardia=nil; s.void_shroud=0
end
local initialize=M.initialize
function M.initialize(a,s)
 if not roles[a.job] then return initialize(a,s) end
 M.reset_job(s); local r=M.party_list(a.job); s.loadout={[r[2]]=true,[r[3]]=true}
end
local allowed=M.allowed
function M.allowed(a,d)
 if not d.additional then return true end
 for _,name in ipairs(M.party_list(a.job)) do if name==d.name then return true end end
 return false
end

-- Monk: chakra generation, two-charge Tornado Kick and Brotherhood procs.
chain({'bootshine','true_strike','snap_punch'},{1000,1200,1400},function(a) gain(a,'chakra',1,5) end)
chain({'four_point_fury','rockbreaker'},{600,800},nil,true)
for _,key in ipairs({'four_point_fury','rockbreaker'}) do local run=A[key].run; A[key].run=function(a,b) run(a,b); gain(a,'chakra',1,5) end end
action('six_sided_star',function(a,b) damage(a,b,2400); gain(a,'chakra',1,5) end)
action('the_forbidden_chakra',function(a,b) gain(a,'chakra',-5,5); damage(a,b,2000) end,{check=require_resource('chakra',5)})
action('enlightenment',function(a,b) gain(a,'chakra',-5,5); shape(a,b,D.enlightenment.range,1200,false) end,{check=require_resource('chakra',5)})
hit('tornado_kick',1800)
action('shoulder_tackle',function(a,b) move(a,b.x,b.y,b.z); damage(a,b,800) end,{check=bound})
action('axe_kick',function(a) area(a,a,D.axe_kick.radius,1000,function() gain(a,'chakra',1,5) end) end)
action('riddle_of_fire',function(a) buff(a,a,'riddle_of_fire',6); buff(a,a,'riddle_weapon',6) end)
action('riddle_of_earth',function(a) shield(a,a,'riddle_of_earth',2500,6) end)
action('brotherhood',function(a) partybuff(a,'brotherhood',10) end)

-- Ninja: mudra transformations, Bunshin, stealth and Ninki.
chain({'spinning_edge','gust_slash','aeolian_edge'},{1000,1200,1400},function(a) gain(a,'ninki',10) end)
chain({'death_blossom','hakke_mujinsatsu'},{600,800},nil,true)
for _,key in ipairs({'death_blossom','hakke_mujinsatsu'}) do local run=A[key].run; A[key].run=function(a,b) run(a,b); gain(a,'ninki',10) end end
action('throwing_dagger',function(a,b) damage(a,b,800); gain(a,'ninki',10) end)
action('bhavacakra',function(a,b) gain(a,'ninki',-50); damage(a,b,2000) end,{check=require_resource('ninki',50)})
action('hellfrog_medium',function(a,b) gain(a,'ninki',-50); area(a,b,D.hellfrog_medium.radius,1200) end,{check=require_resource('ninki',50)})
action('assassinate',function(a,b) damage(a,b,1000+2000*(1-b.hp/b.max_hp)) end)
action('ill_wind',function(a) area(a,a,D.ill_wind.radius,800,function(t) buff(a,t,'silence',1); emit('interrupt',a.id,t.id) end) end)
action('three_mudra',function(a) buff(a,a,'three_mudra',5) end)
action('bunshin',function(a) buff(a,a,'bunshin',10); a.statuses.bunshin.stacks=3 end)
action('shukuchi',function(a,b) move(a,b.x,b.y,b.z); shield(a,a,'shukuchi',2000,6) end,{check=bound})
action('kakuremi',function(a) buff(a,a,'kakuremi',3); p(a).kakuremi_until=M.now+3 end)
action('hyosho_ranryu',function(a,b) a.statuses.three_mudra=nil; damage(a,b,2000); gain(a,'ninki',20); buff(a,a,'huton',35) end,{check=require_buff('three_mudra')})
action('goka_mekkyaku',function(a,b) a.statuses.three_mudra=nil; area(a,b,D.goka_mekkyaku.radius,1200,function() gain(a,'ninki',10) end); buff(a,a,'huton',35) end,{check=require_buff('three_mudra')})
action('trick_attack',function(a,b) damage(a,b,1200); buff(a,b,'trick_attack',6); a.statuses.hidden=nil end,{check=require_buff('hidden')})

-- Bard and Machinist.
local function song(a,amount,stack)
 if has(a.id,'wanderers_minuet') or has(a.id,'armys_paeon') then gain(a,'soul_voice',amount); if stack then gain(a,'repertoire',1,has(a.id,'wanderers_minuet') and 3 or 4) end end
end
action('burst_shot',function(a,b) damage(a,b,1200); song(a,10,true) end)
action('quick_nock',function(a,b) shape(a,b,D.quick_nock.range,800,true,function() song(a,10,false) end) end)
action('apex_arrow',function(a,b) local n=24*p(a).soul_voice; p(a).soul_voice=0; shape(a,b,D.apex_arrow.range,n,false) end,{check=require_resource('soul_voice',50)})
action('empyreal_arrow',function(a,b) damage(a,b,1000); song(a,10,true) end)
action('pitch_perfect',function(a,b) damage(a,b,600*p(a).repertoire); p(a).repertoire=0 end,{check=function(a) return require_buff('wanderers_minuet')(a) or require_resource('repertoire',1)(a) end})
action('sidewinder',function(a,b) damage(a,b,800+1600*(1-b.hp/b.max_hp)) end)
action('shadowbite',function(a,b) area(a,b,D.shadowbite.radius,600+1200*b.hp/b.max_hp) end)
action('repelling_shot',function(a,b) damage(a,b,800); step(a,-10); song(a,20,true) end,{check=bound})
for _,key in ipairs({'the_wanderers_minuet','armys_paeon'}) do action(key,function(a,b)
 damage(a,b,600); a.statuses.wanderers_minuet=nil; a.statuses.armys_paeon=nil; p(a).repertoire=0; buff(a,a,key=='the_wanderers_minuet' and 'wanderers_minuet' or key,30)
end) end
action('natures_minne',function(a,b) buff(a,b,'natures_minne',15) end)
chain({'heated_split_shot','heated_slug_shot','heated_clean_shot'},{1000,1200,1400},function(a) gain(a,'heat',10) end)
action('spread_shot',function(a,b) shape(a,b,D.spread_shot.range,800,true,function() gain(a,'heat',5) end) end)
hit('drill',2000)
action('air_anchor',function(a,b) damage(a,b,2000); gain(a,'heat',20) end)
action('bioblaster',function(a,b) shape(a,b,D.bioblaster.range,1600,true,function(t) buff(a,t,'bioblaster',10) end) end)
hit('gauss_round',1200)
action('ricochet',function(a,b) area(a,b,D.ricochet.radius,800) end)
action('wildfire',function(a,b) buff(a,b,'wildfire_'..a.id,6); b.statuses['wildfire_'..a.id].stored=0 end)
action('blank',function(a,b) damage(a,b,has(b.id,'wildfire_'..a.id) and 2400 or 800)
 if not M.immune(b,'bind') then local dx,dz=b.x-a.x,b.z-a.z; local len=math.sqrt(dx*dx+dz*dz); if len>0 then move(b,b.x+15*dx/len,b.y,b.z+15*dz/len) end end
end)
action('hypercharge',function(a) gain(a,'heat',-50); local remaining=has(a.id,'overheated') and a.statuses.overheated.expires-M.now or 0; buff(a,a,'overheated',math.min(10,remaining+5)) end,{check=require_resource('heat',50)})
action('tactician',function(a) partybuff(a,'tactician',15) end)
action('heat_blast',function(a,b) damage(a,b,1400); reduce(a,'gauss_round',7.5); reduce(a,'ricochet',7.5) end,{check=require_buff('overheated')})
action('auto_crossbow',function(a,b) shape(a,b,D.auto_crossbow.range,1600,true) end,{check=require_buff('overheated')})

-- Summoner: original DoT/Aetherflow kit, not the Endwalker PvE summons.
action('ruin_iii',function(a,b) damage(a,b,1600) end)
action('bio_iii',function(a,b) dot(a,b,'bio',600,18) end)
action('energy_drain',function(a,b) damage(a,b,800); gain(a,'aetherflow',1,2) end)
action('tri_disaster',function(a,b) damage(a,b,800); dot(a,b,'bio',600,18); gain(a,'aetherflow',1,2) end)
action('outburst',function(a,b) area(a,b,D.outburst.radius,1000) end)
action('fester',function(a,b) gain(a,'aetherflow',-1,2); damage(a,b,has(b.id,'bio_'..a.id) and 2000 or 1000) end,{check=require_resource('aetherflow',1)})
action('painflare',function(a,b) gain(a,'aetherflow',-1,2); area(a,b,D.painflare.radius,1200,function(t) buff(a,t,'stun',2); emit('interrupt',a.id,t.id) end) end,{check=require_resource('aetherflow',1)})
action('bane',function(a,b)
 gain(a,'aetherflow',-1,2); local s=b.statuses['bio_'..a.id]
 if s and s.expires>M.now then for _,t in ipairs(enemies(a,b,D.bane.radius)) do if t.id~=b.id then dot(a,t,'bio',600,s.expires-M.now); t.statuses['bio_'..a.id].next_tick=s.next_tick end end end
end,{check=require_resource('aetherflow',1)})
action('wither',function(a,b) shape(a,b,D.wither.range,1600,true,function(t) strip(t) end) end)
action('deathflare',function(a,b) area(a,b,D.deathflare.radius,2400) end,{check=require_buff('dreadwyrm_trance')})
for _,key in ipairs({'dreadwyrm_trance','firebird_trance'}) do action(key,function(a)
 gain(a,'aetherflow',-2,2); buff(a,a,key,15); p(a).trance_until=M.now+15; p(a).trance_kind=key; p(a).firebird_next=key=='dreadwyrm_trance'; reduce(a,'tri_disaster',1000)
end,{check=function(a) if has(a.id,'dreadwyrm_trance') or has(a.id,'firebird_trance') then return 'Already in trance.' end; if key=='firebird_trance' and not p(a).firebird_next then return 'Dreadwyrm Trance required first.' end; return require_resource('aetherflow',2)(a) end}) end
hit('fountain_of_fire',2000,{check=require_buff('firebird_trance')})
action('brand_of_purgatory',function(a,b) area(a,b,D.brand_of_purgatory.radius,1200) end,{check=require_buff('firebird_trance')})

-- Red Mage: dualcast, balanced mana and the complete melee/finisher chain.
chain({'enchanted_riposte','enchanted_zwerchhau','enchanted_redoublement'},{1600,1800,2000},function(a,_,i) if i==1 then gain(a,'black_mana',-50); gain(a,'white_mana',-50) elseif i==3 then buff(a,a,'verfinisher',15) end end)
A.enchanted_riposte.check=function(a) return require_resource('black_mana',50)(a) or require_resource('white_mana',50)(a) end
for _,d in ipairs({{'verstone','white_mana',1400,10},{'verfire','black_mana',1400,10},{'veraero','white_mana',1800,15},{'verthunder','black_mana',1800,15},{'verholy','white_mana',2400,20},{'verflare','black_mana',2400,20}}) do
 action(d[1],function(a,b) damage(a,b,d[3]); gain(a,d[2],d[4]); if d[1]=='verholy' or d[1]=='verflare' then a.statuses.verfinisher=nil; buff(a,a,'scorch_ready',15) end end,
 {check=(d[1]=='veraero' or d[1]=='verthunder') and require_buff('dualcast') or (d[1]=='verholy' or d[1]=='verflare') and require_buff('verfinisher') or nil})
end
for _,key in ipairs({'scatter','impact'}) do action(key,function(a,b) area(a,b,D[key].radius,key=='scatter' and 800 or 1200,function() gain(a,'white_mana',5); gain(a,'black_mana',5) end) end,{check=key=='impact' and require_buff('dualcast') or nil}) end
action('vercure',function(a,b) heal(a,b,3000); if a.combat or p(a).combat_until>M.now or (M.players[b.id] and M.players[b.id].combat_until>M.now) then gain(a,'white_mana',5); gain(a,'black_mana',5) end end)
for _,key in ipairs({'enchanted_reprise','enchanted_moulinet'}) do action(key,function(a,b) gain(a,'black_mana',-25); gain(a,'white_mana',-25); if key=='enchanted_reprise' then damage(a,b,2000) else shape(a,b,D[key].range,1600,true) end end,{check=function(a) return require_resource('black_mana',25)(a) or require_resource('white_mana',25)(a) end}) end
action('corps_a_corps',function(a,b) move(a,b.x,b.y,b.z); damage(a,b,800); buff(a,b,'heavy',5) end,{check=bound})
action('displacement',function(a,b) damage(a,b,800); step(a,-15); buff(a,b,'bind',3) end,{check=bound})
action('engagement',function(a,b) damage(a,b,1200); shield(a,a,'engagement',2500,6) end)
action('embolden',function(a) partybuff(a,'embolden',15) end)
action('scorch',function(a,b) a.statuses.scorch_ready=nil; damage(a,b,3000); gain(a,'black_mana',10); gain(a,'white_mana',10) end,{check=require_buff('scorch_ready')})

-- Scholar and Astrologian.
local function healing_combat(a,b) return a.combat or p(a).combat_until>M.now or b.combat or M.players[b.id] and M.players[b.id].combat_until>M.now end
for _,key in ipairs({'physick','adloquium','succor'}) do action(key,function(a,b)
 local targets=key=='succor' and allies(a,a,D[key].radius) or {b}; local combat=false
 for _,t in ipairs(targets) do local n=key=='succor' and 1000 or 2000; heal(a,t,n); if key~='physick' then shield(a,t,'galvanize',I.healing_potency(a,t,n),10) end; combat=combat or healing_combat(a,t) end
 if combat then gain(a,'faerie',10) end
end) end
action('broil_iii',function(a,b) damage(a,b,1200); gain(a,'faerie',20) end)
action('biolysis',function(a,b) damage(a,b,800); buff(a,b,'biolysis',15); gain(a,'faerie',20) end)
action('lustrate',function(a,b) heal(a,b,3000) end,{cooldown='sch_heal'})
action('indomitability',function(a) for _,b in ipairs(allies(a,a,D.indomitability.radius)) do heal(a,b,1500) end end,{cooldown='sch_heal'})
action('excogitation',function(a,b) buff(a,b,'excogitation',15) end)
action('focalization',function(a) partybuff(a,'focalization',15) end)
action('aura_blast',function(a) area(a,a,D.aura_blast.radius,2000,function() gain(a,'faerie',10) end) end)
action('dissipation',function(a) gain(a,'faerie',-50); buff(a,a,'dissipation',10); reduce(a,'lustrate',15) end,{check=require_resource('faerie',50)})
for _,key in ipairs({'benefic','benefic_ii','helios','aspected_benefic','aspected_helios'}) do action(key,function(a,b)
 local wide=key=='helios' or key=='aspected_helios'; local aspect=key=='aspected_benefic' or key=='aspected_helios'
 for _,t in ipairs(wide and allies(a,a,D[key].radius) or {b}) do heal(a,t,key=='benefic_ii' and 4000 or 2000)
  if aspect then regen(a,t,'aspected_regen',wide and 400 or 800,15); shield(a,t,'aspected_barrier',I.healing_potency(a,t,wide and 1000 or 2000),10) end
 end
end,{check=(key=='aspected_benefic' or key=='aspected_helios') and require_buff('neutral_sect') or nil}) end
hit('malefic_iv',1200)
action('combust_iii',function(a,b) damage(a,b,800); buff(a,b,'combust',15) end)
action('gravity',function(a,b) area(a,b,D.gravity.radius,1600+1600*b.hp/b.max_hp) end)
action('essential_dignity',function(a,b) heal(a,b,2500+2500*(1-b.hp/b.max_hp)) end)
action('celestial_opposition',function(a)
 for _,b in ipairs(enemies(a,a,D.celestial_opposition.radius)) do strip(b) end
 for _,b in ipairs(allies(a,a,D.celestial_opposition.radius)) do regen(a,b,'celestial_regen',800,15); shield(a,b,'celestial_barrier',2000,10) end
end)
action('minor_arcana',function(a) p(a).card=math.random(2)==1 and 'lord_of_crowns' or 'lady_of_crowns' end,{cooldown='arcana'})
for _,key in ipairs({'lord_of_crowns','lady_of_crowns'}) do action(key,function(a,b) p(a).card=nil; buff(a,b,key,15) end,{cooldown='arcana',check=function(a) if p(a).card~=key then return 'Draw the corresponding card first.' end end}) end
action('divination',function(a) partybuff(a,'divination',15) end)
action('neutral_sect',function(a) buff(a,a,'neutral_sect',15) end)

-- Dark Knight.
chain({'hard_slash','syphon_strike','souleater'},{1000,1200,1400},function(a,_,i,n) if i==2 then mp(a,500) elseif i==3 then heal(a,a,n,true); gain(a,'dark_blood',25) end end)
chain({'unleash','stalwart_soul'},{600,800},function(a,_,i,n) if i==1 then mp(a,250) else heal(a,a,n,true) end end,true)
local stalwart=A.stalwart_soul.run; A.stalwart_soul.run=function(a,b) stalwart(a,b); gain(a,'dark_blood',25) end
action('unmend',function(a,b) damage(a,b,800); gain(a,'dark_blood',10) end)
for _,key in ipairs({'bloodspiller','quietus'}) do action(key,function(a,b)
 if not has(a.id,'delirium') then gain(a,'dark_blood',-50) end
 if key=='bloodspiller' then heal(a,a,damage(a,b,1800),true); mp(a,500) else area(a,a,D.quietus.radius,1000,function(_,n) heal(a,a,n,true); mp(a,250) end) end
end,{check=function(a) if not has(a.id,'delirium') then return require_resource('dark_blood',50)(a) end end}) end
for _,key in ipairs({'edge_of_shadow','flood_of_shadow'}) do action(key,function(a,b)
 a.statuses.dark_arts=nil; local remaining=has(a.id,'darkside') and a.statuses.darkside.expires-M.now or 0; buff(a,a,'darkside',math.min(30,remaining+15))
 if key=='edge_of_shadow' then damage(a,b,1000); buff(a,b,'shadow_heal_down',10) else shape(a,b,D[key].range,600,false,function(t) buff(a,t,'shadow_heal_down',10) end) end
end) end
action('plunge',function(a,b) move(a,b.x,b.y,b.z); damage(a,b,600) end,{check=bound})
action('delirium',function(a) buff(a,a,'delirium',6) end)
action('the_blackest_night',function(a,b) shield(a,b,'blackest_night',3000,6) end)
action('dark_missionary',function(a) partybuff(a,'dark_missionary',10) end)

-- Gunbreaker: cartridge combos, continuations and target-role junctions.
chain({'keen_edge','brutal_shell','solid_barrel'},{1000,1200,1400},function(a,_,i) if i==2 then shield(a,a,'brutal_shell',1200,10) elseif i==3 then gain(a,'cartridges',1,2) end end)
chain({'demon_slice','demon_slaughter'},{600,800},nil,true)
local slaughter=A.demon_slaughter.run; A.demon_slaughter.run=function(a,b) slaughter(a,b); shield(a,a,'brutal_shell',800,10); gain(a,'cartridges',1,2) end
chain({'gnashing_fang','savage_claw','wicked_talon'},{1400,1600,1800},function(a,_,i,n)
 if i==1 then gain(a,'cartridges',-1,2) end; heal(a,a,n,true)
 a.statuses.ready_rip=nil; a.statuses.ready_tear=nil; a.statuses.ready_gouge=nil; buff(a,a,({'ready_rip','ready_tear','ready_gouge'})[i],5)
end)
A.gnashing_fang.check=require_resource('cartridges',1)
action('lightning_shot',function(a,b) damage(a,b,800); buff(a,a,'lightning_bonus',5) end)
for _,key in ipairs({'burst_strike','fated_circle'}) do action(key,function(a,b)
 gain(a,'cartridges',-1,2); if key=='burst_strike' then heal(a,a,damage(a,b,1800),true) else area(a,a,D[key].radius,1200,function(_,n) heal(a,a,n,true) end) end
end,{check=require_resource('cartridges',1)}) end
action('continuation',function() end,{check=function() return 'Execute Gnashing Fang combo first.' end})
for _,d in ipairs({{'jugular_rip','ready_rip'},{'abdomen_tear','ready_tear'},{'eye_gouge','ready_gouge'}}) do action(d[1],function(a,b) a.statuses[d[2]]=nil; damage(a,b,800) end,{check=require_buff(d[2])}) end
action('rough_divide',function(a,b) move(a,b.x,b.y,b.z); damage(a,b,600); gain(a,'cartridges',1,2) end,{check=bound})
action('draw_and_junction',function(a,b)
 a.statuses.draw_power=nil; a.statuses.draw_fortitude=nil
 local defensive=({[19]=true,[21]=true,[32]=true,[37]=true,[20]=true,[22]=true,[30]=true,[34]=true,[39]=true})[b.job]
 buff(a,a,defensive and 'draw_fortitude' or 'draw_power',15); buff(a,b,defensive and 'junction_vulnerable' or 'junction_weak',15)
end)
action('bow_shock',function(a) area(a,a,D.bow_shock.radius,1000,function(b) if has(a.id,'draw_power') then buff(a,b,'heavy',5) elseif has(a.id,'draw_fortitude') then buff(a,b,'bind',3) end end) end)
action('aurora',function(a,b) heal(a,b,3000); regen(a,b,'aurora_regen',600,15) end)

-- Reaper: Soul Reaver and Enshroud use distinct server resources.
chain({'slice','waxing_slice','infernal_slice'},{1200,1400,1600},function(a) gain(a,'soul',10) end)
chain({'spinning_scythe','nightmare_scythe'},{800,1000},nil,true)
for _,key in ipairs({'spinning_scythe','nightmare_scythe'}) do local run=A[key].run; A[key].run=function(a,b) run(a,b); gain(a,'soul',10) end end
hit('harpe',1400)
action('soul_slice',function(a,b) damage(a,b,1600); gain(a,'soul',50) end,{cooldown='soul_charge'})
action('soul_scythe',function(a) area(a,a,D.soul_scythe.radius,1000,function() gain(a,'soul',20) end) end,{cooldown='soul_charge'})
for _,key in ipairs({'blood_stalk','grim_swathe','gluttony','unveiled_gibbet','unveiled_gallows'}) do action(key,function(a,b)
 gain(a,'soul',-50)
 if key=='gluttony' then area(a,b,D[key].radius,2000) elseif key=='grim_swathe' then shape(a,b,D[key].range,1000,true) else damage(a,b,1600) end
 buff(a,a,'soul_reaver',10); a.statuses.soul_reaver.stacks=key=='gluttony' and 2 or 1
end,{cooldown=key=='gluttony' and key or 'avatar',check=function(a)
 if has(a.id,'enshrouded') then return 'Unavailable during Enshroud.' end
 if key=='unveiled_gibbet' and not has(a.id,'enhanced_gibbet') or key=='unveiled_gallows' and not has(a.id,'enhanced_gallows') then return 'Enhanced action required.' end
 return require_resource('soul',50)(a)
end}) end
for _,key in ipairs({'hells_ingress','hells_egress'}) do action(key,function(a)
 p(a).hellgate={x=a.x,y=a.y,z=a.z}; step(a,key=='hells_ingress' and 15 or -15); buff(a,a,'harpe_instant',10); buff(a,a,'threshold',10)
end,{cooldown='hells_gate',check=bound}) end
action('regress',function(a) local point=p(a).hellgate; move(a,point.x,point.y,point.z); a.statuses.threshold=nil end,{target='self',check=function(a) return bound(a) or require_buff('threshold')(a) end})
action('arcane_crest',function(a) shield(a,a,'arcane_crest',2500,5) end)
action('enshroud',function(a) gain(a,'shroud',-50); buff(a,a,'enshrouded',15); p(a).lemure=5; p(a).void_shroud=0 end,{check=require_resource('shroud',50)})
for _,key in ipairs({'gibbet','gallows','guillotine'}) do action(key,function(a,b)
 if key=='guillotine' then shape(a,b,D[key].range,1200,true) else
  damage(a,b,has(a.id,'enhanced_'..key) and 2200 or 1800); a.statuses['enhanced_'..key]=nil; buff(a,a,key=='gibbet' and 'enhanced_gallows' or 'enhanced_gibbet',30)
 end
 gain(a,'shroud',20); local s=a.statuses.soul_reaver; s.stacks=s.stacks-1; if s.stacks<=0 then a.statuses.soul_reaver=nil end
end,{check=function(a) if key=='gallows' and not has(a.id,'enhanced_gallows') then return 'Enhanced Gallows required.' end; return require_buff('soul_reaver')(a) end}) end
for _,key in ipairs({'void_reaping','cross_reaping','grim_reaping'}) do action(key,function(a,b)
 if key=='grim_reaping' then shape(a,b,D[key].range,1200,true) else
  damage(a,b,has(a.id,'enhanced_'..key) and 2200 or 1800); a.statuses['enhanced_'..key]=nil; buff(a,a,key=='void_reaping' and 'enhanced_cross_reaping' or 'enhanced_void_reaping',30)
 end
 gain(a,'lemure',-1,5); gain(a,'void_shroud',1,5); if p(a).lemure==0 then a.statuses.enshrouded=nil end
end,{check=function(a) if key=='cross_reaping' and not has(a.id,'enhanced_cross_reaping') then return 'Enhanced Cross Reaping required.' end; return require_buff('enshrouded')(a) or require_resource('lemure',1)(a) end}) end
action('communio',function(a,b) area(a,b,D.communio.radius,2400); a.statuses.enshrouded=nil; p(a).lemure=0; p(a).void_shroud=0 end,{check=function(a) return require_buff('enshrouded')(a) or require_resource('lemure',1)(a) end})
for _,key in ipairs({'lemures_slice','lemures_scythe'}) do action(key,function(a,b)
 gain(a,'void_shroud',-2,5); if key=='lemures_slice' then damage(a,b,1600) else shape(a,b,D[key].range,1000,true) end
end,{cooldown='avatar',check=function(a) return require_buff('enshrouded')(a) or require_resource('void_shroud',2)(a) end}) end

-- Sage: Kardia, Eukrasia, Addersgall and six-layer Haima.
local function kardia(a)
 local b=state(p(a).kardia); if valid(b) and I.friendly(a,b) then heal(a,b,1000) end
end
action('dosis_iii',function(a,b) damage(a,b,1200); kardia(a) end)
action('diagnosis',function(a,b) heal(a,b,2000) end)
action('prognosis',function(a) for _,b in ipairs(allies(a,a,D.prognosis.radius)) do heal(a,b,2000) end end)
action('eukrasia',function(a) buff(a,a,'eukrasia',3600) end)
action('phlegma_iii',function(a,b) damage(a,b,2400); kardia(a) end)
action('pneuma',function(a,b)
 -- Proximity curve is an explicit approximation pending packet evidence.
 shape(a,b,D.pneuma.range,0,false,function(t) damage(a,t,1500+1500*math.max(0,1-distance(a,t)/D.pneuma.range)) end)
 for _,t in ipairs(allies(a,a,20)) do heal(a,t,3000); buff(a,t,'pneuma',15) end; kardia(a)
end)
action('druochole',function(a,b) gain(a,'addersgall',-1,3); heal(a,b,3000) end,{check=require_resource('addersgall',1)})
action('ixochole',function(a) gain(a,'addersgall',-1,3); for _,b in ipairs(allies(a,a,D.ixochole.radius)) do heal(a,b,1500) end end,{check=require_resource('addersgall',1)})
action('haima',function(a,b) shield(a,b,'haima',1000,15); b.statuses.haima.stacks=5 end)
action('icarus',function(a,b) move(a,b.x,b.y,b.z) end,{target='any',check=bound})
action('kardia',function(a,b) p(a).kardia=b.id end)
action('eukrasian_dosis_iii',function(a,b) a.statuses.eukrasia=nil; damage(a,b,1200); buff(a,b,'eukrasian_dosis',15); kardia(a) end,{check=require_buff('eukrasia')})
for _,key in ipairs({'eukrasian_diagnosis','eukrasian_prognosis'}) do action(key,function(a,b)
 a.statuses.eukrasia=nil
 for _,t in ipairs(key=='eukrasian_diagnosis' and {b} or allies(a,a,D[key].radius)) do heal(a,t,2000); shield(a,t,'eukrasian_barrier',I.healing_potency(a,t,2000),10) end
end,{check=require_buff('eukrasia')}) end

-- Dancer: partner effects, feathers, Esprit and ordered step execution.
local function partner(a) local b=state(p(a).partner); return valid(b) and I.friendly(a,b) and b or nil end
local function partnerbuff(a,key,duration) buff(a,a,key,duration); local b=partner(a); if b then buff(a,b,key,duration) end end
for _,d in ipairs({{'cascade',1000,false,false},{'fountain',1200,false,true},{'windmill',600,true,false},{'bladeshower',800,true,true},{'reverse_cascade',1600,false,false},{'fountainfall',1800,false,true},{'rising_windmill',1000,true,false},{'bloodshower',1200,true,true}}) do
 local key,n,wide,second=table.unpack(d); local enhanced=key=='reverse_cascade' or key=='fountainfall' or key=='rising_windmill' or key=='bloodshower'
 action(key,function(a,b)
  if enhanced then a.statuses.en_avant=nil end
  if wide then area(a,a,D[key].radius,n,function() gain(a,'esprit',5) end) else damage(a,b,n); gain(a,'esprit',10) end
  if second then gain(a,'feathers',1,4); setcombo(a,nil) else setcombo(a,wide and 'windmill' or 'cascade') end
 end,{combo=true,check=function(a) if enhanced and not has(a.id,'en_avant') then return 'En Avant required.' end; if second and not combo(a,wide and 'windmill' or 'cascade') then return 'Previous combo action required.' end end})
end
action('saber_dance',function(a,b) gain(a,'esprit',-50); area(a,b,D.saber_dance.radius,2000); partnerbuff(a,'saber_dance',15) end,{check=require_resource('esprit',50)})
for _,key in ipairs({'fan_dance','fan_dance_ii'}) do action(key,function(a,b)
 gain(a,'feathers',-1,4); if key=='fan_dance' then damage(a,b,1000) else area(a,a,D[key].radius,600) end; buff(a,a,'flourishing_fan',15)
end,{check=require_resource('feathers',1)}) end
action('fan_dance_iii',function(a,b) a.statuses.flourishing_fan=nil; area(a,b,D.fan_dance_iii.radius,1200); partnerbuff(a,'fan_dance_iii',15) end,{check=require_buff('flourishing_fan')})
for _,key in ipairs({'standard_step','technical_step'}) do action(key,function(a)
 p(a).dance=key; p(a).steps=0; p(a).dance_until=M.now+10
 local sequence={'emboite','entrechat','jete','pirouette'}
 for i=4,2,-1 do local j=math.random(i); sequence[i],sequence[j]=sequence[j],sequence[i] end
 p(a).sequence=sequence
end) end
for _,key in ipairs({'emboite','entrechat','jete','pirouette'}) do action(key,function(a)
 local s=p(a); if s.sequence[s.steps+1]==key then s.steps=math.min(s.dance=='standard_step' and 2 or 4,s.steps+1) end
end,{check=function(a) if not p(a).dance then return 'Dance required.' end end}) end
for _,key in ipairs({'standard_finish','technical_finish'}) do action(key,function(a)
 local s=p(a); local steps=s.steps; local potency=300*(steps+1)
 for _,b in ipairs(enemies(a,a,D[key].radius)) do
  -- Archived guide gives 300% proximity scaling, but not its curve.
  damage(a,b,potency*(1+3*math.max(0,1-distance(a,b)/math.max(1,D[key].radius)))); gain(a,'esprit',5)
 end
 if steps>0 then if key=='standard_finish' then partnerbuff(a,'standard_finish',60) else partybuff(a,'technical_finish',15,D[key].radius) end end
 s.dance=nil
end,{check=function(a) if p(a).dance~=(key=='standard_finish' and 'standard_step' or 'technical_step') then return 'Corresponding dance required.' end end}) end
action('curing_waltz',function(a)
 for _,b in ipairs(allies(a,a,D.curing_waltz.radius)) do heal(a,b,2000) end
 local other=partner(a); if other then for _,b in ipairs(allies(a,other,D.curing_waltz.radius)) do heal(a,b,2000) end end
end)
action('closed_position',function(a,b) p(a).partner=b.id end,{target='other_ally'})
action('ending',function(a) p(a).partner=nil end)
action('en_avant',function(a) step(a,10); buff(a,a,'en_avant',5) end,{check=bound})

-- Shared hooks preserve existing WHM/BLM/SAM/PLD/WAR/DRG behavior.
action('head_graze',function(a,b) buff(a,b,'silence',2); emit('interrupt',a.id,b.id) end)
action('leg_graze',function(a,b) buff(a,b,'heavy',5) end,{additional=true})
action('foot_graze',function(a,b) buff(a,b,'bind',3) end,{additional=true})
action('concentrate',function(a) buff(a,a,'concentrate',5) end,{additional=true})
action('recuperate',function(a)
 heal(a,a,3000); a.statuses.heavy=nil; a.statuses.bind=nil; a.statuses.holmgang=nil; emit('remove_holmgang',a.id,a.id)
 for _,id in ipairs(a.native_statuses or {}) do if id==13 or id==14 or id==1344 or id==1345 then emit('cleanse',a.id,a.id,id) end end
end,{additional=true})
action('peloton',function(a) for _,b in ipairs(allies(a,a,20)) do buff(a,b,'bolt',10); emit('speed',a.id,b.id,150) end end,{additional=true})
action('terminal_velocity',function(a,b)
 shape(a,b,30,0,false,function(t)
  -- Adrenaline bypasses player damage multipliers, but invulnerability and
  -- barriers still apply. Use the existing raw-damage path shared by other LBs.
  M.raw_adrenaline_damage(a,t,6000); buff(a,t,'terminal_velocity',10)
 end)
end,{adrenaline=true})
for _,key in ipairs(ranged) do A[key].jobs={[31]=true,[38]=true} end
A.terminal_velocity.jobs={[31]=true,[38]=true}
local function aura(a,key)
 for _,b in ipairs(allies(a,a,30)) do if has(b.id,key) then return true end end
 return false
end
local outgoing=M.outgoing
function M.outgoing(id,healing)
 local f=outgoing(id,healing); local a=state(id); if not a then return f end
 if has(id,'combust') then f=f*.9 end
 if has(id,'dissipation') then f=f*1.25 end
 if has(id,'draw_power') then f=f*1.1 end
 if has(id,'junction_weak') then f=f*.9 end
 if not healing then
  for _,entry in ipairs({{'riddle_of_fire',1.1},{'brotherhood',1.05},{'darkside',1.1},{'divination',1.1},{'lord_of_crowns',1.2},{'saber_dance',1.1}}) do if has(id,entry[1]) then f=f*entry[2] end end
  if p(a) and (p(a).lightning_active or p(a).concentrate_active) then f=f*1.5 end
  if p(a) and p(a).riddle_active then f=f*1.1 end
  if aura(a,'wanderers_minuet') then f=f*1.05 end
  if has(id,'embolden') then f=f*(1+.02*math.ceil((a.statuses.embolden.expires-M.now)/3)) end
 end
 return f
end
function M.healing_received(a,b,amount)
 for _,entry in ipairs({{'natures_minne',1.1},{'dark_missionary',1.1},{'focalization',1.2},{'biolysis',.9},{'shadow_heal_down',.9},{'terminal_velocity',.75}}) do if has(b.id,entry[1]) then amount=amount*entry[2] end end
 return amount
end
local incoming=M.incoming
function M.incoming(a,b,n)
 for _,entry in ipairs({{'riddle_of_earth',.9},{'natures_minne',.9},{'tactician',.9},{'dark_missionary',.9},{'divination',.9},{'lady_of_crowns',.8},{'fan_dance_iii',.9},{'trick_attack',1.1},{'bioblaster',1.1},{'draw_fortitude',.9},{'junction_vulnerable',1.1},{'pneuma',.9}}) do if has(b.id,entry[1]) then n=n*entry[2] end end
 return incoming(a,b,math.floor(M.absorb_extra(a,b,n)))
end
function M.absorb_extra(a,b,n)
 -- Existing shields are absorbed in the older hook; only new barriers here.
 for _,key in ipairs({'riddle_of_earth','shukuchi','engagement','galvanize','aspected_barrier','celestial_barrier','blackest_night','brutal_shell','arcane_crest','eukrasian_barrier'}) do
  local s=b.statuses[key]
  if has(b.id,key) and s.shield then local absorbed=math.min(n,s.shield); n=n-absorbed; s.shield=s.shield-absorbed
   if s.shield<=0 then b.statuses[key]=nil; if key=='blackest_night' then local caster=state(s.source); if valid(caster) then buff(caster,caster,'dark_arts',30) end end
    if key=='arcane_crest' then for _,t in ipairs(allies(b,b,15)) do buff(b,t,'crest_regen',15,500) end end
   end
  end
 end
 if has(b.id,'haima') then local s=b.statuses.haima
  while n>0 and s.shield>0 do local hit=math.min(n,s.shield); n=n-hit; s.shield=s.shield-hit
   if s.shield==0 then if s.stacks>0 then s.stacks=s.stacks-1; s.shield=1000 else b.statuses.haima=nil; break end end
  end
 end
 return n
end
local afterdamage=M.after_damage
function M.after_damage(a,b,n)
 afterdamage(a,b,n)
 if n>0 then
  b.statuses.kakuremi=nil; b.statuses.hidden=nil; if p(b) then p(b).kakuremi_until=nil end
  local s=b.statuses['wildfire_'..a.id]; if s and s.expires>M.now then s.stored=s.stored+n*.33 end
  if p(b) then p(b).contributions=p(b).contributions or {}; p(b).contributions[a.id]=M.now end
  if has(a.id,'bloodbath') and (a.job==20 or a.job==30 or a.job==39) then heal(a,a,n,true) end
 end
 if b.hp>0 and b.hp<b.max_hp*.5 and has(b.id,'excogitation') then
  local s=b.statuses.excogitation; b.statuses.excogitation=nil; local caster=state(s.source); if valid(caster) then heal(caster,b,4000) end
 end
 if b.hp<=0 and p(b) then for id,time in pairs(p(b).contributions or {}) do local caster=state(id)
  if valid(caster) and p(caster) and M.now-time<=10 then if caster.job==30 then reduce(caster,'assassinate',1000) elseif caster.job==32 then reduce(caster,'plunge',1000) end end
 end; p(b).contributions={} end
end
local effective=M.effective
function M.effective(a,d)
 local r=effective(a,d); if not a or not p(a) then return r end
 if d.spell and (has(a.id,'dualcast') or has(a.id,'abridged')) then r.cast=0 end
 if d.name=='harpe' and has(a.id,'harpe_instant') then r.cast=0 end
 if (d.spell or d.weapon) and has(a.id,'eukrasian_dosis') then r.cast=r.cast*1.1; r.recast=r.recast*1.1 end
 if d.spell and has(a.id,'neutral_sect') then r.cast=r.cast*.9; r.recast=r.recast*.9 end
 if d.weapon and has(a.id,'huton') then r.recast=r.recast*.85 end
 if d.weapon or d.spell then
  if aura(a,'armys_paeon') or has(a.id,'standard_finish') or has(a.id,'technical_finish') then r.cast=r.cast*.95; r.recast=r.recast*.95 end
  if d.weapon and has(a.id,'armys_paeon') then r.cast=r.cast*(1-.05*p(a).repertoire); r.recast=r.recast*(1-.05*p(a).repertoire) end
 end
 if (d.name=='edge_of_shadow' or d.name=='flood_of_shadow') and has(a.id,'dark_arts') then r.mp=0 end
 -- Draw resolves to the held card without reserving the same cooldown twice.
 -- Minor Arcana's draw is 1s; playing the card reserves the shared 30s timer.
 local check=r.check
 r.check=function(c,b)
  if b and has(b.id,'hidden') and not I.friendly(c,b) then return 'Target is hidden.' end
  if p(c).dance and not d.additional and not d.adrenaline and d.name~='bolt' and d.name~='kit' and d.name~='en_avant' and d.name~='head_graze' and not d.name:find('finish') and not ({emboite=true,entrechat=true,jete=true,pirouette=true})[d.name] then return 'Only dance and support actions are available while dancing.' end
  if check then return check(c,b) end
 end
 return r
end
local resolve=M.resolve
local chains={bootshine={'bootshine','true_strike','snap_punch'},four_point_fury={'four_point_fury','rockbreaker'},spinning_edge={'spinning_edge','gust_slash','aeolian_edge'},death_blossom={'death_blossom','hakke_mujinsatsu'},heated_split_shot={'heated_split_shot','heated_slug_shot','heated_clean_shot'},enchanted_riposte={'enchanted_riposte','enchanted_zwerchhau','enchanted_redoublement'},hard_slash={'hard_slash','syphon_strike','souleater'},unleash={'unleash','stalwart_soul'},keen_edge={'keen_edge','brutal_shell','solid_barrel'},demon_slice={'demon_slice','demon_slaughter'},gnashing_fang={'gnashing_fang','savage_claw','wicked_talon'},slice={'slice','waxing_slice','infernal_slice'},spinning_scythe={'spinning_scythe','nightmare_scythe'}}
function M.resolve(a,key)
 if a.job==37 and key=='continuation' then for _,d in ipairs({{'ready_rip','jugular_rip'},{'ready_tear','abdomen_tear'},{'ready_gouge','eye_gouge'}}) do if has(a.id,d[1]) then return d[2] end end end
 if a.job==40 and has(a.id,'eukrasia') then local transformed=({dosis_iii='eukrasian_dosis_iii',diagnosis='eukrasian_diagnosis',prognosis='eukrasian_prognosis'})[key]; if transformed then return transformed end end
 if a.job==39 then
  if key=='hells_ingress' and has(a.id,'threshold') then return 'regress' end
  if has(a.id,'enshrouded') then local replacement=({slice=has(a.id,'enhanced_cross_reaping') and 'cross_reaping' or 'void_reaping',spinning_scythe='grim_reaping',blood_stalk='lemures_slice',grim_swathe='lemures_scythe',harpe='communio'})[key]; if replacement then return replacement end end
  if has(a.id,'soul_reaver') then if key=='slice' then return has(a.id,'enhanced_gallows') and 'gallows' or 'gibbet' elseif key=='spinning_scythe' then return 'guillotine' end end
  if key=='blood_stalk' then if has(a.id,'enhanced_gibbet') then return 'unveiled_gibbet' elseif has(a.id,'enhanced_gallows') then return 'unveiled_gallows' end end
 end
 if a.job==30 then
  if has(a.id,'three_mudra') and key=='spinning_edge' then return 'hyosho_ranryu' end
  if has(a.id,'three_mudra') and key=='death_blossom' then return 'goka_mekkyaku' end
  if key=='kakuremi' and has(a.id,'hidden') then return 'trick_attack' end
 end
 if a.job==31 and has(a.id,'overheated') then if key=='heated_split_shot' then return 'heat_blast' elseif key=='spread_shot' then return 'auto_crossbow' end end
 if a.job==27 then
  if key=='dreadwyrm_trance' and p(a).firebird_next and not has(a.id,'dreadwyrm_trance') then return 'firebird_trance' end
  if has(a.id,'firebird_trance') then if key=='ruin_iii' then return 'fountain_of_fire' elseif key=='outburst' then return 'brand_of_purgatory' end end
 end
 if a.job==35 then
  if (key=='verstone' or key=='verfire') and has(a.id,'scorch_ready') then return 'scorch' end
  if has(a.id,'verfinisher') then if key=='verstone' then return 'verholy' elseif key=='verfire' then return 'verflare' end end
  if has(a.id,'dualcast') then if key=='verstone' then return 'veraero' elseif key=='verfire' then return 'verthunder' elseif key=='scatter' then return 'impact' end end
 end
 if a.job==33 then
  if key=='minor_arcana' and p(a).card then return p(a).card end
  if has(a.id,'neutral_sect') then if key=='benefic' then return 'aspected_benefic' elseif key=='helios' then return 'aspected_helios' end end
 end
 if a.job==38 then
  if p(a).dance then
   if key==p(a).dance then return key=='standard_step' and 'standard_finish' or 'technical_finish' end
   -- The four buttons remain fixed step buttons. The server's randomized
   -- sequence is also sent to the native step gauge and !feast job.
   local stepkey=({cascade='emboite',windmill='entrechat',fan_dance='jete',fan_dance_ii='pirouette'})[key]; if stepkey then return stepkey end
  end
  if key=='closed_position' and p(a).partner then return 'ending' end
  if key=='cascade' then return combo(a,'cascade') and (has(a.id,'en_avant') and 'fountainfall' or 'fountain') or has(a.id,'en_avant') and 'reverse_cascade' or key end
  if key=='windmill' then return combo(a,'windmill') and (has(a.id,'en_avant') and 'bloodshower' or 'bladeshower') or has(a.id,'en_avant') and 'rising_windmill' or key end
 end
 for base,sequence in pairs(chains) do if key==base then for i=1,#sequence-1 do if combo(a,sequence[i]) then return sequence[i+1] end end end end
 return resolve(a,key)
end
local before=M.before_action
function M.before_action(a,d)
 before(a,d); local s=p(a); s.cast_executed=d.cast>0; s.last_target=s.pending_target
 if d.weapon and has(a.id,'riddle_weapon') then s.riddle_active=true end
 if d.weapon and has(a.id,'lightning_bonus') then s.lightning_active=true; a.statuses.lightning_bonus=nil end
 if d.weapon and has(a.id,'concentrate') then s.concentrate_active=true; a.statuses.concentrate=nil end
 if d.spell then if has(a.id,'dualcast') then a.statuses.dualcast=nil end; if has(a.id,'abridged') then a.statuses.abridged=nil end end
 if d.name~='shukuchi' and d.name~='bolt' and d.name~='kakuremi' and d.name~='trick_attack' then a.statuses.kakuremi=nil; a.statuses.hidden=nil; s.kakuremi_until=nil end
end
local after=M.after_action
function M.after_action(a,d)
 after(a,d); local s=p(a)
 if s.riddle_active then s.riddle_active=false; a.statuses.riddle_weapon=nil end
 s.lightning_active=false; s.concentrate_active=false
 if d.name=='harpe' then a.statuses.harpe_instant=nil end
 if d.spell and s.cast_executed then if a.job==35 then buff(a,a,'dualcast',10) elseif a.job==33 and (d.name=='benefic' or d.name=='aspected_benefic' or d.name=='malefic_iv') then buff(a,a,'abridged',10) end end
 if (d.spell or d.weapon) and has(a.id,'brotherhood') then local source=state(a.statuses.brotherhood.source); if valid(source) and p(source) and math.random()<.5 then gain(source,'chakra',1,5) end end
end
local advance=M.advance
function M.advance(now)
 -- Resolve deferred effects before the base engine expires their status rows.
 M.now=now
 for _,a in pairs(M.actors) do if valid(a) then
  for key,s in pairs(a.statuses) do
   if s.expires<=now and key:find('^wildfire_') then local source=state(s.source); a.statuses[key]=nil; if valid(source) then damage(source,a,math.floor(s.stored)) end end
   if key=='excogitation' and (s.expires<=now or a.hp<a.max_hp*.5) then local source=state(s.source); a.statuses[key]=nil; if valid(source) then heal(source,a,4000) end end
   if key=='haima' and s.expires<=now then local source=state(s.source); a.statuses[key]=nil; if valid(source) then heal(source,a,1000*s.stacks) end end
  end
  local s=p(a)
  if s then
   if s.kakuremi_until and now>=s.kakuremi_until then local expires=s.kakuremi_until+10; s.kakuremi_until=nil; a.statuses.kakuremi=nil; if now<expires then buff(a,a,'hidden',expires-now) end end
   if s.dance and now>=s.dance_until then s.dance=nil end
   if a.job==40 and a.match_active~=false then
    s.gall_time=(s.gall_time or 0)+math.max(0,now-s.last_tick)
    while s.gall_time>=10 and s.addersgall<3 do s.gall_time=s.gall_time-10; gain(a,'addersgall',1,3) end
    if s.addersgall==3 then s.gall_time=0 end
   end
  end
 end end
 advance(now)
 sync_controls()
end
local command=M.command
function M.command(id,text,target,now)
 local result=command(id,text,target,now); sync_controls(); return result
end
local status=M.job_status
function M.job_status(a,s)
 if roles[a.job] then
  local values={}; for _,key in ipairs({'chakra','ninki','soul_voice','repertoire','heat','esprit','feathers','aetherflow','black_mana','white_mana','faerie','dark_blood','cartridges','soul','shroud','lemure','addersgall','addersting'}) do if (s[key] or 0)>0 then table.insert(values,key..' '..s[key]) end end
  if s.dance then table.insert(values,'Next step: '..(s.sequence[s.steps+1] or 'finish')) end
  return 'Feast job resources: '..table.concat(values,', ')
 end
 return status(a,s)
end
local gauges=M.job_gauge
function M.job_gauge(id)
 local a=state(id); if a and p(a) and roles[a.job] then
  local s=p(a)
  local function timer(key) return math.floor(math.max(0,has(id,key) and a.statuses[key].expires-M.now or 0)*1000) end
  if a.job==23 then return {23,s.soul_voice,s.repertoire,0,has(id,'wanderers_minuet') and 1 or has(id,'armys_paeon') and 2 or 0,0,math.max(timer('wanderers_minuet'),timer('armys_paeon'))} end
  if a.job==31 then return {31,s.heat,0,0,has(id,'overheated') and 1 or 0,0,timer('overheated')} end
  if a.job==35 then return {35,s.black_mana,s.white_mana,0,0,0,0} end
  if a.job==33 then return {33,s.card=='lord_of_crowns' and 7 or s.card=='lady_of_crowns' and 8 or 0,0,0,0,0,0} end
  if a.job==32 then return {32,s.dark_blood,0,0,has(id,'dark_arts') and 1 or 0,0,timer('darkside')} end
  if a.job==39 then return {39,s.soul,s.shroud,has(id,'enshrouded') and s.lemure or 0,has(id,'enshrouded') and s.void_shroud or 0,0,timer('enshrouded')} end
  if a.job==40 then return {40,s.addersgall,0,0,has(id,'eukrasia') and 1 or 0,0,math.floor(s.gall_time*1000)} end
  if a.job==27 then return {27,s.aetherflow,0,0,has(id,'firebird_trance') and 2 or has(id,'dreadwyrm_trance') and 1 or 0,0,math.max(timer('firebird_trance'),timer('dreadwyrm_trance'))} end
  if a.job==38 then
   local packed=0; if s.dance then for i=1,(s.dance=='standard_step' and 2 or 4) do packed=packed+({emboite=1,entrechat=2,jete=3,pirouette=4})[s.sequence[i]]*16^(i-1) end end
   return {38,s.esprit,s.feathers,s.dance and s.steps or 0,s.dance=='standard_step' and 1 or s.dance=='technical_step' and 2 or 0,0,packed}
  end
  local key=({[20]='chakra',[30]='ninki',[23]='soul_voice',[31]='heat',[38]='esprit',[27]='aetherflow',[35]='black_mana',[28]='faerie',[32]='dark_blood',[37]='cartridges',[39]='soul',[40]='addersgall'})[a.job]
  return {a.job,key and p(a)[key] or 0,0,0,0,0,0}
 end
 return gauges(id)
end
local ground=M.ground_command
function M.ground_command(id,key,x,y,z,now)
 if not D[key] or D[key].target~='ground' then return ground(id,key,x,y,z,now) end
 local a=state(id); if not a or not p(a) or x~=x or y~=y or z~=z or math.abs(x)>1000 or math.abs(y)>1000 or math.abs(z)>1000 then return 'Invalid ground position.' end
 p(a).ground={id=id,x=x,y=y,z=z,hp=1,max_hp=1,mp=0,statuses={}}; local result=M.command(id,key,id,now); p(a).ground=nil; return result
end
-- A missing implementation is a startup error, never a silent empty skill.
for key,d in pairs(D) do
 local def=assert(A[key],'Missing historical action: '..key)
 if d.job==30 and def.weapon then local run=def.run; def.run=function(a,b)
  run(a,b)
  if has(a.id,'bunshin') then local s=a.statuses.bunshin; s.stacks=s.stacks-1
   if d.radius>0 then area(a,a,d.radius,600) else damage(a,b,d.range>5 and 800 or 1000) end
   gain(a,'ninki',10); if s.stacks==0 then a.statuses.bunshin=nil end
  end
 end end
end
local bot=M.bot_action
function M.bot_action(id,target,heal_target)
 local a=state(id); if not a or not roles[a.job] then return bot(id,target,heal_target) end
 local basic=({[20]='bootshine',[30]='spinning_edge',[23]='burst_shot',[31]='heated_split_shot',[38]='cascade',[27]='ruin_iii',[35]='verstone',[28]='broil_iii',[33]='malefic_iv',[32]='hard_slash',[37]='keen_edge',[39]='slice',[40]='dosis_iii'})[a.job]
 if heal_target~=0 then local cure=({[28]='physick',[33]='benefic',[40]='diagnosis'})[a.job]; if cure then basic=cure; target=heal_target end end
 local key=M.resolve(a,basic); return key,target,math.floor(M.effective(a,A[key]).cast*1000)
end
end
