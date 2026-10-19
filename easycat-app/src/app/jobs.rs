// File: easycat-app/src/app/jobs.rs
//! Background export and sample-based size estimation.
use super::{App, Measure};
use crate::panels::fmt_bytes;
use crate::worker::{self, MeasureArgs};
use easycat_core::ffmpeg::Backend;
use easycat_core::Job;
use eframe::egui;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

impl App {
    pub(super) fn start_export(&mut self, ctx: &egui::Context) {
        if self.exporting || self.video.is_none() || self.updates.required() {
            return;
        }
        if let Some(reason) = self.video.as_ref().and_then(|v| v.export_blocker()) {
            self.error = Some(reason);
            return;
        }
        self.stop_play();
        self.stop_measure();
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

    /// Export result, with a warning when software fallback was required.
    pub(super) fn finish_export(&mut self, r: Result<(u64, Backend), String>) {
        self.exporting = false;
        self.export_cancel = None;
        self.result = Some(match r {
            Err(e) if e == "cancelled" => ("Export cancelled".into(), false),
            Ok((b, backend)) => {
                let fell_back = self.pending_expects_hw && backend == Backend::Software;
                let note = if fell_back {
                    " · warning: hardware unavailable; using software (slower)".to_string()
                } else {
                    format!(" · {}", backend.label())
                };
                (
                    format!(
                        "Done: {} (estimate {}){note}",
                        fmt_bytes(b),
                        fmt_bytes(self.pending_estimate)
                    ),
                    fell_back,
                )
            }
            Err(e) => (format!("Error: {e}"), true),
        });
    }

    /// Refine only the selected profile, after the user has stopped changing settings.
    pub(super) fn update_estimate(&mut self, ctx: &egui::Context) {
        let key = self.video.as_ref().map(|v| v.estimate_key());
        if key != self.estimate_key {
            self.stop_measure();
            if let Some(v) = &mut self.video {
                v.invalidate_stale_measures();
            }
            self.estimate_key = key;
            self.measure_error = None;
            self.estimate_deadline =
                Some(std::time::Instant::now() + std::time::Duration::from_millis(700));
        }
        let Some(deadline) = self.estimate_deadline else {
            return;
        };
        if self.exporting {
            self.estimate_deadline = None;
            return;
        }
        if self.play.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
            return;
        }
        let now = std::time::Instant::now();
        if now < deadline {
            ctx.request_repaint_after(deadline - now);
            return;
        }
        self.estimate_deadline = None;
        if self.measure.is_some() {
            return;
        }
        if let Some(v) = &self.video {
            if let crate::video::Choice::Preset(i) = v.choice {
                if !v.measured.contains_key(&i) {
                    self.start_measure(ctx, Some(i));
                }
            }
        }
    }

    pub(super) fn stop_measure(&mut self) {
        if let Some(m) = self.measure.take() {
            m.stop.store(true, Ordering::Relaxed);
        }
    }

    /// Measure one profile (`Some`) or all profiles not yet measured (`None`).
    pub(super) fn start_measure(&mut self, ctx: &egui::Context, only: Option<usize>) {
        if self.exporting {
            return;
        }
        self.stop_measure();
        self.measure_error = None;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automatic_estimation_is_debounced_cancels_stale_work_and_skips_cached_profiles() {
        let ctx = egui::Context::default();
        let mut app = App::new(None, &ctx);
        let info = easycat_core::probe::parse_probe(r#"{"streams":[{"codec_type":"video","codec_name":"h264","width":1920,"height":1080}],"format":{"duration":"60"}}"#).unwrap();
        let presets = easycat_core::presets::builtin(
            &info,
            &std::collections::HashSet::from(["libx264".into()]),
        );
        let (tx, _rx) = std::sync::mpsc::channel();
        let mut video = crate::video::Video::new("input.mp4".into(), info, presets, tx, false);
        video.choice = crate::video::Choice::Preset(0);
        app.video = Some(video);
        app.update_estimate(&ctx);
        assert!(app.estimate_deadline.is_some());
        assert!(app.measure.is_none());
        let stop = Arc::new(AtomicBool::new(false));
        app.measure = Some(Measure {
            id: 1,
            stop: stop.clone(),
            done: 0,
            total: 1,
        });
        app.video.as_mut().unwrap().end = 30.0;
        app.update_estimate(&ctx);
        assert!(stop.load(Ordering::Relaxed));
        assert!(app.measure.is_none());
        app.video.as_mut().unwrap().measured.insert(0, 1000);
        app.estimate_deadline = Some(std::time::Instant::now());
        app.update_estimate(&ctx);
        assert!(app.measure.is_none());
        assert!(app.estimate_deadline.is_none());
        app.video.as_mut().unwrap().measured.clear();
        app.estimate_deadline = Some(std::time::Instant::now());
        app.update_estimate(&ctx);
        assert_eq!(app.measure.as_ref().map(|m| m.total), Some(1));
        app.stop_measure();
        app.estimate_deadline = Some(std::time::Instant::now());
        app.exporting = true;
        app.update_estimate(&ctx);
        assert!(app.measure.is_none());
        assert!(app.estimate_deadline.is_none());
    }
}
