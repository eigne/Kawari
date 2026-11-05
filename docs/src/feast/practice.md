# Practice: dummies and bots

`server/feast/practice.rs` lets one player test without seven others. Both
modes start a fresh match, and only work when you are the only real player
in the arena.

## Dummies

`!feast dummies [1-3]` spawns up to three stationary opponents (Claws; you
are a Fang) with 5000 HP and 100 medals each, standing next to you. They are
practice targets: KO one and collect its medals. `!feast attack on` makes
them hit back (1000 damage every 2.5 seconds within 3 yalms); `off` stops it.
KO'd dummies stay as corpses until the next setup.

## Bots

`!feast bots` (or `bots on`) fills both teams for a 4v4: three allies and four
enemies, playing White Mage, Paladin, Samurai and Black Mage (your own job is
left out of your team). `!feast attack off` pauses them; `!feast bots off`
removes them.

Bots are battle NPCs that the combat engine treats as players. Each world
tick, `tick_bots`:

1. respawns a bot ten seconds after its KO, in its team's jail;
2. picks a goal: a kit or enemy medal pile it can collect, or else the
   nearest enemy or supply box;
3. if in range (4 yalms for melee, 20 for the rest) and ready, asks the engine
   what to do (`M.bot_action`: an emergency heal on a teammate under 30% HP,
   otherwise the job's rotation), then uses it through `combat::command`,
   exactly like a player, so all rules apply;
4. otherwise runs towards the goal at player speed, along the navmesh.

Bots are deliberately simple. To make one smarter, change its priorities in
`M.bot_action` (Lua) or its goals in `tick_bots` (Rust).

## How they fit into a match

Practice actors are entries in `FeastState::practice`. The arena treats them
as participants (they hold medals, drop them and collect them), the engine
treats bots as players (they get a kit, cooldowns and gauges), and the NPC AI
leaves them alone. They use the same team battalions as players, so allies
show as allies and enemies as enemies. `!feast restart` after a result sets
up the same practice again.
