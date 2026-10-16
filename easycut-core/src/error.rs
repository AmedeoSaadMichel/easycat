// File: easycut-core/src/error.rs
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("impossibile avviare {program}: {source} (FFmpeg è installato?)")]
    Spawn {
        program: &'static str,
        source: std::io::Error,
    },
    #[error("{program} è fallito ({status}):\n{stderr}")]
    Failed {
        program: &'static str,
        status: String,
        stderr: String,
    },
    #[error("output di ffprobe non valido: {0}")]
    BadProbe(String),
    #[error("file di origine non trovato: {0}")]
    MissingInput(PathBuf),
    #[error("il file di output coincide con l'origine: {0}")]
    OutputIsInput(PathBuf),
    #[error("intervallo di taglio non valido: {0}")]
    BadRange(String),
    #[error("annullato")]
    Cancelled,
}

pub type Result<T> = std::result::Result<T, Error>;
