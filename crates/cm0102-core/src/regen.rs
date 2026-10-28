// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! Regens: generation counts and attribute inheritance.
//!
//! Behaviour from `docs/season.md`: regen consumes the retired-staff
//! list and, when more than two entries are pending, generates that
//! count minus two new players. Unattached regens draw two uniform
//! 1–1500 values and one 1–500 value; attached regens inherit clamped
//! fields from the retiree with strength-ratio gates at one-half and
//! three-quarters and a 7,501 threshold branch.

/// How many regens to generate from `pending_retired` retired-staff
/// entries: the count minus two when more than two are pending, else
/// none.
pub fn regen_count(pending_retired: usize) -> usize {
    pending_retired.saturating_sub(2)
}

/// Upper bound of the two wide unattached draws.
pub const UNATTACHED_WIDE_MAX: u32 = 1500;
/// Upper bound of the narrow unattached draw.
pub const UNATTACHED_NARROW_MAX: u32 = 500;

/// Strength-ratio gates for attached inheritance.
pub const HALF_GATE_NUM: i64 = 1;
/// Denominator of the half gate.
pub const HALF_GATE_DEN: i64 = 2;
/// Strength-ratio gate at three quarters (numerator).
pub const THREE_QUARTER_GATE_NUM: i64 = 3;
/// Denominator of the three-quarter gate.
pub const THREE_QUARTER_GATE_DEN: i64 = 4;

/// Threshold branch inside attached inheritance.
pub const ATTACHED_THRESHOLD: i64 = 7_501;

/// One unattached regen draw: two uniform 1–1500 values and one
/// uniform 1–500 value, from explicit rolls so callers own the RNG.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnattachedDraws {
    /// First wide roll (1–1500).
    pub wide_a: u32,
    /// Second wide roll (1–1500).
    pub wide_b: u32,
    /// Narrow roll (1–500).
    pub narrow: u32,
}

/// Check explicit rolls against their documented ranges.
pub fn valid_unattached(draws: &UnattachedDraws) -> bool {
    (1..=UNATTACHED_WIDE_MAX).contains(&draws.wide_a)
        && (1..=UNATTACHED_WIDE_MAX).contains(&draws.wide_b)
        && (1..=UNATTACHED_NARROW_MAX).contains(&draws.narrow)
}

/// Attached inheritance of one clamped field from the retiree: the
/// retiree value scaled by the strength ratio, gated at one-half and
/// three-quarters, with the 7,501 threshold branch applied to the
/// scaled value, then clamped to `clamp_max`.
pub fn inherit_attached(
    retiree_value: i64,
    strength_num: i64,
    strength_den: i64,
    clamp_max: i64,
) -> i64 {
    let scaled = if strength_den <= 0 {
        0
    } else {
        retiree_value * strength_num / strength_den
    };
    let gated = if strength_num * HALF_GATE_DEN < HALF_GATE_NUM * strength_den {
        scaled / 2
    } else if strength_num * THREE_QUARTER_GATE_DEN < THREE_QUARTER_GATE_NUM * strength_den {
        scaled * 3 / 4
    } else {
        scaled
    };
    let branched = if gated >= ATTACHED_THRESHOLD {
        gated - ATTACHED_THRESHOLD
    } else {
        gated
    };
    branched.clamp(0, clamp_max)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Counts below three yield nothing; above, count minus two.
    #[test]
    fn counts_follow_count_minus_two() {
        assert_eq!(regen_count(0), 0);
        assert_eq!(regen_count(2), 0);
        assert_eq!(regen_count(3), 1);
        assert_eq!(regen_count(10), 8);
    }

    /// Range checks accept the documented draw bounds only.
    #[test]
    fn unattached_ranges_match_spec() {
        let ok = UnattachedDraws {
            wide_a: 1,
            wide_b: 1500,
            narrow: 500,
        };
        assert!(valid_unattached(&ok));
        let bad = UnattachedDraws {
            wide_a: 1501,
            wide_b: 1,
            narrow: 1,
        };
        assert!(!valid_unattached(&bad));
        let bad_narrow = UnattachedDraws {
            wide_a: 1,
            wide_b: 1,
            narrow: 501,
        };
        assert!(!valid_unattached(&bad_narrow));
    }

    /// Gates halve below one-half, take three quarters below
    /// three-quarters, and pass through at full strength.
    #[test]
    fn inheritance_gates_follow_strength_ratio() {
        assert_eq!(inherit_attached(100, 1, 4, 200), 12);
        assert_eq!(inherit_attached(100, 2, 3, 200), 49);
        assert_eq!(inherit_attached(100, 1, 1, 200), 100);
        assert_eq!(inherit_attached(20_000, 1, 1, 1_000_000), 20_000 - 7_501);
        assert_eq!(inherit_attached(100, 1, 1, 50), 50);
    }
}
