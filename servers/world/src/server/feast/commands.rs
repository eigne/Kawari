//! The `!feast` chat commands that act on a running match.
//!
//! `!feast` itself (enter), `!feast leave`, `!feast den` and `!feast duty`
//! are Lua (resources/scripts/commands/debug/Feast.lua), because they move
//! the player between zones. Everything else is routed here instead of Lua
//! by `is_match_command`.
use std::{sync::Arc, time::Instant};

use kawari::common::ObjectId;
use parking_lot::Mutex;

use super::{arena, combat, practice, rules::Phase, supplies};
use crate::server::{instance::Instance, network::NetworkState};

/// The subcommands handled by the world server rather than Feast.lua.
const MATCH_COMMANDS: [&str; 10] = [
    "status", "hud", "restart", "dummies", "attack", "ground", "actions", "loadout", "job",
    "supply",
];

/// Whether a chat message is a `!feast` command for the running match.
pub fn is_match_command(text: &str) -> bool {
    let mut words = text.split_whitespace();
    words.next() == Some("!feast")
        && words
            .next()
            .is_some_and(|word| MATCH_COMMANDS.contains(&word))
}

const USAGE: &str =
    "Use !feast status, !feast restart, !feast dummies [1-3], or !feast attack on|off.";

/// Runs `text`, the words after `!feast`, for `id`.
pub fn command(
    instance: &mut Instance,
    network: &Arc<Mutex<NetworkState>>,
    id: ObjectId,
    text: &str,
) {
    if !super::in_arena(instance) {
        super::notice(
            network,
            id,
            "Enter the Feast arena first with !feast.".into(),
        );
        return;
    }
    arena::tick(instance, network);
    let words: Vec<_> = text.split_whitespace().collect();
    match words.as_slice() {
        // Sent by the HUD for ground-targeted actions; see combat::ground_action.
        ["ground", action, x, y, z] => {
            if let (Ok(action), Ok(x), Ok(y), Ok(z)) = (action.parse(), x.parse(), y.parse(), z.parse()) {
                combat::ground_action(instance, network, id, action, [x, y, z]);
            }
        }
        ["supply", rest @ ..] => {
            let kind = if rest.first() == Some(&"heart") {
                supplies::Kind::WolfsHeart
            } else {
                supplies::Kind::Adrenaline
            };
            spawn_supply_wave(instance, network, id, kind);
        }
        ["loadout", ..] if !preparing(instance) => super::notice(
            network,
            id,
            "Additional actions are set in the Wolves' Den (!feast den) and cannot change during a match.".into(),
        ),
        // Additional actions are chosen in the Wolves' Den; the HUD sends
        // them during preparation, and they stay fixed once the match begins.
        ["loadout", ..] => engine_command(instance, network, id, text),
        ["actions"] => engine_command(instance, network, id, "list"),
        ["job"] => engine_command(instance, network, id, "status"),
        ["dummies", ..] | ["attack", ..] => practice::command(instance, network, id, &words),
        // Polled by the HUD for the respawn countdown when it receives no
        // director frames (exploration mode). Private, hidden by the HUD.
        ["hud"] => super::notice(
            network,
            id,
            format!("KFEASTRESPAWN1|{}", arena::respawn_remaining(instance, id, Instant::now())),
        ),
        ["restart"] => restart(instance, network, id),
        ["status"] => status(instance, network, id),
        _ => super::notice(network, id, USAGE.into()),
    }
}

fn preparing(instance: &Instance) -> bool {
    instance
        .feast
        .arena
        .as_ref()
        .is_some_and(|a| a.rules.phase == Phase::Countdown)
}

fn engine_command(
    instance: &mut Instance,
    network: &Arc<Mutex<NetworkState>>,
    id: ObjectId,
    text: &str,
) {
    combat::tick(instance, network);
    combat::command(instance, network, id, text);
}

/// `!feast supply [heart]`: spawns a supply wave now, for testing.
fn spawn_supply_wave(
    instance: &mut Instance,
    network: &Arc<Mutex<NetworkState>>,
    id: ObjectId,
    kind: supplies::Kind,
) {
    match instance.feast.arena.take() {
        Some(mut arena) if arena.rules.phase == Phase::Active => {
            supplies::spawn_wave(instance, network, &mut arena.supplies, kind);
            instance.feast.arena = Some(arena);
            super::notice(network, id, format!("Spawned a {kind:?} supply wave."));
        }
        arena => {
            instance.feast.arena = arena;
            super::notice(
                network,
                id,
                "Supplies can only spawn during the match.".into(),
            );
        }
    }
}

/// `!feast restart`: starts the next match after a result. Practice
/// sessions are set up again with the same opponents.
fn restart(instance: &mut Instance, network: &Arc<Mutex<NetworkState>>, id: ObjectId) {
    let finished = instance
        .feast
        .arena
        .as_ref()
        .is_some_and(|a| matches!(a.rules.phase, Phase::Finished(_)));
    if !finished {
        super::notice(
            network,
            id,
            "A match can only restart after its result.".into(),
        );
        return;
    }
    if !instance.feast.practice.is_empty() {
        let count = instance.feast.practice.len().to_string();
        practice::command(instance, network, id, &["dummies", &count]);
    } else {
        arena::end_match(instance, network);
        arena::tick(instance, network);
    }
}

/// `!feast status`: the clock, scores and the player's own medals.
fn status(instance: &Instance, network: &Arc<Mutex<NetworkState>>, id: ObjectId) {
    let Some(arena) = &instance.feast.arena else {
        return;
    };
    super::notice(network, id, arena.rules.status(arena.now()));
    let message = match arena.rules.players.get(&id.0) {
        Some(p) => format!(
            "Team {}: {} medals / {} cap. Walk over enemy drops to collect them.",
            p.team.name(),
            p.medals,
            super::rules::medal_limit(p.job).unwrap_or(0)
        ),
        None => "This match's roster is locked. Join the next countdown.".into(),
    };
    super::notice(network, id, message);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_match_commands_but_keeps_lua_entry_and_exit() {
        for command in [
            "!feast status",
            "!feast hud",
            "!feast   status ",
            "!feast\trestart",
            "!feast dummies 3",
            "!feast attack off",
            "!feast supply",
            "!feast supply heart",
        ] {
            assert!(is_match_command(command), "{command}");
        }
        for command in [
            "!feast",
            "!feast leave",
            "!feast den",
            "!feast unknown",
            "!whm status",
        ] {
            assert!(!is_match_command(command), "{command}");
        }
    }
}
