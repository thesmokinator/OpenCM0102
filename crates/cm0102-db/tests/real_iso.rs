// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! End-to-end season flow against the real disc data. Runs only when
//! `CM0102_ISO` points at a CM 01/02 ISO image; otherwise it skips.
//! Verifies league grouping on real bytes, calendar generation, the
//! first Continue stop on a real match day, and next-year rollover.

use cm0102_core::{
    ContinueRules, GameState, Schedule, StopReason, continue_until_event, first_round_for_year,
    DEFAULT_LEGS, DEFAULT_ROUND_SPACING,
};
use cm0102_db::source::{CabinetSource, GameDataSource, IsoSource};
use cm0102_db::formats::{parse_clubs, parse_names, parse_nations, parse_officials};

/// Real-data season flow, skipped without `CM0102_ISO`.
#[test]
fn real_iso_reaches_first_playable_match() {
    let Ok(iso_path) = std::env::var("CM0102_ISO") else {
        eprintln!("skipping: CM0102_ISO is not set");
        return;
    };
    let iso = IsoSource::new(std::path::PathBuf::from(iso_path)).expect("open ISO");
    let source = CabinetSource::from_iso(&iso).expect("read cabinet");
    let clubs = parse_clubs(&source.read_entry("CM3 Data/club.dat").expect("club.dat"))
        .expect("parse clubs");

    // Premier League membership is exactly the 01/02 twenty.
    let managed = clubs
        .iter()
        .find(|club| club.name == "Arsenal")
        .expect("Arsenal imported");
    assert_eq!(managed.league_id, Some(7));
    let members = clubs
        .iter()
        .filter(|club| club.league_id == Some(7))
        .count();
    assert_eq!(members, 20);

    // Calendar generation: 38 rounds for the managed club.
    let schedule = Schedule::for_club_league(
        managed,
        &clubs,
        first_round_for_year(2001),
        DEFAULT_ROUND_SPACING,
        DEFAULT_LEGS,
    );
    let own: Vec<_> = schedule
        .fixtures
        .iter()
        .filter(|fixture| {
            fixture.home_club_id == managed.id || fixture.away_club_id == managed.id
        })
        .collect();
    assert_eq!(own.len(), 38);
    let first_match = own.iter().map(|fixture| fixture.date).min().expect("dated");

    // Continue from season start stops exactly on the first match day.
    let mut state = GameState::new_season();
    state.managed_club_id = Some(managed.id);
    state.schedule = schedule;
    let schedule = state.schedule.clone();
    let outcome = continue_until_event(&mut state, &ContinueRules::default(), &schedule);
    assert_eq!(outcome.stop, StopReason::OwnMatch);
    assert_eq!(state.current_date, first_match);

    // Past the last fixture, next year's calendar regenerates.
    let last = state.schedule.last_date().expect("last fixture");
    assert!(last > first_match);
    let next = Schedule::for_club_league(
        managed,
        &clubs,
        first_round_for_year(last.year + 1),
        DEFAULT_ROUND_SPACING,
        DEFAULT_LEGS,
    );
    assert_eq!(next.fixtures.len(), state.schedule.fixtures.len());
    assert!(
        next.fixtures
            .iter()
            .all(|fixture| fixture.date.year >= last.year)
    );

    // Officials carry 0-20 strictness; England's pool has rated referees.
    let first_names = parse_names(&source.read_entry("CM3 Data/first_names.dat").expect("names"));
    let second_names =
        parse_names(&source.read_entry("CM3 Data/second_names.dat").expect("names"));
    let officials = parse_officials(
        &source.read_entry("CM3 Data/officials.dat").expect("officials.dat"),
        &first_names,
        &second_names,
    );
    assert!(officials.len() > 3000, "officials: {}", officials.len());
    assert!(officials.iter().all(|o| o.strictness <= 20));
    // Nation ids shift between data versions: resolve by name.
    let nations = parse_nations(&source.read_entry("CM3 Data/nation.dat").expect("nation.dat"));
    let england = nations
        .iter()
        .find(|nation| nation.name == "England")
        .expect("England")
        .id;
    let rated_english = officials
        .iter()
        .filter(|o| o.nation_id == Some(england) && o.strictness > 0)
        .count();
    assert!(rated_english > 10, "rated english: {rated_english}");
}
