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

/// Preset disponibili per questo video e per gli encoder installati.
/// Le risoluzioni non vengono mai ingrandite oltre l'origine.
pub fn builtin(info: &VideoInfo, encoders: &HashSet<String>) -> Vec<Preset> {
    use Codec::*;
    use Container::*;
    let all = [
        p(
            "compat",
            "Compatibile",
            "Funziona ovunque",
            H264,
            Mp4,
            None,
            23,
        ),
        p(
            "compact",
            "Compatto",
            "Ottimo rapporto qualità/peso",
            H265,
            Mp4,
            None,
            28,
        ),
        p(
            "compact-1080",
            "Compatto 1080p",
            "Full HD, molto leggero",
            H265,
            Mp4,
            Some(1080),
            28,
        ),
        p(
            "compact-720",
            "Compatto 720p",
            "HD, leggerissimo",
            H265,
            Mp4,
            Some(720),
            28,
        ),
        p(
            "web-720",
            "Web 720p",
            "WebM per siti e browser",
            Vp9,
            Webm,
            Some(720),
            33,
        ),
        p(
            "tiny-480",
            "Messaggi 480p",
            "Per chat e email",
            H264,
            Mp4,
            Some(480),
            28,
        ),
        p(
            "min-720",
            "Massima compressione",
            "AV1, il più piccolo",
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
