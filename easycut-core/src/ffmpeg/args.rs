// File: easycut-core/src/ffmpeg/args.rs
//! Costruzione degli argomenti FFmpeg. Funzioni pure, testabili senza FFmpeg.
use crate::estimate::{audio_kbps, output_dims};
use crate::job::{Codec, Container, Job, Mode, RateControl, Settings, Speed};
use crate::probe::VideoInfo;
use crate::{Error, Result};

/// Come viene elaborato il video, dal più veloce al più compatibile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// Decodifica, rotazione, ridimensionamento e codifica tutti sul chip (VideoToolbox).
    Chip,
    /// Solo la codifica sul chip; decodifica e filtri sulla CPU.
    HardwareEncode,
    /// Tutto sulla CPU.
    Software,
}

impl Backend {
    pub fn label(self) -> &'static str {
        match self {
            Backend::Chip => "tutto sul chip",
            Backend::HardwareEncode => "encoder hardware",
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

/// Ordine in cui provare i backend per questo job: se uno fallisce si passa al successivo.
/// La pipeline tutta sul chip conviene solo con sorgenti oltre 8 bit (misurato:
/// 1,6 s contro 2,8 s su 4K HEVC 10 bit; nessun guadagno su 8 bit).
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

/// Opzioni d'ingresso (prima di `-i`).
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

/// Filtro video, se serve. Sul chip la rotazione va applicata a mano: FFmpeg
/// non ruota da solo i fotogrammi hardware (verificato con PSNR sulle tre rotazioni).
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
        // Con la dimensione target il bitrate lo aggiunge `build_args`, che conosce la durata.
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
    // x264 da sorgenti a 10 bit produrrebbe "High 10", che molti player non riproducono.
    if s.codec == Codec::H264 && hw.is_none() {
        a.extend(["-pix_fmt".into(), "yuv420p".into()]);
    }
    if s.codec == Codec::H265 && s.container != Container::Webm {
        a.extend(["-tag:v".into(), "hvc1".into()]);
    }
    a
}

/// Durata del tratto che verrà scritto.
pub fn job_duration(job: &Job, info: &VideoInfo) -> f64 {
    job.trim.map_or(info.duration, |t| t.duration())
}

/// Argomenti completi per eseguire `job` con `backend`.
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
        Mode::Transcode(s) => {
            if let Some(f) = filter(s, info, backend) {
                a.extend(["-vf".into(), f]);
            }
            a.extend(video_args(s, backend));
            let audio = audio_kbps(s.container);
            if let RateControl::TargetBytes(bytes) = s.rate {
                let duration = job_duration(job, info);
                if duration <= 0.0 {
                    return Err(Error::BadRange("durata nulla".into()));
                }
                let total_kbps = bytes as f64 * 8.0 / duration / 1000.0;
                let video_kbps = (total_kbps - audio).max(50.0);
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
    fn taglio_usa_stream_copy() {
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
    fn intervallo_invalido() {
        let t = Some(Trim {
            start: 5.0,
            end: 1.0,
        });
        assert!(build_args(&job(t, Mode::Copy), &info(8, 0), Backend::Software).is_err());
    }

    #[test]
    fn output_uguale_input_rifiutato() {
        let j = Job {
            input: PathBuf::from("a.mp4"),
            output: PathBuf::from("a.mp4"),
            trim: None,
            mode: Mode::Copy,
        };
        assert!(build_args(&j, &info(8, 0), Backend::Software).is_err());
    }

    #[test]
    fn h265_con_scala_e_tag() {
        let mut s = settings(Codec::H265, Container::Mp4, RateControl::Crf(28));
        s.height = Some(720);
        let a = build_args(
            &job(None, Mode::Transcode(s)),
            &info(8, 0),
            Backend::Software,
        )
        .unwrap();
        for x in ["libx265", "scale=-2:720", "hvc1", "aac", "medium"] {
            assert!(has(&a, x), "manca {x}");
        }
    }

    #[test]
    fn webm_usa_vp9_e_opus() {
        let s = settings(Codec::Vp9, Container::Webm, RateControl::Crf(33));
        let a = build_args(
            &job(None, Mode::Transcode(s)),
            &info(8, 0),
            Backend::Software,
        )
        .unwrap();
        for x in ["libvpx-vp9", "libopus"] {
            assert!(has(&a, x), "manca {x}");
        }
    }

    #[test]
    fn dimensione_target_diventa_bitrate() {
        // 10 MB in 80 s = 1000 kbps totali; 128 kbps audio => 872k video
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
    fn h264_software_sempre_a_8_bit() {
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
    fn velocita_cambia_il_preset_software() {
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
    fn senza_hardware_solo_software() {
        let s = settings(Codec::H265, Container::Mp4, RateControl::Crf(28));
        assert_eq!(
            backends(&Mode::Transcode(s), &info(10, 0)),
            vec![Backend::Software]
        );
        assert_eq!(backends(&Mode::Copy, &info(10, 0)), vec![Backend::Software]);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn catena_di_ripiego_su_mac() {
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
        // VP9 non ha encoder hardware su Mac.
        s.codec = Codec::Vp9;
        assert_eq!(
            backends(&Mode::Transcode(s), &info(10, 0)),
            vec![Backend::Software]
        );
    }

    #[test]
    fn chip_ruota_e_ridimensiona_sul_chip() {
        let mut s = settings(Codec::H265, Container::Mp4, RateControl::Crf(28));
        s.height = Some(1080);
        let mut i = info(10, 90);
        // Video verticale: le dimensioni del probe sono già ruotate.
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
        assert!(pos_hw < pos_i, "-hwaccel deve precedere -i");
    }

    #[test]
    fn chip_senza_rotazione_ne_scala_non_ha_filtri() {
        let s = settings(Codec::H265, Container::Mp4, RateControl::Crf(28));
        assert_eq!(filter(&s, &info(10, 0), Backend::Chip), None);
    }
}
