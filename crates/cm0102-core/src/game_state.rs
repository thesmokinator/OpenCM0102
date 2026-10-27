// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! Top-level mutable game state owned by the application.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::calendar::GameDate;
use crate::data::Manager;
use crate::schedule::Schedule;
use crate::tactics::TeamTactics;
use crate::traits::MatchResult;

/// Matches banned after a second yellow (provisional).
pub const SECOND_YELLOW_BAN: u32 = 1;
/// Matches banned after a straight red (provisional).
pub const STRAIGHT_RED_BAN: u32 = 3;
/// Season yellows triggering a ban (provisional).
pub const YELLOWS_FOR_BAN: u8 = 5;
/// Matches banned on reaching the yellow threshold (provisional).
pub const YELLOW_COUNT_BAN: u32 = 1;

/// The whole simulated world at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    /// Current in-game date.
    pub current_date: GameDate,
    /// Club managed by the human player, if a career has started.
    pub managed_club_id: Option<u32>,
    /// Human player's manager, once a career has started.
    pub manager: Option<Manager>,
    /// This season's fixture list. Defaults to empty so careers saved
    /// before calendars existed still load; the next Continue
    /// regenerates the calendar through the season rollover.
    #[serde(default)]
    pub schedule: Schedule,
    /// Managed club's tactics: lineup plus instructions. Defaults to
    /// an empty 4-4-2 so older saves still load; the UI auto-picks a
    /// lineup from the squad when none is stored.
    #[serde(default)]
    pub tactics: TeamTactics,
    /// Matches remaining banned, by player id.
    #[serde(default)]
    pub suspensions: HashMap<u32, u32>,
    /// Date the player is fit again, by player id.
    #[serde(default)]
    pub injuries: HashMap<u32, GameDate>,
    /// Season yellow-card counts, by player id.
    #[serde(default)]
    pub yellow_counts: HashMap<u32, u8>,
}

impl GameState {
    /// A new game at the start of the 01/02 season.
    pub fn new_season() -> Self {
        Self {
            current_date: GameDate::SEASON_START,
            managed_club_id: None,
            manager: None,
            schedule: Schedule::default(),
            tactics: TeamTactics::default(),
            suspensions: HashMap::new(),
            injuries: HashMap::new(),
            yellow_counts: HashMap::new(),
        }
    }

    /// True when the player may play today: not banned and fit.
    pub fn is_available(&self, player_id: u32) -> bool {
        if self.suspensions.get(&player_id).is_some_and(|left| *left > 0) {
            return false;
        }
        self.injuries
            .get(&player_id)
            .map_or(true, |fit| self.current_date >= *fit)
    }

    /// Apply a match report to season discipline: bans from reds,
    /// yellow accumulation, and injury layoffs from today.
    pub fn apply_match_report(&mut self, report: &MatchResult) {
        for rating in &report.ratings {
            if rating.red {
                let matches = if rating.yellow {
                    SECOND_YELLOW_BAN
                } else {
                    STRAIGHT_RED_BAN
                };
                self.suspensions.insert(rating.player_id, matches);
            } else if rating.yellow {
                let count = self
                    .yellow_counts
                    .get(&rating.player_id)
                    .copied()
                    .unwrap_or(0)
                    + 1;
                if count >= YELLOWS_FOR_BAN {
                    self.suspensions.insert(rating.player_id, YELLOW_COUNT_BAN);
                    self.yellow_counts.remove(&rating.player_id);
                } else {
                    self.yellow_counts.insert(rating.player_id, count);
                }
            }
        }
        for injury in &report.injuries {
            let fit = self.current_date.add_days(injury.days_out);
            self.injuries
                .entry(injury.player_id)
                .and_modify(|date| *date = (*date).max(fit))
                .or_insert(fit);
        }
    }

    /// Serve one suspension match for the listed players, dropping
    /// bans that reach zero.
    pub fn serve_suspension_match(&mut self, player_ids: &[u32]) {
        for id in player_ids {
            if let Some(left) = self.suspensions.get_mut(id) {
                *left = left.saturating_sub(1);
                if *left == 0 {
                    self.suspensions.remove(id);
                }
            }
        }
    }

    /// Reset season yellow counts for a new season. Bans carry over.
    pub fn reset_season_cards(&mut self) {
        self.yellow_counts.clear();
    }

    /// Start a career: the manager takes charge of `club_id` on the
    /// first day of the season, mirroring the original game's
    /// create-manager flow (manager, nationality, club, save, start).
    pub fn start_career(&mut self, manager: Manager, club_id: u32) {
        self.current_date = GameDate::SEASON_START;
        self.managed_club_id = Some(club_id);
        self.manager = Some(manager);
    }

    /// Advance the world by one day.
    pub fn advance_day(&mut self) {
        self.current_date = self.current_date.next_day();
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self::new_season()
    }
}

/// Native career-save envelope: our own interim format, versioned so
/// older saves stay readable as the state grows. This is unrelated to
/// the original game's binary save format, whose support is tracked
/// separately (bidirectional compatibility is a declared goal, pending
/// analysis of user-supplied save files).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedCareer {
    /// Envelope version; readers accept only versions they know.
    pub version: u32,
    /// The saved world state.
    pub state: GameState,
}

impl SavedCareer {
    /// Current envelope version written by this release.
    pub const CURRENT_VERSION: u32 = 1;

    /// Wrap a state in the current envelope version.
    pub fn new(state: GameState) -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            state,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A new season starts on 1 July 2001 with no managed club.
    #[test]
    fn new_season_starts_at_season_start() {
        let state = GameState::new_season();
        assert_eq!(state.current_date, GameDate::SEASON_START);
        assert_eq!(state.managed_club_id, None);
        assert_eq!(state.manager, None);
    }

    /// Advancing moves the date forward by exactly one day.
    #[test]
    fn advance_day_moves_one_day() {
        let mut state = GameState::new_season();
        state.advance_day();
        assert_eq!(state.current_date, GameDate::new(2001, 7, 2));
    }

    /// Starting a career records the manager and club on season start.
    #[test]
    fn start_career_records_manager_and_club() {
        let mut state = GameState::new_season();
        state.advance_day();
        let manager = Manager {
            first_name: "Arrigo".to_string(),
            second_name: "Sacchi".to_string(),
            nation_id: 1,
        };
        state.start_career(manager.clone(), 676);
        assert_eq!(state.current_date, GameDate::SEASON_START);
        assert_eq!(state.managed_club_id, Some(676));
        assert_eq!(state.manager, Some(manager));
    }

    /// A career survives a save-envelope round trip unchanged.
    #[test]
    fn saved_career_round_trip() {
        let mut state = GameState::new_season();
        state.start_career(
            Manager {
                first_name: "Arrigo".to_string(),
                second_name: "Sacchi".to_string(),
                nation_id: 1,
            },
            676,
        );
        state.advance_day();
        let saved = SavedCareer::new(state);
        let json = serde_json::to_string(&saved).expect("serializable");
        let restored: SavedCareer = serde_json::from_str(&json).expect("deserializable");
        assert_eq!(restored.version, SavedCareer::CURRENT_VERSION);
        assert_eq!(restored.state.current_date, GameDate::new(2001, 7, 2));
        assert_eq!(restored.state.managed_club_id, Some(676));
        assert_eq!(
            restored.state.manager.expect("manager").full_name(),
            "Arrigo Sacchi"
        );
    }

    /// Reports ban reds, accumulate yellows and sideline injuries.
    #[test]
    fn match_reports_drive_discipline() {
        use crate::traits::{MatchInjury, MatchStats, PlayerRating};
        let mut state = GameState::new_season();
        let report = MatchResult {
            home_goals: 1,
            away_goals: 0,
            events: Vec::new(),
            stats: MatchStats::default(),
            ratings: vec![
                PlayerRating {
                    player_id: 1,
                    name: "Sent Off".to_string(),
                    home: true,
                    rating: 55,
                    minutes: 60,
                    goals: 0,
                    yellow: false,
                    red: true,
                },
                PlayerRating {
                    player_id: 2,
                    name: "Booked".to_string(),
                    home: true,
                    rating: 68,
                    minutes: 90,
                    goals: 0,
                    yellow: true,
                    red: false,
                },
            ],
            shootout: None,
            injuries: vec![MatchInjury {
                player_id: 3,
                name: "Crocked".to_string(),
                home: true,
                days_out: 7,
            }],
        };
        state.apply_match_report(&report);
        assert_eq!(state.suspensions.get(&1), Some(&STRAIGHT_RED_BAN));
        assert_eq!(state.yellow_counts.get(&2), Some(&1));
        assert!(!state.is_available(1));
        assert!(state.is_available(2));
        assert!(!state.is_available(3));
        assert_eq!(
            state.injuries.get(&3),
            Some(&GameDate::new(2001, 7, 8))
        );
        state.serve_suspension_match(&[1, 2]);
        assert_eq!(state.suspensions.get(&1), Some(&(STRAIGHT_RED_BAN - 1)));
    }

    /// Five yellows earn a ban and reset the count.
    #[test]
    fn yellow_accumulation_bans() {
        use crate::traits::{MatchStats, PlayerRating};
        let mut state = GameState::new_season();
        for _ in 0..YELLOWS_FOR_BAN {
            state.apply_match_report(&MatchResult {
                home_goals: 0,
                away_goals: 0,
                events: Vec::new(),
                stats: MatchStats::default(),
                ratings: vec![PlayerRating {
                    player_id: 9,
                    name: "Dirty".to_string(),
                    home: true,
                    rating: 65,
                    minutes: 90,
                    goals: 0,
                    yellow: true,
                    red: false,
                }],
                shootout: None,
                injuries: Vec::new(),
            });
        }
        assert_eq!(state.suspensions.get(&9), Some(&YELLOW_COUNT_BAN));
        assert!(!state.yellow_counts.contains_key(&9));
    }

    /// Saves written before calendars existed load with an empty
    /// schedule, which the season rollover regenerates.
    #[test]
    fn old_saves_load_without_schedule() {
        let json = r#"{"version":1,"state":{"current_date":{"year":2001,"month":7,"day":1},"managed_club_id":676,"manager":{"first_name":"Arrigo","second_name":"Sacchi","nation_id":1}}}"#;
        let restored: SavedCareer = serde_json::from_str(json).expect("deserializable");
        assert!(restored.state.schedule.fixtures.is_empty());
        assert_eq!(restored.state.managed_club_id, Some(676));
    }
}
