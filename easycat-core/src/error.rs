// File: easycat-core/src/error.rs
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("could not start {program}: {source} (is FFmpeg installed?)")]
    Spawn {
        program: &'static str,
        source: std::io::Error,
    },
    #[error("{program} failed ({status}):\n{stderr}")]
    Failed {
        program: &'static str,
        status: String,
        stderr: String,
    },
    #[error("invalid ffprobe output: {0}")]
    BadProbe(String),
    #[error("source file not found: {0}")]
    MissingInput(PathBuf),
    #[error("output file is the source file: {0}")]
    OutputIsInput(PathBuf),
    #[error("invalid trim range: {0}")]
    BadRange(String),
    #[error("invalid crop: {0}")]
    BadCrop(String),
    #[error("{0}")]
    IncompatibleTracks(String),
    #[error("cancelled")]
    Cancelled,
}

pub type Result<T> = std::result::Result<T, Error>;
