// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! Engine-wide traits. The database crate implements `DataProvider` over
//! SQLite; the match crate implements `MatchEngine`. Nothing in the UI may
//! depend on a concrete implementation.

use crate::data::{Club, Nation, Official, Player};
use crate::tactics::Mentality;
use crate::weather::MatchWeather;
use crate::GameDate;

/// Error type shared by all providers. Kept intentionally small: storage
/// crates map their own errors into these variants.
#[derive(Debug)]
pub enum CoreError {
    /// The requested entity does not exist.
    NotFound(String),
    /// The backing store failed.
    Storage(String),
}

impl std::fmt::Display for CoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CoreError::NotFound(what) => write!(f, "not found: {what}"),
            CoreError::Storage(what) => write!(f, "storage error: {what}"),
        }
    }
}

impl std::error::Error for CoreError {}

/// Result alias for provider operations.
pub type Result<T> = std::result::Result<T, CoreError>;

/// Read access to the game database.
pub trait DataProvider: Send + Sync {
    /// Fetch one player by id.
    fn get_player(&self, id: u32) -> Result<Player>;
    /// Fetch every player registered to a club.
    fn get_players_by_club(&self, club_id: u32) -> Result<Vec<Player>>;
    /// Fetch one club by id.
    fn get_club(&self, id: u32) -> Result<Club>;
    /// List every club, ordered by name.
    fn list_clubs(&self) -> Result<Vec<Club>>;
    /// Count players currently in the database.
    fn count_players(&self) -> Result<u32>;
    /// List every nation, ordered by name.
    fn list_nations(&self) -> Result<Vec<Nation>>;
    /// List every official of a nation, ordered by name.
    fn list_officials_by_nation(&self, nation_id: u32) -> Result<Vec<Official>>;
    /// Reputation of a club competition (league slot), if known.
    fn competition_reputation(&self, competition_id: u32) -> Result<Option<i16>>;
}

/// One player as the match engine sees them: identity, display
/// name, and the 54 attributes in [`ATTRIBUTE_NAMES`](crate::ATTRIBUTE_NAMES)
/// order (empty when unknown, in which case average ratings apply).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MatchPlayer {
    /// Stable identifier from the source data.
    pub id: u32,
    /// Display name for match events.
    pub name: String,
    /// Current ability, used when attributes are missing.
    pub current_ability: i16,
    /// The 54 attributes (0–20), possibly empty.
    pub attributes: Vec<u8>,
    /// Match fitness, 0–100. Fully fit players perform at their
    /// rated attributes; exhausted ones perform at half.
    pub condition: u8,
    /// Confidence and form, typically −3..=+3 with 0 neutral.
    /// Lifts (or drags) the whole side's chance creation.
    pub morale: i8,
}

impl MatchPlayer {
    /// Raw attribute (0–20) at `ATTRIBUTE_NAMES` position `index`;
    /// missing entries read as an average 10.
    pub fn attribute(&self, index: usize) -> u8 {
        self.attributes.get(index).copied().unwrap_or(10)
    }

    /// Attribute on the engine's internal 0–200 scale: the raw value
    /// times ten, matching the observed normalization.
    pub fn skill(&self, index: usize) -> u32 {
        u32::from(self.attribute(index)) * 10
    }

    /// Fitness multiplier for attribute rolls: 1.0 when fully fit,
    /// sinking to 0.5 when exhausted.
    pub fn condition_scale(&self) -> f32 {
        0.5 + 0.5 * f32::from(self.condition.min(100)) / 100.0
    }
}

/// Everything a match engine needs to simulate one fixture.
#[derive(Debug, Clone)]
pub struct MatchContext {
    /// Home club id.
    pub home_club_id: u32,
    /// Away club id.
    pub away_club_id: u32,
    /// Date the fixture is played on.
    pub date: GameDate,
    /// Deterministic seed for this fixture.
    pub seed: u64,
    /// Average current ability of the home starting eleven.
    pub home_avg_ca: f32,
    /// Average current ability of the away starting eleven.
    pub away_avg_ca: f32,
    /// Home starting eleven, shirt order.
    pub home_players: Vec<MatchPlayer>,
    /// Away starting eleven, shirt order.
    pub away_players: Vec<MatchPlayer>,
    /// Home named substitutes.
    pub home_subs: Vec<MatchPlayer>,
    /// Away named substitutes.
    pub away_subs: Vec<MatchPlayer>,
    /// Home team mentality: attacking sides create more, defensive
    /// sides create less.
    pub home_mentality: Mentality,
    /// Away team mentality.
    pub away_mentality: Mentality,
    /// Home side keeps men behind the ball, damping the opponent's
    /// chance creation.
    pub home_men_behind_ball: bool,
    /// Away side keeps men behind the ball.
    pub away_men_behind_ball: bool,
    /// Play extra time and a shootout when level (cup ties).
    pub extra_time_if_level: bool,
    /// Match weather, rolled from the venue's climate zone.
    pub weather: MatchWeather,
    /// Assigned referee: name shown, strictness rules bookings.
    pub referee: Referee,
}

/// One match injury with its layoff in days.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MatchInjury {
    /// Stable identifier from the source data.
    pub player_id: u32,
    /// Display name.
    pub name: String,
    /// True for the home side.
    pub home: bool,
    /// Days sidelined after the match.
    pub days_out: u32,
}

/// Penalty-shootout score of a cup tie.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Shootout {
    /// Home penalties scored.
    pub home_scored: u8,
    /// Away penalties scored.
    pub away_scored: u8,
    /// True when the home side won the shootout.
    pub home_wins: bool,
}

/// One side's ambient counters for a single minute of play.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize,
)]
pub struct SideMinute {
    /// Shots attempted.
    pub shots: u16,
    /// Shots on target.
    pub on_target: u16,
    /// Corners taken.
    pub corners: u16,
    /// Free kicks taken.
    pub free_kicks: u16,
    /// Throw-ins taken.
    pub throw_ins: u16,
    /// Offsides flagged.
    pub offsides: u16,
    /// Passes completed.
    pub passes: u16,
    /// Tackles won.
    pub tackles: u16,
    /// Headers won.
    pub headers: u16,
}

/// Both sides' counters for one minute of play, in match order.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize,
)]
pub struct MinuteStats {
    /// Home side's counters.
    pub home: SideMinute,
    /// Away side's counters.
    pub away: SideMinute,
}

/// Team counts of one simulated fixture: the ten-counter block the
/// original game's match screens show, live and full time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct MatchStats {
    /// Home shots (all).
    pub home_shots: u32,
    /// Away shots (all).
    pub away_shots: u32,
    /// Home shots on target (including goals).
    pub home_on_target: u32,
    /// Away shots on target (including goals).
    pub away_on_target: u32,
    /// Home corners.
    #[serde(default)]
    pub home_corners: u32,
    /// Away corners.
    #[serde(default)]
    pub away_corners: u32,
    /// Home free kicks.
    #[serde(default)]
    pub home_free_kicks: u32,
    /// Away free kicks.
    #[serde(default)]
    pub away_free_kicks: u32,
    /// Home throw-ins.
    #[serde(default)]
    pub home_throw_ins: u32,
    /// Away throw-ins.
    #[serde(default)]
    pub away_throw_ins: u32,
    /// Home offsides.
    #[serde(default)]
    pub home_offsides: u32,
    /// Away offsides.
    #[serde(default)]
    pub away_offsides: u32,
    /// Home passes completed.
    #[serde(default)]
    pub home_passes: u32,
    /// Away passes completed.
    #[serde(default)]
    pub away_passes: u32,
    /// Home tackles won.
    #[serde(default)]
    pub home_tackles: u32,
    /// Away tackles won.
    #[serde(default)]
    pub away_tackles: u32,
    /// Home headers won.
    #[serde(default)]
    pub home_headers: u32,
    /// Away headers won.
    #[serde(default)]
    pub away_headers: u32,
}

/// One player's match record: rating and key events.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PlayerRating {
    /// Stable identifier from the source data.
    pub player_id: u32,
    /// Display name.
    pub name: String,
    /// True for the home side.
    pub home: bool,
    /// Rating in deci-points (74 = 7.4).
    pub rating: u8,
    /// Minutes played.
    pub minutes: u8,
    /// Minute they came on (0 for starters).
    #[serde(default)]
    pub on_minute: u8,
    /// Goals scored.
    pub goals: u8,
    /// Booked.
    pub yellow: bool,
    /// Dismissed.
    pub red: bool,
    /// Shots attempted.
    #[serde(default)]
    pub shots: u16,
    /// Shots on target.
    #[serde(default)]
    pub on_target: u16,
    /// Tackles won.
    #[serde(default)]
    pub tackles: u16,
    /// Passes completed.
    #[serde(default)]
    pub passes: u16,
    /// Headers won.
    #[serde(default)]
    pub headers: u16,
    /// Interceptions made.
    #[serde(default)]
    pub interceptions: u16,
    /// Times flagged offside.
    #[serde(default)]
    pub offsides: u16,
}

/// What happened in one match incident.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EventKind {
    /// Kick-off.
    Kickoff,
    /// Open-play goal for the event's side.
    Goal,
    /// Scored penalty.
    Penalty,
    /// Missed penalty.
    MissedPenalty,
    /// Own goal by the event's side (scores for the other side).
    OwnGoal,
    /// Shot saved by the keeper.
    ShotSaved,
    /// Shot off target.
    ShotOffTarget,
    /// Shot against the frame, staying out.
    ShotWoodwork,
    /// Shot blocked by a defender.
    ShotBlocked,
    /// Yellow card.
    Yellow,
    /// Dismissal (second yellow or straight red).
    Red,
    /// Injury forcing a substitution.
    Injury,
    /// Planned substitution.
    Sub,
    /// Half-time whistle.
    HalfTime,
    /// Full-time whistle.
    FullTime,
    /// Shootout verdict after extra time.
    Shootout,
}

/// One minute-stamped incident of a simulated fixture, in
/// chronological order.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MatchEvent {
    /// Minute the incident happened (extra time runs past 90).
    pub minute: u32,
    /// True when the protagonist plays for the home side.
    pub home: bool,
    /// What happened.
    pub kind: EventKind,
    /// Protagonist name (scorer, booked player, substitute on).
    pub player: String,
    /// Secondary detail: player replaced, injury layoff, shootout
    /// scoreline.
    pub other: Option<String>,
    /// Commentary event id for cfg packs (original numbering, so a
    /// player's own file resolves). Absent in older reports.
    #[serde(default)]
    pub commentary_id: Option<u16>,
    /// Shot distance in yards, filling the `<d>` token.
    #[serde(default)]
    pub distance_yards: Option<u16>,
}

impl MatchEvent {
    /// One-line display, e.g. `12' Henry` or `45' Smith (og)`.
    pub fn display(&self) -> String {
        let text = match self.kind {
            EventKind::Kickoff => "Kick off".to_string(),
            EventKind::HalfTime => "Half time".to_string(),
            EventKind::FullTime => "Full time".to_string(),
            EventKind::ShotSaved => format!("{} (saved)", self.player),
            EventKind::ShotOffTarget => format!("{} (off target)", self.player),
            EventKind::ShotWoodwork => format!("{} (woodwork)", self.player),
            EventKind::ShotBlocked => format!("{} (blocked)", self.player),
            EventKind::Goal => self.player.clone(),
            EventKind::Penalty => format!("{} (pen)", self.player),
            EventKind::MissedPenalty => format!("{} (m/pen)", self.player),
            EventKind::OwnGoal => {
                if self.player.is_empty() {
                    "Own goal".to_string()
                } else {
                    format!("{} (og)", self.player)
                }
            }
            EventKind::Yellow => format!("{} (y)", self.player),
            EventKind::Red => format!("{} sent off", self.player),
            EventKind::Injury => format!("{} injured", self.player),
            EventKind::Sub => match &self.other {
                Some(off) => format!("{} on for {off}", self.player),
                None => format!("{} on", self.player),
            },
            EventKind::Shootout => match &self.other {
                Some(score) => format!("{} wins {score} on penalties", self.player),
                None => format!("{} win on penalties", self.player),
            },
        };
        format!("{}' {text}", self.minute)
    }

    /// Goals this incident adds to (home, away).
    pub fn goals(&self) -> (u32, u32) {
        match self.kind {
            EventKind::Goal | EventKind::Penalty => {
                if self.home {
                    (1, 0)
                } else {
                    (0, 1)
                }
            }
            EventKind::OwnGoal => {
                if self.home {
                    (0, 1)
                } else {
                    (1, 0)
                }
            }
            _ => (0, 0),
        }
    }
}

/// Outcome of one simulated fixture.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MatchResult {
    /// Goals scored by the home club.
    pub home_goals: u32,
    /// Goals scored by the away club.
    pub away_goals: u32,
    /// Match incidents in chronological order.
    pub events: Vec<MatchEvent>,
    /// Shot counts for both clubs.
    pub stats: MatchStats,
    /// Per-player ratings for everyone who took the pitch.
    pub ratings: Vec<PlayerRating>,
    /// Shootout score, when a cup tie needed one.
    pub shootout: Option<Shootout>,
    /// Match injuries with layoffs.
    pub injuries: Vec<MatchInjury>,
    /// Minute the first half ended (45 plus stoppage). Older
    /// reports without it read as 45.
    #[serde(default = "default_first_half_end")]
    pub first_half_end: u32,
    /// Per-minute counters in match order: the live view sums the
    /// prefix up to the current minute. Empty in older reports.
    #[serde(default)]
    pub timeline: Vec<MinuteStats>,
    /// Match weather. Older reports without it read as calm.
    #[serde(default)]
    pub weather: MatchWeather,
    /// Assigned referee. Missing in older reports.
    #[serde(default)]
    pub referee: Option<Referee>,
    /// Attendance. Missing in older reports.
    #[serde(default)]
    pub attendance: Option<u32>,
}

/// Neutral strictness on the 0-20 scale: the booking rate pivot.
pub const NEUTRAL_STRICTNESS: u8 = 10;

/// The referee assigned to a fixture.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Referee {
    /// Display name.
    pub name: String,
    /// Strictness 0-20 (10 is neutral): strict referees book
    /// more, lenient ones wave play on.
    pub strictness: u8,
}

impl Default for Referee {
    /// An unnamed neutral referee, used until the data assigns one.
    fn default() -> Self {
        Self {
            name: String::new(),
            strictness: NEUTRAL_STRICTNESS,
        }
    }
}

impl Referee {
    /// True for the fallback unnamed referee.
    pub fn is_unnamed(&self) -> bool {
        self.name.is_empty()
    }
}

/// Strictness for an official id when the database predates the
/// field. Our reconstruction (editable): stable per referee, in
/// the observed mid band, so the same official always behaves
/// the same way.
pub fn strictness_for_official(id: u32) -> u8 {
    const FLOOR: u32 = 8;
    const SPREAD: u32 = 8;
    (FLOOR + (id.wrapping_mul(2_654_435_761) >> 8) % SPREAD) as u8
}

/// First-half end for reports recorded before it existed.
fn default_first_half_end() -> u32 {
    45
}

impl MatchResult {
    /// Team counts up to and including `minute` (1-based): the live
    /// view's running block. Minutes past the timeline clamp.
    pub fn stats_upto(&self, minute: u32) -> MatchStats {
        let mut stats = MatchStats::default();
        for played in self.timeline.iter().take(minute as usize) {
            stats.home_shots += u32::from(played.home.shots);
            stats.away_shots += u32::from(played.away.shots);
            stats.home_on_target += u32::from(played.home.on_target);
            stats.away_on_target += u32::from(played.away.on_target);
            stats.home_corners += u32::from(played.home.corners);
            stats.away_corners += u32::from(played.away.corners);
            stats.home_free_kicks += u32::from(played.home.free_kicks);
            stats.away_free_kicks += u32::from(played.away.free_kicks);
            stats.home_throw_ins += u32::from(played.home.throw_ins);
            stats.away_throw_ins += u32::from(played.away.throw_ins);
            stats.home_offsides += u32::from(played.home.offsides);
            stats.away_offsides += u32::from(played.away.offsides);
            stats.home_passes += u32::from(played.home.passes);
            stats.away_passes += u32::from(played.away.passes);
            stats.home_tackles += u32::from(played.home.tackles);
            stats.away_tackles += u32::from(played.away.tackles);
            stats.home_headers += u32::from(played.home.headers);
            stats.away_headers += u32::from(played.away.headers);
        }
        stats
    }
}

/// A match simulation engine. Implementations must be deterministic for a
/// given `MatchContext::seed`.
pub trait MatchEngine: Send + Sync {
    /// Simulate one fixture.
    fn simulate(&self, ctx: &MatchContext) -> MatchResult;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strictness_stays_in_range_and_stable() {
        for id in 0..500u32 {
            let strictness = strictness_for_official(id);
            assert!((8..=15).contains(&strictness), "{id}: {strictness}");
            assert_eq!(strictness, strictness_for_official(id));
        }
        // The spread is used, not a single value.
        let distinct: std::collections::HashSet<u8> =
            (0..500u32).map(strictness_for_official).collect();
        assert!(distinct.len() > 5, "{}", distinct.len());
    }

    #[test]
    fn legacy_reports_gain_empty_referee_and_crowd() {
        let report: MatchResult = serde_json::from_str(
            r#"{"home_goals":1,"away_goals":0,"events":[],"stats":{"home_shots":0,"away_shots":0,"home_on_target":0,"away_on_target":0,"home_corners":0,"away_corners":0,"home_free_kicks":0,"away_free_kicks":0,"home_throw_ins":0,"away_throw_ins":0,"home_offsides":0,"away_offsides":0,"home_passes":0,"away_passes":0,"home_tackles":0,"away_tackles":0,"home_headers":0,"away_headers":0},"ratings":[],"shootout":null,"injuries":[],"first_half_end":45,"timeline":[]}"#,
        )
        .expect("legacy report parses");
        assert_eq!(report.referee, None);
        assert_eq!(report.attendance, None);
    }
}
