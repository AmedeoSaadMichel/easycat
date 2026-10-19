// File: easycat-app/src/app/playback.rs
//! Playback and seeking.
use super::{App, Play};
use crate::controls::Action;
use crate::worker::{self, PlayArgs};
use eframe::egui;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

impl App {
    pub(super) fn stop_play(&mut self) {
        if let Some(p) = self.play.take() {
            p.stop.store(true, Ordering::Relaxed);
        }
    }

    /// Start at the current position, or selection start if out of range.
    pub(super) fn start_play(&mut self, ctx: &egui::Context) {
        if self.exporting {
            return;
        }
        self.stop_play();
        let Some(v) = &mut self.video else { return };
        let t0 = if v.playhead >= v.start && v.playhead < v.end - 0.1 {
            v.playhead
        } else {
            v.start
        };
        v.playhead = t0;
        self.play_id += 1;
        let stop = Arc::new(AtomicBool::new(false));
        let args = PlayArgs::new(
            v.path.clone(),
            &v.info,
            self.play_id,
            t0,
            v.end - t0,
            if self.muted { 0 } else { self.volume },
            stop.clone(),
        );
        worker::spawn_play(args, self.tx.clone(), ctx.clone());
        self.play = Some(Play {
            id: self.play_id,
            stop,
        });
    }

    pub(super) fn seek(&mut self, ctx: &egui::Context, t: f64, resume: bool) {
        let was_playing = self.play.is_some();
        self.stop_play();
        if let Some(v) = &mut self.video {
            v.playhead = t.clamp(0.0, v.info.duration);
            v.request_preview(v.playhead);
        }
        if resume
            && was_playing
            && self
                .video
                .as_ref()
                .is_some_and(|v| v.playhead >= v.start && v.playhead < v.end - 0.1)
        {
            self.start_play(ctx);
        }
    }

    pub(super) fn apply(&mut self, ctx: &egui::Context, action: Action) {
        if action == Action::ToggleFullscreen {
            if self.preview_fullscreen || (self.video.is_some() && !self.exporting) {
                self.preview_fullscreen = !self.preview_fullscreen;
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.preview_fullscreen));
            }
            return;
        }
        let Some(v) = &mut self.video else { return };
        let (start, end, pos) = (v.start, v.end, v.playhead);
        match action {
            Action::None | Action::ToggleFullscreen => {}
            Action::TogglePlay => {
                if self.play.is_some() {
                    self.stop_play();
                } else {
                    self.start_play(ctx);
                }
            }
            Action::Seek(t, resume) => self.seek(ctx, t, resume),
            Action::Skip(dt) => self.seek(ctx, (pos + dt).clamp(start, end), true),
            Action::SetVolume(volume) => {
                self.volume = volume;
                self.muted = false;
                if self.play.is_some() {
                    self.start_play(ctx);
                }
            }
            Action::Step(dt) => self.seek(ctx, pos + dt, false),
            Action::ToStart => self.seek(ctx, start, false),
            Action::ToEnd => self.seek(ctx, end, false),
            Action::ToggleMute => {
                self.muted = !self.muted;
                if self.play.is_some() {
                    self.start_play(ctx);
                }
            }
            Action::SetStart => v.start = pos.min(end - 0.1).max(0.0),
            Action::SetEnd => v.end = pos.max(start + 0.1).min(v.info.duration),
        }
    }
}
