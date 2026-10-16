// File: easycut-core/src/ffmpeg/mod.rs
//! Unico modulo che sa come invocare FFmpeg per esportare e misurare.
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

pub(crate) fn run_capture_bytes(program: &'static str, args: &[String]) -> Result<Vec<u8>> {
    let out = Command::new(program)
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

/// Encoder disponibili nell'installazione di FFmpeg.
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

/// Esegue il job e ritorna il backend effettivamente usato.
pub fn run(job: &Job, info: &VideoInfo) -> Result<Backend> {
    run_with_progress(job, info, &mut |_| {}, &AtomicBool::new(false))
}

/// Come `run`, ma riporta l'avanzamento (0..1) e può essere annullato con `cancel`.
/// Prova i backend in ordine (chip, encoder hardware, software) finché uno riesce.
/// In caso di errore o annullamento rimuove l'output parziale.
pub fn run_with_progress(
    job: &Job,
    info: &VideoInfo,
    progress: &mut dyn FnMut(f64),
    cancel: &AtomicBool,
) -> Result<Backend> {
    // Validazione prima di toccare il disco: un job rifiutato non deve mai
    // cancellare nulla (l'output potrebbe coincidere con l'origine).
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

fn run_once(
    job: &Job,
    info: &VideoInfo,
    backend: Backend,
    progress: &mut dyn FnMut(f64),
    cancel: &AtomicBool,
) -> Result<()> {
    let duration = job_duration(job, info);
    let mut args = build_args(job, info, backend)?;
    // Opzioni globali: avanzamento in formato chiave=valore su stdout.
    args.splice(
        1..1,
        ["-progress".to_string(), "pipe:1".into(), "-nostats".into()],
    );
    let mut child = Command::new("ffmpeg")
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|source| Error::Spawn {
            program: "ffmpeg",
            source,
        })?;
    // stderr va svuotato in parallelo, altrimenti FFmpeg si blocca a buffer pieno.
    let mut stderr = child.stderr.take().expect("stderr è piped");
    let err_reader = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = stderr.read_to_string(&mut s);
        s
    });
    let stdout = child.stdout.take().expect("stdout è piped");
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
                progress((us / 1e6 / duration).clamp(0.0, 1.0));
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

/// Byte prodotti codificando solo il video di un breve tratto.
fn sample_bytes(
    input: &Path,
    info: &VideoInfo,
    (start, len): (f64, f64),
    s: &Settings,
    backend: Backend,
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
    run_capture_bytes("ffmpeg", &a).map(|b| b.len() as u64)
}

/// Misura la dimensione finale codificando alcuni campioni del tratto scelto,
/// con la stessa catena di backend dell'esportazione.
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
    let mut samples = Vec::new();
    for sample in sample_plan(range_start, secs) {
        if cancel.load(Ordering::Relaxed) {
            return Err(Error::Cancelled);
        }
        let mut result = Err(Error::Cancelled);
        for &b in &chain {
            result = sample_bytes(input, info, sample, s, b);
            if result.is_ok() {
                break;
            }
        }
        samples.push((result?, sample.1));
    }
    Ok(from_samples(&samples, info, s, secs))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job::Mode;

    #[test]
    fn run_con_output_uguale_input_non_cancella_il_file() {
        let path = std::env::temp_dir().join(format!("easycut-test-{}.bin", std::process::id()));
        std::fs::write(&path, b"dati").unwrap();
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
            bit_depth: 8,
            rotation: 0,
        };
        assert!(run(&j, &info).is_err());
        assert!(path.exists());
        std::fs::remove_file(&path).unwrap();
    }
}
