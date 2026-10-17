// File: easycut-core/src/presets.rs
use crate::job::{Codec, Container, RateControl, Settings, Speed};
use crate::probe::VideoInfo;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct Preset {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub settings: Settings,
}

fn p(
    id: &'static str,
    name: &'static str,
    description: &'static str,
    codec: Codec,
    container: Container,
    height: Option<u32>,
    crf: u8,
) -> Preset {
    Preset {
        id,
        name,
        description,
        settings: Settings {
            codec,
            container,
            height,
            rate: RateControl::Crf(crf),
            hardware: false,
            speed: Speed::Balanced,
        },
    }
}

/// Profiles available for this video and the installed encoders.
/// Never upscale beyond the source resolution.
pub fn builtin(info: &VideoInfo, encoders: &HashSet<String>) -> Vec<Preset> {
    use Codec::*;
    use Container::*;
    let all = [
        p(
            "compat",
            "Compatible",
            "Works everywhere",
            H264,
            Mp4,
            None,
            23,
        ),
        p(
            "compact",
            "Compact",
            "Good quality-to-size balance",
            H265,
            Mp4,
            None,
            28,
        ),
        p(
            "compact-1080",
            "Compact 1080p",
            "Full HD, smaller files",
            H265,
            Mp4,
            Some(1080),
            28,
        ),
        p(
            "compact-720",
            "Compact 720p",
            "HD, very small files",
            H265,
            Mp4,
            Some(720),
            28,
        ),
        p(
            "web-720",
            "Web 720p",
            "WebM for websites and browsers",
            Vp9,
            Webm,
            Some(720),
            33,
        ),
        p(
            "tiny-480",
            "Messages 480p",
            "For messaging and email",
            H264,
            Mp4,
            Some(480),
            28,
        ),
        p(
            "min-720",
            "Maximum compression",
            "AV1, smallest files",
            Av1,
            Mp4,
            Some(720),
            38,
        ),
    ];
    all.into_iter()
        .filter(|pr| encoders.contains(pr.settings.codec.software_encoder()))
        .filter(|pr| pr.settings.height.is_none_or(|h| h < info.height))
        .collect()
}
