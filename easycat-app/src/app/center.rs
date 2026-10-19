// File: easycat-app/src/app/center.rs
//! Large preview, transport controls and coral timeline selection.
use super::App;
use crate::{
    controls::{self, Action},
    panels::fmt_bytes,
    theme, timeline,
};
use eframe::egui::{self, RichText};

impl App {
    pub(super) fn fullscreen_preview(&mut self, ctx: &egui::Context) -> Action {
        let mut action = Action::None;
        let (playing, muted) = (self.play.is_some(), self.muted);
        egui::TopBottomPanel::bottom("fullscreen_controls")
            .frame(egui::Frame::none().fill(theme::BG).inner_margin(12.0))
            .show(ctx, |ui| {
                if let Some(v) = &self.video {
                    let requested = controls::show(
                        ui,
                        playing,
                        muted,
                        &mut self.volume,
                        1.0 / v.info.fps.max(1.0),
                    );
                    if requested != Action::None {
                        action = requested;
                    }
                    ui.horizontal(|ui| {
                        ui.label(format!(
                            "{} / {}",
                            timeline::fmt_time(v.playhead),
                            timeline::fmt_time(v.info.duration)
                        ));
                        if ui.button("Exit fullscreen · Esc").clicked() {
                            action = Action::ToggleFullscreen;
                        }
                    });
                }
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(theme::BG))
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                if let Some(v) = &self.video {
                    if v.info.height > v.info.width {
                        if let Some(art) = &self.header {
                            theme::video_background(ui.painter(), art, rect);
                        }
                    }
                    if let Some(texture) = &v.preview {
                        let target = theme::video_rect(rect, texture.size_vec2());
                        ui.painter().image(
                            texture.id(),
                            target,
                            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }
                }
            });
        action
    }

    pub(super) fn center(&mut self, ctx: &egui::Context) -> Action {
        let (playing, muted) = (self.play.is_some(), self.muted);
        let mut action = Action::None;
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(theme::BG).inner_margin(20.0))
            .show(ctx, |ui| {
                let Some(v) = &mut self.video else {
                    ui.vertical_centered(|ui| {
                        ui.add_space((ui.available_height() * 0.12).max(12.0));
                        let (cat, _) =
                            ui.allocate_exact_size(egui::vec2(132.0, 132.0), egui::Sense::hover());
                        theme::cat(ui.painter(), cat.shrink(8.0));
                        ui.add_space(18.0);
                        if self.loading {
                            ui.spinner();
                            ui.heading("Preparing your video…");
                            return;
                        }
                        ui.label(
                            RichText::new("A little cut. Another universe.")
                                .size(27.0)
                                .strong(),
                        );
                        ui.label(
                            RichText::new("Drop your video here to get started")
                                .color(theme::MUTED),
                        );
                        ui.add_space(12.0);
                        if ui.add(theme::primary("Open video")).clicked() {
                            self.pick_requested = true;
                        }
                        ui.label(
                            RichText::new("MP4, MOV, MKV, WebM, AVI and more")
                                .small()
                                .color(theme::MUTED),
                        );
                        ui.add_space(12.0);
                        egui::CollapsingHeader::new("Open from a path").show(ui, |ui| {
                            let response = ui.add(
                                egui::TextEdit::singleline(&mut self.path_text)
                                    .desired_width(350.0)
                                    .hint_text("Video path"),
                            );
                            let enter = response.lost_focus()
                                && ui.input(|i| i.key_pressed(egui::Key::Enter));
                            if (enter || ui.button("Open path").clicked())
                                && !self.path_text.trim().is_empty()
                            {
                                self.initial = Some(self.path_text.trim().into());
                            }
                        });
                    });
                    return;
                };
                let info = &v.info;
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(
                            v.path
                                .file_name()
                                .map(|n| n.to_string_lossy())
                                .unwrap_or_default(),
                        )
                        .strong(),
                    )
                    .on_hover_text(v.path.display().to_string());
                    ui.label(
                        RichText::new(format!(
                            "{}×{} · {} · {} · {}",
                            info.width,
                            info.height,
                            info.codec,
                            timeline::fmt_time(info.duration),
                            fmt_bytes(info.size_bytes)
                        ))
                        .small()
                        .color(theme::MUTED),
                    );
                });
                let duration = info.duration;
                let fps = info.fps;
                let aspect = info.width as f32 / info.height.max(1) as f32;
                let reserve = if ctx.screen_rect().height() < 600.0 {
                    214.0
                } else {
                    250.0
                };
                let preview_h = (ui.available_height() - reserve).max(40.0);
                ui.scope(|ui| {
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(14, 10, 20))
                        .rounding(12.0)
                        .stroke(egui::Stroke::new(1.0_f32, theme::LINE))
                        .show(ui, |ui| {
                            let avail = egui::vec2(ui.available_width(), preview_h);
                            let (canvas, _) = ui.allocate_exact_size(avail, egui::Sense::hover());
                            let painter = ui.painter_at(canvas.shrink(1.0));
                            if aspect < 1.0 {
                                if let Some(art) = &self.header {
                                    theme::video_background(&painter, art, canvas);
                                }
                            }
                            if let Some(tex) = &v.preview {
                                let target = theme::video_rect(canvas, tex.size_vec2());
                                painter.image(
                                    tex.id(),
                                    target,
                                    egui::Rect::from_min_max(
                                        egui::Pos2::ZERO,
                                        egui::pos2(1.0, 1.0),
                                    ),
                                    egui::Color32::WHITE,
                                );
                            } else {
                                ui.put(
                                    egui::Rect::from_center_size(
                                        canvas.center(),
                                        egui::vec2(24.0, 24.0),
                                    ),
                                    egui::Spinner::new(),
                                );
                            }
                            painter.rect_stroke(
                                canvas.shrink(1.0),
                                10.0,
                                egui::Stroke::new(1.0_f32, theme::LINE),
                            );
                        });
                    ui.add_enabled_ui(!self.exporting, |ui| {
                        let pressed = controls::show(
                            ui,
                            playing,
                            muted,
                            &mut self.volume,
                            1.0 / fps.max(1.0),
                        );
                        if pressed != Action::None {
                            action = pressed;
                        }
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!(
                                    "{} / {}",
                                    timeline::fmt_time(v.playhead),
                                    timeline::fmt_time(duration)
                                ))
                                .small()
                                .color(theme::MUTED),
                            );
                        });
                        ui.add_space(6.0);
                        let (s0, e0) = (v.start, v.end);
                        if let Some(t) = timeline::show(
                            ui,
                            &v.thumbs,
                            aspect,
                            duration,
                            timeline::Range {
                                start: &mut v.start,
                                end: &mut v.end,
                                playhead: &mut v.playhead,
                            },
                        ) {
                            action = Action::Seek(t, v.start == s0 && v.end == e0);
                        }
                        ui.add_space(6.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new("START").small().color(theme::MUTED));
                            let old_start = v.start;
                            if ui
                                .add(
                                    egui::DragValue::new(&mut v.start)
                                        .range(0.0..=(v.end - 0.1).max(0.0))
                                        .speed(1.0 / fps.max(1.0))
                                        .custom_formatter(|value, _| timeline::fmt_time(value))
                                        .custom_parser(timeline::parse_time),
                                )
                                .on_hover_text("Drag to adjust, or enter minutes:seconds")
                                .changed()
                                && v.start != old_start
                            {
                                action = Action::Seek(v.start, false);
                            }
                            ui.label(RichText::new("END").small().color(theme::MUTED));
                            if ui
                                .add(
                                    egui::DragValue::new(&mut v.end)
                                        .range((v.start + 0.1).min(duration)..=duration)
                                        .speed(1.0 / fps.max(1.0))
                                        .custom_formatter(|value, _| timeline::fmt_time(value))
                                        .custom_parser(timeline::parse_time),
                                )
                                .on_hover_text("Drag to adjust, or enter minutes:seconds")
                                .changed()
                            {
                                action = Action::Seek(v.end, false);
                            }
                            ui.label(
                                RichText::new(format!(
                                    "Duration: {}",
                                    timeline::fmt_time(v.selected_secs())
                                ))
                                .strong(),
                            );
                        });
                    });
                });
            });
        action
    }
}
