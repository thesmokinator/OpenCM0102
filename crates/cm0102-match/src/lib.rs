// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! Match simulation. The engine shipped here is a **declared placeholder**:
//! it exists so the season loop and the UI can be exercised end to end
//! behind the `MatchEngine` trait. It is not a model of the original
//! CM 01/02 match engine; faithful behaviour is specified in
//! `docs/match-engine-spec.md` before any engine may claim to reproduce it.

use cm0102_core::{MatchContext, MatchEngine, MatchResult};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rand_distr::{Distribution, Poisson};

/// Placeholder engine: goals follow a Poisson distribution around fixed
/// home/away expectations. Deterministic for a given seed.
#[derive(Debug, Default)]
pub struct SimpleMatchEngine;

impl MatchEngine for SimpleMatchEngine {
    fn simulate(&self, ctx: &MatchContext) -> MatchResult {
        let mut rng = ChaCha8Rng::seed_from_u64(ctx.seed);
        let home_goals = sample_goals(&mut rng, 1.45);
        let away_goals = sample_goals(&mut rng, 1.10);
        let mut events = Vec::new();
        for goal in 0..home_goals {
            events.push(format!("{}' Home club scores!", 10 + goal * 17));
        }
        for goal in 0..away_goals {
            events.push(format!("{}' Away club scores!", 14 + goal * 19));
        }
        events.sort();
        MatchResult {
            home_goals,
            away_goals,
            events,
        }
    }
}

/// Draw a goal count from a Poisson distribution with mean `lambda`.
fn sample_goals(rng: &mut ChaCha8Rng, lambda: f64) -> u32 {
    Poisson::new(lambda)
        .expect("lambda is a positive constant")
        .sample(rng) as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use cm0102_core::GameDate;

    fn context(seed: u64) -> MatchContext {
        MatchContext {
            home_club_id: 1,
            away_club_id: 2,
            date: GameDate::SEASON_START,
            seed,
        }
    }

    /// The same seed always produces the same result.
    #[test]
    fn is_deterministic_per_seed() {
        let engine = SimpleMatchEngine;
        assert_eq!(engine.simulate(&context(42)), engine.simulate(&context(42)));
    }

    /// Events are produced exactly for the goals scored.
    #[test]
    fn events_match_goal_count() {
        let engine = SimpleMatchEngine;
        let result = engine.simulate(&context(7));
        assert_eq!(
            result.events.len() as u32,
            result.home_goals + result.away_goals
        );
    }

    /// Goal totals over many seeds stay in a plausible football range.
    #[test]
    fn goal_distribution_is_plausible() {
        let engine = SimpleMatchEngine;
        let total: u32 = (0..500)
            .map(|seed| {
                let result = engine.simulate(&context(seed));
                result.home_goals + result.away_goals
            })
            .sum();
        let average = f64::from(total) / 500.0;
        assert!(
            (1.5..=4.5).contains(&average),
            "average goals per match: {average}"
        );
    }
}
