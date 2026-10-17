// File: easycut-app/src/app.rs
//! Application state, worker messages and update loop.
mod bars;
mod center;
mod jobs;
mod playback;
#[cfg(test)]
mod preview;

use crate::controls::{self, Action};
use crate::panels::PanelAction;
use crate::video::{EstimateKey, Video};
use crate::worker::{self, Msg};
use easycut_core::{ffmpeg, presets};
use eframe::egui::{self, ColorImage, TextureOptions};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "mov", "m4v", "mkv", "webm", "avi", "wmv", "flv", "mts", "m2ts", "ts", "3gp", "mpg",
    "mpeg", "ogv",
];

struct Measure {
    id: u64,
    stop: Arc<AtomicBool>,
    done: usize,
    total: usize,
}

struct Play {
    id: u64,
    stop: Arc<AtomicBool>,
}

pub struct App {
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
    gen: u64,
    encoders: HashSet<String>,
    path_text: String,
    video: Option<Video>,
    loading: bool,
    error: Option<String>,
    exporting: bool,
    result: Option<(String, bool)>,
    pending_estimate: u64,
    pending_expects_hw: bool,
    pick_requested: bool,
    initial: Option<PathBuf>,
    play: Option<Play>,
    play_id: u64,
    muted: bool,
    volume: u8,
    progress: f32,
    export_cancel: Option<Arc<AtomicBool>>,
    measure: Option<Measure>,
    measure_id: u64,
    estimate_key: Option<EstimateKey>,
    estimate_deadline: Option<std::time::Instant>,
    measure_error: Option<String>,
    header: Option<egui::TextureHandle>,
    brand_art: Option<egui::TextureHandle>,
    decorations: Option<egui::TextureHandle>,
    footer_art: Option<egui::TextureHandle>,
    preview_fullscreen: bool,
    window_resize: crate::window_resize::WindowResize,
}

impl App {
    pub fn new(initial: Option<PathBuf>, ctx: &egui::Context) -> Self {
        let (tx, rx) = channel();
        App {
            tx,
            rx,
            gen: 0,
            encoders: ffmpeg::available_encoders(),
            path_text: String::new(),
            video: None,
            loading: false,
            error: None,
            exporting: false,
            result: None,
            pending_estimate: 0,
            pending_expects_hw: false,
            pick_requested: false,
            initial,
            play: None,
            play_id: 0,
            muted: false,
            volume: 100,
            progress: 0.0,
            export_cancel: None,
            measure: None,
            measure_id: 0,
            estimate_key: None,
            estimate_deadline: None,
            measure_error: None,
            header: crate::theme::load_header(ctx),
            brand_art: crate::theme::load_brand(ctx),
            decorations: crate::theme::load_decorations(ctx),
            footer_art: crate::theme::load_footer(ctx),
            preview_fullscreen: false,
            window_resize: Default::default(),
        }
    }

    fn load(&mut self, ctx: &egui::Context, path: PathBuf) {
        self.stop_play();
        self.stop_measure();
        self.estimate_key = None;
        self.estimate_deadline = None;
        self.measure_error = None;
        self.gen += 1;
        self.video = None;
        self.error = None;
        self.result = None;
        self.loading = true;
        self.path_text = path.display().to_string();
        worker::spawn_probe(path, self.gen, self.tx.clone(), ctx.clone());
    }

    fn texture(
        ctx: &egui::Context,
        name: &str,
        f: &easycut_core::frames::Frame,
    ) -> egui::TextureHandle {
        let img = ColorImage::from_rgb([f.width, f.height], &f.rgb);
        ctx.load_texture(name, img, TextureOptions::LINEAR)
    }

    /// Update the preview by reusing its texture.
    fn set_preview(ctx: &egui::Context, v: &mut Video, f: &easycut_core::frames::Frame) {
        match &mut v.preview {
            Some(t) => t.set(
                ColorImage::from_rgb([f.width, f.height], &f.rgb),
                TextureOptions::LINEAR,
            ),
            None => v.preview = Some(Self::texture(ctx, "preview", f)),
        }
    }

    fn poll(&mut self, ctx: &egui::Context) {
        let mut last_play = None;
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                Msg::Loaded(g, _) | Msg::Thumb(g, _, _) | Msg::Preview(g, _) if g != self.gen => {}
                Msg::Loaded(_, Err(e)) => {
                    self.loading = false;
                    self.error = Some(e);
                }
                Msg::Loaded(g, Ok(info)) => {
                    self.loading = false;
                    let path = PathBuf::from(&self.path_text);
                    let list = presets::builtin(&info, &self.encoders);
                    worker::spawn_thumbs(path.clone(), &info, g, self.tx.clone(), ctx.clone());
                    let ptx =
                        worker::spawn_preview(path.clone(), &info, g, self.tx.clone(), ctx.clone());
                    let has_h265 = self.encoders.contains("libx265");
                    let v = Video::new(path, info, list, ptx, has_h265);
                    v.request_preview(0.0);
                    self.video = Some(v);
                }
                Msg::Thumb(_, i, f) => {
                    if let Some(v) = &mut self.video {
                        v.thumbs[i] = Some(Self::texture(ctx, &format!("thumb{i}"), &f));
                    }
                }
                Msg::Preview(_, f) => {
                    // Single-frame previews would be stale during playback.
                    if self.play.is_none() {
                        if let Some(v) = &mut self.video {
                            Self::set_preview(ctx, v, &f);
                        }
                    }
                }
                Msg::PlayFrame(id, t, f) => {
                    if self.play.as_ref().is_some_and(|p| p.id == id) {
                        last_play = Some((t, f));
                    }
                }
                Msg::PlayEnded(id) => {
                    if self.play.as_ref().is_some_and(|p| p.id == id) {
                        self.play = None;
                    }
                }
                Msg::Progress(p) => self.progress = p as f32,
                Msg::Measured(id, idx, bytes) => {
                    if let (Some(m), Some(v)) = (&mut self.measure, &mut self.video) {
                        if m.id == id {
                            v.measured.insert(idx, bytes);
                            m.done += 1;
                        }
                    }
                }
                Msg::MeasureFailed(id, error) => {
                    if self.measure.as_ref().is_some_and(|m| m.id == id) {
                        self.measure_error = Some(error);
                    }
                }
                Msg::MeasureDone(id) => {
                    if self.measure.as_ref().is_some_and(|m| m.id == id) {
                        self.measure = None;
                    }
                }
                Msg::Done(r) => self.finish_export(r),
            }
        }
        if let (Some((t, f)), Some(v)) = (last_play, &mut self.video) {
            v.playhead = t;
            Self::set_preview(ctx, v, &f);
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        if let Some(p) = self.initial.take() {
            self.load(ctx, p);
        }
        if let Some(p) = ctx
            .input(|i| i.raw.dropped_files.first().and_then(|f| f.path.clone()))
            .filter(|_| !self.exporting)
        {
            self.load(ctx, p);
        }
        let open_shortcut = egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::O);
        if !self.exporting && ctx.input_mut(|i| i.consume_shortcut(&open_shortcut)) {
            self.pick_requested = true;
        }
        self.poll(ctx);
        if self.preview_fullscreen {
            let frame = self
                .video
                .as_ref()
                .map_or(1.0 / 30.0, |v| 1.0 / v.info.fps.max(1.0));
            let key = controls::fullscreen_shortcuts(ctx, frame);
            let clicked = self.fullscreen_preview(ctx);
            self.apply(
                ctx,
                if clicked != Action::None {
                    clicked
                } else {
                    key
                },
            );
            return;
        }
        self.top_bar(ctx);
        self.bottom_bar(ctx);
        if self
            .video
            .as_mut()
            .is_some_and(|v| v.invalidate_stale_measures())
        {
            self.stop_measure();
        }
        let panel_action = self.export_panel(ctx);
        match panel_action {
            PanelAction::None => {}
            PanelAction::MeasureAll => self.start_measure(ctx, None),
            PanelAction::MeasureOne(i) => self.start_measure(ctx, Some(i)),
        }
        let frame = self
            .video
            .as_ref()
            .map_or(1.0 / 30.0, |v| 1.0 / v.info.fps.max(1.0));
        let key = if self.exporting {
            Action::None
        } else {
            controls::shortcuts(ctx, frame)
        };
        let clicked = self.center(ctx);
        if std::mem::take(&mut self.pick_requested) {
            if let Some(p) = self.pick_video() {
                self.load(ctx, p);
            }
        }
        self.apply(
            ctx,
            if clicked != Action::None {
                clicked
            } else {
                key
            },
        );
        self.update_estimate(ctx);
        self.window_resize.show(ctx);
    }
}
