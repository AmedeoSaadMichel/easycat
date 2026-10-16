// File: easycut-core/src/lib.rs
pub mod error;
pub mod estimate;
pub mod ffmpeg;
pub mod frames;
pub mod job;
pub mod playback;
pub mod presets;
pub mod probe;

pub use error::{Error, Result};
pub use job::{Codec, Container, Job, Mode, RateControl, Settings, Speed, Trim};
pub use probe::{probe, VideoInfo};
