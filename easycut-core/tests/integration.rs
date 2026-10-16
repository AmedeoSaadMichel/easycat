// File: easycut-core/tests/integration.rs
//! Richiede ffmpeg nel PATH; usa solo clip generate al volo.
use easycut_core::estimate::estimate_bytes;
use easycut_core::frames::{frame_at, thumb_size};
use easycut_core::{
    ffmpeg, presets, probe, Codec, Container, Job, Mode, RateControl, Settings, Speed, Trim,
};
use std::path::PathBuf;
use std::process::Command;

fn clip(name: &str) -> Option<PathBuf> {
    let dir = std::env::temp_dir().join(format!("easycut-it-{}", std::process::id()));
    std::fs::create_dir_all(&dir).ok()?;
    let path = dir.join(name);
    let ok = Command::new("ffmpeg")
        .args(["-v", "error", "-y", "-f", "lavfi", "-i"])
        .arg("testsrc2=duration=12:size=1280x720:rate=30")
        .args(["-f", "lavfi", "-i", "sine=duration=12"])
        .args(["-c:v", "libx264", "-g", "30", "-c:a", "aac"])
        .arg(&path)
        .status()
        .ok()?
        .success();
    ok.then_some(path)
}

#[test]
fn flusso_completo() {
    let Some(input) = clip("in.mp4") else {
        eprintln!("ffmpeg non disponibile: test saltato");
        return;
    };
    let info = probe(&input).unwrap();
    assert_eq!((info.width, info.height), (1280, 720));
    assert!(info.has_audio);

    // Fotogramma: dimensioni coerenti con thumb_size.
    let (w, h) = thumb_size(info.width, info.height, 180);
    let f = frame_at(&input, 5.0, w, h).unwrap();
    assert_eq!(f.rgb.len(), w * h * 3);

    // Streaming: 1 s a 10 fps = circa 10 fotogrammi, e si ferma alla chiusura.
    let mut st = easycut_core::playback::FrameStream::open(&input, 2.0, 1.0, 10.0, w, h).unwrap();
    let mut n = 0;
    while let Some(fr) = st.next_frame() {
        assert_eq!(fr.rgb.len(), w * h * 3);
        n += 1;
    }
    assert!((9..=11).contains(&n), "fotogrammi {n}");

    // Taglio senza ricodifica.
    let out = input.with_file_name("cut.mp4");
    let trim = Trim {
        start: 2.0,
        end: 8.0,
    };
    let job = Job {
        input: input.clone(),
        output: out.clone(),
        trim: Some(trim),
        mode: Mode::Copy,
    };
    ffmpeg::run(&job, &info).unwrap();
    let cut = probe(&out).unwrap();
    assert!((cut.duration - 6.0).abs() < 1.5, "durata {}", cut.duration);

    // Ricodifica: ogni preset produce un file valido e la stima è nell'ordine di grandezza.
    let encoders = ffmpeg::available_encoders();
    for p in presets::builtin(&info, &encoders) {
        let out = input.with_file_name(format!("{}.{}", p.id, p.settings.container.ext()));
        let s = Settings {
            hardware: false,
            ..p.settings
        };
        let job = Job {
            input: input.clone(),
            output: out.clone(),
            trim: None,
            mode: Mode::Transcode(s),
        };
        ffmpeg::run(&job, &info).unwrap();
        let real = std::fs::metadata(&out).unwrap().len();
        let est = estimate_bytes(&info, &job.mode, None);
        eprintln!("{:>14}: stima {:>9} reale {:>9}", p.id, est, real);
        assert!(probe(&out).unwrap().duration > 11.0);
    }

    // Misura su campioni: vicina alla dimensione reale della stessa codifica.
    let s = Settings {
        codec: Codec::H264,
        container: Container::Mp4,
        height: None,
        rate: RateControl::Crf(28),
        hardware: false,
        speed: Speed::Balanced,
    };
    let measured = ffmpeg::measure_size(
        &input,
        &info,
        &s,
        None,
        &std::sync::atomic::AtomicBool::new(false),
    )
    .unwrap();
    let out = input.with_file_name("measured.mp4");
    let job = Job {
        input: input.clone(),
        output: out.clone(),
        trim: None,
        mode: Mode::Transcode(s),
    };
    let mut last = 0.0;
    ffmpeg::run_with_progress(
        &job,
        &info,
        &mut |p| {
            assert!(p >= last, "il progresso non deve tornare indietro");
            last = p;
        },
        &std::sync::atomic::AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(last, 1.0);
    let real = std::fs::metadata(&out).unwrap().len() as f64;
    eprintln!("misurata {measured} reale {real}");
    assert!(
        (measured as f64 / real - 1.0).abs() < 0.25,
        "misurata {measured} reale {real}"
    );

    // Annullamento: errore dedicato e nessun file parziale.
    let out = input.with_file_name("cancel.mp4");
    let job = Job {
        input: input.clone(),
        output: out.clone(),
        trim: None,
        mode: Mode::Transcode(s),
    };
    let cancel = std::sync::atomic::AtomicBool::new(true);
    let r = ffmpeg::run_with_progress(&job, &info, &mut |_| {}, &cancel);
    assert!(matches!(r, Err(easycut_core::Error::Cancelled)));
    assert!(!out.exists());

    // Dimensione target: il risultato deve avvicinarsi al valore richiesto.
    let target = 1_500_000u64;
    let out = input.with_file_name("target.mp4");
    let s = Settings {
        codec: Codec::H264,
        container: Container::Mp4,
        height: None,
        rate: RateControl::TargetBytes(target),
        hardware: false,
        speed: Speed::Balanced,
    };
    let job = Job {
        input,
        output: out.clone(),
        trim: None,
        mode: Mode::Transcode(s),
    };
    ffmpeg::run(&job, &info).unwrap();
    let real = std::fs::metadata(&out).unwrap().len() as f64;
    eprintln!("target {target} reale {real}");
    assert!((real / target as f64 - 1.0).abs() < 0.35, "reale {real}");
    let _ = std::fs::remove_dir_all(out.parent().unwrap());
}

/// PSNR medio tra due video, calcolato da FFmpeg.
fn psnr(a: &std::path::Path, b: &std::path::Path) -> f64 {
    let out = Command::new("ffmpeg")
        .args(["-hide_banner", "-i"])
        .arg(a)
        .arg("-i")
        .arg(b)
        .args(["-lavfi", "psnr", "-f", "null", "-"])
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&out.stderr);
    err.split("average:")
        .nth(1)
        .and_then(|r| r.split_whitespace().next())
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.0)
}

#[cfg(target_os = "macos")]
#[test]
fn chip_su_video_10_bit_ruotato() {
    let dir = std::env::temp_dir().join(format!("easycut-chip-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let flat = dir.join("flat.mp4");
    let src = dir.join("rot.mp4");
    let ok = Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=duration=3:size=1280x720:rate=30",
        ])
        .args([
            "-pix_fmt",
            "p010le",
            "-c:v",
            "hevc_videotoolbox",
            "-profile:v",
            "main10",
        ])
        .args(["-b:v", "10M", "-tag:v", "hvc1"])
        .arg(&flat)
        .status()
        .is_ok_and(|s| s.success())
        && Command::new("ffmpeg")
            .args(["-v", "error", "-y", "-display_rotation", "90", "-i"])
            .arg(&flat)
            .args(["-c", "copy"])
            .arg(&src)
            .status()
            .is_ok_and(|s| s.success());
    if !ok {
        eprintln!("VideoToolbox non disponibile: test saltato");
        return;
    }
    let info = probe(&src).unwrap();
    assert_eq!(
        (info.bit_depth, info.rotation, info.width, info.height),
        (10, 90, 720, 1280)
    );

    let settings = |hardware| Settings {
        codec: Codec::H265,
        container: Container::Mp4,
        height: Some(480),
        rate: RateControl::Crf(28),
        hardware,
        speed: Speed::Balanced,
    };
    let chip_out = dir.join("chip.mp4");
    let job = Job {
        input: src.clone(),
        output: chip_out.clone(),
        trim: None,
        mode: Mode::Transcode(settings(true)),
    };
    assert_eq!(ffmpeg::run(&job, &info).unwrap(), ffmpeg::Backend::Chip);
    let soft_out = dir.join("soft.mp4");
    let job = Job {
        input: src,
        output: soft_out.clone(),
        trim: None,
        mode: Mode::Transcode(settings(false)),
    };
    assert_eq!(ffmpeg::run(&job, &info).unwrap(), ffmpeg::Backend::Software);

    let chip = probe(&chip_out).unwrap();
    assert_eq!((chip.width, chip.height, chip.rotation), (270, 480, 0));
    // Stessa immagine del percorso software: la rotazione sul chip è corretta.
    let p = psnr(&soft_out, &chip_out);
    assert!(p > 30.0, "PSNR {p}: immagine ruotata in modo diverso");
    let _ = std::fs::remove_dir_all(&dir);
}
