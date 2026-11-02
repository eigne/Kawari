//! Feast presentation contract. These are fork-owned director variables, not CC fields.
//!
//! The standard DirectorVars payload retains flag/branch zero. Its ten data bytes
//! contain a version marker and an atomic, recipient-specific match snapshot.
use super::feast_match::{Match, Phase, Team};
use super::{instance::Instance, network::{NetworkState, DestinationNetwork}};
use crate::FromServer;
use kawari::ipc::zone::{ServerZoneIpcData, ServerZoneIpcSegment};
use std::time::Instant;

pub const VERSION: u8 = 0xF1;

/// One ordered frame: summary, present participants, then commit/ground medals.
/// Continuation records are consumed by the client module before the stock
/// director sees them; only the summary belongs in native UnionData.
pub fn send(instance: &Instance, rules: &Match, now: f64, network: &mut NetworkState) {
    let Some(director) = instance.directors.first() else { return };
    if director.id.0 != 0x80039c56 { return; }
    for (&actor, player) in &instance.actors {
        if !matches!(player, super::actor::NetworkedActor::Player { .. }) || !player.is_valid() { continue; }
        let mut records = vec![Snapshot::from_match(rules, now, actor.0,
            super::feast_match::respawn_remaining(instance, actor, Instant::now())).data];
        for (&id, p) in rules.players.iter().filter(|(_, p)| p.present) {
            let mut data = [0; 10];
            data[0] = 0xF2;
            data[1] = if p.team == Team::Fang { 1 } else { 2 };
            data[2..6].copy_from_slice(&id.to_le_bytes());
            data[6..8].copy_from_slice(&p.medals.to_le_bytes());
            data[8] = p.job;
            data[9] = u8::from(p.alive);
            records.push(data);
            if let Some(engine) = &instance.feast_whm {
                let mut gauge = [0; 10];
                gauge[0] = 0xF5;
                gauge[1..5].copy_from_slice(&id.to_le_bytes());
                gauge[5] = engine.adrenaline(id).min(100);
                records.push(gauge);
                match engine.party_actions(id) {
                    Ok(actions) => for (slot, (action, remaining)) in actions.into_iter().take(3).enumerate() {
                        let mut cooldown = [0; 10];
                        cooldown[0] = 0xF4;
                        cooldown[1..5].copy_from_slice(&id.to_le_bytes());
                        cooldown[5] = slot as u8;
                        cooldown[6..8].copy_from_slice(&action.to_le_bytes());
                        cooldown[8..10].copy_from_slice(&remaining.to_le_bytes());
                        records.push(cooldown);
                    },
                    Err(error) => tracing::warn!(%error, id, "Feast party cooldown snapshot failed"),
                }
            }
        }
        let mut commit = [0; 10];
        if let Some(engine) = &instance.feast_whm {
            if let Ok(gauge) = engine.job_gauge(actor.0) {
                if gauge.len() == 7 && matches!(gauge[0],19|20|21|22|23|24|25|27|28|30|31|32|33|34|35|37|38|39|40) {
                    let mut data = [0;10]; data[0] = 0xF7;
                    for i in 0..6 { data[i+1] = gauge[i] as u8; }
                    data[7..9].copy_from_slice(&gauge[6].to_le_bytes());
                    records.push(data);
                }
            }
            if let Ok(actions) = engine.hotbar(actor.0) {
                for (base, resolved, cast, recast) in actions {
                    let mut data = [0; 10]; data[0] = 0xF6;
                    data[2..4].copy_from_slice(&base.to_le_bytes());
                    data[4..6].copy_from_slice(&resolved.to_le_bytes());
                    data[6..8].copy_from_slice(&cast.to_le_bytes());
                    data[8..10].copy_from_slice(&recast.to_le_bytes());
                    records.push(data);
                }
            }
        }
        commit[0] = 0xF3;
        let ground = rules.drops.iter().map(|d| u32::from(d.medals)).sum::<u32>().min(u16::MAX as u32) as u16;
        commit[1..3].copy_from_slice(&ground.to_le_bytes());
        records.push(commit);
        for data in records {
            let packet = ServerZoneIpcSegment::new(ServerZoneIpcData::DirectorVars {
                handler_id: director.id, flag: 0, branch: 0, data,
                unk1: 0, unk2: 0, unk3: 0, unk4: 0,
            });
            network.send_to_by_actor_id(actor, FromServer::PacketSegment(packet, actor), DestinationNetwork::ZoneClients);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Snapshot {
    pub data: [u8; 10],
}

impl Snapshot {
    pub fn from_match(rules: &Match, now: f64, actor: u32, respawn_ms: u32) -> Self {
        let phase = match rules.phase {
            Phase::Countdown => 0,
            Phase::Active => 1,
            Phase::SuddenDeath => 2,
            Phase::Finished(Some(Team::Fang)) => 3,
            Phase::Finished(Some(Team::Claw)) => 4,
            Phase::Finished(None) => 5,
        };
        let player = rules.players.get(&actor).filter(|p| p.present);
        let team = match player.map(|p| p.team) {
            Some(Team::Fang) => 1,
            Some(Team::Claw) => 2,
            None => 0,
        };
        let remaining = if matches!(rules.phase, Phase::Finished(_)) {
            0
        } else {
            (rules.deadline - now).max(0.0).ceil().min(u16::MAX as f64) as u16
        };
        let (fang, claw) = rules.scores();
        let mut data = [0; 10];
        data[0] = VERSION;
        data[1] = phase | (team << 3);
        data[2..4].copy_from_slice(&remaining.to_le_bytes());
        data[4..6].copy_from_slice(&fang.to_le_bytes());
        data[6..8].copy_from_slice(&claw.to_le_bytes());
        data[8] = player.map_or(0, |p| p.medals.min(255) as u8);
        data[9] = respawn_ms.div_ceil(1000).min(255) as u8;
        Self { data }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::feast_match::Input;

    #[test]
    fn snapshot_is_personalized_and_uses_integer_medals() {
        let mut rules = Match::new(0.0);
        rules.step(0.0, &[
            Input { id: 1, job: 24, alive: true, position: [0.0; 3] },
            Input { id: 2, job: 24, alive: true, position: [0.0; 3] },
        ]);
        let snapshot = Snapshot::from_match(&rules, 1.2, 1, 1501);
        assert_eq!(snapshot.data, [0xF1, 8, 4, 0, 100, 0, 100, 0, 100, 2]);
        let other = Snapshot::from_match(&rules, 1.2, 2, 0);
        assert_eq!(other.data[1], 16);
        let spectator = Snapshot::from_match(&rules, 1.2, 99, 0);
        assert_eq!(spectator.data[1], 0);
        assert_eq!(spectator.data[8], 0);
    }

    #[test]
    fn results_stop_the_clock_and_expired_respawn_is_zero() {
        let mut rules = Match::new(0.0);
        rules.phase = Phase::Finished(Some(Team::Claw));
        rules.deadline = 999.0;
        let snapshot = Snapshot::from_match(&rules, 1.0, 0, 0);
        assert_eq!(snapshot.data[1], 4);
        assert_eq!(&snapshot.data[2..4], &[0, 0]);
        assert_eq!(snapshot.data[9], 0);
    }
}
