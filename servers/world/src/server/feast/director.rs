//! Match snapshots for the HUD plugin, sent through the native duty director.
//!
//! The Feast's own director packets are gone from today's client, so the
//! server reuses the generic `DirectorVars` packet of the arena's duty
//! director (handler 0x80039C56). Each packet carries ten data bytes; we
//! fill them with our own records, identified by their first byte:
//!
//! | Byte 0 | Record        | Bytes 1-9 |
//! |--------|---------------|-----------|
//! | `F1`   | summary       | phase \| team<<3, seconds left (u16), Fang score (u16), Claw score (u16), own medals, respawn seconds |
//! | `F2`   | participant   | team (1 Fang, 2 Claw), actor ID (u32), medals (u16), job, alive |
//! | `F5`   | adrenaline    | actor ID (u32), adrenaline 0-100 |
//! | `F4`   | party action  | actor ID (u32), slot 0-2, action ID (u16), tenths of a second left (u16) |
//! | `F7`   | job gauge     | job, six gauge bytes, timer (u16); only for the recipient |
//! | `F6`   | hotbar action | 0, base ID (u16), current ID (u16), cast ms (u16), recast ms (u16); recipient only |
//! | `F3`   | commit        | medals on the ground (u16) |
//!
//! A frame is `F1`, any records, then `F3`. The HUD (DirectorFrame.cs) only
//! uses a frame once its `F3` arrives, so it never shows half an update. The
//! HUD hides `F2`-`F7` from the game; the game only ever sees `F1`.
use super::rules::{Match, Phase, Team};
use crate::{
    FromServer,
    server::{
        actor::NetworkedActor,
        instance::Instance,
        network::{DestinationNetwork, NetworkState},
    },
};
use kawari::ipc::zone::{ServerZoneIpcData, ServerZoneIpcSegment};
use std::time::Instant;

/// The arena duty's director (InstanceContent 40022).
pub const DIRECTOR_ID: u32 = 0x80039C56;

const SUMMARY: u8 = 0xF1;
const PARTICIPANT: u8 = 0xF2;
const COMMIT: u8 = 0xF3;
const PARTY_ACTION: u8 = 0xF4;
const ADRENALINE: u8 = 0xF5;
const HOTBAR_ACTION: u8 = 0xF6;
const JOB_GAUGE: u8 = 0xF7;

type Record = [u8; 10];

/// Sends every player in the instance their own frame.
pub fn send(instance: &Instance, rules: &Match, now: f64, network: &mut NetworkState) {
    let Some(director) = instance.directors.first() else {
        return;
    };
    // Exploration mode has no duty director, so the HUD falls back to chat.
    if director.id.0 != DIRECTOR_ID {
        return;
    }
    for (&actor, player) in &instance.actors {
        if !matches!(player, NetworkedActor::Player { .. }) || !player.is_valid() {
            continue;
        }
        let respawn_ms = super::arena::respawn_remaining(instance, actor, Instant::now());
        for data in frame(instance, rules, now, actor.0, respawn_ms) {
            let packet = ServerZoneIpcSegment::new(ServerZoneIpcData::DirectorVars {
                handler_id: director.id,
                flag: 0,
                branch: 0,
                data,
                unk1: 0,
                unk2: 0,
                unk3: 0,
                unk4: 0,
            });
            network.send_to_by_actor_id(
                actor,
                FromServer::PacketSegment(packet, actor),
                DestinationNetwork::ZoneClients,
            );
        }
    }
}

/// One recipient's frame, in order.
fn frame(
    instance: &Instance,
    rules: &Match,
    now: f64,
    recipient: u32,
    respawn_ms: u32,
) -> Vec<Record> {
    let engine = instance.feast.combat.as_ref();
    let mut records = vec![summary(rules, now, recipient, respawn_ms)];
    for (&id, p) in rules.players.iter().filter(|(_, p)| p.present) {
        let mut data = [0; 10];
        data[0] = PARTICIPANT;
        data[1] = team_byte(Some(p.team));
        data[2..6].copy_from_slice(&id.to_le_bytes());
        data[6..8].copy_from_slice(&p.medals.to_le_bytes());
        data[8] = p.job;
        data[9] = u8::from(p.alive);
        records.push(data);
        let Some(engine) = engine else {
            continue;
        };
        let mut data = [0; 10];
        data[0] = ADRENALINE;
        data[1..5].copy_from_slice(&id.to_le_bytes());
        data[5] = engine.adrenaline(id).min(100);
        records.push(data);
        match engine.party_actions(id) {
            Ok(actions) => {
                for (slot, (action, remaining)) in actions.into_iter().take(3).enumerate() {
                    let mut data = [0; 10];
                    data[0] = PARTY_ACTION;
                    data[1..5].copy_from_slice(&id.to_le_bytes());
                    data[5] = slot as u8;
                    data[6..8].copy_from_slice(&action.to_le_bytes());
                    data[8..10].copy_from_slice(&remaining.to_le_bytes());
                    records.push(data);
                }
            }
            Err(error) => tracing::warn!(%error, id, "Feast party cooldown snapshot failed"),
        }
    }
    if let Some(engine) = engine {
        if let Ok(gauge) = engine.job_gauge(recipient)
            && gauge.len() == 7
            && super::is_feast_job(gauge[0] as u8)
        {
            let mut data = [0; 10];
            data[0] = JOB_GAUGE;
            for i in 0..6 {
                data[i + 1] = gauge[i] as u8;
            }
            data[7..9].copy_from_slice(&gauge[6].to_le_bytes());
            records.push(data);
        }
        if let Ok(actions) = engine.hotbar(recipient) {
            for (base, resolved, cast, recast) in actions {
                let mut data = [0; 10];
                data[0] = HOTBAR_ACTION;
                data[2..4].copy_from_slice(&base.to_le_bytes());
                data[4..6].copy_from_slice(&resolved.to_le_bytes());
                data[6..8].copy_from_slice(&cast.to_le_bytes());
                data[8..10].copy_from_slice(&recast.to_le_bytes());
                records.push(data);
            }
        }
    }
    let mut commit = [0; 10];
    commit[0] = COMMIT;
    let ground: u32 = rules.drops.iter().map(|d| u32::from(d.medals)).sum();
    commit[1..3].copy_from_slice(&(ground.min(u16::MAX as u32) as u16).to_le_bytes());
    records.push(commit);
    records
}

/// 0 for spectators, 1 for Fangs, 2 for Claws.
fn team_byte(team: Option<Team>) -> u8 {
    match team {
        Some(Team::Fang) => 1,
        Some(Team::Claw) => 2,
        None => 0,
    }
}

/// The `F1` record: the match as `recipient` sees it.
fn summary(rules: &Match, now: f64, recipient: u32, respawn_ms: u32) -> Record {
    let phase = match rules.phase {
        Phase::Countdown => 0,
        Phase::Active => 1,
        Phase::SuddenDeath => 2,
        Phase::Finished(Some(Team::Fang)) => 3,
        Phase::Finished(Some(Team::Claw)) => 4,
        Phase::Finished(None) => 5,
    };
    let player = rules.players.get(&recipient).filter(|p| p.present);
    let remaining = rules.seconds_left(now).min(u16::MAX as u32) as u16;
    let (fang, claw) = rules.scores();
    let mut data = [0; 10];
    data[0] = SUMMARY;
    data[1] = phase | (team_byte(player.map(|p| p.team)) << 3);
    data[2..4].copy_from_slice(&remaining.to_le_bytes());
    data[4..6].copy_from_slice(&fang.to_le_bytes());
    data[6..8].copy_from_slice(&claw.to_le_bytes());
    data[8] = player.map_or(0, |p| p.medals.min(255) as u8);
    data[9] = respawn_ms.div_ceil(1000).min(255) as u8;
    data
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::feast::rules::Input;

    #[test]
    fn summary_is_personalized_and_uses_integer_medals() {
        let mut rules = Match::new(0.0);
        rules.step(
            0.0,
            &[
                Input {
                    id: 1,
                    job: 24,
                    alive: true,
                    position: [0.0; 3],
                },
                Input {
                    id: 2,
                    job: 24,
                    alive: true,
                    position: [0.0; 3],
                },
            ],
        );
        // This byte fixture is shared with the HUD tests (feast-hud-tests).
        assert_eq!(
            summary(&rules, 1.2, 1, 1501),
            [0xF1, 8, 4, 0, 100, 0, 100, 0, 100, 2]
        );
        assert_eq!(summary(&rules, 1.2, 2, 0)[1], 16);
        let spectator = summary(&rules, 1.2, 99, 0);
        assert_eq!(spectator[1], 0);
        assert_eq!(spectator[8], 0);
    }

    #[test]
    fn results_stop_the_clock_and_expired_respawn_is_zero() {
        let mut rules = Match::new(0.0);
        rules.phase = Phase::Finished(Some(Team::Claw));
        rules.deadline = 999.0;
        let data = summary(&rules, 1.0, 0, 0);
        assert_eq!(data[1], 4);
        assert_eq!(&data[2..4], &[0, 0]);
        assert_eq!(data[9], 0);
    }
}
