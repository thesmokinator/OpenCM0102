// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! Season advancement: the daily pipeline plus event-driven Continue.
//!
//! The simulation ticks one day at a time ([`process_day`]), while the
//! Continue button loops days until the next date needing the manager
//! ([`continue_until_event`]), per the model in `docs/season.md`.
//! Stop conditions are data ([`ContinueRules`], serializable for the
//! future rules files); the event feed is a [`DayEventSource`] so the
//! real engines (fixtures, inbox, transfers, deadlines) plug in as
//! they land. Until they do, UI advancement routes through
//! [`process_day`] with an empty feed.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::board::BoardOutcome;
use crate::calendar::GameDate;
use crate::game_state::GameState;
use crate::transfers::{PermitOutcome, SetupEvent, process_permits};
use crate::regen::regen_count;

/// Manager-facing events produced while simulating one day.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DayEvents {
    /// The managed club plays today.
    pub own_match: bool,
    /// New inbox items addressed to the manager.
    pub inbox_items: u32,
    /// Transfer offer responses arrived.
    pub offer_responses: u32,
    /// A deadline falls today (window, registration, hearing).
    pub deadline_today: bool,
}

/// Editable stop conditions for event-driven Continue.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContinueRules {
    /// Stop on days the managed club plays.
    pub stop_on_own_match: bool,
    /// Stop on days with new inbox items.
    pub stop_on_inbox: bool,
    /// Stop on days with offer responses.
    pub stop_on_offer_response: bool,
    /// Stop on deadline days.
    pub stop_on_deadline: bool,
    /// Safety cap: never simulate more days in one press.
    pub max_days: u32,
}

impl Default for ContinueRules {
    /// The 01/02 behaviour: stop on every manager-facing event.
    fn default() -> Self {
        Self {
            stop_on_own_match: true,
            stop_on_inbox: true,
            stop_on_offer_response: true,
            stop_on_deadline: true,
            max_days: 60,
        }
    }
}

/// Why [`continue_until_event`] stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    /// A day with the managed club playing.
    OwnMatch,
    /// A day with new inbox items.
    Inbox,
    /// A day with offer responses.
    OfferResponse,
    /// A deadline day.
    Deadline,
    /// Cap reached with no triggering event.
    MaxDaysReached,
}

/// Outcome of one Continue press.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContinueOutcome {
    /// Days simulated by this press (at least one).
    pub days_advanced: u32,
    /// Why the loop stopped.
    pub stop: StopReason,
}

/// Feeds manager-facing events into the daily pipeline. Engines
/// implement this as they land; tests use scripted feeds.
pub trait DayEventSource {
    /// Events for `date` from the managed club's point of view.
    fn events_for(&self, date: GameDate, managed_club_id: Option<u32>) -> DayEvents;
}

/// An event feed that never produces anything: used until the real
/// engines (fixtures first) land.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoEvents;

impl DayEventSource for NoEvents {
    fn events_for(&self, _date: GameDate, _managed_club_id: Option<u32>) -> DayEvents {
        DayEvents::default()
    }
}

/// Simulate one day: advance the date, then run the daily pipeline
/// stages in order for the new current date. Stage order is still
/// open (see `docs/season.md`); engines hook their stage in here as
/// they land.
pub fn process_day<S: DayEventSource>(state: &mut GameState, source: &S) -> DayEvents {
    state.advance_day();
    // Pipeline stages (order open): fixtures, news/inbox, transfers,
    // training/fitness, finance, board. Each stage contributes its
    // slice of the day's events once implemented.
    source.events_for(state.current_date, state.managed_club_id)
}

/// Run [`process_day`] until the rules match the day's events.
/// Always simulates at least one day; the safety cap bounds the run.
pub fn continue_until_event<S: DayEventSource>(
    state: &mut GameState,
    rules: &ContinueRules,
    source: &S,
) -> ContinueOutcome {
    continue_until_event_with_hook(state, rules, source, &mut |_| {})
}

/// Same as [`continue_until_event`], but runs `daily` after each day
/// advance so the application can simulate the day's world (fixtures,
/// recovery, finance) and push news. Fresh inbox items dated today
/// feed the inbox stop rule.
pub fn continue_until_event_with_hook<S: DayEventSource>(
    state: &mut GameState,
    rules: &ContinueRules,
    source: &S,
    daily: &mut dyn FnMut(&mut GameState),
) -> ContinueOutcome {
    let mut days_advanced = 0;
    loop {
        let mut events = process_day(state, source);
        days_advanced += 1;
        daily(state);
        events.inbox_items += state.fresh_news().len() as u32;
        if let Some(stop) = stops_for(rules, &events) {
            return ContinueOutcome {
                days_advanced,
                stop,
            };
        }
        if days_advanced >= rules.max_days.max(1) {
            return ContinueOutcome {
                days_advanced,
                stop: StopReason::MaxDaysReached,
            };
        }
    }
}

/// Input to one day's engine stages: the caller (editable rules,
/// later) resolves what the world did today; the pipeline applies
/// the documented mechanics and reports what happened for the inbox.
#[derive(Debug, Clone, Default)]
pub struct DailyEngineInput {
    /// Qualifying dates for pending permit decisions, by player id.
    pub qualifying: HashMap<u32, GameDate>,
    /// Expiry season stamped on permits granted today.
    pub permit_season: u16,
    /// Nation stamping permits granted today.
    pub permit_nation: u32,
    /// Signed board-confidence step (0 applies nothing). Result
    /// formulas stay open, so the caller owns the magnitude.
    pub board_step: i32,
    /// Board outcome to queue as news, if the board acted today.
    pub board_outcome: Option<BoardOutcome>,
    /// Staff ids retiring today, feeding regen processing.
    pub retired_today: Vec<u32>,
    /// Scripted setup transfers/loans due today, queued for the
    /// database layer to apply.
    pub setup_due: Vec<SetupEvent>,
}

/// What one day's engine stages did: the application renders the
/// inbox news from this report.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DailyEngineReport {
    /// Players granted a permit today, in ascending id order.
    pub permit_grants: Vec<u32>,
    /// Players refused a permit today, in ascending id order.
    pub permit_refusals: Vec<u32>,
    /// Regens newly due from retired-staff consumption.
    pub regens_due_now: usize,
    /// Setup events queued for the database layer.
    pub setup_queued: usize,
    /// Board outcome to announce, if the board acted today.
    pub board_outcome: Option<BoardOutcome>,
}

/// Run one day's engine stages against `state` for `today`:
/// permit decisions, the board-confidence step, retired-list
/// consumption into regen dues, and setup-event queueing. Pure
/// state changes; the caller renders news from the report.
pub fn apply_daily_engines(
    state: &mut GameState,
    today: GameDate,
    input: DailyEngineInput,
) -> DailyEngineReport {
    let outcomes = process_permits(
        &mut state.permits,
        today,
        &input.qualifying,
        input.permit_season,
        input.permit_nation,
    );
    let mut players: Vec<u32> = outcomes.keys().copied().collect();
    players.sort_unstable();
    let mut report = DailyEngineReport {
        board_outcome: input.board_outcome,
        ..DailyEngineReport::default()
    };
    for player_id in players {
        match outcomes[&player_id] {
            PermitOutcome::Granted => report.permit_grants.push(player_id),
            PermitOutcome::Refused => report.permit_refusals.push(player_id),
        }
    }
    if input.board_step != 0 {
        state.board_confidence.apply_step(today, input.board_step);
    }
    state.retired_staff.extend(input.retired_today);
    let due = regen_count(state.retired_staff.len());
    if due > 0 {
        state.regen_due += due;
        state.retired_staff.clear();
        report.regens_due_now = due;
    }
    report.setup_queued = input.setup_due.len();
    state.pending_transfers.extend(input.setup_due);
    report
}

/// First matching stop condition, in spec priority order. Public
/// so the application can run its own day loop (its daily hook
/// needs world access the core hook cannot borrow).
pub fn stops_for(rules: &ContinueRules, events: &DayEvents) -> Option<StopReason> {
    if rules.stop_on_own_match && events.own_match {
        Some(StopReason::OwnMatch)
    } else if rules.stop_on_inbox && events.inbox_items > 0 {
        Some(StopReason::Inbox)
    } else if rules.stop_on_offer_response && events.offer_responses > 0 {
        Some(StopReason::OfferResponse)
    } else if rules.stop_on_deadline && events.deadline_today {
        Some(StopReason::Deadline)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Scripted feed: an own match every `every` days from season start.
    struct EveryMatch {
        every: u32,
    }

    impl DayEventSource for EveryMatch {
        fn events_for(&self, date: GameDate, _managed_club_id: Option<u32>) -> DayEvents {
            let start = GameDate::SEASON_START;
            let days = date.day as i64 - start.day as i64
                + 31 * (date.month as i64 - start.month as i64)
                + 366 * (date.year as i64 - start.year as i64);
            DayEvents {
                own_match: days > 0 && days % self.every as i64 == 0,
                ..DayEvents::default()
            }
        }
    }

    #[test]
    fn process_day_advances_exactly_one_day() {
        let mut state = GameState::new_season();
        let events = process_day(&mut state, &NoEvents);
        assert_eq!(state.current_date, GameDate::new(2001, 7, 2));
        assert_eq!(events, DayEvents::default());
    }

    #[test]
    fn continue_stops_on_first_own_match() {
        let mut state = GameState::new_season();
        let outcome = continue_until_event(
            &mut state,
            &ContinueRules::default(),
            &EveryMatch { every: 5 },
        );
        assert_eq!(outcome.days_advanced, 5);
        assert_eq!(outcome.stop, StopReason::OwnMatch);
        assert_eq!(state.current_date, GameDate::new(2001, 7, 6));
    }

    #[test]
    fn disabled_rule_skips_its_event() {
        let mut state = GameState::new_season();
        let rules = ContinueRules {
            stop_on_own_match: false,
            max_days: 6,
            ..ContinueRules::default()
        };
        let outcome = continue_until_event(&mut state, &rules, &EveryMatch { every: 5 });
        assert_eq!(outcome.days_advanced, 6);
        assert_eq!(outcome.stop, StopReason::MaxDaysReached);
    }

    #[test]
    fn empty_feed_runs_to_the_cap() {
        let mut state = GameState::new_season();
        let rules = ContinueRules {
            max_days: 3,
            ..ContinueRules::default()
        };
        let outcome = continue_until_event(&mut state, &rules, &NoEvents);
        assert_eq!(outcome.days_advanced, 3);
        assert_eq!(outcome.stop, StopReason::MaxDaysReached);
        assert_eq!(state.current_date, GameDate::new(2001, 7, 4));
    }

    /// One day's engines decide permits, move confidence, consume
    /// retirements into regen dues and queue setup events.
    #[test]
    fn daily_engines_apply_all_stages() {
        use crate::board::SackCause;
        use crate::transfers::SetupEvent;

        let mut state = GameState::new_season();
        let today = GameDate::new(2001, 7, 5);
        let confidence_before = state.board_confidence.value;
        let input = DailyEngineInput {
            qualifying: HashMap::from([
                (1u32, GameDate::new(2001, 7, 6)),
                (2u32, GameDate::new(2001, 9, 1)),
            ]),
            permit_season: 2002,
            permit_nation: 50,
            board_step: -500,
            board_outcome: Some(BoardOutcome::PatienceWarning),
            retired_today: vec![10, 11, 12, 13],
            setup_due: vec![SetupEvent::FutureTransfer {
                player: ["A".to_string(), "B".to_string(), "C".to_string()],
                from_club: "X".to_string(),
                to_club: "Y".to_string(),
                date: today,
            }],
        };
        let report = apply_daily_engines(&mut state, today, input);
        assert_eq!(report.permit_grants, vec![1]);
        assert_eq!(report.permit_refusals, vec![2]);
        assert!(crate::transfers::holds_permit(&state.permits, 1));
        assert_eq!(state.board_confidence.value, confidence_before - 500);
        assert_eq!(report.board_outcome, Some(BoardOutcome::PatienceWarning));
        assert_eq!(report.regens_due_now, 2);
        assert_eq!(state.regen_due, 2);
        assert!(state.retired_staff.is_empty());
        assert_eq!(report.setup_queued, 1);
        assert_eq!(state.pending_transfers.len(), 1);
        let _ = SackCause::Results;
    }

    /// Empty input leaves engine state untouched.
    #[test]
    fn daily_engines_rest_on_empty_input() {
        let mut state = GameState::new_season();
        let today = GameDate::SEASON_START;
        let report =
            apply_daily_engines(&mut state, today, DailyEngineInput::default());
        assert_eq!(report, DailyEngineReport::default());
        assert!(state.permits.is_empty());
        assert_eq!(state.regen_due, 0);
        assert!(state.board_confidence.history.is_empty());
    }

    #[test]
    fn inbox_beats_offer_response_in_priority() {
        let rules = ContinueRules::default();
        let events = DayEvents {
            inbox_items: 2,
            offer_responses: 1,
            ..DayEvents::default()
        };
        assert_eq!(stops_for(&rules, &events), Some(StopReason::Inbox));
    }

    /// Fresh inbox items pushed by the daily hook stop Continue.
    #[test]
    fn hook_news_stops_continue() {
        use crate::news::NewsCategory;
        let mut state = GameState::new_season();
        let rules = ContinueRules {
            stop_on_own_match: false,
            max_days: 10,
            ..ContinueRules::default()
        };
        let outcome = continue_until_event_with_hook(
            &mut state,
            &rules,
            &NoEvents,
            &mut |state| {
                state.push_news(
                    NewsCategory::Board,
                    "headline".to_string(),
                    "body".to_string(),
                );
            },
        );
        assert_eq!(outcome.days_advanced, 1);
        assert_eq!(outcome.stop, StopReason::Inbox);
    }

    /// Read news never stops Continue again.
    #[test]
    fn read_news_does_not_stop() {
        use crate::news::NewsCategory;
        let mut state = GameState::new_season();
        state.push_news(
            NewsCategory::Board,
            "headline".to_string(),
            "body".to_string(),
        );
        state.mark_news_read_through_today();
        let rules = ContinueRules {
            max_days: 3,
            ..ContinueRules::default()
        };
        let outcome = continue_until_event(&mut state, &rules, &NoEvents);
        assert_eq!(outcome.days_advanced, 3);
        assert_eq!(outcome.stop, StopReason::MaxDaysReached);
    }

    #[test]
    fn continue_rules_round_trip_as_data() {
        let rules = ContinueRules::default();
        let json = serde_json::to_string(&rules).expect("serializable");
        let restored: ContinueRules = serde_json::from_str(&json).expect("deserializable");
        assert_eq!(restored, rules);
    }
}
