// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! Core domain types and traits for OpenCM0102.
//!
//! This crate has no I/O: it defines the data model, the game calendar and
//! the traits (`DataProvider`, `MatchEngine`) that the database, match and
//! simulation crates implement. Keeping it dependency-light lets the UI and
//! the engines evolve independently.

pub mod calendar;
pub mod data;
pub mod game_state;
pub mod schedule;
pub mod simulation;
pub mod tactics;
pub mod traits;

pub use calendar::GameDate;
pub use data::{Club, Manager, Nation, Player, ATTRIBUTE_NAMES};
pub use game_state::{GameState, SavedCareer};
pub use schedule::{
    Fixture, FixtureResult, Schedule, DEFAULT_LEGS, DEFAULT_ROUND_SPACING, NON_LEAGUE_SLOTS,
    first_round_for_year,
};
pub use tactics::{
    Formation, Lineup, LineupError, Marking, Mentality, PassTo, PlayerInstructions,
    SetPieceRole, TeamInstructions, TeamTactics, LINEUP_SIZE,
};
pub use simulation::{
    ContinueOutcome, ContinueRules, DayEvents, DayEventSource, NoEvents, StopReason,
    continue_until_event, process_day,
};
pub use traits::{DataProvider, MatchContext, MatchEngine, MatchResult};
