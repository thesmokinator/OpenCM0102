// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! Engine-wide traits. The database crate implements `DataProvider` over
//! SQLite; the match crate implements `MatchEngine`. Nothing in the UI may
//! depend on a concrete implementation.

use crate::data::{Club, Nation, Player};
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
}

/// Outcome of one simulated fixture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchResult {
    /// Goals scored by the home club.
    pub home_goals: u32,
    /// Goals scored by the away club.
    pub away_goals: u32,
    /// Human-readable match events, e.g. `12' Henry scores!`.
    pub events: Vec<String>,
}

/// A match simulation engine. Implementations must be deterministic for a
/// given `MatchContext::seed`.
pub trait MatchEngine: Send + Sync {
    /// Simulate one fixture.
    fn simulate(&self, ctx: &MatchContext) -> MatchResult;
}
