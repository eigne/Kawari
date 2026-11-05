# The Feast: developer guide

This part of the book is for developers working on **the Feast**, the 4v4
medal PvP mode that this fork of Kawari brings back. It assumes you can read
Rust, Lua and C#, but not that you know anything about how the Feast was
added. Read this page first, then [Setting up](setup.md) and
[Architecture](architecture.md). The other chapters can be read as needed.

## What the Feast was

The Feast was FINAL FANTASY XIV's ranked PvP mode until patch 6.1 (April 2022),
when Crystalline Conflict replaced it. Two teams of four, the **Fangs** and
the **Claws**, fought in a small arena:

- Everyone starts with 100 medals. Knocking out (KO) an enemy makes them drop
  half their medals on the ground; walking over an enemy's pile collects it.
- A team wins by holding 600 medals, or by holding more when the six-minute
  match ends. A tie goes to sudden death: the next pickup wins.
- Each job can carry only so many medals (tanks 200, healers 125).
- Supply boxes appear during the match. Breaking one gives the team that
  damaged it most a kit: adrenaline or MP regeneration.
- PvP actions were a separate, smaller kit per job, plus two role actions
  each player chose in the Wolves' Den, and an adrenaline rush (a limit break).

When patch 6.1 removed it, the server logic disappeared and most of the client
was cut: the arena's territory row was blanked, the duty definition deleted,
the UI code removed and the PvP action rows emptied of names and icons. But a
lot survived in the client's files: the arena map, the action animations and
effects, the medal and supply box models, the UI layouts and textures. This
fork rebuilds the missing pieces around what survived.

## What this fork restores

| Piece | Where it lives | Chapter |
|---|---|---|
| The arena (Crystal Tower Training Grounds, territory 745) and its duty | client data patch in `tools/feast-map`, server `server/feast/arena.rs` | [The arena](arena.md) |
| Match rules: teams, medals, drops, sudden death, culling | `server/feast/rules.rs` | [Match rules](match-rules.md) |
| Supply boxes and kits | `server/feast/supplies.rs` | [Match rules](match-rules.md#supplies) |
| Combat for all 19 Feast jobs | Lua, `resources/scripts/feast/` | [The combat engine](combat-engine.md) |
| Casting, recasts, animations | server + Lua + HUD | [Actions and casting](actions-and-casting.md) |
| The Feast UI: header, team lists, gauges, hotbars | Dalamud plugin, `tools/feast-hud` | [The HUD plugin](hud-plugin.md) |
| Practice bots and dummies | `server/feast/practice.rs` | [Practice](practice.md) |

The **server is the authority** for everything: rules, damage, cooldowns,
medals. The client is today's unmodified game plus two things: a small patch
to its data files so it knows the arena exists, and the HUD plugin, which
makes the game's surviving UI show Feast data and lets it use the old actions.

## How the pieces fit

```text
            ┌──────────────────── today's game client ─────────────────────┐
            │  game data, patched: territory 745 + Feast duty (tools/feast-map) │
            │  Kawari Feast HUD (Dalamud plugin, tools/feast-hud)            │
            │    - historical actions on the hotbars                        │
            │    - Feast header, team lists, gauges, medals                 │
            └───────────▲───────────────────────────────┬──────────────────┘
       director frames, │ casts, results,               │ ActionRequest,
       spawns, statuses │ HP, positions                 │ movement, chat commands
            ┌───────────┴───────────────────────────────▼──────────────────┐
            │  Kawari world server (servers/world)                          │
            │    server/feast/  arena.rs   one match per arena instance      │
            │                   rules.rs   medals, phases, winner            │
            │                   combat.rs  ──► Lua combat engine             │
            │                   effects.rs ◄── its events, as packets        │
            │                   director.rs  match frames for the HUD        │
            └────────────────────────────────────────────────────────────┘
```

## Words used in this guide

- **Arena**: territory 745, Crystal Tower Training Grounds (map name `l1p2`).
  The server code calls one running match in one arena instance an `Arena`.
- **Duty**: the arena's ContentFinderCondition 476 / InstanceContent 40022.
  Entering through the duty gives the client a duty director, which the HUD needs.
- **Director**: the client object that runs a duty's UI. The server reuses the
  arena duty's director to send match data to the HUD (see
  [Director protocol](director-protocol.md)).
- **Historical action**: an Action sheet row from patch 6.08 that the Feast
  used. Rows keep their IDs, animations and effects in today's client, but lost
  their names, icons and timings. `resources/feast-actions.json` lists them.
- **Combat engine**: the Lua code in `resources/scripts/feast/` that decides
  every combat outcome.
- **Battalion**: a number in each character's spawn data that the client uses
  to decide who is an enemy. Fangs are battalion 1, Claws 2.
- **CC**: Crystalline Conflict, the PvP mode that replaced the Feast. Its UI is
  reused for the Feast's.
- **Wolves' Den**: the PvP lobby (territory 250), where actions are chosen.

## History

The Feast was rebuilt in many small steps, and the early ones left notes:
long investigation logs (`HANDOFF.md`, `docs/src/development/feast-*.md`) and
a first client plugin (`tools/feast-plugin`). Those were removed when this
guide was written, because they described intermediate states. They remain in
the git history before the commit that added this guide, if you need the
evidence behind a decision.
