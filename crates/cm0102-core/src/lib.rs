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
pub mod crowd;
pub mod data;
pub mod game_state;
pub mod news;
pub mod schedule;
pub mod simulation;
pub mod tactics;
pub mod traits;
pub mod weather;

pub use calendar::GameDate;
pub use crowd::{crowd, CROWD_BASE, CROWD_REP_SQUARE, CROWD_VARIANCE_PCT, CROWD_WET_KEEP_PCT};
pub use data::{
    Club, Manager, Nation, Official, Player, ATTRIBUTE_NAMES, ATTR_AGGRESSION, ATTR_ANTICIPATION,
    ATTR_ATTACKER, ATTR_DECISIONS, ATTR_DEFENDER, ATTR_DIRTINESS, ATTR_DM, ATTR_DRIBBLING,
    ATTR_FINISHING, ATTR_GOALKEEPER, ATTR_HANDLING, ATTR_HEADING, ATTR_INJURY_PRONENESS,
    ATTR_JUMPING, ATTR_LONG_SHOTS, ATTR_MARKING, ATTR_ONE_ON_ONES, ATTR_PACE, ATTR_PASSING,
    ATTR_PENALTIES, ATTR_POSITIONING, ATTR_REFLEXES, ATTR_SWEEPER, ATTR_TACKLING,
    ATTR_TECHNIQUE,
};
pub use game_state::{
    GameState, SavedCareer, SECOND_YELLOW_BAN, STRAIGHT_RED_BAN, YELLOW_COUNT_BAN,
    YELLOWS_FOR_BAN,
};
pub use schedule::{
    Fixture, FixtureResult, Schedule, TableRow, TableRules, DEFAULT_LEGS, DEFAULT_ROUND_SPACING,
    NON_LEAGUE_SLOTS, first_round_for_year,
};
pub use tactics::{
    Formation, Lineup, LineupError, Marking, Mentality, PassTo, PlayerInstructions,
    SetPieceRole, TeamInstructions, TeamTactics, LINEUP_SIZE,
};
pub use news::{NewsCategory, NewsItem};
pub use simulation::{
    ContinueOutcome, ContinueRules, DayEvents, DayEventSource, NoEvents, StopReason,
    continue_until_event, continue_until_event_with_hook, process_day, stops_for,
};
pub use traits::{
    strictness_for_official, DataProvider, EventKind, MatchContext, MatchEngine, MatchEvent,
    MatchInjury, MatchPlayer, MatchResult, MatchStats, MinuteStats, PlayerRating, Referee,
    Shootout, SideMinute, NEUTRAL_STRICTNESS,
};
pub use weather::{
    is_postponed, league_nation, league_zone, nation_zone, roll_weather, ClimateZone,
    MatchWeather, Pitch, SeasonSlice, WeatherError, WeatherTable, FROZEN_POSTPONE_PCT,
    WATERLOGGED_POSTPONE_PCT,
};
