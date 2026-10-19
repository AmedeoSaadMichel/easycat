// File: easycat-core/src/ffmpeg/mod.rs
//! FFmpeg process boundary for export and measurement.
mod args;

pub use args::{backends, build_args, hardware_encoder, job_duration, Backend};

use crate::estimate::{from_samples, sample_plan};
use crate::job::{Job, RateControl, Settings, Trim};
use crate::probe::VideoInfo;
use crate::{Error, Result};
use std::collections::HashSet;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};

/// Keep bundled console tools invisible when called by the Windows GUI.
pub(crate) fn command(program: &str) -> Command {
    let mut command = Command::new(program);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    #[cfg(not(target_os = "windows"))]
    let _ = &mut command;
    command
}

pub(crate) fn run_capture_bytes(program: &'static str, args: &[String]) -> Result<Vec<u8>> {
    let out = command(program)
        .args(args)
        .output()
        .map_err(|source| Error::Spawn { program, source })?;
    if !out.status.success() {
        return Err(Error::Failed {
            program,
            status: out.status.to_string(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        });
    }
    Ok(out.stdout)
}

pub(crate) fn run_capture(program: &'static str, args: &[String]) -> Result<String> {
    run_capture_bytes(program, args).map(|b| String::from_utf8_lossy(&b).into_owned())
}

/// Encoders available in the installed FFmpeg build.
pub fn available_encoders() -> HashSet<String> {
    let args = ["-hide_banner".to_string(), "-encoders".to_string()];
    run_capture("ffmpeg", &args)
        .map(|out| {
            out.lines()
                .filter_map(|l| {
                    let mut it = l.split_whitespace();
                    let flags = it.next()?;
                    if flags.len() == 6 {
                        it.next().map(str::to_string)
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Run the job and return the backend actually used.
pub fn run(job: &Job, info: &VideoInfo) -> Result<Backend> {
    run_with_progress(job, info, &mut |_| {}, &AtomicBool::new(false))
}

/// Like `run`, with progress (0..1) and cancellation through `cancel`.
/// Try backends in order (full hardware, hardware encoder, software) until success.
/// Remove partial output on failure or cancellation.
pub fn run_with_progress(
    job: &Job,
    info: &VideoInfo,
    progress: &mut dyn FnMut(f64),
    cancel: &AtomicBool,
) -> Result<Backend> {
    // Validate before touching disk: a rejected job must never
    // delete anything (the output might be the input).
    build_args(job, info, Backend::Software)?;
    let mut first_err = None;
    for backend in backends(&job.mode, info) {
        progress(0.0);
        match run_once(job, info, backend, progress, cancel) {
            Ok(()) => return Ok(backend),
            Err(Error::Cancelled) => {
                first_err = Some(Error::Cancelled);
                break;
            }
            Err(e) => {
                first_err.get_or_insert(e);
            }
        }
    }
    let _ = std::fs::remove_file(&job.output);
    Err(first_err.unwrap_or(Error::Cancelled))
}

/// FFmpeg timestamps may briefly regress while reordered frames are flushed.
fn report_progress(value: f64, last: &mut f64, progress: &mut dyn FnMut(f64)) {
    if value.is_finite() {
        let value = value.clamp(0.0, 1.0);
        if value > *last {
            *last = value;
            progress(value);
        }
    }
}

fn run_once(
    job: &Job,
    info: &VideoInfo,
    backend: Backend,
    progress: &mut dyn FnMut(f64),
    cancel: &AtomicBool,
) -> Result<()> {
    let duration = job_duration(job, info);
    let mut args = build_args(job, info, backend)?;
    // Global options: key=value progress on stdout.
    args.splice(
        1..1,
        ["-progress".to_string(), "pipe:1".into(), "-nostats".into()],
    );
    let mut child = command("ffmpeg")
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|source| Error::Spawn {
            program: "ffmpeg",
            source,
        })?;
    // Drain stderr concurrently to avoid blocking FFmpeg on a full pipe.
    let mut stderr = child.stderr.take().expect("stderr is piped");
    let err_reader = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = stderr.read_to_string(&mut s);
        s
    });
    let stdout = child.stdout.take().expect("stdout is piped");
    let mut last_progress = 0.0;
    for line in BufReader::new(stdout).lines().map_while(|l| l.ok()) {
        if cancel.load(Ordering::Relaxed) {
            let _ = child.kill();
            break;
        }
        if let Some(us) = line
            .strip_prefix("out_time_us=")
            .and_then(|v| v.trim().parse::<f64>().ok())
        {
            if duration > 0.0 {
                report_progress(us / 1e6 / duration, &mut last_progress, progress);
            }
        }
    }
    let status = child.wait().map_err(|source| Error::Spawn {
        program: "ffmpeg",
        source,
    })?;
    let stderr = err_reader.join().unwrap_or_default();
    if cancel.load(Ordering::Relaxed) {
        return Err(Error::Cancelled);
    }
    if !status.success() {
        return Err(Error::Failed {
            program: "ffmpeg",
            status: status.to_string(),
            stderr,
        });
    }
    progress(1.0);
    Ok(())
}

/// Drain both pipes concurrently and check cancellation every 20 ms.
fn capture_sample(args: &[String], cancel: &AtomicBool) -> Result<u64> {
    if cancel.load(Ordering::Relaxed) {
        return Err(Error::Cancelled);
    }
    let mut child = command("ffmpeg")
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|source| Error::Spawn {
            program: "ffmpeg",
            source,
        })?;
    // Both pipes exist because they were configured with Stdio::piped().
    let mut stdout = child.stdout.take().expect("stdout is piped");
    let mut stderr = child.stderr.take().expect("stderr is piped");
    std::thread::scope(|scope| {
        let output = scope.spawn(move || std::io::copy(&mut stdout, &mut std::io::sink()));
        let errors = scope.spawn(move || {
            let mut text = String::new();
            let _ = stderr.read_to_string(&mut text);
            text
        });
        let status = loop {
            if cancel.load(Ordering::Relaxed) {
                let _ = child.kill();
                let _ = child.wait();
                return Err(Error::Cancelled);
            }
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => std::thread::sleep(std::time::Duration::from_millis(20)),
                Err(source) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(Error::Spawn {
                        program: "ffmpeg",
                        source,
                    });
                }
            }
        };
        let stderr = errors.join().unwrap_or_default();
        if cancel.load(Ordering::Relaxed) {
            return Err(Error::Cancelled);
        }
        if !status.success() {
            return Err(Error::Failed {
                program: "ffmpeg",
                status: status.to_string(),
                stderr,
            });
        }
        output
            .join()
            .unwrap_or_else(|_| Err(std::io::Error::other("sample reader interrupted")))
            .map_err(|source| Error::Spawn {
                program: "ffmpeg",
                source,
            })
    })
}

/// Bytes produced by encoding only the video of a sample window.
fn sample_bytes(
    input: &Path,
    info: &VideoInfo,
    (start, len): (f64, f64),
    s: &Settings,
    backend: Backend,
    cancel: &AtomicBool,
) -> Result<u64> {
    let mut a: Vec<String> = vec!["-v".into(), "error".into()];
    a.extend(args::input_args(backend));
    a.extend([
        "-ss".into(),
        start.to_string(),
        "-t".into(),
        len.to_string(),
    ]);
    a.extend(["-i".into(), input.display().to_string()]);
    if let Some(f) = args::filter(s, info, backend) {
        a.extend(["-vf".into(), f]);
    }
    a.extend(args::video_args(s, backend));
    a.extend(["-an", "-f", "matroska", "pipe:1"].map(String::from));
    capture_sample(&a, cancel)
}

/// Estimate output size by encoding sample windows within the selection,
/// using the same backend fallback chain as export.
pub fn measure_size(
    input: &Path,
    info: &VideoInfo,
    s: &Settings,
    trim: Option<Trim>,
    cancel: &AtomicBool,
) -> Result<u64> {
    if let RateControl::TargetBytes(b) = s.rate {
        return Ok(b);
    }
    let (range_start, secs) = trim.map_or((0.0, info.duration), |t| (t.start, t.duration()));
    let chain = backends(&crate::job::Mode::Transcode(*s), info);
    let plan = sample_plan(range_start, secs);
    let mut last_error = Error::Cancelled;
    for backend in chain {
        let mut samples = Vec::new();
        for &sample in &plan {
            if cancel.load(Ordering::Relaxed) {
                return Err(Error::Cancelled);
            }
            match sample_bytes(input, info, sample, s, backend, cancel) {
                Ok(bytes) => samples.push((bytes, sample.1)),
                Err(Error::Cancelled) => return Err(Error::Cancelled),
                Err(error) => {
                    last_error = error;
                    break;
                }
            }
        }
        // Every sample must use the same backend; never mix hardware/software bitrates.
        if samples.len() == plan.len() {
            return Ok(from_samples(&samples, info, s, secs));
        }
    }
    Err(last_error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job::Mode;

    #[test]
    fn reordered_timestamps_do_not_move_progress_backwards() {
        let mut last = 0.0;
        let mut reported = Vec::new();
        for value in [0.0, 0.4, 0.3, f64::NAN, 0.6, 0.59, 1.1] {
            report_progress(value, &mut last, &mut |value| reported.push(value));
        }
        assert_eq!(reported, vec![0.4, 0.6, 1.0]);
    }

    #[test]
    fn running_sample_cancels_without_waiting_for_completion() {
        let cancel = AtomicBool::new(false);
        let args: Vec<String> = [
            "-v",
            "error",
            "-re",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=64x64:rate=10",
            "-t",
            "10",
            "-an",
            "-c:v",
            "rawvideo",
            "-f",
            "nut",
            "pipe:1",
        ]
        .map(String::from)
        .to_vec();
        let start = std::time::Instant::now();
        std::thread::scope(|scope| {
            scope.spawn(|| {
                std::thread::sleep(std::time::Duration::from_millis(150));
                cancel.store(true, Ordering::Relaxed);
            });
            assert!(matches!(
                capture_sample(&args, &cancel),
                Err(Error::Cancelled)
            ));
        });
        assert!(start.elapsed() < std::time::Duration::from_secs(2));
    }

    #[test]
    fn rejected_same_input_output_does_not_delete_file() {
        let path = std::env::temp_dir().join(format!("easycat-test-{}.bin", std::process::id()));
        std::fs::write(&path, b"data").unwrap();
        let j = Job {
            input: path.clone(),
            output: path.clone(),
            trim: None,
            mode: Mode::Copy,
        };
        let info = VideoInfo {
            duration: 1.0,
            width: 2,
            height: 2,
            codec: String::new(),
            fps: 1.0,
            size_bytes: 4,
            bitrate: None,
            has_audio: false,
            tracks: Vec::new(),
            track_indices: Vec::new(),
            bit_depth: 8,
            rotation: 0,
        };
        assert!(run(&j, &info).is_err());
        assert!(path.exists());
        std::fs::remove_file(&path).unwrap();
    }
}
