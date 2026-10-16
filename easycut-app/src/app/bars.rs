// File: easycut-app/src/app/bars.rs
//! Barra superiore (apertura file) e inferiore (salvataggio ed esportazione).
use super::{App, VIDEO_EXTENSIONS};
use crate::panels::fmt_bytes;
use eframe::egui::{self, RichText};
use std::path::PathBuf;
use std::sync::atomic::Ordering;

impl App {
    /// Dialogo di sistema per scegliere un video; parte dalla cartella del video aperto.
    pub(super) fn pick_video(&self) -> Option<PathBuf> {
        let mut dialog = rfd::FileDialog::new()
            .set_title("Scegli un video")
            .add_filter("Video", VIDEO_EXTENSIONS)
            .add_filter("Tutti i file", &["*"]);
        if let Some(dir) = self.video.as_ref().and_then(|v| v.path.parent()) {
            dialog = dialog.set_directory(dir);
        }
        dialog.pick_file()
    }

    pub(super) fn top_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button("Scegli video…")
                    .on_hover_text("Apri un video dal disco (⌘O)")
                    .clicked()
                {
                    self.pick_requested = true;
                }
                let r = ui.add(
                    egui::TextEdit::singleline(&mut self.path_text)
                        .desired_width(ui.available_width() - 60.0)
                        .hint_text("Trascina qui un video o scrivi il percorso"),
                );
                let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if (ui.button("Apri").clicked() || enter) && !self.path_text.trim().is_empty() {
                    self.load(ctx, PathBuf::from(self.path_text.trim()));
                }
            });
            if let Some(e) = &self.error {
                ui.colored_label(egui::Color32::LIGHT_RED, e);
            }
        });
    }

    pub(super) fn bottom_bar(&mut self, ctx: &egui::Context) {
        let mut export = false;
        let mut cancel = false;
        let (exporting, progress) = (self.exporting, self.progress);
        if let Some(v) = &mut self.video {
            egui::TopBottomPanel::bottom("bottom").show(ctx, |ui| {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("Salva come:");
                    ui.add(egui::TextEdit::singleline(&mut v.out_stem).desired_width(260.0));
                    ui.label(format!(".{}", v.out_ext()));
                    ui.separator();
                    ui.label(RichText::new(format!("~{}", fmt_bytes(v.estimate()))).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let btn = egui::Button::new(RichText::new("Esporta").strong());
                        if ui.add_enabled(!exporting, btn).clicked() {
                            export = true;
                        }
                        if exporting {
                            if ui.button("Annulla").clicked() {
                                cancel = true;
                            }
                            ui.add(
                                egui::ProgressBar::new(progress)
                                    .desired_width(240.0)
                                    .show_percentage(),
                            );
                        } else if let Some((r, warn)) = &self.result {
                            if *warn {
                                ui.colored_label(egui::Color32::from_rgb(230, 150, 60), r);
                            } else {
                                ui.label(r);
                            }
                        }
                    });
                });
                ui.add_space(4.0);
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
    }
}
