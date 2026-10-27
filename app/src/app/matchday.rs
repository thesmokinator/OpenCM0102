// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! Match-day flow: pre-match build-up, live commentary reveal with a
//! half-time pause, and the full-time report. Continue enters this
//! screen on match days; the match cannot be skipped, only played.
//! Mid-match tactical changes are not supported yet: the engine
//! resolves the whole fixture at kick-off and the live view reveals
//! the recorded incidents progressively.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use cm0102_core::{Fixture, MatchResult, PlayerRating};

use super::*;
use crate::i18n::{t, tr};

/// How fast live incidents are revealed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CommentarySpeed {
    /// One incident every ~1.8s.
    VerySlow,
    /// One incident every ~0.9s.
    Slow,
    /// One incident every ~0.4s.
    #[default]
    Normal,
    /// One incident every ~0.15s.
    Fast,
    /// One incident every ~40ms.
    VeryFast,
}

impl CommentarySpeed {
    /// Every speed, slowest first.
    pub const ALL: &'static [CommentarySpeed] = &[
        CommentarySpeed::VerySlow,
        CommentarySpeed::Slow,
        CommentarySpeed::Normal,
        CommentarySpeed::Fast,
        CommentarySpeed::VeryFast,
    ];

    /// Pause between revealed incidents.
    pub fn interval(self) -> Duration {
        match self {
            CommentarySpeed::VerySlow => Duration::from_millis(1800),
            CommentarySpeed::Slow => Duration::from_millis(900),
            CommentarySpeed::Normal => Duration::from_millis(400),
            CommentarySpeed::Fast => Duration::from_millis(150),
            CommentarySpeed::VeryFast => Duration::from_millis(40),
        }
    }

    /// Localized label.
    pub fn label(self) -> String {
        t(match self {
            CommentarySpeed::VerySlow => "matchday-speed-very-slow",
            CommentarySpeed::Slow => "matchday-speed-slow",
            CommentarySpeed::Normal => "matchday-speed-normal",
            CommentarySpeed::Fast => "matchday-speed-fast",
            CommentarySpeed::VeryFast => "matchday-speed-very-fast",
        })
    }
}

/// Match-day screen state. UI memory only, never saved.
pub struct MatchDayState {
    /// The fixture being played.
    pub fixture: Fixture,
    /// Engine report once kicked off, `None` before kick-off.
    pub report: Option<MatchResult>,
    /// Incidents revealed so far in the live view.
    pub revealed: usize,
    /// Commentary speed.
    pub speed: CommentarySpeed,
    /// Second half underway (half-time pause dismissed).
    pub second_half: bool,
    /// Post-match news already generated.
    pub news_done: bool,
    /// Bans before kick-off, by player id (for news diffing).
    pub pre_suspensions: HashMap<u32, u32>,
    /// Injury fit-dates before kick-off, by player id.
    pub pre_injuries: HashMap<u32, cm0102_core::GameDate>,
    /// Last reveal tick, if any.
    pub last_tick: Option<Instant>,
    /// Selected live tab.
    pub tab: MatchDayTab,
}

/// Live view tabs, mirroring the original match screens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MatchDayTab {
    /// Running score plus revealed incidents.
    #[default]
    Overview,
    /// Shot counts (full time only).
    Stats,
    /// Per-player ratings (full time only).
    Ratings,
}

impl MatchDayTab {
    /// Every tab, in display order.
    pub const ALL: &'static [MatchDayTab] =
        &[MatchDayTab::Overview, MatchDayTab::Stats, MatchDayTab::Ratings];

    /// Localized label.
    pub fn label(self) -> String {
        t(match self {
            MatchDayTab::Overview => "matchday-tab-overview",
            MatchDayTab::Stats => "matchday-tab-stats",
            MatchDayTab::Ratings => "matchday-tab-ratings",
        })
    }
}

impl MatchDayState {
    /// Pre-match state for a fixture not yet kicked off.
    pub fn pre_match(fixture: Fixture) -> Self {
        Self {
            fixture,
            report: None,
            revealed: 0,
            speed: CommentarySpeed::Normal,
            second_half: false,
            news_done: false,
            pre_suspensions: HashMap::new(),
            pre_injuries: HashMap::new(),
            last_tick: None,
            tab: MatchDayTab::Overview,
        }
    }

    /// Running score from the revealed incidents.
    pub fn live_score(&self) -> (u32, u32) {
        let Some(report) = &self.report else {
            return (0, 0);
        };
        report
            .events
            .iter()
            .take(self.revealed)
            .map(|event| event.goals())
            .fold((0, 0), |(h, a), (eh, ea)| (h + eh, a + ea))
    }

    /// Index of the first second-half incident: reveal pauses here
    /// for half time.
    pub fn second_half_index(&self) -> usize {
        let Some(report) = &self.report else {
            return 0;
        };
        report
            .events
            .iter()
            .position(|event| event.minute > report.first_half_end)
            .unwrap_or(report.events.len())
    }

    /// Current match minute from the last revealed incident.
    pub fn live_minute(&self) -> u32 {
        let Some(report) = &self.report else {
            return 0;
        };
        report
            .events
            .iter()
            .take(self.revealed)
            .map(|event| event.minute)
            .max()
            .unwrap_or(0)
    }

    /// True once every incident is revealed.
    pub fn is_full_time(&self) -> bool {
        self.report
            .as_ref()
            .is_some_and(|report| self.revealed >= report.events.len())
    }
}

/// Presentation-only weather pick, deterministic per fixture seed.
/// The original game drives scheduling from weather; no gameplay
/// effect is modelled here.
pub fn weather_label_key(seed: u64) -> &'static str {
    const KEYS: [&str; 8] = [
        "weather-clear",
        "weather-drizzle",
        "weather-shower",
        "weather-downpour",
        "weather-breezy",
        "weather-strong-wind",
        "weather-light-snow",
        "weather-heavy-snow",
    ];
    KEYS[(seed % KEYS.len() as u64) as usize]
}

/// Full report view shared by the match-day screen and the fixtures
/// detail panel: incidents, shot counts and per-player ratings.
pub(super) fn show_match_report(ui: &mut egui::Ui, report: &MatchResult) {
    ui.heading(t("fixtures-detail-events"));
    for event in &report.events {
        ui.label(event.display());
    }
    let stats = report.stats;
    ui.label(tr(
        "fixtures-detail-stats",
        &[
            ("hs", &stats.home_shots.to_string()),
            ("as", &stats.away_shots.to_string()),
            ("hot", &stats.home_on_target.to_string()),
            ("aot", &stats.away_on_target.to_string()),
        ],
    ));
    ui.heading(t("fixtures-detail-ratings"));
    show_ratings_grid(ui, &report.ratings);
}

/// Per-player ratings grid.
pub(super) fn show_ratings_grid(ui: &mut egui::Ui, ratings: &[PlayerRating]) {
    egui::Grid::new("matchday_ratings_grid")
        .striped(true)
        .show(ui, |ui| {
            ui.strong(t("shell-th-name"));
            ui.strong(t("fixtures-th-rating"));
            ui.strong(t("fixtures-th-minutes"));
            ui.strong(t("fixtures-th-goals"));
            ui.strong(t("fixtures-th-cards"));
            ui.end_row();
            for rating in ratings {
                ui.label(&rating.name);
                ui.label(format!("{}.{}", rating.rating / 10, rating.rating % 10));
                ui.label(rating.minutes.to_string());
                ui.label(rating.goals.to_string());
                let cards = match (rating.yellow, rating.red) {
                    (false, false) => t("missing-value"),
                    (true, false) => "Y".to_string(),
                    (_, true) => "R".to_string(),
                };
                ui.label(cards);
                ui.end_row();
            }
        });
}

impl OpenCmApp {
    /// Enter the match-day screen for the managed club's due
    /// fixture. Does nothing when none is due.
    pub(super) fn enter_match_day(&mut self) {
        let today = self.state.current_date;
        let Some(managed) = self.state.managed_club_id else {
            return;
        };
        if let Some(fixture) = self.state.schedule.due_unplayed(managed, today).cloned() {
            self.shell.match_day = Some(MatchDayState::pre_match(fixture));
        }
    }

    /// Match-day screen replacing the sidebar layout while active.
    pub(super) fn show_matchday(&mut self, ui: &mut egui::Ui) {
        let started = self
            .shell
            .match_day
            .as_ref()
            .is_some_and(|day| day.report.is_some());
        if started {
            self.show_live_match(ui);
        } else {
            self.show_prematch(ui);
        }
    }

    /// Pre-match build-up: tie, venue, weather and the managed
    /// eleven, with kick-off.
    fn show_prematch(&mut self, ui: &mut egui::Ui) {
        let Some(day) = self.shell.match_day.as_ref() else {
            return;
        };
        let fixture = day.fixture.clone();
        ui.heading(t("matchday-title"));
        ui.label(self.fixture_line(&fixture));
        let venue = self.club_display_name(fixture.home_club_id);
        let weather = t(weather_label_key(fixture.seed()));
        ui.label(tr(
            "matchday-venue",
            &[
                ("venue", &venue),
                ("date", &fixture.date.to_string()),
                ("weather", &weather),
            ],
        ));
        ui.separator();
        ui.heading(t("matchday-your-eleven"));
        let managed = self.state.managed_club_id.unwrap_or(u32::MAX);
        let (eleven, _) = self.match_squad(managed);
        if eleven.is_empty() {
            ui.label(t("tactics-no-squad"));
        } else {
            for player in &eleven {
                ui.label(&player.name);
            }
        }
        ui.separator();
        if ui.button(t("matchday-play-match")).clicked() {
            self.kick_off();
        }
    }

    /// Resolve the fixture at kick-off: simulate, record, apply
    /// discipline, snapshot pre-match bans and injuries for news.
    fn kick_off(&mut self) {
        let Some(day) = self.shell.match_day.as_mut() else {
            return;
        };
        let fixture = day.fixture.clone();
        day.pre_suspensions = self.state.suspensions.clone();
        day.pre_injuries = self.state.injuries.clone();
        let outcome = self.simulate_and_record(&fixture);
        if let Some(day) = self.shell.match_day.as_mut() {
            day.report = Some(outcome);
            day.revealed = 0;
            day.second_half = false;
            day.last_tick = None;
        }
    }

    /// Live commentary plus half-time pause and full-time report.
    fn show_live_match(&mut self, ui: &mut egui::Ui) {
        // Advance the reveal on a timer, pausing at half time.
        let interval = self
            .shell
            .match_day
            .as_ref()
            .map(|day| day.speed.interval())
            .unwrap_or(Duration::from_millis(400));
        let mut start_second_half = false;
        let mut full_time = false;
        if let Some(day) = self.shell.match_day.as_mut() {
            let Some(report) = day.report.as_ref() else {
                return;
            };
            let total = report.events.len();
            let ht_index = day.second_half_index();
            let at_half_time =
                !day.second_half && ht_index > 0 && day.revealed >= ht_index && total > ht_index;
            let ticked = day
                .last_tick
                .is_some_and(|last| last.elapsed() >= interval);
            if day.last_tick.is_none() || (ticked && !at_half_time && day.revealed < total) {
                day.revealed = (day.revealed + 1).min(total);
                day.last_tick = Some(Instant::now());
            }
            full_time = day.is_full_time();
        }
        if full_time {
            self.show_full_time(ui);
            return;
        }
        let Some(day) = self.shell.match_day.as_ref() else {
            return;
        };
        let fixture = day.fixture.clone();
        let report = day.report.clone().expect("live match has a report");
        let (home_goals, away_goals) = day.live_score();
        let minute = day.live_minute();
        let revealed = day.revealed;
        let ht_index = day.second_half_index();
        let at_half_time =
            !day.second_half && ht_index > 0 && revealed >= ht_index && report.events.len() > ht_index;
        let tab = day.tab;
        ui.heading(t("matchday-title"));
        ui.label(
            tr(
                "matchday-live-score",
                &[
                    ("home", &self.club_display_name(fixture.home_club_id)),
                    ("hg", &home_goals.to_string()),
                    ("ag", &away_goals.to_string()),
                    ("away", &self.club_display_name(fixture.away_club_id)),
                    ("minute", &minute.to_string()),
                ],
            ),
        );
        ui.horizontal(|ui| {
            ui.label(t("matchday-speed"));
            let mut speed = self.shell.match_day.as_ref().expect("live").speed;
            egui::ComboBox::from_id_salt("matchday_speed")
                .selected_text(speed.label())
                .show_ui(ui, |ui| {
                    for candidate in CommentarySpeed::ALL {
                        ui.selectable_value(&mut speed, *candidate, candidate.label());
                    }
                });
            if let Some(day) = self.shell.match_day.as_mut() {
                day.speed = speed;
            }
        });
        ui.horizontal(|ui| {
            for candidate in MatchDayTab::ALL {
                let selected = tab == *candidate;
                if ui.selectable_label(selected, candidate.label()).clicked() {
                    if let Some(day) = self.shell.match_day.as_mut() {
                        day.tab = *candidate;
                    }
                }
            }
        });
        ui.separator();
        match tab {
            MatchDayTab::Overview => {
                egui::ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for event in report.events.iter().take(revealed) {
                            ui.label(event.display());
                        }
                    });
            }
            MatchDayTab::Stats | MatchDayTab::Ratings => {
                ui.label(t("matchday-available-at-ft"));
            }
        }
        if at_half_time {
            ui.separator();
            ui.heading(t("matchday-half-time"));
            if ui.button(t("matchday-second-half")).clicked() {
                start_second_half = true;
            }
        } else {
            ui.ctx().request_repaint_after(interval);
        }
        if start_second_half {
            if let Some(day) = self.shell.match_day.as_mut() {
                day.second_half = true;
                day.last_tick = None;
            }
        }
    }

    /// Full-time report: score, stats, ratings, then back to the
    /// main screen with the post-match news queued.
    fn show_full_time(&mut self, ui: &mut egui::Ui) {
        let pending = self.shell.match_day.as_ref().and_then(|day| {
            if day.news_done {
                return None;
            }
            day.report
                .clone()
                .map(|report| (day.fixture.clone(), report, day.pre_suspensions.clone(), day.pre_injuries.clone()))
        });
        if let Some((fixture, report, pre_suspensions, pre_injuries)) = pending {
            self.finalize_own_match(&fixture, &report, &pre_suspensions, &pre_injuries);
            if let Some(day) = self.shell.match_day.as_mut() {
                day.news_done = true;
            }
        }
        let Some(day) = self.shell.match_day.as_ref() else {
            return;
        };
        let fixture = day.fixture.clone();
        let report = day.report.clone().expect("full time has a report");
        let tab = day.tab;
        ui.heading(t("matchday-full-time"));
        ui.label(
            tr(
                "matchday-final-score",
                &[
                    ("home", &self.club_display_name(fixture.home_club_id)),
                    ("hg", &report.home_goals.to_string()),
                    ("ag", &report.away_goals.to_string()),
                    ("away", &self.club_display_name(fixture.away_club_id)),
                ],
            ),
        );
        ui.horizontal(|ui| {
            for candidate in MatchDayTab::ALL {
                let selected = tab == *candidate;
                if ui.selectable_label(selected, candidate.label()).clicked() {
                    if let Some(day) = self.shell.match_day.as_mut() {
                        day.tab = *candidate;
                    }
                }
            }
        });
        ui.separator();
        match tab {
            MatchDayTab::Overview => {
                for event in &report.events {
                    ui.label(event.display());
                }
                if let Some(shootout) = report.shootout {
                    ui.label(tr(
                        "matchday-shootout",
                        &[
                            ("hs", &shootout.home_scored.to_string()),
                            ("as", &shootout.away_scored.to_string()),
                        ],
                    ));
                }
            }
            MatchDayTab::Stats => {
                let stats = report.stats;
                ui.label(tr(
                    "fixtures-detail-stats",
                    &[
                        ("hs", &stats.home_shots.to_string()),
                        ("as", &stats.away_shots.to_string()),
                        ("hot", &stats.home_on_target.to_string()),
                        ("aot", &stats.away_on_target.to_string()),
                    ],
                ));
            }
            MatchDayTab::Ratings => {
                show_ratings_grid(ui, &report.ratings);
            }
        }
        ui.separator();
        if ui.button(t("matchday-back")).clicked() {
            self.shell.match_day = None;
            self.open_news_queue();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weather_pick_is_deterministic_per_seed() {
        assert_eq!(weather_label_key(7), weather_label_key(7));
        assert!(weather_label_key(0).starts_with("weather-"));
    }

    #[test]
    fn live_score_follows_revealed_goals() {
        use cm0102_core::{EventKind, MatchEvent};
        let mut day = MatchDayState::pre_match(Fixture {
            date: cm0102_core::GameDate::SEASON_START,
            home_club_id: 1,
            away_club_id: 2,
            competition_id: 7,
            result: None,
            report: None,
        });
        day.report = Some(MatchResult {
            home_goals: 1,
            away_goals: 1,
            events: vec![
                MatchEvent {
                    minute: 10,
                    home: true,
                    kind: EventKind::Goal,
                    player: "A".to_string(),
                    other: None,
                },
                MatchEvent {
                    minute: 80,
                    home: false,
                    kind: EventKind::Goal,
                    player: "B".to_string(),
                    other: None,
                },
            ],
            stats: cm0102_core::MatchStats::default(),
            ratings: Vec::new(),
            shootout: None,
            injuries: Vec::new(),
            first_half_end: 46,
        });
        assert_eq!(day.live_score(), (0, 0));
        assert!(!day.is_full_time());
        day.revealed = 1;
        assert_eq!(day.live_score(), (1, 0));
        assert_eq!(day.second_half_index(), 1);
        assert_eq!(day.live_minute(), 10);
        day.revealed = 2;
        assert_eq!(day.live_score(), (1, 1));
        assert!(day.is_full_time());
    }
}
