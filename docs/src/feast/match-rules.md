# Match rules

The rules of a medal match live in `server/feast/rules.rs`, as pure logic
with its own tests. Supply boxes are in `server/feast/supplies.rs`. The
rules follow the official Feast guide as of patch 6.08; values the guide does
not give are marked "provisional" in the code and listed in
[Known gaps](known-gaps.md).

## The model

`Match` holds the phase, a deadline, the participants (`Player`: team, job,
medals, position, alive, present) and the medal piles on the ground (`Drop`).
The arena calls `Match::step(now, inputs)` every tick with one `Input` per
participant currently in the instance. Anyone missing from the inputs is
treated as disconnected. `step` updates the state and leaves two lists for the
arena to act on: `announcements` (chat messages) and `pickups` (to animate).

Time is seconds since the arena started (`Arena::now`).

## Phases

```text
Countdown ──(5 s)──► Active ──(6 min)──► Finished(winner)
                       │                   ▲
                       │ tied at time       │ next pickup, or 2 min
                       └──► SuddenDeath ────┘   (draw)
```

| Phase | What happens |
|---|---|
| `Countdown` | Preparation (`COUNTDOWN`, 5 s; a [custom match](arena.md#custom-matches) holds it until the host starts). Players are jailed. New arrivals join; teams and jobs can still change. |
| `Active` | The match (`MATCH_TIME`, 360 s). Jobs lock: a player who changes job can no longer act or collect. |
| `SuddenDeath` | If regulation ends tied (`SUDDEN_DEATH`, 120 s). The first medal pickup wins; time running out is a draw. |
| `Finished(team)` | The result. Nothing changes until `!feast restart`. |

A team holding `WINNING_SCORE` (600) medals wins immediately. When time runs
out, the team holding more wins. Medals on the ground count for nobody.

## Teams

During the countdown, each new participant joins the smaller team (Fangs on a
tie), up to four per side, in order of actor ID. A participant who leaves
during the countdown loses their place. After the countdown the roster is
fixed: late arrivals are spectators.

## Medals

- Everyone starts with `STARTING_MEDALS` (100).
- **Limits** by role (`medal_limit`): tanks 200; melee 175; ranged and casters
  150; healers 125. A job without a limit did not exist in the Feast and
  cannot join.
- **KO:** the player drops half their medals, rounded up, where they fell.
- **Disconnect:** the player drops all their medals in an *abandoned* pile.
  If they reconnect while it is still there, they get it back.
- **Pickup:** a living enemy standing within 2 yalms of a pile takes as many
  medals as they have room for. A partly collected pile stays.
- **Expiry:** an ordinary pile nobody collects returns to its owner after 60
  seconds. Abandoned piles never expire.
- **Reserved room:** a player's own piles count against their limit
  (`pickup_space`), so dying and collecting elsewhere cannot exceed it.

The arena shows piles with the original Feast medal models, larger for more
medals (`arena::medal_prefab`), coloured by team. Each viewer sees a pile
dimmed when they cannot collect it: own team, dead, job changed, or full
(`arena::medal_state`, `Match::can_collect`). A pickup plays the model's own
pickup animation and sound before the object is removed.

## Marked for Culling

To stop teams from waiting each other out: after 90 seconds without any KO,
every living participant gets one stack of Marked for Culling, plus one every
15 seconds, up to five (`culling_stacks`). Each stack makes them take 10% more
damage (the Lua engine reads the stacks from the actor's status 994). A KO
resets the timer. Sudden death starts at one stack and ignores KOs.

## Supplies

Supply boxes spawn at 1:30 (adrenaline), 3:00 (Wolf's Heart) and 4:30
(adrenaline) into the match (`supplies::WAVES`), never in sudden death.

- A box is a hostile battle NPC that anyone can attack, standing on an
  invisible controller bound to the original layout prefab (which provides the
  model, the break animation and the sound). Adrenaline boxes appear north and
  south of the centre, the Wolf's Heart box at the centre.
- The server totals each team's damage to the box (`record_damage`, fed by the
  engine's `damage` events). When it breaks, the team with more total damage
  gets a kit on the ground; a tie goes to the Fangs.
- Any living member of that team who walks over the kit collects it:
  adrenaline +25, or Wolf's Heart (MP regeneration for 60 seconds). Rewards
  are applied by the combat engine (`M.supply`).
- Box HP (5000) and how long an untouched kit lasts (60 s) are provisional.

`!feast supply [heart]` spawns a wave immediately, for testing.

## Changing the rules

- Timings and limits are constants at the top of `rules.rs` and `supplies.rs`.
- Every rule has a test in the same file. Run `cargo test -p kawari-world
  feast` after a change, and add a test for the new behaviour.
- The rules only see `Input`s, so they work the same for players and
  practice dummies.
