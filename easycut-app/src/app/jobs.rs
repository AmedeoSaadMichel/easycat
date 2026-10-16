// File: easycut-app/src/app/jobs.rs
//! Esportazione e misura delle dimensioni in background.
use super::{App, Measure};
use crate::panels::fmt_bytes;
use crate::worker::{self, MeasureArgs};
use easycut_core::ffmpeg::Backend;
use easycut_core::Job;
use eframe::egui;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

impl App {
    pub(super) fn start_export(&mut self, ctx: &egui::Context) {
        let Some(v) = &self.video else { return };
        let job = Job {
            input: v.path.clone(),
            output: v.output_path(),
            trim: v.trim(),
            mode: v.mode(),
        };
        self.pending_estimate = v.estimate();
        self.pending_expects_hw = v.expects_hardware();
        self.exporting = true;
        self.progress = 0.0;
        self.result = None;
        let cancel = Arc::new(AtomicBool::new(false));
        self.export_cancel = Some(cancel.clone());
        worker::spawn_export(job, v.info.clone(), cancel, self.tx.clone(), ctx.clone());
    }

    /// Esito dell'esportazione, con avviso se si è dovuto ripiegare sul software.
    pub(super) fn finish_export(&mut self, r: Result<(u64, Backend), String>) {
        self.exporting = false;
        self.export_cancel = None;
        self.result = Some(match r {
            Err(e) if e == "annullato" => ("Esportazione annullata".into(), false),
            Ok((b, backend)) => {
                let fell_back = self.pending_expects_hw && backend == Backend::Software;
                let note = if fell_back {
                    " · attenzione: hardware non disponibile, usato il software (più lento)"
                        .to_string()
                } else {
                    format!(" · {}", backend.label())
                };
                (
                    format!(
                        "Fatto: {} (stima {}){note}",
                        fmt_bytes(b),
                        fmt_bytes(self.pending_estimate)
                    ),
                    fell_back,
                )
            }
            Err(e) => (format!("Errore: {e}"), true),
        });
    }

    pub(super) fn stop_measure(&mut self) {
        if let Some(m) = self.measure.take() {
            m.stop.store(true, Ordering::Relaxed);
        }
    }

    /// Misura un preset (`Some`) oppure tutti quelli non ancora misurati (`None`).
    pub(super) fn start_measure(&mut self, ctx: &egui::Context, only: Option<usize>) {
        self.stop_measure();
        let Some(v) = &self.video else { return };
        let items: Vec<_> = match only {
            Some(i) => vec![(i, v.settings_for(i))],
            None => (0..v.presets.len())
                .filter(|i| !v.measured.contains_key(i))
                .map(|i| (i, v.settings_for(i)))
                .collect(),
        };
        if items.is_empty() {
            return;
        }
        self.measure_id += 1;
        let stop = Arc::new(AtomicBool::new(false));
        self.measure = Some(Measure {
            id: self.measure_id,
            stop: stop.clone(),
            done: 0,
            total: items.len(),
        });
        let args = MeasureArgs {
            path: v.path.clone(),
            info: v.info.clone(),
            trim: v.trim(),
            id: self.measure_id,
            items,
            stop,
        };
        worker::spawn_measure(args, self.tx.clone(), ctx.clone());
    }
}
