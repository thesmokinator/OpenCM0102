// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! Daily world simulation and the news inbox: every simulated day
//! plays the due AI fixtures (plus the managed club's on holiday),
//! pushes a results roundup, and queues board, injury and ban items
//! around the managed club's matches.

use std::collections::{HashMap, HashSet};

use cm0102_core::{
    BoardOutcome, DailyEngineInput, EventKind, Fixture, FixtureResult, GameDate, MatchContext,
    MatchEngine, MatchResult, MatchWeather, Mentality, NewsCategory, Referee, SackCause,
    apply_daily_engines, due_events,
};
use cm0102_match::SimulationEngine;

use super::*;
use crate::i18n::{t, tr};

/// Cap on scorelines listed in one results roundup.
const ROUNDUP_LINES: usize = 15;
/// Days a postponed fixture moves out.
const POSTPONE_DAYS: u32 = 7;
/// Reputation assumed when the competition is unknown.
const UNKNOWN_REPUTATION: i16 = 8;

/// Pick one official deterministically, weighted by reputation so
/// established referees take most matches and filler rows appear
/// rarely. Returns `None` for an empty pool.
fn pick_official(officials: &[cm0102_core::Official], seed: u64) -> Option<&cm0102_core::Official> {
    let total: u64 = officials.iter().map(official_weight).sum();
    if total == 0 {
        return None;
    }
    let mut draw = seed % total;
    for official in officials {
        let weight = official_weight(official);
        if draw < weight {
            return Some(official);
        }
        draw -= weight;
    }
    officials.last()
}

/// Assignment weight: reputation plus a base share so unrated
/// officials still appear.
fn official_weight(official: &cm0102_core::Official) -> u64 {
    1 + official.reputation.unwrap_or(0).max(0) as u64
}

impl OpenCmApp {
    /// Simulate one world day: every due unplayed fixture not
    /// involving the managed club, plus — on holiday, when the
    /// assistant takes charge — the managed club's due fixtures.
    /// Pushes one results roundup when AI fixtures were played.
    pub(super) fn simulate_day(&mut self, auto_own: bool) {
        let today = self.state.current_date;
        let managed = self.state.managed_club_id;
        let involves_managed = |fixture: &Fixture| {
            managed.is_some_and(|id| {
                fixture.home_club_id == id || fixture.away_club_id == id
            })
        };
        let due: Vec<Fixture> = self
            .state
            .schedule
            .fixtures
            .iter()
            .filter(|fixture| {
                fixture.result.is_none()
                    && fixture.date <= today
                    && (auto_own || !involves_managed(fixture))
            })
            .cloned()
            .collect();
        let mut strengths = HashMap::new();
        let mut squads = HashMap::new();
        let mut roundup: Vec<(Fixture, MatchResult)> = Vec::new();
        for fixture in due {
            if self.postpone_if_unplayable(&fixture) {
                continue;
            }
            if involves_managed(&fixture) {
                let pre_susp = self.state.suspensions.clone();
                let pre_inj = self.state.injuries.clone();
                let outcome = self.simulate_and_record(&fixture);
                self.finalize_own_match(&fixture, &outcome, &pre_susp, &pre_inj);
            } else {
                let outcome = self.simulate_ai_fixture(&fixture, &mut strengths, &mut squads);
                roundup.push((fixture, outcome));
            }
        }
        self.push_results_roundup(&roundup);
        self.run_daily_engines();
    }

    /// Run the day's engine stages (permits, board confidence, regen
    /// dues, scripted setup transfers) and announce their outcomes.
    /// Permit and board inputs have no source flows yet, so those
    /// stages rest until applications and result formulas land; the
    /// scripted setup queue feeds from the data dir's cfg today.
    fn run_daily_engines(&mut self) {
        let today = self.state.current_date;
        let setup_due: Vec<cm0102_core::SetupEvent> =
            due_events(&self.setup_script, today).into_iter().cloned().collect();
        let input = DailyEngineInput {
            setup_due,
            ..DailyEngineInput::default()
        };
        let report = apply_daily_engines(&mut self.state, today, input);
        self.announce_setup_moves(today);
        for player_id in report.permit_grants {
            let name = self.staff_display_name(player_id);
            self.state.push_news(
                NewsCategory::Transfer,
                tr("news-permit-grant-headline", &[("name", &name)]),
                tr("news-permit-grant-body", &[("name", &name)]),
            );
        }
        for player_id in report.permit_refusals {
            let name = self.staff_display_name(player_id);
            self.state.push_news(
                NewsCategory::Transfer,
                tr("news-permit-refuse-headline", &[("name", &name)]),
                tr("news-permit-refuse-body", &[("name", &name)]),
            );
        }
        if let Some(outcome) = report.board_outcome {
            let (headline_key, body_key) = match outcome {
                BoardOutcome::VoteOfConfidence => {
                    ("news-board-vote-headline", "news-board-vote-body")
                }
                BoardOutcome::PatienceWarning => {
                    ("news-board-warning-headline", "news-board-warning-body")
                }
                BoardOutcome::Sacking(cause) => match cause {
                    SackCause::Results => {
                        ("news-board-sacked-headline", "news-board-sacked-results-body")
                    }
                    SackCause::LeaguePosition => {
                        ("news-board-sacked-headline", "news-board-sacked-position-body")
                    }
                    SackCause::SupporterPressure => {
                        ("news-board-sacked-headline", "news-board-sacked-supporters-body")
                    }
                },
            };
            self.state
                .push_news(NewsCategory::Board, t(headline_key), t(body_key));
        }
    }

    /// Announce scripted transfers/loans touching the managed club:
    /// arrivals and departures stop Continue through the inbox.
    fn announce_setup_moves(&mut self, today: GameDate) {
        let managed = self.managed_club_name();
        let due: Vec<cm0102_core::SetupEvent> = self
            .state
            .pending_transfers
            .iter()
            .filter(|event| {
                event.applies_on() == today
                    && (event.to_club() == managed || event.from_club() == managed)
            })
            .cloned()
            .collect();
        for event in due {
            let player = event.player_name();
            let arriving = event.to_club() == managed;
            let other = if arriving {
                event.from_club()
            } else {
                event.to_club()
            };
            let loan = event.is_loan();
            let (headline_key, body_key) = match (arriving, loan) {
                (true, false) => ("news-transfer-sign-headline", "news-transfer-in-body"),
                (true, true) => ("news-loan-in-headline", "news-transfer-in-body"),
                (false, false) => ("news-transfer-out-headline", "news-transfer-out-body"),
                (false, true) => ("news-loan-out-headline", "news-transfer-out-body"),
            };
            self.state.push_news(
                NewsCategory::Transfer,
                tr(headline_key, &[("player", &player), ("club", other)]),
                tr(body_key, &[("player", &player), ("club", other)]),
            );
        }
    }

    /// Display name for a staff id, falling back to the id itself.
    fn staff_display_name(&self, player_id: u32) -> String {
        self.provider
            .as_ref()
            .and_then(|provider| provider.get_player(player_id).ok())
            .map(|player| player.full_name())
            .unwrap_or_else(|| player_id.to_string())
    }

    /// Match weather for a fixture: the home ground's climate zone
    /// on the fixture date, rolled from the fixture seed.
    pub(super) fn fixture_weather(&self, fixture: &Fixture) -> MatchWeather {
        let zone = self
            .clubs
            .iter()
            .find(|club| club.id == fixture.home_club_id)
            .and_then(|club| club.league_id)
            .map_or(1, cm0102_core::league_zone);
        cm0102_core::roll_weather(
            &self.weather_table,
            zone,
            fixture.date.day_of_year(),
            fixture.seed(),
        )
    }

    /// Postpone a fixture whose pitch is unplayable, moving it a
    /// week out and telling the manager when their club is
    /// involved. Returns true when the fixture was postponed.
    pub(super) fn postpone_if_unplayable(&mut self, fixture: &Fixture) -> bool {
        let weather = self.fixture_weather(fixture);
        if !cm0102_core::is_postponed(&weather, fixture.seed()) {
            return false;
        }
        let new_date = self.state.current_date.add_days(POSTPONE_DAYS);
        self.state.schedule.postpone(
            fixture.date,
            fixture.home_club_id,
            fixture.away_club_id,
            new_date,
        );
        let managed = self.state.managed_club_id;
        let involved = managed.is_some_and(|id| {
            fixture.home_club_id == id || fixture.away_club_id == id
        });
        if involved {
            let home = self.club_display_name(fixture.home_club_id);
            let away = self.club_display_name(fixture.away_club_id);
            let date = new_date.to_string();
            let reason = t(super::matchday::pitch_label_key(&weather.pitch));
            self.state.push_news(
                NewsCategory::Postponed,
                tr("news-postponed-headline", &[("home", &home), ("away", &away)]),
                tr(
                    "news-postponed-body",
                    &[("date", &date), ("reason", &reason)],
                ),
            );
        }
        true
    }

    /// Referee for a fixture: an official of the home nation,
    /// picked deterministically from the fixture seed. Falls back
    /// to a neutral unnamed referee without data.
    pub(super) fn fixture_referee(&self, fixture: &Fixture) -> Referee {
        let nation = self
            .clubs
            .iter()
            .find(|club| club.id == fixture.home_club_id)
            .and_then(|club| club.league_id)
            .map(cm0102_core::league_nation)
            .unwrap_or("England");
        let official = self.provider.as_ref().and_then(|provider| {
            let nations = provider.list_nations().unwrap_or_default();
            let nation_id = nations
                .iter()
                .find(|candidate| candidate.name == nation)
                .map(|candidate| candidate.id)?;
            let officials = provider.list_officials_by_nation(nation_id).unwrap_or_default();
            pick_official(&officials, fixture.seed()).cloned()
        });
        official.map_or_else(Referee::default, |official| {
            // Zero reads as unset, not lenient: most filler rows
            // carry it, and it would zero the booking rate.
            let strictness = official.strictness.filter(|value| *value > 0).unwrap_or_else(|| {
                cm0102_core::strictness_for_official(official.id)
            });
            Referee {
                name: official.name.clone(),
                strictness,
            }
        })
    }

    /// Attendance for a fixture from the competition's reputation
    /// and the weather over the ground.
    pub(super) fn fixture_attendance(&self, fixture: &Fixture, weather: &MatchWeather) -> u32 {
        let reputation = self
            .provider
            .as_ref()
            .and_then(|provider| {
                provider
                    .competition_reputation(fixture.competition_id)
                    .unwrap_or(None)
            })
            .unwrap_or(UNKNOWN_REPUTATION);
        cm0102_core::crowd(reputation, weather, fixture.seed())
    }

    /// Engine context for a fixture: squads, strengths, weather,
    /// referee and the managed club's instructions.
    pub(super) fn match_context(&self, fixture: &Fixture) -> MatchContext {
        let (home_players, home_subs) = self.match_squad(fixture.home_club_id);
        let (away_players, away_subs) = self.match_squad(fixture.away_club_id);
        let (home_mentality, home_bus) = self.side_instructions(fixture.home_club_id);
        let (away_mentality, away_bus) = self.side_instructions(fixture.away_club_id);
        MatchContext {
            home_club_id: fixture.home_club_id,
            away_club_id: fixture.away_club_id,
            date: fixture.date,
            seed: fixture.seed(),
            weather: self.fixture_weather(fixture),
            referee: self.fixture_referee(fixture),
            home_avg_ca: self.team_avg_ca(fixture.home_club_id),
            away_avg_ca: self.team_avg_ca(fixture.away_club_id),
            home_players,
            away_players,
            home_subs,
            away_subs,
            home_mentality,
            away_mentality,
            home_men_behind_ball: home_bus,
            away_men_behind_ball: away_bus,
            // League fixtures only until cups land.
            extra_time_if_level: false,
        }
    }

    /// Simulate one fixture through the match engine, serve bans,
    /// record the score and report, and apply season discipline.
    /// Used for instant matches (AI fixtures, holiday).
    pub(super) fn simulate_and_record(&mut self, fixture: &Fixture) -> MatchResult {
        let context = self.match_context(fixture);
        let mut outcome = SimulationEngine.simulate(&context);
        outcome.attendance = Some(self.fixture_attendance(fixture, &outcome.weather));
        self.serve_match_bans(fixture.home_club_id, fixture.away_club_id);
        self.state.schedule.record_result(
            fixture.date,
            fixture.home_club_id,
            fixture.away_club_id,
            FixtureResult {
                home_goals: outcome.home_goals,
                away_goals: outcome.away_goals,
            },
            Some(outcome.clone()),
        );
        self.state.apply_match_report(&outcome);
        outcome
    }

    /// Simulate one AI fixture with cached squads and strengths,
    /// recording score, report and discipline like any other match.
    fn simulate_ai_fixture(
        &mut self,
        fixture: &Fixture,
        strengths: &mut HashMap<u32, f32>,
        squads: &mut HashMap<u32, (Vec<cm0102_core::MatchPlayer>, Vec<cm0102_core::MatchPlayer>)>,
    ) -> MatchResult {
        let (home_players, home_subs) =
            self.match_squad_cached(fixture.home_club_id, squads);
        let (away_players, away_subs) =
            self.match_squad_cached(fixture.away_club_id, squads);
        let weather = self.fixture_weather(fixture);
        let referee = self.fixture_referee(fixture);
        let context = MatchContext {
            home_club_id: fixture.home_club_id,
            away_club_id: fixture.away_club_id,
            date: fixture.date,
            seed: fixture.seed(),
            weather,
            referee,
            home_avg_ca: self.team_avg_ca_cached(fixture.home_club_id, strengths),
            away_avg_ca: self.team_avg_ca_cached(fixture.away_club_id, strengths),
            home_players,
            away_players,
            home_subs,
            away_subs,
            home_mentality: Mentality::Normal,
            away_mentality: Mentality::Normal,
            home_men_behind_ball: false,
            away_men_behind_ball: false,
            // League fixtures only until cups land.
            extra_time_if_level: false,
        };
        let mut outcome = SimulationEngine.simulate(&context);
        outcome.attendance = Some(self.fixture_attendance(fixture, &outcome.weather));
        self.serve_match_bans(fixture.home_club_id, fixture.away_club_id);
        self.state.schedule.record_result(
            fixture.date,
            fixture.home_club_id,
            fixture.away_club_id,
            FixtureResult {
                home_goals: outcome.home_goals,
                away_goals: outcome.away_goals,
            },
            Some(outcome.clone()),
        );
        self.state.apply_match_report(&outcome);
        outcome
    }

    /// One roundup item listing the day's other results, newest
    /// first in the inbox.
    fn push_results_roundup(&mut self, played: &[(Fixture, MatchResult)]) {
        if played.is_empty() {
            return;
        }
        let mut lines: Vec<String> = played
            .iter()
            .map(|(fixture, outcome)| {
                let home = self.club_display_name(fixture.home_club_id);
                let away = self.club_display_name(fixture.away_club_id);
                format!("{home} {}-{} {away}", outcome.home_goals, outcome.away_goals)
            })
            .collect();
        lines.sort();
        let total = lines.len();
        lines.truncate(ROUNDUP_LINES);
        let mut body = lines.join("\n");
        if total > ROUNDUP_LINES {
            let more = (total - ROUNDUP_LINES).to_string();
            body.push_str(&tr("news-roundup-more", &[("n", &more)]));
        }
        self.state.push_news(
            NewsCategory::ResultsRoundup,
            t("news-roundup-headline"),
            body,
        );
    }

    /// Post-match inbox items for the managed club: the report,
    /// fresh injuries and bans (diffed against the pre-match
    /// snapshot), and the board's reaction to the result.
    pub(super) fn finalize_own_match(
        &mut self,
        fixture: &Fixture,
        outcome: &MatchResult,
        pre_suspensions: &HashMap<u32, u32>,
        pre_injuries: &HashMap<u32, GameDate>,
    ) {
        let Some(managed) = self.state.managed_club_id else {
            return;
        };
        let managed_home = fixture.home_club_id == managed;
        let played = Fixture {
            result: Some(FixtureResult {
                home_goals: outcome.home_goals,
                away_goals: outcome.away_goals,
            }),
            ..fixture.clone()
        };
        let mut body: Vec<String> = outcome
            .events
            .iter()
            .filter(|event| {
                matches!(
                    event.kind,
                    EventKind::Goal | EventKind::Penalty | EventKind::OwnGoal
                )
            })
            .map(|event| event.display())
            .collect();
        if body.is_empty() {
            body.push(t("news-report-no-goals"));
        }
        if let Some(shootout) = outcome.shootout {
            body.push(tr(
                "matchday-shootout",
                &[
                    ("hs", &shootout.home_scored.to_string()),
                    ("as", &shootout.away_scored.to_string()),
                ],
            ));
        }
        self.state.push_news(
            NewsCategory::MatchReport,
            self.fixture_line(&played),
            body.join("\n"),
        );
        let squad_ids: HashSet<u32> = self
            .provider
            .as_ref()
            .and_then(|provider| provider.get_players_by_club(managed).ok())
            .unwrap_or_default()
            .iter()
            .map(|player| player.id)
            .collect();
        for injury in &outcome.injuries {
            if !squad_ids.contains(&injury.player_id) {
                continue;
            }
            if self.state.injuries.get(&injury.player_id) == pre_injuries.get(&injury.player_id) {
                continue;
            }
            let days = injury.days_out.to_string();
            self.state.push_news(
                NewsCategory::Injury,
                tr(
                    "news-injury-headline",
                    &[("name", &injury.name), ("days", &days)],
                ),
                tr(
                    "news-injury-body",
                    &[("name", &injury.name), ("days", &days)],
                ),
            );
        }
        for (player_id, left) in self.state.suspensions.clone() {
            if !squad_ids.contains(&player_id) {
                continue;
            }
            let was = pre_suspensions.get(&player_id).copied().unwrap_or(0);
            // Bans tick down one per match served just before the
            // report applies, so only counts above that are fresh.
            if left <= was.saturating_sub(1) {
                continue;
            }
            let name = self
                .provider
                .as_ref()
                .and_then(|provider| provider.get_player(player_id).ok())
                .map(|player| player.full_name())
                .unwrap_or_else(|| player_id.to_string());
            let matches = left.to_string();
            self.state.push_news(
                NewsCategory::Suspension,
                tr(
                    "news-ban-headline",
                    &[("name", &name), ("matches", &matches)],
                ),
                tr("news-ban-body", &[("name", &name), ("matches", &matches)]),
            );
        }
        let (headline_key, body_key) = board_reaction(
            managed_home,
            outcome.home_goals,
            outcome.away_goals,
        );
        self.state
            .push_news(NewsCategory::Board, t(headline_key), t(body_key));
    }

    /// Queue every fresh item for modal presentation.
    pub(super) fn open_news_queue(&mut self) {
        self.shell.news_queue = self
            .state
            .fresh_news()
            .iter()
            .map(|item| item.id)
            .collect();
        self.shell.news_queue_total = self.shell.news_queue.len();
    }

    /// Modal fresh-news queue: Next steps through the items marking
    /// each read; Close marks the rest read and dismisses.
    pub(super) fn show_news_queue(&mut self, ctx: &egui::Context) {
        let Some(id) = self.shell.news_queue.first().copied() else {
            return;
        };
        let total = self.shell.news_queue_total.max(1).to_string();
        let index = self
            .shell
            .news_queue_total
            .saturating_sub(self.shell.news_queue.len())
            + 1;
        let index_text = index.to_string();
        let item = self.state.news.iter().find(|item| item.id == id).cloned();
        let Some(item) = item else {
            self.shell.news_queue.remove(0);
            return;
        };
        let mut advance = false;
        let mut close = false;
        egui::Window::new(tr("news-queue-title", &[("i", &index_text), ("n", &total)]))
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.heading(&item.headline);
                ui.label(format!(
                    "{} · {}",
                    item.date,
                    news_category_label(&item.category)
                ));
                ui.separator();
                ui.label(&item.body);
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button(t("news-next")).clicked() {
                        advance = true;
                    }
                    if ui.button(t("news-close")).clicked() {
                        close = true;
                    }
                });
            });
        if advance {
            self.state.mark_news_read(id);
            self.shell.news_queue.remove(0);
        } else if close {
            let queued = self.shell.news_queue.clone();
            for queued_id in queued {
                self.state.mark_news_read(queued_id);
            }
            self.shell.news_queue.clear();
        }
    }

    /// News section: every inbox item newest first; clicking one
    /// shows its body and marks it read.
    pub(super) fn show_news_section(&mut self, ui: &mut egui::Ui) {
        ui.heading(t("section-news"));
        if self.state.news.is_empty() {
            ui.label(t("news-empty"));
            return;
        }
        let mut items: Vec<(u64, String, String, bool)> = self
            .state
            .news
            .iter()
            .map(|item| {
                (
                    item.id,
                    item.date.to_string(),
                    item.headline.clone(),
                    item.read,
                )
            })
            .collect();
        items.sort_by_key(|(id, _, _, _)| std::cmp::Reverse(*id));
        let mut selected = self.shell.news_selected;
        egui::Grid::new("shell_news_grid")
            .striped(true)
            .show(ui, |ui| {
                for (id, date, headline, read) in &items {
                    ui.label(date);
                    let label = if *read {
                        headline.clone()
                    } else {
                        format!("● {headline}")
                    };
                    if ui.selectable_label(selected == Some(*id), label).clicked() {
                        selected = Some(*id);
                    }
                    ui.end_row();
                }
            });
        if let Some(id) = selected {
            self.shell.news_selected = Some(id);
            self.state.mark_news_read(id);
            if let Some(item) = self.state.news.iter().find(|item| item.id == id) {
                ui.separator();
                ui.heading(&item.headline);
                ui.label(format!(
                    "{} · {}",
                    item.date,
                    news_category_label(&item.category)
                ));
                ui.label(&item.body);
            }
        }
    }

    /// Unread inbox items, for the sidebar badge.
    pub(super) fn unread_news_count(&self) -> usize {
        self.state.news.iter().filter(|item| !item.read).count()
    }
}

/// Board's one-line verdict on the managed club's result.
fn board_reaction(managed_home: bool, home_goals: u32, away_goals: u32) -> (&'static str, &'static str) {
    let (scored, conceded) = if managed_home {
        (home_goals, away_goals)
    } else {
        (away_goals, home_goals)
    };
    if scored > conceded {
        ("news-board-win-headline", "news-board-win-body")
    } else if scored == conceded {
        ("news-board-draw-headline", "news-board-draw-body")
    } else {
        ("news-board-loss-headline", "news-board-loss-body")
    }
}

/// Localized inbox category name.
fn news_category_label(category: &NewsCategory) -> String {
    t(match category {
        NewsCategory::MatchReport => "news-cat-match-report",
        NewsCategory::ResultsRoundup => "news-cat-results-roundup",
        NewsCategory::Injury => "news-cat-injury",
        NewsCategory::Suspension => "news-cat-suspension",
        NewsCategory::Board => "news-cat-board",
        NewsCategory::Transfer => "news-cat-transfer",
        NewsCategory::Postponed => "news-cat-postponed",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn board_reaction_follows_managed_result() {
        assert_eq!(
            board_reaction(true, 2, 0),
            ("news-board-win-headline", "news-board-win-body")
        );
        assert_eq!(
            board_reaction(false, 1, 1),
            ("news-board-draw-headline", "news-board-draw-body")
        );
        assert_eq!(
            board_reaction(false, 3, 1),
            ("news-board-loss-headline", "news-board-loss-body")
        );
    }

    #[test]
    fn official_pick_is_weighted_and_deterministic() {
        use cm0102_core::Official;
        let pool = vec![
            Official {
                id: 1,
                name: "Top Ref".to_string(),
                nation_id: Some(61),
                strictness: Some(12),
                reputation: Some(99),
            },
            Official {
                id: 2,
                name: "Filler".to_string(),
                nation_id: Some(61),
                strictness: Some(0),
                reputation: Some(0),
            },
        ];
        assert_eq!(pick_official(&[], 7), None);
        assert_eq!(
            pick_official(&pool, 7).map(|official| official.id),
            pick_official(&pool, 7).map(|official| official.id),
        );
        // Weight 100 vs 1: the top referee takes ~99% of seeds.
        let top = (0..500u64)
            .filter(|seed| pick_official(&pool, *seed).is_some_and(|o| o.id == 1))
            .count();
        assert!(top > 480, "top takes {top}/500");
    }
}
