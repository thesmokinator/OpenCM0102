// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! Cup draws: per-round draw modes with strength seeding.
//!
//! Behaviour from `docs/competitions.md`: each round selects one of
//! three draw modes from its own configuration — a plain random
//! shuffle, a seeding-ordered draw, or a seeding-ordered draw with
//! same-nation avoidance. Seeded modes order entrants by seeding
//! value (highest first) then deal them into ties from a random
//! starting slot with a wraparound scan, flipping a coin for
//! home/away per tie; unpaired entrants (including byes carried in
//! from an earlier round) carry through as byes.

/// Per-round draw mode, from the round's own configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawMode {
    /// Fisher-Yates shuffle, then pair in order.
    Plain,
    /// Sort by seeding highest-first, rotate from a random start,
    /// then pair in order.
    Seeded,
    /// As [`DrawMode::Seeded`], but the wraparound scan skips a
    /// same-nation pairing when another opponent is available.
    SeededNationAvoidance,
}

/// One entrant to a cup-draw round.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entrant {
    /// Club id.
    pub club_id: u32,
    /// Seeding value: higher means stronger. Only used by the
    /// seeded modes; plain draws ignore it.
    pub seeding: i64,
    /// Nation id for same-nation avoidance. `None` opts out.
    pub nation: Option<u32>,
}

impl Entrant {
    /// Entrant without nation scope (plain and unscoped seeded draws).
    pub fn unscoped(club_id: u32, seeding: i64) -> Self {
        Self {
            club_id,
            seeding,
            nation: None,
        }
    }
}

/// One drawn tie.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CupTie {
    /// Home club id.
    pub home: u32,
    /// Away club id.
    pub away: u32,
}

/// Outcome of one drawn round.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrawOutcome {
    /// Drawn ties, in draw order.
    pub ties: Vec<CupTie>,
    /// Unpaired club ids: byes carried in plus the new odd one out.
    pub byes: Vec<u32>,
}

/// Seeding value for one club: squad-value term plus sixteen times
/// the club strength field plus the weighted best-fifteen player
/// ratings (see `docs/competitions.md`). League and cup ranking
/// share this key, sorted highest-first.
pub fn seeding_score(value_term: i64, strength: i64, best_fifteen_weighted: i64) -> i64 {
    value_term + 16 * strength + best_fifteen_weighted
}

/// Squad-value term: squad value divided by 400, or by 300 in the
/// alternative accounting mode.
pub fn value_term(squad_value: i64, alternative_accounting: bool) -> i64 {
    if alternative_accounting {
        squad_value / 300
    } else {
        squad_value / 400
    }
}

/// Draw one round: `entrants` plus `byes_in` (carried from an earlier
/// round) form the pot. An odd pot leaves one bye out. Deterministic
/// per `seed`.
pub fn draw_round(
    entrants: &[Entrant],
    byes_in: &[u32],
    mode: DrawMode,
    seed: u64,
) -> DrawOutcome {
    let mut rng = SplitMix64(seed);
    let mut pot: Vec<Entrant> = entrants.to_vec();
    for club_id in byes_in {
        if !pot.iter().any(|entrant| entrant.club_id == *club_id) {
            pot.push(Entrant::unscoped(*club_id, i64::MIN));
        }
    }
    if pot.len() < 2 {
        return DrawOutcome {
            ties: Vec::new(),
            byes: pot.iter().map(|entrant| entrant.club_id).collect(),
        };
    }
    let order: Vec<Entrant> = match mode {
        DrawMode::Plain => {
            fisher_yates(&mut pot, &mut rng);
            pot
        }
        DrawMode::Seeded | DrawMode::SeededNationAvoidance => {
            pot.sort_by(|a, b| b.seeding.cmp(&a.seeding).then(a.club_id.cmp(&b.club_id)));
            let start = (rng.next() % pot.len() as u64) as usize;
            pot.iter()
                .cycle()
                .skip(start)
                .take(pot.len())
                .copied()
                .collect()
        }
    };
    let avoid_same_nation = mode == DrawMode::SeededNationAvoidance;
    let mut remaining: Vec<Entrant> = order;
    let mut ties = Vec::new();
    while remaining.len() >= 2 {
        let first = remaining.remove(0);
        let partner = if avoid_same_nation {
            pick_avoiding_same_nation(&remaining, &first)
        } else {
            0
        };
        let second = remaining.remove(partner);
        let (home, away) = if rng.next() % 2 == 0 {
            (first.club_id, second.club_id)
        } else {
            (second.club_id, first.club_id)
        };
        ties.push(CupTie { home, away });
    }
    DrawOutcome {
        ties,
        byes: remaining.iter().map(|entrant| entrant.club_id).collect(),
    }
}

/// Index of the first entrant with a different nation (or no nation
/// scope), falling back to the head of the list when every pairing
/// would share a nation.
fn pick_avoiding_same_nation(remaining: &[Entrant], first: &Entrant) -> usize {
    let Some(first_nation) = first.nation else {
        return 0;
    };
    remaining
        .iter()
        .position(|entrant| entrant.nation != Some(first_nation))
        .unwrap_or(0)
}

/// Fisher-Yates shuffle over the pot.
fn fisher_yates(pot: &mut [Entrant], rng: &mut SplitMix64) {
    for i in (1..pot.len()).rev() {
        let j = (rng.next() % (i as u64 + 1)) as usize;
        pot.swap(i, j);
    }
}

/// SplitMix64: small deterministic RNG so draws need no dependencies
/// and reproduce per seed.
struct SplitMix64(u64);

impl SplitMix64 {
    /// Next pseudorandom value.
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Plain draw pairs every entrant with no bye on an even pot.
    #[test]
    fn plain_draw_pairs_even_pot() {
        let entrants: Vec<Entrant> =
            (1..=8).map(|id| Entrant::unscoped(id, 0)).collect();
        let outcome = draw_round(&entrants, &[], DrawMode::Plain, 7);
        assert_eq!(outcome.ties.len(), 4);
        assert!(outcome.byes.is_empty());
        let mut seen = HashSet::new();
        for tie in &outcome.ties {
            assert!(seen.insert(tie.home));
            assert!(seen.insert(tie.away));
        }
        assert_eq!(seen.len(), 8);
    }

    /// Same seed reproduces the same draw; another seed differs almost
    /// surely on an 8-pot.
    #[test]
    fn plain_draw_is_deterministic_per_seed() {
        let entrants: Vec<Entrant> =
            (1..=8).map(|id| Entrant::unscoped(id, 0)).collect();
        let first = draw_round(&entrants, &[], DrawMode::Plain, 42);
        let second = draw_round(&entrants, &[], DrawMode::Plain, 42);
        assert_eq!(first, second);
        let other = draw_round(&entrants, &[], DrawMode::Plain, 43);
        assert_ne!(first, other);
    }

    /// Odd pots leave exactly one bye out.
    #[test]
    fn odd_pot_leaves_one_bye() {
        let entrants: Vec<Entrant> =
            (1..=7).map(|id| Entrant::unscoped(id, 0)).collect();
        let outcome = draw_round(&entrants, &[], DrawMode::Plain, 3);
        assert_eq!(outcome.ties.len(), 3);
        assert_eq!(outcome.byes.len(), 1);
    }

    /// Carried-in byes join the pot instead of being dropped.
    #[test]
    fn carried_byes_join_the_pot() {
        let entrants: Vec<Entrant> =
            (1..=6).map(|id| Entrant::unscoped(id, 0)).collect();
        let outcome = draw_round(&entrants, &[99], DrawMode::Plain, 5);
        assert_eq!(outcome.ties.len(), 3);
        assert_eq!(outcome.byes.len(), 1);
        let mut seen: Vec<u32> = outcome
            .ties
            .iter()
            .flat_map(|tie| [tie.home, tie.away])
            .chain(outcome.byes.clone())
            .collect();
        seen.sort_unstable();
        assert_eq!(seen, vec![1, 2, 3, 4, 5, 6, 99]);
    }

    /// Seeding helper follows the documented key.
    #[test]
    fn seeding_score_sums_documented_terms() {
        assert_eq!(seeding_score(100, 10, 50), 100 + 160 + 50);
        assert_eq!(value_term(8000, false), 20);
        assert_eq!(value_term(9000, true), 30);
    }

    /// Seeded draws still pair the whole pot deterministically.
    #[test]
    fn seeded_draw_pairs_whole_pot() {
        let entrants: Vec<Entrant> =
            (1..=6).map(|id| Entrant::unscoped(id, i64::from(id))).collect();
        let outcome = draw_round(&entrants, &[], DrawMode::Seeded, 11);
        let repeat = draw_round(&entrants, &[], DrawMode::Seeded, 11);
        assert_eq!(outcome, repeat);
        assert_eq!(outcome.ties.len(), 3);
        assert!(outcome.byes.is_empty());
    }

    /// Nation avoidance never pairs same-nation clubs when an
    /// alternative opponent exists.
    #[test]
    fn nation_avoidance_splits_same_nation_pairs() {
        let entrants = vec![
            Entrant {
                club_id: 1,
                seeding: 100,
                nation: Some(10),
            },
            Entrant {
                club_id: 2,
                seeding: 90,
                nation: Some(10),
            },
            Entrant {
                club_id: 3,
                seeding: 80,
                nation: Some(20),
            },
            Entrant {
                club_id: 4,
                seeding: 70,
                nation: Some(20),
            },
        ];
        for seed in 0..50 {
            let outcome =
                draw_round(&entrants, &[], DrawMode::SeededNationAvoidance, seed);
            assert_eq!(outcome.ties.len(), 2);
            for tie in &outcome.ties {
                let home_nation = entrants
                    .iter()
                    .find(|entrant| entrant.club_id == tie.home)
                    .and_then(|entrant| entrant.nation);
                let away_nation = entrants
                    .iter()
                    .find(|entrant| entrant.club_id == tie.away)
                    .and_then(|entrant| entrant.nation);
                assert_ne!(home_nation, away_nation, "seed {seed}");
            }
        }
    }
}
