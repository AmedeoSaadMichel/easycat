// File: easycut-app/src/main.rs
#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]
mod app;
mod controls;
mod panels;
mod runtime;
mod theme;
mod timeline;
mod video;
mod window_chrome;
mod window_resize;
mod worker;

use std::path::PathBuf;

fn main() -> eframe::Result {
    runtime::configure_packaged_tools();
    let options = eframe::NativeOptions {
        viewport: window_chrome::viewport(),
        centered: true,
        persist_window: false,
        ..Default::default()
    };
    let initial = std::env::args_os().nth(1).map(PathBuf::from);
    eframe::run_native(
        "easycat",
        options,
        Box::new(|cc| {
            theme::install(&cc.egui_ctx);
            Ok(Box::new(app::App::new(initial, &cc.egui_ctx)))
        }),
    )
}
