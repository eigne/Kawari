# Solo practice bots

Enter the arena with `!feast`, then use `!feast bots`.
This resets practice to a 4v4 match: the player and three allied bots versus four
enemy bots, each starting with 100 medals. Setup requires exactly one real player.
The normal 5-second preparation and match rules apply.

- `!feast attack off` pauses bot decisions and movement (an existing cast can finish).
- `!feast attack on` resumes them.
- `!feast bots off` removes practice actors and resets the solo match.
- `!feast restart` recreates the bot teams after the match ends.
- `!feast dummies [1-3]` still creates the old stationary opponents.

Bots now use WHM/PLD/SAM/BLM with 5000 HP. Each side has one of each supported
job; allied bots omit the player's job. They use the shared server Feast engine,
including cast times, cooldowns, MP,
crowd control, animations, and team restrictions. They pursue nearby enemies and
supply boxes, walk over eligible medals and team-owned kits, and respawn at their
team's jail after ten seconds. Supply damage and rewards use the normal rules.

Movement uses the arena navmesh when available. Without navigation data it falls
back to direct pursuit, which can cross scenery. The navmesh generator leaves out
the jail barriers (shared groups 7403947/7403946), which open during play; a
mesh generated before that fix seals bots inside their jails. Delete
`navimesh/bg/ffxiv/pvp_xx/pvp/l1p2/navimesh/l1p2.nvm` and run
`kawari-navimesh 745 <that path>` to regenerate it. There is no advanced positioning,
full rotation or coordinated focus fire. WHM heals, BLM manages elemental states,
SAM uses combos/Iaijutsu/Kenki, and PLD uses Oath, spells and defenses.
Native movement, targeting, and presentation still need an in-client playtest.

Validation: world regression tests cover 4v4 roster/scores, preparation and pause
gates, movement, actual delayed Glare damage, ten-second respawn, medal retention,
and removal. Production world binary built with Oodle enabled.
