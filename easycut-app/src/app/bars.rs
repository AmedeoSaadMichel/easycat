// File: easycut-app/src/app/bars.rs
//! Cosmic header, export panel and job status.
use super::{App, VIDEO_EXTENSIONS};
use crate::{
    panels::{self, PanelAction},
    theme,
};
use eframe::egui::{self, RichText};
use std::path::PathBuf;
use std::sync::atomic::Ordering;

impl App {
    pub(super) fn pick_video(&self) -> Option<PathBuf> {
        let mut dialog = rfd::FileDialog::new()
            .set_title("easycat · Open video")
            .add_filter("Video", VIDEO_EXTENSIONS)
            .add_filter("All files", &["*"]);
        if let Some(dir) = self.video.as_ref().and_then(|v| v.path.parent()) {
            dialog = dialog.set_directory(dir);
        }
        dialog.pick_file()
    }

    pub(super) fn top_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top")
            .exact_height(88.0)
            .frame(egui::Frame::none().fill(theme::BG))
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                if let Some(texture) = &self.brand_art {
                    let source =
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 0.34));
                    ui.painter().image(
                        texture.id(),
                        rect,
                        theme::cover_uv(source, texture.size_vec2(), rect.size()),
                        egui::Color32::WHITE,
                    );
                }
                theme::stars(ui.painter(), rect);
                crate::window_chrome::drag_region(ui, rect);
                let mut content = ui.new_child(
                    egui::UiBuilder::new().max_rect(rect.shrink2(egui::vec2(18.0, 6.0))),
                );
                content.horizontal(|ui| {
                    if cfg!(target_os = "macos") {
                        ui.add_space(76.0);
                    }
                    let (logo, response) =
                        ui.allocate_exact_size(egui::vec2(292.0, 76.0), egui::Sense::hover());
                    if let Some(texture) = &self.brand_art {
                        let source = egui::Rect::from_min_max(
                            egui::pos2(0.025, 0.51),
                            egui::pos2(0.56, 0.91),
                        );
                        let brand = egui::Rect::from_min_size(logo.min, egui::vec2(292.0, 60.0));
                        ui.painter().image(
                            texture.id(),
                            theme::video_rect(brand, source.size() * texture.size_vec2()),
                            source,
                            egui::Color32::WHITE,
                        );
                        let tagline = logo.min + egui::vec2(100.0, 69.0);
                        ui.painter().rect_filled(
                            egui::Rect::from_min_size(
                                tagline - egui::vec2(4.0, 9.0),
                                egui::vec2(196.0, 18.0),
                            ),
                            3.0,
                            theme::BG,
                        );
                        ui.painter().text(
                            tagline,
                            egui::Align2::LEFT_CENTER,
                            "Cut. Transform. Meow.",
                            egui::FontId::proportional(14.0),
                            theme::MUTED,
                        );
                    }
                    response.on_hover_text("easycat · Cut. Transform. Meow.");
                });
                let open_rect = egui::Rect::from_min_size(
                    egui::pos2(rect.right() - 163.0, rect.bottom() - 44.0),
                    egui::vec2(145.0, 38.0),
                );
                let open = ui.add_enabled_ui(!self.exporting && !self.updates.required(), |ui| {
                    ui.put(
                        open_rect,
                        egui::Button::new("Open video")
                            .fill(theme::BG)
                            .stroke(egui::Stroke::new(1.0_f32, theme::LILAC)),
                    )
                    .on_hover_text("Open from disk · ⌘O / Ctrl+O")
                    .clicked()
                });
                if open.inner {
                    self.pick_requested = true;
                }
                crate::window_chrome::controls(ui, rect);
            });
    }

    pub(super) fn bottom_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status")
            .exact_height(64.0)
            .frame(
                egui::Frame::none()
                    .fill(theme::BG)
                    .inner_margin(egui::Margin::symmetric(20.0, 8.0)),
            )
            .show(ctx, |ui| {
                if let Some(texture) = &self.footer_art {
                    let panel = ui.max_rect().expand2(egui::vec2(20.0, 8.0));
                    // Repeat at natural proportions rather than stretching the terrain.
                    let source = egui::Rect::from_min_max(
                        egui::pos2(0.0, 280.0 / 724.0),
                        egui::pos2(1.0, 534.0 / 724.0),
                    );
                    let height = 46.0;
                    let width = height * (source.width() * texture.size_vec2().x)
                        / (source.height() * texture.size_vec2().y);
                    let mut x = panel.left();
                    while x < panel.right() {
                        let rect = egui::Rect::from_min_size(
                            egui::pos2(x, panel.bottom() - height),
                            egui::vec2(width, height),
                        );
                        ui.painter().with_clip_rect(panel).image(
                            texture.id(),
                            rect,
                            source,
                            egui::Color32::WHITE,
                        );
                        x += width;
                    }
                }
                ui.horizontal(|ui| {
                    let status_width = (ui.available_width() - 180.0).max(0.0);
                    ui.allocate_ui_with_layout(
                        egui::vec2(status_width, 28.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.horizontal_wrapped(|ui| {
                                self.updates.footer(ui);
                                ui.label(
                                    RichText::new("Local. Private. No uploads.")
                                        .small()
                                        .color(theme::MUTED),
                                );
                                if let Some(error) = &self.error {
                                    ui.colored_label(theme::CORAL, error);
                                }
                                if let Some((result, warning)) = &self.result {
                                    ui.separator();
                                    ui.label(RichText::new(result).small().color(if *warning {
                                        theme::CORAL
                                    } else {
                                        theme::MINT
                                    }));
                                }
                            });
                        },
                    );
                    let support = ui
                        .add_sized(
                            [172.0, 28.0],
                            egui::Button::new(RichText::new("Buy me a coffee").color(theme::BG))
                                .fill(theme::YELLOW),
                        )
                        .on_hover_text("Support easycat · Opens buymeacoffee.com in your browser");
                    if support.clicked() {
                        ctx.open_url(egui::OpenUrl::new_tab("https://buymeacoffee.com/asmichel"));
                    }
                });
            });
    }

    pub(super) fn export_panel(&mut self, ctx: &egui::Context) -> PanelAction {
        let mut action = PanelAction::None;
        let mut export = false;
        let mut cancel = false;
        let measuring = self.measure.as_ref().map(|m| (m.done, m.total));
        let exporting = self.exporting;
        let tiny = ctx.screen_rect().height() < 600.0;
        let width = (ctx.screen_rect().width() * 0.29).clamp(280.0, 380.0);
        if let Some(v) = &mut self.video {
            egui::SidePanel::right("formats")
                .exact_width(width)
                .frame(
                    egui::Frame::none()
                        .fill(theme::PANEL)
                        .inner_margin(if tiny { 10.0 } else { 18.0 })
                        .stroke(egui::Stroke::new(1.0_f32, theme::LINE)),
                )
                .show(ctx, |ui| {
                    // Reserve the export action before laying out the settings.
                    egui::TopBottomPanel::bottom("export_action")
                        .show_separator_line(false)
                        .frame(egui::Frame::none().inner_margin(egui::Margin {
                            top: if tiny { 4.0 } else { 12.0 },
                            ..Default::default()
                        }))
                        .show_inside(ui, |ui| {
                            if tiny {
                                ui.spacing_mut().item_spacing.y = 2.0;
                            }
                            ui.separator();
                            let estimate_label = if tiny && measuring.is_some() {
                                "Refining estimate…"
                            } else if matches!(v.choice, crate::video::Choice::Target) {
                                "Requested size"
                            } else if v.estimate_is_measured() {
                                "Sample-based estimate"
                            } else {
                                "Rough estimate"
                            };
                            ui.horizontal(|ui| {
                                let response = ui
                                    .add(
                                        egui::Label::new(
                                            RichText::new(estimate_label).color(theme::MUTED),
                                        )
                                        .sense(egui::Sense::click()),
                                    )
                                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                                    .on_hover_text(
                                        "Estimate options · Click to refine or measure profiles",
                                    );
                                let popup_id = ui.make_persistent_id("estimate_options");
                                if response.clicked() {
                                    ui.memory_mut(|memory| memory.toggle_popup(popup_id));
                                }
                                egui::popup_below_widget(
                                    ui,
                                    popup_id,
                                    &response,
                                    egui::PopupCloseBehavior::CloseOnClick,
                                    |ui| {
                                        ui.add_enabled_ui(!exporting, |ui| {
                                            let requested = panels::estimate_options(
                                                ui,
                                                v,
                                                measuring.is_some(),
                                            );
                                            if !matches!(requested, PanelAction::None) {
                                                action = requested;
                                            }
                                        });
                                    },
                                );
                                if let Some(texture) = &self.decorations {
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            let (rect, _) = ui.allocate_exact_size(
                                                if tiny {
                                                    egui::vec2(50.0, 18.0)
                                                } else {
                                                    egui::vec2(68.0, 60.0)
                                                },
                                                egui::Sense::hover(),
                                            );
                                            let source = egui::Rect::from_min_max(
                                                egui::pos2(480.0 / 1536.0, 510.0 / 1024.0),
                                                egui::pos2(1020.0 / 1536.0, 1000.0 / 1024.0),
                                            );
                                            ui.painter().image(
                                                texture.id(),
                                                theme::video_rect(
                                                    if tiny {
                                                        egui::Rect::from_min_size(
                                                            rect.min,
                                                            egui::vec2(50.0, 44.0),
                                                        )
                                                    } else {
                                                        rect
                                                    },
                                                    source.size() * texture.size_vec2(),
                                                ),
                                                source,
                                                egui::Color32::WHITE,
                                            );
                                        },
                                    );
                                }
                            });
                            if measuring.is_some() && !tiny {
                                ui.label(
                                    RichText::new("Refining estimate…")
                                        .small()
                                        .color(theme::MINT),
                                );
                            }
                            if let Some(error) = &self.measure_error {
                                ui.label(
                                    RichText::new("Could not refine estimate")
                                        .small()
                                        .color(theme::CORAL),
                                )
                                .on_hover_text(error);
                            }
                            panels::size_line(ui, v.estimate(), v.original_slice_bytes());
                            ui.add_enabled_ui(!exporting, |ui| {
                                ui.horizontal(|ui| {
                                    ui.add(
                                        egui::TextEdit::singleline(&mut v.out_stem)
                                            .desired_width((ui.available_width() - 55.0).max(80.0)),
                                    );
                                    ui.label(format!(".{}", v.out_ext()));
                                });
                            });
                            if !tiny {
                                ui.label(
                                    RichText::new("Save next to the original")
                                        .small()
                                        .color(theme::MUTED),
                                )
                                .on_hover_text(v.output_path().display().to_string());
                            }
                            if exporting {
                                ui.add(
                                    egui::ProgressBar::new(self.progress)
                                        .fill(theme::MINT)
                                        .show_percentage(),
                                );
                                if ui
                                    .add_sized(
                                        [ui.available_width(), 38.0],
                                        egui::Button::new("Cancel export"),
                                    )
                                    .clicked()
                                {
                                    cancel = true;
                                }
                            } else {
                                let blocker = v.export_blocker();
                                if let Some(reason) = &blocker {
                                    ui.label(
                                        RichText::new(
                                            "Resolve incompatible tracks above to export.",
                                        )
                                        .small()
                                        .color(theme::CORAL),
                                    )
                                    .on_hover_text(reason);
                                }
                                let response = ui
                                    .add_enabled_ui(blocker.is_none(), |ui| {
                                        ui.add_sized(
                                            [ui.available_width(), if tiny { 38.0 } else { 56.0 }],
                                            theme::primary("Export video"),
                                        )
                                    })
                                    .inner;
                                theme::paw(
                                    ui.painter(),
                                    response.rect.center() - egui::vec2(99.0, 0.0),
                                    theme::BG,
                                );
                                if response.clicked() {
                                    export = true;
                                }
                            }
                        });
                    ui.label(
                        RichText::new("What would you like to do?")
                            .strong()
                            .size(if tiny { 15.0 } else { 20.0 }),
                    );
                    ui.scope(|ui| {
                        if tiny {
                            ui.spacing_mut().item_spacing.y = 2.0;
                            ui.spacing_mut().interact_size.y = 22.0;
                            ui.spacing_mut().button_padding.y = 2.0;
                        }
                        ui.add_enabled_ui(!exporting, |ui| {
                            panels::show(ui, v);
                        });
                    });
                });
        }
        if export {
            self.start_export(ctx);
        }
        if cancel {
            if let Some(c) = &self.export_cancel {
                c.store(true, Ordering::Relaxed);
            }
        }
        action
    }
}
