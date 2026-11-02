-- Feast rules from the official job guide archived 2022-04-09 (6.08).
-- Isolated from PvE/Crystalline Conflict. Time is supplied by the server in seconds.
-- Potencies are fixed prototype HP amounts, not the PvE stat-based damage formula.
local M = { actors = {}, players = {}, events = {}, now = 0 }
local SLIDECAST_WINDOW = 0.500
-- The client starts recasts on input and the server on receipt, so a queued
-- action can arrive slightly early when its latency is lower than the last.
-- New recasts chain from the previous deadline, so this never shortens them.
local RECAST_TOLERANCE = 0.200
local actions = {}
M.actions = actions
local cleanse_names = { "stun", "sleep", "bind", "heavy", "silence" }
local native_cc = { [2]="stun", [3]="sleep", [13]="bind", [14]="heavy", [7]="silence",
    [1343]="stun", [1344]="heavy", [1345]="bind", [1347]="silence", [1348]="sleep" }

local function emit(kind, actor, target, amount, message)
    table.insert(M.events, { kind=kind, actor=actor, target=target or actor,
        amount=amount or 0, message=message or "" })
end
local function distance(a,b)
    return math.sqrt((a.x-b.x)^2+(a.y-b.y)^2+(a.z-b.z)^2)
end
local function state(id)
    local a=M.actors[id]
    if a and not a.statuses then a.statuses={} end
    return a
end
local function has(id, name)
    local a=state(id)
    return a and a.statuses[name] and a.statuses[name].expires > M.now
end
local function friendly(a,b)
    if a.match_team and b.match_team then return a.match_team==b.match_team end
    return a.id == b.id or (M.players[a.id] and M.players[b.id]
        and M.players[a.id].team == M.players[b.id].team)
end
local function valid(a) return a and a.hp > 0 end
local function allies(source,center,radius)
    local result={}
    for _,a in pairs(M.actors) do
        if valid(a) and friendly(source,a) and distance(center,a)<=radius then
            table.insert(result,a)
        end
    end
    table.sort(result,function(a,b) return a.id<b.id end)
    return result
end
local function enemies(source,center,radius)
    local result={}
    for _,a in pairs(M.actors) do
        if valid(a) and not friendly(source,a) and (not a.player or M.players[a.id] or a.match_team)
            and distance(center,a)<=radius then table.insert(result,a) end
    end
    table.sort(result,function(a,b) return a.id<b.id end)
    return result
end
local function buff(source,target,name,duration,potency)
    if M.immune and M.immune(target,name) then return end
    target.statuses[name]={source=source.id,expires=M.now+duration,
        potency=potency,next_tick=M.now+3}
    if name=="dia" then emit("dia_visual",source.id,target.id,duration) end
end
local function outgoing(id, healing)
    local scale=has(id,"temperance") and 1.2 or 1
    if M.outgoing then scale=scale*M.outgoing(id,healing) end
    if healing and has(id,"largesse") then scale=scale*1.25 end
    return scale
end
local function temperance_aura(target)
    for id,a in pairs(M.actors) do
        if valid(a) and has(id,"temperance") and friendly(a,target)
            and distance(a,target)<=30 then return true end
    end
    return false
end
function M.modify_damage(source,target,amount,now)
    M.now=now
    local b=state(target)
    local a=state(source)
    if (a and a.match_active==false) or (b and b.match_active==false) then return 0 end
    if a and b and a.match_team and b.match_team and friendly(a,b) then return 0 end
    if not b then return amount end
    if b.invulnerable then return 0 end
    local factor=outgoing(source,false)
    factor=factor*(1+0.1*math.min(5,b.match_culling or 0))
    if has(target,"dia") then factor=factor*1.1 end
    if has(target,"protect") then factor=factor*0.8 end
    if temperance_aura(b) then factor=factor*0.9 end
    if has(target,"attunement") then factor=factor*0.01 end
    amount=math.max(0,math.floor(amount*factor+0.000001))
    if M.incoming then amount=M.incoming(a,b,amount) end
    if amount>0 then b.statuses.sleep=nil end
    if M.players[source] then M.players[source].combat_until=now+10 end
    if M.players[target] then M.players[target].combat_until=now+10 end
    return amount
end
local function damage(a,b,amount)
    amount=M.modify_damage(a.id,b.id,amount,M.now)
    b.hp=math.max(0,b.hp-amount)
    emit("hp",a.id,b.id,b.hp)
    emit("damage",a.id,b.id,amount)
    if M.after_damage then M.after_damage(a,b,amount) end
    return amount
end
local function heal(a,b,amount,percent)
    if not valid(b) then return end -- healing does not resurrect
    if not percent then amount=amount*outgoing(a.id,true) end
    if M.healing_received then amount=M.healing_received(a,b,amount) end
    local before=b.hp
    b.hp=math.min(b.max_hp,b.hp+math.floor(amount+0.000001))
    emit("hp",a.id,b.id,b.hp)
    emit("heal",a.id,b.id,b.hp-before)
    if M.players[a.id] and M.players[b.id] and M.players[b.id].combat_until>M.now then
        M.players[a.id].combat_until=M.now+10
    end
end
local function cleanse(b)
    local removed=false
    for _,name in ipairs(cleanse_names) do
        if has(b.id,name) then b.statuses[name]=nil; removed=true end
    end
    for _,id in ipairs(b.native_statuses or {}) do
        if native_cc[id] then
            removed=true
            emit("cleanse",b.id,b.id,id)
        end
    end
    b.native_statuses={}
    return removed
end
local function missing_hp_heal(a) return 3000+3000*(1-a.hp/a.max_hp) end
local function add(name,id,target,cast,recast,mp,range,run,extra)
    local action={name=name,id=id,target=target,cast=cast,recast=recast,mp=mp,
        range=range,run=run,charges=1}
    for k,v in pairs(extra or {}) do action[k]=v end
    actions[name]=action
end
local function single_heal(potency)
    return function(a,b) heal(a,b,potency) end
end
local function area_heal(potency,regen)
    return function(a)
        for _,b in ipairs(allies(a,a,15)) do
            heal(a,b,potency)
            if regen then buff(a,b,"rapture_regen",15,regen*outgoing(a.id,true)) end
        end
    end
end
add("cure",8895,"ally",1.5,2.4,0,30,single_heal(2000),{gcd=true})
add("cure2",8896,"ally",1.5,2.4,1000,30,single_heal(4000),{gcd=true})
add("medica",18945,"self",2,2.4,1000,0,area_heal(2000),{gcd=true})
add("glare",17789,"enemy",1.5,2.4,0,25,function(a,b) damage(a,b,1200) end,{gcd=true})
add("dia",17790,"enemy",0,2.4,1000,25,function(a,b)
    damage(a,b,800); if valid(b) then buff(a,b,"dia",15) end
end,{gcd=true})
add("solace",17791,"ally",0,2.4,0,30,function(a,b)
    heal(a,b,4000); buff(a,b,"solace_regen",15,400*outgoing(a.id,true))
end,{gcd=true,lily=true})
add("rapture",18946,"self",0,2.4,0,0,area_heal(2000,200),{gcd=true,lily=true})
add("misery",17793,"enemy",0,2.4,0,25,function(a,b)
    for _,target in ipairs(enemies(a,b,5)) do damage(a,target,2400) end
end,{gcd=true,blood=true})
add("tetra",13975,"ally",0,15,0,30,single_heal(4000),{charges=2})
add("assize",9620,"self",0,30,0,0,function(a)
    for _,b in ipairs(enemies(a,a,15)) do damage(a,b,1000) end
    for _,b in ipairs(allies(a,a,15)) do heal(a,b,2000) end
end)
add("temperance",17794,"self",0,60,0,0,function(a)
    buff(a,a,"temperance",15)
    emit("temperance_visual",a.id,a.id,15)
end)
add("purify",1584,"ally",0,30,0,30,function(a,b)
    heal(a,b,cleanse(b) and 4000 or 2000)
end,{usable_while_cc=true})
add("bolt",18942,"self",0,30,0,0,function(a)
    buff(a,a,"bolt",10); emit("speed",a.id,a.id,150)
end)
add("kit",18943,"self",0,15,0,0,function(a) heal(a,a,missing_hp_heal(a)) end,{charges=3})
add("repose",17690,"enemy",0,30,0,25,function(a,b)
    buff(a,b,"sleep",3); emit("interrupt",a.id,b.id)
end,{additional=true})
add("protect",17832,"ally",0,45,0,30,function(a,b) buff(a,b,"protect",10) end,{additional=true})
add("rescue",17688,"other_ally",0,30,0,30,function(a,b)
    b.statuses.holmgang=nil
    -- Holmgang's native status ID is supplied separately by the adapter if present.
    emit("remove_holmgang",a.id,b.id)
    b.x,b.y,b.z=a.x,a.y,a.z
    table.insert(M.events,{kind="move",actor=a.id,target=b.id,x=a.x,y=a.y,z=a.z})
end,{additional=true})
add("largesse",18957,"self",0,45,0,0,function(a) buff(a,a,"largesse",10) end,{additional=true})
add("attunement",17689,"self",0,30,0,0,function(a)
    heal(a,a,missing_hp_heal(a)); buff(a,a,"attunement",3)
end,{additional=true})
add("rain",3362,"self",0,0,0,0,function(a)
    for _,b in ipairs(allies(a,a,30)) do cleanse(b); heal(a,b,b.max_hp*0.5,true) end
end,{adrenaline=true})

function M.enabled(id) return M.players[id]~=nil end
function M.blocked(id, movement, physical)
    local a=state(id)
    if not a then return false end
    if has(id,"attunement") or has(id,"stun") or has(id,"sleep") then return true end
    if movement then return has(id,"bind") or false end
    return (not physical and has(id,"silence")) or false
end
local function cancel_cast(id,name,prepared)
    local p=M.players[id]
    name=name or (p and p.pending and p.pending.name)
    if p and name and actions[name].gcd then
        -- Cast-start recast is provisional: interruption refunds it. Successful
        -- casts keep their original timing, without an extra post-cast GCD.
        p.gcd=0
        emit("reset_gcd",id,id)
    end
    local reserved=prepared or (p and p.pending and p.pending.definition)
    if p and reserved and reserved.cooldown_recast then
        local d=reserved; local times=p.cooldowns[d.cooldown or name]
        if times then table.remove(times) end
        emit("reset_action_cooldown",id,id,0,name)
    end
    if p then p.pending=nil end
    emit("cancel",id,id)
end
function M.sync(snapshots,now)
    M.now=now
    local present={}
    for _,snap in ipairs(snapshots) do
        local old=state(snap.id)
        if old and old.job~=snap.job and M.players[snap.id] then
            local p=M.players[snap.id]
            if p.pending then cancel_cast(snap.id) end
            p.gcd=0; p.cooldowns={}; old.statuses={}
            if M.initialize and (snap.job==24 or (M.jobs and M.jobs[snap.job])) then M.initialize(snap,p) end
        end
        snap.statuses=old and old.statuses or {}
        M.actors[snap.id]=snap; present[snap.id]=true
        if M.players[snap.id] and snap.match_team then M.players[snap.id].team=snap.match_team end
        if snap.match_active==false and snap.statuses.wolfs_heart then
            snap.statuses.wolfs_heart=nil; emit("wolfs_heart",snap.id,snap.id,0)
        end
        if snap.hp<=0 then
            if snap.statuses.temperance then emit("temperance_visual",snap.id,snap.id,0) end
            if snap.statuses.wolfs_heart then emit("wolfs_heart",snap.id,snap.id,0) end
            if snap.statuses.bolt then emit("speed",snap.id,snap.id,100) end
            snap.statuses={}
            local p=M.players[snap.id]
            if p then
                if p.pending then cancel_cast(snap.id) end
                if M.reset_job then M.reset_job(p) end
                p.pending=nil; p.lilies=0; p.blood=0; p.lily_time=0; p.combat_until=0
            end
        end
    end
    for id in pairs(M.actors) do
        if not present[id] then M.actors[id]=nil; M.players[id]=nil end
    end
end
local function definition(id,name)
    local d=actions[name]
    if d and M.effective then return M.effective(state(id),d) end
    return d
end
local function available(p,d)
    local times=p.cooldowns[d.cooldown or d.name] or {}
    local active,blocking={},0
    for _,time in ipairs(times) do
        if time>M.now then table.insert(active,time) end
        if time>M.now+RECAST_TOLERANCE then blocking=blocking+1 end
    end
    p.cooldowns[d.cooldown or d.name]=active
    return blocking<d.charges
end
local function error_for(id,name,target,finishing,cast_origin,prepared)
    local p=M.players[id]; local a=state(id); local d=prepared or definition(id,name); local b=state(target)
    if d and d.point then b=d.point end
    if not p then return "Use !whm on first." end
    if not d then return "Unknown Feast action; use !whm list." end
    if d.target=="ground" and not d.point then return "Select a point on the ground." end
    if not valid(a) then return "Cannot act while dead." end
    if d.job and d.job~=a.job and not (d.jobs and d.jobs[a.job]) then return "This action belongs to another job." end
    if M.allowed and not M.allowed(a,d) then return "This additional action belongs to another role." end
    if a.match_active==false then return "Feast combat is closed until the match begins." end
    if has(id,"attunement") then return "Attunement prevents actions." end
    if not d.usable_while_cc and M.blocked(id,false,d.weapon) then return "Crowd control prevents this action." end
    for _,status in ipairs(a.native_statuses or {}) do
        if not (M.owns_native_status and M.owns_native_status(id,status)) and not d.usable_while_cc and (native_cc[status]=="stun" or native_cc[status]=="sleep"
            or (native_cc[status]=="silence" and not d.weapon)) then return "Crowd control prevents this action." end
    end
    if not valid(b) then return "Select a living target in this arena." end
    if b.match_active==false then return "That player is not an active match participant." end
    if d.target=="enemy" and (friendly(a,b) or (b.player and not M.players[b.id] and not b.match_team)) then return "Select an enemy." end
    if (d.target=="ally" or d.target=="other_ally") and not friendly(a,b) then return "Select a teammate or yourself." end
    if d.target=="other_ally" and a.id==b.id then return "Rescue requires another teammate." end
    if distance(cast_origin or a,b)>d.range and d.target~="self" then return "Target is out of range." end
    if d.additional and not p.loadout[name] then return "Select this additional action with !whm loadout <first> <second>." end
    if a.mp<d.mp then return "Not enough MP." end
    if d.lily and p.lilies<1 then return "No lilies available." end
    if d.blood and p.blood<3 then return "The Blood Lily is not in full bloom." end
    if d.adrenaline and p.adrenaline<100 then return "Adrenaline is not full." end
    if d.check then local err=d.check(a,b,p); if err then return err end end
    if not finishing then
        if p.pending then return "Already casting." end
        if d.gcd and p.gcd>M.now+RECAST_TOLERANCE then return "Global cooldown is active." end
        if (not d.gcd or d.cooldown_recast) and not available(p,d) then return "Action is on cooldown." end
    end
end
local function complete(id,name,target,cast_origin,prepared)
    local p=M.players[id]; local a=state(id); local d=prepared or definition(id,name); local b=state(target)
    if d and d.point then b=d.point end
    local err=error_for(id,name,target,true,cast_origin,d)
    if err then cancel_cast(id,name,d); emit("notice",id,id,0,err); return end
    a.mp=a.mp-d.mp; if d.mp>0 then emit("mp",id,id,a.mp) end
    if d.lily then p.lilies=p.lilies-1; p.blood=math.min(3,p.blood+1) end
    if d.blood then p.blood=0 end
    if d.adrenaline then p.adrenaline=0 end
    local before=b.hp
    if M.before_action then M.before_action(a,d) end
    local first_effect=#M.events+1
    d.run(a,b)
    if M.after_action then M.after_action(a,d) end
    emit("animation",id,target,math.abs(b.hp-before),name)
    M.events[#M.events].healing=b.hp>before
    local animation=M.events[#M.events]; animation.hits={}
    for i=first_effect,#M.events-1 do
        local e=M.events[i]
        if (e.kind=="damage" or e.kind=="heal") and e.amount>0 then
            table.insert(animation.hits,{target=e.target,amount=e.amount,healing=e.kind=="heal"})
        end
    end
    if d.point then M.events[#M.events].x=d.point.x; M.events[#M.events].y=d.point.y; M.events[#M.events].z=d.point.z end
    emit("notice",id,id,0,"Feast "..name.." executed.")
end
-- Movement is recorded at packet processing time, independently of the next
-- simulation tick. An early move stays cancelled even if that tick is delayed.
function M.movement(id,now,x,y,z)
    local p=M.players[id]; local cast=p and p.pending
    if cast and now<cast.ends-SLIDECAST_WINDOW
        and distance({x=x,y=y,z=z},cast.origin)>0.1 then
        cast.moved_early=true
    end
end
function M.advance(now)
    M.now=now
    for id,a in pairs(M.actors) do
        for name,s in pairs(a.statuses) do
            if name=="wolfs_heart" and valid(a) then
                while s.next_tick<=now and s.next_tick<=s.expires do
                    a.mp=math.min(10000,a.mp+200)
                    emit("mp",id,id,a.mp)
                    s.next_tick=s.next_tick+1
                end
            end
            if s.potency and valid(a) then
                while s.next_tick<=now and s.next_tick<=s.expires do
                    -- Regen snapshots healing modifiers when applied.
                    local source=state(s.source) or a
                    if s.dot then damage(source,a,s.potency) else heal(source,a,s.potency,true) end
                    s.next_tick=s.next_tick+3
                end
            end
            if s.expires<=now then
                if name=="wolfs_heart" then emit("wolfs_heart",id,id,0) end
                if name=="temperance" then emit("temperance_visual",id,id,0) end
                if name=="bolt" then emit("speed",id,id,100) end
                if name=="dia" then emit("dia_visual",s.source,id,0) end
                a.statuses[name]=nil
            end
        end
    end
    for id,p in pairs(M.players) do
        local a=state(id)
        if valid(a) then
            local elapsed=math.max(0,now-p.last_tick)
            -- Three-minute passive fill; halted in preparation and after the match.
            if a.match_active==true then p.adrenaline=math.min(100,p.adrenaline+elapsed*100/180) end
            local fighting=a.combat or p.combat_until>p.last_tick
            if M.advance_job then M.advance_job(a,p,elapsed) end
            if fighting and a.job==24 and a.match_active~=false and p.lilies<3 then
                local active=a.combat and elapsed or math.max(0,math.min(now,p.combat_until)-p.last_tick)
                p.lily_time=p.lily_time+active
                while p.lily_time>=10 and p.lilies<3 do p.lilies=p.lilies+1; p.lily_time=p.lily_time-10 end
                -- A full gauge cannot bank a fourth lily for an immediate refill.
                if p.lilies==3 then p.lily_time=0 end
            end
            if p.pending then
                local cast=p.pending
                if M.blocked(id,false,cast.definition and cast.definition.weapon) or cast.moved_early
                    or (now<cast.ends-SLIDECAST_WINDOW and distance(a,cast.origin)>0.1) then
                    cancel_cast(id,cast.name); emit("notice",id,id,0,"Cast interrupted.")
                elseif now>=cast.ends then
                    p.pending=nil; complete(id,cast.name,cast.target,cast.origin,cast.definition)
                end
            end
        end
        p.last_tick=now
    end
end
-- Native recast details use elapsed/total in units of ten milliseconds.
function M.recast(id,name,now)
    local p=M.players[id]; local d=definition(id,name)
    if p and p.last_input==name then d=p.last_definition end
    if not p or not d then return 0,0 end
    local total=(d.cooldown_recast or d.recast)*d.charges
    local remaining
    if d.gcd and not d.cooldown_recast then remaining=math.max(0,p.gcd-now)
    else
        local times=p.cooldowns[d.cooldown or name] or {}
        remaining=math.max(0,(times[#times] or now)-now)
    end
    return math.floor(math.max(0,total-remaining)*100),math.floor(total*100)
end
-- Party HUD reads authoritative cooldown deadlines, never client action guesses.
function M.party_actions(id,now)
    local p=M.players[id]; local result={}
    if not p then return result end
    local job=state(id).job
    -- The first party slot is adrenaline, not a third additional action.
    local adrenaline=({[19]=3359,[21]=3359,[32]=3359,[37]=3359,[20]=3360,[22]=3360,[30]=3360,[34]=3360,[39]=3360,[23]=4249,[31]=4249,[38]=4249,[25]=3361,[27]=3361,[35]=3361,[24]=3362,[28]=3362,[33]=3362,[40]=3362})[job]
    if adrenaline then table.insert(result,{adrenaline,0}) end
    for _,name in ipairs(M.party_list and M.party_list(state(id).job) or {"purify","repose","protect","rescue","largesse","attunement"}) do
        if actions[name].additional and p.loadout[name] and #result<3 then
            local times=p.cooldowns[actions[name].cooldown or name] or {}
            table.insert(result,{actions[name].id,math.ceil(math.max(0,(times[1] or now)-now)*10)})
        end
    end
    return result
end
function M.drain()
    local result=M.events; M.events={}; return result
end
function M.command(id,text,target,now)
    M.advance(now)
    local args={}; for word in text:lower():gmatch("%S+") do table.insert(args,word) end
    local name=args[1] or "status"
    local a=state(id)
    if not a or not a.player then return "Player is not ready." end
    if name=="on" then
        if a.job~=24 and not (M.jobs and M.jobs[a.job]) then return "This Feast job is not implemented." end
        if M.players[id] then return "Feast White Mage is already enabled." end
        M.players[id]={team=a.match_team or "fang",cooldowns={},gcd=0,lilies=0,blood=0,adrenaline=0,
            lily_time=0,last_tick=now,combat_until=0,loadout={protect=true,largesse=true}}
        if M.initialize then M.initialize(a,M.players[id]) end
        return "Feast rules enabled (team "..M.players[id].team.."). Additional actions are chosen in the Wolves' Den (!feast den)."
    end
    if name=="off" then
        if has(id,"temperance") then emit("temperance_visual",id,id,0) end
        cancel_cast(id)
        if has(id,"bolt") then emit("speed",id,id,100) end
        M.players[id]=nil; a.statuses={}; return "Feast White Mage disabled."
    end
    if name=="list" then
        local names={}; for key,d in pairs(actions) do if not d.job or d.job==a.job or (d.jobs and d.jobs[a.job]) then table.insert(names,key) end end
        table.sort(names); return table.concat(names," ")
    end
    local p=M.players[id]
    if not p then return "Use !whm on first." end
    if name=="status" then
        if M.job_status then local status=M.job_status(a,p); if status then return status end end
        local statuses={}; for status,s in pairs(a.statuses) do
            if s.expires>now then table.insert(statuses,status) end
        end
        table.sort(statuses)
        return string.format("Feast WHM: team=%s lilies=%d blood=%d/3 adrenaline=%d/100 MP=%d; %s",p.team,p.lilies,p.blood,math.floor(p.adrenaline),a.mp,table.concat(statuses,", "))
    end
    if name=="team" or name=="loadout" then
        if a.combat or p.combat_until>now or p.pending then return "Cannot change team/loadout in combat." end
        if name=="team" then
            if a.match_team then return "The arena assigns and locks match teams automatically." end
            if args[2]~="fang" and args[2]~="claw" then return "Use !whm team fang|claw." end
            p.team=args[2]; return "Team set to "..p.team
        end
        if args[2]==args[3] or not actions[args[2]] or not actions[args[3]]
            or not actions[args[2]].additional or not actions[args[3]].additional or args[4] then
            return "Choose two distinct actions: repose protect rescue largesse attunement."
        end
        if M.allowed and (not M.allowed(a,actions[args[2]]) or not M.allowed(a,actions[args[3]])) then return "Select additional actions for your role." end
        p.loadout={[args[2]]=true,[args[3]]=true}; return "Additional action loadout updated."
    end
    if name=="adrenaline" then
        -- Explicit test control, independent of normal match generation.
        p.adrenaline=100; return "Test control: adrenaline filled."
    end
    if M.resolve then name=M.resolve(a,name) end
    local d=definition(id,name)
    if not d then return "Unknown Feast action; use !whm list." end
    if d.target=="self" or args[2]=="self" then target=id
    elseif target==0 and d.target=="ally" then target=id end
    local err=error_for(id,name,target,false,nil,d)
    if err then return err end
    p.last_input=args[1]; p.last_definition=d
    if d.gcd then p.gcd=math.max(now,p.gcd)+d.recast end
    if not d.gcd or d.cooldown_recast then
        local times=p.cooldowns[d.cooldown or name]
        table.insert(times,math.max(now,times[#times] or now)+(d.cooldown_recast or d.recast))
    end
    if d.cast>0 then
        p.pending={definition=d,name=name,target=target,ends=now+d.cast,origin={x=a.x,y=a.y,z=a.z}}
        emit("cast",id,target,math.floor(d.cast*1000),name)
        if d.point then M.events[#M.events].x=d.point.x; M.events[#M.events].y=d.point.y; M.events[#M.events].z=d.point.z end
        return string.format("Casting Feast %s (%.1fs).",name,d.cast)
    end
    complete(id,name,target,nil,d)
    return ""
end

function M.supply(id,wolfs_heart,now)
    M.now=now
    local a=state(id); local p=M.players[id]
    if not valid(a) or not p then return end
    if wolfs_heart then
        buff(a,a,"wolfs_heart",60)
        a.statuses.wolfs_heart.next_tick=now+1
        emit("wolfs_heart",id,id,60)
    else
        p.adrenaline=math.min(100,p.adrenaline+25)
    end
end
function M.adrenaline(id)
    local p=M.players[id]
    return p and math.floor(p.adrenaline) or 0
end
M.internal={add=add,state=state,has=has,buff=buff,damage=damage,heal=heal,emit=emit,
    allies=allies,enemies=enemies,friendly=friendly,valid=valid,distance=distance,outgoing=outgoing,
    healing_potency=function(a,b,n)
        n=n*outgoing(a.id,true)
        return M.healing_received and M.healing_received(a,b,n) or n
    end}
return M
