// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! OpenCM0102 application entry point.
//!
//! Starts the native egui application (1024x720, minimum 960x640).
//! All heavy lifting lives in the `cm0102-*` crates; this binary
//! only owns windows and widgets.

mod app;
mod i18n;
mod theme;

use app::OpenCmApp;

/// Application entry point: opens the main window and runs the event loop.
fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui_viewport(),
        ..Default::default()
    };
    eframe::run_native(
        "OpenCM0102",
        options,
        Box::new(|creation_context| Ok(Box::new(OpenCmApp::new(creation_context)))),
    )
}

/// The main window viewport: roomy default with a minimum that
/// keeps the header, content and footer usable.
fn egui_viewport() -> eframe::egui::ViewportBuilder {
    eframe::egui::ViewportBuilder::default()
        .with_inner_size([1024.0, 720.0])
        .with_min_inner_size([960.0, 640.0])
        .with_title("OpenCM0102")
}
