// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! Match-day flow: pre-match build-up, live commentary reveal with a
//! half-time pause, and the full-time report. Continue enters this
//! screen on match days; the match cannot be skipped, only played.
//! The reveal drives a live simulation minute by minute, so the
//! half-time team talk, pauses and substitutions change what
//! happens next instead of narrating a fixed report.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use cm0102_core::{
    EventKind, Fixture, FixtureResult, MatchEvent, MatchResult, MatchStats, Mentality, PlayerRating,
};
use cm0102_match::LiveMatch;

use super::*;
use crate::i18n::{t, tr};

/// How fast live incidents are revealed. Display pacing only:
/// speed never changes what happens, only how fast it is told.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CommentarySpeed {
    /// Twice the cfg hold (a 900ms incident lingers 1.8s).
    VerySlow,
    /// Half again the cfg hold.
    Slow,
    /// The cfg hold as authored (~0.9s typical, ~3s for goals).
    #[default]
    Medium,
    /// Less than half the cfg hold.
    Fast,
    /// A flash of the cfg hold.
    VeryFast,
}

impl CommentarySpeed {
    /// Every speed, slowest first.
    pub const ALL: &'static [CommentarySpeed] = &[
        CommentarySpeed::VerySlow,
        CommentarySpeed::Slow,
        CommentarySpeed::Medium,
        CommentarySpeed::Fast,
        CommentarySpeed::VeryFast,
    ];

    /// Multiplier on the cfg display hold: Medium plays the
    /// author's timing untouched.
    fn delay_scale(self) -> f32 {
        match self {
            CommentarySpeed::VerySlow => 2.0,
            CommentarySpeed::Slow => 1.5,
            CommentarySpeed::Medium => 1.0,
            CommentarySpeed::Fast => 0.4,
            CommentarySpeed::VeryFast => 0.1,
        }
    }

    /// Localized label.
    pub fn label(self) -> String {
        t(match self {
            CommentarySpeed::VerySlow => "matchday-speed-very-slow",
            CommentarySpeed::Slow => "matchday-speed-slow",
            CommentarySpeed::Medium => "matchday-speed-medium",
            CommentarySpeed::Fast => "matchday-speed-fast",
            CommentarySpeed::VeryFast => "matchday-speed-very-fast",
        })
    }
}

/// Where the match day stands: the footer shows one primary
/// action per phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MatchPhase {
    /// No match day active (hub screens).
    Hub,
    /// Build-up before kick-off: Play Match.
    Prematch,
    /// Live reveal: speed control only.
    Live,
    /// Half-time pause: Start Second Half.
    HalfTime,
    /// Full-time report: Back to the hub.
    FullTime,
}

/// Match-day screen state. UI memory only, never saved.
pub struct MatchDayState {
    /// The fixture being played.
    pub fixture: Fixture,
    /// Live simulation once kicked off, `None` before kick-off.
    pub live: Option<LiveMatch>,
    /// Display pause: the reveal (and with it the sim) holds.
    pub paused: bool,
    /// Engine snapshot once kicked off, `None` before kick-off.
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
    /// Rendered commentary lines per incident, aligned with events.
    pub commentary: Vec<Vec<String>>,
    /// Display hold per incident in milliseconds.
    pub holds: Vec<u32>,
    /// Past-tense report prose.
    pub prose: Vec<String>,
    /// Big-moment banner (goal, red card) with its start time.
    pub banner: Option<MomentBanner>,
    /// Banner sequence: every banner animates on its own id.
    pub banner_seq: u64,
}

/// A big moment worth a banner: who, when, for which side.
#[derive(Clone)]
pub struct MomentBanner {
    /// Goal or red card.
    pub kind: MomentKind,
    /// Match minute.
    pub minute: u32,
    /// Player name.
    pub player: String,
    /// True when the player's side is home.
    pub home: bool,
    /// When the banner fired.
    pub since: Instant,
    /// Sequence number, unique per banner.
    pub seq: u64,
}

/// Banner-worthy moments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MomentKind {
    /// A goal (including penalties and own goals).
    Goal,
    /// A red card.
    RedCard,
}

impl MomentKind {
    /// Banner accent colour.
    fn accent(self) -> egui::Color32 {
        match self {
            MomentKind::Goal => crate::theme::palette::GOLD,
            MomentKind::RedCard => crate::theme::palette::CARD_RED,
        }
    }

    /// Localized banner title key.
    fn title_key(self) -> &'static str {
        match self {
            MomentKind::Goal => "matchday-goal",
            MomentKind::RedCard => "matchday-red-card",
        }
    }
}

/// Seconds a big-moment banner stays up.
const BANNER_SECS: f32 = 3.5;

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
            live: None,
            paused: false,
            report: None,
            revealed: 0,
            speed: CommentarySpeed::Medium,
            second_half: false,
            news_done: false,
            pre_suspensions: HashMap::new(),
            pre_injuries: HashMap::new(),
            last_tick: None,
            tab: MatchDayTab::Overview,
            commentary: Vec::new(),
            holds: Vec::new(),
            prose: Vec::new(),
            banner: None,
            banner_seq: 0,
        }
    }

    /// Fire a banner when `event` is a big moment.
    fn fire_banner_for(&mut self, event: &MatchEvent) {
        let kind = match event.kind {
            EventKind::Goal | EventKind::Penalty | EventKind::OwnGoal => MomentKind::Goal,
            EventKind::Red => MomentKind::RedCard,
            _ => return,
        };
        self.banner_seq += 1;
        self.banner = Some(MomentBanner {
            kind,
            minute: event.minute,
            player: event.player.clone(),
            home: event.home,
            since: Instant::now(),
            seq: self.banner_seq,
        });
    }

    /// Hold before revealing incident `index`: the cfg display
    /// hold scaled by speed, so big incidents linger longer.
    fn hold_for(&self, index: usize) -> Duration {
        let delay = self.holds.get(index).copied().unwrap_or(900);
        let scaled = delay as f32 * self.speed.delay_scale();
        Duration::from_millis(scaled.max(30.0) as u64)
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

    /// True at the interval: the sim waits for release and every
    /// played incident is revealed.
    pub fn at_half_time(&self) -> bool {
        if self.second_half {
            return false;
        }
        let capped = self.live.as_ref().is_some_and(|live| live.capped_for_interval());
        let drained = self
            .report
            .as_ref()
            .is_some_and(|report| self.revealed >= report.events.len());
        capped && drained
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

/// Localized label key for a wind byte (0-4).
pub(super) fn wind_label_key(wind: u8) -> &'static str {
    match wind {
        0 => "weather-wind-calm",
        1 => "weather-wind-breezy",
        2 => "weather-wind-gusty",
        3 => "weather-wind-strong",
        _ => "weather-wind-gale",
    }
}

/// Localized label key for a precipitation byte.
pub(super) fn precip_label_key(precip: u8) -> &'static str {
    match precip {
        0 => "weather-precip-dry",
        1 => "weather-precip-wet",
        2 => "weather-precip-drizzle",
        3 => "weather-precip-shower",
        5 => "weather-precip-downpour",
        6 => "weather-precip-sleet",
        8 => "weather-precip-light-snow",
        9 => "weather-precip-heavy-snow",
        _ => "weather-precip-dry",
    }
}

/// Localized label key for a referee strictness descriptor, if
/// the value earns one (verified bands: lenient to 5, strict
/// from 15, harsh at 20).
pub(super) fn strictness_label_key(strictness: u8) -> Option<&'static str> {
    if strictness >= 20 {
        Some("referee-harsh")
    } else if strictness >= 15 {
        Some("referee-strict")
    } else if strictness <= 5 {
        Some("referee-lenient")
    } else {
        None
    }
}

/// Referee display name with its strictness descriptor, if any.
fn referee_display_name(referee: &cm0102_core::Referee) -> String {
    let mut label = referee.name.clone();
    if let Some(key) = strictness_label_key(referee.strictness) {
        label.push_str(&format!(" ({})", t(key)));
    }
    label
}

/// Localized label key for a pitch state.
pub(super) fn pitch_label_key(pitch: &cm0102_core::Pitch) -> &'static str {
    match pitch {
        cm0102_core::Pitch::Normal => "weather-pitch-normal",
        cm0102_core::Pitch::Hard => "weather-pitch-hard",
        cm0102_core::Pitch::Frozen => "weather-pitch-frozen",
        cm0102_core::Pitch::Waterlogged => "weather-pitch-waterlogged",
    }
}

/// Minute-pill colours for an incident kind: gold for goals,
/// card colours for bookings, quiet grey otherwise.
fn event_pill(kind: EventKind) -> (egui::Color32, egui::Color32) {
    use crate::theme::palette;
    match kind {
        EventKind::Goal | EventKind::Penalty | EventKind::OwnGoal => {
            (palette::GOLD, egui::Color32::BLACK)
        }
        EventKind::Yellow => (palette::CARD_YELLOW, egui::Color32::BLACK),
        EventKind::Red => (palette::CARD_RED, egui::Color32::WHITE),
        EventKind::Sub => (palette::ACCENT_DIM, palette::INK),
        EventKind::Kickoff | EventKind::HalfTime | EventKind::FullTime => {
            (palette::WELL, palette::INK_FAINT)
        }
        _ => (palette::WELL, palette::INK_DIM),
    }
}

/// One feed row: minute pill plus commentary lines (or the plain
/// incident text). `flash` (0-1) tints the row while fresh.
fn show_feed_row(
    ui: &mut egui::Ui,
    event: &MatchEvent,
    lines: Option<&Vec<String>>,
    flash: f32,
) {
    use crate::theme::palette;
    let flash = flash.clamp(0.0, 1.0);
    let fill = palette::ACCENT_DIM.gamma_multiply(0.35 * flash);
    egui::Frame::new()
        .fill(fill)
        .corner_radius(6.0)
        .inner_margin(egui::Margin::symmetric(6, 3))
        .show(ui, |ui| {
            ui.horizontal_top(|ui| {
                let (pill_fill, pill_ink) = event_pill(event.kind);
                crate::theme::pill(ui, &format!("{}'", event.minute), pill_fill, pill_ink);
                ui.vertical(|ui| {
                    ui.add_space(1.0);
                    match lines {
                        Some(lines) => {
                            for line in lines {
                                ui.label(line);
                            }
                        }
                        None => {
                            ui.label(event.display());
                        }
                    }
                });
            });
        });
}

/// Animated big-moment banner: fades in on fire, fades out over
/// its lifetime. Callers repaint while a banner is alive.
fn show_moment_banner(ui: &mut egui::Ui, banner: &MomentBanner, team: &str) {
    let age = banner.since.elapsed().as_secs_f32();
    let fade = 1.0 - (age / BANNER_SECS).clamp(0.0, 1.0);
    let glow = ui
        .ctx()
        .animate_bool(egui::Id::new(("moment_banner", banner.seq)), true);
    let alpha = (glow * fade).clamp(0.0, 1.0);
    if alpha <= 0.0 {
        return;
    }
    let accent = banner.kind.accent().gamma_multiply(alpha);
    let ink = if banner.kind == MomentKind::Goal {
        egui::Color32::BLACK.gamma_multiply(alpha)
    } else {
        egui::Color32::WHITE.gamma_multiply(alpha)
    };
    egui::Frame::new()
        .fill(accent)
        .corner_radius(10.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new(t(banner.kind.title_key()))
                        .size(26.0)
                        .color(ink),
                );
                ui.label(
                    egui::RichText::new(format!("{}  {}'", banner.player, banner.minute))
                        .size(15.0)
                        .color(ink),
                );
                ui.label(egui::RichText::new(team).size(12.5).color(ink));
            });
        });
}

/// Full report view shared by the match-day screen and the fixtures
/// detail panel: incidents, the ten-counter team block and
/// per-player ratings.
pub(super) fn show_match_report(ui: &mut egui::Ui, report: &MatchResult) {
    if let Some(crowd) = report.attendance {
        ui.label(tr(
            "matchday-attendance",
            &[("count", &crowd.to_string())],
        ));
    }
    if let Some(referee) = report
        .referee
        .as_ref()
        .filter(|referee| !referee.is_unnamed())
    {
        let name = referee_display_name(referee);
        ui.label(tr("matchday-referee", &[("name", &name)]));
    }
    ui.heading(t("fixtures-detail-events"));
    for event in &report.events {
        show_feed_row(ui, event, None, 0.0);
    }
    ui.heading(t("matchday-tab-stats"));
    show_stats_grid(ui, &report.stats);
    ui.heading(t("fixtures-detail-ratings"));
    show_ratings_grid(ui, &report.ratings);
}

/// Ten-counter team block: home value, label, away value.
pub(super) fn show_stats_grid(ui: &mut egui::Ui, stats: &MatchStats) {
    let rows = [
        ("stats-shots", stats.home_shots, stats.away_shots),
        ("stats-on-target", stats.home_on_target, stats.away_on_target),
        ("stats-corners", stats.home_corners, stats.away_corners),
        (
            "stats-free-kicks",
            stats.home_free_kicks,
            stats.away_free_kicks,
        ),
        (
            "stats-throw-ins",
            stats.home_throw_ins,
            stats.away_throw_ins,
        ),
        ("stats-offsides", stats.home_offsides, stats.away_offsides),
        ("stats-passes", stats.home_passes, stats.away_passes),
        ("stats-tackles", stats.home_tackles, stats.away_tackles),
        ("stats-headers", stats.home_headers, stats.away_headers),
    ];
    egui::Grid::new("matchday_stats_grid")
        .striped(true)
        .show(ui, |ui| {
            for (key, home, away) in rows {
                ui.label(home.to_string());
                ui.strong(t(key));
                ui.label(away.to_string());
                ui.end_row();
            }
        });
}

/// One live player row derived from revealed incidents.
#[derive(Debug, PartialEq, Eq)]
struct LiveRow {
    name: String,
    minutes: u32,
    goals: u32,
    yellow: bool,
    red: bool,
}

/// Live player table from the revealed prefix: minutes from
/// on/off incidents, goals and cards counted so far. Ratings and
/// ambient counters stay full-time only.
fn live_player_rows(
    ratings: &[PlayerRating],
    revealed: &[MatchEvent],
    live_minute: u32,
) -> Vec<LiveRow> {
    ratings
        .iter()
        .filter(|rating| u32::from(rating.on_minute) <= live_minute)
        .map(|rating| {
            let on = u32::from(rating.on_minute);
            let mut off = live_minute;
            let mut goals = 0;
            let mut yellow = false;
            let mut red = false;
            for event in revealed {
                match event.kind {
                    EventKind::Goal | EventKind::Penalty
                        if event.player == rating.name && event.home == rating.home =>
                    {
                        goals += 1;
                    }
                    EventKind::Yellow if event.player == rating.name => yellow = true,
                    EventKind::Red if event.player == rating.name => {
                        red = true;
                        off = off.min(event.minute);
                    }
                    EventKind::Sub if event.other.as_deref() == Some(&rating.name) => {
                        off = off.min(event.minute);
                    }
                    _ => {}
                }
            }
            LiveRow {
                name: rating.name.clone(),
                minutes: off.saturating_sub(on),
                goals,
                yellow,
                red,
            }
        })
        .collect()
}

/// Live player table: minutes, goals and cards so far.
fn show_live_players_grid(ui: &mut egui::Ui, rows: &[LiveRow]) {
    egui::Grid::new("matchday_live_grid")
        .striped(true)
        .show(ui, |ui| {
            ui.strong(t("shell-th-name"));
            ui.strong(t("fixtures-th-minutes"));
            ui.strong(t("fixtures-th-goals"));
            ui.strong(t("fixtures-th-cards"));
            ui.end_row();
            for row in rows {
                ui.label(&row.name);
                ui.label(row.minutes.to_string());
                ui.label(row.goals.to_string());
                let cards = match (row.yellow, row.red) {
                    (false, false) => t("missing-value"),
                    (true, false) => "Y".to_string(),
                    (_, true) => "R".to_string(),
                };
                ui.label(cards);
                ui.end_row();
            }
        });
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
            ui.strong(t("fixtures-th-shots"));
            ui.strong(t("fixtures-th-passes"));
            ui.strong(t("fixtures-th-tackles"));
            ui.strong(t("fixtures-th-headers"));
            ui.strong(t("fixtures-th-cards"));
            ui.end_row();
            for rating in ratings {
                ui.label(&rating.name);
                ui.label(format!("{}.{}", rating.rating / 10, rating.rating % 10));
                ui.label(rating.minutes.to_string());
                ui.label(rating.goals.to_string());
                ui.label(format!("{}/{}", rating.shots, rating.on_target));
                ui.label(rating.passes.to_string());
                ui.label(rating.tackles.to_string());
                ui.label(rating.headers.to_string());
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

    /// Current match phase, driving the footer's contextual actions.
    pub(super) fn match_phase(&self) -> MatchPhase {
        let Some(day) = self.shell.match_day.as_ref() else {
            return MatchPhase::Hub;
        };
        if day.report.is_none() {
            return MatchPhase::Prematch;
        }
        if day.at_half_time() {
            return MatchPhase::HalfTime;
        }
        if day.is_full_time() {
            return MatchPhase::FullTime;
        }
        MatchPhase::Live
    }

    /// Half-time team talk and pause panel: substitutions and
    /// mentality for the managed club. Changes apply to the live
    /// sim at once and reshape what happens next.
    fn show_intervention(&mut self, ui: &mut egui::Ui) {
        let Some(managed) = self.state.managed_club_id else {
            return;
        };
        let Some(day) = self.shell.match_day.as_ref() else {
            return;
        };
        let home = day.fixture.home_club_id == managed;
        let Some(live) = day.live.as_ref() else {
            return;
        };
        let (pitch, bench) = live.roster(home);
        let subs_left = live.subs_left(home);
        let (mentality, _) = live.instructions(home);
        ui.heading(t("matchday-team-talk"));
        let mut picked = None;
        ui.horizontal(|ui| {
            ui.label(t("matchday-mentality"));
            for (candidate, key) in [
                (Mentality::Attacking, "matchday-attacking"),
                (Mentality::Normal, "matchday-normal"),
                (Mentality::Defensive, "matchday-defensive"),
            ] {
                if ui.selectable_label(mentality == candidate, t(key)).clicked() {
                    picked = Some(candidate);
                }
            }
        });
        if let Some(next) = picked {
            if let Some(live) = self
                .shell
                .match_day
                .as_mut()
                .and_then(|day| day.live.as_mut())
            {
                live.set_mentality(home, next);
            }
            self.resimulate_to_revealed();
        }
        ui.add_space(4.0);
        if subs_left == 0 || bench.is_empty() {
            ui.label(t("matchday-no-subs"));
            return;
        }
        ui.label(tr(
            "matchday-subs-left",
            &[("n", &subs_left.to_string())],
        ));
        let mut sub_off = None;
        egui::Grid::new("matchday_subs").show(ui, |ui| {
            for player in &pitch {
                let mut name = player.name.clone();
                if player.goals > 0 {
                    name.push_str(&format!(" ({})", player.goals));
                }
                if player.yellow {
                    name.push_str(" (y)");
                }
                ui.label(name);
                if ui.button(t("matchday-sub-off")).clicked() {
                    sub_off = Some(player.id);
                }
                ui.end_row();
            }
        });
        if let Some(out_id) = sub_off {
            if let Some(live) = self
                .shell
                .match_day
                .as_mut()
                .and_then(|day| day.live.as_mut())
            {
                live.substitute(home, out_id);
            }
            self.resimulate_to_revealed();
        }
    }

    /// Dismiss the half-time pause and resume the reveal.
    pub(super) fn start_second_half(&mut self) {
        if let Some(day) = self.shell.match_day.as_mut() {
            if let Some(live) = day.live.as_mut() {
                live.open_second_half();
            }
            day.second_half = true;
            day.last_tick = None;
        }
    }

    /// Commentary speed picker, living in the footer during matches.
    pub(super) fn show_speed_combo(&mut self, ui: &mut egui::Ui) {
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

    /// Broadcast scoreboard: clubs, score, status pill and a
    /// pulsing live dot. Shared by the live and full-time views.
    fn show_scoreboard(
        &self,
        ui: &mut egui::Ui,
        fixture: &Fixture,
        home_goals: u32,
        away_goals: u32,
        status: &str,
        live: bool,
    ) {
        use crate::theme::palette;
        let home = self.club_display_name(fixture.home_club_id);
        let away = self.club_display_name(fixture.away_club_id);
        crate::theme::card().show(ui, |ui| {
            // Fixed height: bare columns would claim all the
            // vertical space and push the rest off-screen.
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), 52.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.columns(3, |cols| {
                        cols[0].with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                ui.label(egui::RichText::new(home).size(17.0));
                            },
                        );
                        cols[1].vertical_centered(|ui| {
                            ui.label(
                                egui::RichText::new(format!(
                                    "{home_goals} - {away_goals}"
                                ))
                                .text_style(egui::TextStyle::Name("score".into())),
                            );
                        });
                        cols[2].with_layout(
                            egui::Layout::left_to_right(egui::Align::Center),
                            |ui| {
                                ui.label(egui::RichText::new(away).size(17.0));
                            },
                        );
                    });
                },
            );
            ui.add_space(2.0);
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.horizontal(|ui| {
                    if live {
                        let beat = ui.ctx().input(|input| input.time) as f32 * 4.0;
                        let glow = 0.55 + 0.45 * beat.sin().abs();
                        let (rect, _) = ui.allocate_exact_size(
                            egui::vec2(10.0, 10.0),
                            egui::Sense::hover(),
                        );
                        ui.painter().circle_filled(
                            rect.center(),
                            4.5,
                            palette::ACCENT.gamma_multiply(glow),
                        );
                        ui.label(
                            egui::RichText::new(t("matchday-live"))
                                .size(12.5)
                                .color(palette::ACCENT),
                        );
                    }
                    crate::theme::pill(ui, status, palette::WELL, palette::INK);
                });
            });
        });
    }

    /// Pre-match build-up: tie, venue, weather and the managed
    /// eleven, with kick-off.
    fn show_prematch(&mut self, ui: &mut egui::Ui) {
        let Some(day) = self.shell.match_day.as_ref() else {
            return;
        };
        let fixture = day.fixture.clone();
        ui.heading(t("matchday-title"));
        self.show_scoreboard(ui, &fixture, 0, 0, &fixture.date.to_string(), false);
        ui.add_space(4.0);
        egui::ScrollArea::vertical().show(ui, |ui| {
            let venue = self.club_display_name(fixture.home_club_id);
            let played = self.fixture_weather(&fixture);
            let weather = format!(
                "{}, {} — {}",
                t(precip_label_key(played.precip)),
                t(wind_label_key(played.wind)),
                t(pitch_label_key(&played.pitch)),
            );
            crate::theme::meta(
                ui,
                &tr(
                    "matchday-venue",
                    &[
                        ("venue", &venue),
                        ("date", &fixture.date.to_string()),
                        ("weather", &weather),
                    ],
                ),
            );
            let referee = self.fixture_referee(&fixture);
            if !referee.is_unnamed() {
                let name = referee_display_name(&referee);
                crate::theme::meta(ui, &tr("matchday-referee", &[("name", &name)]));
            }
            ui.add_space(6.0);
            crate::theme::section(ui, &t("matchday-your-eleven"));
            let managed = self.state.managed_club_id.unwrap_or(u32::MAX);
            let (eleven, _) = self.match_squad(managed);
            if eleven.is_empty() {
                ui.label(t("tactics-no-squad"));
            } else {
                crate::theme::card().show(ui, |ui| {
                    for player in &eleven {
                        ui.label(&player.name);
                    }
                });
            }
            ui.add_space(8.0);
        });
    }

    /// Resolve the fixture at kick-off: simulate, record, apply
    /// discipline, snapshot pre-match bans and injuries for news,
    /// and start the live simulation. Nothing is recorded yet: the
    /// result lands in the schedule at full time.
    pub(super) fn kick_off(&mut self) {
        let Some(day) = self.shell.match_day.as_mut() else {
            return;
        };
        let fixture = day.fixture.clone();
        if self.postpone_if_unplayable(&fixture) {
            self.shell.match_day = None;
            self.open_news_queue();
            return;
        }
        let Some(day) = self.shell.match_day.as_mut() else {
            return;
        };
        day.pre_suspensions = self.state.suspensions.clone();
        day.pre_injuries = self.state.injuries.clone();
        self.serve_match_bans(fixture.home_club_id, fixture.away_club_id);
        let context = self.match_context(&fixture);
        let attendance = self.fixture_attendance(&fixture, &context.weather);
        let mut live = LiveMatch::new(context);
        live.set_attendance(attendance);
        live.play_until_incident(0);
        let report = live.snapshot();
        let home_name = self.club_display_name(fixture.home_club_id);
        let away_name = self.club_display_name(fixture.away_club_id);
        let rendered = cm0102_match::render_match(
            &self.commentary_pack,
            &report.events,
            (&home_name, &away_name),
            None,
            None,
            fixture.seed(),
        );
        if let Some(day) = self.shell.match_day.as_mut() {
            day.live = Some(live);
            day.report = Some(report);
            day.revealed = 0;
            day.second_half = false;
            day.paused = false;
            day.last_tick = None;
            day.commentary = rendered.lines;
            day.holds = rendered.delays;
            day.prose = rendered.prose;
            day.banner = None;
        }
    }

    /// Pull the simulation up to the revealed incidents and refresh
    /// the snapshot plus its rendered commentary.
    fn resimulate_to_revealed(&mut self) {
        let revealed = self
            .shell
            .match_day
            .as_ref()
            .map(|day| day.revealed)
            .unwrap_or(0);
        let Some(day) = self.shell.match_day.as_mut() else {
            return;
        };
        let Some(live) = day.live.as_mut() else {
            return;
        };
        live.play_until_incident(revealed);
        day.report = Some(live.snapshot());
        let fixture = day.fixture.clone();
        let events = day.report.as_ref().map(|report| report.events.clone());
        let Some(events) = events else {
            return;
        };
        let home_name = self.club_display_name(fixture.home_club_id);
        let away_name = self.club_display_name(fixture.away_club_id);
        let rendered = cm0102_match::render_match(
            &self.commentary_pack,
            &events,
            (&home_name, &away_name),
            None,
            None,
            fixture.seed(),
        );
        if let Some(day) = self.shell.match_day.as_mut() {
            day.commentary = rendered.lines;
            day.holds = rendered.delays;
            day.prose = rendered.prose;
        }
    }

    /// Live commentary plus half-time pause and full-time report.
    /// Each tick pulls the simulation forward, so the reveal never
    /// outruns what is played; pausing holds both.
    fn show_live_match(&mut self, ui: &mut egui::Ui) {
        // Advance the reveal on a timer, pausing at half time.
        let tick = match self.shell.match_day.as_ref() {
            Some(day) if day.report.is_some() => {
                let hold = day.hold_for(day.revealed);
                let ticked = day.last_tick.is_some_and(|last| last.elapsed() >= hold);
                day.last_tick.is_none() || (ticked && !day.at_half_time() && !day.paused)
            }
            _ => return,
        };
        if tick {
            self.resimulate_to_revealed();
        }
        let mut full_time = false;
        if let Some(day) = self.shell.match_day.as_mut() {
            let total = day.report.as_ref().map(|report| report.events.len()).unwrap_or(0);
            if day.revealed < total
                && (day.last_tick.is_none() || {
                    let hold = day.hold_for(day.revealed);
                    let ticked = day.last_tick.is_some_and(|last| last.elapsed() >= hold);
                    ticked && !day.at_half_time() && !day.paused
                })
            {
                day.revealed = (day.revealed + 1).min(total);
                day.last_tick = Some(Instant::now());
                let fired = day
                    .report
                    .as_ref()
                    .and_then(|report| report.events.get(day.revealed.saturating_sub(1)).cloned());
                if let Some(event) = fired {
                    day.fire_banner_for(&event);
                }
            }
            if day
                .banner
                .as_ref()
                .is_some_and(|banner| banner.since.elapsed().as_secs_f32() > BANNER_SECS)
            {
                day.banner = None;
            }
            full_time = day.is_full_time() && !day.at_half_time();
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
        let at_half_time = day.at_half_time();
        let can_intervene = at_half_time || day.paused;
        let tab = day.tab;
        let banner = day.banner.clone();
        let last_tick = day.last_tick;
        let status = if at_half_time {
            "HT".to_string()
        } else {
            format!("{minute}'")
        };
        self.show_scoreboard(ui, &fixture, home_goals, away_goals, &status, true);
        ui.add_space(4.0);
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
        if can_intervene {
            self.show_intervention(ui);
            ui.separator();
        }
        match tab {
            MatchDayTab::Overview => {
                let commentary = self
                    .shell
                    .match_day
                    .as_ref()
                    .map(|live| live.commentary.clone())
                    .unwrap_or_default();
                egui::ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        if let Some(banner) = banner.as_ref() {
                            let club = if banner.home {
                                fixture.home_club_id
                            } else {
                                fixture.away_club_id
                            };
                            let team = self.club_display_name(club);
                            show_moment_banner(ui, banner, &team);
                            ui.add_space(4.0);
                        }
                        // The latest row flashes briefly when revealed.
                        let fresh = last_tick
                            .map(|tick| 1.0 - tick.elapsed().as_secs_f32() / 0.9)
                            .unwrap_or(0.0);
                        for (index, event) in report.events.iter().enumerate().take(revealed) {
                            let flash = if index + 1 == revealed { fresh } else { 0.0 };
                            show_feed_row(ui, event, commentary.get(index), flash);
                        }
                        if at_half_time {
                            ui.separator();
                            ui.heading(t("matchday-half-time"));
                        }
                    });
            }
            MatchDayTab::Stats => {
                ui.label(tr(
                    "matchday-stats-minute",
                    &[("minute", &minute.to_string())],
                ));
                show_stats_grid(ui, &report.stats_upto(minute));
            }
            MatchDayTab::Ratings => {
                let end = revealed.min(report.events.len());
                let rows = live_player_rows(&report.ratings, &report.events[..end], minute);
                egui::ScrollArea::vertical().show(ui, |ui| {
                    show_live_players_grid(ui, &rows);
                });
            }
        }
        if !at_half_time {
            let hold = self
                .shell
                .match_day
                .as_ref()
                .map(|day| day.hold_for(day.revealed))
                .unwrap_or(Duration::from_millis(900));
            ui.ctx().request_repaint_after(hold);
        }
        // Animation pump: smooth banner fades and a pulsing live
        // dot. The reveal stays Instant-gated, so extra repaints
        // never speed the match up.
        let banner_alive = banner.is_some();
        if banner_alive || !at_half_time {
            ui.ctx().request_repaint_after(Duration::from_millis(120));
        }
    }

    /// Full-time report: record the result and discipline once,
    /// then score, stats, ratings, and back to the main screen with
    /// the post-match news queued.
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
            self.state.schedule.record_result(
                fixture.date,
                fixture.home_club_id,
                fixture.away_club_id,
                FixtureResult {
                    home_goals: report.home_goals,
                    away_goals: report.away_goals,
                },
                Some(report.clone()),
            );
            self.state.apply_match_report(&report);
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
        self.show_scoreboard(
            ui,
            &fixture,
            report.home_goals,
            report.away_goals,
            "FT",
            false,
        );
        ui.add_space(4.0);
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
                let commentary = self
                    .shell
                    .match_day
                    .as_ref()
                    .map(|live| live.commentary.clone())
                    .unwrap_or_default();
                let prose = self
                    .shell
                    .match_day
                    .as_ref()
                    .map(|live| live.prose.clone())
                    .unwrap_or_default();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (index, event) in report.events.iter().enumerate() {
                        show_feed_row(ui, event, commentary.get(index), 0.0);
                    }
                    if !prose.is_empty() {
                        ui.separator();
                        ui.heading(t("matchday-report"));
                        for line in &prose {
                            ui.label(line);
                        }
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
                });
            }
            MatchDayTab::Stats => {
                show_stats_grid(ui, &report.stats);
            }
            MatchDayTab::Ratings => {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    show_ratings_grid(ui, &report.ratings);
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weather_keys_cover_all_bytes() {
        assert_eq!(wind_label_key(0), "weather-wind-calm");
        assert_eq!(wind_label_key(4), "weather-wind-gale");
        assert_eq!(precip_label_key(0), "weather-precip-dry");
        assert_eq!(precip_label_key(5), "weather-precip-downpour");
        assert_eq!(precip_label_key(6), "weather-precip-sleet");
        assert_eq!(precip_label_key(9), "weather-precip-heavy-snow");
        assert_eq!(
            pitch_label_key(&cm0102_core::Pitch::Waterlogged),
            "weather-pitch-waterlogged"
        );
    }

    #[test]
    fn medium_plays_cfg_holds_untouched() {
        let mut day = MatchDayState::pre_match(Fixture {
            date: cm0102_core::GameDate::SEASON_START,
            home_club_id: 1,
            away_club_id: 2,
            competition_id: 7,
            result: None,
            report: None,
        });
        day.holds = vec![900, 3000];
        assert_eq!(day.hold_for(0), Duration::from_millis(900));
        assert_eq!(day.hold_for(1), Duration::from_millis(3000));
        day.speed = CommentarySpeed::VerySlow;
        assert_eq!(day.hold_for(0), Duration::from_millis(1800));
        day.speed = CommentarySpeed::Fast;
        assert_eq!(day.hold_for(0), Duration::from_millis(360));
        day.speed = CommentarySpeed::VeryFast;
        assert_eq!(day.hold_for(0), Duration::from_millis(90));
        // Missing holds fall back to the typical cfg beat.
        assert_eq!(day.hold_for(9), Duration::from_millis(90));
    }

    #[test]
    fn banners_fire_only_for_goals_and_reds() {
        use cm0102_core::MatchEvent;
        let mut day = MatchDayState::pre_match(Fixture {
            date: cm0102_core::GameDate::SEASON_START,
            home_club_id: 1,
            away_club_id: 2,
            competition_id: 7,
            result: None,
            report: None,
        });
        let event = |kind| MatchEvent {
            minute: 23,
            home: true,
            kind,
            player: "Striker".to_string(),
            other: None,
            commentary_id: None,
            distance_yards: None,
        };
        day.fire_banner_for(&event(EventKind::Yellow));
        assert!(day.banner.is_none());
        day.fire_banner_for(&event(EventKind::Goal));
        let banner = day.banner.as_ref().expect("goal banners");
        assert_eq!(banner.kind, MomentKind::Goal);
        assert_eq!(banner.minute, 23);
        day.fire_banner_for(&event(EventKind::Red));
        let banner = day.banner.as_ref().expect("red banners");
        assert_eq!(banner.kind, MomentKind::RedCard);
        assert_eq!(day.banner_seq, 2);
    }

    #[test]
    fn pills_tint_goals_and_cards() {
        use crate::theme::palette;
        assert_eq!(
            event_pill(EventKind::Goal),
            (palette::GOLD, egui::Color32::BLACK)
        );
        assert_eq!(
            event_pill(EventKind::Yellow),
            (palette::CARD_YELLOW, egui::Color32::BLACK)
        );
        assert_eq!(
            event_pill(EventKind::Red),
            (palette::CARD_RED, egui::Color32::WHITE)
        );
        assert_eq!(
            event_pill(EventKind::ShotSaved),
            (palette::WELL, palette::INK_DIM)
        );
    }

    #[test]
    fn strictness_bands_match_the_original() {
        assert_eq!(strictness_label_key(0), Some("referee-lenient"));
        assert_eq!(strictness_label_key(5), Some("referee-lenient"));
        assert_eq!(strictness_label_key(6), None);
        assert_eq!(strictness_label_key(14), None);
        assert_eq!(strictness_label_key(15), Some("referee-strict"));
        assert_eq!(strictness_label_key(19), Some("referee-strict"));
        assert_eq!(strictness_label_key(20), Some("referee-harsh"));
    }

    /// Live rows derive minutes, goals and cards from revealed
    /// incidents only.
    #[test]
    fn live_rows_follow_revealed_prefix() {
        use cm0102_core::MatchEvent;
        let ratings = vec![
            PlayerRating {
                player_id: 1,
                name: "Starter".to_string(),
                home: true,
                rating: 70,
                minutes: 90,
                on_minute: 0,
                goals: 1,
                yellow: true,
                red: false,
                shots: 2,
                on_target: 1,
                tackles: 0,
                passes: 30,
                headers: 0,
                interceptions: 0,
                offsides: 0,
            },
            PlayerRating {
                player_id: 2,
                name: "Sub".to_string(),
                home: true,
                rating: 68,
                minutes: 30,
                on_minute: 60,
                goals: 0,
                yellow: false,
                red: false,
                shots: 0,
                on_target: 0,
                tackles: 1,
                passes: 9,
                headers: 0,
                interceptions: 0,
                offsides: 0,
            },
        ];
        let events = vec![
            MatchEvent {
                minute: 10,
                home: true,
                kind: EventKind::Goal,
                player: "Starter".to_string(),
                other: None,
                commentary_id: None,
                distance_yards: None,
            },
            MatchEvent {
                minute: 30,
                home: true,
                kind: EventKind::Yellow,
                player: "Starter".to_string(),
                other: None,
                commentary_id: None,
                distance_yards: None,
            },
            MatchEvent {
                minute: 60,
                home: true,
                kind: EventKind::Sub,
                player: "Sub".to_string(),
                other: Some("Winger".to_string()),
                commentary_id: None,
                distance_yards: None,
            },
        ];
        let rows = live_player_rows(&ratings, &events[..2], 45);
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0],
            LiveRow {
                name: "Starter".to_string(),
                minutes: 45,
                goals: 1,
                yellow: true,
                red: false,
            }
        );
        let rows = live_player_rows(&ratings, &events, 70);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1].minutes, 10);
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
                commentary_id: None,
                distance_yards: None,
                },
                MatchEvent {
                    minute: 80,
                    home: false,
                    kind: EventKind::Goal,
                    player: "B".to_string(),
                    other: None,
                commentary_id: None,
                distance_yards: None,
                },
            ],
            stats: cm0102_core::MatchStats::default(),
            ratings: Vec::new(),
            shootout: None,
            injuries: Vec::new(),
            first_half_end: 46,
            timeline: Vec::new(),
            weather: cm0102_core::MatchWeather::default(),
            referee: None,
            attendance: None,
        });
        assert_eq!(day.live_score(), (0, 0));
        assert!(!day.is_full_time());
        day.revealed = 1;
        assert_eq!(day.live_score(), (1, 0));
        assert_eq!(day.live_minute(), 10);
        day.revealed = 2;
        assert_eq!(day.live_score(), (1, 1));
        assert!(day.is_full_time());
    }

    /// The interval is detected from the sim cap, not the event
    /// list: everything played and revealed, second half unreleased.
    #[test]
    fn interval_detection_uses_sim_cap() {
        use cm0102_core::{MatchContext, MatchPlayer, Mentality, Referee};
        fn squad(prefix: &str, base: u32) -> Vec<MatchPlayer> {
            (0..11)
                .map(|i| MatchPlayer {
                    id: base + i,
                    name: format!("{prefix}{i}"),
                    current_ability: 100,
                    attributes: vec![10u8; 54],
                    condition: 100,
                    morale: 0,
                })
                .collect()
        }
        let context = MatchContext {
            home_club_id: 1,
            away_club_id: 2,
            date: cm0102_core::GameDate::SEASON_START,
            seed: 21,
            home_avg_ca: 100.0,
            away_avg_ca: 100.0,
            home_players: squad("H", 0),
            away_players: squad("A", 100),
            home_subs: squad("HS", 200),
            away_subs: squad("AS", 300),
            home_mentality: Mentality::Normal,
            away_mentality: Mentality::Normal,
            home_men_behind_ball: false,
            away_men_behind_ball: false,
            extra_time_if_level: false,
            weather: cm0102_core::MatchWeather::default(),
            referee: Referee::default(),
        };
        let mut day = MatchDayState::pre_match(Fixture {
            date: cm0102_core::GameDate::SEASON_START,
            home_club_id: 1,
            away_club_id: 2,
            competition_id: 7,
            result: None,
            report: None,
        });
        let mut live = LiveMatch::new(context);
        live.play_until_incident(10_000);
        assert!(live.capped_for_interval());
        let report = live.snapshot();
        day.revealed = report.events.len();
        day.live = Some(live);
        day.report = Some(report);
        assert!(day.at_half_time());
        // Everything is revealed, so the raw full-time check also
        // holds: match_phase() resolves the tie by testing the
        // interval first.
        assert!(day.is_full_time());
    }
}
