// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! Data model for clubs and players, mirroring what the `.dat` importer
//! fills in. Attributes documented in `docs/dat-format.md` are added here
//! only once their on-disk layout has been observed and specified.

use serde::{Deserialize, Serialize};

/// A football club known to the engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Club {
    /// Stable identifier from the source data.
    pub id: u32,
    /// Display name, e.g. `Arsenal`.
    pub name: String,
    /// Current-season league slot: `club.dat` byte 87, equal to the
    /// `club_comp.dat` id for real leagues. `None` when the imported
    /// database predates league membership (re-import to populate).
    /// Bucket values (see `schedule::NON_LEAGUE_SLOTS`) mean the club
    /// plays in no simulated league.
    #[serde(default)]
    pub league_id: Option<u32>,
}

/// A nation known to the engine (manager nationality, club origin).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Nation {
    /// Stable identifier from the source data.
    pub id: u32,
    /// Display name, e.g. `Italy`.
    pub name: String,
}

/// A match official (referee) from the source data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Official {
    /// Stable identifier from the source data.
    pub id: u32,
    /// Display name, e.g. `Pierluigi Collina`.
    pub name: String,
    /// Nation id, when national in scope.
    pub nation_id: Option<u32>,
    /// Strictness 0-20. Missing in databases imported before the
    /// field was read.
    #[serde(default)]
    pub strictness: Option<u8>,
    /// Reputation, weighting match assignment.
    #[serde(default)]
    pub reputation: Option<i16>,
}

/// The human player's manager in a career.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manager {
    /// First (given) name.
    pub first_name: String,
    /// Second (family) name.
    pub second_name: String,
    /// Nationality, referencing [`Nation::id`].
    pub nation_id: u32,
}

impl Manager {
    /// Full display name, e.g. `Arrigo Sacchi`.
    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.second_name)
    }
}

/// A player known to the engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Player {
    /// Stable identifier from the source data.
    pub id: u32,
    /// First (given) name.
    pub first_name: String,
    /// Second (family) name.
    pub second_name: String,
    /// Club the player belongs to, if any.
    pub club_id: Option<u32>,
    /// Current ability, once the source format documents it.
    pub current_ability: Option<i16>,
    /// Potential ability, once the source format documents it.
    pub potential_ability: Option<i16>,
    /// Preferred squad number, for players.
    pub squad_number: Option<u8>,
    /// Home / current / world reputation (0–200), when documented.
    pub home_reputation: Option<i16>,
    /// See `home_reputation`.
    pub current_reputation: Option<i16>,
    /// See `home_reputation`.
    pub world_reputation: Option<i16>,
    /// The 54 positional and playing attributes (0–20), in the order
    /// given by `ATTRIBUTE_NAMES`; empty for non-playing staff.
    pub attributes: Vec<u8>,
    /// Date of birth, when documented in the source record.
    pub birth_date: Option<crate::GameDate>,
    /// Raw job code from the source record (11 = player, 5 = manager,
    /// other values are non-playing staff roles not yet labelled).
    pub job: u8,
    /// Whether this person is a player (job code 11).
    pub is_player: bool,
}

impl Player {
    /// Full display name, e.g. `Thierry Henry`.
    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.second_name)
    }
}

/// Names of the 54 player attributes, in stored order: first the
/// positional ratings, then the playing attributes.
pub const ATTRIBUTE_NAMES: [&str; 54] = [
    "Goalkeeper",
    "Sweeper",
    "Defender",
    "Defensive Midfielder",
    "Midfielder",
    "Attacking Midfielder",
    "Attacker",
    "Wing Back",
    "Right Side",
    "Left Side",
    "Central",
    "Free Role",
    "Acceleration",
    "Aggression",
    "Agility",
    "Anticipation",
    "Balance",
    "Bravery",
    "Consistency",
    "Corners",
    "Crossing",
    "Decisions",
    "Dirtiness",
    "Dribbling",
    "Finishing",
    "Flair",
    "Free Kicks",
    "Handling",
    "Heading",
    "Important Matches",
    "Injury Proneness",
    "Jumping",
    "Leadership",
    "Left Foot",
    "Long Shots",
    "Marking",
    "Movement",
    "Natural Fitness",
    "One on Ones",
    "Pace",
    "Passing",
    "Penalties",
    "Positioning",
    "Reflexes",
    "Right Foot",
    "Stamina",
    "Strength",
    "Tackling",
    "Teamwork",
    "Technique",
    "Throw Ins",
    "Versatility",
    "Vision",
    "Work Rate",
];

/// Tackling position inside [`ATTRIBUTE_NAMES`].
pub const ATTR_TACKLING: usize = 46;
/// Marking position inside [`ATTRIBUTE_NAMES`].
pub const ATTR_MARKING: usize = 35;

/// Positions of widely used attributes inside [`ATTRIBUTE_NAMES`].
pub const ATTR_GOALKEEPER: usize = 0;
/// Sweeper positional rating.
pub const ATTR_SWEEPER: usize = 1;
/// Defender positional rating.
pub const ATTR_DEFENDER: usize = 2;
/// Defensive midfielder positional rating.
pub const ATTR_DM: usize = 3;
/// Attacker positional rating.
pub const ATTR_ATTACKER: usize = 6;
/// Aggression.
pub const ATTR_AGGRESSION: usize = 13;
/// Anticipation.
pub const ATTR_ANTICIPATION: usize = 15;
/// Decisions.
pub const ATTR_DECISIONS: usize = 21;
/// Dirtiness.
pub const ATTR_DIRTINESS: usize = 22;
/// Dribbling.
pub const ATTR_DRIBBLING: usize = 23;
/// Finishing.
pub const ATTR_FINISHING: usize = 24;
/// Handling (goalkeeper).
pub const ATTR_HANDLING: usize = 27;
/// Heading.
pub const ATTR_HEADING: usize = 28;
/// Injury proneness.
pub const ATTR_INJURY_PRONENESS: usize = 30;
/// Jumping.
pub const ATTR_JUMPING: usize = 31;
/// Long shots.
pub const ATTR_LONG_SHOTS: usize = 34;
/// One on ones.
pub const ATTR_ONE_ON_ONES: usize = 38;
/// Pace.
pub const ATTR_PACE: usize = 39;
/// Passing.
pub const ATTR_PASSING: usize = 40;
/// Penalties.
pub const ATTR_PENALTIES: usize = 41;
/// Positioning.
pub const ATTR_POSITIONING: usize = 42;
/// Reflexes (goalkeeper).
pub const ATTR_REFLEXES: usize = 43;
/// Technique.
pub const ATTR_TECHNIQUE: usize = 49;

#[cfg(test)]
mod tests {
    use super::*;

    /// `full_name` joins first and second name with a single space.
    #[test]
    fn full_name_joins_names() {
        let player = Player {
            id: 1,
            first_name: "Thierry".to_string(),
            second_name: "Henry".to_string(),
            club_id: None,
            current_ability: None,
            potential_ability: None,
            squad_number: None,
            home_reputation: None,
            current_reputation: None,
            world_reputation: None,
            attributes: Vec::new(),
            birth_date: None,
            job: 11,
            is_player: true,
        };
        assert_eq!(player.full_name(), "Thierry Henry");
    }
}
