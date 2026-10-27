// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! Main-menu shell: sidebar navigation with a top info bar, mirroring
//! the original game's main screen layout (sections on the left,
//! content on the right, manager/club/date plus Continue on top).
//! Sections render real data where an engine exists (squad) and an
//! honest placeholder citing the backing spec everywhere else.

use cm0102_core::{
    Fixture, Formation, Lineup, MatchPlayer, Mentality, Player, SavedCareer,
    Schedule, TableRules, TeamTactics, LINEUP_SIZE, process_day,
};

/// Fallback average ability when no lineup resolves (placeholder).
const DEFAULT_AVG_CA: f32 = 140.0;

use super::career::{save_career_to, saves_dir};
use super::matchday::MatchDayState;
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
    /// Played fixture shown in the detail panel, if any.
    pub fixture_detail: Option<Fixture>,
    /// Active match-day screen, hiding the sidebar layout.
    pub match_day: Option<MatchDayState>,
    /// Fresh-news ids queued for modal presentation.
    pub news_queue: Vec<u64>,
    /// Queue size when opened, for the (i/n) title.
    pub news_queue_total: usize,
    /// Inbox item selected in the News section, if any.
    pub news_selected: Option<u64>,
}

impl OpenCmApp {
    /// Main-menu screen: top info bar, next match, sidebar, content.
    /// The match-day screen replaces everything below the top bar.
    pub fn show_game(&mut self, ui: &mut egui::Ui) {
        if self.shell.match_day.is_some() {
            self.show_matchday(ui);
            return;
        }
        ui.horizontal(|ui| {
            self.show_sidebar(ui);
            ui.separator();
            ui.vertical(|ui| {
                self.show_section(ui);
            });
        });
    }

    /// Fixed headerbar: manager, club, date and next match info on
    /// the left, the Menu on the right. Actions live in the footer.
    pub(super) fn show_game_header(&mut self, ui: &mut egui::Ui) {
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
            if let Some((line, due)) = self.next_match_info() {
                ui.separator();
                ui.label(line);
                if due
                    && self.shell.match_day.is_none()
                    && ui.button(t("fixtures-match-day")).clicked()
                {
                    self.enter_match_day();
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.menu_button(t("shell-menu"), |ui| {
                    if ui.button(t("shell-save-game")).clicked() {
                        self.save_active_career();
                        ui.close_menu();
                    }
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

    /// Simulate a week with the assistant in charge, then report
    /// where the holiday landed.
    pub(super) fn holiday_week(&mut self) {
        for _ in 0..7 {
            let schedule = self.state.schedule.clone();
            process_day(&mut self.state, &schedule);
            // The assistant takes charge of due matches.
            self.simulate_day(true);
        }
        self.rollover_calendar_if_needed();
        self.status = tr(
            "shell-holidayed-to",
            &[("date", &self.state.current_date.to_string())],
        );
    }

    /// Next unplayed fixture of the managed club as a display line,
    /// plus whether it is due (playable now).
    fn next_match_info(&self) -> Option<(String, bool)> {
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
            .min_by_key(|fixture| fixture.date)?;
        let date = next.date.to_string();
        let game = self.fixture_line(next);
        let line = tr("fixtures-next-match", &[("match", &game), ("date", &date)]);
        let due = managed
            .and_then(|id| self.state.schedule.due_unplayed(id, today))
            .is_some();
        Some((line, due))
    }

    /// Section list on the left, original-game order. News carries
    /// an unread badge.
    fn show_sidebar(&mut self, ui: &mut egui::Ui) {
        let unread = self.unread_news_count();
        ui.vertical(|ui| {
            ui.set_min_width(140.0);
            for section in GameSection::ALL {
                let mut label = section.label();
                if *section == GameSection::News && unread > 0 {
                    label = format!("{label} ({unread})");
                }
                let selected = self.shell.section == *section;
                if ui.selectable_label(selected, label).clicked() {
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
            GameSection::News => self.show_news_section(ui),
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

    /// Managed club's fixtures: results so far and upcoming rounds.
    /// Matches are played from the match-day screen, never here.
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
            .cloned()
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
        let rows: Vec<(String, String)> = played
            .iter()
            .rev()
            .map(|fixture| (fixture.date.to_string(), self.fixture_line(fixture)))
            .collect();
        let mut clicked: Option<Option<Fixture>> = None;
        egui::Grid::new("shell_results_grid")
            .striped(true)
            .show(ui, |ui| {
                for (fixture, (date, line)) in played.iter().rev().zip(rows.iter()) {
                    ui.label(date);
                    let selected = self.shell.fixture_detail.as_ref().is_some_and(|detail| {
                        detail.date == fixture.date
                            && detail.home_club_id == fixture.home_club_id
                            && detail.away_club_id == fixture.away_club_id
                    });
                    if ui.selectable_label(selected, line).clicked() {
                        clicked = Some(if selected { None } else { Some(fixture.clone()) });
                    }
                    ui.end_row();
                }
            });
        if let Some(detail) = clicked {
            self.shell.fixture_detail = detail;
        }
        self.show_fixture_detail(ui);
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

    /// Detail panel for the selected played fixture: events, shot
    /// counts and per-player ratings from the stored report.
    fn show_fixture_detail(&mut self, ui: &mut egui::Ui) {
        let Some(detail) = self.shell.fixture_detail.clone() else {
            return;
        };
        let Some(report) = self
            .state
            .schedule
            .fixtures
            .iter()
            .find(|fixture| {
                fixture.date == detail.date
                    && fixture.home_club_id == detail.home_club_id
                    && fixture.away_club_id == detail.away_club_id
            })
            .and_then(|fixture| fixture.report.clone())
        else {
            ui.label(t("fixtures-no-report"));
            return;
        };
        ui.separator();
        super::matchday::show_match_report(ui, &report);
    }

    /// One-line display of a fixture: score when played, versus when not.
    pub(super) fn fixture_line(&self, fixture: &Fixture) -> String {
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

    /// Mentality and deep-block flag for a side: the managed club
    /// plays its own instructions, every other club a balanced default.
    pub(super) fn side_instructions(&self, club_id: u32) -> (Mentality, bool) {
        if self.state.managed_club_id == Some(club_id) {
            let team = &self.state.tactics.team;
            (team.mentality, team.men_behind_ball)
        } else {
            (Mentality::Normal, false)
        }
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
                    let mut line = format!("{} (CA {ca})", player.full_name());
                    let available = self.state.is_available(player.id);
                    if !available {
                        line.push_str(&self.unavailability_marker(player.id));
                    }
                    if ui
                        .add_enabled(available, egui::SelectableLabel::new(false, line))
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

    /// Display name plus ability of a cached squad player, with an
    /// unavailability marker when banned or injured.
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
                let mut line = format!("{} (CA {ca})", player.full_name());
                if !self.state.is_available(player_id) {
                    line.push_str(&self.unavailability_marker(player_id));
                }
                line
            })
            .unwrap_or_else(|| t("missing-value"))
    }

    /// Short marker describing why a player is unavailable.
    fn unavailability_marker(&self, player_id: u32) -> String {
        if self
            .state
            .suspensions
            .get(&player_id)
            .is_some_and(|left| *left > 0)
        {
            return format!(" {}", t("tactics-suspended"));
        }
        if let Some(fit) = self.state.injuries.get(&player_id) {
            if self.state.current_date < *fit {
                return format!(" {} {fit}", t("tactics-injured"));
            }
        }
        String::new()
    }

    /// Average current ability of a club's eleven: the stored lineup
    /// for the managed club, a best-eleven auto-pick for AI clubs.
    pub(super) fn team_avg_ca(&self, club_id: u32) -> f32 {
        self.team_avg_ca_cached(club_id, &mut std::collections::HashMap::new())
    }

    /// Cached variant for batch simulation across many fixtures.
    pub(super) fn team_avg_ca_cached(
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

    /// Starting eleven and named substitutes as engine players: the
    /// stored lineup for the managed club, a best-eleven auto-pick
    /// for AI clubs.
    pub(super) fn match_squad(&self, club_id: u32) -> (Vec<MatchPlayer>, Vec<MatchPlayer>) {
        self.match_squad_cached(club_id, &mut std::collections::HashMap::new())
    }

    /// Cached variant for batch simulation across many fixtures.
    pub(super) fn match_squad_cached(
        &self,
        club_id: u32,
        cache: &mut std::collections::HashMap<u32, (Vec<MatchPlayer>, Vec<MatchPlayer>)>,
    ) -> (Vec<MatchPlayer>, Vec<MatchPlayer>) {
        if let Some(squad) = cache.get(&club_id) {
            return squad.clone();
        }
        let players = self
            .provider
            .as_ref()
            .and_then(|provider| provider.get_players_by_club(club_id).ok())
            .unwrap_or_default();
        let available: Vec<Player> = players
            .iter()
            .filter(|player| player.is_player && self.state.is_available(player.id))
            .cloned()
            .collect();
        let ids: Vec<u32> = if Some(club_id) == self.state.managed_club_id
            && self.state.tactics.lineup.validate().is_ok()
        {
            // Stored lineup minus the unavailable, backfilled best-first.
            let mut kept: Vec<u32> = self
                .state
                .tactics
                .lineup
                .starting
                .iter()
                .copied()
                .filter(|id| self.state.is_available(*id))
                .collect();
            let mut backfill: Vec<&Player> = available
                .iter()
                .filter(|player| !kept.contains(&player.id))
                .collect();
            backfill.sort_by_key(|player| std::cmp::Reverse(player.current_ability));
            for player in backfill {
                if kept.len() >= LINEUP_SIZE {
                    break;
                }
                kept.push(player.id);
            }
            kept
        } else {
            Lineup::auto_pick(Formation::FourFourTwo, &available).starting
        };
        let engine_player = |player: &Player| MatchPlayer {
            id: player.id,
            name: player.full_name(),
            current_ability: player.current_ability.unwrap_or(100),
            attributes: player.attributes.clone(),
            // Fitness tracking lands with training; everyone starts fit.
            condition: 100,
            morale: 0,
        };
        let eleven: Vec<MatchPlayer> = ids
            .iter()
            .filter_map(|id| players.iter().find(|player| player.id == *id).map(engine_player))
            .collect();
        let subs: Vec<MatchPlayer> = available
            .iter()
            .filter(|player| !ids.contains(&player.id))
            .take(7)
            .map(engine_player)
            .collect();
        cache.insert(club_id, (eleven.clone(), subs.clone()));
        (eleven, subs)
    }

    /// Simulate every due AI fixture: unplayed, dated today or
    /// earlier, not involving the managed club. The manager's own
    /// matches are never touched — overdue ones stay playable.
    /// Serve one suspension match for every squad member of both
    /// clubs. Runs before the new report is applied, so fresh bans
    /// start counting from the next match.
    pub(super) fn serve_match_bans(&mut self, home_club_id: u32, away_club_id: u32) {
        let mut ids = Vec::new();
        if let Some(provider) = &self.provider {
            for club_id in [home_club_id, away_club_id] {
                if let Ok(squad) = provider.get_players_by_club(club_id) {
                    ids.extend(squad.iter().map(|player| player.id));
                }
            }
        }
        self.state.serve_suspension_match(&ids);
    }

    /// Club display name, falling back to the raw id when unknown.
    pub(super) fn club_display_name(&self, club_id: u32) -> String {
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
    pub(super) fn save_active_career(&mut self) {
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
