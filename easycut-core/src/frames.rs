// File: easycut-core/src/frames.rs
//! Estrazione di singoli fotogrammi (RGB grezzo) per anteprima e timeline.
use crate::{ffmpeg, Error, Result};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Frame {
    pub width: usize,
    pub height: usize,
    /// RGB, 3 byte per pixel.
    pub rgb: Vec<u8>,
}

/// Dimensioni (pari) di una miniatura alta `height`, con le proporzioni dell'origine.
pub fn thumb_size(src_w: u32, src_h: u32, height: u32) -> (usize, usize) {
    let h = (height as usize).max(2) & !1;
    let w = ((src_w as f64 / src_h.max(1) as f64) * h as f64).round() as usize;
    ((w.max(2)) & !1, h)
}

/// Il fotogramma al secondo `t`, scalato a `width`x`height`.
pub fn frame_at(input: &Path, t: f64, width: usize, height: usize) -> Result<Frame> {
    let args = [
        "-v".to_string(),
        "error".into(),
        "-ss".into(),
        format!("{:.3}", t.max(0.0)),
        "-i".into(),
        input.display().to_string(),
        "-frames:v".into(),
        "1".into(),
        "-vf".into(),
        format!("scale={width}:{height}"),
        "-f".into(),
        "rawvideo".into(),
        "-pix_fmt".into(),
        "rgb24".into(),
        "pipe:1".into(),
    ];
    let rgb = ffmpeg::run_capture_bytes("ffmpeg", &args)?;
    if rgb.len() != width * height * 3 {
        return Err(Error::BadProbe(format!(
            "fotogramma di {} byte, attesi {}",
            rgb.len(),
            width * height * 3
        )));
    }
    Ok(Frame { width, height, rgb })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn miniatura_mantiene_proporzioni_con_dimensioni_pari() {
        assert_eq!(thumb_size(1920, 1080, 90), (160, 90));
        assert_eq!(thumb_size(1080, 1920, 90), (50, 90));
    }
}
