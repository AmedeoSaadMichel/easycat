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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
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
