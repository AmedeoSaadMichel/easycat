// File: easycut-core/src/playback.rs
//! Preview playback: streamed frames and audio through ffplay.
//! Along with `ffmpeg` and `frames`, this module starts FFmpeg processes.
use crate::frames::Frame;
use crate::{Error, Result};
use std::io::Read;
use std::path::Path;
use std::process::{Child, ChildStdout, Stdio};

/// RGB frame stream at constant `fps`, starting at `start`.
/// Dropping the stream terminates FFmpeg.
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
        let mut child = crate::ffmpeg::command("ffmpeg")
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
        let stdout = child.stdout.take().expect("stdout is piped");
        Ok(FrameStream {
            child,
            stdout,
            width,
            height,
        })
    }

    /// Next frame, or `None` at end of stream.
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

/// Video audio through windowless ffplay; stops on drop.
pub struct AudioPlayer {
    child: Child,
}

impl AudioPlayer {
    /// Return `None` if ffplay is unavailable; playback remains muted.
    pub fn spawn(input: &Path, start: f64, duration: f64, volume: u8) -> Option<Self> {
        let child = crate::ffmpeg::command("ffplay")
            .args(["-nodisp", "-autoexit", "-vn", "-loglevel", "quiet", "-ss"])
            .arg(format!("{:.3}", start.max(0.0)))
            .args(["-t", &format!("{duration:.3}")])
            .args(["-volume", &volume.min(100).to_string()])
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
