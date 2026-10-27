// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! Application theme: one dark broadcast-style palette, type scale
//! and shared components. Green is the club accent, used sparingly;
//! surfaces stay neutral charcoal so live content (scores, goals,
//! cards) carries the colour.

use eframe::egui;
use egui::{Color32, TextStyle};

/// Named palette. Every colour in the UI comes from here.
pub mod palette {
    use super::egui::Color32;

    /// App background: near-black with a green hint.
    pub const BG: Color32 = Color32::from_rgb(0x14, 0x18, 0x15);
    /// Raised surfaces (panels, cards, scoreboard).
    pub const SURFACE: Color32 = Color32::from_rgb(0x1f, 0x26, 0x21);
    /// Sunken rows and wells.
    pub const WELL: Color32 = Color32::from_rgb(0x18, 0x1e, 0x19);
    /// Hairlines and card borders.
    pub const LINE: Color32 = Color32::from_rgb(0x2e, 0x38, 0x2f);
    /// Primary text.
    pub const INK: Color32 = Color32::from_rgb(0xed, 0xf0, 0xeb);
    /// Secondary text (meta lines, timestamps).
    pub const INK_DIM: Color32 = Color32::from_rgb(0xa3, 0xac, 0xa0);
    /// Faint text (hints, placeholders).
    pub const INK_FAINT: Color32 = Color32::from_rgb(0x6e, 0x77, 0x6c);
    /// Pitch-green accent: live dot, primary actions, highlights.
    pub const ACCENT: Color32 = Color32::from_rgb(0x3d, 0xd1, 0x74);
    /// Darker accent for pressed/hover fills.
    pub const ACCENT_DIM: Color32 = Color32::from_rgb(0x1e, 0x56, 0x31);
    /// Broadcast gold: goals and the moments that matter.
    pub const GOLD: Color32 = Color32::from_rgb(0xf5, 0xc5, 0x18);
    /// Yellow cards.
    pub const CARD_YELLOW: Color32 = Color32::from_rgb(0xff, 0xd6, 0x0a);
    /// Red cards and danger.
    pub const CARD_RED: Color32 = Color32::from_rgb(0xff, 0x5a, 0x52);
}

/// Apply the theme: dark visuals, type scale and spacing.
pub fn apply(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.dark_mode = true;
    visuals.panel_fill = palette::BG;
    visuals.window_fill = palette::SURFACE;
    visuals.faint_bg_color = palette::WELL;
    visuals.extreme_bg_color = Color32::from_rgb(0x0e, 0x12, 0x0e);
    visuals.code_bg_color = palette::WELL;
    visuals.warn_fg_color = palette::GOLD;
    visuals.error_fg_color = palette::CARD_RED;
    visuals.widgets.noninteractive.bg_fill = palette::SURFACE;
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, palette::LINE);
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, palette::INK);
    visuals.widgets.inactive.bg_fill = palette::SURFACE;
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, palette::LINE);
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, palette::INK);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(0x27, 0x31, 0x29);
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, palette::ACCENT_DIM);
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, palette::INK);
    visuals.widgets.active.bg_fill = palette::ACCENT_DIM;
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, palette::ACCENT);
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, palette::INK);
    visuals.selection.bg_fill = palette::ACCENT_DIM;
    visuals.selection.stroke = egui::Stroke::new(1.0, palette::ACCENT);
    visuals.hyperlink_color = palette::ACCENT;
    visuals.widgets.open.bg_fill = palette::WELL;
    visuals.widgets.open.bg_stroke = egui::Stroke::new(1.0, palette::LINE);
    visuals.widgets.open.fg_stroke = egui::Stroke::new(1.0, palette::INK);
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (TextStyle::Small, egui::FontId::proportional(12.0)),
        (TextStyle::Body, egui::FontId::proportional(14.5)),
        (TextStyle::Button, egui::FontId::proportional(14.5)),
        (TextStyle::Heading, egui::FontId::proportional(22.0)),
        (TextStyle::Name("score".into()), egui::FontId::proportional(34.0)),
        (
            TextStyle::Name("score-sub".into()),
            egui::FontId::proportional(15.0),
        ),
        (TextStyle::Monospace, egui::FontId::monospace(13.0)),
    ]
    .into_iter()
    .collect();
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(14.0, 8.0);
    style.interaction.selectable_labels = false;
    ctx.set_style(style);
}

/// Card frame for raised content blocks.
pub fn card() -> egui::Frame {
    egui::Frame::new()
        .fill(palette::SURFACE)
        .stroke(egui::Stroke::new(1.0, palette::LINE))
        .corner_radius(10.0)
        .inner_margin(12.0)
}

/// Primary action button (kick-off, confirm).
pub fn primary_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    let button = egui::Button::new(
        egui::RichText::new(label)
            .size(16.0)
            .color(Color32::from_rgb(0x0c, 0x14, 0x0d)),
    )
    .fill(palette::ACCENT)
    .corner_radius(8.0)
    .min_size(egui::vec2(200.0, 40.0));
    ui.add(button)
}

/// Small pill badge (minutes, tags). `fill` tints the chip.
pub fn pill(ui: &mut egui::Ui, text: &str, fill: Color32, ink: Color32) {
    egui::Frame::new()
        .fill(fill)
        .corner_radius(6.0)
        .inner_margin(egui::Margin {
            left: 7,
            right: 7,
            top: 2,
            bottom: 2,
        })
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).size(12.5).color(ink));
        });
}

/// Section heading with a thin rule underneath.
pub fn section(ui: &mut egui::Ui, title: &str) {
    ui.label(
        egui::RichText::new(title)
            .size(13.0)
            .color(palette::INK_DIM),
    );
    let rect = ui.available_rect_before_wrap();
    ui.painter().hline(
        rect.left()..=rect.right(),
        rect.top(),
        egui::Stroke::new(1.0, palette::LINE),
    );
    ui.add_space(4.0);
}

/// Dim meta line (venue, date, officials).
pub fn meta(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text).size(13.0).color(palette::INK_DIM));
}
