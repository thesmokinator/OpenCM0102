// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! Match simulation: minute-by-minute chance resolution.
//!
//! The mechanism follows the observed original-game flow documented in
//! `docs/match-engine-spec.md`: uniform rolls over attributes on a
//! 0–200 scale, competing rolls selecting the shot category, a keeper
//! contest, and a distance falloff on the goal test. The event set
//! (goals, penalties, missed penalties, own goals, bookings,
//! dismissals, injuries, substitutions) and the per-player ratings
//! mirror the original's match record; rates, weights and the rating
//! formula are calibrated placeholders until measured. Each is a
//! named constant in this file.

use cm0102_core::{
    EventKind, MatchContext, MatchEngine, MatchEvent, MatchInjury, MatchPlayer, MatchResult,
    MatchStats, Mentality, MinuteStats,
    Shootout, ATTR_AGGRESSION, ATTR_ANTICIPATION, ATTR_ATTACKER, ATTR_DEFENDER, ATTR_DIRTINESS,
    ATTR_FINISHING, ATTR_HANDLING, ATTR_HEADING, ATTR_INJURY_PRONENESS, ATTR_JUMPING,
    ATTR_LONG_SHOTS, ATTR_ONE_ON_ONES, ATTR_PASSING, ATTR_PENALTIES, ATTR_REFLEXES, ATTR_TACKLING,
};

pub mod commentary;
pub mod duel;
pub use commentary::{
    CfgTokens, CommentaryPack, MatchCommentary, RenderedLine, TextHint, render, render_match,
};
pub use duel::{DuelResult, DuelWinner, GridBand, grid_band};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// Base chance probability per team per minute, in permille.
/// Rebalanced for the duel gate (roughly half of all chances end in
/// a tackle), so post-duel shot volume matches the calibrated range.
const CHANCE_PER_MILLE: u32 = 88;
/// Strength adjustment bounds applied to the chance rate.
const RATE_CLAMP: (f32, f32) = (0.7, 1.4);
/// Chance-rate multiplier per team mentality.
const ATTACKING_MENTALITY_RATE: f32 = 1.15;
/// Chance-rate multiplier for a defensive mentality.
const DEFENSIVE_MENTALITY_RATE: f32 = 0.85;
/// Chance-rate dampener when the opponent sits deep.
const BEHIND_BALL_DAMPEN: f32 = 0.9;
/// Chance-rate lift per point of average side morale.
const MORALE_RATE_STEP: f32 = 0.03;
/// Chance-rate dampener in downpour, sleet or snow.
const WET_CHANCE_DAMPEN: f32 = 0.94;
/// Chance-rate dampener in a gale.
const GALE_CHANCE_DAMPEN: f32 = 0.95;
/// Closest shooting distance in metres.
const MIN_DISTANCE: u32 = 5;
/// Distance spread above the minimum, in metres.
const DISTANCE_SPREAD: u32 = 28;
/// One-on-one chances only happen inside this distance.
const ONE_ON_ONE_RANGE: u32 = 12;
/// Flat bonus added to the shooter's side of the goal test.
const SHOT_BONUS: i32 = 30;
/// Extra penalty added to the keeper's side of the goal test.
const KEEPER_BONUS: i32 = 40;
/// Quadratic distance falloff divisor: penalty = metres² / this.
const FALLOFF_DIVISOR: i32 = 8;
/// Penalty-kick probability per match, in percent.
const PENALTY_PERCENT: u32 = 9;
/// Flat bonus added to the taker's penalty roll.
const PENALTY_BONUS: u32 = 40;
/// Goals turned into own goals, in percent.
const OWN_GOAL_PERCENT: u32 = 2;
/// Booking probability per team per minute, in permille.
const BOOKING_PER_MILLE: u32 = 18;
/// Bookings that are straight reds, in percent.
const STRAIGHT_RED_PERCENT: u32 = 2;
/// Booked players tread carefully: their misconduct weight is divided.
const BOOKED_CAUTION_DIVISOR: u32 = 4;
/// Injury probability per team per minute, in permille.
const INJURY_PER_MILLE: u32 = 1;
/// Longest injury layoff in days (layoffs run 1..=this).
const INJURY_MAX_DAYS: u32 = 15;
/// Minutes for planned tactical substitutions.
const SUB_MINUTES: [u32; 2] = [63, 76];
/// Substitutions allowed per side per match.
const MAX_SUBS: u32 = 3;
/// Extra-time halves in minutes.
const EXTRA_TIME_HALF: u32 = 15;
/// Kicks per side before sudden death.
const SHOOTOUT_ROUNDS: usize = 5;
/// Base rating in deci-points (70 = 7.0).
const BASE_RATING: i32 = 70;
/// Rating floor/ceiling in deci-points.
const RATING_CLAMP: (i32, i32) = (50, 100);
/// Completed passes per team per minute: base plus a roll spread.
const PASSES_BASE: u32 = 3;
/// Pass spread above the base per team per minute.
const PASSES_SPREAD: u32 = 4;
/// Tackles won probability per team per minute, in permille.
const TACKLE_PER_MILLE: u32 = 200;
/// Headers won probability per team per minute, in permille.
const HEADER_PER_MILLE: u32 = 150;
/// Interception probability per team per minute, in permille.
const INTERCEPT_PER_MILLE: u32 = 80;
/// Offside probability per team per minute, in permille.
const OFFSIDE_PER_MILLE: u32 = 15;
/// Throw-in probability per team per minute, in permille.
const THROW_PER_MILLE: u32 = 220;
/// Free-kick probability per team per minute, in permille.
const FREEKICK_PER_MILLE: u32 = 120;
/// Corner probability per team per minute, in permille.
const CORNER_PER_MILLE: u32 = 60;
/// Goals scored via the frame, in percent of goals.
const POST_IN_PERCENT: u32 = 8;
/// Off-target shots hitting the frame, in percent.
const WOODWORK_PERCENT: u32 = 12;
/// Off-target shots blocked by a defender, in percent.
const BLOCKED_PERCENT: u32 = 15;

/// How a chance is finished: the winning category of the competing
/// attribute rolls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShotKind {
    /// Beat the keeper one on one (close range only).
    OneOnOne,
    /// Placed finish.
    Placed,
    /// Driven / long-range strike.
    Driven,
    /// Header.
    Header,
}

/// One player taking part plus their match record so far.
#[derive(Debug, Clone)]
struct Tracker {
    /// The player.
    player: MatchPlayer,
    /// Minute they came on (0 for starters).
    on_minute: u32,
    /// Minute they went off, if they did.
    off_minute: Option<u32>,
    /// Booked.
    yellow: bool,
    /// Dismissed.
    red: bool,
    /// Goals scored.
    goals: u8,
    /// Missed a penalty.
    missed_pen: bool,
    /// Scored an own goal.
    own_goal: bool,
    /// Shots attempted.
    shots: u16,
    /// Shots on target.
    on_target: u16,
    /// Tackles won.
    tackles: u16,
    /// Passes completed.
    passes: u16,
    /// Headers won.
    headers: u16,
    /// Interceptions made.
    interceptions: u16,
    /// Times flagged offside.
    offsides: u16,
}

impl Tracker {
    /// Starter entering at kick-off.
    fn starter(player: MatchPlayer) -> Self {
        Self {
            player,
            on_minute: 0,
            off_minute: None,
            yellow: false,
            red: false,
            goals: 0,
            missed_pen: false,
            own_goal: false,
            shots: 0,
            on_target: 0,
            tackles: 0,
            passes: 0,
            headers: 0,
            interceptions: 0,
            offsides: 0,
        }
    }

    /// Live performance score driving tactical substitutions.
    fn performance(&self) -> i32 {
        i32::from(self.player.current_ability) + i32::from(self.goals) * 8
            - if self.yellow { 15 } else { 0 }
    }
}

/// One side's evolving match state.
#[derive(Debug, Default)]
struct Side {
    /// Players on the pitch, shirt order.
    pitch: Vec<Tracker>,
    /// Named substitutes remaining.
    bench: Vec<MatchPlayer>,
    /// Players who went off (dismissed, injured or substituted).
    departed: Vec<Tracker>,
    /// Substitutions used.
    subs_used: u32,
    /// Goals conceded (for clean-sheet ratings).
    conceded: u32,
    /// Whether the side won (for ratings).
    won: bool,
}

impl Side {
    /// Build from starters and named substitutes.
    fn new(players: &[MatchPlayer], subs: &[MatchPlayer]) -> Self {
        Self {
            pitch: players.iter().cloned().map(Tracker::starter).collect(),
            bench: subs.to_vec(),
            departed: Vec::new(),
            subs_used: 0,
            conceded: 0,
            won: false,
        }
    }

    /// Remove the tracker at `index` from the pitch, keeping their
    /// record for the ratings. Dismissals flag them red.
    fn send_off(&mut self, index: usize, minute: u32, red: bool) {
        if index < self.pitch.len() {
            let mut tracker = self.pitch.remove(index);
            tracker.off_minute = Some(minute);
            tracker.red = red;
            self.departed.push(tracker);
        }
    }

    /// Replace the tracker at `index` with the strongest remaining
    /// substitute. Returns the (outgoing, incoming) names, if made.
    fn substitute(&mut self, index: usize, minute: u32) -> Option<(String, String)> {
        if self.subs_used >= MAX_SUBS || index >= self.pitch.len() {
            return None;
        }
        let best = self
            .bench
            .iter()
            .enumerate()
            .max_by_key(|(_, player)| player.current_ability)
            .map(|(i, _)| i)?;
        let incoming = self.bench.remove(best);
        let mut outgoing = self.pitch.remove(index);
        outgoing.off_minute = Some(minute);
        let name = outgoing.player.name.clone();
        self.departed.push(outgoing);
        self.pitch.insert(
            index.min(self.pitch.len()),
            Tracker {
                player: incoming.clone(),
                on_minute: minute,
                off_minute: None,
                yellow: false,
                red: false,
                goals: 0,
                missed_pen: false,
                own_goal: false,
                shots: 0,
                on_target: 0,
                tackles: 0,
                passes: 0,
                headers: 0,
                interceptions: 0,
                offsides: 0,
            },
        );
        self.subs_used += 1;
        Some((name, incoming.name))
    }
}

/// Minute-by-minute engine: chances, bookings, injuries and
/// substitutions resolve per team per minute; ratings derive from the
/// recorded events. Deterministic for a given seed.
#[derive(Debug, Default)]
pub struct SimulationEngine;

impl MatchEngine for SimulationEngine {
    fn simulate(&self, ctx: &MatchContext) -> MatchResult {
        let mut rng = ChaCha8Rng::seed_from_u64(ctx.seed);
        let mut result = MatchResult {
            home_goals: 0,
            away_goals: 0,
            events: Vec::new(),
            stats: MatchStats::default(),
            ratings: Vec::new(),
            shootout: None,
            injuries: Vec::new(),
            first_half_end: 45,
            timeline: Vec::new(),
            weather: ctx.weather,
            referee: Some(ctx.referee.clone()),
            attendance: None,
        };
        let mut events: Vec<MatchEvent> = Vec::new();
        let mut sides = [
            Side::new(&ctx.home_players, &ctx.home_subs),
            Side::new(&ctx.away_players, &ctx.away_subs),
        ];
        let (mut minutes, first_half_end) = full_time(&mut rng);
        result.first_half_end = first_half_end;
        events.push(whistle(1, EventKind::Kickoff, 52));
        play_period(&mut rng, ctx, &mut sides, 1, minutes, &mut result, &mut events);
        if roll(&mut rng, 100) < PENALTY_PERCENT {
            let minute = roll(&mut rng, minutes) + 1;
            resolve_penalty(&mut rng, &mut sides, minute, &mut result, &mut events);
        }
        events.push(whistle(first_half_end, EventKind::HalfTime, 433));
        if ctx.extra_time_if_level && result.home_goals == result.away_goals {
            play_period(
                &mut rng,
                ctx,
                &mut sides,
                minutes + 1,
                minutes + 2 * EXTRA_TIME_HALF,
                &mut result,
                &mut events,
            );
            minutes += 2 * EXTRA_TIME_HALF;
            if result.home_goals == result.away_goals {
                result.shootout = Some(shootout(&mut rng, &sides, minutes + 1, &mut events));
            }
        }
        sides[0].conceded = result.away_goals;
        sides[1].conceded = result.home_goals;
        sides[0].won = result.home_goals > result.away_goals;
        sides[1].won = result.away_goals > result.home_goals;
        result.stats = fold_timeline(&result.timeline);
        for (home, side) in [&sides[0], &sides[1]].into_iter().enumerate() {
            for tracker in side.pitch.iter().chain(side.departed.iter()) {
                result.ratings.push(rate(tracker, side, home == 0, minutes));
            }
        }
        events.push(whistle(minutes, EventKind::FullTime, 193));
        events.sort_by_key(|event| event.minute);
        result.events = events;
        result
    }
}

/// 90 minutes plus stoppage time per half, with the minute the
/// first half ended.
fn full_time(rng: &mut ChaCha8Rng) -> (u32, u32) {
    let first_half = 1 + roll(rng, 2);
    let second_half = 2 + roll(rng, 3);
    (90 + first_half + second_half, 45 + first_half)
}

/// Play minutes `first..=last` for both sides: bookings, injuries,
/// planned subs and chances.
#[allow(clippy::too_many_arguments)]
fn play_period(
    rng: &mut ChaCha8Rng,
    ctx: &MatchContext,
    sides: &mut [Side; 2],
    first: u32,
    last: u32,
    result: &mut MatchResult,
    events: &mut Vec<MatchEvent>,
) {
    for minute in first..=last {
        let mut played = MinuteStats::default();
        for home in [true, false] {
            // sides[0] is home: true -> 0.
            let index = usize::from(!home);
            maybe_book(
                rng,
                &mut sides[index],
                home,
                minute,
                ctx.referee.strictness,
                events,
            );
            maybe_injure(rng, &mut sides[index], home, minute, events, result);
            maybe_substitute(&mut sides[index], home, minute, events);
            ambient_minute(rng, &mut sides[index], &mut played, home);
            if chance_falls(rng, ctx, sides, home) {
                resolve_chance(rng, sides, home, minute, result, &mut played, events);
            }
        }
        result.timeline.push(played);
    }
}

/// Ambient minute: passes, tackles, headers, interceptions,
/// offsides and restarts resolve per team per minute with
/// attribute-weighted credit. The schema mirrors the original
/// ten-counter block; the per-minute rates are calibrated
/// placeholders.
fn ambient_minute(
    rng: &mut ChaCha8Rng,
    side: &mut Side,
    played: &mut MinuteStats,
    home: bool,
) {
    if side.pitch.is_empty() {
        return;
    }
    let acc = if home { &mut played.home } else { &mut played.away };
    let passes = PASSES_BASE + roll(rng, PASSES_SPREAD);
    acc.passes += passes as u16;
    for _ in 0..passes {
        if let Some(index) = pick_weighted(rng, &side.pitch, ATTR_PASSING) {
            side.pitch[index].passes += 1;
        }
    }
    if roll(rng, 1000) < TACKLE_PER_MILLE {
        acc.tackles += 1;
        if let Some(index) = pick_weighted(rng, &side.pitch, ATTR_TACKLING) {
            side.pitch[index].tackles += 1;
        }
    }
    if roll(rng, 1000) < HEADER_PER_MILLE {
        acc.headers += 1;
        if let Some(index) = pick_weighted(rng, &side.pitch, ATTR_HEADING) {
            side.pitch[index].headers += 1;
        }
    }
    if roll(rng, 1000) < INTERCEPT_PER_MILLE {
        if let Some(index) = pick_weighted(rng, &side.pitch, ATTR_ANTICIPATION) {
            side.pitch[index].interceptions += 1;
        }
    }
    if roll(rng, 1000) < OFFSIDE_PER_MILLE {
        acc.offsides += 1;
        if let Some(index) = pick_weighted(rng, &side.pitch, ATTR_ATTACKER) {
            side.pitch[index].offsides += 1;
        }
    }
    if roll(rng, 1000) < THROW_PER_MILLE {
        acc.throw_ins += 1;
    }
    if roll(rng, 1000) < FREEKICK_PER_MILLE {
        acc.free_kicks += 1;
    }
    if roll(rng, 1000) < CORNER_PER_MILLE {
        acc.corners += 1;
    }
}

/// Pitch index weighted by one attribute (fitness-scaled, +1 so
/// everyone stays eligible). `None` when nobody plays.
fn pick_weighted(rng: &mut ChaCha8Rng, pitch: &[Tracker], index: usize) -> Option<usize> {
    if pitch.is_empty() {
        return None;
    }
    let total: u32 = pitch
        .iter()
        .map(|tracker| bound(&tracker.player, index) + 1)
        .sum();
    let mut draw = roll(rng, total);
    for (i, tracker) in pitch.iter().enumerate() {
        let weight = bound(&tracker.player, index) + 1;
        if draw < weight {
            return Some(i);
        }
        draw -= weight;
    }
    Some(pitch.len() - 1)
}

/// Penalty shootout: five rounds with early decision, then sudden
/// death. Taker order follows penalty ability among the players on
/// the pitch, cycling when exhausted.
fn shootout(
    rng: &mut ChaCha8Rng,
    sides: &[Side; 2],
    minute: u32,
    events: &mut Vec<MatchEvent>,
) -> Shootout {
    let takers: [Vec<&Tracker>; 2] = [
        order_takers(&sides[0]),
        order_takers(&sides[1]),
    ];
    let (mut scored, mut taken) = ([0u8; 2], [0u8; 2]);
    let mut kick = 0;
    loop {
        let side = kick % 2;
        let order = &takers[side];
        if !order.is_empty() {
            let taker = order[(taken[side] as usize) % order.len()];
            taken[side] += 1;
            if roll(rng, 200) < bound(&taker.player, ATTR_PENALTIES) + PENALTY_BONUS {
                scored[side] += 1;
            }
        } else {
            taken[side] += 1;
        }
        kick += 1;
        if decided(scored, taken) {
            break;
        }
    }
    let home_wins = scored[0] > scored[1];
    events.push(MatchEvent {
        minute,
        home: home_wins,
        kind: EventKind::Shootout,
        player: if home_wins { "Home" } else { "Away" }.to_string(),
        other: Some(format!("{}-{}", scored[0], scored[1])),
        commentary_id: Some(196),
        distance_yards: None,
    });
    Shootout {
        home_scored: scored[0],
        away_scored: scored[1],
        home_wins,
    }
}

/// Players on the pitch ordered by penalty ability.
fn order_takers(side: &Side) -> Vec<&Tracker> {
    let mut takers: Vec<&Tracker> = side.pitch.iter().collect();
    takers.sort_by_key(|tracker| std::cmp::Reverse(bound(&tracker.player, ATTR_PENALTIES)));
    takers
}

/// True once the shootout has a winner: after both sides took the
/// same kicks, beyond the scheduled rounds with unequal scores, or
/// when a lead is unassailable within the scheduled rounds.
fn decided(scored: [u8; 2], taken: [u8; 2]) -> bool {
    if taken[0] != taken[1] {
        return false;
    }
    let rounds = SHOOTOUT_ROUNDS as u8;
    if taken[0] >= rounds {
        return scored[0] != scored[1];
    }
    let remaining = rounds - taken[0];
    scored[0] > scored[1] + remaining || scored[1] > scored[0] + remaining
}

/// Uniform roll in `0..max` (0 when `max` is 0).
fn roll(rng: &mut ChaCha8Rng, max: u32) -> u32 {
    if max == 0 {
        0
    } else {
        rng.random_range(0..max)
    }
}

/// Whether `home` (else away) carves out a chance this minute. The
/// base rate bends with relative eleven strength, squad size, team
/// mentality, the opponent's shape and the side's average morale.
fn chance_falls(rng: &mut ChaCha8Rng, ctx: &MatchContext, sides: &[Side; 2], home: bool) -> bool {
    if sides[usize::from(!home)].pitch.is_empty() || sides[usize::from(home)].pitch.is_empty() {
        return false;
    }
    let (own, opp, mentality, opp_bus, eleven) = if home {
        (
            ctx.home_avg_ca.max(1.0),
            ctx.away_avg_ca.max(1.0),
            &ctx.home_mentality,
            ctx.away_men_behind_ball,
            &ctx.home_players,
        )
    } else {
        (
            ctx.away_avg_ca.max(1.0),
            ctx.home_avg_ca.max(1.0),
            &ctx.away_mentality,
            ctx.home_men_behind_ball,
            &ctx.away_players,
        )
    };
    let ratio = (own / opp).clamp(RATE_CLAMP.0, RATE_CLAMP.1);
    let men = sides[usize::from(!home)].pitch.len() as f32 / 11.0;
    let mentality_rate = match mentality {
        Mentality::Attacking => ATTACKING_MENTALITY_RATE,
        Mentality::Normal => 1.0,
        Mentality::Defensive => DEFENSIVE_MENTALITY_RATE,
    };
    let morale = average_morale(eleven);
    let mut rate = CHANCE_PER_MILLE as f32 * ratio * men * mentality_rate;
    if opp_bus {
        rate *= BEHIND_BALL_DAMPEN;
    }
    rate *= 1.0 + morale * MORALE_RATE_STEP;
    rate *= weather_chance_mult(&ctx.weather);
    roll(rng, 1000) < rate.max(0.0) as u32
}

/// Chance multiplier for the match weather. Reconstructed: heavy
/// rain or snow scraps up play a little, gales move the long ball.
fn weather_chance_mult(weather: &cm0102_core::MatchWeather) -> f32 {
    let mut mult = 1.0;
    if weather.precip >= 5 {
        mult *= WET_CHANCE_DAMPEN;
    }
    if weather.wind >= 4 {
        mult *= GALE_CHANCE_DAMPEN;
    }
    mult
}

/// Average morale of a starting eleven, 0 when empty.
fn average_morale(eleven: &[MatchPlayer]) -> f32 {
    if eleven.is_empty() {
        return 0.0;
    }
    eleven.iter().map(|player| f32::from(player.morale)).sum::<f32>() / eleven.len() as f32
}

/// Attribute bound for rolls: the 0–200 skill scaled by fitness.
fn bound(player: &MatchPlayer, index: usize) -> u32 {
    (player.skill(index) as f32 * player.condition_scale()) as u32
}

/// Book a player, weighted by dirtiness and aggression. A second
/// yellow — or a straight red — dismisses them. Strict referees
/// reach for the pocket more, lenient ones wave play on.
fn maybe_book(
    rng: &mut ChaCha8Rng,
    side: &mut Side,
    home: bool,
    minute: u32,
    strictness: u8,
    events: &mut Vec<MatchEvent>,
) {
    let rate =
        BOOKING_PER_MILLE * u32::from(strictness) / u32::from(cm0102_core::NEUTRAL_STRICTNESS);
    if side.pitch.is_empty() || roll(rng, 1000) >= rate {
        return;
    }
    let weights: Vec<u32> = side
        .pitch
        .iter()
        .map(|tracker| {
            let weight = misconduct_weight(&tracker.player);
            if tracker.yellow {
                weight / BOOKED_CAUTION_DIVISOR
            } else {
                weight
            }
        })
        .collect();
    let total: u32 = weights.iter().sum();
    if total == 0 {
        return;
    }
    let mut draw = roll(rng, total);
    let mut index = 0;
    for (i, weight) in weights.iter().enumerate() {
        if draw < *weight {
            index = i;
            break;
        }
        draw -= weight;
    }
    let straight_red = roll(rng, 100) < STRAIGHT_RED_PERCENT;
    if side.pitch[index].yellow || straight_red {
        let name = side.pitch[index].player.name.clone();
        side.send_off(index, minute, true);
        events.push(MatchEvent {
            minute,
            home,
            kind: EventKind::Red,
            player: name,
            other: None,
            commentary_id: Some(if straight_red { 184 } else { 183 }),
            distance_yards: None,
        });
    } else {
        side.pitch[index].yellow = true;
        let name = side.pitch[index].player.name.clone();
        events.push(MatchEvent {
            minute,
            home,
            kind: EventKind::Yellow,
            player: name,
            other: None,
            commentary_id: Some(185),
            distance_yards: None,
        });
    }
}

/// Misconduct weight from dirtiness and aggression.
fn misconduct_weight(player: &MatchPlayer) -> u32 {
    bound(player, ATTR_DIRTINESS) + bound(player, ATTR_AGGRESSION) + 1
}

/// Injure a player, weighted by injury proneness, forcing a
/// substitution when one is available. The layoff runs 1..=15 days.
fn maybe_injure(
    rng: &mut ChaCha8Rng,
    side: &mut Side,
    home: bool,
    minute: u32,
    events: &mut Vec<MatchEvent>,
    result: &mut MatchResult,
) {
    if side.pitch.is_empty() || roll(rng, 1000) >= INJURY_PER_MILLE {
        return;
    }
    let total: u32 = side
        .pitch
        .iter()
        .map(|tracker| bound(&tracker.player, ATTR_INJURY_PRONENESS) + 1)
        .sum();
    let mut draw = roll(rng, total);
    let mut index = 0;
    for (i, tracker) in side.pitch.iter().enumerate() {
        let weight = bound(&tracker.player, ATTR_INJURY_PRONENESS) + 1;
        if draw < weight {
            index = i;
            break;
        }
        draw -= weight;
    }
    let victim = side.pitch[index].player.clone();
    let days_out = 1 + roll(rng, INJURY_MAX_DAYS);
    events.push(MatchEvent {
        minute,
        home,
        kind: EventKind::Injury,
        player: victim.name.clone(),
        other: Some(days_out.to_string()),
        commentary_id: Some(22),
        distance_yards: None,
    });
    result.injuries.push(MatchInjury {
        player_id: victim.id,
        name: victim.name.clone(),
        home,
        days_out,
    });
    let name = victim.name;
    if let Some((_, incoming)) = side.substitute(index, minute) {
        events.push(MatchEvent {
            minute,
            home,
            kind: EventKind::Sub,
            player: incoming,
            other: Some(name),
            commentary_id: Some(407),
            distance_yards: None,
        });
    } else {
        side.send_off(index, minute, false);
    }
}

/// Planned tactical substitution: the weakest outfield performer
/// makes way at the scheduled minutes.
fn maybe_substitute(side: &mut Side, home: bool, minute: u32, events: &mut Vec<MatchEvent>) {
    if !SUB_MINUTES.contains(&minute) || side.subs_used >= MAX_SUBS || side.bench.is_empty() {
        return;
    }
    let start = if side.pitch.len() > 1 { 1 } else { 0 };
    let weakest = (start..side.pitch.len())
        .min_by_key(|i| side.pitch[*i].performance());
    if let Some(index) = weakest {
        if let Some((outgoing, incoming)) = side.substitute(index, minute) {
            events.push(MatchEvent {
                minute,
                home,
                kind: EventKind::Sub,
                player: incoming,
                other: Some(outgoing),
                commentary_id: Some(27),
                distance_yards: None,
            });
        }
    }
}

/// Duel the shooter against a random defending outfielder in paired
/// rolls bounded by (30 - attribute). Returns `(contested,
/// held_by)`: `held_by` is `Some` index when the defender held the
/// duel (ties hold), and `contested` is true on a narrow attacking
/// win below the clean-break margin. Empty defences wave through.
fn contest_duel(rng: &mut ChaCha8Rng, shooter: &MatchPlayer, defence: &Side) -> (bool, Option<usize>) {
    let start = if defence.pitch.len() > 1 { 1 } else { 0 };
    if start >= defence.pitch.len() {
        return (false, None);
    }
    let span = (defence.pitch.len() - start) as u32;
    let index = start + roll(rng, span) as usize;
    let defender = &defence.pitch[index].player;
    let roll_a = roll(rng, duel::duel_bound(shooter.attribute(ATTR_ATTACKER)));
    let roll_b = roll(rng, duel::duel_bound(defender.attribute(ATTR_DEFENDER)));
    let result = duel::resolve_duel(
        shooter.attribute(ATTR_ATTACKER),
        defender.attribute(ATTR_DEFENDER),
        roll_a,
        roll_b,
    );
    match result.winner {
        DuelWinner::Defender => (false, Some(index)),
        DuelWinner::Attacker => (result.margin6 < duel::CLEAN_BREAK_MARGIN, None),
    }
}

/// Drain the shooter's condition by half a small drawn value after
/// each shot: stamina costs action.
fn drain_shooter(rng: &mut ChaCha8Rng, tracker: &mut Tracker) {
    let drain = duel::stamina_drain(roll(rng, 5));
    tracker.player.condition = tracker.player.condition.saturating_sub(drain.min(255) as u8);
}

/// Resolve one open-play chance: pick a shooter, roll the competing
/// category rolls, contest the keeper, apply the distance falloff.
fn resolve_chance(
    rng: &mut ChaCha8Rng,
    sides: &mut [Side; 2],
    home: bool,
    minute: u32,
    result: &mut MatchResult,
    played: &mut MinuteStats,
    events: &mut Vec<MatchEvent>,
) {
    let (attack, defend) = if home { (0, 1) } else { (1, 0) };
    let Some(shooter_index) = pick_shooter(rng, &sides[attack]) else {
        return;
    };
    let keeper_skill = keeper_skill(&sides[defend]);
    let shooter = sides[attack].pitch[shooter_index].player.clone();
    // Documented duel first: the shooter must beat a random defending
    // outfielder in paired rolls bounded by (30 - attribute). A held
    // duel ends in a block; a narrow win leaves the shot contested.
    let (contested, held_index) = contest_duel(rng, &shooter, &sides[defend]);
    if let Some(defender_index) = held_index {
        sides[attack].pitch[shooter_index].shots += 1;
        drain_shooter(rng, &mut sides[attack].pitch[shooter_index]);
        sides[defend].pitch[defender_index].tackles += 1;
        bump_shots(played, home, false);
        if home {
            played.away.tackles += 1;
        } else {
            played.home.tackles += 1;
        }
        events.push(MatchEvent {
            minute,
            home,
            kind: EventKind::ShotBlocked,
            player: shooter.name.clone(),
            other: None,
            commentary_id: Some(37),
            distance_yards: None,
        });
        return;
    }
    let distance = MIN_DISTANCE + roll(rng, DISTANCE_SPREAD);
    let (kind, winner) = winning_category(rng, &shooter, distance);
    sides[attack].pitch[shooter_index].shots += 1;
    drain_shooter(rng, &mut sides[attack].pitch[shooter_index]);
    let falloff = (distance * distance) as i32 / FALLOFF_DIVISOR;
    let mut keeper_roll = roll(rng, keeper_skill.max(1)) as i32;
    if contested {
        keeper_roll += duel::CONTESTED_KEEPER_BONUS;
    }
    if winner as i32 + SHOT_BONUS > keeper_roll + falloff + KEEPER_BONUS {
        sides[attack].pitch[shooter_index].goals += 1;
        sides[attack].pitch[shooter_index].on_target += 1;
        bump_shots(played, home, true);
        let (scored_home, scored_away) =
            score_goal(rng, sides, home, minute, &shooter.name, distance, events);
        result.home_goals += scored_home;
        result.away_goals += scored_away;
    } else if roll(rng, 200) < winner.saturating_sub((falloff.max(0) / 3) as u32) {
        bump_shots(played, home, true);
        sides[attack].pitch[shooter_index].on_target += 1;
        let keeper = sides[defend]
            .pitch
            .first()
            .map(|tracker| tracker.player.name.clone());
        events.push(MatchEvent {
            minute,
            home,
            kind: EventKind::ShotSaved,
            player: shooter.name.clone(),
            other: keeper,
            commentary_id: Some(saved_commentary_id(distance, &kind)),
            distance_yards: Some(distance.min(u32::from(u16::MAX)) as u16),
        });
    } else {
        bump_shots(played, home, false);
        let outcome = roll(rng, 100);
        let (kind, id) = if outcome < WOODWORK_PERCENT {
            let id = if roll(rng, 2) == 0 { 138 } else { 139 };
            (EventKind::ShotWoodwork, id)
        } else if outcome < WOODWORK_PERCENT + BLOCKED_PERCENT {
            let id = if matches!(kind, ShotKind::Header) { 38 } else { 37 };
            (EventKind::ShotBlocked, id)
        } else {
            (EventKind::ShotOffTarget, miss_commentary_id(rng, distance))
        };
        events.push(MatchEvent {
            minute,
            home,
            kind,
            player: shooter.name.clone(),
            other: None,
            commentary_id: Some(id),
            distance_yards: Some(distance.min(u32::from(u16::MAX)) as u16),
        });
    }
}

/// Commentary id for a saved shot by technique and distance.
fn saved_commentary_id(distance: u32, kind: &ShotKind) -> u16 {
    match kind {
        ShotKind::OneOnOne => 167,
        ShotKind::Header => 166,
        _ if distance > 25 => 164,
        _ => 166,
    }
}

/// Period whistle marker (kick-off, half/full time).
fn whistle(minute: u32, kind: EventKind, commentary_id: u16) -> MatchEvent {
    MatchEvent {
        minute,
        home: true,
        kind,
        player: String::new(),
        other: None,
        commentary_id: Some(commentary_id),
        distance_yards: None,
    }
}

/// Commentary id for a miss: nearer misses read closer.
fn miss_commentary_id(rng: &mut ChaCha8Rng, distance: u32) -> u16 {
    let near = roll(rng, 2) == 0;
    if distance <= ONE_ON_ONE_RANGE {
        if near {
            134
        } else {
            135
        }
    } else if distance <= 25 {
        if near {
            131
        } else {
            132
        }
    } else {
        127
    }
}

/// Pick the shooter by index, weighted by attacking rating and
/// finishing. Returns `None` with no outfield players.
fn pick_shooter(rng: &mut ChaCha8Rng, side: &Side) -> Option<usize> {
    let start = if side.pitch.len() > 1 { 1 } else { 0 };
    let total: u32 = side.pitch[start..].iter().map(|tracker| shooter_weight(&tracker.player)).sum();
    if total == 0 {
        return None;
    }
    let mut draw = roll(rng, total);
    for (i, tracker) in side.pitch.iter().enumerate().skip(start) {
        let weight = shooter_weight(&tracker.player);
        if draw < weight {
            return Some(i);
        }
        draw -= weight;
    }
    Some(side.pitch.len() - 1)
}

/// Shooter weight from attacking rating and finishing.
fn shooter_weight(player: &MatchPlayer) -> u32 {
    bound(player, ATTR_ATTACKER) + bound(player, ATTR_FINISHING) / 2 + 1
}

/// Keeper skill from the shirt-order keeper, else best handling.
fn keeper_skill(side: &Side) -> u32 {
    if let Some(keeper) = side.pitch.first() {
        return (bound(&keeper.player, ATTR_HANDLING) + bound(&keeper.player, ATTR_REFLEXES)) / 2;
    }
    100
}

/// Competing rolls: each shot category rolls its attribute, the best
/// roll wins (ties go to the earlier category).
fn winning_category(
    rng: &mut ChaCha8Rng,
    shooter: &MatchPlayer,
    distance: u32,
) -> (ShotKind, u32) {
    let mut candidates = Vec::with_capacity(4);
    if distance <= ONE_ON_ONE_RANGE {
        candidates.push((ShotKind::OneOnOne, roll(rng, bound(shooter, ATTR_ONE_ON_ONES).max(1))));
    }
    candidates.push((ShotKind::Placed, roll(rng, bound(shooter, ATTR_FINISHING).max(1))));
    candidates.push((ShotKind::Driven, roll(rng, bound(shooter, ATTR_LONG_SHOTS).max(1))));
    let aerial = (bound(shooter, ATTR_HEADING) + bound(shooter, ATTR_JUMPING)) / 2;
    candidates.push((ShotKind::Header, roll(rng, aerial.max(1))));
    let mut best = candidates[0];
    for candidate in candidates.into_iter().skip(1) {
        if candidate.1 > best.1 {
            best = candidate;
        }
    }
    best
}

/// Record a goal, possibly turning it into an own goal. Returns
/// the (home, away) goals to add: the attack always scores.
fn score_goal(
    rng: &mut ChaCha8Rng,
    sides: &mut [Side; 2],
    home: bool,
    minute: u32,
    scorer: &str,
    distance: u32,
    events: &mut Vec<MatchEvent>,
) -> (u32, u32) {
    let scored = if home { (1, 0) } else { (0, 1) };
    let yards = Some(distance.min(u32::from(u16::MAX)) as u16);
    if roll(rng, 100) < OWN_GOAL_PERCENT {
        let defend = usize::from(!home);
        let start = if sides[defend].pitch.len() > 1 { 1 } else { 0 };
        if start < sides[defend].pitch.len() {
            let span = (sides[defend].pitch.len() - start) as u32;
            let culprit = start + roll(rng, span) as usize;
            sides[defend].pitch[culprit].own_goal = true;
            let name = sides[defend].pitch[culprit].player.name.clone();
            events.push(MatchEvent {
                minute,
                home: !home,
                kind: EventKind::OwnGoal,
                player: name,
                other: None,
                commentary_id: Some(141),
                distance_yards: yards,
            });
        } else {
            events.push(MatchEvent {
                minute,
                home: !home,
                kind: EventKind::OwnGoal,
                player: String::new(),
                other: None,
                commentary_id: Some(141),
                distance_yards: yards,
            });
        }
    } else {
        let id = if roll(rng, 100) < POST_IN_PERCENT {
            if roll(rng, 2) == 0 {
                565
            } else {
                566
            }
        } else {
            191
        };
        events.push(MatchEvent {
            minute,
            home,
            kind: EventKind::Goal,
            player: scorer.to_string(),
            other: None,
            commentary_id: Some(id),
            distance_yards: yards,
        });
    }
    scored
}

/// Resolve a spot kick for a random side.
fn resolve_penalty(
    rng: &mut ChaCha8Rng,
    sides: &mut [Side; 2],
    minute: u32,
    result: &mut MatchResult,
    events: &mut Vec<MatchEvent>,
) {
    let home = roll(rng, 2) == 0;
    let side = usize::from(!home);
    let Some(taker_index) = sides[side]
        .pitch
        .iter()
        .enumerate()
        .max_by_key(|(_, tracker)| bound(&tracker.player, ATTR_PENALTIES))
        .map(|(i, _)| i)
    else {
        return;
    };
    let taker = sides[side].pitch[taker_index].player.clone();
    sides[side].pitch[taker_index].shots += 1;
    if roll(rng, 200) < bound(&taker, ATTR_PENALTIES) + PENALTY_BONUS {
        sides[side].pitch[taker_index].goals += 1;
        sides[side].pitch[taker_index].on_target += 1;
        bump_penalty_shot(result, minute, home, true);
        if home {
            result.home_goals += 1;
        } else {
            result.away_goals += 1;
        }
        events.push(MatchEvent {
            minute,
            home,
            kind: EventKind::Penalty,
            player: taker.name.clone(),
            other: None,
            commentary_id: Some(143),
            distance_yards: Some(12),
        });
    } else {
        sides[side].pitch[taker_index].missed_pen = true;
        bump_penalty_shot(result, minute, home, false);
        events.push(MatchEvent {
            minute,
            home,
            kind: EventKind::MissedPenalty,
            player: taker.name.clone(),
            other: None,
            commentary_id: Some(142),
            distance_yards: Some(12),
        });
    }
}

/// Count a shot (and optionally on target) for a side in the
/// current minute.
fn bump_shots(played: &mut MinuteStats, home: bool, on_target: bool) {
    let acc = if home { &mut played.home } else { &mut played.away };
    acc.shots += 1;
    if on_target {
        acc.on_target += 1;
    }
}

/// Count a retroactive penalty shot in its minute of the timeline.
fn bump_penalty_shot(result: &mut MatchResult, minute: u32, home: bool, on_target: bool) {
    let index = minute.saturating_sub(1) as usize;
    if let Some(played) = result.timeline.get_mut(index) {
        bump_shots(played, home, on_target);
    }
}

/// Full-time team counts from the timeline: one source of truth,
/// so live prefixes always agree with the final block.
fn fold_timeline(timeline: &[MinuteStats]) -> MatchStats {
    let mut stats = MatchStats::default();
    for played in timeline {
        stats.home_shots += u32::from(played.home.shots);
        stats.away_shots += u32::from(played.away.shots);
        stats.home_on_target += u32::from(played.home.on_target);
        stats.away_on_target += u32::from(played.away.on_target);
        stats.home_corners += u32::from(played.home.corners);
        stats.away_corners += u32::from(played.away.corners);
        stats.home_free_kicks += u32::from(played.home.free_kicks);
        stats.away_free_kicks += u32::from(played.away.free_kicks);
        stats.home_throw_ins += u32::from(played.home.throw_ins);
        stats.away_throw_ins += u32::from(played.away.throw_ins);
        stats.home_offsides += u32::from(played.home.offsides);
        stats.away_offsides += u32::from(played.away.offsides);
        stats.home_passes += u32::from(played.home.passes);
        stats.away_passes += u32::from(played.away.passes);
        stats.home_tackles += u32::from(played.home.tackles);
        stats.away_tackles += u32::from(played.away.tackles);
        stats.home_headers += u32::from(played.home.headers);
        stats.away_headers += u32::from(played.away.headers);
    }
    stats
}

/// Rate one player from their recorded match events.
fn rate(tracker: &Tracker, side: &Side, home: bool, minutes: u32) -> cm0102_core::PlayerRating {
    let mut rating = BASE_RATING + i32::from(tracker.goals) * 9;
    if tracker.yellow {
        rating -= 6;
    }
    if tracker.red {
        rating -= 15;
    }
    if tracker.missed_pen {
        rating -= 5;
    }
    if tracker.own_goal {
        rating -= 8;
    }
    if side.conceded == 0 && is_defender(&tracker.player) {
        rating += 4;
    }
    if side.won {
        rating += 2;
    }
    cm0102_core::PlayerRating {
        player_id: tracker.player.id,
        name: tracker.player.name.clone(),
        home,
        rating: rating.clamp(RATING_CLAMP.0, RATING_CLAMP.1) as u8,
        minutes: tracker
            .off_minute
            .unwrap_or(minutes)
            .saturating_sub(tracker.on_minute)
            .min(120) as u8,
        on_minute: tracker.on_minute.min(120) as u8,
        goals: tracker.goals,
        yellow: tracker.yellow,
        red: tracker.red,
        shots: tracker.shots,
        on_target: tracker.on_target,
        tackles: tracker.tackles,
        passes: tracker.passes,
        headers: tracker.headers,
        interceptions: tracker.interceptions,
        offsides: tracker.offsides,
    }
}

/// Defenders and keepers qualify for clean-sheet credit.
fn is_defender(player: &MatchPlayer) -> bool {
    use cm0102_core::{ATTR_DEFENDER, ATTR_DM, ATTR_GOALKEEPER, ATTR_SWEEPER};
    [ATTR_GOALKEEPER, ATTR_SWEEPER, ATTR_DEFENDER, ATTR_DM]
        .iter()
        .any(|index| player.attribute(*index) >= 12)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cm0102_core::{GameDate, PlayerRating};

    fn player(name: &str, finishing: u8) -> MatchPlayer {
        let mut attributes = vec![10u8; 54];
        attributes[ATTR_FINISHING] = finishing;
        attributes[ATTR_ATTACKER] = 12;
        MatchPlayer {
            id: 1,
            name: name.to_string(),
            current_ability: 150,
            attributes,
            condition: 100,
            morale: 0,
        }
    }

    fn eleven(prefix: &str, finishing: u8) -> Vec<MatchPlayer> {
        (0..11)
            .map(|i| {
                let mut attributes = vec![10u8; 54];
                attributes[ATTR_FINISHING] = finishing;
                attributes[ATTR_ATTACKER] = 12;
                attributes[ATTR_HANDLING] = 12;
                attributes[ATTR_REFLEXES] = 12;
                MatchPlayer {
                    id: i,
                    name: format!("{prefix} {i}"),
                    current_ability: 150,
                    attributes,
                    condition: 100,
                    morale: 0,
                }
            })
            .collect()
    }

    fn context(seed: u64) -> MatchContext {
        MatchContext {
            home_club_id: 1,
            away_club_id: 2,
            date: GameDate::SEASON_START,
            seed,
            home_avg_ca: 150.0,
            away_avg_ca: 150.0,
            home_players: eleven("Home", 12),
            away_players: eleven("Away", 12),
            home_subs: eleven("HSub", 10),
            away_subs: eleven("ASub", 10),
            home_mentality: Mentality::Normal,
            away_mentality: Mentality::Normal,
            home_men_behind_ball: false,
            away_men_behind_ball: false,
            extra_time_if_level: false,
            weather: cm0102_core::MatchWeather::default(),
            referee: cm0102_core::Referee::default(),
        }
    }

    /// Goal events (not bookings, subs or misses) match the scoreline.
    fn goal_events(result: &MatchResult) -> usize {
        result
            .events
            .iter()
            .filter(|event| {
                matches!(
                    event.kind,
                    EventKind::Goal | EventKind::Penalty | EventKind::OwnGoal
                )
            })
            .count()
    }

    /// The same seed always produces the same result.
    #[test]
    fn is_deterministic_per_seed() {
        let engine = SimulationEngine;
        assert_eq!(engine.simulate(&context(42)), engine.simulate(&context(42)));
    }

    /// Events are produced exactly for the goals scored.
    #[test]
    fn events_match_goal_count() {
        let engine = SimulationEngine;
        let result = engine.simulate(&context(7));
        assert_eq!(
            goal_events(&result) as u32,
            result.home_goals + result.away_goals
        );
    }

    /// One timeline entry per minute played, regulation and extra
    /// time alike.
    #[test]
    fn timeline_covers_every_minute() {
        let engine = SimulationEngine;
        for seed in 0..20 {
            let result = engine.simulate(&context(seed));
            assert!((93..=97).contains(&result.timeline.len()), "{}", result.timeline.len());
            let mut ctx = context(seed);
            ctx.extra_time_if_level = true;
            let cup = engine.simulate(&ctx);
            if cup.shootout.is_some() || cup.events.iter().any(|e| e.minute > 96) {
                assert!(cup.timeline.len() >= result.timeline.len());
            }
        }
    }

    /// Per-player counters sum exactly to the team block: every
    /// credit has one home.
    #[test]
    fn player_counters_sum_to_team_block() {
        let engine = SimulationEngine;
        for seed in 0..30 {
            let result = engine.simulate(&context(seed));
            for home in [true, false] {
                let side: Vec<_> = result.ratings.iter().filter(|r| r.home == home).collect();
                let sum = |get: fn(&&PlayerRating) -> u16| -> u32 {
                    side.iter().map(|r| u32::from(get(r))).sum()
                };
                let stats = result.stats;
                let (shots, on_target, tackles, passes, headers, offsides) = if home {
                    (stats.home_shots, stats.home_on_target, stats.home_tackles, stats.home_passes, stats.home_headers, stats.home_offsides)
                } else {
                    (stats.away_shots, stats.away_on_target, stats.away_tackles, stats.away_passes, stats.away_headers, stats.away_offsides)
                };
                assert_eq!(sum(|r| r.shots), shots, "seed {seed}");
                assert_eq!(sum(|r| r.on_target), on_target, "seed {seed}");
                assert_eq!(sum(|r| r.tackles), tackles, "seed {seed}");
                assert_eq!(sum(|r| r.passes), passes, "seed {seed}");
                assert_eq!(sum(|r| r.headers), headers, "seed {seed}");
                assert_eq!(sum(|r| r.offsides), offsides, "seed {seed}");
            }
        }
    }

    /// Ambient totals sit in plausible football ranges.
    #[test]
    fn ambient_totals_are_plausible() {
        let engine = SimulationEngine;
        let (mut passes, mut tackles, mut corners, mut fk, mut offs, mut throws, mut headers) =
            (0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32);
        let games = 100u32;
        for seed in 0..u64::from(games) {
            let stats = engine.simulate(&context(seed)).stats;
            passes += stats.home_passes + stats.away_passes;
            tackles += stats.home_tackles + stats.away_tackles;
            corners += stats.home_corners + stats.away_corners;
            fk += stats.home_free_kicks + stats.away_free_kicks;
            offs += stats.home_offsides + stats.away_offsides;
            throws += stats.home_throw_ins + stats.away_throw_ins;
            headers += stats.home_headers + stats.away_headers;
        }
        let avg = |total: u32| f64::from(total) / f64::from(games);
        assert!((600.0..=1200.0).contains(&avg(passes)), "passes {}", avg(passes));
        assert!((20.0..=60.0).contains(&avg(tackles)), "tackles {}", avg(tackles));
        assert!((4.0..=20.0).contains(&avg(corners)), "corners {}", avg(corners));
        assert!((10.0..=45.0).contains(&avg(fk)), "fk {}", avg(fk));
        assert!(avg(offs) <= 10.0, "offsides {}", avg(offs));
        assert!((20.0..=70.0).contains(&avg(throws)), "throw-ins {}", avg(throws));
        assert!((10.0..=60.0).contains(&avg(headers)), "headers {}", avg(headers));
    }

    /// Every emitted commentary id resolves in the built-in pack,
    /// and shot incidents (saves, misses, woodwork, blocks) occur.
    #[test]
    fn commentary_ids_resolve_in_builtin_pack() {
        use crate::commentary::CommentaryPack;
        let engine = SimulationEngine;
        let pack = CommentaryPack::builtin().expect("builtin parses");
        let mut shots = 0;
        for seed in 0..100 {
            let result = engine.simulate(&context(seed));
            for event in &result.events {
                let Some(id) = event.commentary_id else {
                    panic!("missing commentary id: {event:?}");
                };
                assert!(pack.event(id).is_some(), "unknown id {id}: {event:?}");
                if matches!(
                    event.kind,
                    EventKind::ShotSaved
                        | EventKind::ShotOffTarget
                        | EventKind::ShotWoodwork
                        | EventKind::ShotBlocked
                ) {
                    shots += 1;
                }
            }
        }
        assert!(shots > 100, "{shots}");
    }

    /// Side prefix of test elevens (`Home`/`HSub` vs `Away`/`ASub`).
    fn side_prefix(home: bool, name: &str) -> bool {
        if home {
            name.starts_with("Home") || name.starts_with("HSub")
        } else {
            name.starts_with("Away") || name.starts_with("ASub")
        }
    }

    /// Penalty takers and injured players belong to the incident's
    /// side: guards the home/away index wiring.
    #[test]
    fn incidents_credit_the_right_side() {
        let engine = SimulationEngine;
        let (mut pens, mut injuries) = (0, 0);
        for seed in 0..200 {
            let result = engine.simulate(&context(seed));
            for event in &result.events {
                if matches!(
                    event.kind,
                    EventKind::Penalty | EventKind::MissedPenalty
                ) {
                    assert!(side_prefix(event.home, &event.player), "{event:?}");
                    pens += 1;
                }
            }
            for injury in &result.injuries {
                assert!(side_prefix(injury.home, &injury.name), "{injury:?}");
                injuries += 1;
            }
        }
        assert!(pens > 0 && injuries > 0, "{pens} pens, {injuries} injuries");
    }

    /// Every goal was a shot on target: single counting, no double
    /// bumps.
    #[test]
    fn goals_never_exceed_shots_on_target() {
        let engine = SimulationEngine;
        for seed in 0..100 {
            let result = engine.simulate(&context(seed));
            assert!(result.home_goals <= result.stats.home_on_target);
            assert!(result.away_goals <= result.stats.away_on_target);
            assert!(result.stats.home_on_target <= result.stats.home_shots);
            assert!(result.stats.away_on_target <= result.stats.away_shots);
        }
    }

    /// The first half ends at 45 plus one or two stoppage minutes.
    #[test]
    fn first_half_end_covers_stoppage() {
        let engine = SimulationEngine;
        for seed in 0..50 {
            let end = engine.simulate(&context(seed)).first_half_end;
            assert!((46..=47).contains(&end), "{end}");
        }
    }

    /// Summing per-event goals rebuilds the scoreline, in minute
    /// order: the live commentary's running score is exact.
    #[test]
    fn event_goals_rebuild_scoreline() {
        let engine = SimulationEngine;
        for seed in 0..100 {
            let result = engine.simulate(&context(seed));
            let (mut home, mut away) = (0, 0);
            let mut last_minute = 0;
            for event in &result.events {
                assert!(event.minute >= last_minute, "out of order");
                last_minute = event.minute;
                let (h, a) = event.goals();
                home += h;
                away += a;
            }
            assert_eq!((home, away), (result.home_goals, result.away_goals));
        }
    }

    /// Goal totals over many seeds stay in a plausible football range.
    #[test]
    fn goal_distribution_is_plausible() {
        let engine = SimulationEngine;
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

    /// A stronger eleven outscores a weaker one on average.
    #[test]
    fn stronger_eleven_scores_more() {
        let engine = SimulationEngine;
        let (mut strong, mut weak) = (0u32, 0u32);
        for seed in 0..200 {
            let mut ctx = context(seed);
            ctx.home_players = eleven("Home", 18);
            ctx.away_players = eleven("Away", 6);
            ctx.home_avg_ca = 180.0;
            ctx.away_avg_ca = 120.0;
            let result = engine.simulate(&ctx);
            strong += result.home_goals;
            weak += result.away_goals;
        }
        assert!(strong > weak, "{strong} vs {weak}");
    }

    /// Close-range chances convert far better than long-range ones.
    #[test]
    fn conversion_falls_with_distance() {
        let close = conversion_at(6, 300);
        let far = conversion_at(30, 300);
        assert!(close > far * 2.0, "{close} vs {far}");
    }

    /// Conversion rate over `seeds` matches with every chance forced
    /// to `distance` metres (through the public simulation).
    fn conversion_at(distance: u32, seeds: u64) -> f64 {
        let (mut goals, mut shots) = (0u32, 0u32);
        for seed in 0..seeds {
            let mut rng = ChaCha8Rng::seed_from_u64(seed);
            let ctx = context(seed);
            let side = Side::new(&ctx.home_players, &[]);
            let keeper = keeper_skill(&side);
            let shooter = &ctx.home_players[5];
            for _ in 0..20 {
                let (_, winner) = winning_category(&mut rng, shooter, distance);
                shots += 1;
                let falloff = (distance * distance) as i32 / FALLOFF_DIVISOR;
                let keeper_roll = roll(&mut rng, keeper.max(1)) as i32;
                if winner as i32 + SHOT_BONUS > keeper_roll + falloff + KEEPER_BONUS {
                    goals += 1;
                }
            }
        }
        f64::from(goals) / f64::from(shots)
    }

    /// The dominant attribute wins most category rolls.
    #[test]
    fn dominant_attribute_wins_categories() {
        let mut rng = ChaCha8Rng::seed_from_u64(99);
        let striker = player("Striker", 20);
        let mut placed = 0;
        for _ in 0..500 {
            let (kind, _) = winning_category(&mut rng, &striker, 20);
            if kind == ShotKind::Placed {
                placed += 1;
            }
        }
        assert!(placed > 300, "{placed}");
    }

    /// Shots on target always include every goal.
    #[test]
    fn on_target_covers_goals() {
        let engine = SimulationEngine;
        for seed in 0..50 {
            let result = engine.simulate(&context(seed));
            assert!(result.stats.home_on_target >= result.home_goals);
            assert!(result.stats.away_on_target >= result.away_goals);
            assert!(result.stats.home_shots >= result.stats.home_on_target);
            assert!(result.stats.away_shots >= result.stats.away_on_target);
        }
    }

    /// Bookings, dismissals, injuries and substitutions all occur.
    #[test]
    fn match_incidents_occur() {
        let engine = SimulationEngine;
        let (mut cards, mut offs, mut injuries, mut subs) = (0, 0, 0, 0);
        for seed in 0..200 {
            for event in &engine.simulate(&context(seed)).events {
                match event.kind {
                    EventKind::Yellow | EventKind::Red => cards += 1,
                    EventKind::Injury => injuries += 1,
                    EventKind::Sub => subs += 1,
                    _ => {}
                }
                if event.kind == EventKind::Red {
                    offs += 1;
                }
            }
        }
        assert!(cards > 50, "{cards}");
        assert!(offs > 0, "no dismissals");
        assert!(injuries > 0, "no injuries");
        assert!(subs > 100, "{subs}");
    }

    /// Level cup ties always produce a shootout winner.
    #[test]
    fn shootout_decides_level_cup_ties() {
        let engine = SimulationEngine;
        let mut shootouts = 0;
        for seed in 0..100 {
            let mut ctx = context(seed);
            ctx.extra_time_if_level = true;
            let result = engine.simulate(&ctx);
            if result.home_goals == result.away_goals {
                let shootout = result.shootout.expect("shootout");
                assert_ne!(shootout.home_scored, shootout.away_scored);
                assert_eq!(
                    shootout.home_wins,
                    shootout.home_scored > shootout.away_scored
                );
                shootouts += 1;
            } else {
                assert!(result.shootout.is_none());
            }
        }
        assert!(shootouts > 0, "no shootout in 100 ties");
    }

    /// League matches never go to a shootout.
    #[test]
    fn league_matches_skip_shootouts() {
        let engine = SimulationEngine;
        for seed in 0..50 {
            assert!(engine.simulate(&context(seed)).shootout.is_none());
        }
    }

    /// Total goals for `home` over `seeds` matches with a tweak.
    fn total_goals(tweak: &dyn Fn(&mut MatchContext), home: bool, seeds: u64) -> u32 {
        let engine = SimulationEngine;
        (0..seeds)
            .map(|seed| {
                let mut ctx = context(seed);
                tweak(&mut ctx);
                let result = engine.simulate(&ctx);
                if home {
                    result.home_goals
                } else {
                    result.away_goals
                }
            })
            .sum()
    }

    /// Attacking mentality creates markedly more than defensive.
    #[test]
    fn mentality_moves_chance_creation() {
        let attacking = total_goals(
            &|ctx| ctx.home_mentality = Mentality::Attacking,
            true,
            200,
        );
        let defensive = total_goals(
            &|ctx| ctx.home_mentality = Mentality::Defensive,
            true,
            200,
        );
        assert!(attacking > defensive + 20, "{attacking} vs {defensive}");
    }

    /// The report carries the weather it was played in.
    #[test]
    fn result_carries_match_weather() {
        let engine = SimulationEngine;
        let mut ctx = context(7);
        ctx.weather = cm0102_core::MatchWeather {
            wind: 4,
            precip: 9,
            warmth: 0,
            pitch: cm0102_core::Pitch::Frozen,
        };
        let result = engine.simulate(&ctx);
        assert_eq!(result.weather, ctx.weather);
    }

    /// Storms scrap up play: fewer goals than calm dry days.
    #[test]
    fn storms_damp_chance_creation() {
        let storm = |ctx: &mut MatchContext| {
            ctx.weather = cm0102_core::MatchWeather {
                wind: 4,
                precip: 5,
                warmth: 3,
                pitch: cm0102_core::Pitch::Waterlogged,
            };
        };
        let calm = total_goals(&|_| {}, true, 300);
        let wet = total_goals(&storm, true, 300);
        assert!(calm > wet, "{calm} vs {wet}");
    }

    /// Total cards (yellows and reds) over `seeds` matches.
    fn total_cards(tweak: &dyn Fn(&mut MatchContext), seeds: u64) -> usize {
        let engine = SimulationEngine;
        (0..seeds)
            .map(|seed| {
                let mut ctx = context(seed);
                tweak(&mut ctx);
                engine
                    .simulate(&ctx)
                    .events
                    .iter()
                    .filter(|event| {
                        matches!(event.kind, EventKind::Yellow | EventKind::Red)
                    })
                    .count()
            })
            .sum()
    }

    /// Strict referees book more than lenient ones.
    #[test]
    fn strictness_moves_bookings() {
        let strict = total_cards(&|ctx| ctx.referee.strictness = 18, 200);
        let lenient = total_cards(&|ctx| ctx.referee.strictness = 2, 200);
        assert!(strict > lenient + 20, "{strict} vs {lenient}");
    }

    /// Sitting deep damps the opponent's scoring.
    #[test]
    fn men_behind_ball_damp_opponent() {
        let open = total_goals(&|_| {}, false, 200);
        let bus = total_goals(&|ctx| ctx.home_men_behind_ball = true, false, 200);
        assert!(open > bus, "{open} vs {bus}");
    }

    /// Exhausted elevens score markedly less than fit ones.
    #[test]
    fn condition_scales_performance() {
        let fit = total_goals(&|_| {}, true, 200);
        let tired = total_goals(
            &|ctx| {
                for player in &mut ctx.home_players {
                    player.condition = 20;
                }
            },
            true,
            200,
        );
        assert!(fit > tired + 20, "{fit} vs {tired}");
    }

    /// High morale outscores low morale over a season's worth of games.
    #[test]
    fn morale_lifts_scoring() {
        let up = total_goals(
            &|ctx| {
                for player in &mut ctx.home_players {
                    player.morale = 3;
                }
            },
            true,
            400,
        );
        let down = total_goals(
            &|ctx| {
                for player in &mut ctx.home_players {
                    player.morale = -3;
                }
            },
            true,
            400,
        );
        assert!(up > down, "{up} vs {down}");
    }

    /// Ratings stay in range, reward clean scorers and punish
    /// dismissals.
    #[test]
    fn ratings_reward_scorers() {
        let engine = SimulationEngine;
        let mut scorer_seen = false;
        for seed in 0..100 {
            for rating in &engine.simulate(&context(seed)).ratings {
                assert!((50..=100).contains(&rating.rating), "{rating:?}");
                if rating.goals > 0 && !rating.yellow && !rating.red {
                    assert!(rating.rating > BASE_RATING as u8, "{rating:?}");
                    scorer_seen = true;
                }
                if rating.red && rating.goals == 0 {
                    assert!(rating.rating < BASE_RATING as u8, "{rating:?}");
                }
            }
        }
        assert!(scorer_seen);
    }
}
