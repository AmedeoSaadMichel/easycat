// File: easycut-core/src/playback.rs
//! Riproduzione per l'anteprima: fotogrammi in streaming e audio con ffplay.
//! Insieme a `ffmpeg` e `frames` è l'unico punto che avvia processi FFmpeg.
use crate::frames::Frame;
use crate::{Error, Result};
use std::io::Read;
use std::path::Path;
use std::process::{Child, ChildStdout, Command, Stdio};

/// Flusso di fotogrammi RGB a `fps` costanti, a partire da `start`.
/// Alla distruzione termina il processo FFmpeg.
pub struct FrameStream {
    child: Child,
    stdout: ChildStdout,
    width: usize,
    height: usize,
}

impl FrameStream {
    pub fn open(
        input: &Path,
        start: f64,
        duration: f64,
        fps: f64,
        width: usize,
        height: usize,
    ) -> Result<Self> {
        let mut child = Command::new("ffmpeg")
            .args(["-v", "error", "-ss"])
            .arg(format!("{:.3}", start.max(0.0)))
            .arg("-i")
            .arg(input)
            .args(["-t", &format!("{duration:.3}"), "-an", "-vf"])
            .arg(format!("fps={fps:.3},scale={width}:{height}"))
            .args(["-f", "rawvideo", "-pix_fmt", "rgb24", "pipe:1"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|source| Error::Spawn {
                program: "ffmpeg",
                source,
            })?;
        let stdout = child.stdout.take().expect("stdout è piped");
        Ok(FrameStream {
            child,
            stdout,
            width,
            height,
        })
    }

    /// Il prossimo fotogramma, o `None` a fine flusso.
    pub fn next_frame(&mut self) -> Option<Frame> {
        let mut rgb = vec![0u8; self.width * self.height * 3];
        self.stdout.read_exact(&mut rgb).ok()?;
        Some(Frame {
            width: self.width,
            height: self.height,
            rgb,
        })
    }
}

impl Drop for FrameStream {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Audio del video riprodotto da ffplay (senza finestra). Si ferma alla distruzione.
pub struct AudioPlayer {
    child: Child,
}

impl AudioPlayer {
    /// `None` se ffplay non è disponibile: la riproduzione resta muta.
    pub fn spawn(input: &Path, start: f64, duration: f64) -> Option<Self> {
        let child = Command::new("ffplay")
            .args(["-nodisp", "-autoexit", "-vn", "-loglevel", "quiet", "-ss"])
            .arg(format!("{:.3}", start.max(0.0)))
            .args(["-t", &format!("{duration:.3}")])
            .arg(input)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        Some(AudioPlayer { child })
    }
}

impl Drop for AudioPlayer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
