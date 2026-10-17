// File: easycut-app/src/video.rs
//! Opened video state: trim, output format and size estimates.
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
    Remux(Container),
    Preset(usize),
    Target,
}

#[derive(Clone, PartialEq)]
pub struct EstimateKey {
    path: PathBuf,
    choice: Choice,
    start: u64,
    end: u64,
    hardware: bool,
    speed: Speed,
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
    pub remux_excluded: Vec<u32>,
    pub target_mb: f64,
    pub hardware: bool,
    pub speed: Speed,
    pub out_stem: String,
    /// Sample-based sizes by profile index.
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
            remux_excluded: Vec::new(),
            target_mb,
            hardware: cfg!(target_os = "macos"),
            speed: Speed::Balanced,
            out_stem: format!("{stem}_easycat"),
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

    /// Approximate source bytes within the selected range.
    pub fn original_slice_bytes(&self) -> u64 {
        if self.info.duration <= 0.0 {
            return 0;
        }
        (self.info.size_bytes as f64 * self.selected_secs() / self.info.duration) as u64
    }

    fn mode_for(&self, c: Choice) -> Mode {
        match c {
            Choice::Copy => Mode::Copy,
            Choice::Remux(c) => {
                if self.remux_excluded.is_empty() {
                    Mode::Remux(c)
                } else {
                    Mode::RemuxSelected {
                        container: c,
                        excluded_tracks: self.remux_excluded.clone(),
                    }
                }
            }
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

    /// Measurements are valid only for the current trim, hardware setting and speed.
    /// Return true when the signature changes, even with no completed samples.
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
        self.measured.clear();
        true
    }

    pub fn estimate_key(&self) -> EstimateKey {
        EstimateKey {
            path: self.path.clone(),
            choice: self.choice,
            start: self.start.to_bits(),
            end: self.end.to_bits(),
            hardware: self.hardware,
            speed: self.speed,
        }
    }

    pub fn estimate_is_measured(&self) -> bool {
        matches!(self.choice, Choice::Preset(i) if self.measured.contains_key(&i))
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
            Mode::Remux(c) | Mode::RemuxSelected { container: c, .. } => c.ext().into(),
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
            " · software only, slower"
        } else {
            ""
        };
        format!(".{} · {} · {res}{soft}", s.container.ext(), s.codec.label())
    }

    /// Whether export should use hardware, for fallback warnings.
    pub fn expects_hardware(&self) -> bool {
        match self.mode() {
            Mode::Transcode(s) => s.hardware && hardware_encoder(s.codec).is_some(),
            Mode::Copy | Mode::Remux(_) | Mode::RemuxSelected { .. } => false,
        }
    }

    pub fn export_blocker(&self) -> Option<String> {
        if let Choice::Remux(container) = self.choice {
            easycut_core::job::remux_tracks(&self.info, container, &self.remux_excluded)
                .err()
                .map(|error| error.to_string())
        } else {
            None
        }
    }

    pub fn set_choice(&mut self, c: Choice) {
        if self.choice != c {
            self.remux_excluded.clear();
        }
        self.choice = c;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remux_exclusions_require_consent_and_reset_with_destination() {
        let info = easycut_core::probe::parse_probe(r#"{"streams":[{"index":0,"codec_type":"video","codec_name":"h264"},{"index":1,"codec_type":"audio","codec_name":"aac"},{"index":2,"codec_type":"data","codec_name":"bin_data"}],"format":{"duration":"60"}}"#).unwrap();
        let (tx, _rx) = std::sync::mpsc::channel();
        let mut video = Video::new("camera.mov".into(), info, vec![], tx, true);
        video.set_choice(Choice::Remux(Container::Mkv));
        assert!(video.export_blocker().is_some());
        video.remux_excluded.push(2);
        assert!(video.export_blocker().is_none());
        assert!(matches!(video.mode(), Mode::RemuxSelected { .. }));
        video.set_choice(Choice::Remux(Container::Mp4));
        assert!(video.remux_excluded.is_empty());
        assert!(video.export_blocker().is_some());
        video.remux_excluded = vec![1, 2];
        assert!(video.export_blocker().is_some());
    }

    #[test]
    fn invalidate_pending_and_completed_estimates_when_settings_change() {
        let info = easycut_core::probe::parse_probe(r#"{"streams":[{"codec_type":"video","codec_name":"h264","width":1920,"height":1080}],"format":{"duration":"60"}}"#).unwrap();
        let (tx, _rx) = std::sync::mpsc::channel();
        let mut video = Video::new("input.mp4".into(), info, vec![], tx, true);
        assert!(video.invalidate_stale_measures());
        assert!(!video.invalidate_stale_measures());
        // An unfinished measurement must be invalidated even before it yields any bytes.
        video.end = 30.0;
        assert!(video.invalidate_stale_measures());
        video.measured.insert(0, 42);
        video.hardware = !video.hardware;
        assert!(video.invalidate_stale_measures());
        assert!(video.measured.is_empty());
        let key = video.estimate_key();
        video.choice = Choice::Target;
        assert!(key != video.estimate_key());
        video.speed = Speed::Fast;
        assert!(video.invalidate_stale_measures());
    }
}
