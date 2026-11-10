# The combat engine

Every combat rule of the Feast (damage, healing, casts, cooldowns, crowd
control, job gauges) is decided by a Lua engine in
`resources/scripts/feast/`. The server's `combat.rs` drives it and
`effects.rs` applies what it decides. This chapter explains how the engine
works; [Actions and casting](actions-and-casting.md) covers timing, and
[Jobs](jobs.md) how to add to it. The header comment of `Engine.lua` is the
short reference for everything below.

## Why Lua, and how it is loaded

The Feast has around 470 actions across 19 jobs, each with its own small
rules. In Lua each is a few lines, can be edited without rebuilding the
server, and can be tested directly (`resources/tests/feast_*.lua`).

`CombatEngine::new` loads, in order:

1. `Engine.lua`, which returns the engine table (called `M` in the scripts)
   with White Mage already defined;
2. `Jobs.lua`, a function that adds five jobs and the shared hooks to `M`;
3. `AdditionalJobs.lua`, a function that adds the other 13 jobs, given
   `AdditionalActionData.lua` (their generated metadata).

An arena instance creates its engine when the first player arrives and drops
it when the match ends, so the next match reloads the scripts from disk.
A Lua error loading them is reported to the player in chat.

## The calling convention

The engine never touches the world. Every call from the server is:

1. **`M.sync(snapshots, now)`**: replace `M.actors` with a snapshot of every
   actor: HP, MP, position, rotation, job, whether it is a player, its team
   and match state, its Marked for Culling stacks and its native statuses
   (`ActorSnapshot` in `combat.rs`). Lua-side statuses are carried over.
2. **One function**: `M.command` (use an action or a text command),
   `M.advance` (time passes), `M.ground_command`, `M.supply`,
   `M.modify_damage`, or a read-only query (`M.hotbar`, `M.job_gauge`,
   `M.party_actions`, `M.recast`, `M.blocked`, `M.enabled`, `M.bot_action`).
3. **`M.drain()`**: return and clear the events produced.

`now` is seconds since the engine was created. `CombatEngine::run` wraps
these three steps; `effects::apply` applies the events in order.

## State

- **`M.actors[id]`**: the latest snapshot, plus `statuses`: a table from
  status name to `{source, expires, potency, next_tick, ...}`. Statuses are
  plain names (`"protect"`, `"sleep"`, `"thunder_<caster id>"`), not game IDs.
- **`M.players[id]`**: the Feast state of every player with Feast rules
  enabled ("on"): team, `cooldowns` (action key → list of deadlines, one per
  charge used), `gcd` (deadline), `pending` (the cast in progress), `loadout`
  (the two chosen additional actions), `adrenaline` (0–100), `combat_until`,
  and each job's resources (lilies, Kenki, Oath, and so on).
- **`M.actions[key]`**: action definitions. See below.

A player is enabled automatically on arrival (`combat::tick` sends `on`).
`sync` resets a player's cooldowns, statuses and resources when their job
changes, and clears statuses and resources when they die.

## Actions

An action is defined with `add` (in `Engine.lua` and `Jobs.lua`) or with
`action` (in `AdditionalJobs.lua`, which reads the metadata from
`AdditionalActionData.lua`):

```lua
--  key      historical ID  target  cast recast  MP  range  effect
add("glare", 17789,         "enemy", 1.5, 2.4,    0,  25,    function(a,b) damage(a,b,1200) end, {gcd=true})
```

- `target` is `"self"`, `"enemy"`, `"ally"` (a teammate or yourself),
  `"other_ally"`, `"ground"` or `"any"`.
- The effect function receives the caster `a` and the target `b` (actor
  snapshots) and runs when the action lands.
- Optional fields: `gcd` (uses the global cooldown), `charges`, `cooldown`
  (a cooldown key shared by several actions, e.g. Lustrate and
  Indomitability both use `"sch_heal"`), `cooldown_recast` (an
  own recast on top of the GCD, like Drill), `additional` (a role action chosen
  in the loadout), `adrenaline` (needs a full adrenaline gauge), `weapon` or
  `spell` (silence only stops spells), `usable_while_cc`, `combo`, and
  `check = function(a, b, p)` returning an error message or nil.

When a player uses an action, `M.command`:

1. lets `M.resolve` turn the button into what it currently is (a combo step,
   a proc), and `M.effective` apply buffs to a copy of its definition (faster
   casts, free MP);
2. checks everything with `error_for`: alive, right job, allowed role action,
   match running, not crowd-controlled, valid target in range, MP, resources,
   the `check` function, and timing (not casting, GCD and cooldown ready);
3. on success starts its GCD or cooldown, then either starts a cast (`pending`
   plus a `cast` event) or lands it at once with `complete`.

`complete` re-checks everything except timing, pays MP and resources, runs
the effect, and emits an `animation` event listing every target it hit.

## Damage and healing

`damage(a, b, amount)` passes the amount through `M.modify_damage`:

1. zero if either side is not an active match participant, or they are on the
   same team, or the target is invulnerable;
2. multiplied by the caster's `M.outgoing` (their buffs and debuffs), by
   Marked for Culling (+10% per stack), and by the target's own mitigation
   (Dia +10%, Protect −20%, Temperance aura −10%, Attunement −99%);
3. then `M.incoming(a, b, amount)` for job mitigation, shields
   (`M.absorb_extra` and friends) and Holmgang;
4. damage wakes a sleeping target, and puts both sides in combat for 10 s.

It then emits `hp` (the new HP) and `damage` (the amount), and calls
`M.after_damage` (counters, Wildfire, Bloodbath and so on).

`heal(a, b, amount, percent)` applies `M.outgoing(id, true)` (Temperance,
Largesse) unless it is a percentage heal, then `M.healing_received`. Healing
never raises the dead.

The server also sends damage from outside the engine (scripted NPC actions,
practice dummies) through `M.modify_damage` (`combat::adjust_damage`), so
buffs and shields apply to it too.

## Statuses and crowd control

`buff(source, target, name, duration, potency)` adds a status, unless
`M.immune` says the target ignores it. A status with a `potency` heals every
3 seconds (or damages, if `dot` is set) in `M.advance`, which also removes
expired statuses.

Crowd control is ordinary statuses: `stun`, `sleep`, `bind`, `heavy`,
`silence`. `M.blocked(id, movement)` says whether one stops an actor:
stun, sleep and Attunement stop everything, bind stops movement, silence stops
spells. The server asks it before accepting player movement and before
NPC AI acts.

The client needs native statuses to show icons and to slow or root the
player. Every Lua status listed in `M.status_icons` is mirrored to its
Status sheet row through `status_visual` events (stun 1343, heavy 1344,
bind 1345, silence 1347, sleep 1348, and about 80 buffs and debuffs); see
[Status icons](jobs.md#status-icons). The engine also respects native crowd
control applied by other means (`native_cc` in `Engine.lua`), but ignores
the copies it sent itself (`sent_icons`). Bolt's sprint is sent separately,
as a `speed` event.

## Hooks

Job files change the engine's behaviour by defining optional functions on
`M`. The list is in the header of `Engine.lua`; the important ones:

| Hook | Used for |
|---|---|
| `M.resolve(a, key)` | What a button currently is: combos, procs, stance swaps. |
| `M.effective(a, d)` | Buffs that change an action's cast, recast or cost. |
| `M.outgoing`, `M.incoming`, `M.healing_received`, `M.absorb_extra`, `M.after_damage` | Damage and healing modifiers, shields, counters. |
| `M.initialize`, `M.reset_job`, `M.advance_job` | Job resources: set up, cleared, and ticked. |
| `M.party_list`, `M.allowed` | Each role's six role actions; which a job may use. |
| `M.job_gauge`, `M.hotbar` | What the HUD shows (see [Director protocol](director-protocol.md)). |

`Jobs.lua` defines these hooks first. `AdditionalJobs.lua` *wraps* them so
both files' jobs keep working:

```lua
local outgoing=M.outgoing          -- keep the previous version
function M.outgoing(id,healing)
 local f=outgoing(id,healing)      -- call it first
 if has(id,'combust') then f=f*.9 end
 ...
 return f
end
```

Follow the same pattern when adding a hook: never replace an existing one.

## Events

The engine reports what happened as events (`emit(kind, actor, target,
amount, message)`), which `effects.rs` applies in order. The full list is in
the `Engine.lua` header and in the `match` of `effects::apply`. The ones to
know:

| Event | Means |
|---|---|
| `hp`, `mp` | Target's new HP or MP. The only way the engine changes them. |
| `cast` | A cast starts: everyone sees a cast bar for `amount` ms. |
| `animation` | An action lands: everyone sees its animation and numbers, `hits` lists every target. |
| `cancel`, `reset_gcd`, `reset_action_cooldown` | A cast was refused or interrupted: cancel the client's cast and refund its recasts. |
| `move` | Teleport the target (dashes, knockbacks, Rescue). |
| `notice` | A chat message for the player. |

## Adrenaline and job resources

Adrenaline fills on its own from 0 to 100 over three minutes while the match
runs, plus 25 per adrenaline kit. At 100 the job's adrenaline rush (a limit
break) can be used, which empties it.

Job gauges (lilies, Beast Gauge, Kenki, ...) are fields on the player,
changed by actions (`gauge(a, "kenki", 10)`) and ticked by `M.advance_job`.
White Mage's lilies are ticked in `M.advance` itself. `M.job_gauge`
packs them for the HUD, which writes them into the game's own gauge widgets.
