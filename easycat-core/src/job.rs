// File: easycat-core/src/job.rs
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

    /// FFmpeg software encoder name.
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
    /// Constant quality (lower means better quality and larger files).
    Crf(u8),
    /// Requested output bytes (average bitrate).
    TargetBytes(u64),
}

/// Export speed versus file size trade-off.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Speed {
    Fast,
    #[default]
    Balanced,
    /// Slower, smaller files at comparable quality.
    Small,
}

impl Speed {
    pub fn label(self) -> &'static str {
        match self {
            Speed::Fast => "Fast",
            Speed::Balanced => "Balanced",
            Speed::Small => "Smaller file",
        }
    }
}

/// A rectangle of the picture to keep, in pixels of the displayed (already
/// rotated) video, measured from its top-left corner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Crop {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Crop {
    /// Largest rectangle of the given aspect ratio that fits, centred.
    pub fn centered(frame_w: u32, frame_h: u32, ratio_w: u32, ratio_h: u32) -> Crop {
        let (rw, rh) = (ratio_w.max(1) as u64, ratio_h.max(1) as u64);
        let (fw, fh) = (frame_w as u64, frame_h as u64);
        let (w, h) = if fw * rh <= fh * rw {
            (fw, fw * rh / rw)
        } else {
            (fh * rw / rh, fh)
        };
        let (w, h) = ((w as u32) & !1, (h as u32) & !1);
        Crop {
            x: ((frame_w - w) / 2) & !1,
            y: ((frame_h - h) / 2) & !1,
            width: w,
            height: h,
        }
    }

    /// Checks the rectangle against a frame. Encoders need even sizes.
    pub fn validate(&self, frame_w: u32, frame_h: u32) -> Result<(), String> {
        if self.width < 16 || self.height < 16 {
            return Err(format!("crop {}×{} is too small", self.width, self.height));
        }
        if [self.width, self.height, self.x, self.y]
            .iter()
            .any(|v| v & 1 != 0)
        {
            return Err("crop position and size must be even numbers".into());
        }
        if self.x + self.width > frame_w || self.y + self.height > frame_h {
            return Err(format!(
                "crop {}×{} at {},{} does not fit in {frame_w}×{frame_h}",
                self.width, self.height, self.x, self.y
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    /// Part of the picture to keep; `None` keeps all of it.
    pub crop: Option<Crop>,
    pub codec: Codec,
    pub container: Container,
    /// Target height; `None` preserves the original resolution.
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
    /// Stream copy: fast and lossless, with keyframe-only trim precision.
    Copy,
    /// Change container by copying all compatible tracks.
    Remux(Container),
    /// Copy tracks after explicitly excluding incompatible optional streams.
    RemuxSelected {
        container: Container,
        excluded_tracks: Vec<u32>,
    },
    Transcode(Settings),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Job {
    pub input: PathBuf,
    pub output: PathBuf,
    pub trim: Option<Trim>,
    pub mode: Mode,
}

fn track_supported(container: Container, kind: &str, codec: &str) -> bool {
    match (container, kind) {
        (Container::Mp4, "video") => {
            matches!(codec, "h264" | "hevc" | "av1" | "mpeg4")
        }
        (Container::Mp4, "audio") => {
            matches!(codec, "aac" | "mp3" | "ac3" | "eac3" | "alac")
        }
        (Container::Mp4, "subtitle") => codec == "mov_text",
        (Container::Mkv, "video") => matches!(
            codec,
            "h264" | "hevc" | "av1" | "vp8" | "vp9" | "mpeg4" | "mpeg2video"
        ),
        (Container::Mkv, "audio") => matches!(
            codec,
            "aac"
                | "mp3"
                | "ac3"
                | "eac3"
                | "flac"
                | "opus"
                | "vorbis"
                | "dts"
                | "pcm_s16le"
                | "pcm_s24le"
        ),
        (Container::Mkv, "subtitle") => matches!(
            codec,
            "subrip" | "ass" | "ssa" | "webvtt" | "hdmv_pgs_subtitle" | "dvd_subtitle"
        ),
        (Container::Webm, "video") => matches!(codec, "vp8" | "vp9" | "av1"),
        (Container::Webm, "audio") => matches!(codec, "opus" | "vorbis"),
        _ => false,
    }
}

#[derive(Debug, Clone)]
pub struct RemuxIssue {
    pub index: u32,
    pub kind: String,
    pub codec: String,
    pub removable: bool,
}

impl RemuxIssue {
    pub fn description(&self) -> String {
        format!(
            "Track #{}: {} ({})",
            self.index,
            if self.kind.is_empty() {
                "unknown"
            } else {
                &self.kind
            },
            if self.codec.is_empty() {
                "unknown codec"
            } else {
                &self.codec
            }
        )
    }
}

pub fn remux_issues(info: &crate::probe::VideoInfo, container: Container) -> Vec<RemuxIssue> {
    info.tracks
        .iter()
        .enumerate()
        .filter(|(_, (kind, codec))| !track_supported(container, kind, codec))
        .map(|(position, (kind, codec))| RemuxIssue {
            index: info
                .track_indices
                .get(position)
                .copied()
                .unwrap_or(position as u32),
            kind: kind.clone(),
            codec: codec.clone(),
            removable: matches!(kind.as_str(), "data" | "subtitle" | "attachment"),
        })
        .collect()
}

/// Validate every stream; optional exclusions never permit dropping video or audio.
pub fn remux_tracks(
    info: &crate::probe::VideoInfo,
    container: Container,
    excluded: &[u32],
) -> crate::Result<Vec<u32>> {
    if info.tracks.is_empty() {
        return Err(crate::Error::IncompatibleTracks(
            "track information unavailable".into(),
        ));
    }
    let indices: Vec<_> = info
        .tracks
        .iter()
        .enumerate()
        .map(|(position, _)| {
            info.track_indices
                .get(position)
                .copied()
                .unwrap_or(position as u32)
        })
        .collect();
    if indices
        .iter()
        .collect::<std::collections::HashSet<_>>()
        .len()
        != indices.len()
    {
        return Err(crate::Error::IncompatibleTracks(
            "Ambiguous stream indices; cannot safely select tracks.".into(),
        ));
    }
    let issues = remux_issues(info, container);
    for index in excluded {
        if !issues
            .iter()
            .any(|issue| issue.index == *index && issue.removable)
        {
            return Err(crate::Error::IncompatibleTracks(format!("Track #{index} cannot be excluded: only incompatible optional tracks may be omitted; video and audio are always retained.")));
        }
    }
    let blocked: Vec<_> = issues
        .iter()
        .filter(|issue| !excluded.contains(&issue.index))
        .map(|issue| issue.description())
        .collect();
    if !blocked.is_empty() {
        return Err(crate::Error::IncompatibleTracks(format!(
            "Cannot copy to .{}: {}",
            container.ext(),
            blocked.join("; ")
        )));
    }
    Ok(indices
        .into_iter()
        .filter(|index| !excluded.contains(index))
        .collect())
}

pub fn validate_remux(info: &crate::probe::VideoInfo, container: Container) -> crate::Result<()> {
    remux_tracks(info, container, &[]).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remux_requires_explicit_optional_exclusions_and_keeps_all_audio() {
        let info = crate::probe::parse_probe(r#"{"streams":[{"index":0,"codec_type":"video","codec_name":"h264"},{"index":2,"codec_type":"audio","codec_name":"aac"},{"index":4,"codec_type":"data","codec_name":"bin_data"}],"format":{"duration":"1"}}"#).unwrap();
        let issues = remux_issues(&info, Container::Mkv);
        assert_eq!(issues[0].index, 4);
        assert!(issues[0].removable);
        assert!(remux_tracks(&info, Container::Mkv, &[]).is_err());
        assert_eq!(
            remux_tracks(&info, Container::Mkv, &[4]).unwrap(),
            vec![0, 2]
        );
        assert!(remux_tracks(&info, Container::Mkv, &[2, 4]).is_err());
        assert!(remux_tracks(&info, Container::Mkv, &[99]).is_err());
        assert!(remux_tracks(&info, Container::Webm, &[4]).is_err());
    }

    #[test]
    fn remux_checks_secondary_audio_and_unknown_codecs() {
        let mut info = crate::probe::parse_probe(r#"{"streams":[{"codec_type":"video","codec_name":"vp9"},{"codec_type":"audio","codec_name":"opus"}],"format":{"duration":"1"}}"#).unwrap();
        assert!(validate_remux(&info, Container::Webm).is_ok());
        assert!(validate_remux(&info, Container::Mkv).is_ok());
        info.tracks.push(("audio".into(), "aac".into()));
        assert!(validate_remux(&info, Container::Webm).is_err());
        info.tracks = vec![("video".into(), "h264".into())];
        assert!(validate_remux(&info, Container::Mp4).is_ok());
        info.tracks.push(("audio".into(), String::new()));
        assert!(validate_remux(&info, Container::Mp4).is_err());
        info.tracks.clear();
        assert!(validate_remux(&info, Container::Mkv).is_err());
    }
}

#[cfg(test)]
mod crop_tests {
    use super::*;

    #[test]
    fn centered_crops_fit_and_are_even() {
        let square = Crop::centered(1920, 1080, 1, 1);
        assert_eq!(
            square,
            Crop {
                x: 420,
                y: 0,
                width: 1080,
                height: 1080
            }
        );
        let portrait = Crop::centered(1920, 1080, 9, 16);
        assert_eq!((portrait.width, portrait.height), (606, 1080));
        assert!(portrait.x.is_multiple_of(2) && portrait.validate(1920, 1080).is_ok());
        let same = Crop::centered(1920, 1080, 16, 9);
        assert_eq!(
            (same.width, same.height, same.x, same.y),
            (1920, 1080, 0, 0)
        );
    }

    #[test]
    fn validate_rejects_odd_tiny_and_outside_rectangles() {
        let frame = (1920, 1080);
        let ok = Crop {
            x: 10,
            y: 10,
            width: 100,
            height: 100,
        };
        assert!(ok.validate(frame.0, frame.1).is_ok());
        assert!(Crop { width: 101, ..ok }
            .validate(frame.0, frame.1)
            .is_err());
        assert!(Crop { x: 11, ..ok }.validate(frame.0, frame.1).is_err());
        assert!(Crop { width: 8, ..ok }.validate(frame.0, frame.1).is_err());
        assert!(Crop { x: 1900, ..ok }.validate(frame.0, frame.1).is_err());
    }
}
