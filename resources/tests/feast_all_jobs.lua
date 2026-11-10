local checks=0
local function eq(a,b,message) checks=checks+1; assert(a==b,(message or '')..': '..tostring(a)..' ~= '..tostring(b)) end
local function near(a,b) checks=checks+1; assert(math.abs(a-b)<.001,tostring(a)..' ~= '..tostring(b)) end
local function setup(job)
 local m=make_engine(); install_jobs(m)
 local function actor(id,j,team,x) return {id=id,job=j,player=true,hp=50000,max_hp=50000,mp=10000,x=x,y=0,z=0,match_team=team,match_active=true} end
 m.sync({actor(1,job,'fang',0),actor(2,24,'claw',3),actor(3,24,'fang',1),actor(4,24,'claw',4)},0)
 for id=1,4 do m.command(id,'on',0,0) end
 return m,m.actors[1],m.actors[2],m.players[1],m.actors[3]
end
local function use(m,key,target,now)
 local result=m.command(1,key,target or 2,now or m.now)
 checks=checks+1; assert(result=='' or result:find('^Casting Feast'),key..': '..result)
 if m.players[1].pending then m.advance(m.players[1].pending.ends) end
end
for _,job in ipairs({19,20,21,22,23,24,25,27,28,30,31,32,33,34,35,37,38,39,40}) do
 local m=setup(job); eq(m.enabled(1),true); eq(#m.party_actions(1,0),3)
 if job~=24 then checks=checks+1; assert(#m.hotbar(1)>8 and #m.hotbar(1)<=64) end
 eq(m.job_gauge(1)[1],job)
 for _,slot in ipairs(m.hotbar(1)) do checks=checks+1; assert(slot[1]>0 and slot[2]>0) end
end
for _,job in ipairs({36,41,42}) do
 local m=make_engine(); install_jobs(m); m.sync({{id=1,job=job,player=true,hp=10000,max_hp=10000,mp=10000,x=0,y=0,z=0}},0)
 eq(m.command(1,'on',0,0),'This Feast job is not implemented.')
end
-- Monk: combo/chakra spend and next-weaponskill buff consumed once.
local m,a,b,p,c=setup(20)
for _,t in ipairs({0,2,4,6,8}) do use(m,'bootshine',2,t) end
eq(p.chakra,5); eq(b.hp,44200)
use(m,'the_forbidden_chakra',2,8.1); eq(p.chakra,0); eq(b.hp,42200)
eq(m.command(1,'the_forbidden_chakra',2,9.2),'chakra required.')
use(m,'riddle_of_fire',1,10); use(m,'bootshine',2,10); eq(b.hp,40506)
use(m,'bootshine',2,12); eq(b.hp,39406)
-- Ninja: mudra substitution/consumption, Huton, clone attacks and stealth break.
m,a,b,p,c=setup(30)
use(m,'three_mudra',1,0); eq(m.resolve(a,'spinning_edge'),'hyosho_ranryu')
use(m,'spinning_edge',2,0); eq(p.ninki,20); eq(b.hp,48000); eq(m.internal.has(1,'three_mudra'),nil)
near(m.effective(a,m.actions.spinning_edge).recast,2.04)
use(m,'bunshin',1,1); use(m,'spinning_edge',2,2.4); eq(p.ninki,40); eq(b.hp,46000)
use(m,'kakuremi',1,3); m.advance(6); eq(m.internal.has(1,'hidden'),true)
eq(m.command(2,'glare',1,6),'Target is hidden.')
use(m,'kakuremi',2,6); eq(m.internal.has(1,'hidden'),nil); eq(m.internal.has(2,'trick_attack'),true)
-- Bard: song, Repertoire and Soul Voice costs.
m,a,b,p,c=setup(23)
use(m,'the_wanderers_minuet',2,0); use(m,'burst_shot',2,0); eq(p.repertoire,1); eq(p.soul_voice,10)
use(m,'empyreal_arrow',2,.1); eq(p.repertoire,2); use(m,'pitch_perfect',2,.2); eq(p.repertoire,0)
p.soul_voice=100; local before=b.hp; use(m,'apex_arrow',2,2.4); eq(p.soul_voice,0); eq(before-b.hp,2520)
eq(m.party_actions(1,2.4)[1][1],4249)
eq(m.command(1,'terminal_velocity',2,3),'Adrenaline is not full.')
-- Machinist: shared heat actions, charge reduction and delayed Wildfire.
m,a,b,p,c=setup(31)
p.heat=50; use(m,'hypercharge',1,0); eq(p.heat,0); eq(m.resolve(a,'heated_split_shot'),'heat_blast')
use(m,'gauss_round',2,0); local deadline=p.cooldowns[m.actions.gauss_round.cooldown][1]
use(m,'heated_split_shot',2,0); near(p.cooldowns[m.actions.gauss_round.cooldown][1],deadline-7.5)
use(m,'wildfire',2,1); use(m,'gauss_round',2,1); before=b.hp; m.advance(7); eq(before-b.hp,396)
-- Summoner: owned DoT, spread, shared Aetherflow and trance transformations.
m,a,b,p,c=setup(27)
use(m,'bio_iii',2,0); eq(a.mp,9000); m.advance(3); eq(b.hp,49400)
use(m,'energy_drain',2,3); eq(p.aetherflow,1)
use(m,'bane',2,3.1); eq(p.aetherflow,0); eq(m.internal.has(4,'bio_1'),true)
p.aetherflow=2; use(m,'dreadwyrm_trance',1,4); eq(p.aetherflow,0)
m.advance(19); eq(m.resolve(a,'dreadwyrm_trance'),'firebird_trance')
p.aetherflow=2; use(m,'dreadwyrm_trance',1,34); eq(m.resolve(a,'ruin_iii'),'fountain_of_fire')
-- Red Mage: real cast -> Dualcast -> instant; complete melee finisher chain.
m,a,b,p,c=setup(35)
use(m,'verstone',2,0); eq(p.white_mana,10); eq(m.resolve(a,'verfire'),'verthunder')
use(m,'verfire',2,2.4); eq(p.black_mana,15); eq(m.internal.has(1,'dualcast'),nil)
p.black_mana=50; p.white_mana=50
use(m,'enchanted_riposte',2,4.8); eq(p.black_mana,0); eq(p.white_mana,0)
use(m,'enchanted_riposte',2,7.2); use(m,'enchanted_riposte',2,9.6)
eq(m.resolve(a,'verstone'),'verholy'); use(m,'verstone',2,12); eq(m.resolve(a,'verfire'),'scorch')
use(m,'verfire',2,14.4); eq(p.white_mana,30); eq(p.black_mana,10)
-- Scholar: barriers, shared heal charges, Excog at threshold and at expiry.
m,a,b,p,c=setup(28)
a.combat=true; c.hp=30000; use(m,'adloquium',3,0); eq(p.faerie,10); eq(c.hp,32000)
m.internal.damage(b,c,3000); eq(c.hp,31000)
use(m,'lustrate',3,2); use(m,'indomitability',1,2); use(m,'lustrate',3,2)
eq(m.command(1,'indomitability',1,2),'Action is on cooldown.')
use(m,'excogitation',3,3); c.hp=26000; m.internal.damage(b,c,2000); eq(c.hp,28000); eq(c.statuses.excogitation,nil)
use(m,'excogitation',3,4); m.advance(19); eq(c.hp,32000)
-- AST: Abridged, sect conversions and cards share one cooldown.
m,a,b,p,c=setup(33)
use(m,'benefic',3,0); eq(m.internal.has(1,'abridged'),true)
use(m,'benefic_ii',3,2.4); eq(m.internal.has(1,'abridged'),nil)
use(m,'neutral_sect',1,3); eq(m.resolve(a,'benefic'),'aspected_benefic')
use(m,'minor_arcana',1,3); local card=p.card; eq(m.resolve(a,'minor_arcana'),card)
use(m,'minor_arcana',3,4); eq(p.card,nil); eq(m.command(1,'minor_arcana',1,5),'Action is on cooldown.')
-- DRK: resource combos, Dark Arts only on barrier break, Delirium free spend.
m,a,b,p,c=setup(32)
use(m,'hard_slash',2,0); use(m,'hard_slash',2,2.4); use(m,'hard_slash',2,4.8); eq(p.dark_blood,25)
use(m,'the_blackest_night',3,5); m.internal.damage(b,c,3000); eq(m.internal.has(1,'dark_arts'),true)
use(m,'edge_of_shadow',2,5); eq(a.mp,10000); eq(m.internal.has(1,'dark_arts'),nil)
use(m,'delirium',1,6); use(m,'bloodspiller',2,7.2); eq(p.dark_blood,25)
-- GNB: cartridges and continuation charges never bypass combo checks.
m,a,b,p,c=setup(37)
use(m,'keen_edge',2,0); use(m,'keen_edge',2,2.4); use(m,'keen_edge',2,4.8); eq(p.cartridges,1)
use(m,'gnashing_fang',2,7.2); eq(p.cartridges,0); eq(m.resolve(a,'continuation'),'jugular_rip')
use(m,'continuation',2,7.3); eq(m.internal.has(1,'ready_rip'),nil)
use(m,'gnashing_fang',2,9.6); eq(m.resolve(a,'continuation'),'abdomen_tear')
use(m,'draw_and_junction',2,10); eq(m.internal.has(1,'draw_power'),true)
-- DNC: feathers, partner buffs, ordered steps and dance-only restrictions.
m,a,b,p,c=setup(38)
use(m,'closed_position',3,0); use(m,'cascade',2,0); use(m,'cascade',2,2.4); eq(p.feathers,1); eq(p.esprit,20)
use(m,'fan_dance',2,3); eq(p.feathers,0); use(m,'fan_dance_iii',2,3); eq(m.internal.has(3,'fan_dance_iii'),true)
use(m,'standard_step',1,5); eq(m.command(1,'saber_dance',2,5),'Only dance and support actions are available while dancing.')
local t=7.4
for i=1,2 do use(m,p.sequence[i],1,t); t=t+m.actions[p.sequence[i]].recast end
use(m,'standard_step',1,t); eq(p.dance,nil); eq(m.internal.has(3,'standard_finish'),true)
-- RPR: soul -> Soul Reaver -> shroud, Enshroud actions and gate return.
m,a,b,p,c=setup(39)
use(m,'soul_slice',2,0); eq(p.soul,50); use(m,'blood_stalk',2,.1); eq(p.soul,0)
eq(m.resolve(a,'slice'),'gibbet'); use(m,'slice',2,2.4); eq(p.shroud,20)
p.shroud=50; use(m,'enshroud',1,3); eq(p.lemure,5)
use(m,'slice',2,4.8); eq(p.lemure,4); eq(p.void_shroud,1); eq(m.resolve(a,'slice'),'cross_reaping')
use(m,'slice',2,7.3); use(m,'blood_stalk',2,7.4); eq(p.void_shroud,0)
use(m,'harpe',2,9.8); eq(p.lemure,0); eq(m.internal.has(1,'enshrouded'),nil)
use(m,'hells_ingress',1,12); near(a.z,15); use(m,'hells_ingress',1,12.1); near(a.z,0)
-- SGE: automatic Addersgall, Kardia, Eukrasia, layered Haima and expiration heal.
m,a,b,p,c=setup(40)
m.advance(10); eq(p.addersgall,1); c.hp=20000
use(m,'kardia',3,10); use(m,'dosis_iii',2,10); eq(c.hp,21000)
use(m,'druochole',3,12); eq(p.addersgall,0); eq(c.hp,24000)
use(m,'eukrasia',1,12.4); eq(m.resolve(a,'diagnosis'),'eukrasian_diagnosis')
use(m,'diagnosis',3,14.8); eq(c.hp,26000); eq(m.internal.has(1,'eukrasia'),nil)
use(m,'haima',3,15); m.internal.damage(b,c,4500); eq(c.hp,26000); eq(c.statuses.haima.stacks,3); eq(c.statuses.haima.shield,500)
m.advance(30); eq(c.hp,29000); eq(c.statuses.haima,nil)
-- Native controls follow authoritative application/expiry without refreshing
-- every frame. A cleared native sleep snapshot must not re-block an action.
m,a,b,p,c=setup(23); m.drain()
use(m,'head_graze',2,0)
local count=0; for _,e in ipairs(m.drain()) do if e.kind=='status_visual' and e.message=='1347' then count=count+1; eq(e.amount,2000) end end; eq(count,1)
m.advance(1); count=0; for _,e in ipairs(m.drain()) do if e.kind=='status_visual' and e.message=='1347' then count=count+1 end end; eq(count,0)
m.advance(2); count=0; for _,e in ipairs(m.drain()) do if e.kind=='status_visual' and e.message=='1347' then count=count+1; eq(e.amount,0) end end; eq(count,1)
m.actors[2].native_statuses={1347}; use(m,'burst_shot',2,2.1)
eq(m.command(2,'glare',1,2.1):find('Casting Feast')~=nil,true)
-- Ranged adrenaline bypasses player damage buffs, respects barriers, consumes
-- its gauge once, and applies the archived recovery penalty.
m,a,b,p,c=setup(23); p.adrenaline=100; m.internal.buff(a,a,'lord_of_crowns',15)
m.internal.buff(b,b,'galvanize',10); b.statuses.galvanize.shield=2000
use(m,'terminal_velocity',2,0); eq(b.hp,46000); eq(p.adrenaline,0); eq(m.internal.has(2,'terminal_velocity'),true)
local oldhp=b.hp; m.internal.heal(b,b,1000); eq(b.hp-oldhp,750)
-- Interrupted cast spends neither mana nor resource and refunds provisional GCD.
m,a,b,p,c=setup(35); m.command(1,'verstone',2,0); m.internal.buff(b,a,'stun',1); m.advance(.2)
eq(p.pending,nil); eq(p.white_mana,0); eq(p.gcd,0)
-- Passive resource gauges cap and do not bank extra stacks.
m,a,b,p,c=setup(40); m.advance(50); eq(p.addersgall,3); eq(p.gall_time,0)
use(m,'druochole',3,50); m.advance(59.9); eq(p.addersgall,2); m.advance(60); eq(p.addersgall,3)
-- Shukuchi uses the native ground bridge and two sequential recharge slots.
m,a,b,p,c=setup(30)
eq(m.ground_command(1,'shukuchi',10,0,0,0),''); eq(a.x,10)
local elapsed,total=m.recast(1,'shukuchi',0); eq(elapsed,3000); eq(total,6000)
eq(m.ground_command(1,'shukuchi',0,0,0,0),''); eq(a.x,0)
elapsed,total=m.recast(1,'shukuchi',0); eq(elapsed,0); eq(total,6000)
eq(m.ground_command(1,'shukuchi',10,0,0,0),'Action is on cooldown.')
return checks

