// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! Core domain types and traits for OpenCM0102.
//!
//! This crate has no I/O: it defines the data model, the game calendar and
//! the traits (`DataProvider`, `MatchEngine`) that the database, match and
//! simulation crates implement. Keeping it dependency-light lets the UI and
//! the engines evolve independently.

pub mod board;
pub mod calendar;
pub mod crowd;
pub mod cup;
pub mod data;
pub mod preset;
pub mod regen;
pub mod game_state;
pub mod news;
pub mod orig_save;
pub mod schedule;
pub mod simulation;
pub mod tactics;
pub mod transfers;
pub mod traits;
pub mod weather;

pub use calendar::GameDate;
pub use crowd::{crowd, CROWD_BASE, CROWD_REP_SQUARE, CROWD_VARIANCE_PCT, CROWD_WET_KEEP_PCT};
pub use board::{
    BoardOutcome, ConfidenceBand, ConfidenceEntry, ManagerConfidence, SackCause, band,
    CONFIDENCE_MAX,
};
pub use cup::{CupTie, DrawMode, DrawOutcome, Entrant, draw_round, seeding_score, value_term};
pub use regen::{
    UnattachedDraws, inherit_attached, regen_count, valid_unattached, ATTACHED_THRESHOLD,
    UNATTACHED_NARROW_MAX, UNATTACHED_WIDE_MAX,
};
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
pub use transfers::{
    PermitOutcome, PermitRecord, SetupEvent, due_events, holds_permit, parse_setup_events,
    permit_required, process_permits, remove_permit,
};
pub use tactics::{
    Formation, Lineup, LineupError, Marking, Mentality, PassTo, PlayerInstructions,
    SetPieceRole, TeamInstructions, TeamTactics, LINEUP_SIZE,
};
pub use news::{NewsCategory, NewsItem};
pub use orig_save::{
    SaveEntry, SaveError, anchor_name, anchor_u32, carve, parse_table, records, rle_decode,
    DISCIPLINE_RECORD, ENTRY_SIZE, FINANCE_RECORD, FLAG_MIXED, FLAG_PACKED, INJURY_RECORD,
};
pub use preset::{Preset, PresetError, AUTHOR_OFFSET, PLAYER_BITS_MASK, PRESET_SIZES, PRESET_SLOTS};
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
