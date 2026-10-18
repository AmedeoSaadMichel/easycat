// File: easycut-app/src/updates/ui.rs
//! Update notices and the minimum-build gate.
use super::{Status, Updates, BUILD, VERSION};
use crate::theme;
use eframe::egui::{self, RichText};

impl Updates {
    pub fn footer(&mut self, ui: &mut egui::Ui) {
        let detail = match self.status {
            Status::Unavailable => {
                "Could not check for updates. You can keep using easycat offline."
            }
            Status::Current => "You have the latest available build.",
            Status::Available(_) => "A newer build is available.",
            Status::Required(_) => "This build requires an update.",
            Status::Unchecked => "Check for updates.",
        };
        if ui
            .add_enabled(
                self.receiver.is_none(),
                egui::Button::new("Updates").small(),
            )
            .on_hover_text(format!("easycat {VERSION} · Build {BUILD}\n{detail}"))
            .clicked()
        {
            self.check(ui.ctx());
        }
    }

    pub fn notice(&mut self, ctx: &egui::Context, exporting: bool) {
        let (release, required) = match &self.status {
            Status::Available(release) if !self.dismissed => (release.clone(), false),
            Status::Required(release) if exporting => (release.clone(), true),
            _ => return,
        };
        egui::Window::new(if required {
            "Update required"
        } else {
            "Update available"
        })
        .id(egui::Id::new("update_notice"))
        .anchor(egui::Align2::LEFT_TOP, egui::vec2(20.0, 100.0))
        .collapsible(false)
        .resizable(false)
        .default_width(360.0)
        .max_width((ctx.screen_rect().width() * 0.5).clamp(220.0, 420.0))
        .frame(egui::Frame::window(&ctx.style()).fill(theme::SURFACE))
        .show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(if required {
                        "Update required after this export finishes.".to_owned()
                    } else {
                        format!("easycat {} is available.", release.version)
                    })
                    .color(theme::CREAM),
                );
                if ui.button("Download update").clicked() {
                    ctx.open_url(egui::OpenUrl::new_tab(release.url()));
                }
                if !required && ui.small_button("Later").clicked() {
                    self.dismissed = true;
                }
            });
        });
    }

    pub fn required_panel(&mut self, ctx: &egui::Context) {
        let Status::Required(release) = self.status.clone() else {
            return;
        };
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(theme::BG).inner_margin(28.0))
            .show(ctx, |ui| {
                ui.add_space(36.0);
                ui.vertical_centered(|ui| {
                    ui.heading(RichText::new("An update is required").color(theme::CREAM));
                    ui.label(format!(
                        "This is build {BUILD}. The minimum supported build is {}.",
                        release.minimum_build
                    ));
                    ui.label(format!("Install easycat {} to continue.", release.version));
                    ui.add_space(12.0);
                    if ui
                        .add_sized([220.0, 40.0], theme::primary("Download update"))
                        .clicked()
                    {
                        ctx.open_url(egui::OpenUrl::new_tab(release.url()));
                    }
                    #[cfg(target_os = "macos")]
                    ui.label("Homebrew: brew update && brew upgrade --cask easycat");
                    ui.add_space(12.0);
                    if ui
                        .add_enabled(
                            self.receiver.is_none(),
                            egui::Button::new(if self.receiver.is_some() {
                                "Checking…"
                            } else {
                                "Check again"
                            }),
                        )
                        .clicked()
                    {
                        self.check(ctx);
                    }
                    ui.label("If the check fails, you can keep using easycat offline.");
                    if ui.button("Quit easycat").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
            });
    }
}
