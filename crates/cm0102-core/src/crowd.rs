// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! Match attendance: a crowd from the competition's reputation,
//! the weather over the ground, and the fixture seed.
//!
//! The original's crowd formula was not observed: this is our
//! documented reconstruction (editable data). Reputation is real
//! data (0-20 per competition); the curve below turns it into a
//! base crowd, matchday variance and rain move it, and the seed
//! keeps it deterministic.

use crate::weather::MatchWeather;

/// Base crowd every league fixture draws.
pub const CROWD_BASE: u32 = 400;
/// Crowd added per squared reputation point: reputation 20 fills
/// a big ground, reputation 3 a village one.
pub const CROWD_REP_SQUARE: u32 = 90;
/// Matchday variance either way, in percent.
pub const CROWD_VARIANCE_PCT: u32 = 20;
/// Crowd kept back by showers or worse, in percent.
pub const CROWD_WET_KEEP_PCT: u32 = 90;

/// Attendance for a fixture: reputation curve, deterministic
/// ±20% variance from the seed, and rain keeping fans at home.
pub fn crowd(reputation: i16, weather: &MatchWeather, seed: u64) -> u32 {
    let rep = reputation.clamp(0, 25) as u32;
    let base = CROWD_BASE + rep * rep * CROWD_REP_SQUARE;
    let span = 2 * CROWD_VARIANCE_PCT + 1;
    let pct = 100 - CROWD_VARIANCE_PCT + mix(seed) % span;
    let mut total = base * pct / 100;
    if weather.precip >= 3 {
        total = total * CROWD_WET_KEEP_PCT / 100;
    }
    total
}

/// Tiny deterministic mixer so crowds need no RNG dependency.
fn mix(seed: u64) -> u32 {
    let mut x = seed.wrapping_add(0x9e3779b97f4a7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
    ((x ^ (x >> 31)) % 1000) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crowds_are_deterministic() {
        let weather = MatchWeather::default();
        assert_eq!(crowd(18, &weather, 7), crowd(18, &weather, 7));
    }

    #[test]
    fn reputation_orders_crowds() {
        let weather = MatchWeather::default();
        let top = crowd(20, &weather, 42);
        let mid = crowd(12, &weather, 42);
        let low = crowd(4, &weather, 42);
        assert!(top > 25_000, "top flight: {top}");
        assert!((8_000..20_000).contains(&mid), "mid: {mid}");
        assert!(low < 5_000, "lower league: {low}");
    }

    #[test]
    fn rain_keeps_fans_home() {
        let dry = MatchWeather::default();
        let wet = MatchWeather {
            precip: 5,
            ..MatchWeather::default()
        };
        assert!(crowd(18, &wet, 99) < crowd(18, &dry, 99));
    }
}
