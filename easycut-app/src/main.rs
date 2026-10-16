// File: easycut-app/src/main.rs
mod app;
mod controls;
mod panels;
mod timeline;
mod video;
mod worker;

use eframe::egui;
use std::path::PathBuf;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1180.0, 760.0]),
        ..Default::default()
    };
    let initial = std::env::args_os().nth(1).map(PathBuf::from);
    eframe::run_native(
        "Easycut",
        options,
        Box::new(|_cc| Ok(Box::new(app::App::new(initial)))),
    )
}
