// File: easycut-app/src/video.rs
//! Stato del video aperto: taglio, scelta del formato e stime.
use crate::worker::THUMBS;
use easycut_core::estimate::estimate_bytes;
use easycut_core::ffmpeg::hardware_encoder;
use easycut_core::presets::Preset;
use easycut_core::{Codec, Container, Mode, RateControl, Settings, Speed, Trim, VideoInfo};
use eframe::egui::TextureHandle;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::Sender;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    Copy,
    Preset(usize),
    Target,
}

pub struct Video {
    pub path: PathBuf,
    pub info: VideoInfo,
    pub thumbs: Vec<Option<TextureHandle>>,
    pub preview: Option<TextureHandle>,
    pub preview_tx: Sender<f64>,
    pub start: f64,
    pub end: f64,
    pub playhead: f64,
    pub presets: Vec<Preset>,
    pub choice: Choice,
    pub target_mb: f64,
    pub hardware: bool,
    pub speed: Speed,
    pub out_stem: String,
    /// Dimensioni misurate sui campioni, per indice di preset.
    pub measured: HashMap<usize, u64>,
    measured_sig: (u64, u64, bool, Speed),
    target_codec: Codec,
}

impl Video {
    pub fn new(
        path: PathBuf,
        info: VideoInfo,
        presets: Vec<Preset>,
        preview_tx: Sender<f64>,
        has_h265: bool,
    ) -> Self {
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "video".into());
        let target_mb = ((info.size_bytes as f64 / 1e6) * 0.3).max(1.0).round();
        Video {
            path,
            end: info.duration,
            thumbs: vec![None; THUMBS],
            preview: None,
            preview_tx,
            start: 0.0,
            playhead: 0.0,
            presets,
            choice: Choice::Copy,
            target_mb,
            hardware: true,
            speed: Speed::Balanced,
            out_stem: format!("{stem}_easycut"),
            measured: HashMap::new(),
            measured_sig: (0, 0, true, Speed::Balanced),
            target_codec: if has_h265 { Codec::H265 } else { Codec::H264 },
            info,
        }
    }

    pub fn request_preview(&self, t: f64) {
        let _ = self.preview_tx.send(t);
    }

    pub fn trim(&self) -> Option<Trim> {
        let full = self.start <= 0.01 && self.end >= self.info.duration - 0.01;
        (!full).then_some(Trim {
            start: self.start,
            end: self.end,
        })
    }

    pub fn selected_secs(&self) -> f64 {
        self.trim().map_or(self.info.duration, |t| t.duration())
    }

    /// Peso del solo tratto selezionato nel file originale.
    pub fn original_slice_bytes(&self) -> u64 {
        if self.info.duration <= 0.0 {
            return 0;
        }
        (self.info.size_bytes as f64 * self.selected_secs() / self.info.duration) as u64
    }

    fn mode_for(&self, c: Choice) -> Mode {
        match c {
            Choice::Copy => Mode::Copy,
            Choice::Preset(i) => Mode::Transcode(self.settings_for(i)),
            Choice::Target => Mode::Transcode(Settings {
                codec: self.target_codec,
                container: Container::Mp4,
                height: None,
                rate: RateControl::TargetBytes((self.target_mb * 1e6) as u64),
                hardware: self.hardware,
                speed: self.speed,
            }),
        }
    }

    pub fn mode(&self) -> Mode {
        self.mode_for(self.choice)
    }

    /// Le misure valgono solo per questo taglio, scelta hardware e velocità.
    /// Ritorna true se erano da scartare.
    pub fn invalidate_stale_measures(&mut self) -> bool {
        let sig = (
            self.start.to_bits(),
            self.end.to_bits(),
            self.hardware,
            self.speed,
        );
        if sig == self.measured_sig {
            return false;
        }
        self.measured_sig = sig;
        let had = !self.measured.is_empty();
        self.measured.clear();
        had
    }

    pub fn settings_for(&self, i: usize) -> Settings {
        Settings {
            hardware: self.hardware,
            speed: self.speed,
            ..self.presets[i].settings
        }
    }

    pub fn estimate_for(&self, c: Choice) -> u64 {
        if let Choice::Preset(i) = c {
            if let Some(b) = self.measured.get(&i) {
                return *b;
            }
        }
        estimate_bytes(&self.info, &self.mode_for(c), self.trim())
    }

    pub fn estimate(&self) -> u64 {
        self.estimate_for(self.choice)
    }

    pub fn copy_ext(&self) -> String {
        self.path
            .extension()
            .map(|e| e.to_string_lossy().into_owned())
            .unwrap_or_else(|| "mp4".into())
    }

    pub fn out_ext(&self) -> String {
        match self.mode() {
            Mode::Copy => self.copy_ext(),
            Mode::Transcode(s) => s.container.ext().into(),
        }
    }

    pub fn output_path(&self) -> PathBuf {
        let dir = self.path.parent().map(PathBuf::from).unwrap_or_default();
        dir.join(format!("{}.{}", self.out_stem, self.out_ext()))
    }

    pub fn preset_label(&self, i: usize) -> String {
        let s = &self.presets[i].settings;
        let res = s
            .height
            .map_or_else(|| format!("{}p", self.info.height), |h| format!("{h}p"));
        let soft = if self.hardware && hardware_encoder(s.codec).is_none() {
            " · solo software, più lento"
        } else {
            ""
        };
        format!(".{} · {} · {res}{soft}", s.container.ext(), s.codec.label())
    }

    /// True se l'esportazione dovrebbe usare l'hardware (per avvisare se ripiega).
    pub fn expects_hardware(&self) -> bool {
        match self.mode() {
            Mode::Transcode(s) => s.hardware && hardware_encoder(s.codec).is_some(),
            Mode::Copy => false,
        }
    }

    pub fn set_choice(&mut self, c: Choice) {
        self.choice = c;
    }
}
