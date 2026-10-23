// File: easycat-core/tests/integration.rs
//! Requires FFmpeg on PATH; uses generated clips only.
use easycat_core::estimate::estimate_bytes;
use easycat_core::frames::{frame_at, thumb_size};
use easycat_core::{
    ffmpeg, presets, probe, Codec, Container, Crop, Job, Mode, RateControl, Settings, Speed, Trim,
};
use std::path::PathBuf;
use std::process::Command;

fn clip(name: &str) -> Option<PathBuf> {
    let dir = std::env::temp_dir().join(format!("easycat-it-{}", std::process::id()));
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
fn complete_workflow() {
    let Some(input) = clip("in.mp4") else {
        eprintln!("FFmpeg unavailable: skipping test");
        return;
    };
    let info = probe(&input).unwrap();
    assert_eq!((info.width, info.height), (1280, 720));
    assert!(info.has_audio);

    // Frame dimensions must agree with thumb_size.
    let (w, h) = thumb_size(info.width, info.height, 180);
    let f = frame_at(&input, 5.0, w, h).unwrap();
    assert_eq!(f.rgb.len(), w * h * 3);

    // Streaming: 1 s at 10 fps gives about 10 frames; dropping stops the process.
    let mut st = easycat_core::playback::FrameStream::open(&input, 2.0, 1.0, 10.0, w, h).unwrap();
    let mut n = 0;
    while let Some(fr) = st.next_frame() {
        assert_eq!(fr.rgb.len(), w * h * 3);
        n += 1;
    }
    assert!((9..=11).contains(&n), "frames {n}");

    // Trim without re-encoding.
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

    // Every profile must produce valid output and a plausible rough estimate.
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
        eprintln!("{:>14}: estimate {:>9}, actual {:>9}", p.id, est, real);
        assert!(probe(&out).unwrap().duration > 11.0);
    }

    // Sample estimate should approach the actual size with identical settings.
    let s = Settings {
        crop: None,
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
            assert!(p >= last, "progress must not move backwards");
            last = p;
        },
        &std::sync::atomic::AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(last, 1.0);
    let real = std::fs::metadata(&out).unwrap().len() as f64;
    let rough = estimate_bytes(&info, &job.mode, None) as f64;
    eprintln!("rough={rough:.0} bytes, sampled={measured} bytes, actual={real:.0} bytes; rough error={:.1}%, sample error={:.1}%", (rough / real - 1.0).abs() * 100.0, (measured as f64 / real - 1.0).abs() * 100.0);
    assert!((measured as f64 - real).abs() < (rough - real).abs());
    assert!(
        (measured as f64 / real - 1.0).abs() < 0.05,
        "measured {measured}, actual {real}"
    );

    // Cancellation returns a dedicated error and leaves no partial output.
    let out = input.with_file_name("cancel.mp4");
    let job = Job {
        input: input.clone(),
        output: out.clone(),
        trim: None,
        mode: Mode::Transcode(s),
    };
    let cancel = std::sync::atomic::AtomicBool::new(true);
    let r = ffmpeg::run_with_progress(&job, &info, &mut |_| {}, &cancel);
    assert!(matches!(r, Err(easycat_core::Error::Cancelled)));
    assert!(!out.exists());

    // Target size: output should approach the requested value.
    let target = 1_500_000u64;
    let out = input.with_file_name("target.mp4");
    let s = Settings {
        crop: None,
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
    eprintln!("target {target} actual {real}");
    assert!((real / target as f64 - 1.0).abs() < 0.35, "actual {real}");
    let _ = std::fs::remove_dir_all(out.parent().unwrap());
}

/// Average PSNR between two videos, computed by FFmpeg.
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
fn hardware_pipeline_handles_rotated_10bit_video() {
    let dir = std::env::temp_dir().join(format!("easycat-chip-{}", std::process::id()));
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
        eprintln!("VideoToolbox unavailable: skipping test");
        return;
    }
    let info = probe(&src).unwrap();
    assert_eq!(
        (info.bit_depth, info.rotation, info.width, info.height),
        (10, 90, 720, 1280)
    );

    let settings = |hardware| Settings {
        crop: None,
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
    // Compare with software output to verify hardware rotation.
    let p = psnr(&soft_out, &chip_out);
    assert!(p > 30.0, "PSNR {p}: rotation mismatch");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn remux_preserves_video_and_all_audio_tracks() {
    let input = clip("remux-source.mp4").expect("generate test clip");
    let multi = input.with_file_name("remux-multi.mp4");
    assert!(Command::new("ffmpeg")
        .args(["-v", "error", "-y", "-i"])
        .arg(&input)
        .args(["-map", "0:v", "-map", "0:a", "-map", "0:a", "-c", "copy"])
        .arg(&multi)
        .status()
        .unwrap()
        .success());
    let fingerprint = |path: &PathBuf| {
        let out = Command::new("ffmpeg")
            .args(["-v", "error", "-i"])
            .arg(path)
            .args([
                "-map",
                "0",
                "-c",
                "copy",
                "-f",
                "streamhash",
                "-hash",
                "sha256",
                "-",
            ])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        out.stdout
    };
    let original_hash = fingerprint(&multi);
    let mut source = multi;
    for (name, container) in [
        ("remux.mkv", Container::Mkv),
        ("remux-back.mp4", Container::Mp4),
    ] {
        let output = input.with_file_name(name);
        let info = probe(&source).unwrap();
        assert_eq!(info.tracks.len(), 3);
        let job = Job {
            input: source,
            output: output.clone(),
            trim: None,
            mode: Mode::Remux(container),
        };
        assert_eq!(ffmpeg::run(&job, &info).unwrap(), ffmpeg::Backend::Software);
        assert_eq!(probe(&output).unwrap().tracks, info.tracks);
        assert_eq!(fingerprint(&output), original_hash);
        source = output;
    }
    let output = input.with_file_name("remux-invalid.webm");
    let job = Job {
        input: source.clone(),
        output: output.clone(),
        trim: None,
        mode: Mode::Remux(Container::Webm),
    };
    assert!(ffmpeg::run(&job, &probe(&source).unwrap()).is_err());
    assert!(!output.exists());

    // A camera-style MOV includes a timecode data track alongside all media streams.
    let camera = input.with_file_name("camera-timecode.mov");
    assert!(Command::new("ffmpeg")
        .args(["-v", "error", "-y", "-i"])
        .arg(&source)
        .args(["-map", "0", "-c", "copy", "-timecode", "00:00:00:00"])
        .arg(&camera)
        .status()
        .unwrap()
        .success());
    let camera_info = probe(&camera).unwrap();
    let issues = easycat_core::job::remux_issues(&camera_info, Container::Mkv);
    assert!(!issues.is_empty());
    assert!(issues
        .iter()
        .all(|issue| issue.kind == "data" && issue.removable));
    let cleaned = input.with_file_name("camera-clean.mkv");
    let mut job = Job {
        input: camera,
        output: cleaned.clone(),
        trim: None,
        mode: Mode::Remux(Container::Mkv),
    };
    assert!(ffmpeg::run(&job, &camera_info).is_err());
    assert!(!cleaned.exists());
    job.mode = Mode::RemuxSelected {
        container: Container::Mkv,
        excluded_tracks: issues.iter().map(|issue| issue.index).collect(),
    };
    ffmpeg::run(&job, &camera_info).unwrap();
    assert_eq!(probe(&cleaned).unwrap().tracks.len(), 3);
    assert_eq!(fingerprint(&cleaned), original_hash);
}

#[test]
fn stratified_samples_estimate_a_trimmed_export_more_accurately_than_the_heuristic() {
    let dir = std::env::temp_dir().join(format!("easycat-estimate-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let input = dir.join("source.mp4");
    let output = dir.join("output.mp4");
    assert!(Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=duration=48:size=640x360:rate=30",
            "-f",
            "lavfi",
            "-i",
            "sine=duration=48",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-crf",
            "18",
            "-c:a",
            "aac"
        ])
        .arg(&input)
        .status()
        .unwrap()
        .success());
    let info = probe(&input).unwrap();
    let settings = Settings {
        crop: None,
        codec: Codec::H264,
        container: Container::Mp4,
        height: None,
        rate: RateControl::Crf(28),
        hardware: false,
        speed: Speed::Balanced,
    };
    let trim = Some(Trim {
        start: 6.0,
        end: 42.0,
    });
    let job = Job {
        input: input.clone(),
        output: output.clone(),
        trim,
        mode: Mode::Transcode(settings),
    };
    let rough = estimate_bytes(&info, &job.mode, trim) as f64;
    let sampled = ffmpeg::measure_size(
        &input,
        &info,
        &settings,
        trim,
        &std::sync::atomic::AtomicBool::new(false),
    )
    .unwrap() as f64;
    ffmpeg::run(&job, &info).unwrap();
    let actual = std::fs::metadata(&output).unwrap().len() as f64;
    let sample_error = (sampled / actual - 1.0).abs();
    let rough_error = (rough / actual - 1.0).abs();
    eprintln!("36-second trim: rough={rough:.0}, sampled={sampled:.0}, actual={actual:.0}; rough error={:.1}%, sample error={:.1}%", rough_error * 100.0, sample_error * 100.0);
    assert!(sample_error < 0.15, "sample error={sample_error}");
    assert!(sample_error < rough_error);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn crop_and_trim_can_be_combined() {
    let Some(input) = clip("crop.mp4") else {
        eprintln!("FFmpeg unavailable: skipping test");
        return;
    };
    let info = probe(&input).unwrap();
    let crop = Crop::centered(info.width, info.height, 1, 1);
    let s = Settings {
        crop: Some(crop),
        codec: Codec::H264,
        container: Container::Mp4,
        height: None,
        rate: RateControl::Crf(28),
        hardware: false,
        speed: Speed::Fast,
    };
    let out = input.with_file_name("cropped.mp4");
    let job = Job {
        input: input.clone(),
        output: out.clone(),
        trim: Some(Trim {
            start: 2.0,
            end: 6.0,
        }),
        mode: Mode::Transcode(s),
    };
    ffmpeg::run(&job, &info).unwrap();
    let cropped = probe(&out).unwrap();
    assert_eq!((cropped.width, cropped.height), (720, 720));
    assert!((cropped.duration - 4.0).abs() < 0.5, "{}", cropped.duration);
}
