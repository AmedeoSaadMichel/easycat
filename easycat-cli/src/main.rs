// File: easycat-cli/src/main.rs
use clap::{Parser, Subcommand, ValueEnum};
use easycat_core::{
    ffmpeg, probe, Codec, Container, Job, Mode, RateControl, Settings, Speed, Trim,
};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "easycat", about = "Trim and compress videos locally")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Clone, ValueEnum)]
enum CodecArg {
    H264,
    H265,
    Vp9,
    Av1,
}

#[derive(Clone, ValueEnum)]
enum SpeedArg {
    Fast,
    Balanced,
    Small,
}

#[derive(Subcommand)]
enum Cmd {
    /// Show duration, resolution and codec
    Info { input: PathBuf },
    /// Trim without re-encoding (keyframe precision)
    Cut {
        input: PathBuf,
        output: PathBuf,
        #[arg(long)]
        start: f64,
        #[arg(long)]
        end: f64,
    },
    /// Change container without re-encoding; preserve all tracks by default
    Remux {
        input: PathBuf,
        output: PathBuf,
        /// Explicitly omit an incompatible optional stream (repeat for multiple indices)
        #[arg(long = "exclude-track", value_name = "INDEX")]
        excluded_tracks: Vec<u32>,
    },
    /// Transcode / compress / resize (container inferred from output extension)
    Compress {
        input: PathBuf,
        output: PathBuf,
        #[arg(long, value_enum, default_value = "h264")]
        codec: CodecArg,
        #[arg(long)]
        height: Option<u32>,
        #[arg(long, default_value_t = 23)]
        crf: u8,
        /// Requested output size in MB (instead of --crf)
        #[arg(long)]
        target_mb: Option<f64>,
        /// Disable hardware encoding
        #[arg(long)]
        software: bool,
        /// Encoding speed/file size trade-off
        #[arg(long, value_enum, default_value = "balanced")]
        speed: SpeedArg,
        #[arg(long)]
        start: Option<f64>,
        #[arg(long)]
        end: Option<f64>,
    },
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn container_of(path: &Path) -> Container {
    match path.extension().and_then(|e| e.to_str()) {
        Some("webm") => Container::Webm,
        Some("mkv") => Container::Mkv,
        _ => Container::Mp4,
    }
}

fn run(cli: Cli) -> easycat_core::Result<()> {
    match cli.cmd {
        Cmd::Info { input } => {
            let i = probe(&input)?;
            println!(
                "{}x{} {} {:.2}s {:.1}fps {:.1} MB",
                i.width,
                i.height,
                i.codec,
                i.duration,
                i.fps,
                i.size_bytes as f64 / 1e6
            );
            Ok(())
        }
        Cmd::Cut {
            input,
            output,
            start,
            end,
        } => {
            let info = probe(&input)?;
            let job = Job {
                input,
                output,
                trim: Some(Trim { start, end }),
                mode: Mode::Copy,
            };
            ffmpeg::run(&job, &info).map(|_| ())
        }
        Cmd::Remux {
            input,
            output,
            excluded_tracks,
        } => {
            let container = match output
                .extension()
                .and_then(|e| e.to_str())
                .map(str::to_ascii_lowercase)
                .as_deref()
            {
                Some("mp4") => Container::Mp4,
                Some("mkv") => Container::Mkv,
                Some("webm") => Container::Webm,
                _ => {
                    return Err(easycat_core::Error::IncompatibleTracks(
                        "choose an .mp4, .mkv or .webm output".into(),
                    ))
                }
            };
            let info = probe(&input)?;
            ffmpeg::run(
                &Job {
                    input,
                    output,
                    trim: None,
                    mode: if excluded_tracks.is_empty() {
                        Mode::Remux(container)
                    } else {
                        Mode::RemuxSelected {
                            container,
                            excluded_tracks,
                        }
                    },
                },
                &info,
            )
            .map(|_| ())
        }
        Cmd::Compress {
            input,
            output,
            codec,
            height,
            crf,
            target_mb,
            software,
            speed,
            start,
            end,
        } => {
            let info = probe(&input)?;
            let trim = match (start, end) {
                (None, None) => None,
                (s, e) => Some(Trim {
                    start: s.unwrap_or(0.0),
                    end: e.unwrap_or(info.duration),
                }),
            };
            let codec = match codec {
                CodecArg::H264 => Codec::H264,
                CodecArg::H265 => Codec::H265,
                CodecArg::Vp9 => Codec::Vp9,
                CodecArg::Av1 => Codec::Av1,
            };
            let rate = match target_mb {
                Some(mb) => RateControl::TargetBytes((mb * 1e6) as u64),
                None => RateControl::Crf(crf),
            };
            let container = container_of(&output);
            let settings = Settings {
                crop: None,
                codec,
                container,
                height,
                rate,
                hardware: !software,
                speed: match speed {
                    SpeedArg::Fast => Speed::Fast,
                    SpeedArg::Balanced => Speed::Balanced,
                    SpeedArg::Small => Speed::Small,
                },
            };
            let job = Job {
                input,
                output,
                trim,
                mode: Mode::Transcode(settings),
            };
            let backend = ffmpeg::run(&job, &info)?;
            if !software
                && backend == ffmpeg::Backend::Software
                && ffmpeg::hardware_encoder(codec).is_some()
            {
                eprintln!("warning: hardware encoding unavailable; using software");
            }
            Ok(())
        }
    }
}
