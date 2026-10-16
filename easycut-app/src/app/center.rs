// File: easycut-app/src/app/center.rs
//! Pannello centrale: anteprima, timeline e controlli di riproduzione.
use super::App;
use crate::controls::{self, Action};
use crate::panels::fmt_bytes;
use crate::timeline;
use eframe::egui::{self, load::SizedTexture, RichText};

impl App {
    pub(super) fn center(&mut self, ctx: &egui::Context) -> Action {
        let (playing, muted) = (self.play.is_some(), self.muted);
        let mut action = Action::None;
        egui::CentralPanel::default().show(ctx, |ui| {
            let Some(v) = &mut self.video else {
                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() * 0.35);
                    if self.loading {
                        ui.spinner();
                        ui.label(RichText::new("Caricamento…").size(22.0).weak());
                        return;
                    }
                    ui.label(RichText::new("Trascina un video qui").size(22.0).weak());
                    ui.add_space(6.0);
                    ui.label(RichText::new("oppure").weak());
                    ui.add_space(6.0);
                    let btn = egui::Button::new(RichText::new("Scegli video…").size(18.0))
                        .min_size(egui::vec2(200.0, 40.0));
                    if ui.add(btn).clicked() {
                        self.pick_requested = true;
                    }
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new("MP4, MOV, MKV, WebM, AVI e altri")
                            .small()
                            .weak(),
                    );
                });
                return;
            };
            let info = v.info.clone();
            ui.label(
                RichText::new(format!(
                    "{}  ·  {}×{} {}  ·  {}  ·  {}",
                    v.path
                        .file_name()
                        .map(|n| n.to_string_lossy())
                        .unwrap_or_default(),
                    info.width,
                    info.height,
                    info.codec,
                    timeline::fmt_time(info.duration),
                    fmt_bytes(info.size_bytes),
                ))
                .weak(),
            );
            let tl_h = 72.0 + 110.0;
            let avail = egui::vec2(
                ui.available_width(),
                (ui.available_height() - tl_h).max(120.0),
            );
            ui.allocate_ui(avail, |ui| {
                ui.centered_and_justified(|ui| {
                    if let Some(tex) = &v.preview {
                        let ts = tex.size_vec2();
                        let scale = (avail.x / ts.x).min(avail.y / ts.y).min(2.0);
                        ui.add(egui::Image::new(SizedTexture::new(tex.id(), ts * scale)));
                    } else {
                        ui.spinner();
                    }
                });
            });
            ui.add_space(8.0);
            let aspect = info.width as f32 / info.height.max(1) as f32;
            let (s0, e0) = (v.start, v.end);
            let changed = timeline::show(
                ui,
                &v.thumbs,
                aspect,
                info.duration,
                timeline::Range {
                    start: &mut v.start,
                    end: &mut v.end,
                    playhead: &mut v.playhead,
                },
            );
            if let Some(t) = changed {
                // Maniglie: mette in pausa. Click sulla traccia: continua da lì.
                let handles_moved = v.start != s0 || v.end != e0;
                action = Action::Seek(t, !handles_moved);
            }
            ui.add_space(6.0);
            let frame = 1.0 / info.fps.max(1.0);
            let pressed = controls::show(ui, playing, muted, frame);
            if pressed != Action::None {
                action = pressed;
            }
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(format!("Inizio {}", timeline::fmt_time(v.start)));
                ui.label(format!("Fine {}", timeline::fmt_time(v.end)));
                ui.label(
                    RichText::new(format!("Durata {}", timeline::fmt_time(v.selected_secs())))
                        .strong(),
                );
                ui.label(
                    RichText::new(format!("Posizione {}", timeline::fmt_time(v.playhead))).weak(),
                );
                if ui.small_button("Inizio qui (I)").clicked() {
                    action = Action::SetStart;
                }
                if ui.small_button("Fine qui (O)").clicked() {
                    action = Action::SetEnd;
                }
                if ui.small_button("Reimposta").clicked() {
                    v.start = 0.0;
                    v.end = info.duration;
                }
            });
        });
        action
    }
}
