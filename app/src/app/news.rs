// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! Daily world simulation and the news inbox: every simulated day
//! plays the due AI fixtures (plus the managed club's on holiday),
//! pushes a results roundup, and queues board, injury and ban items
//! around the managed club's matches.

use std::collections::{HashMap, HashSet};

use cm0102_core::{
    EventKind, Fixture, FixtureResult, GameDate, MatchContext, MatchEngine, MatchResult,
    Mentality, NewsCategory,
};
use cm0102_match::SimulationEngine;

use super::*;
use crate::i18n::{t, tr};

/// Cap on scorelines listed in one results roundup.
const ROUNDUP_LINES: usize = 15;

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
    }

    /// Simulate one fixture through the match engine, serve bans,
    /// record the score and report, and apply season discipline.
    /// Used for the managed club's matches (match day, holiday).
    pub(super) fn simulate_and_record(&mut self, fixture: &Fixture) -> MatchResult {
        let (home_players, home_subs) = self.match_squad(fixture.home_club_id);
        let (away_players, away_subs) = self.match_squad(fixture.away_club_id);
        let (home_mentality, home_bus) = self.side_instructions(fixture.home_club_id);
        let (away_mentality, away_bus) = self.side_instructions(fixture.away_club_id);
        let context = MatchContext {
            home_club_id: fixture.home_club_id,
            away_club_id: fixture.away_club_id,
            date: fixture.date,
            seed: fixture.seed(),
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
        };
        let outcome = SimulationEngine.simulate(&context);
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
        let context = MatchContext {
            home_club_id: fixture.home_club_id,
            away_club_id: fixture.away_club_id,
            date: fixture.date,
            seed: fixture.seed(),
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
        let outcome = SimulationEngine.simulate(&context);
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
}
