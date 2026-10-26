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
}

/// A nation known to the engine (manager nationality, club origin).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Nation {
    /// Stable identifier from the source data.
    pub id: u32,
    /// Display name, e.g. `Italy`.
    pub name: String,
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
