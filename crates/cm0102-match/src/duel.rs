// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! Documented match-play mechanics: duel resolution, stamina drain,
//! pitch-grid distance bands and rare special events.
//!
//! Behaviour from `docs/match-engine-spec.md` (function-level
//! analysis): duels resolve as paired uniform rolls bounded by
//! `(30 - attribute)` terms with a times-six difference; stamina
//! drains by half a drawn value per action; position derives from
//! pitch-grid coordinates with distance bands at 260 and 376; rare
//! special events fire on a one-in-ten roll inside a narrow state
//! window. The exact rating and man-of-the-match field writers stay
//! open. Thresholds combining the scale constants are calibrated
//! placeholders until measured.

/// Base of the duel roll bound: rolls run `0..(30 - attribute)`.
pub const DUEL_BOUND_BASE: u32 = 30;
/// Difference multiplier applied to the paired-roll margin.
pub const DUEL_DIFF_SCALE: i32 = 6;
/// Scale family from the duel routine.
pub const DUEL_SCALE_TINY: f64 = 0.0025;
/// Unit scale of the duel routine.
pub const DUEL_SCALE_UNIT: f64 = 1.0;
/// Half scale of the duel routine.
pub const DUEL_SCALE_HALF: f64 = 0.5;
/// Double scale of the duel routine.
pub const DUEL_SCALE_DOUBLE: f64 = 2.0;
/// Break scale of the duel routine.
pub const DUEL_SCALE_BREAK: f64 = 10.5;

/// Margin (in times-six units) for a clean break past the defender;
/// smaller wins leave the shot contested. Calibrated placeholder.
pub const CLEAN_BREAK_MARGIN: i32 = 12;
/// Extra keeper-side bonus on contested shots. Calibrated placeholder.
pub const CONTESTED_KEEPER_BONUS: i32 = 15;

/// Near band edge in pitch-grid units.
pub const GRID_BAND_NEAR: u32 = 260;
/// Far band edge in pitch-grid units.
pub const GRID_BAND_FAR: u32 = 376;

/// One-in-ten gate of the rare special events.
pub const SPECIAL_EVENT_CHANCE: u32 = 10;

/// Who won a duel and by what times-six margin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DuelWinner {
    /// Attacker beat the duel.
    Attacker,
    /// Defender held the duel (ties hold).
    Defender,
}

/// Outcome of one paired-roll duel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DuelResult {
    /// Who won.
    pub winner: DuelWinner,
    /// `(roll_a - roll_b)` times six: magnitude of the break.
    pub margin6: i32,
}

/// Resolve one duel from explicit paired rolls: `roll_a` uniform in
/// `0..(30 - attacker_attr)`, `roll_b` uniform in `0..(30 -
/// defender_attr)`. Bounds shrink with quality, so the lower roll
/// wins: the attacker breaks on a negative raw gap, ties hold for
/// the defender.
pub fn resolve_duel(
    attacker_attr: u8,
    defender_attr: u8,
    roll_a: u32,
    roll_b: u32,
) -> DuelResult {
    let _ = (attacker_attr, defender_attr);
    let margin6 = (roll_b as i32 - roll_a as i32) * DUEL_DIFF_SCALE;
    DuelResult {
        winner: if margin6 > 0 {
            DuelWinner::Attacker
        } else {
            DuelWinner::Defender
        },
        margin6,
    }
}

/// Upper roll bound for an attribute: `30 - attribute` (floored at
/// one so every duel can resolve).
pub fn duel_bound(attribute: u8) -> u32 {
    DUEL_BOUND_BASE.saturating_sub(u32::from(attribute)).max(1)
}

/// Stamina drained by one action: half the drawn value.
pub fn stamina_drain(drawn: u32) -> u32 {
    drawn / 2
}

/// Shot range from pitch-grid coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridBand {
    /// Inside 260 grid units.
    Close,
    /// Between 260 and 376 grid units.
    Mid,
    /// Beyond 376 grid units.
    Far,
}

/// Classify a pitch-grid distance into its band.
pub fn grid_band(grid_units: u32) -> GridBand {
    if grid_units < GRID_BAND_NEAR {
        GridBand::Close
    } else if grid_units < GRID_BAND_FAR {
        GridBand::Mid
    } else {
        GridBand::Far
    }
}

/// Rare special-event gate: fires on a one-in-ten roll, and only
/// inside the narrow state window.
pub fn special_event(roll_of_ten: u32, in_window: bool) -> bool {
    in_window && roll_of_ten == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lower rolls break, ties and higher rolls hold, with the
    /// margin scaled times six.
    #[test]
    fn duel_margins_scale_times_six() {
        let win = resolve_duel(10, 10, 4, 7);
        assert_eq!(win.winner, DuelWinner::Attacker);
        assert_eq!(win.margin6, 18);
        let hold = resolve_duel(10, 10, 4, 4);
        assert_eq!(hold.winner, DuelWinner::Defender);
        assert_eq!(hold.margin6, 0);
        let beaten = resolve_duel(18, 5, 9, 2);
        assert_eq!(beaten.winner, DuelWinner::Defender);
        assert_eq!(beaten.margin6, -42);
    }

    /// Bounds shrink with attribute quality and never hit zero.
    #[test]
    fn duel_bounds_follow_thirty_minus_attribute() {
        assert_eq!(duel_bound(20), 10);
        assert_eq!(duel_bound(0), 30);
        assert_eq!(duel_bound(30), 1);
    }

    /// Drains halve the drawn value, rounding down.
    #[test]
    fn drains_take_half_the_drawn_value() {
        assert_eq!(stamina_drain(5), 2);
        assert_eq!(stamina_drain(0), 0);
        assert_eq!(stamina_drain(10), 5);
    }

    /// Bands split at the documented grid edges.
    #[test]
    fn bands_split_at_documented_edges() {
        assert_eq!(grid_band(0), GridBand::Close);
        assert_eq!(grid_band(259), GridBand::Close);
        assert_eq!(grid_band(260), GridBand::Mid);
        assert_eq!(grid_band(375), GridBand::Mid);
        assert_eq!(grid_band(376), GridBand::Far);
    }

    /// Specials need both the one-in-ten roll and the window.
    #[test]
    fn specials_need_roll_and_window() {
        assert!(special_event(0, true));
        assert!(!special_event(1, true));
        assert!(!special_event(0, false));
    }
}
