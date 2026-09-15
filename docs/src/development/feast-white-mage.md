# Feast White Mage prototype

This implements the **pre-6.1 Feast** kit from the [official White Mage guide archived April 9, 2022](https://web.archive.org/web/20220409040536/https://na.finalfantasyxiv.com/jobguide/whitemage/#pvp).
The archived page's PvP section was last updated January 25, 2022. Historical action IDs were checked against the [6.08 action table](https://github.com/xivapi/ffxiv-datamining/blob/bc1a1de27e7bf00adba0a0cc362402459c55b7c0/csv/Action.csv).

## Trying it

For automatic spell buttons, use the [Kawari Feast Dalamud plugin](../../../tools/feast-plugin/README.md).
It temporarily loads hotbars 1–2 upon entering as WHM and sends these commands
internally when clicked or activated by keybind. No per-cast typing is required.
The command controls below remain available for development and loadout setup.

Equip White Mage, enter the restored arena with `!feast`, then enter:

```text
!whm on
!whm list
```

Select an enemy and use `!whm glare` or `!whm dia`. Select a teammate for healing,
or explicitly use `!whm cure self`. With no selected target, single-target heals
default to yourself. Area actions automatically select eligible recipients.
These commands can be put in chat macros.

`!whm status` shows lilies, Blood Lily, adrenaline, MP, team, and active Feast
effects. `!whm off` restores normal action use. While enabled, ordinary hotbar
actions are rejected so current PvE/Crystalline Conflict actions cannot silently
stand in for Feast actions. The historical action IDs are reference metadata;
the prototype does not send removed action IDs to the current client.

For player-versus-player testing, each participant enables the mode, then chooses
`!whm team fang` or `!whm team claw`. Same-team participants are treated as party
members. Spawned battle NPCs are enemies. Other players are excluded from offensive
targeting until they opt in. This is a test team assignment, not match/party setup.

Choose exactly two additional actions, outside combat:

```text
!whm loadout repose rescue
```

The default pair is Protect and Largesse. Changing a loadout preserves cooldowns.
Use `!whm adrenaline` to fill the test adrenaline gauge before `!whm rain`.
For MP testing, the existing `//gm mp 10000` command can replenish MP.

## Implemented actions

Spell GCD is 2.4 seconds. Ranges, radii, cast times, costs, and cooldowns come from
the archive. Damage and healing use the listed potency as a fixed HP amount in
this prototype; they do not use Kawari's PvE stat formula.

| Command | Historical ID | Behavior |
| --- | --- | --- |
| `cure` | 8895 | 2,000 heal; 1.5s cast; no MP cost. |
| `cure2` | 8896 | 4,000 heal; 1.5s cast; 1,000 MP. |
| `medica` | 18945 | 2,000 party heal within 15y; 2s cast; 1,000 MP. |
| `glare` | 17789 | 1,200 damage; 1.5s cast. |
| `dia` | 17790 | 800 damage and 10% increased damage taken for 15s; 1,000 MP. No damage over time. |
| `solace` | 17791 | 4,000 heal plus 400 regen for 15s; costs one lily and nourishes Blood Lily. |
| `rapture` | 18946 | 2,000 party heal within 15y plus 200 regen for 15s; costs one lily and nourishes Blood Lily. |
| `misery` | 17793 | 2,400 damage within 5y of the target; consumes a fully bloomed Blood Lily. |
| `tetra` | 13975 | 4,000 heal; two charges, sequential 15s recharge. |
| `assize` | 9620 | 1,000 enemy damage and 2,000 party healing within 15y; 30s recast. |
| `temperance` | 17794 | 20% outgoing damage/healing bonus; 10% party mitigation in a moving 30y aura; 15s duration, 60s recast. |
| `purify` | 1584 | Removes stun, sleep, bind, heavy, and silence; heals 2,000, doubled if it cleanses an affliction; 30s recast; usable while afflicted. |
| `bolt` | 18942 | Movement buff for 10s; 30s recast. |
| `kit` | 18943 | 3,000–6,000 self-heal based on missing HP; three charges, sequential 15s recharge. |
| `repose` | 17690 | Three-second sleep, broken by damage; interrupts casting; 30s recast. Additional action. |
| `protect` | 17832 | 20% mitigation for 10s; 45s recast. Additional action. |
| `rescue` | 17688 | Pulls another teammate within 30y to the caster; removes historical Holmgang binds, bypassing knockback resistance; 30s recast. Additional action. |
| `largesse` | 18957 | 25% healing bonus for 10s; 45s recast. Additional action. |
| `attunement` | 17689 | 3,000–6,000 self-heal plus 99% mitigation and movement/action lock for 3s; 30s recast. Additional action. |
| `rain` | 3362 | Empyrean Rain: heals 50% maximum HP and cleanses the party within 30y; consumes full adrenaline. Does not resurrect. |

Lilies accumulate once per ten seconds in combat, capped at three. Three lily
expenditures bloom Blood Lily. Cast completion rechecks target validity, range,
MP, and required gauges. Movement and crowd control interrupt casts. Regen uses
five three-second ticks over fifteen seconds. Healing clamps to maximum HP and
does not revive dead targets. Death clears pending casts, lilies, Blood Lily,
and temporary Feast effects.

## Prototype boundaries and assumptions

* Current character maximum HP/MP remain unchanged. This is not a complete historical PvP stat profile.
* Feast actions send current-client cast/action animations plus HP/MP packets. The optional Dalamud plugin supplies temporary spell buttons; historical status icons, cooldown UI, and gauge UI are not restored. Placeholder names/tooltips can differ from the Feast action.
* Internal Feast effects are reported by `!whm status`. Bolt uses the current client's Sprint movement effect for its ten-second duration.
* The guide specifies the endpoints but not the curve for Medical Kit/Attunement. The prototype interpolates linearly by missing-HP fraction. Regen snapshots healing bonuses; periodic timing and multiplicative modifier stacking are implementation assumptions requiring historical verification.
* Combat is inferred from NPC aggro and a ten-second window following damage or combat healing. Match-driven combat state and adrenaline generation are not implemented. Adrenaline must be filled with the explicit test command.
* Kawari's generic 10%-per-tick HP/MP recovery is disabled for opted-in participants. Historical passive regeneration still needs implementation.
* Targeting validates arena membership, team, life state, and distance. Line of sight, full action animation locks, crowd-control resistance/immunity rules, and a rated-match enforcement layer remain future work.
* `!whm off`/`on`, team selection, and adrenaline filling are development controls, not rules suitable for competitive matches.

## Code and validation

The 20 action definitions and their timed state live in
`resources/scripts/feast/WhiteMage.lua`. The adapter is
`servers/world/src/server/feast.rs`. It applies changes to live actors, hooks
incoming damage modifiers, and corrects movement during immobilizing effects.

Run `cargo test -p kawari-world feast`. The Lua behavior suite covers all 20
actions, resource gates, cooldown charges, party/enemy area selection, interruption,
status expiry, and modifiers. A Rust integration test checks live HP/MP changes
and incoming damage mitigation. Client presentation and multiplayer behavior still
need live testing.
