// File: easycut-core/src/ffmpeg/args.rs
//! Pure FFmpeg argument construction, testable without invoking FFmpeg.
use crate::estimate::{audio_kbps, output_dims};
use crate::job::{Codec, Container, Job, Mode, RateControl, Settings, Speed};
use crate::probe::VideoInfo;
use crate::{Error, Result};

/// Video processing backend, from fastest to most compatible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// Hardware decoding, rotation, scaling and encoding (VideoToolbox).
    Chip,
    /// Hardware encoding only; CPU decoding and filters.
    HardwareEncode,
    /// CPU processing only.
    Software,
}

impl Backend {
    pub fn label(self) -> &'static str {
        match self {
            Backend::Chip => "full hardware pipeline",
            Backend::HardwareEncode => "hardware encoder",
            Backend::Software => "software",
        }
    }
}

pub fn hardware_encoder(codec: Codec) -> Option<&'static str> {
    match (codec, cfg!(target_os = "macos")) {
        (Codec::H264, true) => Some("h264_videotoolbox"),
        (Codec::H265, true) => Some("hevc_videotoolbox"),
        _ => None,
    }
}

/// Backend attempt order; advance on failure.
/// Prefer full hardware for sources above 8 bit (previous measurement:
/// 1.6 s versus 2.8 s for 4K HEVC 10-bit; no measured benefit for 8-bit).
pub fn backends(mode: &Mode, info: &VideoInfo) -> Vec<Backend> {
    match mode {
        Mode::Transcode(s) if s.hardware && hardware_encoder(s.codec).is_some() => {
            if info.bit_depth > 8 {
                vec![Backend::Chip, Backend::HardwareEncode, Backend::Software]
            } else {
                vec![Backend::HardwareEncode, Backend::Software]
            }
        }
        _ => vec![Backend::Software],
    }
}

fn even(x: u32) -> u32 {
    (x.max(2) + 1) & !1
}

/// Input options, before `-i`.
pub(crate) fn input_args(backend: Backend) -> Vec<String> {
    if backend == Backend::Chip {
        [
            "-hwaccel",
            "videotoolbox",
            "-hwaccel_output_format",
            "videotoolbox_vld",
        ]
        .map(String::from)
        .to_vec()
    } else {
        Vec::new()
    }
}

/// Video filter, if needed. Hardware rotation must be applied explicitly:
/// FFmpeg does not autorotate hardware frames (PSNR-tested for all three rotations).
pub(crate) fn filter(s: &Settings, info: &VideoInfo, backend: Backend) -> Option<String> {
    if backend == Backend::Chip {
        let mut parts = Vec::new();
        match info.rotation {
            90 => parts.push("transpose_vt=dir=cclock".to_string()),
            270 => parts.push("transpose_vt=dir=clock".to_string()),
            180 => parts.push("transpose_vt=dir=reversal".to_string()),
            _ => {}
        }
        if s.height.is_some_and(|h| h < info.height) {
            let (w, h) = output_dims(info, s);
            parts.push(format!("scale_vt=w={}:h={}", even(w), even(h)));
        }
        return (!parts.is_empty()).then(|| parts.join(","));
    }
    s.height.map(|h| format!("scale=-2:{h}"))
}

pub(crate) fn video_args(s: &Settings, backend: Backend) -> Vec<String> {
    let hw = match backend {
        Backend::Software => None,
        _ => hardware_encoder(s.codec),
    };
    let enc = hw.unwrap_or(s.codec.software_encoder());
    let mut a: Vec<String> = vec!["-c:v".into(), enc.into()];
    if let RateControl::Crf(crf) = s.rate {
        // Target bitrate is added by `build_args`, which knows the duration.
        if hw.is_some() {
            a.extend([
                "-q:v".into(),
                100u8.saturating_sub(crf.saturating_mul(2)).to_string(),
            ]);
        } else if s.codec == Codec::Vp9 {
            a.extend(["-crf".into(), crf.to_string(), "-b:v".into(), "0".into()]);
        } else {
            a.extend(["-crf".into(), crf.to_string()]);
        }
    }
    let speed: &[&str] = match (s.codec, hw.is_some(), s.speed) {
        (_, true, Speed::Fast) => &["-prio_speed", "1"],
        (_, true, _) => &[],
        (Codec::H264 | Codec::H265, false, Speed::Fast) => &["-preset", "veryfast"],
        (Codec::H264 | Codec::H265, false, Speed::Balanced) => &["-preset", "medium"],
        (Codec::H264 | Codec::H265, false, Speed::Small) => &["-preset", "slow"],
        (Codec::Vp9, _, Speed::Fast) => {
            &["-row-mt", "1", "-deadline", "realtime", "-cpu-used", "8"]
        }
        (Codec::Vp9, _, Speed::Balanced) => {
            &["-row-mt", "1", "-deadline", "good", "-cpu-used", "4"]
        }
        (Codec::Vp9, _, Speed::Small) => &["-row-mt", "1", "-deadline", "good", "-cpu-used", "2"],
        (Codec::Av1, _, Speed::Fast) => &["-preset", "10"],
        (Codec::Av1, _, Speed::Balanced) => &["-preset", "8"],
        (Codec::Av1, _, Speed::Small) => &["-preset", "5"],
    };
    a.extend(speed.iter().map(|x| x.to_string()));
    // 10-bit x264 would produce "High 10", unsupported by many players.
    if s.codec == Codec::H264 && hw.is_none() {
        a.extend(["-pix_fmt".into(), "yuv420p".into()]);
    }
    if s.codec == Codec::H265 && s.container != Container::Webm {
        a.extend(["-tag:v".into(), "hvc1".into()]);
    }
    a
}

/// Duration of the output selection.
pub fn job_duration(job: &Job, info: &VideoInfo) -> f64 {
    job.trim.map_or(info.duration, |t| t.duration())
}

/// Complete arguments to run `job` using `backend`.
pub fn build_args(job: &Job, info: &VideoInfo, backend: Backend) -> Result<Vec<String>> {
    if job.input == job.output {
        return Err(Error::OutputIsInput(job.output.clone()));
    }
    let mut a: Vec<String> = vec!["-hide_banner".into(), "-y".into()];
    a.extend(input_args(backend));
    if let Some(t) = job.trim {
        if !(t.start >= 0.0 && t.end > t.start) {
            return Err(Error::BadRange(format!("{}..{}", t.start, t.end)));
        }
        a.extend(["-ss".into(), t.start.to_string()]);
        a.extend(["-t".into(), t.duration().to_string()]);
    }
    a.extend(["-i".into(), job.input.display().to_string()]);
    match &job.mode {
        Mode::Copy => {
            a.extend(["-c", "copy", "-avoid_negative_ts", "make_zero"].map(String::from));
        }
        Mode::Remux(container) | Mode::RemuxSelected { container, .. } => {
            let excluded = match &job.mode {
                Mode::RemuxSelected {
                    excluded_tracks, ..
                } => excluded_tracks.as_slice(),
                _ => &[],
            };
            let tracks = crate::job::remux_tracks(info, *container, excluded)?;
            let format = match container {
                Container::Mp4 => "mp4",
                Container::Mkv => "matroska",
                Container::Webm => "webm",
            };
            if excluded.is_empty() {
                a.extend(["-map", "0"].map(String::from));
            } else {
                for index in tracks {
                    a.extend(["-map".into(), format!("0:{index}")]);
                }
            }
            a.extend(["-c", "copy", "-f", format].map(String::from));
        }
        Mode::Transcode(s) => {
            if let Some(f) = filter(s, info, backend) {
                a.extend(["-vf".into(), f]);
            }
            a.extend(video_args(s, backend));
            let audio = audio_kbps(s.container);
            if let RateControl::TargetBytes(bytes) = s.rate {
                let duration = job_duration(job, info);
                if duration <= 0.0 {
                    return Err(Error::BadRange("zero duration".into()));
                }
                let total_kbps = bytes as f64 * 8.0 / duration / 1000.0;
                let reserved_audio = if info.has_audio { audio } else { 0.0 };
                let video_kbps = (total_kbps - reserved_audio).max(50.0);
                a.extend(["-b:v".into(), format!("{}k", video_kbps.round() as u64)]);
            }
            let acodec = if s.container == Container::Webm {
                "libopus"
            } else {
                "aac"
            };
            a.extend([
                "-c:a".into(),
                acodec.into(),
                "-b:a".into(),
                format!("{}k", audio as u32),
            ]);
        }
    }
    a.push(job.output.display().to_string());
    Ok(a)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job::Trim;
    use std::path::PathBuf;

    pub(crate) fn info(bit_depth: u8, rotation: u16) -> VideoInfo {
        VideoInfo {
            duration: 80.0,
            width: 3840,
            height: 2160,
            codec: "hevc".into(),
            fps: 30.0,
            size_bytes: 100_000_000,
            bitrate: Some(10_000_000),
            has_audio: true,
            tracks: Vec::new(),
            track_indices: Vec::new(),
            bit_depth,
            rotation,
        }
    }

    fn job(trim: Option<Trim>, mode: Mode) -> Job {
        Job {
            input: PathBuf::from("in.mp4"),
            output: PathBuf::from("out.mp4"),
            trim,
            mode,
        }
    }

    fn settings(codec: Codec, container: Container, rate: RateControl) -> Settings {
        Settings {
            codec,
            container,
            height: None,
            rate,
            hardware: false,
            speed: Speed::Balanced,
        }
    }

    fn has(a: &[String], x: &str) -> bool {
        a.iter().any(|s| s == x)
    }

    #[test]
    fn remux_maps_all_tracks_and_rejects_incompatible_codecs() {
        let mut i = info(8, 0);
        i.tracks = vec![
            ("video".into(), "h264".into()),
            ("audio".into(), "aac".into()),
        ];
        let a = build_args(
            &job(None, Mode::Remux(Container::Mkv)),
            &i,
            Backend::Software,
        )
        .unwrap();
        assert!(a.windows(2).any(|w| w == ["-map", "0"]));
        assert!(a.windows(2).any(|w| w == ["-c", "copy"]));
        assert!(a.windows(2).any(|w| w == ["-f", "matroska"]));
        assert!(!has(&a, "-vf"));
        assert!(build_args(
            &job(None, Mode::Remux(Container::Webm)),
            &i,
            Backend::Software
        )
        .is_err());
        i.tracks.push(("subtitle".into(), "subrip".into()));
        assert!(build_args(
            &job(None, Mode::Remux(Container::Mp4)),
            &i,
            Backend::Software
        )
        .is_err());
        i.tracks.push(("data".into(), "bin_data".into()));
        assert!(build_args(
            &job(None, Mode::Remux(Container::Mkv)),
            &i,
            Backend::Software
        )
        .is_err());
    }

    #[test]
    fn selective_remux_maps_real_indices_and_never_drops_audio() {
        let source = crate::probe::parse_probe(r#"{"streams":[{"index":0,"codec_type":"video","codec_name":"h264"},{"index":2,"codec_type":"audio","codec_name":"aac"},{"index":4,"codec_type":"data","codec_name":"bin_data"}],"format":{"duration":"1"}}"#).unwrap();
        let mode = Mode::RemuxSelected {
            container: Container::Mkv,
            excluded_tracks: vec![4],
        };
        let args = build_args(&job(None, mode), &source, Backend::Software).unwrap();
        let maps: Vec<_> = args
            .windows(2)
            .filter(|pair| pair[0] == "-map")
            .map(|pair| pair[1].as_str())
            .collect();
        assert_eq!(maps, vec!["0:0", "0:2"]);
        assert!(args.windows(2).any(|pair| pair == ["-c", "copy"]));
        assert!(build_args(
            &job(
                None,
                Mode::RemuxSelected {
                    container: Container::Mkv,
                    excluded_tracks: vec![2, 4]
                }
            ),
            &source,
            Backend::Software
        )
        .is_err());
    }

    #[test]
    fn trim_uses_stream_copy() {
        let t = Some(Trim {
            start: 1.0,
            end: 5.0,
        });
        let a = build_args(&job(t, Mode::Copy), &info(8, 0), Backend::Software).unwrap();
        assert!(a.windows(2).any(|w| w == ["-c", "copy"]));
        assert!(a.windows(2).any(|w| w == ["-t", "4"]));
        assert_eq!(a.last().unwrap(), "out.mp4");
    }

    #[test]
    fn reject_invalid_range() {
        let t = Some(Trim {
            start: 5.0,
            end: 1.0,
        });
        assert!(build_args(&job(t, Mode::Copy), &info(8, 0), Backend::Software).is_err());
    }

    #[test]
    fn reject_output_equal_to_input() {
        let j = Job {
            input: PathBuf::from("a.mp4"),
            output: PathBuf::from("a.mp4"),
            trim: None,
            mode: Mode::Copy,
        };
        assert!(build_args(&j, &info(8, 0), Backend::Software).is_err());
    }

    #[test]
    fn h265_sets_scale_and_tag() {
        let mut s = settings(Codec::H265, Container::Mp4, RateControl::Crf(28));
        s.height = Some(720);
        let a = build_args(
            &job(None, Mode::Transcode(s)),
            &info(8, 0),
            Backend::Software,
        )
        .unwrap();
        for x in ["libx265", "scale=-2:720", "hvc1", "aac", "medium"] {
            assert!(has(&a, x), "missing {x}");
        }
    }

    #[test]
    fn webm_uses_vp9_and_opus() {
        let s = settings(Codec::Vp9, Container::Webm, RateControl::Crf(33));
        let a = build_args(
            &job(None, Mode::Transcode(s)),
            &info(8, 0),
            Backend::Software,
        )
        .unwrap();
        for x in ["libvpx-vp9", "libopus"] {
            assert!(has(&a, x), "missing {x}");
        }
    }

    #[test]
    fn target_size_sets_bitrate() {
        // 10 MB in 80 s = 1000 kbps total; 128 kbps audio => 872k video
        let s = settings(
            Codec::H264,
            Container::Mp4,
            RateControl::TargetBytes(10_000_000),
        );
        let a = build_args(
            &job(None, Mode::Transcode(s)),
            &info(8, 0),
            Backend::Software,
        )
        .unwrap();
        assert!(a.windows(2).any(|w| w == ["-b:v", "872k"]), "{a:?}");
    }

    #[test]
    fn target_size_does_not_reserve_audio_bitrate_for_silent_sources() {
        let s = settings(
            Codec::H264,
            Container::Mp4,
            RateControl::TargetBytes(10_000_000),
        );
        let mut source = info(8, 0);
        source.has_audio = false;
        let args = build_args(&job(None, Mode::Transcode(s)), &source, Backend::Software).unwrap();
        assert!(args.windows(2).any(|w| w == ["-b:v", "1000k"]));
    }

    #[test]
    fn h264_software_is_always_8bit() {
        let s = settings(Codec::H264, Container::Mp4, RateControl::Crf(23));
        let a = build_args(
            &job(None, Mode::Transcode(s)),
            &info(10, 0),
            Backend::Software,
        )
        .unwrap();
        assert!(a.windows(2).any(|w| w == ["-pix_fmt", "yuv420p"]));
    }

    #[test]
    fn speed_changes_software_preset() {
        let mut s = settings(Codec::H265, Container::Mp4, RateControl::Crf(28));
        s.speed = Speed::Fast;
        assert!(has(&video_args(&s, Backend::Software), "veryfast"));
        s.speed = Speed::Small;
        assert!(has(&video_args(&s, Backend::Software), "slow"));
        s.codec = Codec::Av1;
        assert!(video_args(&s, Backend::Software)
            .windows(2)
            .any(|w| w == ["-preset", "5"]));
    }

    #[test]
    fn hardware_disabled_uses_software_only() {
        let s = settings(Codec::H265, Container::Mp4, RateControl::Crf(28));
        assert_eq!(
            backends(&Mode::Transcode(s), &info(10, 0)),
            vec![Backend::Software]
        );
        assert_eq!(backends(&Mode::Copy, &info(10, 0)), vec![Backend::Software]);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_fallback_chain() {
        let mut s = settings(Codec::H265, Container::Mp4, RateControl::Crf(28));
        s.hardware = true;
        let m = Mode::Transcode(s);
        assert_eq!(
            backends(&m, &info(10, 0)),
            vec![Backend::Chip, Backend::HardwareEncode, Backend::Software]
        );
        assert_eq!(
            backends(&m, &info(8, 0)),
            vec![Backend::HardwareEncode, Backend::Software]
        );
        // No VP9 hardware encoder is configured on macOS.
        s.codec = Codec::Vp9;
        assert_eq!(
            backends(&Mode::Transcode(s), &info(10, 0)),
            vec![Backend::Software]
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn hardware_pipeline_rotates_and_scales() {
        let mut s = settings(Codec::H265, Container::Mp4, RateControl::Crf(28));
        s.height = Some(1080);
        let mut i = info(10, 90);
        // Portrait source: probe dimensions are already rotation-corrected.
        (i.width, i.height) = (2160, 3840);
        assert_eq!(
            filter(&s, &i, Backend::Chip).unwrap(),
            "transpose_vt=dir=cclock,scale_vt=w=608:h=1080"
        );
        i.rotation = 270;
        assert!(filter(&s, &i, Backend::Chip)
            .unwrap()
            .starts_with("transpose_vt=dir=clock,"));
        let a = build_args(&job(None, Mode::Transcode(s)), &i, Backend::Chip).unwrap();
        assert!(a.windows(2).any(|w| w == ["-hwaccel", "videotoolbox"]));
        let pos_hw = a.iter().position(|x| x == "-hwaccel").unwrap();
        let pos_i = a.iter().position(|x| x == "-i").unwrap();
        assert!(pos_hw < pos_i, "-hwaccel must precede -i");
    }

    #[test]
    fn hardware_pipeline_without_transforms_has_no_filters() {
        let s = settings(Codec::H265, Container::Mp4, RateControl::Crf(28));
        assert_eq!(filter(&s, &info(10, 0), Backend::Chip), None);
    }
}
