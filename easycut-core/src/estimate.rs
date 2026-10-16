// File: easycut-core/src/estimate.rs
//! Stima della dimensione del file finale (euristica, non una misura).
use crate::job::{Codec, Container, Mode, RateControl, Settings, Speed, Trim};
use crate::probe::VideoInfo;

pub const AUDIO_KBPS_DEFAULT: f64 = 128.0;
pub const AUDIO_KBPS_OPUS: f64 = 96.0;

pub fn audio_kbps(container: Container) -> f64 {
    if container == Container::Webm {
        AUDIO_KBPS_OPUS
    } else {
        AUDIO_KBPS_DEFAULT
    }
}

/// (bit per pixel per frame, CRF di riferimento) per un contenuto "tipico".
fn reference(codec: Codec) -> (f64, f64) {
    match codec {
        Codec::H264 => (0.07, 23.0),
        Codec::H265 => (0.045, 28.0),
        Codec::Vp9 => (0.045, 32.0),
        Codec::Av1 => (0.035, 35.0),
    }
}

/// Altezza effettiva di uscita: mai oltre l'origine.
pub fn output_height(info: &VideoInfo, s: &Settings) -> u32 {
    s.height.map_or(info.height, |h| h.min(info.height))
}

pub fn output_dims(info: &VideoInfo, s: &Settings) -> (u32, u32) {
    let h = output_height(info, s);
    if info.height == 0 {
        return (info.width, h);
    }
    let w = (info.width as f64 * h as f64 / info.height as f64).round() as u32;
    (w, h)
}

fn selected_duration(info: &VideoInfo, trim: Option<Trim>) -> f64 {
    trim.map_or(info.duration, |t| t.duration()).max(0.0)
}

/// Dimensione stimata in byte del risultato di `mode` sul tratto `trim`.
pub fn estimate_bytes(info: &VideoInfo, mode: &Mode, trim: Option<Trim>) -> u64 {
    let secs = selected_duration(info, trim);
    match mode {
        Mode::Copy => {
            if info.duration <= 0.0 {
                return 0;
            }
            (info.size_bytes as f64 * secs / info.duration) as u64
        }
        Mode::Transcode(s) => transcode_bytes(info, s, secs),
    }
}

fn transcode_bytes(info: &VideoInfo, s: &Settings, secs: f64) -> u64 {
    match s.rate {
        RateControl::TargetBytes(b) => b,
        RateControl::Crf(crf) => {
            let audio_bps = if info.has_audio {
                audio_kbps(s.container) * 1000.0
            } else {
                0.0
            };
            let (w, h) = output_dims(info, s);
            let pixels_per_sec = w as f64 * h as f64 * info.fps.max(1.0);
            let (bpp, ref_crf) = reference(s.codec);
            let mut video_bps = pixels_per_sec * bpp * 2f64.powf((ref_crf - crf as f64) / 6.0);
            // Non può superare di molto quanto il sorgente usa già.
            if let Some(total) = info.bitrate {
                let src_video = (total as f64 - audio_bps).max(total as f64 * 0.5);
                let src_pixels = info.width as f64 * info.height as f64;
                let ratio = (w as f64 * h as f64 / src_pixels.max(1.0)).powf(0.75);
                video_bps = video_bps.min(src_video * ratio);
            }
            // Preset più lenti comprimono meglio (vale solo per il software).
            let hw = s.hardware
                && matches!(s.codec, Codec::H264 | Codec::H265)
                && cfg!(target_os = "macos");
            if !hw {
                video_bps *= match s.speed {
                    Speed::Fast => 1.2,
                    Speed::Balanced => 1.0,
                    Speed::Small => 0.92,
                };
            }
            // L'encoder hardware è meno efficiente di quello software.
            if s.hardware
                && matches!(s.codec, Codec::H264 | Codec::H265)
                && cfg!(target_os = "macos")
            {
                video_bps *= 1.3;
            }
            ((video_bps + audio_bps) * secs / 8.0) as u64
        }
    }
}

/// Durata di ciascun campione misurato.
pub const SAMPLE_SECS: f64 = 3.0;

/// Tratti (inizio assoluto, durata) da codificare per misurare la dimensione:
/// tre campioni a 15%, 50% e 85% del tratto, oppure tutto se è breve.
pub fn sample_plan(range_start: f64, secs: f64) -> Vec<(f64, f64)> {
    if secs <= SAMPLE_SECS * 2.0 {
        return vec![(range_start, secs.max(0.0))];
    }
    [0.15, 0.5, 0.85]
        .iter()
        .map(|f| {
            let centre = range_start + secs * f;
            let start =
                (centre - SAMPLE_SECS / 2.0).clamp(range_start, range_start + secs - SAMPLE_SECS);
            (start, SAMPLE_SECS)
        })
        .collect()
}

/// Dimensione totale prevista a partire dai campioni misurati `(byte, secondi)`.
/// I campioni sono solo video: l'audio si aggiunge a bitrate noto.
pub fn from_samples(samples: &[(u64, f64)], info: &VideoInfo, s: &Settings, secs: f64) -> u64 {
    let bytes: u64 = samples.iter().map(|(b, _)| b).sum();
    let dur: f64 = samples.iter().map(|(_, d)| d).sum();
    if dur <= 0.0 {
        return 0;
    }
    let audio = if info.has_audio {
        audio_kbps(s.container) * 1000.0 / 8.0 * secs
    } else {
        0.0
    };
    (bytes as f64 / dur * secs + audio) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info() -> VideoInfo {
        VideoInfo {
            duration: 100.0,
            width: 1920,
            height: 1080,
            codec: "h264".into(),
            fps: 30.0,
            size_bytes: 100_000_000,
            bitrate: Some(8_000_000),
            has_audio: true,
            bit_depth: 8,
            rotation: 0,
        }
    }

    fn crf(codec: Codec, height: Option<u32>, crf: u8) -> Mode {
        Mode::Transcode(Settings {
            codec,
            container: Container::Mp4,
            height,
            rate: RateControl::Crf(crf),
            hardware: false,
            speed: Speed::Balanced,
        })
    }

    #[test]
    fn copy_e_proporzionale_al_taglio() {
        let t = Trim {
            start: 0.0,
            end: 25.0,
        };
        assert_eq!(estimate_bytes(&info(), &Mode::Copy, Some(t)), 25_000_000);
    }

    #[test]
    fn h265_e_piu_piccolo_di_h264() {
        let a = estimate_bytes(&info(), &crf(Codec::H264, None, 23), None);
        let b = estimate_bytes(&info(), &crf(Codec::H265, None, 28), None);
        assert!(b < a, "{b} < {a}");
    }

    #[test]
    fn risoluzione_minore_riduce_la_stima() {
        let a = estimate_bytes(&info(), &crf(Codec::H264, None, 23), None);
        let b = estimate_bytes(&info(), &crf(Codec::H264, Some(480), 23), None);
        assert!(b < a);
    }

    #[test]
    fn crf_piu_alto_riduce_la_stima() {
        let a = estimate_bytes(&info(), &crf(Codec::H264, Some(720), 20), None);
        let b = estimate_bytes(&info(), &crf(Codec::H264, Some(720), 28), None);
        assert!(b < a);
    }

    #[test]
    fn non_supera_il_sorgente() {
        let e = estimate_bytes(&info(), &crf(Codec::H264, None, 10), None);
        assert!(e <= 100_000_000, "{e}");
    }

    #[test]
    fn velocita_veloce_produce_file_piu_grandi() {
        let mode = |speed| {
            Mode::Transcode(Settings {
                codec: Codec::H265,
                container: Container::Mp4,
                height: Some(720),
                rate: RateControl::Crf(28),
                hardware: false,
                speed,
            })
        };
        let fast = estimate_bytes(&info(), &mode(Speed::Fast), None);
        let small = estimate_bytes(&info(), &mode(Speed::Small), None);
        assert!(fast > small);
    }

    #[test]
    fn target_rispetta_la_dimensione_richiesta() {
        let m = Mode::Transcode(Settings {
            codec: Codec::H264,
            container: Container::Mp4,
            height: None,
            rate: RateControl::TargetBytes(10_000_000),
            hardware: false,
            speed: Speed::Balanced,
        });
        assert_eq!(estimate_bytes(&info(), &m, None), 10_000_000);
    }

    #[test]
    fn piano_campioni_breve_codifica_tutto() {
        assert_eq!(sample_plan(2.0, 5.0), vec![(2.0, 5.0)]);
    }

    #[test]
    fn piano_campioni_resta_dentro_il_tratto() {
        let plan = sample_plan(10.0, 60.0);
        assert_eq!(plan.len(), 3);
        for (start, len) in plan {
            assert!(start >= 10.0 && start + len <= 70.0 + 1e-9, "{start}+{len}");
        }
    }

    #[test]
    fn da_campioni_scala_alla_durata_e_somma_audio() {
        let i = info();
        let s = Settings {
            codec: Codec::H264,
            container: Container::Mp4,
            height: None,
            rate: RateControl::Crf(23),
            hardware: false,
            speed: Speed::Balanced,
        };
        // 1 MB ogni 3 s di video, su 30 s: 10 MB + 128 kbps di audio (480 kB)
        let est = from_samples(&[(1_000_000, 3.0)], &i, &s, 30.0);
        assert_eq!(est, 10_000_000 + 480_000);
    }
}
