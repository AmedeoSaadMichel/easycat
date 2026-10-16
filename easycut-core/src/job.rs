// File: easycut-core/src/job.rs
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Codec {
    H264,
    H265,
    Vp9,
    Av1,
}

impl Codec {
    pub fn label(self) -> &'static str {
        match self {
            Codec::H264 => "H.264",
            Codec::H265 => "H.265",
            Codec::Vp9 => "VP9",
            Codec::Av1 => "AV1",
        }
    }

    /// Nome dell'encoder software di FFmpeg.
    pub fn software_encoder(self) -> &'static str {
        match self {
            Codec::H264 => "libx264",
            Codec::H265 => "libx265",
            Codec::Vp9 => "libvpx-vp9",
            Codec::Av1 => "libsvtav1",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Container {
    Mp4,
    Mkv,
    Webm,
}

impl Container {
    pub fn ext(self) -> &'static str {
        match self {
            Container::Mp4 => "mp4",
            Container::Mkv => "mkv",
            Container::Webm => "webm",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RateControl {
    /// Qualità costante (più basso = migliore e più grande).
    Crf(u8),
    /// Dimensione finale desiderata in byte (bitrate medio).
    TargetBytes(u64),
}

/// Compromesso tra velocità di esportazione e dimensione del file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Speed {
    Fast,
    #[default]
    Balanced,
    /// Più lento, file più piccolo a parità di qualità.
    Small,
}

impl Speed {
    pub fn label(self) -> &'static str {
        match self {
            Speed::Fast => "Veloce",
            Speed::Balanced => "Bilanciato",
            Speed::Small => "Più piccolo",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    pub codec: Codec,
    pub container: Container,
    /// Altezza di destinazione; `None` = risoluzione originale.
    pub height: Option<u32>,
    pub rate: RateControl,
    pub hardware: bool,
    pub speed: Speed,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Trim {
    pub start: f64,
    pub end: f64,
}

impl Trim {
    pub fn duration(&self) -> f64 {
        self.end - self.start
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    /// Nessuna ricodifica: istantaneo, taglio preciso solo al keyframe.
    Copy,
    Transcode(Settings),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Job {
    pub input: PathBuf,
    pub output: PathBuf,
    pub trim: Option<Trim>,
    pub mode: Mode,
}
