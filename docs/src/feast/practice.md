# Practice dummies

`server/feast/practice.rs` gives one player something to hit without a
second character. It only works when you are the only real player in the
arena, and starts a fresh match.

`!feast dummies [1-3]` spawns up to three stationary opponents (Claws; you
are a Fang) with 5000 HP and 100 medals each, standing next to you. They are
practice targets: KO one and collect its medals. `!feast attack on` makes
them hit back (1000 damage every 2.5 seconds within 3 yalms); `off` stops it.
KO'd dummies stay as corpses until the next setup. `!feast restart` after a
result spawns the same number again.

Dummies are entries in `FeastState::practice`. The arena treats them as
participants (they hold, drop and collect medals) and the NPC AI leaves them
alone. They use the Claws' battalion, so they show as enemies. The combat
engine does not give them a job kit: they are targets, not players.

## Testing with other players

For anything closer to a real match (casts against a moving target, healing,
crowd control on a player, the team lists), log in a second character on
another account and queue both with `!feast`. A party that queued together is
split into the two teams automatically; see [The arena](arena.md#hostility-who-is-an-enemy).
