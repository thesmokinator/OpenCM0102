// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! Main-menu shell: sidebar navigation with a top info bar, mirroring
//! the original game's main screen layout (sections on the left,
//! content on the right, manager/club/date plus Continue on top).
//! Sections render real data where an engine exists (squad) and an
//! honest placeholder citing the backing spec everywhere else.

use cm0102_core::{
    Fixture, FixtureResult, Formation, Lineup, MatchContext, MatchEngine, Player, SavedCareer,
    Schedule, TableRules, TeamTactics, process_day,
};
use cm0102_match::SimpleMatchEngine;

/// Fallback average ability when no lineup resolves (placeholder).
const DEFAULT_AVG_CA: f32 = 140.0;

use super::career::{save_career_to, saves_dir};
use super::*;
use crate::i18n::{t, tr};

/// One section of the main-menu sidebar, in sidebar order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GameSection {
    /// Managed club's squad list.
    #[default]
    Squad,
    /// Formations and instructions.
    Tactics,
    /// Training schedules.
    Training,
    /// Upcoming and past fixtures.
    Fixtures,
    /// League table.
    LeagueTable,
    /// Transfer market.
    Transfers,
    /// Budgets and club finances.
    FinancesInfo,
    /// Board confidence and expectations.
    Board,
    /// Non-playing staff.
    Staff,
    /// Player and staff search.
    Search,
    /// News inbox.
    News,
    /// Club history.
    History,
    /// Club records.
    Records,
    /// Awards roll of honour.
    Awards,
    /// Club information.
    ClubInfo,
}

impl GameSection {
    /// Every section, in the order the sidebar shows them.
    pub const ALL: &'static [GameSection] = &[
        GameSection::Squad,
        GameSection::Tactics,
        GameSection::Training,
        GameSection::Fixtures,
        GameSection::LeagueTable,
        GameSection::Transfers,
        GameSection::FinancesInfo,
        GameSection::Board,
        GameSection::Staff,
        GameSection::Search,
        GameSection::News,
        GameSection::History,
        GameSection::Records,
        GameSection::Awards,
        GameSection::ClubInfo,
    ];

    /// Sidebar label, using the original game's vocabulary in the
    /// current language.
    pub fn label(self) -> String {
        t(match self {
            GameSection::Squad => "section-squad",
            GameSection::Tactics => "section-tactics",
            GameSection::Training => "section-training",
            GameSection::Fixtures => "section-fixtures",
            GameSection::LeagueTable => "section-league-table",
            GameSection::Transfers => "section-transfers",
            GameSection::FinancesInfo => "section-finances-info",
            GameSection::Board => "section-board",
            GameSection::Staff => "section-staff",
            GameSection::Search => "section-search",
            GameSection::News => "section-news",
            GameSection::History => "section-history",
            GameSection::Records => "section-records",
            GameSection::Awards => "section-awards",
            GameSection::ClubInfo => "section-club-info",
        })
    }

    /// Versioned spec backing the section, cited by placeholders.
    pub fn spec(self) -> &'static str {
        match self {
            GameSection::Squad => "docs/players.md",
            GameSection::Tactics => "docs/tactics.md",
            GameSection::Training => "docs/season.md",
            GameSection::Fixtures => "docs/competitions.md",
            GameSection::LeagueTable => "docs/competitions.md",
            GameSection::Transfers => "docs/transfers.md",
            GameSection::FinancesInfo => "docs/season.md",
            GameSection::Board => "docs/season.md",
            GameSection::Staff => "docs/season.md",
            GameSection::Search => "docs/players.md",
            GameSection::News => "docs/season.md",
            GameSection::History => "docs/season.md",
            GameSection::Records => "docs/season.md",
            GameSection::Awards => "docs/season.md",
            GameSection::ClubInfo => "docs/competitions.md",
        }
    }
}

/// Sidebar selection state of the main-menu shell.
#[derive(Default)]
pub struct ShellState {
    /// Currently shown section.
    pub section: GameSection,
    /// Club whose squad the tactics cache holds.
    pub tactics_club_id: Option<u32>,
    /// Cached managed-club squad for the tactics section.
    pub tactics_squad: Vec<Player>,
    /// Armed starter slot awaiting a swap, if any.
    pub tactics_selected: Option<usize>,
}

impl OpenCmApp {
    /// Main-menu screen: top info bar, next match, sidebar, content.
    pub fn show_game(&mut self, ui: &mut egui::Ui) {
        self.show_game_top_bar(ui);
        ui.separator();
        self.show_next_match(ui);
        ui.separator();
        ui.horizontal(|ui| {
            self.show_sidebar(ui);
            ui.separator();
            ui.vertical(|ui| {
                self.show_section(ui);
            });
        });
    }

    /// Manager, club, date plus Holiday/Save, as on top of the
    /// original main screen. Continue lives in the bottom action bar.
    fn show_game_top_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let manager = self
                .state
                .manager
                .as_ref()
                .map(|manager| manager.full_name())
                .unwrap_or_else(|| crate::i18n::t("missing-value"));
            ui.strong(&manager);
            ui.separator();
            ui.label(self.managed_club_name());
            ui.separator();
            ui.label(self.state.current_date.to_string());
            ui.separator();
            if ui.button(t("shell-holiday-7-days")).clicked() {
                let schedule = self.state.schedule.clone();
                for _ in 0..7 {
                    process_day(&mut self.state, &schedule);
                }
                self.simulate_other_matches();
                self.rollover_calendar_if_needed();
                self.status = tr(
                    "shell-holidayed-to",
                    &[("date", &self.state.current_date.to_string())],
                );
            }
            if ui.button(t("shell-save-game")).clicked() {
                self.save_active_career();
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.menu_button(t("shell-menu"), |ui| {
                    if ui.button(t("career-back-to-setup")).clicked() {
                        self.screen = Screen::SetupWizard;
                        ui.close_menu();
                    }
                    if ui.button(t("action-exit-game")).clicked() {
                        self.exit_armed = true;
                        ui.close_menu();
                    }
                });
            });
        });
    }

    /// Next unplayed fixture of the managed club, if any.
    fn show_next_match(&mut self, ui: &mut egui::Ui) {
        let today = self.state.current_date;
        let managed = self.state.managed_club_id;
        let next = self
            .state
            .schedule
            .fixtures
            .iter()
            .filter(|fixture| {
                fixture.result.is_none()
                    && fixture.date >= today
                    && managed.is_some_and(|id| {
                        fixture.home_club_id == id || fixture.away_club_id == id
                    })
            })
            .min_by_key(|fixture| fixture.date);
        match next {
            Some(fixture) => {
                let date = fixture.date.to_string();
                let game = self.fixture_line(fixture);
                ui.label(tr(
                    "fixtures-next-match",
                    &[("match", &game), ("date", &date)],
                ));
            }
            None => {
                ui.label(t("fixtures-no-upcoming"));
            }
        }
    }

    /// Section list on the left, original-game order.
    fn show_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.set_min_width(140.0);
            for section in GameSection::ALL {
                let selected = self.shell.section == *section;
                if ui.selectable_label(selected, section.label()).clicked() {
                    self.shell.section = *section;
                }
            }
        });
    }

    /// Section content: real data where implemented, an honest
    /// placeholder citing the backing spec everywhere else.
    fn show_section(&mut self, ui: &mut egui::Ui) {
        match self.shell.section {
            GameSection::Squad => self.show_squad_section(ui),
            GameSection::Tactics => self.show_tactics_section(ui),
            GameSection::Fixtures => self.show_fixtures_section(ui),
            GameSection::LeagueTable => self.show_table_section(ui),
            section => {
                ui.heading(section.label());
                ui.label(tr("shell-not-implemented", &[("spec", section.spec())]));
            }
        }
    }

    /// Managed club's squad, reusing the imported player data.
    fn show_squad_section(&mut self, ui: &mut egui::Ui) {
        let club = self.managed_club_name();
        ui.heading(tr("shell-squad-title", &[("club", &club)]));
        if self.squad.is_empty() {
            ui.label(t("shell-no-players"));
            return;
        }
        egui::Grid::new("shell_squad_grid")
            .striped(true)
            .show(ui, |ui| {
                ui.strong(t("shell-th-name"));
                ui.strong(t("shell-th-role"));
                ui.strong(t("shell-th-ca"));
                ui.strong(t("shell-th-pa"));
                ui.end_row();
                for player in self.squad.iter().take(RENDER_LIMIT) {
                    ui.label(player.full_name());
                    ui.label(role_label(player.job));
                    ui.label(
                        player
                            .current_ability
                            .map(|value| value.to_string())
                            .unwrap_or_else(|| crate::i18n::t("missing-value")),
                    );
                    ui.label(
                        player
                            .potential_ability
                            .map(|value| value.to_string())
                            .unwrap_or_else(|| crate::i18n::t("missing-value")),
                    );
                    ui.end_row();
                }
            });
    }

    /// Managed club's fixtures: results so far and upcoming rounds,
    /// with a Play button for due matches (today or overdue).
    fn show_fixtures_section(&mut self, ui: &mut egui::Ui) {
        ui.heading(t("section-fixtures"));
        let Some(managed_id) = self.state.managed_club_id else {
            ui.label(t("fixtures-no-calendar"));
            return;
        };
        let mut own: Vec<Fixture> = self
            .state
            .schedule
            .fixtures
            .iter()
            .filter(|fixture| {
                fixture.home_club_id == managed_id || fixture.away_club_id == managed_id
            })
            .copied()
            .collect();
        if own.is_empty() {
            ui.label(t("fixtures-no-calendar"));
            return;
        }
        own.sort_by_key(|fixture| fixture.date);
        let today = self.state.current_date;
        let (played, upcoming): (Vec<Fixture>, Vec<Fixture>) =
            own.into_iter().partition(|fixture| fixture.result.is_some());
        ui.heading(t("fixtures-results-title"));
        egui::Grid::new("shell_results_grid")
            .striped(true)
            .show(ui, |ui| {
                for fixture in played.iter().rev() {
                    ui.label(fixture.date.to_string());
                    ui.label(self.fixture_line(fixture));
                    ui.end_row();
                }
            });
        ui.heading(t("fixtures-upcoming-title"));
        egui::Grid::new("shell_fixtures_grid")
            .striped(true)
            .show(ui, |ui| {
                for fixture in &upcoming {
                    if fixture.date == today {
                        ui.label(format!("{} ({})", fixture.date, t("fixtures-today")));
                    } else {
                        ui.label(fixture.date.to_string());
                    }
                    ui.label(self.fixture_line(fixture));
                    let due = fixture.date <= today;
                    if ui
                        .add_enabled(due, egui::Button::new(t("fixtures-play")))
                        .clicked()
                    {
                        self.play_fixture(*fixture);
                    }
                    ui.end_row();
                }
            });
    }

    /// League table of the managed club's group from played results.
    /// Fills in as matches are played; other clubs' results arrive
    /// with automatic simulation.
    fn show_table_section(&mut self, ui: &mut egui::Ui) {
        ui.heading(t("section-league-table"));
        let members: Vec<u32> = self.league_member_ids();
        if members.is_empty() {
            ui.label(t("fixtures-no-calendar"));
            return;
        }
        let table = Schedule::league_table(
            &members,
            &self.state.schedule.fixtures,
            &TableRules::default(),
        );
        let managed = self.state.managed_club_id;
        egui::Grid::new("shell_table_grid")
            .striped(true)
            .show(ui, |ui| {
                for key in [
                    "table-th-pos",
                    "table-th-club",
                    "table-th-p",
                    "table-th-w",
                    "table-th-d",
                    "table-th-l",
                    "table-th-gf",
                    "table-th-ga",
                    "table-th-gd",
                    "table-th-pts",
                ] {
                    ui.strong(t(key));
                }
                ui.end_row();
                for (pos, row) in table.iter().enumerate() {
                    let highlight = managed == Some(row.club_id);
                    let mut cell = |text: String| {
                        if highlight {
                            ui.colored_label(egui::Color32::YELLOW, text);
                        } else {
                            ui.label(text);
                        }
                    };
                    cell((pos + 1).to_string());
                    cell(self.club_display_name(row.club_id));
                    cell(row.played.to_string());
                    cell(row.won.to_string());
                    cell(row.drawn.to_string());
                    cell(row.lost.to_string());
                    cell(row.goals_for.to_string());
                    cell(row.goals_against.to_string());
                    cell(row.goal_difference().to_string());
                    cell(row.points.to_string());
                    ui.end_row();
                }
            });
    }

    /// Club ids sharing the managed club's league slot, if any.
    fn league_member_ids(&self) -> Vec<u32> {
        let Some(managed_id) = self.state.managed_club_id else {
            return Vec::new();
        };
        let Some(slot) = self
            .clubs
            .iter()
            .find(|club| club.id == managed_id)
            .and_then(|club| club.league_id)
        else {
            return Vec::new();
        };
        self.clubs
            .iter()
            .filter(|club| club.league_id == Some(slot))
            .map(|club| club.id)
            .collect()
    }

    /// One-line display of a fixture: score when played, versus when not.
    fn fixture_line(&self, fixture: &Fixture) -> String {
        let home = self.club_display_name(fixture.home_club_id);
        let away = self.club_display_name(fixture.away_club_id);
        match fixture.result {
            Some(result) => {
                let home_goals = result.home_goals.to_string();
                let away_goals = result.away_goals.to_string();
                tr(
                    "shell-match-result",
                    &[
                        ("home", &home),
                        ("home_goals", &home_goals),
                        ("away_goals", &away_goals),
                        ("away", &away),
                    ],
                )
            }
            None => tr(
                "fixtures-versus",
                &[("home", &home), ("away", &away)],
            ),
        }
    }

    /// Simulate one due fixture with the match engine, record the score
    /// and report it in the status line.
    fn play_fixture(&mut self, fixture: Fixture) {
        let context = MatchContext {
            home_club_id: fixture.home_club_id,
            away_club_id: fixture.away_club_id,
            date: fixture.date,
            seed: fixture.seed(),
            home_avg_ca: self.team_avg_ca(fixture.home_club_id),
            away_avg_ca: self.team_avg_ca(fixture.away_club_id),
        };
        let outcome = SimpleMatchEngine.simulate(&context);
        self.state.schedule.record_result(
            fixture.date,
            fixture.home_club_id,
            fixture.away_club_id,
            FixtureResult {
                home_goals: outcome.home_goals,
                away_goals: outcome.away_goals,
            },
        );
        let played = Fixture {
            result: Some(FixtureResult {
                home_goals: outcome.home_goals,
                away_goals: outcome.away_goals,
            }),
            ..fixture
        };
        self.status = self.fixture_line(&played);
    }

    /// Formation picker plus the starting eleven, with best-eleven
    /// auto-pick. Instructions stay default until their effects are
    /// reverse-engineered (see `docs/tactics.md`).
    fn show_tactics_section(&mut self, ui: &mut egui::Ui) {
        ui.heading(t("section-tactics"));
        let Some(managed_id) = self.state.managed_club_id else {
            ui.label(t("tactics-no-squad"));
            return;
        };
        if self.shell.tactics_club_id != Some(managed_id) {
            self.refresh_tactics_squad(managed_id);
        }
        if self.shell.tactics_squad.is_empty() {
            ui.label(t("tactics-no-squad"));
            return;
        }
        ui.horizontal(|ui| {
            ui.label(t("tactics-formation"));
            let mut formation = self.state.tactics.lineup.formation;
            egui::ComboBox::from_id_salt("tactics_formation")
                .selected_text(formation.name())
                .show_ui(ui, |ui| {
                    for candidate in Formation::ALL {
                        ui.selectable_value(&mut formation, *candidate, candidate.name());
                    }
                });
            self.state.tactics.lineup.formation = formation;
            if ui.button(t("tactics-auto-pick")).clicked() {
                let squad = self.shell.tactics_squad.clone();
                self.state.tactics = TeamTactics::auto(formation, &squad);
                self.shell.tactics_selected = None;
            }
        });
        let count = self.state.tactics.lineup.starting.len().to_string();
        if self.state.tactics.lineup.validate().is_err() {
            ui.label(tr("tactics-invalid", &[("count", &count)]));
        }
        ui.heading(t("tactics-starters"));
        let rows: Vec<(usize, u32, String)> = self
            .state
            .tactics
            .lineup
            .starting
            .clone()
            .iter()
            .enumerate()
            .map(|(shirt, id)| (shirt, *id, self.tactics_player_name(*id)))
            .collect();
        egui::Grid::new("shell_tactics_grid")
            .striped(true)
            .show(ui, |ui| {
                for (shirt, _id, name) in &rows {
                    ui.label((shirt + 1).to_string());
                    let armed = self.shell.tactics_selected == Some(*shirt);
                    if ui.selectable_label(armed, name).clicked() {
                        self.shell.tactics_selected =
                            if armed { None } else { Some(*shirt) };
                    }
                    ui.end_row();
                }
            });
        ui.heading(t("tactics-reserves"));
        ui.small(t("tactics-pick-hint"));
        let starting = self.state.tactics.lineup.starting.clone();
        let reserves: Vec<Player> = self
            .shell
            .tactics_squad
            .iter()
            .filter(|player| player.is_player && !starting.contains(&player.id))
            .cloned()
            .collect();
        egui::ScrollArea::vertical()
            .max_height(200.0)
            .show(ui, |ui| {
                for player in &reserves {
                    let ca = player
                        .current_ability
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| t("missing-value"));
                    if ui
                        .selectable_label(false, format!("{} (CA {ca})", player.full_name()))
                        .clicked()
                    {
                        if let Some(slot) = self.shell.tactics_selected {
                            if let Some(pick) =
                                self.state.tactics.lineup.starting.get_mut(slot)
                            {
                                *pick = player.id;
                            }
                            self.shell.tactics_selected = None;
                        }
                    }
                }
            });
    }

    /// Reload the cached squad for the tactics section.
    fn refresh_tactics_squad(&mut self, club_id: u32) {
        self.shell.tactics_squad = self
            .provider
            .as_ref()
            .and_then(|provider| provider.get_players_by_club(club_id).ok())
            .unwrap_or_default();
        self.shell.tactics_club_id = Some(club_id);
    }

    /// Display name plus ability of a cached squad player.
    fn tactics_player_name(&self, player_id: u32) -> String {
        self.shell
            .tactics_squad
            .iter()
            .find(|player| player.id == player_id)
            .map(|player| {
                let ca = player
                    .current_ability
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| t("missing-value"));
                format!("{} (CA {ca})", player.full_name())
            })
            .unwrap_or_else(|| t("missing-value"))
    }

    /// Average current ability of a club's eleven: the stored lineup
    /// for the managed club, a best-eleven auto-pick for AI clubs.
    fn team_avg_ca(&self, club_id: u32) -> f32 {
        self.team_avg_ca_cached(club_id, &mut std::collections::HashMap::new())
    }

    /// Cached variant for batch simulation across many fixtures.
    fn team_avg_ca_cached(
        &self,
        club_id: u32,
        cache: &mut std::collections::HashMap<u32, f32>,
    ) -> f32 {
        if let Some(avg) = cache.get(&club_id) {
            return *avg;
        }
        let players = self
            .provider
            .as_ref()
            .and_then(|provider| provider.get_players_by_club(club_id).ok())
            .unwrap_or_default();
        if players.is_empty() {
            return DEFAULT_AVG_CA;
        }
        let ids: Vec<u32> = if Some(club_id) == self.state.managed_club_id
            && self.state.tactics.lineup.validate().is_ok()
        {
            self.state.tactics.lineup.starting.clone()
        } else {
            Lineup::auto_pick(Formation::FourFourTwo, &players).starting
        };
        let mut total = 0i32;
        let mut count = 0i32;
        for id in &ids {
            if let Some(ca) = players
                .iter()
                .find(|player| player.id == *id)
                .and_then(|player| player.current_ability)
            {
                total += i32::from(ca);
                count += 1;
            }
        }
        let avg = if count > 0 {
            total as f32 / count as f32
        } else {
            DEFAULT_AVG_CA
        };
        cache.insert(club_id, avg);
        avg
    }

    /// Simulate every due AI fixture: unplayed, dated today or
    /// earlier, not involving the managed club. The manager's own
    /// matches are never touched — overdue ones stay playable.
    pub(super) fn simulate_other_matches(&mut self) {
        let today = self.state.current_date;
        let managed = self.state.managed_club_id;
        let due: Vec<Fixture> = self
            .state
            .schedule
            .fixtures
            .iter()
            .filter(|fixture| {
                fixture.result.is_none()
                    && fixture.date <= today
                    && !managed.is_some_and(|id| {
                        fixture.home_club_id == id || fixture.away_club_id == id
                    })
            })
            .copied()
            .collect();
        let mut strengths = std::collections::HashMap::new();
        for fixture in due {
            let context = MatchContext {
                home_club_id: fixture.home_club_id,
                away_club_id: fixture.away_club_id,
                date: fixture.date,
                seed: fixture.seed(),
                home_avg_ca: self.team_avg_ca_cached(fixture.home_club_id, &mut strengths),
                away_avg_ca: self.team_avg_ca_cached(fixture.away_club_id, &mut strengths),
            };
            let outcome = SimpleMatchEngine.simulate(&context);
            self.state.schedule.record_result(
                fixture.date,
                fixture.home_club_id,
                fixture.away_club_id,
                FixtureResult {
                    home_goals: outcome.home_goals,
                    away_goals: outcome.away_goals,
                },
            );
        }
    }

    /// Club display name, falling back to the raw id when unknown.
    fn club_display_name(&self, club_id: u32) -> String {
        self.clubs
            .iter()
            .find(|club| club.id == club_id)
            .map(|club| club.name.clone())
            .unwrap_or_else(|| {
                let id = club_id.to_string();
                tr("career-unknown-club", &[("id", &id)])
            })
    }

    /// Managed club display name, or `—` when no career is active.
    fn managed_club_name(&self) -> String {
        self.state
            .managed_club_id
            .and_then(|id| self.clubs.iter().find(|club| club.id == id))
            .map(|club| club.name.clone())
            .unwrap_or_else(|| crate::i18n::t("missing-value"))
    }

    /// Re-save the active career to the file it was started from or
    /// loaded from.
    fn save_active_career(&mut self) {
        let Some(file_name) = self.career.active_save_file.clone() else {
            self.status = t("shell-no-active-save");
            return;
        };
        let path = match saves_dir() {
            Ok(dir) => dir.join(&file_name),
            Err(error) => {
                self.status = error;
                return;
            }
        };
        match save_career_to(&path, &SavedCareer::new(self.state.clone())) {
            Ok(()) => self.status = tr("shell-saved-as", &[("file", &file_name)]),
            Err(error) => self.status = error,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn sidebar_lists_fifteen_unique_sections() {
        let labels: Vec<String> = GameSection::ALL.iter().map(|s| s.label()).collect();
        assert_eq!(labels.len(), 15);
        assert_eq!(labels.iter().collect::<HashSet<_>>().len(), 15);
    }

    #[test]
    fn shell_opens_on_squad() {
        assert_eq!(ShellState::default().section, GameSection::Squad);
    }

    #[test]
    fn every_section_cites_a_spec() {
        for section in GameSection::ALL {
            assert!(section.spec().starts_with("docs/"), "{section:?}");
        }
    }
}
