//! The medal match itself: phases, teams, medals, drops and the winner.
//!
//! This file is pure game logic. It knows nothing about actors, packets or
//! Lua; the arena (`arena.rs`) feeds it one `Input` per participant each tick
//! and turns its results (drops, pickups, announcements) into world objects.
//! Times are seconds since the arena started.
use std::collections::BTreeMap;

/// Preparation before the gates open.
pub const COUNTDOWN: f64 = 5.0;
/// Regulation time.
pub const MATCH_TIME: f64 = 360.0;
/// Overtime after a tied regulation; the next medal pickup wins.
pub const SUDDEN_DEATH: f64 = 120.0;
/// Medals every participant starts with.
pub const STARTING_MEDALS: u16 = 100;
/// A team holding this many medals wins immediately.
pub const WINNING_SCORE: u16 = 600;
/// Seconds without a KO before Marked for Culling starts stacking.
pub const CULLING_DELAY: f64 = 90.0;
/// Seconds between Marked for Culling stacks.
pub const CULLING_INTERVAL: f64 = 15.0;
// The official guide does not specify these two values. Prototype tuning.
const DROP_LIFETIME: f64 = 60.0;
const PICKUP_RADIUS: f32 = 2.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Team {
    Fang,
    Claw,
}

impl Team {
    pub fn name(self) -> &'static str {
        match self {
            Self::Fang => "fang",
            Self::Claw => "claw",
        }
    }
}

/// How many medals a job can carry. Tanks carry the most, healers the fewest.
/// `None` means the job did not exist in the Feast and cannot join a match.
pub fn medal_limit(job: u8) -> Option<u16> {
    match job {
        19 | 21 | 32 | 37 => Some(200),
        20 | 22 | 30 | 34 | 39 | 41 => Some(175),
        23 | 25 | 27 | 31 | 35 | 38 | 42 => Some(150),
        24 | 28 | 33 | 40 => Some(125),
        _ => None,
    }
}

/// Maximum HP in the arena, by role: tanks 20,000, healers and melee
/// 17,500, physical ranged 16,000, casters 15,000. Every player of a role
/// has the same HP, whatever their level and gear.
pub fn max_hp(job: u8) -> u32 {
    match job {
        19 | 21 | 32 | 37 => 20_000,
        24 | 28 | 33 | 40 => 17_500,
        20 | 22 | 30 | 34 | 39 | 41 => 17_500,
        23 | 31 | 38 => 16_000,
        _ => 15_000,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    /// Preparation: players are jailed, teams and jobs can still change.
    Countdown,
    Active,
    SuddenDeath,
    /// The result, `None` for a draw. A new match needs `!feast restart`.
    Finished(Option<Team>),
}

#[derive(Clone, Debug)]
pub struct Player {
    pub team: Team,
    pub job: u8,
    pub medals: u16,
    pub position: [f32; 3],
    pub alive: bool,
    /// False while the player is disconnected or zoning.
    pub present: bool,
}

/// One participant's state this tick, as seen by the world server.
#[derive(Clone, Debug)]
pub struct Input {
    pub id: u32,
    pub job: u8,
    pub position: [f32; 3],
    pub alive: bool,
}

/// A pile of medals on the ground.
#[derive(Clone, Debug)]
pub struct Drop {
    pub id: u32,
    pub owner: u32,
    pub team: Team,
    pub medals: u16,
    pub position: [f32; 3],
    /// After this, unclaimed medals return to their owner.
    pub expires: f64,
    /// Dropped by disconnecting; never expires, returns when the owner reconnects.
    pub abandoned: bool,
}

/// A pickup that happened this tick, so the arena can play its animation.
#[derive(Clone, Debug)]
pub struct PickupVisual {
    pub drop_id: u32,
    pub medals: u16,
    pub position: [f32; 3],
}

#[derive(Debug)]
pub struct Match {
    pub phase: Phase,
    /// End of the current phase.
    pub deadline: f64,
    pub players: BTreeMap<u32, Player>,
    pub drops: Vec<Drop>,
    next_drop: u32,
    last_clock: u32,
    /// Chat notices for all participants, drained by the arena.
    pub announcements: Vec<String>,
    /// Pickups from the last `step`.
    pub pickups: Vec<PickupVisual>,
    last_knockout: f64,
}

impl Match {
    pub fn new(now: f64) -> Self {
        Self {
            phase: Phase::Countdown,
            deadline: now + COUNTDOWN,
            players: BTreeMap::new(),
            drops: Vec::new(),
            next_drop: 1,
            last_clock: u32::MAX,
            announcements: Vec::new(),
            pickups: Vec::new(),
            last_knockout: now + COUNTDOWN,
        }
    }

    /// Medals held by (Fangs, Claws). Medals on the ground count for nobody.
    pub fn scores(&self) -> (u16, u16) {
        self.players
            .values()
            .fold((0, 0), |(f, c), p| match p.team {
                Team::Fang => (f + p.medals, c),
                Team::Claw => (f, c + p.medals),
            })
    }

    /// Marked for Culling stacks: none until 90 seconds pass without a KO,
    /// then one more every 15 seconds, up to five. Sudden death starts at one
    /// immediately and ignores further KOs.
    pub fn culling_stacks(&self, now: f64) -> u8 {
        let first = match self.phase {
            Phase::Active => self.last_knockout + CULLING_DELAY,
            Phase::SuddenDeath => self.deadline - SUDDEN_DEATH,
            _ => return 0,
        };
        if now < first {
            0
        } else {
            (1.0 + ((now - first) / CULLING_INTERVAL).floor()).min(5.0) as u8
        }
    }

    /// Whether this participant may fight: the match is running and they still
    /// have the job they entered with (jobs lock when the gates open).
    pub fn active(&self, id: u32, job: u8) -> bool {
        matches!(self.phase, Phase::Active | Phase::SuddenDeath)
            && self
                .players
                .get(&id)
                .is_some_and(|p| p.present && p.job == job)
    }

    /// Medals a player can still pick up. Their own piles on the ground count
    /// against their limit, so dying and walking back cannot exceed it.
    pub fn pickup_space(&self, id: u32) -> u16 {
        let Some(p) = self.players.get(&id) else {
            return 0;
        };
        let reserved: u16 = self
            .drops
            .iter()
            .filter(|d| d.owner == id)
            .map(|d| d.medals)
            .sum();
        medal_limit(p.job)
            .unwrap_or(0)
            .saturating_sub(p.medals + reserved)
    }

    /// Whether `input` could pick up `drop` right now (used for the pile's look).
    pub fn can_collect(&self, input: &Input, drop: &Drop) -> bool {
        self.active(input.id, input.job)
            && self
                .players
                .get(&input.id)
                .is_some_and(|p| p.alive && p.team != drop.team)
            && self.pickup_space(input.id) > 0
    }

    /// One line for `!feast status` and the periodic clock announcements.
    pub fn status(&self, now: f64) -> String {
        let (f, c) = self.scores();
        let state = match self.phase {
            Phase::Countdown => "Preparation",
            Phase::Active => "Match",
            Phase::SuddenDeath => "Sudden death",
            Phase::Finished(Some(Team::Fang)) => "Fangs win",
            Phase::Finished(Some(Team::Claw)) => "Claws win",
            Phase::Finished(None) => "Draw",
        };
        let seconds = self.seconds_left(now);
        format!(
            "Feast: {state} {:02}:{:02} | Fangs {f} - Claws {c}",
            seconds / 60,
            seconds % 60
        )
    }

    /// Whole seconds left in the current phase; zero once finished.
    pub fn seconds_left(&self, now: f64) -> u32 {
        if matches!(self.phase, Phase::Finished(_)) {
            0
        } else {
            (self.deadline - now).max(0.0).ceil() as u32
        }
    }

    /// Seconds the native duty timer should show: preparation plus the match,
    /// then the time left in the match or sudden death. A finished match
    /// shows a full round, since the next one starts from preparation.
    pub fn duty_time_left(&self, now: f64) -> u32 {
        let left = match self.phase {
            Phase::Countdown => self.deadline - now + MATCH_TIME,
            Phase::Active | Phase::SuddenDeath => self.deadline - now,
            Phase::Finished(_) => COUNTDOWN + MATCH_TIME,
        };
        left.max(0.0).ceil() as u32
    }

    fn finish(&mut self, winner: Option<Team>, now: f64) {
        self.phase = Phase::Finished(winner);
        self.announcements.push(self.status(now));
        self.drops.clear();
    }

    /// A KO drops half of a player's medals (rounded up); leaving drops all.
    fn drop_medals(&mut self, id: u32, all: bool, now: f64) {
        let p = self.players.get_mut(&id).unwrap();
        let count = if all { p.medals } else { p.medals.div_ceil(2) };
        if count == 0 {
            return;
        }
        p.medals -= count;
        self.drops.push(Drop {
            id: self.next_drop,
            owner: id,
            team: p.team,
            medals: count,
            position: p.position,
            expires: now + DROP_LIFETIME,
            abandoned: all,
        });
        self.next_drop += 1;
        self.announcements.push(format!(
            "{} player {id} dropped {count} medals.",
            p.team.name()
        ));
    }

    /// Advances the match to `now`. `inputs` lists every participant in the
    /// arena; anyone missing is treated as disconnected.
    pub fn step(&mut self, now: f64, inputs: &[Input]) {
        self.pickups.clear();
        if matches!(self.phase, Phase::Finished(_)) {
            return;
        }
        // Close the scoring window before processing movement/deaths from this tick.
        if self.phase == Phase::Active && now >= self.deadline {
            let (f, c) = self.scores();
            if f != c {
                self.finish(Some(if f > c { Team::Fang } else { Team::Claw }), now);
                return;
            }
            self.phase = Phase::SuddenDeath;
            self.deadline += SUDDEN_DEATH;
            self.announcements
                .push("Sudden death: the next medal pickup wins!".into());
        }
        if self.phase == Phase::SuddenDeath && now >= self.deadline {
            self.finish(None, now);
            return;
        }
        let inputs: BTreeMap<_, _> = inputs.iter().map(|p| (p.id, p)).collect();
        if self.phase == Phase::Countdown {
            self.join_during_countdown(&inputs, now);
        }
        self.update_players(&inputs, now);
        if self.phase == Phase::Countdown {
            self.clock(now);
            return;
        }
        self.return_expired_drops(now);
        if let Some(team) = self.collect_drops(&inputs) {
            self.finish(Some(team), now);
            return;
        }
        let (fang, claw) = self.scores();
        if fang >= WINNING_SCORE || claw >= WINNING_SCORE {
            let winner = if fang >= WINNING_SCORE {
                Team::Fang
            } else {
                Team::Claw
            };
            self.finish(Some(winner), now);
            return;
        }
        self.clock(now);
    }

    /// New arrivals join the smaller team (Fangs on a tie), up to four a side.
    /// Players who leave during the countdown lose their place.
    fn join_during_countdown(&mut self, inputs: &BTreeMap<u32, &Input>, now: f64) {
        for (&id, input) in inputs {
            if self.players.contains_key(&id) || medal_limit(input.job).is_none() {
                continue;
            }
            let fangs = self
                .players
                .values()
                .filter(|p| p.team == Team::Fang)
                .count();
            let claws = self.players.len() - fangs;
            if fangs + claws == 8 {
                continue;
            }
            let team = if fangs <= claws {
                Team::Fang
            } else {
                Team::Claw
            };
            self.players.insert(
                id,
                Player {
                    team,
                    job: input.job,
                    medals: STARTING_MEDALS,
                    position: input.position,
                    alive: input.alive,
                    present: true,
                },
            );
            self.announcements.push(format!(
                "Player {id} joined {} ({STARTING_MEDALS} medals, cap {}).",
                team.name(),
                medal_limit(input.job).unwrap()
            ));
        }
        self.players.retain(|id, _| inputs.contains_key(id));
        if now >= self.deadline {
            self.phase = Phase::Active;
            self.deadline += MATCH_TIME;
            self.announcements
                .push("Feast begins! Six minutes remaining.".into());
        }
    }

    /// Copies positions/health from the inputs and drops medals on KOs and
    /// disconnects. Reconnecting restores medals from the player's own
    /// disconnect drop if nobody has picked them up.
    fn update_players(&mut self, inputs: &BTreeMap<u32, &Input>, now: f64) {
        let ids: Vec<_> = self.players.keys().copied().collect();
        for id in ids {
            let input = inputs.get(&id);
            let p = self.players.get_mut(&id).unwrap();
            let was_present = p.present;
            let was_alive = p.alive;
            p.present = input.is_some();
            if let Some(input) = input {
                p.position = input.position;
                p.alive = input.alive;
                if self.phase == Phase::Countdown && medal_limit(input.job).is_some() {
                    p.job = input.job;
                }
            }
            if self.phase == Phase::Countdown {
                continue;
            }
            if was_present && !p.present {
                self.drop_medals(id, true, now);
            } else if was_present && was_alive && !p.alive {
                self.last_knockout = now;
                self.drop_medals(id, false, now);
            }
            // Rejoining restores only unclaimed disconnect drops, not ordinary KO drops.
            if !was_present && input.is_some() {
                self.drops.retain(|drop| {
                    if drop.owner == id && drop.abandoned {
                        self.players.get_mut(&id).unwrap().medals += drop.medals;
                        false
                    } else {
                        true
                    }
                });
            }
        }
    }

    fn return_expired_drops(&mut self, now: f64) {
        self.drops.retain(|drop| {
            if !drop.abandoned && now >= drop.expires {
                self.players.get_mut(&drop.owner).unwrap().medals += drop.medals;
                false
            } else {
                true
            }
        });
    }

    /// Living enemies standing on a pile take as many medals as they have
    /// room for. Returns the winning team if this happened in sudden death.
    fn collect_drops(&mut self, inputs: &BTreeMap<u32, &Input>) -> Option<Team> {
        let mut winner = None;
        'drops: for index in 0..self.drops.len() {
            let ids: Vec<_> = self.players.keys().copied().collect();
            for id in ids {
                let p = &self.players[&id];
                let drop = &self.drops[index];
                if drop.medals == 0
                    || !p.present
                    || !p.alive
                    || p.team == drop.team
                    || inputs.get(&id).is_none_or(|input| input.job != p.job)
                {
                    continue;
                }
                let distance: f32 = p
                    .position
                    .iter()
                    .zip(drop.position)
                    .map(|(a, b)| (a - b).powi(2))
                    .sum();
                if !distance.is_finite() || distance > PICKUP_RADIUS * PICKUP_RADIUS {
                    continue;
                }
                let amount = self.pickup_space(id).min(drop.medals);
                if amount == 0 {
                    continue;
                }
                self.pickups.push(PickupVisual {
                    drop_id: drop.id,
                    medals: drop.medals,
                    position: drop.position,
                });
                self.drops[index].medals -= amount;
                let p = self.players.get_mut(&id).unwrap();
                p.medals += amount;
                self.announcements.push(format!(
                    "{} player {id} picked up {amount} medals (holding {}).",
                    p.team.name(),
                    p.medals
                ));
                if self.phase == Phase::SuddenDeath {
                    winner = Some(p.team);
                    break 'drops;
                }
            }
        }
        self.drops.retain(|d| d.medals > 0);
        winner
    }

    /// Announces the clock every 30 seconds and for the last ten.
    fn clock(&mut self, now: f64) {
        let remaining = (self.deadline - now).max(0.0).ceil() as u32;
        if self.last_clock != remaining && (remaining <= 10 || remaining.is_multiple_of(30)) {
            self.announcements.push(self.status(now));
        }
        self.last_clock = remaining;
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    pub fn inputs() -> Vec<Input> {
        vec![
            Input {
                id: 1,
                job: 24,
                position: [0.0; 3],
                alive: true,
            },
            Input {
                id: 2,
                job: 24,
                position: [10.0, 0.0, 0.0],
                alive: true,
            },
        ]
    }

    /// A two-player match that has just left preparation.
    pub fn active() -> (Match, Vec<Input>) {
        let mut m = Match::new(0.0);
        let p = inputs();
        m.step(0.0, &p);
        m.step(COUNTDOWN, &p);
        (m, p)
    }

    /// Medals held plus medals on the ground; never changes during a match.
    fn total(m: &Match) -> u16 {
        let (f, c) = m.scores();
        f + c + m.drops.iter().map(|d| d.medals).sum::<u16>()
    }

    #[test]
    fn culling_resets_on_ko_but_not_during_sudden_death() {
        let (mut m, mut p) = active();
        assert_eq!(m.culling_stacks(COUNTDOWN + 89.999), 0);
        assert_eq!(m.culling_stacks(COUNTDOWN + 90.0), 1);
        assert_eq!(m.culling_stacks(COUNTDOWN + 105.0), 2);
        assert_eq!(m.culling_stacks(COUNTDOWN + 150.0), 5);
        p[1].alive = false;
        m.step(COUNTDOWN + 151.0, &p);
        assert_eq!(m.culling_stacks(COUNTDOWN + 151.0), 0);
        assert_eq!(m.culling_stacks(COUNTDOWN + 241.0), 1);
        // Let unclaimed medals return, so regulation ends tied.
        m.step(COUNTDOWN + 220.0, &p);
        let sudden = COUNTDOWN + MATCH_TIME;
        m.step(sudden, &p);
        assert_eq!(m.phase, Phase::SuddenDeath);
        assert_eq!(m.culling_stacks(sudden), 1);
        p[0].alive = false;
        m.step(sudden + 20.0, &p);
        assert_eq!(m.culling_stacks(sudden + 20.0), 2);
        assert_eq!(m.culling_stacks(sudden + 60.0), 5);
        m.step(sudden + SUDDEN_DEATH, &p);
        assert_eq!(m.culling_stacks(sudden + SUDDEN_DEATH), 0);
    }

    #[test]
    fn disconnect_drop_remains_after_ordinary_expiry() {
        let (mut m, p) = active();
        m.step(COUNTDOWN + 1.0, &p[1..]);
        m.step(COUNTDOWN + DROP_LIFETIME + 2.0, &p[1..]);
        assert_eq!(m.drops.len(), 1);
        assert_eq!(m.players[&1].medals, 0);
        m.step(COUNTDOWN + DROP_LIFETIME + 3.0, &p);
        assert!(m.drops.is_empty());
        assert_eq!(m.players[&1].medals, 100);
    }

    #[test]
    fn six_hundred_medals_ends_regulation_immediately() {
        let (mut m, p) = active();
        m.players.get_mut(&1).unwrap().medals = 599;
        m.step(COUNTDOWN + 1.0, &p);
        assert_eq!(m.phase, Phase::Active);
        m.players.get_mut(&1).unwrap().medals = 600;
        m.step(COUNTDOWN + 2.0, &p);
        assert_eq!(m.phase, Phase::Finished(Some(Team::Fang)));
    }

    #[test]
    fn countdown_balances_and_locks_roster_and_jobs() {
        let mut m = Match::new(0.0);
        let mut p = inputs();
        m.step(0.0, &p);
        assert_eq!(m.scores(), (100, 100));
        assert!(!m.active(1, 24));
        m.step(COUNTDOWN - 1.0, &p);
        assert_eq!(m.phase, Phase::Countdown);
        m.step(COUNTDOWN, &p);
        assert!(m.active(1, 24));
        assert!(!m.active(1, 19));
        p.push(Input {
            id: 3,
            job: 19,
            position: [0.0; 3],
            alive: true,
        });
        m.step(COUNTDOWN + 1.0, &p);
        assert!(!m.players.contains_key(&3));
        assert_eq!(m.deadline, COUNTDOWN + MATCH_TIME);
        assert_eq!(
            (
                medal_limit(19),
                medal_limit(20),
                medal_limit(23),
                medal_limit(24)
            ),
            (Some(200), Some(175), Some(150), Some(125))
        );
    }

    #[test]
    fn max_hp_follows_role() {
        // Paladin, White Mage, Samurai, Bard, Black Mage.
        let hp = [19, 24, 34, 23, 25].map(max_hp);
        assert_eq!(hp, [20_000, 17_500, 17_500, 16_000, 15_000]);
    }

    #[test]
    fn native_duty_time_follows_the_match_clock() {
        let mut m = Match::new(0.0);
        let p = inputs();
        m.step(0.0, &p);
        assert_eq!(m.duty_time_left(10.0), (COUNTDOWN + MATCH_TIME) as u32 - 10);
        m.step(COUNTDOWN, &p);
        assert_eq!(m.phase, Phase::Active);
        assert_eq!(m.duty_time_left(COUNTDOWN + 100.0), MATCH_TIME as u32 - 100);
        m.phase = Phase::SuddenDeath;
        m.deadline += SUDDEN_DEATH;
        assert_eq!(
            m.duty_time_left(COUNTDOWN + MATCH_TIME),
            SUDDEN_DEATH as u32
        );
        m.phase = Phase::Finished(None);
        assert_eq!(m.duty_time_left(1000.0), (COUNTDOWN + MATCH_TIME) as u32);
    }

    #[test]
    fn death_once_enemy_pickup_cap_overflow_and_expiry() {
        let (mut m, mut p) = active();
        p[0].alive = false;
        m.step(COUNTDOWN + 1.0, &p);
        assert_eq!(m.players[&1].medals, 50);
        assert_eq!(m.drops[0].medals, 50);
        m.step(COUNTDOWN + 2.0, &p);
        assert_eq!(m.drops.len(), 1);
        p[1].position = [0.0; 3];
        m.step(COUNTDOWN + 3.0, &p);
        assert_eq!(m.players[&2].medals, 125);
        assert_eq!(m.drops[0].medals, 25);
        assert_eq!(total(&m), 200);
        m.step(COUNTDOWN + DROP_LIFETIME + 1.0, &p);
        assert!(m.drops.is_empty());
        assert_eq!(m.scores(), (75, 125));
        assert_eq!(total(&m), 200);
    }

    #[test]
    fn own_drops_reserve_capacity_and_cannot_be_reclaimed_by_walking() {
        let (mut m, mut p) = active();
        p[0].alive = false;
        p[1].alive = false;
        m.step(COUNTDOWN + 1.0, &p);
        p[0].alive = true;
        p[1].alive = true;
        m.step(COUNTDOWN + 2.0, &p);
        assert_eq!(m.scores(), (50, 50));
        p[0].position = [10.0, 0.0, 0.0];
        m.step(COUNTDOWN + 3.0, &p);
        // 50 held + 50 reserved leaves only 25 capacity.
        assert_eq!(m.players[&1].medals, 75);
        assert_eq!(m.drops.iter().find(|d| d.owner == 2).unwrap().medals, 25);
        assert_eq!(total(&m), 200);
    }

    #[test]
    fn match_end_freezes_scores_before_late_pickup() {
        let (mut m, mut p) = active();
        p[0].alive = false;
        m.step(COUNTDOWN + MATCH_TIME - 1.0, &p);
        p[1].position = [0.0; 3];
        m.step(COUNTDOWN + MATCH_TIME, &p);
        assert_eq!(m.phase, Phase::Finished(Some(Team::Claw)));
        assert_eq!(m.scores(), (50, 100));
        m.step(COUNTDOWN + MATCH_TIME + 80.0, &p);
        assert_eq!(m.scores(), (50, 100));
    }

    #[test]
    fn sudden_death_pickup_wins_and_timeout_draws() {
        let (mut m, mut p) = active();
        m.step(COUNTDOWN + MATCH_TIME, &p);
        assert_eq!(m.phase, Phase::SuddenDeath);
        p[0].alive = false;
        m.step(COUNTDOWN + MATCH_TIME + 1.0, &p);
        assert_eq!(m.phase, Phase::SuddenDeath);
        p[1].position = [0.0; 3];
        m.step(COUNTDOWN + MATCH_TIME + 2.0, &p);
        assert_eq!(m.phase, Phase::Finished(Some(Team::Claw)));
        let (mut m, p) = active();
        m.step(COUNTDOWN + MATCH_TIME + SUDDEN_DEATH, &p);
        assert_eq!(m.phase, Phase::Finished(None));
    }

    #[test]
    fn disconnect_drops_all_and_reconnect_only_restores_unclaimed_medals() {
        let (mut m, mut p) = active();
        m.step(COUNTDOWN + 1.0, &p[1..]);
        assert_eq!(m.players[&1].medals, 0);
        assert_eq!(m.drops[0].medals, 100);
        p[1].position = [0.0; 3];
        m.step(COUNTDOWN + 2.0, &p[1..]);
        assert_eq!(m.drops[0].medals, 75);
        m.step(COUNTDOWN + 3.0, &p);
        assert_eq!(m.scores(), (75, 125));
        assert!(m.drops.is_empty());
        assert_eq!(total(&m), 200);
    }

    #[test]
    fn medal_eligibility_follows_team_job_life_and_capacity() {
        let (mut m, mut inputs) = active();
        let drop = Drop {
            id: 1,
            owner: 2,
            team: Team::Claw,
            medals: 63,
            position: [0.0; 3],
            expires: 100.0,
            abandoned: false,
        };
        assert!(m.can_collect(&inputs[0], &drop));
        assert!(!m.can_collect(&inputs[1], &drop));
        m.drops.push(Drop {
            owner: 1,
            team: Team::Fang,
            medals: 25,
            ..drop.clone()
        });
        assert!(!m.can_collect(&inputs[0], &drop)); // 100 held + 25 reserved
        m.drops.clear();
        assert!(m.can_collect(&inputs[0], &drop));
        inputs[0].job = 19;
        assert!(!m.can_collect(&inputs[0], &drop));
        inputs[0].job = 24;
        m.players.get_mut(&1).unwrap().alive = false;
        assert!(!m.can_collect(&inputs[0], &drop));
    }
}
