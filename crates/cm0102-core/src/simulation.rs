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

use crate::calendar::GameDate;
use crate::game_state::GameState;

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
    let mut days_advanced = 0;
    loop {
        let events = process_day(state, source);
        days_advanced += 1;
        if let Some(stop) = match_stop(rules, &events) {
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

/// First matching stop condition, in spec priority order.
fn match_stop(rules: &ContinueRules, events: &DayEvents) -> Option<StopReason> {
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

    #[test]
    fn inbox_beats_offer_response_in_priority() {
        let rules = ContinueRules::default();
        let events = DayEvents {
            inbox_items: 2,
            offer_responses: 1,
            ..DayEvents::default()
        };
        assert_eq!(match_stop(&rules, &events), Some(StopReason::Inbox));
    }

    #[test]
    fn continue_rules_round_trip_as_data() {
        let rules = ContinueRules::default();
        let json = serde_json::to_string(&rules).expect("serializable");
        let restored: ContinueRules = serde_json::from_str(&json).expect("deserializable");
        assert_eq!(restored, rules);
    }
}
