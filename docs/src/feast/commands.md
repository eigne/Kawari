# Commands

All commands are typed in normal chat. They are debug commands, so the
character needs the rank Kawari requires for those.

## Entering and leaving (Lua: `commands/debug/Feast.lua`)

| Command | Does |
|---|---|
| `!feast` | Queues for the Feast duty (or enters directly in exploration mode). Use from a town or open-world zone. |
| `!feast duty` | Queues for the duty explicitly. |
| `!feast leave` | Leaves the arena. |
| `!feast den` | Travels to the Wolves' Den, where PvP actions are set up. |

## In the arena (Rust: `server/feast/commands.rs`)

| Command | Does |
|---|---|
| `!feast status` | The clock, both scores, and your team, medals and limit. |
| `!feast restart` | After a result: starts the next match (practice setups are recreated). |
| `!feast bots [on\|off]` | Solo 4v4 against bots, or removes them. See [Practice](practice.md). |
| `!feast dummies [1-3]` | Stationary practice opponents. |
| `!feast attack on\|off` | Makes dummies and bots act, or pauses them. |
| `!feast supply [heart]` | Spawns an adrenaline (or Wolf's Heart) supply wave now. Testing only. |
| `!feast actions` | Lists your job's Feast action keys. |
| `!feast job` | Shows your job resources (gauge values). |
| `!feast loadout <a> <b>` | Sets your two additional actions, during preparation only. The HUD sends this for you. |
| `!feast ground <id> <x> <y> <z>` | Uses a ground-targeted action. The HUD sends this for you. |
| `!feast hud` | Replies with a hidden respawn countdown. The HUD polls this in exploration mode. |
| `!whm <text>` | Sends `<text>` to the combat engine as your command: an action key (`!whm glare` uses Glare on your target), `list`, `status`, `adrenaline` (fills your adrenaline, for testing). |

## HUD plugin

| Command | Does |
|---|---|
| `/feastactions` | Opens the PvP Actions window (Wolves' Den only). |
| `/feasthud` | Explains where to move the match header (HUD Layout: Crystalline Conflict Progress Gauge). |
