// File: easycut-core/src/probe.rs
use crate::{ffmpeg, Error, Result};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct VideoInfo {
    pub duration: f64,
    /// Dimensioni già corrette per la rotazione.
    pub width: u32,
    pub height: u32,
    pub codec: String,
    pub fps: f64,
    pub size_bytes: u64,
    /// Bitrate totale del file in bit/s, se noto.
    pub bitrate: Option<u64>,
    pub has_audio: bool,
    /// Bit per componente del video (8, 10, 12…).
    pub bit_depth: u8,
    /// Rotazione di visualizzazione in gradi antiorari: 0, 90, 180 o 270.
    pub rotation: u16,
}

#[derive(Deserialize)]
struct Raw {
    streams: Vec<RawStream>,
    format: RawFormat,
}

#[derive(Deserialize)]
struct RawStream {
    codec_type: Option<String>,
    codec_name: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    r_frame_rate: Option<String>,
    pix_fmt: Option<String>,
    bits_per_raw_sample: Option<String>,
    tags: Option<RawTags>,
    side_data_list: Option<Vec<RawSideData>>,
}

#[derive(Deserialize)]
struct RawTags {
    rotate: Option<String>,
}

#[derive(Deserialize)]
struct RawSideData {
    rotation: Option<f64>,
}

#[derive(Deserialize)]
struct RawFormat {
    duration: Option<String>,
    size: Option<String>,
    bit_rate: Option<String>,
}

fn parse_fraction(s: &str) -> f64 {
    match s.split_once('/') {
        Some((n, d)) => {
            let (n, d): (f64, f64) = (n.parse().unwrap_or(0.0), d.parse().unwrap_or(1.0));
            if d == 0.0 {
                0.0
            } else {
                n / d
            }
        }
        None => s.parse().unwrap_or(0.0),
    }
}

/// Rotazione normalizzata (0/90/180/270, antioraria come la side data di FFmpeg).
fn rotation(s: &RawStream) -> u16 {
    let from_side = s
        .side_data_list
        .iter()
        .flatten()
        .filter_map(|d| d.rotation)
        .next();
    // Il vecchio tag `rotate` è in senso orario.
    let from_tag = s
        .tags
        .as_ref()
        .and_then(|t| t.rotate.as_ref())
        .and_then(|r| r.parse::<f64>().ok())
        .map(|r| -r);
    let deg = from_side.or(from_tag).unwrap_or(0.0);
    let quarter = (deg / 90.0).round() as i64;
    (quarter.rem_euclid(4) * 90) as u16
}

fn bit_depth(s: &RawStream) -> u8 {
    if let Some(b) = s
        .bits_per_raw_sample
        .as_deref()
        .and_then(|b| b.parse().ok())
    {
        return b;
    }
    let fmt = s.pix_fmt.as_deref().unwrap_or("");
    let core = fmt.trim_end_matches("le").trim_end_matches("be");
    if fmt.starts_with("p010") || core.ends_with("10") {
        10
    } else if fmt.starts_with("p016") || core.ends_with("12") && !fmt.starts_with("nv") {
        12
    } else {
        8
    }
}

pub fn parse_probe(json: &str) -> Result<VideoInfo> {
    let raw: Raw = serde_json::from_str(json).map_err(|e| Error::BadProbe(e.to_string()))?;
    let has_audio = raw
        .streams
        .iter()
        .any(|s| s.codec_type.as_deref() == Some("audio"));
    let video = raw
        .streams
        .into_iter()
        .find(|s| s.codec_type.as_deref() == Some("video"))
        .ok_or_else(|| Error::BadProbe("nessuno stream video".into()))?;
    let duration = raw
        .format
        .duration
        .and_then(|d| d.parse().ok())
        .ok_or_else(|| Error::BadProbe("durata mancante".into()))?;
    let (mut width, mut height) = (video.width.unwrap_or(0), video.height.unwrap_or(0));
    let rotation = rotation(&video);
    let bit_depth = bit_depth(&video);
    if rotation % 180 == 90 {
        std::mem::swap(&mut width, &mut height);
    }
    Ok(VideoInfo {
        duration,
        width,
        height,
        fps: video.r_frame_rate.as_deref().map_or(0.0, parse_fraction),
        codec: video.codec_name.unwrap_or_default(),
        size_bytes: raw.format.size.and_then(|s| s.parse().ok()).unwrap_or(0),
        bitrate: raw.format.bit_rate.and_then(|s| s.parse().ok()),
        has_audio,
        bit_depth,
        rotation,
    })
}

pub fn probe(input: &Path) -> Result<VideoInfo> {
    if !input.exists() {
        return Err(Error::MissingInput(input.to_path_buf()));
    }
    let args = [
        "-v".to_string(),
        "error".into(),
        "-print_format".into(),
        "json".into(),
        "-show_format".into(),
        "-show_streams".into(),
        input.display().to_string(),
    ];
    let out = ffmpeg::run_capture("ffprobe", &args)?;
    parse_probe(&out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legge_info_video() {
        let json = r#"{"streams":[{"codec_type":"audio"},{"codec_type":"video","codec_name":"h264","width":1920,"height":1080,"r_frame_rate":"30000/1001"}],"format":{"duration":"12.5","size":"1000","bit_rate":"640"}}"#;
        let info = parse_probe(json).unwrap();
        assert_eq!((info.width, info.height), (1920, 1080));
        assert_eq!(info.codec, "h264");
        assert_eq!(info.duration, 12.5);
        assert!((info.fps - 29.97).abs() < 0.01);
        assert_eq!(info.size_bytes, 1000);
        assert_eq!(info.bitrate, Some(640));
        assert!(info.has_audio);
    }

    #[test]
    fn rotazione_scambia_le_dimensioni() {
        let json = r#"{"streams":[{"codec_type":"video","codec_name":"h264","width":1920,"height":1080,"side_data_list":[{"rotation":-90}]}],"format":{"duration":"1"}}"#;
        let info = parse_probe(json).unwrap();
        assert_eq!((info.width, info.height), (1080, 1920));
        assert_eq!(info.rotation, 270);
        assert!(!info.has_audio);
    }

    #[test]
    fn rotazione_e_profondita_colore() {
        let s = |extra: &str| {
            format!(
                r#"{{"streams":[{{"codec_type":"video","width":4,"height":2{extra}}}],"format":{{"duration":"1"}}}}"#
            )
        };
        let i = parse_probe(&s(
            r#","pix_fmt":"yuv420p10le","side_data_list":[{"rotation":90}]"#,
        ))
        .unwrap();
        assert_eq!((i.bit_depth, i.rotation, i.width), (10, 90, 2));
        let i = parse_probe(&s(
            r#","pix_fmt":"nv12","side_data_list":[{"rotation":-180}]"#,
        ))
        .unwrap();
        assert_eq!((i.bit_depth, i.rotation, i.width), (8, 180, 4));
        // Il tag `rotate` è orario: 90 orario = 270 antiorario.
        let i = parse_probe(&s(r#","pix_fmt":"p010le","tags":{"rotate":"90"}"#)).unwrap();
        assert_eq!((i.bit_depth, i.rotation), (10, 270));
        let i = parse_probe(&s(r#","pix_fmt":"yuv420p","bits_per_raw_sample":"10""#)).unwrap();
        assert_eq!(i.bit_depth, 10);
    }

    #[test]
    fn senza_video_e_errore() {
        let json = r#"{"streams":[{"codec_type":"audio"}],"format":{"duration":"1"}}"#;
        assert!(parse_probe(json).is_err());
    }
}
