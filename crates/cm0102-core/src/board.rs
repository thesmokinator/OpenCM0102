// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! Board confidence: the 0–10,000 value, its six message bands and
//! the queued board outcomes.
//!
//! Behaviour from `docs/season.md`: the confidence value lives in
//! the manager-confidence file's main block alongside counted dated
//! sub-records, and shows in six bands (very disappointed below
//! 3,501; unsatisfied to 4,275; satisfied to 6,500; pleased to
//! 8,500; very pleased to 9,500; delighted above). Outcomes surface
//! as queued news events: votes of confidence, patience warnings
//! and sackings with cause variants. How results move the value is
//! still open, so deltas arrive as explicit signed steps from the
//! caller (editable rules later), never from a hardcoded formula.

use serde::{Deserialize, Serialize};

use crate::calendar::GameDate;

/// Maximum confidence value.
pub const CONFIDENCE_MAX: u16 = 10_000;

/// Band edges from the observed message mapping.
pub const VERY_DISAPPOINTED_MAX: u16 = 3_500;
/// Top of the unsatisfied band.
pub const UNSATISFIED_MAX: u16 = 4_275;
/// Top of the satisfied band.
pub const SATISFIED_MAX: u16 = 6_500;
/// Top of the pleased band.
pub const PLEASED_MAX: u16 = 8_500;
/// Top of the very-pleased band.
pub const VERY_PLEASED_MAX: u16 = 9_500;

/// Display band of a confidence value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfidenceBand {
    /// Below 3,501.
    VeryDisappointed,
    /// Up to 4,275.
    Unsatisfied,
    /// Up to 6,500.
    Satisfied,
    /// Up to 8,500.
    Pleased,
    /// Up to 9,500.
    VeryPleased,
    /// Above 9,500.
    Delighted,
}

/// Band for `value` on the 0–10,000 scale.
pub fn band(value: u16) -> ConfidenceBand {
    let value = value.min(CONFIDENCE_MAX);
    if value <= VERY_DISAPPOINTED_MAX {
        ConfidenceBand::VeryDisappointed
    } else if value <= UNSATISFIED_MAX {
        ConfidenceBand::Unsatisfied
    } else if value <= SATISFIED_MAX {
        ConfidenceBand::Satisfied
    } else if value <= PLEASED_MAX {
        ConfidenceBand::Pleased
    } else if value <= VERY_PLEASED_MAX {
        ConfidenceBand::VeryPleased
    } else {
        ConfidenceBand::Delighted
    }
}

/// Why a board sacked the manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SackCause {
    /// Results.
    Results,
    /// League position.
    LeaguePosition,
    /// Supporter pressure.
    SupporterPressure,
}

/// Board outcome queued as a news event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardOutcome {
    /// Public vote of confidence.
    VoteOfConfidence,
    /// Patience warning.
    PatienceWarning,
    /// Sacking with its cause variant.
    Sacking(SackCause),
}

/// One dated confidence sub-record from the persisted file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfidenceEntry {
    /// Date the entry was recorded.
    pub date: GameDate,
    /// Value on that date.
    pub value: u16,
}

/// Per-manager confidence: main-block value plus dated sub-records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagerConfidence {
    /// Current value on the 0–10,000 scale.
    pub value: u16,
    /// Dated history, oldest first.
    #[serde(default)]
    pub history: Vec<ConfidenceEntry>,
}

impl Default for ManagerConfidence {
    /// Neutral start: the top of the satisfied band.
    fn default() -> Self {
        Self::neutral()
    }
}

impl ManagerConfidence {
    /// Neutral start: the top of the satisfied band.
    pub fn neutral() -> Self {
        Self {
            value: SATISFIED_MAX,
            history: Vec::new(),
        }
    }

    /// Current display band.
    pub fn band(&self) -> ConfidenceBand {
        band(self.value)
    }

    /// Move the value by an explicit signed step (from editable
    /// rules, once observed), clamping to the scale and recording
    /// the dated entry.
    pub fn apply_step(&mut self, date: GameDate, step: i32) {
        let next = (i32::from(self.value) + step).clamp(0, i32::from(CONFIDENCE_MAX));
        self.value = next as u16;
        self.history.push(ConfidenceEntry { date, value: self.value });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Band edges match the documented message mapping exactly.
    #[test]
    fn band_edges_match_documented_mapping() {
        assert_eq!(band(0), ConfidenceBand::VeryDisappointed);
        assert_eq!(band(3_500), ConfidenceBand::VeryDisappointed);
        assert_eq!(band(3_501), ConfidenceBand::Unsatisfied);
        assert_eq!(band(4_275), ConfidenceBand::Unsatisfied);
        assert_eq!(band(4_276), ConfidenceBand::Satisfied);
        assert_eq!(band(6_500), ConfidenceBand::Satisfied);
        assert_eq!(band(6_501), ConfidenceBand::Pleased);
        assert_eq!(band(8_500), ConfidenceBand::Pleased);
        assert_eq!(band(8_501), ConfidenceBand::VeryPleased);
        assert_eq!(band(9_500), ConfidenceBand::VeryPleased);
        assert_eq!(band(9_501), ConfidenceBand::Delighted);
        assert_eq!(band(10_000), ConfidenceBand::Delighted);
    }

    /// Steps clamp to the scale and record dated entries.
    #[test]
    fn steps_clamp_and_record_history() {
        let mut confidence = ManagerConfidence::neutral();
        assert_eq!(confidence.band(), ConfidenceBand::Satisfied);
        let date = GameDate::new(2001, 8, 1);
        confidence.apply_step(date, 2_000);
        assert_eq!(confidence.value, 8_500);
        assert_eq!(confidence.history, vec![ConfidenceEntry { date, value: 8_500 }]);
        confidence.apply_step(date, 9_999);
        assert_eq!(confidence.value, CONFIDENCE_MAX);
        confidence.apply_step(date, -20_000);
        assert_eq!(confidence.value, 0);
        assert_eq!(confidence.band(), ConfidenceBand::VeryDisappointed);
    }
}
