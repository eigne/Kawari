//! The custom-match lobby: which team each player is on, who hosts, and the
//! parties that go with the teams.
//!
//! A party that enters the arena together is a custom match. Its first half
//! (in party list order) becomes the Fangs and the rest the Claws, its leader
//! becomes the host, and preparation waits until the host uses `!feast
//! start`. The host can rearrange the teams with `!feast swap` before each
//! match. Players who enter without a party join the smaller team and their
//! match starts on its own.
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use kawari::common::ObjectId;
use parking_lot::Mutex;

use super::rules::{Phase, Team};
use crate::{
    common::PartyUpdateTargets,
    server::{
        actor::NetworkedActor,
        instance::Instance,
        network::NetworkState,
        party::{disband_party, form_party, get_party_id_from_actor_id},
    },
};

#[derive(Debug, Default)]
pub struct Lobby {
    /// The player who may arrange the teams and start matches.
    pub host: Option<ObjectId>,
    /// Each player's team. Kept across the matches of this instance.
    pub teams: BTreeMap<u32, Team>,
    /// Preparation does not run out until the host uses `!feast start`.
    pub waiting: bool,
    /// Players who have loaded into the arena at least once.
    seen: BTreeSet<u32>,
}

/// The players loaded into the arena, by ID.
fn players(instance: &Instance) -> Vec<ObjectId> {
    let mut players: Vec<_> = instance
        .actors
        .iter()
        .filter(|(_, actor)| {
            matches!(actor, NetworkedActor::Player { .. })
                && actor.get_common_spawn().max_health_points != 0
        })
        .map(|(id, _)| *id)
        .collect();
    players.sort_by_key(|id| id.0);
    players
}

/// The players with a team, Fangs first. `!feast teams` numbers them in
/// this order, from 1.
fn roster(instance: &Instance) -> Vec<(ObjectId, Team)> {
    let teams = &instance.feast.lobby.teams;
    let mut roster: Vec<_> = players(instance)
        .into_iter()
        .filter_map(|id| Some((id, *teams.get(&id.0)?)))
        .collect();
    roster.sort_by_key(|(id, team)| (*team == Team::Claw, id.0));
    roster
}

fn name(instance: &Instance, id: ObjectId) -> String {
    instance
        .find_actor(id)
        .map(|a| a.get_common_spawn().name.clone())
        .unwrap_or_default()
}

fn notify_all(instance: &Instance, network: &Arc<Mutex<NetworkState>>, message: &str) {
    for id in players(instance) {
        super::notice(network, id, message.into());
    }
}

/// Gives newly arrived players a team and keeps a host. Runs before each
/// arena tick.
pub fn plan(instance: &mut Instance, network: &Arc<Mutex<NetworkState>>) {
    let present = players(instance);
    let mut lobby = std::mem::take(&mut instance.feast.lobby);
    {
        let network = network.lock();
        for id in &present {
            if lobby.teams.contains_key(&id.0) {
                continue;
            }
            let Some(party) =
                get_party_id_from_actor_id(&network, *id).and_then(|p| network.parties.get(&p))
            else {
                continue;
            };
            // The whole party gets its teams now, including members who are
            // still loading.
            let members: Vec<_> = party.members.iter().filter(|m| m.is_valid()).collect();
            if members.len() < 2 {
                continue;
            }
            let fangs = members.len().div_ceil(2);
            for (index, member) in members.iter().enumerate() {
                let team = if index < fangs {
                    Team::Fang
                } else {
                    Team::Claw
                };
                lobby.teams.entry(member.actor_id.0).or_insert(team);
            }
            lobby.host.get_or_insert(party.leader_id);
            lobby.waiting = true;
        }
    }
    // A host who has not loaded yet keeps the role; one who left loses it.
    if lobby
        .host
        .is_none_or(|host| lobby.seen.contains(&host.0) && !present.contains(&host))
    {
        lobby.host = present.first().copied();
    }
    let arrivals: Vec<_> = present
        .iter()
        .filter(|id| lobby.seen.insert(id.0))
        .copied()
        .collect();
    let waiting = lobby.waiting;
    let host = lobby.host;
    instance.feast.lobby = lobby;
    if waiting {
        let host = host.map(|h| name(instance, h)).unwrap_or_default();
        for id in arrivals {
            super::notice(
                network,
                id,
                format!(
                    "Custom match hosted by {host}. !feast teams shows the teams; the host changes them with !feast swap and begins with !feast start."
                ),
            );
        }
    }
}

/// Starts waiting for the host again, for the next match of a custom match.
pub fn rematch(instance: &mut Instance, network: &Arc<Mutex<NetworkState>>) {
    let present = players(instance);
    let lobby = &mut instance.feast.lobby;
    lobby.teams.retain(|id, _| present.contains(&ObjectId(*id)));
    if present.len() < 2 {
        return;
    }
    lobby.waiting = true;
    notify_all(
        instance,
        network,
        "Next match: the host can change teams with !feast swap and begins with !feast start.",
    );
}

/// Makes the parties match the teams: one party per team during a match,
/// because the client treats party members as allies whatever their
/// battalion, and one party for everyone while the host arranges the teams
/// and after the result, so the group stays together when it leaves.
pub fn sync_parties(instance: &Instance, network: &Arc<Mutex<NetworkState>>, phase: Phase) {
    let lobby = &instance.feast.lobby;
    let together =
        matches!(phase, Phase::Finished(_)) || phase == Phase::Countdown && lobby.waiting;
    let roster = roster(instance);
    let mut network = network.lock();
    let spanning: Vec<_> = network
        .parties
        .iter()
        .filter(|(_, party)| {
            let mut teams = party
                .members
                .iter()
                .filter_map(|m| lobby.teams.get(&m.actor_id.0));
            !together
                && teams
                    .next()
                    .is_some_and(|first| teams.any(|team| team != first))
        })
        .map(|(id, _)| *id)
        .collect();
    for party_id in spanning {
        disband(&mut network, party_id);
    }
    let groups = if together {
        vec![roster.iter().map(|(id, _)| *id).collect::<Vec<_>>()]
    } else {
        [Team::Fang, Team::Claw]
            .iter()
            .map(|team| {
                roster
                    .iter()
                    .filter(|(_, t)| t == team)
                    .map(|(id, _)| *id)
                    .collect()
            })
            .collect()
    };
    for members in groups {
        if members.len() < 2 {
            continue;
        }
        let parties: BTreeSet<_> = members
            .iter()
            .map(|id| get_party_id_from_actor_id(&network, *id))
            .collect();
        if parties.len() == 1 && parties.first() != Some(&None) {
            continue;
        }
        for party_id in parties.into_iter().flatten() {
            disband(&mut network, party_id);
        }
        let leader = lobby
            .host
            .filter(|host| members.contains(host))
            .unwrap_or(members[0]);
        let party_id = form_party(&mut network, instance, leader, &members);
        tracing::info!(?party_id, members = members.len(), "Formed a Feast party");
    }
}

fn disband(network: &mut NetworkState, party_id: u64) {
    let Some(party) = network.parties.get(&party_id) else {
        return;
    };
    let leader = party
        .get_member_by_actor_id(party.leader_id)
        .unwrap_or_default();
    disband_party(
        network,
        party_id,
        PartyUpdateTargets {
            execute_account_id: leader.account_id,
            execute_content_id: leader.content_id,
            execute_name: leader.name,
            ..Default::default()
        },
    );
}

fn phase(instance: &Instance) -> Phase {
    instance
        .feast
        .arena
        .as_ref()
        .map_or(Phase::Countdown, |a| a.rules.phase)
}

/// Whether `id` is the host; tells them if not.
pub fn host_only(instance: &Instance, network: &Arc<Mutex<NetworkState>>, id: ObjectId) -> bool {
    let host = instance.feast.lobby.host;
    if host.is_none_or(|host| host == id) {
        return true;
    }
    super::notice(
        network,
        id,
        format!(
            "Only the host ({}) can do that.",
            name(instance, host.unwrap())
        ),
    );
    false
}

fn describe(instance: &Instance) -> String {
    let roster = roster(instance);
    let list = |team: Team| {
        let names: Vec<_> = roster
            .iter()
            .enumerate()
            .filter(|(_, (_, t))| *t == team)
            .map(|(index, (id, _))| format!("{} {}", index + 1, name(instance, *id)))
            .collect();
        if names.is_empty() {
            "nobody".into()
        } else {
            names.join(", ")
        }
    };
    format!("Fangs: {} | Claws: {}", list(Team::Fang), list(Team::Claw))
}

/// `!feast teams`: the numbered teams and the host.
pub fn teams(instance: &Instance, network: &Arc<Mutex<NetworkState>>, id: ObjectId) {
    super::notice(network, id, describe(instance));
    let lobby = &instance.feast.lobby;
    let host = lobby.host.map(|h| name(instance, h)).unwrap_or_default();
    let state = if lobby.waiting {
        "waiting for the host's !feast start"
    } else {
        match phase(instance) {
            Phase::Finished(_) => "the host begins the next match with !feast restart",
            Phase::Countdown => "the match is about to begin",
            _ => "match in progress",
        }
    };
    super::notice(network, id, format!("Host: {host}; {state}."));
}

/// `!feast swap <number> [number]`: the host moves one player to the other
/// team, or exchanges two players. Numbers are those of `!feast teams`.
pub fn swap(
    instance: &mut Instance,
    network: &Arc<Mutex<NetworkState>>,
    id: ObjectId,
    numbers: &[&str],
) {
    if !host_only(instance, network, id) {
        return;
    }
    if matches!(phase(instance), Phase::Active | Phase::SuddenDeath) {
        super::notice(
            network,
            id,
            "Teams can only change before a match or after its result.".into(),
        );
        return;
    }
    let roster = roster(instance);
    let picked: Option<Vec<_>> = numbers
        .iter()
        .map(|n| {
            let index = n.parse::<usize>().ok()?.checked_sub(1)?;
            roster.get(index).copied()
        })
        .collect();
    let size = |team: Team| roster.iter().filter(|(_, t)| *t == team).count();
    let moves: Vec<(ObjectId, Team)> = match picked.as_deref() {
        Some([(player, team)]) if size(team.other()) < 4 => vec![(*player, team.other())],
        Some([_]) => {
            super::notice(network, id, "That team is already full.".into());
            return;
        }
        Some([(a, team_a), (b, team_b)]) if team_a != team_b => {
            vec![(*a, *team_b), (*b, *team_a)]
        }
        Some([_, _]) => {
            super::notice(network, id, "Those two are on the same team.".into());
            return;
        }
        _ => {
            super::notice(
                network,
                id,
                "Use !feast swap <number> to move one player, or !feast swap <number> <number> to exchange two. !feast teams shows the numbers.".into(),
            );
            return;
        }
    };
    for (player, team) in moves {
        instance.feast.lobby.teams.insert(player.0, team);
    }
    let message = describe(instance);
    notify_all(instance, network, &message);
}

/// `!feast start`: the host ends the wait; preparation then runs as usual.
pub fn start(instance: &mut Instance, network: &Arc<Mutex<NetworkState>>, id: ObjectId) {
    if !host_only(instance, network, id) {
        return;
    }
    if !instance.feast.lobby.waiting {
        let message = match phase(instance) {
            Phase::Finished(_) => "Begin the next match with !feast restart.",
            _ => "The match has already been started.",
        };
        super::notice(network, id, message.into());
        return;
    }
    instance.feast.lobby.waiting = false;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::{
        feast::{TERRITORY, arena, commands::command},
        party::{Party, PartyMember},
    };

    /// Four Bards in the arena, in one party listed as 3, 1, 4, 2 and led by 3.
    fn party_of_four() -> (Instance, Arc<Mutex<NetworkState>>) {
        let mut instance = Instance::default();
        instance.zone.id = TERRITORY;
        let mut party = Party {
            leader_id: ObjectId(3),
            ..Default::default()
        };
        for id in [3, 1, 4, 2] {
            instance.insert_empty_actor(ObjectId(id));
            let p = instance
                .find_actor_mut(ObjectId(id))
                .unwrap()
                .get_common_spawn_mut();
            p.class_job = 23;
            p.health_points = 10000;
            p.max_health_points = 10000;
            party.members.push(PartyMember {
                actor_id: ObjectId(id),
                ..Default::default()
            });
        }
        let mut network = NetworkState::default();
        network.parties.insert(1, party);
        (instance, Arc::new(Mutex::new(network)))
    }

    fn team(instance: &Instance, id: u32) -> Team {
        instance.feast.arena.as_ref().unwrap().rules.players[&id].team
    }

    fn party_members(network: &Arc<Mutex<NetworkState>>) -> Vec<Vec<u32>> {
        let network = network.lock();
        let mut parties: Vec<Vec<u32>> = network
            .parties
            .values()
            .map(|p| p.members.iter().map(|m| m.actor_id.0).collect())
            .collect();
        parties.sort();
        parties
    }

    #[test]
    fn party_order_sets_the_teams_and_the_leader_hosts() {
        let (mut instance, network) = party_of_four();
        arena::tick(&mut instance, &network);
        assert_eq!(instance.feast.lobby.host, Some(ObjectId(3)));
        assert!(instance.feast.lobby.waiting);
        assert_eq!(
            [1, 2, 3, 4].map(|id| team(&instance, id)),
            [Team::Fang, Team::Claw, Team::Fang, Team::Claw]
        );
        // The group stays one party while the host arranges the teams.
        assert_eq!(party_members(&network), [[3, 1, 4, 2]]);
    }

    #[test]
    fn only_the_host_swaps_and_starts() {
        let (mut instance, network) = party_of_four();
        arena::tick(&mut instance, &network);
        // `!feast teams` numbers them 1 3 | 2 4.
        command(&mut instance, &network, ObjectId(1), "swap 1 3");
        command(&mut instance, &network, ObjectId(1), "start");
        arena::tick(&mut instance, &network);
        assert_eq!(team(&instance, 1), Team::Fang);
        assert!(instance.feast.lobby.waiting);

        command(&mut instance, &network, ObjectId(3), "swap 1 3");
        arena::tick(&mut instance, &network);
        assert_eq!(
            [1, 2, 3, 4].map(|id| team(&instance, id)),
            [Team::Claw, Team::Fang, Team::Fang, Team::Claw]
        );
        // Moving one player: now 2 3 | 1 4, and number 4 goes to the Fangs.
        command(&mut instance, &network, ObjectId(3), "swap 4");
        arena::tick(&mut instance, &network);
        assert_eq!(team(&instance, 4), Team::Fang);

        command(&mut instance, &network, ObjectId(3), "start");
        assert!(!instance.feast.lobby.waiting);
    }

    #[test]
    fn teams_get_their_own_parties_and_reunite_after_the_result() {
        let (mut instance, network) = party_of_four();
        arena::tick(&mut instance, &network);
        command(&mut instance, &network, ObjectId(3), "start");
        arena::tick(&mut instance, &network);
        assert_eq!(party_members(&network), [[1, 3], [2, 4]]);
        let leaders: BTreeSet<_> = network
            .lock()
            .parties
            .values()
            .map(|p| p.leader_id.0)
            .collect();
        assert_eq!(leaders, BTreeSet::from([2, 3]));

        instance.feast.arena.as_mut().unwrap().rules.phase = Phase::Finished(None);
        arena::tick(&mut instance, &network);
        assert_eq!(party_members(&network), [[1, 3, 2, 4]]);
        assert_eq!(
            network.lock().parties.values().next().unwrap().leader_id,
            ObjectId(3)
        );

        // The next match waits for the host again, with the same teams.
        command(&mut instance, &network, ObjectId(3), "restart");
        assert!(instance.feast.lobby.waiting);
        assert_eq!(team(&instance, 2), Team::Claw);
        assert_eq!(party_members(&network), [[1, 3, 2, 4]]);
    }
}
