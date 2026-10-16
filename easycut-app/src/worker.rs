// File: easycut-app/src/worker.rs
//! Lavoro in background: nessuna chiamata a FFmpeg sul thread UI.
use easycut_core::ffmpeg::Backend;
use easycut_core::frames::{self, Frame};
use easycut_core::playback::{AudioPlayer, FrameStream};
use easycut_core::{ffmpeg, probe, Job, Settings, Trim, VideoInfo};
use eframe::egui::Context;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const THUMBS: usize = 20;
const THUMB_H: u32 = 72;
const PREVIEW_H: u32 = 540;

pub enum Msg {
    Loaded(u64, Result<VideoInfo, String>),
    Thumb(u64, usize, Frame),
    Preview(u64, Frame),
    /// Fotogramma di riproduzione: id, istante nel video, immagine.
    PlayFrame(u64, f64, Frame),
    PlayEnded(u64),
    /// Avanzamento dell'esportazione, 0..1.
    Progress(f64),
    /// Misura di un preset: id misurazione, indice preset, byte.
    Measured(u64, usize, u64),
    MeasureDone(u64),
    /// Esito dell'esportazione: byte scritti e backend usato.
    Done(Result<(u64, Backend), String>),
}

fn send(tx: &Sender<Msg>, ctx: &Context, msg: Msg) {
    let _ = tx.send(msg);
    ctx.request_repaint();
}

pub fn spawn_probe(path: PathBuf, gen: u64, tx: Sender<Msg>, ctx: Context) {
    std::thread::spawn(move || {
        let r = probe(&path).map_err(|e| e.to_string());
        send(&tx, &ctx, Msg::Loaded(gen, r));
    });
}

pub fn spawn_thumbs(path: PathBuf, info: &VideoInfo, gen: u64, tx: Sender<Msg>, ctx: Context) {
    let (w, h) = frames::thumb_size(info.width, info.height, THUMB_H);
    let duration = info.duration;
    std::thread::spawn(move || {
        for i in 0..THUMBS {
            let t = (i as f64 + 0.5) / THUMBS as f64 * duration;
            if let Ok(f) = frames::frame_at(&path, t, w, h) {
                send(&tx, &ctx, Msg::Thumb(gen, i, f));
            }
        }
    });
}

/// Thread dell'anteprima: riceve istanti e risponde solo all'ultimo richiesto.
/// Termina quando il `Sender` restituito viene scartato.
pub fn spawn_preview(
    path: PathBuf,
    info: &VideoInfo,
    gen: u64,
    tx: Sender<Msg>,
    ctx: Context,
) -> Sender<f64> {
    let (w, h) = frames::thumb_size(info.width, info.height, PREVIEW_H);
    let (req_tx, req_rx) = channel::<f64>();
    std::thread::spawn(move || {
        while let Ok(mut t) = req_rx.recv() {
            while let Ok(newer) = req_rx.try_recv() {
                t = newer;
            }
            if let Ok(f) = frames::frame_at(&path, t, w, h) {
                send(&tx, &ctx, Msg::Preview(gen, f));
            }
        }
    });
    req_tx
}

pub fn spawn_export(
    job: Job,
    info: VideoInfo,
    cancel: Arc<AtomicBool>,
    tx: Sender<Msg>,
    ctx: Context,
) {
    std::thread::spawn(move || {
        let mut last = -1.0;
        let mut on_progress = |p: f64| {
            if p - last >= 0.005 || p >= 1.0 {
                last = p;
                send(&tx, &ctx, Msg::Progress(p));
            }
        };
        let r = ffmpeg::run_with_progress(&job, &info, &mut on_progress, &cancel)
            .map_err(|e| e.to_string())
            .and_then(|backend| {
                std::fs::metadata(&job.output)
                    .map(|m| (m.len(), backend))
                    .map_err(|e| e.to_string())
            });
        send(&tx, &ctx, Msg::Done(r));
    });
}

pub struct MeasureArgs {
    pub path: PathBuf,
    pub info: VideoInfo,
    pub trim: Option<Trim>,
    pub id: u64,
    /// Preset da misurare: (indice, impostazioni).
    pub items: Vec<(usize, Settings)>,
    pub stop: Arc<AtomicBool>,
}

/// Misura in sequenza la dimensione reale di ciascun preset.
pub fn spawn_measure(a: MeasureArgs, tx: Sender<Msg>, ctx: Context) {
    std::thread::spawn(move || {
        for (idx, settings) in &a.items {
            if a.stop.load(Ordering::Relaxed) {
                break;
            }
            let r = ffmpeg::measure_size(&a.path, &a.info, settings, a.trim, &a.stop);
            if let Ok(bytes) = r {
                send(&tx, &ctx, Msg::Measured(a.id, *idx, bytes));
            }
        }
        send(&tx, &ctx, Msg::MeasureDone(a.id));
    });
}

const PLAY_H: u32 = 360;

pub struct PlayArgs {
    pub path: PathBuf,
    pub id: u64,
    pub start: f64,
    pub duration: f64,
    pub fps: f64,
    pub size: (usize, usize),
    pub muted: bool,
    pub stop: Arc<AtomicBool>,
}

impl PlayArgs {
    pub fn new(
        path: PathBuf,
        info: &VideoInfo,
        id: u64,
        start: f64,
        duration: f64,
        muted: bool,
        stop: Arc<AtomicBool>,
    ) -> Self {
        let fps = if info.fps > 1.0 {
            info.fps.min(30.0)
        } else {
            25.0
        };
        let size = frames::thumb_size(info.width, info.height, PLAY_H);
        PlayArgs {
            path,
            id,
            start,
            duration,
            fps,
            size,
            muted,
            stop,
        }
    }
}

/// Riproduce `duration` secondi da `start` al ritmo reale. Si ferma con `stop`.
pub fn spawn_play(a: PlayArgs, tx: Sender<Msg>, ctx: Context) {
    std::thread::spawn(move || {
        let (w, h) = a.size;
        if let Ok(mut stream) = FrameStream::open(&a.path, a.start, a.duration, a.fps, w, h) {
            let mut audio = None;
            let mut began: Option<Instant> = None;
            let mut i = 0u64;
            while !a.stop.load(Ordering::Relaxed) {
                let Some(frame) = stream.next_frame() else {
                    break;
                };
                // L'orologio parte al primo fotogramma, così video e audio restano allineati.
                if began.is_none() {
                    began = Some(Instant::now());
                    if !a.muted {
                        audio = AudioPlayer::spawn(&a.path, a.start, a.duration);
                    }
                }
                let due =
                    began.unwrap_or_else(Instant::now) + Duration::from_secs_f64(i as f64 / a.fps);
                if let Some(wait) = due.checked_duration_since(Instant::now()) {
                    std::thread::sleep(wait);
                }
                if a.stop.load(Ordering::Relaxed) {
                    break;
                }
                send(
                    &tx,
                    &ctx,
                    Msg::PlayFrame(a.id, a.start + i as f64 / a.fps, frame),
                );
                i += 1;
            }
            drop(audio);
        }
        send(&tx, &ctx, Msg::PlayEnded(a.id));
    });
}
