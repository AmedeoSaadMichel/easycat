// File: easycat-app/src/main.rs
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
mod updates;
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
    let argument = std::env::args_os().nth(1);
    #[cfg(debug_assertions)]
    let simulation = argument.as_ref().and_then(|a| match a.to_str() {
        Some("--simulate-update=optional") => Some(false),
        Some("--simulate-update=required") => Some(true),
        _ => None,
    });
    #[cfg(debug_assertions)]
    let initial = if simulation.is_some() {
        None
    } else {
        argument.map(PathBuf::from)
    };
    #[cfg(not(debug_assertions))]
    let initial = argument.map(PathBuf::from);
    #[cfg(debug_assertions)]
    let title = match simulation {
        Some(false) => "easycat · Optional update simulation",
        Some(true) => "easycat · Required update simulation",
        None => "easycat",
    };
    #[cfg(not(debug_assertions))]
    let title = "easycat";
    eframe::run_native(
        title,
        options,
        Box::new(|cc| {
            theme::install(&cc.egui_ctx);
            let mut app = app::App::new(initial, &cc.egui_ctx);
            #[cfg(debug_assertions)]
            if let Some(required) = simulation {
                app.simulate_update(required);
            } else {
                app.check_for_updates(&cc.egui_ctx);
            }
            #[cfg(not(debug_assertions))]
            app.check_for_updates(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
    )
}
