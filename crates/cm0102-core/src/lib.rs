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
pub mod traits;

pub use calendar::GameDate;
pub use data::{Club, Manager, Nation, Player, ATTRIBUTE_NAMES};
pub use game_state::{GameState, SavedCareer};
pub use traits::{DataProvider, MatchContext, MatchEngine, MatchResult};
