// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! Tactics model: formations, lineups and instructions.
//!
//! The catalog comes from the shipped UI text (see `docs/tactics.md`
//! for the evidence): 9 formations, the 14-row player instruction
//! grid and the team instruction set. Only option sets with observed
//! values are enumerated; everything else stays open on purpose.
//! In-match effects are NOT modelled here — the match engine reads
//! lineups through [`MatchContext`](crate::MatchContext) strengths
//! and applies placeholder behaviour until the real effects are
//! reverse-engineered.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::data::Player;

/// Named formations from the game's preferred-formation list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Formation {
    /// Four defenders, four midfielders, two strikers.
    #[default]
    FourFourTwo,
    /// Four defenders, three midfielders, three strikers.
    FourThreeThree,
    /// Four defenders, five midfielders, one striker.
    FourFiveOne,
    /// Four defenders, two midfielders, four strikers.
    FourTwoFour,
    /// Four defenders, diamond midfield, two strikers.
    FourOneTwoOneTwo,
    /// Three defenders, five midfielders, two strikers.
    ThreeFiveTwo,
    /// Three defenders, four midfielders, three strikers.
    ThreeFourThree,
    /// Five defenders, three midfielders, two strikers.
    FiveThreeTwo,
    /// Five defenders with a sweeper, three midfielders, two strikers.
    FiveThreeTwoSweeper,
}

impl Formation {
    /// Every formation in the game's list order.
    pub const ALL: &'static [Formation] = &[
        Formation::ThreeFourThree,
        Formation::FourFiveOne,
        Formation::FourOneTwoOneTwo,
        Formation::ThreeFiveTwo,
        Formation::FourTwoFour,
        Formation::FourThreeThree,
        Formation::FourFourTwo,
        Formation::FiveThreeTwo,
        Formation::FiveThreeTwoSweeper,
    ];

    /// Display name, e.g. `4-4-2`.
    pub const fn name(self) -> &'static str {
        match self {
            Formation::ThreeFourThree => "3-4-3",
            Formation::FourFiveOne => "4-5-1",
            Formation::FourOneTwoOneTwo => "4-1-2-1-2",
            Formation::ThreeFiveTwo => "3-5-2",
            Formation::FourTwoFour => "4-2-4",
            Formation::FourThreeThree => "4-3-3",
            Formation::FourFourTwo => "4-4-2",
            Formation::FiveThreeTwo => "5-3-2",
            Formation::FiveThreeTwoSweeper => "5-3-2 Sweeper",
        }
    }
}

/// Players in a starting eleven.
pub const LINEUP_SIZE: usize = 11;

/// A starting eleven: formation plus player ids in shirt order
/// (index 0 is the goalkeeper slot).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Lineup {
    /// Shape the eleven plays in.
    pub formation: Formation,
    /// Starting player ids, shirt order.
    pub starting: Vec<u32>,
}

/// Why a lineup was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineupError {
    /// Not exactly 11 players.
    WrongSize(usize),
    /// The same player picked twice.
    Duplicate(u32),
}

impl Lineup {
    /// A lineup is valid with exactly 11 distinct players.
    pub fn validate(&self) -> Result<(), LineupError> {
        if self.starting.len() != LINEUP_SIZE {
            return Err(LineupError::WrongSize(self.starting.len()));
        }
        let mut seen = HashMap::with_capacity(LINEUP_SIZE);
        for id in &self.starting {
            if seen.insert(id, ()).is_some() {
                return Err(LineupError::Duplicate(*id));
            }
        }
        Ok(())
    }

    /// Best-eleven auto-pick: the 11 players with the highest current
    /// ability (missing ability counts as 0). Squads shorter than 11
    /// yield a short, invalid lineup — the caller decides how to
    /// handle it.
    pub fn auto_pick(formation: Formation, squad: &[Player]) -> Self {
        let mut candidates: Vec<&Player> = squad.iter().filter(|p| p.is_player).collect();
        candidates.sort_by_key(|player| std::cmp::Reverse(player.current_ability.unwrap_or(0)));
        let starting = candidates
            .into_iter()
            .take(LINEUP_SIZE)
            .map(|player| player.id)
            .collect();
        Self {
            formation,
            starting,
        }
    }
}

/// Player marking instruction (observed option set).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Marking {
    /// Tight man marking.
    Man,
    /// Man-mark one named opponent.
    ManToPlayer,
    /// Zonal marking.
    Zonal,
    /// No marking instruction.
    None,
}

/// Set-piece positioning (observed option set).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SetPieceRole {
    /// Stay forward for set pieces.
    Forward,
    /// Stay back for set pieces.
    Back,
    /// Normal set-piece positioning.
    Normal,
}

/// Pass-direction instruction (observed option set).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PassTo {
    /// Favour the right side.
    Right,
    /// Favour the centre.
    Centre,
    /// Favour the left side.
    Left,
    /// No direction restriction.
    All,
}

/// Per-player instructions. Only rows with observed option sets are
/// modelled; the rest (cross ball, through balls, long shots,
/// hold-up ball, run with ball, forward runs, free role, pressing,
/// tackling, passing levels) are still open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PlayerInstructions {
    /// Marking instruction, if set.
    pub marking: Option<Marking>,
    /// Attacking set-piece role, if set.
    pub attacking_set_piece: Option<SetPieceRole>,
    /// Defensive set-piece role, if set.
    pub defensive_set_piece: Option<SetPieceRole>,
    /// Pass-direction instruction, if set.
    pub pass_to: Option<PassTo>,
}

/// Team mentality (observed option set).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Mentality {
    /// Balanced mentality.
    #[default]
    Normal,
    /// Attacking mentality.
    Attacking,
    /// Defensive mentality.
    Defensive,
}

/// Team-level instructions (observed set).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TeamInstructions {
    /// Team mentality.
    pub mentality: Mentality,
    /// Play on the counter attack.
    pub counter_attack: bool,
    /// Use the offside trap.
    pub offside_trap: bool,
    /// Keep men behind the ball.
    pub men_behind_ball: bool,
    /// Playmaker player id, if appointed.
    pub playmaker: Option<u32>,
    /// Left/right corner takers.
    pub corner_takers: (Option<u32>, Option<u32>),
    /// Left/right free-kick takers.
    pub free_kick_takers: (Option<u32>, Option<u32>),
    /// Penalty takers in order.
    pub penalty_takers: Vec<u32>,
}

/// Everything the manager sets for a match: lineup plus team and
/// per-player instructions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TeamTactics {
    /// Starting eleven.
    pub lineup: Lineup,
    /// Team instructions.
    pub team: TeamInstructions,
    /// Per-player instructions by player id.
    pub players: HashMap<u32, PlayerInstructions>,
}

impl TeamTactics {
    /// Default instructions with an auto-picked lineup from `squad`.
    pub fn auto(formation: Formation, squad: &[Player]) -> Self {
        Self {
            lineup: Lineup::auto_pick(formation, squad),
            team: TeamInstructions::default(),
            players: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn player(id: u32, ca: Option<i16>) -> Player {
        Player {
            id,
            first_name: "A".to_string(),
            second_name: format!("Player{id}"),
            club_id: Some(1),
            current_ability: ca,
            potential_ability: None,
            squad_number: None,
            home_reputation: None,
            current_reputation: None,
            world_reputation: None,
            attributes: Vec::new(),
            birth_date: None,
            job: 11,
            is_player: true,
        }
    }

    /// The catalog holds the 9 game formations in list order.
    #[test]
    fn formation_catalog_matches_the_game() {
        let names: Vec<&str> = Formation::ALL.iter().map(|f| f.name()).collect();
        assert_eq!(
            names,
            vec![
                "3-4-3",
                "4-5-1",
                "4-1-2-1-2",
                "3-5-2",
                "4-2-4",
                "4-3-3",
                "4-4-2",
                "5-3-2",
                "5-3-2 Sweeper",
            ]
        );
        assert_eq!(Formation::default(), Formation::FourFourTwo);
    }

    /// Validation accepts 11 distinct ids and rejects the rest.
    #[test]
    fn lineup_validation_checks_size_and_duplicates() {
        let valid = Lineup {
            formation: Formation::FourFourTwo,
            starting: (1..=11).collect(),
        };
        assert_eq!(valid.validate(), Ok(()));
        let short = Lineup {
            formation: Formation::FourFourTwo,
            starting: (1..=10).collect(),
        };
        assert_eq!(short.validate(), Err(LineupError::WrongSize(10)));
        let mut dup_ids: Vec<u32> = (1..=10).collect();
        dup_ids.push(1);
        let dup = Lineup {
            formation: Formation::FourFourTwo,
            starting: dup_ids,
        };
        assert_eq!(dup.validate(), Err(LineupError::Duplicate(1)));
    }

    /// Auto-pick takes the 11 highest current abilities in order.
    #[test]
    fn auto_pick_selects_best_eleven() {
        let squad: Vec<Player> = (0..15)
            .map(|i| player(i, Some(100 + i as i16)))
            .collect();
        let lineup = Lineup::auto_pick(Formation::FourThreeThree, &squad);
        assert_eq!(lineup.starting.len(), 11);
        assert_eq!(lineup.starting[0], 14);
        assert_eq!(lineup.validate(), Ok(()));
    }

    /// Missing ability counts as 0, non-players are skipped.
    #[test]
    fn auto_pick_handles_unknown_ability_and_staff() {
        let mut squad: Vec<Player> = (0..11).map(|i| player(i, None)).collect();
        let mut coach = player(99, Some(190));
        coach.is_player = false;
        coach.job = 5;
        squad.push(coach);
        squad.push(player(20, Some(150)));
        let lineup = Lineup::auto_pick(Formation::FourFourTwo, &squad);
        assert_eq!(lineup.starting[0], 20);
        assert!(!lineup.starting.contains(&99));
    }

    /// Tactics round-trip as data for save persistence.
    #[test]
    fn tactics_round_trip_as_data() {
        let squad: Vec<Player> = (0..15).map(|i| player(i, Some(120))).collect();
        let tactics = TeamTactics::auto(Formation::ThreeFiveTwo, &squad);
        let json = serde_json::to_string(&tactics).expect("serializable");
        let restored: TeamTactics = serde_json::from_str(&json).expect("deserializable");
        assert_eq!(restored, tactics);
    }
}
