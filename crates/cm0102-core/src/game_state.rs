// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! Top-level mutable game state owned by the application.

use serde::{Deserialize, Serialize};

use crate::calendar::GameDate;
use crate::data::Manager;
use crate::schedule::Schedule;
use crate::tactics::TeamTactics;

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
        }
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
