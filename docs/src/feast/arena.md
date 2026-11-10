# The arena

How the Crystal Tower Training Grounds (territory 745, map `l1p2`) became a
working Feast arena: entering it, the duty, the jails, respawning, and making
the other team's players enemies.

## The client data patch

The client decides which territory to load from a row in its TerritoryType
sheet. Patch 6.1 blanked row 745, so the client cannot load the arena even
though the map files (`bg/ffxiv/pvp_xx/pvp/l1p2/`) are all still installed.
A server cannot fix this: the zone-in packet only carries the territory ID.

`tools/feast-map` therefore patches the client's sheets (see [Setting up](setup.md)):

- `feast_map.py` restores TerritoryType 745 (name, map, place names, weather)
  as an ordinary open-world area. That alone is "exploration mode".
- `prepare_feast_duty.py` / `install_feast_duty.py` restore the duty:
  ContentFinderCondition 476 and InstanceContent 40022 with their original
  IDs, and link the territory to them (intended use 28, a PvP duty).

Both write only the changed rows inside the existing data pages, refuse to
run against an unexpected client build, and back up the original bytes.

The server reads the same files. Before letting anyone queue, it checks the
three rows agree (`GameData::feast_duty_metadata_ready`); `Feast.lua` asks
`feast_entry_mode()` whether to use the duty, exploration mode, or refuse.

## Entering and leaving

`!feast` (in `resources/scripts/commands/debug/Feast.lua`):

- **Duty mode:** registers for content 476, exactly like the Duty Finder.
  The client gets a duty pop; accepting it creates an arena instance and
  zones the party into it. A party enters together, as a
  [custom match](#custom-matches).
- **Exploration mode:** teleports straight to territory 745.

Either way the player arrives in the west jail (`change_zone_warp_to_entrance`
in `zone.rs` has a Feast case, since the map has no entrance marker). The
first tick of the arena then assigns their team and moves them to the right
jail (`arena::sync_jails`).

`!feast leave` abandons the duty (or teleports to Limsa in exploration
mode). `!feast den` goes to the Wolves' Den, where actions are chosen.

### The duty director and timer

Entering a duty gives the client a director, which owns the duty's UI and
timer. The arena's director is InstanceContent 40022 (handler `0x80039C56`).
The client opens Crystalline Conflict's UI for it, which the HUD turns into
the Feast's ([The HUD plugin](hud-plugin.md)).

- The duty's length is set to preparation plus the match
  (`feast::DUTY_DURATION`), not the sheet's time limit.
- When the match changes phase, the server restarts the client's duty timer
  with `DutyCommence` and the time left (`arena::restart_duty_timer`). The
  client ignores `SetDutyTimeRemaining` for this duty.
- A respawn reloads the zone, but the director must be initialized only once
  per player (`FeastState::directors_sent`); doing it twice leaves a second
  CC header on screen.

## Custom matches

A party that enters together plays a custom match (`server/feast/lobby.rs`).
This is how eight people play 4v4: form a party of up to eight with ordinary
invitations, and the leader uses `!feast`.

- **Only the leader** can take the party in (`ToServer::JoinContent` in
  `server/mod.rs` refuses other members).
- **Teams follow the party list.** The first half of the list is the Fangs,
  the rest the Claws (members 1–4 and 5–8 of a full party). Any size up to
  4v4 works, even or not.
- **The leader is the host.** Preparation does not run out while the lobby is
  `waiting`: `arena::tick` calls `Match::hold` every tick until the host uses
  `!feast start`, then the usual countdown runs. If the host leaves, the
  lowest-ID player present takes over.
- **`!feast teams`** lists both teams with a number per player. The host
  moves one player with `!feast swap <number>` or exchanges two with
  `!feast swap <number> <number>`, before a match or after its result. The
  lobby passes each player's team to the rules as `Input::team`.
- **Parties.** While the host arranges the teams, and again after the
  result, everyone is in one party. During the match each team has its own
  party (see [Hostility](#hostility-who-is-an-enemy)). So the group is still
  together when it leaves the arena after a result; players who leave
  mid-match are in their team's party.
- **Rematch.** After the result the host uses `!feast restart`; the teams are
  kept, and the lobby waits for `!feast start` again.

The lobby lives in `FeastState`, not in the `Arena`, so it survives from one
match to the next. It is reset when the instance empties.

Players who enter without a party are not a custom match: they join the
smaller team and the countdown starts on its own.

## Jails and gates

Each team starts in a jail: Fangs at x = −58, Claws at x = 58 (y = 0.5),
four standing positions per side (`arena::team_spawn`). The gates are closed
during preparation and open when the match starts.

The gates are not spawned objects but **shared groups** in the map's layout:
7403947/7403946 (collision) and 7515404/7515405 (the glowing barrier). The
server controls them by spawning invisible EventObjs bound to them
(`bind_layout_id`) and animating those with
`SetSharedGroupTimelineState`: timeline slot 1 closes, slot 3 opens. Details
that took a while to find, all in `arena::sync_jails`:

- Each player must receive the update after their client has spawned the
  controller object, or the update is lost. The server tracks what each
  viewer last received and waits for the spawn.
- The controllers' spawn data (`args1`) also carries the current state, for
  players who arrive later.
- A zone load clears what that player has seen (`arena::on_zone_loaded`), so a
  reconnect gets the current gates.

## Hostility: who is an enemy

Getting opponents to show as red, be targetable with Tab and be valid targets
for enemy-only actions took three separate fixes. They are the most
surprising part of the arena.

The client decides how to treat another character in `ClassifyTarget`. In
order, it checks: is it in my party (ally)? Is it an enemy? An "enemy" for
players means different **battalions** that the Battalion sheet marks as
hostile, but only if the territory's **BattalionMode** allows player
battalions to count.

1. **Battalions.** Each team gets a battalion that is hostile to the other:
   Fangs 1, Claws 2 (`arena::team_battalion`; 1–3 are the Frontline Grand
   Companies). The client only reads battalions from spawn packets, so when
   a team is assigned (after the player spawned), the server sends the player
   their own respawn (`FromServer::SetFeastBattalion`, which makes the
   connection respawn them with `feast_battalion`) and despawns them for
   everyone else, who then see them spawn again (`arena::sync_battalions`).
2. **BattalionMode.** Row 745 has BattalionMode 1, which only lets *NPC*
   battalions count, so players can never be enemies there. This is client
   data that the server cannot change, so the HUD changes it in the client's
   memory to 4 (the value PvP territories use) while in the arena
   (`NativeBattalionMode.cs`).
3. **Parties.** Party members are always allies, before battalions are
   checked. A party that entered together spans both teams, so during a
   match the server disbands any party whose members are on different teams
   and forms one party per team instead (`lobby::sync_parties`, with
   `party::form_party`). The party handlers tolerate a party disappearing
   while a member is still loading.

Practice dummies use the Claws' battalion, so they are enemies for the same
reasons.

## Respawning

A KO'd Feast player waits ten seconds, then may return to their team's jail
with full HP.

- `kill_actor` starts the wait (`arena::begin_respawn_lock`); repeated death
  notifications do not restart it.
- The client's "Return" sends `ClientTrigger` 200 with kind 8, which becomes
  `ToServer::ReturnAfterDeath`. In the arena (`zone.rs`):
  - too early: the server keeps the request as a queued task
    (`QueuedTaskData::FeastRespawn`) and carries it out when the wait ends,
    because the client has already faded to black and would otherwise stay
    there;
  - on time: it reinitializes the player inside the same instance
    (`prepare_feast_respawn` sends `PrepareZoning` and replaces the actor) and
    zones them into their jail with `WarpType::Resurrection`.
- The HUD shows the wait on the confirmation button ([RespawnPrompt](hud-plugin.md#hostility-medals-and-the-respawn-countdown)).

## Other arena rules

- **Everyone in view.** Normally actors far apart are despawned for each
  other. The team lists need every participant loaded, so match participants
  always see each other (`server/mod.rs`, "feast_participants").
- **HP by role.** In the arena a player's maximum HP depends only on their
  role, not their level or gear, and they arrive at full HP:

  | Role | Maximum HP |
  |---|---|
  | Tank (PLD, WAR, DRK, GNB) | 20,000 |
  | Healer (WHM, SCH, AST, SGE) | 17,500 |
  | Melee (MNK, DRG, NIN, SAM, RPR) | 17,500 |
  | Physical ranged (BRD, MCH, DNC) | 16,000 |
  | Caster (BLM, SMN, RDM) | 15,000 |

  The table is `rules::max_hp`; `zone_connection/stats.rs` applies it.
- **No regeneration.** Players with Feast rules enabled do not regenerate HP
  outside combat; the engine owns their HP.
