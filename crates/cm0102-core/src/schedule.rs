// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! Fixtures and league calendars: the first real [`DayEventSource`].
//!
//! A [`Schedule`] is the season's dated fixture list. League calendars
//! are generated with the classic circle-method round robin
//! ([`Schedule::league_calendar`]); the exact 01/02 calendar quirks
//! (derby placement, TV moves) are still open research — see
//! `docs/competitions.md` — so generation stays parameterized
//! (first round date, spacing, legs) to absorb them as editable rules.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::calendar::GameDate;
use crate::data::Club;
use crate::simulation::{DayEvents, DayEventSource};
use crate::traits::MatchResult;

/// Byte-87 slots that are minnow buckets, not simulated leagues
/// (distribution verified on the 3.9.60 data — see
/// `docs/competitions.md`). Slot 0 is included defensively: no league
/// uses it, and grouping by it would merge unrelated clubs.
pub const NON_LEAGUE_SLOTS: &[u32] = &[0, 57, 58, 90, 97, 98, 101, 254];

/// Default days between league rounds: one round per week.
pub const DEFAULT_ROUND_SPACING: u32 = 7;

/// Default league format: home and away legs.
pub const DEFAULT_LEGS: u8 = 2;

/// First league round of the season starting in `year`: 18 August, the
/// 01/02 opening weekend. Per-league start dates are still open
/// research; this approximation moves forward year by year until the
/// observed dates land as editable rules.
pub const fn first_round_for_year(year: i32) -> GameDate {
    GameDate::new(year, 8, 18)
}

/// Final score of a played fixture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixtureResult {
    /// Goals scored by the home club.
    pub home_goals: u32,
    /// Goals scored by the away club.
    pub away_goals: u32,
}

/// Points awarded per result. Three for a win is the assumed 01/02
/// default; the value is data so the observed rule replaces it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableRules {
    /// Points for a win.
    pub points_win: u32,
    /// Points for a draw.
    pub points_draw: u32,
}

impl Default for TableRules {
    fn default() -> Self {
        Self {
            points_win: 3,
            points_draw: 1,
        }
    }
}

/// One league-table row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TableRow {
    /// Club id.
    pub club_id: u32,
    /// Games played.
    pub played: u32,
    /// Games won.
    pub won: u32,
    /// Games drawn.
    pub drawn: u32,
    /// Games lost.
    pub lost: u32,
    /// Goals scored.
    pub goals_for: u32,
    /// Goals conceded.
    pub goals_against: u32,
    /// Points.
    pub points: u32,
}

impl TableRow {
    /// Zero row for `club_id`.
    pub fn new(club_id: u32) -> Self {
        Self {
            club_id,
            played: 0,
            won: 0,
            drawn: 0,
            lost: 0,
            goals_for: 0,
            goals_against: 0,
            points: 0,
        }
    }

    /// Goals scored minus goals conceded.
    pub fn goal_difference(self) -> i32 {
        self.goals_for as i32 - self.goals_against as i32
    }
}

/// One dated fixture between two clubs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fixture {
    /// Day the match is played.
    pub date: GameDate,
    /// Home club id.
    pub home_club_id: u32,
    /// Away club id.
    pub away_club_id: u32,
    /// Competition id (club_comp/nation_comp id space).
    pub competition_id: u32,
    /// Final score once the match has been played.
    #[serde(default)]
    pub result: Option<FixtureResult>,
    /// Full match report (events, ratings, stats) once played.
    #[serde(default)]
    pub report: Option<MatchResult>,
}

impl Fixture {
    /// Deterministic match seed from the fixture's identity (FNV-1a
    /// over date, clubs and competition): the same fixture always
    /// seeds the same simulation.
    pub fn seed(&self) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        for word in [
            self.date.year as u64,
            u64::from(self.date.month),
            u64::from(self.date.day),
            u64::from(self.home_club_id),
            u64::from(self.away_club_id),
            u64::from(self.competition_id),
        ] {
            for byte in word.to_le_bytes() {
                hash ^= u64::from(byte);
                hash = hash.wrapping_mul(0x100000001b3);
            }
        }
        hash
    }
}

/// The season's fixture list, kept sorted by date.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Schedule {
    /// Every fixture, sorted by (date, home, away).
    pub fixtures: Vec<Fixture>,
}

impl Schedule {
    /// Build a schedule from fixtures, sorting them by date.
    pub fn new(mut fixtures: Vec<Fixture>) -> Self {
        fixtures.sort_by_key(|fixture| {
            (
                fixture.date,
                fixture.home_club_id,
                fixture.away_club_id,
            )
        });
        Self { fixtures }
    }

    /// Generate a league calendar: every club meets every other club
    /// once per leg, rounds spaced `days_between_rounds` apart from
    /// `first_round`. Home/away swaps on even legs (0-based odd), so a
    /// two-leg calendar is perfectly venue-balanced. Fewer than two
    /// clubs yields an empty schedule.
    pub fn league_calendar(
        competition_id: u32,
        club_ids: &[u32],
        first_round: GameDate,
        days_between_rounds: u32,
        legs: u8,
    ) -> Self {
        let rounds = round_robin_rounds(club_ids);
        let mut fixtures = Vec::new();
        for leg in 0..legs.max(1) {
            for (round_index, round) in rounds.iter().enumerate() {
                let round_number = rounds.len() as u32 * u32::from(leg) + round_index as u32;
                let date = first_round.add_days(round_number * days_between_rounds);
                for &(home, away) in round {
                    let (home_club_id, away_club_id) =
                        if leg % 2 == 1 { (away, home) } else { (home, away) };
                    fixtures.push(Fixture {
                        date,
                        home_club_id,
                        away_club_id,
                        competition_id,
                        result: None,
                        report: None,
                    });
                }
            }
        }
        Self::new(fixtures)
    }

    /// Calendar for the league `managed` plays in: every club sharing
    /// its byte-87 slot, ordered by id for determinism. Empty when the
    /// slot is unknown (stale database) or a minnow bucket. League
    /// membership is the current-season snapshot; promotion and
    /// relegation between seasons are still open.
    pub fn for_club_league(
        managed: &Club,
        clubs: &[Club],
        first_round: GameDate,
        days_between_rounds: u32,
        legs: u8,
    ) -> Self {
        let Some(slot) = managed.league_id else {
            return Self::default();
        };
        if NON_LEAGUE_SLOTS.contains(&slot) {
            return Self::default();
        }
        let mut members: Vec<u32> = clubs
            .iter()
            .filter(|club| club.league_id == Some(slot))
            .map(|club| club.id)
            .collect();
        members.sort_unstable();
        Self::league_calendar(slot, &members, first_round, days_between_rounds, legs)
    }

    /// Every fixture played on `date`.
    pub fn fixtures_on(&self, date: GameDate) -> Vec<&Fixture> {
        self.fixtures
            .iter()
            .filter(|fixture| fixture.date == date)
            .collect()
    }

    /// Earliest unplayed fixture involving `club_id` due on or
    /// before `date`: the match the manager must play next.
    pub fn due_unplayed(&self, club_id: u32, date: GameDate) -> Option<&Fixture> {
        self.fixtures
            .iter()
            .filter(|fixture| {
                fixture.date <= date
                    && fixture.result.is_none()
                    && (fixture.home_club_id == club_id || fixture.away_club_id == club_id)
            })
            .min_by_key(|fixture| fixture.date)
    }

    /// True when `club_id` has an unplayed fixture on `date`.
    pub fn involves_club_on(&self, club_id: u32, date: GameDate) -> bool {
        self.fixtures.iter().any(|fixture| {
            fixture.date == date
                && fixture.result.is_none()
                && (fixture.home_club_id == club_id || fixture.away_club_id == club_id)
        })
    }

    /// Record the final score and report of one fixture. Returns
    /// false when no unplayed fixture matches (wrong day, wrong
    /// clubs, or already played), leaving the schedule untouched.
    pub fn record_result(
        &mut self,
        date: GameDate,
        home_club_id: u32,
        away_club_id: u32,
        result: FixtureResult,
        report: Option<MatchResult>,
    ) -> bool {
        let Some(fixture) = self.fixtures.iter_mut().find(|fixture| {
            fixture.date == date
                && fixture.home_club_id == home_club_id
                && fixture.away_club_id == away_club_id
                && fixture.result.is_none()
        }) else {
            return false;
        };
        fixture.result = Some(result);
        fixture.report = report;
        true
    }

    /// Move one unplayed fixture to `new_date` (a postponement).
    /// Returns false when no unplayed fixture matches, leaving the
    /// schedule untouched.
    pub fn postpone(
        &mut self,
        date: GameDate,
        home_club_id: u32,
        away_club_id: u32,
        new_date: GameDate,
    ) -> bool {
        let Some(fixture) = self.fixtures.iter_mut().find(|fixture| {
            fixture.date == date
                && fixture.home_club_id == home_club_id
                && fixture.away_club_id == away_club_id
                && fixture.result.is_none()
        }) else {
            return false;
        };
        fixture.date = new_date;
        true
    }

    /// Latest fixture date, if the schedule is non-empty.
    pub fn last_date(&self) -> Option<GameDate> {
        self.fixtures.iter().map(|fixture| fixture.date).max()
    }

    /// True when no fixture remains on or after `date`: the season is
    /// over and next year's calendar must be generated.
    pub fn is_exhausted(&self, date: GameDate) -> bool {
        self.last_date().map_or(true, |last| last < date)
    }

    /// League table over `members` from played fixtures. Unplayed
    /// fixtures and fixtures outside the group are ignored; members
    /// without played games get a zero row. Sorted by points, goal
    /// difference, goals scored — the ordering is provisional until
    /// the tiebreak rules are observed (see `docs/competitions.md`).
    pub fn league_table(members: &[u32], fixtures: &[Fixture], rules: &TableRules) -> Vec<TableRow> {
        let mut rows: HashMap<u32, TableRow> = members
            .iter()
            .map(|id| (*id, TableRow::new(*id)))
            .collect();
        for fixture in fixtures {
            let Some(result) = fixture.result else {
                continue;
            };
            if !rows.contains_key(&fixture.home_club_id)
                || !rows.contains_key(&fixture.away_club_id)
            {
                continue;
            }
            for (id, scored, conceded) in [
                (
                    fixture.home_club_id,
                    result.home_goals,
                    result.away_goals,
                ),
                (
                    fixture.away_club_id,
                    result.away_goals,
                    result.home_goals,
                ),
            ] {
                let row = rows.get_mut(&id).expect("membership checked");
                row.played += 1;
                row.goals_for += scored;
                row.goals_against += conceded;
                match scored.cmp(&conceded) {
                    std::cmp::Ordering::Greater => {
                        row.won += 1;
                        row.points += rules.points_win;
                    }
                    std::cmp::Ordering::Less => row.lost += 1,
                    std::cmp::Ordering::Equal => {
                        row.drawn += 1;
                        row.points += rules.points_draw;
                    }
                }
            }
        }
        let mut table: Vec<TableRow> = rows.into_values().collect();
        table.sort_by(|a, b| {
            b.points
                .cmp(&a.points)
                .then(b.goal_difference().cmp(&a.goal_difference()))
                .then(b.goals_for.cmp(&a.goals_for))
                .then(a.club_id.cmp(&b.club_id))
        });
        table
    }

    /// Next season's calendar covering `from`: generates from `from`'s
    /// year forward until fixtures remain (at most 30 seasons ahead),
    /// so deep holidays and old saves land on a playable season.
    /// Returns `None` when the club has no league calendar (unknown
    /// or bucket slot). Membership is the current-season snapshot:
    /// promotion and relegation between seasons are still open.
    pub fn next_season(managed: &Club, clubs: &[Club], from: GameDate) -> Option<Schedule> {
        for year in from.year..from.year + 30 {
            let schedule = Self::for_club_league(
                managed,
                clubs,
                first_round_for_year(year),
                DEFAULT_ROUND_SPACING,
                DEFAULT_LEGS,
            );
            if schedule.fixtures.is_empty() {
                return None;
            }
            if !schedule.is_exhausted(from) {
                return Some(schedule);
            }
        }
        None
    }
}

impl DayEventSource for Schedule {
    /// The managed club's unplayed fixtures stop Continue from their
    /// date on: a match cannot be skipped, only played. Played
    /// fixtures never stop it again. Every other slice of
    /// [`DayEvents`] stays empty until its engine lands.
    fn events_for(&self, date: GameDate, managed_club_id: Option<u32>) -> DayEvents {
        let own_match = managed_club_id.is_some_and(|id| {
            self.fixtures.iter().any(|fixture| {
                fixture.date <= date
                    && fixture.result.is_none()
                    && (fixture.home_club_id == id || fixture.away_club_id == id)
            })
        });
        DayEvents {
            own_match,
            ..DayEvents::default()
        }
    }
}

/// Single round robin as rounds of (home, away) pairs, via the circle
/// method: the first club stays fixed while the rest rotate. Odd club
/// counts get a bye per round. Only the fixed club's venue alternates
/// per round; full home/away balance comes from the swapped second leg.
fn round_robin_rounds(club_ids: &[u32]) -> Vec<Vec<(u32, u32)>> {
    if club_ids.len() < 2 {
        return Vec::new();
    }
    let mut teams: Vec<Option<u32>> = club_ids.iter().copied().map(Some).collect();
    if teams.len() % 2 == 1 {
        teams.push(None);
    }
    let club_count = teams.len();
    let mut rounds = Vec::with_capacity(club_count - 1);
    for round in 0..club_count - 1 {
        let mut pairs = Vec::with_capacity(club_count / 2);
        for i in 0..club_count / 2 {
            if let (Some(a), Some(b)) = (teams[i], teams[club_count - 1 - i]) {
                if i == 0 && round % 2 == 1 {
                    pairs.push((b, a));
                } else {
                    pairs.push((a, b));
                }
            }
        }
        rounds.push(pairs);
        teams[1..].rotate_right(1);
    }
    rounds
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_state::GameState;
    use crate::simulation::{ContinueRules, StopReason, continue_until_event};

    /// Four clubs: three rounds of two matches, every pair once.
    #[test]
    fn round_robin_covers_every_pair_once() {
        let rounds = round_robin_rounds(&[1, 2, 3, 4]);
        assert_eq!(rounds.len(), 3);
        let mut pairs: Vec<(u32, u32)> = rounds.into_iter().flatten().collect();
        for pair in &mut pairs {
            if pair.0 > pair.1 {
                *pair = (pair.1, pair.0);
            }
        }
        pairs.sort_unstable();
        assert_eq!(
            pairs,
            vec![(1, 2), (1, 3), (1, 4), (2, 3), (2, 4), (3, 4)]
        );
    }

    /// Five clubs: five rounds, one bye each, nobody meets itself.
    #[test]
    fn round_robin_handles_odd_club_counts() {
        let rounds = round_robin_rounds(&[1, 2, 3, 4, 5]);
        assert_eq!(rounds.len(), 5);
        assert!(rounds.iter().all(|round| round.len() == 2));
        let mut appearances = [0u32; 6];
        for round in &rounds {
            for &(home, away) in round {
                assert_ne!(home, away);
                appearances[home as usize] += 1;
                appearances[away as usize] += 1;
            }
        }
        assert!(appearances[1..].iter().all(|&count| count == 4));
    }

    /// Two legs: every ordered pair once, venues perfectly balanced.
    #[test]
    fn two_legs_balance_home_and_away() {
        let schedule = Schedule::league_calendar(7, &[1, 2, 3, 4], GameDate::SEASON_START, 7, 2);
        assert_eq!(schedule.fixtures.len(), 12);
        let mut home = [0u32; 5];
        let mut away = [0u32; 5];
        for fixture in &schedule.fixtures {
            home[fixture.home_club_id as usize] += 1;
            away[fixture.away_club_id as usize] += 1;
        }
        for club in 1..=4 {
            assert_eq!(home[club], 3, "club {club} home games");
            assert_eq!(away[club], 3, "club {club} away games");
        }
    }

    /// Rounds start on the given date and keep their spacing across
    /// month boundaries.
    #[test]
    fn calendar_dates_keep_round_spacing() {
        let schedule = Schedule::league_calendar(7, &[1, 2, 3, 4], GameDate::new(2001, 7, 28), 7, 1);
        let dates: Vec<GameDate> = schedule.fixtures.iter().map(|f| f.date).collect();
        assert_eq!(dates[0], GameDate::new(2001, 7, 28));
        assert_eq!(dates[2], GameDate::new(2001, 8, 4));
        assert_eq!(dates[4], GameDate::new(2001, 8, 11));
    }

    /// The source flags exactly the managed club's match days.
    #[test]
    fn source_flags_own_match_days_only() {
        let schedule = Schedule::league_calendar(7, &[1, 2], GameDate::SEASON_START, 7, 1);
        let match_day = GameDate::SEASON_START;
        assert!(schedule.events_for(match_day, Some(1)).own_match);
        assert!(schedule.events_for(match_day, Some(2)).own_match);
        assert!(!schedule.events_for(match_day, Some(99)).own_match);
        assert!(!schedule.events_for(match_day, None).own_match);
        // A match cannot be skipped: unplayed fixtures keep stopping
        // Continue on later days until played.
        assert!(schedule.events_for(GameDate::new(2001, 7, 2), Some(1)).own_match);
        let mut played = schedule.clone();
        let score = FixtureResult {
            home_goals: 1,
            away_goals: 0,
        };
        assert!(played.record_result(match_day, 1, 2, score, None));
        assert!(!played.events_for(GameDate::new(2001, 7, 2), Some(1)).own_match);
    }

    /// Continue with a real schedule stops on the first own match.
    #[test]
    fn continue_stops_on_scheduled_match() {
        let schedule =
            Schedule::league_calendar(7, &[1, 2], GameDate::new(2001, 7, 8), 7, 1);
        let mut state = GameState::new_season();
        state.managed_club_id = Some(1);
        let outcome = continue_until_event(&mut state, &ContinueRules::default(), &schedule);
        assert_eq!(outcome.days_advanced, 7);
        assert_eq!(outcome.stop, StopReason::OwnMatch);
        assert_eq!(state.current_date, GameDate::new(2001, 7, 8));
    }

    /// Fewer than two clubs means no fixtures and no events.
    #[test]
    fn degenerate_calendars_are_empty() {
        let schedule = Schedule::league_calendar(7, &[1], GameDate::SEASON_START, 7, 2);
        assert!(schedule.fixtures.is_empty());
        assert!(!schedule.events_for(GameDate::SEASON_START, Some(1)).own_match);
    }

    /// Schedules round-trip as data for future save persistence.
    #[test]
    fn schedule_round_trips_as_data() {
        let schedule = Schedule::league_calendar(7, &[1, 2, 3, 4], GameDate::SEASON_START, 7, 2);
        let json = serde_json::to_string(&schedule).expect("serializable");
        let restored: Schedule = serde_json::from_str(&json).expect("deserializable");
        assert_eq!(restored, schedule);
    }

    fn club(id: u32, league_id: Option<u32>) -> Club {
        Club {
            id,
            name: format!("Club {id}"),
            league_id,
        }
    }

    /// League calendars group exactly the clubs sharing the slot.
    #[test]
    fn for_club_league_groups_by_slot() {
        let clubs = vec![
            club(1, Some(7)),
            club(2, Some(7)),
            club(3, Some(7)),
            club(4, Some(7)),
            club(5, Some(8)),
        ];
        let schedule = Schedule::for_club_league(
            &clubs[0],
            &clubs,
            GameDate::SEASON_START,
            7,
            1,
        );
        assert_eq!(schedule.fixtures.len(), 6);
        assert!(
            schedule
                .fixtures
                .iter()
                .all(|f| f.home_club_id != 5 && f.away_club_id != 5)
        );
    }

    /// Bucket slots and unknown slots yield no calendar.
    #[test]
    fn for_club_league_rejects_buckets_and_unknown() {
        let clubs = vec![club(1, Some(101)), club(2, Some(101)), club(3, None)];
        assert!(
            Schedule::for_club_league(&clubs[0], &clubs, GameDate::SEASON_START, 7, 2)
                .fixtures
                .is_empty()
        );
        assert!(
            Schedule::for_club_league(&clubs[2], &clubs, GameDate::SEASON_START, 7, 2)
                .fixtures
                .is_empty()
        );
    }

    /// Recording a result sticks; mismatches and replays are refused.
    #[test]
    fn record_result_accepts_one_score_only() {
        let mut schedule =
            Schedule::league_calendar(7, &[1, 2], GameDate::SEASON_START, 7, 1);
        let score = FixtureResult {
            home_goals: 2,
            away_goals: 1,
        };
        assert!(schedule.record_result(GameDate::SEASON_START, 1, 2, score, None));
        assert!(!schedule.record_result(GameDate::SEASON_START, 1, 2, score, None));
        assert!(!schedule.record_result(GameDate::SEASON_START, 2, 1, score, None));
        assert!(!schedule.record_result(GameDate::new(2001, 7, 2), 1, 2, score, None));
        assert_eq!(schedule.fixtures[0].result, Some(score));
    }

    #[test]
    fn postpone_moves_one_unplayed_fixture() {
        let mut schedule =
            Schedule::league_calendar(7, &[1, 2], GameDate::SEASON_START, 7, 1);
        let moved = GameDate::SEASON_START.add_days(7);
        assert!(schedule.postpone(GameDate::SEASON_START, 1, 2, moved));
        assert!(!schedule.postpone(GameDate::SEASON_START, 1, 2, moved));
        assert!(!schedule.postpone(GameDate::SEASON_START, 2, 1, moved));
        let fixture = schedule
            .fixtures
            .iter()
            .find(|fixture| fixture.home_club_id == 1 && fixture.away_club_id == 2)
            .expect("fixture kept");
        assert_eq!(fixture.date, moved);
        assert_eq!(fixture.result, None);
    }

    /// Played fixtures stop flagging match days.
    #[test]
    fn played_fixtures_do_not_stop_continue() {
        let mut schedule =
            Schedule::league_calendar(7, &[1, 2], GameDate::new(2001, 7, 8), 7, 1);
        let match_day = GameDate::new(2001, 7, 8);
        assert!(schedule.events_for(match_day, Some(1)).own_match);
        let fixture = schedule.fixtures_on(match_day)[0];
        let (home, away) = (fixture.home_club_id, fixture.away_club_id);
        assert!(schedule.record_result(
            match_day,
            home,
            away,
            FixtureResult {
                home_goals: 0,
                away_goals: 0,
            },
            None
        ));
        assert!(!schedule.events_for(match_day, Some(1)).own_match);
    }

    /// Exhaustion flips once the date passes the last fixture.
    #[test]
    fn exhaustion_tracks_the_last_fixture() {
        let schedule =
            Schedule::league_calendar(7, &[1, 2], GameDate::SEASON_START, 7, 1);
        assert!(!schedule.is_exhausted(GameDate::SEASON_START));
        assert!(schedule.is_exhausted(GameDate::new(2001, 7, 2)));
        assert!(Schedule::default().is_exhausted(GameDate::SEASON_START));
    }

    /// Rollover covers the requesting date and advances year by year.
    #[test]
    fn next_season_covers_year_after_year() {
        let clubs = vec![
            club(1, Some(7)),
            club(2, Some(7)),
            club(3, Some(7)),
            club(4, Some(7)),
        ];
        let rolled = Schedule::next_season(&clubs[0], &clubs, GameDate::new(2002, 6, 1))
            .expect("calendar");
        assert_eq!(
            rolled.fixtures.iter().map(|f| f.date).min(),
            Some(GameDate::new(2002, 8, 18))
        );
        assert!(!rolled.is_exhausted(GameDate::new(2002, 6, 1)));
        // A date years ahead still lands on a covering calendar.
        let far = Schedule::next_season(&clubs[0], &clubs, GameDate::new(2005, 3, 15))
            .expect("calendar");
        assert!(!far.is_exhausted(GameDate::new(2005, 3, 15)));
        // Bucket clubs get no calendar at all.
        let bucket = vec![club(9, Some(101)), club(10, Some(101))];
        assert_eq!(
            Schedule::next_season(&bucket[0], &bucket, GameDate::new(2002, 6, 1)),
            None
        );
    }

    /// First rounds advance one year per season.
    #[test]
    fn first_round_moves_year_by_year() {
        assert_eq!(first_round_for_year(2001), GameDate::new(2001, 8, 18));
        assert_eq!(first_round_for_year(2005), GameDate::new(2005, 8, 18));
    }

    /// Tables count only played in-group fixtures, sorted by the
    /// provisional Pts/GD/GF order with zero rows for the rest.
    #[test]
    fn league_table_counts_and_sorts() {
        let date = GameDate::SEASON_START;
        let played = |home: u32, away: u32, hs: u32, as_: u32| Fixture {
            date,
            home_club_id: home,
            away_club_id: away,
            competition_id: 7,
            result: Some(FixtureResult {
                home_goals: hs,
                away_goals: as_,
            }),
            report: None,
        };
        let fixtures = vec![
            played(1, 2, 2, 0),
            played(3, 4, 1, 1),
            Fixture {
                date,
                home_club_id: 1,
                away_club_id: 3,
                competition_id: 7,
                result: None,
                report: None,
            },
            played(1, 9, 5, 0),
        ];
        let table = Schedule::league_table(&[1, 2, 3, 4], &fixtures, &TableRules::default());
        let order: Vec<u32> = table.iter().map(|row| row.club_id).collect();
        assert_eq!(order, vec![1, 3, 4, 2]);
        assert_eq!(table[0].points, 3);
        assert_eq!(table[0].goal_difference(), 2);
        assert_eq!(table[1].points, 1);
        assert_eq!(table[3].played, 1);
        assert_eq!(table[3].points, 0);
    }

    /// Seeds are stable per fixture and differ across fixtures.
    #[test]
    fn seeds_are_deterministic_and_distinct() {
        let schedule =
            Schedule::league_calendar(7, &[1, 2, 3, 4], GameDate::SEASON_START, 7, 2);
        let seeds: Vec<u64> = schedule.fixtures.iter().map(Fixture::seed).collect();
        assert_eq!(seeds[0], schedule.fixtures[0].seed());
        let mut unique = seeds.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), seeds.len());
    }
}
