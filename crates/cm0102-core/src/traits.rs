// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! Engine-wide traits. The database crate implements `DataProvider` over
//! SQLite; the match crate implements `MatchEngine`. Nothing in the UI may
//! depend on a concrete implementation.

use crate::data::{Club, Nation, Player};
use crate::tactics::Mentality;
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

/// Team shot counts of one simulated fixture.
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
    /// Goals scored.
    pub goals: u8,
    /// Booked.
    pub yellow: bool,
    /// Dismissed.
    pub red: bool,
}

/// What happened in one match incident.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EventKind {
    /// Open-play goal for the event's side.
    Goal,
    /// Scored penalty.
    Penalty,
    /// Missed penalty.
    MissedPenalty,
    /// Own goal by the event's side (scores for the other side).
    OwnGoal,
    /// Yellow card.
    Yellow,
    /// Dismissal (second yellow or straight red).
    Red,
    /// Injury forcing a substitution.
    Injury,
    /// Planned substitution.
    Sub,
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
}

impl MatchEvent {
    /// One-line display, e.g. `12' Henry` or `45' Smith (og)`.
    pub fn display(&self) -> String {
        let text = match self.kind {
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
}

/// First-half end for reports recorded before it existed.
fn default_first_half_end() -> u32 {
    45
}

/// A match simulation engine. Implementations must be deterministic for a
/// given `MatchContext::seed`.
pub trait MatchEngine: Send + Sync {
    /// Simulate one fixture.
    fn simulate(&self, ctx: &MatchContext) -> MatchResult;
}
