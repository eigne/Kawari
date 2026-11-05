# Actions, casting and timing

How a Feast action gets from a hotbar button to an animation on everyone's
screen, and how cast times, slidecasting, the GCD, cooldowns and charges are
kept in step between the client and the server.

## Historical action IDs

Every Feast action is its original Action sheet row from patch 6.08: Glare
is 17789, Cure II 8896, Raiden Thrust 18916. Patch 6.1 did not delete these
rows. It blanked their names and icons (icon 784), removed their cast and
recast times and unmarked them as PvP, but kept their **animations and
effects** (cast timeline, hit timeline, VFX, sounds). So if the client is told
to cast row 17789, it plays the real Feast Glare.

`resources/feast-actions.json` restores what was blanked, for every action of
every job: name, icon, description, cast, recast, cooldown groups, charges,
range, radius and MP cost. It was built from the archived 6.08 Action sheet and
the archived official job guides (scripts in `tools/feast-assets`). The server
uses it to map an ID to the action's Lua key (`feast::actions`); the HUD uses
it to answer the client's questions about these rows.

Each action also has a definition in the Lua engine with the same key and ID.
A server test checks the two agree.

## The client side: making blank rows usable

The HUD puts the job's historical actions on the native PvP hotbars
(`FeastHotbars.cs`). From then on the player uses them like any other action,
and the client does its normal work: checks, cast bar, recast sweep,
animation, `ActionRequest`. Because the rows are blank, the HUD hooks the
functions the client asks about an action and answers from the catalog and
from the server's live records:

| Client function | HUD answer |
|---|---|
| `GetActionStatus` | Can it be used now? No outside a running match, without the needed adrenaline or lilies (`FeastResources`), while casting, or while recasting (with charges counted). Everything else is left to the server. |
| `GetSpellIdForAction` | What the button is now: a combo's next step or a proc (from the server's hotbar records, see below). |
| `GetAdjustedCastTime`, `GetAdjustedRecastTime` | Cast and recast from the server's hotbar records, or the catalog. |
| `GetRecastGroup`, `GetAdditionalRecastGroup` | The catalog's cooldown groups, so the right buttons sweep together (the GCD is group 57). |
| `GetMaxCharges` | The catalog's charges. |
| `GetIconIdForSlot`, `LoadIconId`, `UpdateHotbarSlot` | The historical icon (of the current combo step). |
| `UseActionLocation` | Ground-targeted actions: see below. |

Each hook only answers for the current job's Feast actions inside the Wolves'
Den or arena; anything else goes to the game's original function.

**Combos and procs.** What a button turns into is decided on the server by
the engine's `M.resolve`. Each director frame carries a hotbar record (`F6`)
for every button: its ID, what it currently becomes, and its current cast and
recast in milliseconds (`M.hotbar`). The HUD uses these to swap icons and
spell IDs and to report buffed cast times, so a Raiden Thrust button shows
and casts Full Thrust when the combo is ready.

**Tooltips.** `NativeActionDetail.cs` fills the native tooltip with the
catalog's name, description and numbers.

## The server side: one request

1. The client sends `ActionRequest { action_id, sequence, target }`. For a
   combo button the ID is the button's (Raiden Thrust), not what it resolves
   to.
2. `combat::native_action` advances the engine, then calls
   `M.command(actor, key, target, now)`, where the engine resolves the combo
   and checks the action.
3. **Refused:** no `cast` or `animation` event comes back. The server sends
   `CancelCast` so the client drops its cast bar, and shows the engine's reason
   in chat.
4. **Accepted:** the server keeps the request in
   `CombatEngine::native_requests` until the action lands, so it can be
   acknowledged.

## Casts and animations

Two packets present an action, both built in `effects::presentation`:

- **`ActorCast`** for a `cast` event: the cast bar and casting animation.
  `spell_id` is the resolved historical action (it selects the animation);
  `action_id` is the request's (so the caster's client recognises its own
  cast); `cast_time` comes from the engine. Everyone nearby, including the
  caster, gets it.
- **`ActionEffect1`**, or **`AoeEffect8`** for more than one target, for an
  `animation` event: the hit animation and the damage or healing numbers.
  `spell_id` is again the resolved action. If the caster's request is still
  pending, the result also carries its `action_id`, `action_type` and
  `sequence`: that is how the client knows its request was carried out, ends
  its cast and keeps its recast. `AoeEffect8` holds eight targets; larger hits
  are sent as several packets.

These packets only present the action. HP has already been changed by
separate `hp` events.

An earlier version cast actions through chat commands, so the client never
knew it was casting and a plugin had to play the casting poses and effects
by hand. With real `ActionRequest`s the client does it all itself; that code
is gone.

## Cast timing and slidecasting

When a cast starts, the engine records `pending = {ends, origin}`: when it
finishes and where the caster stood.

- **Slidecasting.** Moving during the last half second of a cast does not
  interrupt it (`SLIDECAST_WINDOW = 0.5` in `Engine.lua`). Moving more than
  0.1 yalm before that cancels the cast.
- **Movement is checked as it arrives.** Each movement packet calls
  `M.movement` (from the movement handler in `server/mod.rs`), which marks the
  cast `moved_early` if it is too soon. A late world tick therefore cannot
  miss an early move.
- **Completion.** The cast lands on the first engine update at or after
  `ends`: the next world tick (at most about half a second later), or sooner if
  the player acts. Range is measured from `origin`, so sliding out of range in
  the last half second is fine.
- **Interruption** by movement, death or crowd control cancels the client's
  cast and refunds what the cast reserved: the GCD (`reset_gcd`) and any own
  cooldown (`reset_action_cooldown`).

## The GCD, cooldowns and leeway

The client starts a recast when it sends the request; the server starts the
same recast when it accepts it. Keeping the two aligned is the source of
several rules:

- **Recasts start at input**, in `M.command`, not when a cast lands. A cast
  shorter than the GCD therefore never delays the next GCD. Interruption
  refunds it.
- **Leeway** (`RECAST_TOLERANCE = 0.2` in `Engine.lua`). The client queues
  the next action at the end of a recast. That request can arrive a little
  before the server's recast ends, if this request's network delay was shorter
  than the last one's. The engine accepts actions up to 0.2 s early.
- **No drift.** A new GCD starts from the previous deadline, not from now
  (`p.gcd = max(now, p.gcd) + recast`), so arriving early never shortens the
  recast; the same applies to charges.
- **No resync on acceptance.** The server does not resend the recast when it
  accepts an action: the client already started it, and a resend arrives a
  round trip late and visibly rolls the sweep back. Refusals, refunds and
  cooldown reductions do resend (`SetCooldownTimer`, elapsed and total in
  hundredths of a second, by zero-based cooldown group).
- **Weaponskills with their own recast** (Drill, Air Anchor: catalog
  `secondary` 57) use both their own group and the GCD. The client does not start
  the GCD for them, so the server sends it (`native_action`).
- **Charges.** Each use adds a deadline to the action's list; the action is
  ready while fewer than `charges` deadlines are still running. The engine
  reports the recast as elapsed/total over all charges (`M.recast`), and the
  HUD considers a charge ready once one charge's worth has elapsed
  (`FeastJobActions.ChargeReady`).
- The client's own gates (casting, recasting) stay on. Letting the client
  send actions the server would refuse caused visible cast cancels.

## Ground-targeted actions

Soten, Shukuchi and Cometeor are aimed with the ground cursor. The server does
not implement the location-action packet, so the HUD's `UseActionLocation`
hook sends `!feast ground <id> <x> <y> <z>` as a chat command instead. The
server checks the point (`combat::ground_action`) and calls
`M.ground_command`. Because the client did not start the recast itself, the
server sends it.

## Party list action slots and adrenaline

Each team list row shows three action slots: the job's adrenaline rush and
the two additional actions the player chose, with their remaining cooldowns
(`M.party_actions`, director record `F4`). The adrenaline gauge (record `F5`)
fills the native limit break bar.
