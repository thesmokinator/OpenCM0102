// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! OpenCM0102 application entry point.
//!
//! Starts the native egui application at 1280x800. All heavy lifting lives
//! in the `cm0102-*` crates; this binary only owns windows and widgets.

mod app;

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

/// The main window viewport: 1280x800 as per the classic layout brief.
fn egui_viewport() -> eframe::egui::ViewportBuilder {
    eframe::egui::ViewportBuilder::default()
        .with_inner_size([1280.0, 800.0])
        .with_title("OpenCM0102")
}
