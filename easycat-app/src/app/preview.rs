// File: easycat-app/src/app/preview.rs
//! Headless UI verification and previews without personal videos.
use super::*;
use crate::{theme, video::Choice};
use eframe::{
    egui::{
        self,
        epaint::{ImageData, Primitive},
        Color32, Pos2, Rect, TextureId,
    },
    icon_data::IconDataExt,
};
use std::collections::HashMap;

fn apply_textures(textures: &mut HashMap<TextureId, egui::ColorImage>, output: &egui::FullOutput) {
    for (id, delta) in &output.textures_delta.set {
        let image = match &delta.image {
            ImageData::Color(image) => (**image).clone(),
            ImageData::Font(image) => egui::ColorImage {
                size: image.size,
                pixels: image.srgba_pixels(None).collect(),
            },
        };
        if let Some([x, y]) = delta.pos {
            let original = textures.get_mut(id).unwrap();
            for row in 0..image.size[1] {
                let start = (y + row) * original.size[0] + x;
                original.pixels[start..start + image.size[0]]
                    .copy_from_slice(&image.pixels[row * image.size[0]..(row + 1) * image.size[0]]);
            }
        } else {
            textures.insert(*id, image);
        }
    }
}

// Software rasterizer for verification previews only; no extra dependencies.
fn screenshot(
    ctx: &egui::Context,
    output: egui::FullOutput,
    textures: &HashMap<TextureId, egui::ColorImage>,
    width: usize,
    height: usize,
    name: &str,
) {
    let mut pixels = vec![theme::BG; width * height];
    for clipped in ctx.tessellate(output.shapes, 1.0) {
        let Primitive::Mesh(mesh) = clipped.primitive else {
            continue;
        };
        let texture = &textures[&mesh.texture_id];
        for triangle in mesh.indices.as_chunks::<3>().0 {
            let v = [
                mesh.vertices[triangle[0] as usize],
                mesh.vertices[triangle[1] as usize],
                mesh.vertices[triangle[2] as usize],
            ];
            let edge =
                |a: Pos2, b: Pos2, p: Pos2| (p.x - a.x) * (b.y - a.y) - (p.y - a.y) * (b.x - a.x);
            let area = edge(v[0].pos, v[1].pos, v[2].pos);
            if area.abs() < 0.0001 {
                continue;
            }
            let bounds = Rect::from_min_max(
                egui::pos2(
                    v.iter().map(|v| v.pos.x).fold(f32::INFINITY, f32::min),
                    v.iter().map(|v| v.pos.y).fold(f32::INFINITY, f32::min),
                ),
                egui::pos2(
                    v.iter().map(|v| v.pos.x).fold(f32::NEG_INFINITY, f32::max),
                    v.iter().map(|v| v.pos.y).fold(f32::NEG_INFINITY, f32::max),
                ),
            )
            .intersect(clipped.clip_rect)
            .intersect(Rect::from_min_size(
                Pos2::ZERO,
                egui::vec2(width as f32, height as f32),
            ));
            for y in bounds.top().max(0.0) as usize..bounds.bottom().ceil().max(0.0) as usize {
                for x in bounds.left().max(0.0) as usize..bounds.right().ceil().max(0.0) as usize {
                    if x >= width || y >= height {
                        continue;
                    }
                    let p = egui::pos2(x as f32 + 0.5, y as f32 + 0.5);
                    let w = [
                        edge(v[1].pos, v[2].pos, p) / area,
                        edge(v[2].pos, v[0].pos, p) / area,
                        edge(v[0].pos, v[1].pos, p) / area,
                    ];
                    if w.iter().any(|w| *w < -0.0001) {
                        continue;
                    }
                    let u = (0..3).map(|i| v[i].uv.x * w[i]).sum::<f32>();
                    let t = (0..3).map(|i| v[i].uv.y * w[i]).sum::<f32>();
                    let tx = ((u * texture.size[0] as f32) as usize).min(texture.size[0] - 1);
                    let ty = ((t * texture.size[1] as f32) as usize).min(texture.size[1] - 1);
                    let sample = texture.pixels[ty * texture.size[0] + tx].to_array();
                    let mut color = [0.0; 4];
                    for c in 0..4 {
                        color[c] = (0..3)
                            .map(|i| v[i].color.to_array()[c] as f32 * w[i])
                            .sum::<f32>()
                            * sample[c] as f32
                            / 255.0;
                    }
                    let dst = pixels[y * width + x].to_array();
                    let mut blended = [255; 4];
                    for c in 0..3 {
                        blended[c] = (color[c] + dst[c] as f32 * (1.0 - color[3] / 255.0))
                            .clamp(0.0, 255.0) as u8;
                    }
                    pixels[y * width + x] =
                        Color32::from_rgba_premultiplied(blended[0], blended[1], blended[2], 255);
                }
            }
        }
    }
    let image = egui::IconData {
        width: width as u32,
        height: height as u32,
        rgba: pixels.iter().flat_map(|p| p.to_array()).collect(),
    };
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/ui-preview");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join(format!("{name}.png")),
        image.to_png_bytes().unwrap(),
    )
    .unwrap();
}

#[test]
fn easycat_layout_keeps_export_accessible() {
    for (width, height, loaded, exporting, name) in [
        (1280, 840, true, false, "easycat"),
        (1180, 850, true, false, "easycat-minimum"),
        (1280, 840, true, false, "easycat-switch-operation"),
        (1013, 768, true, false, "easycat-portrait"),
        (1280, 900, true, false, "easycat-remux-blocked"),
        (1280, 900, true, false, "easycat-remux-ready"),
        (720, 480, true, false, "easycat-small"),
        (900, 640, true, false, "easycat-default"),
        (720, 480, true, false, "easycat-small-target"),
        (720, 480, true, false, "easycat-small-copy"),
        (1280, 840, true, false, "easycat-crop"),
        (720, 480, true, false, "easycat-small-crop"),
        (720, 480, true, true, "easycat-export"),
        (720, 480, true, false, "easycat-refining"),
        (1280, 840, false, false, "easycat-empty"),
    ] {
        let ctx = egui::Context::default();
        theme::install(&ctx);
        let mut app = App::new(None, &ctx);
        if loaded {
            let mut info = easycat_core::probe::parse_probe(r#"{"streams":[{"codec_type":"video","codec_name":"h264","width":3840,"height":2160,"r_frame_rate":"30/1"},{"codec_type":"audio","codec_name":"aac"}],"format":{"duration":"42","size":"52000000","bit_rate":"9900000"}}"#).unwrap();
            let portrait = name == "easycat-portrait";
            if portrait {
                (info.width, info.height) = (2160, 3840);
            }
            let presets =
                presets::builtin(&info, &HashSet::from(["libx264".into(), "libx265".into()]));
            let (tx, _rx) = channel();
            let mut video = Video::new("viaggio_cosmico.mp4".into(), info, presets, tx, true);
            video.start = 5.0;
            video.end = 35.0;
            video.playhead = 12.0;
            video.choice = Choice::Preset(2);
            if name.ends_with("target") {
                video.set_choice(Choice::Target);
            }
            if name.ends_with("crop") {
                video.crop_on = true;
            }
            if name.ends_with("copy") {
                video.set_choice(Choice::Copy);
            }
            if name.starts_with("easycat-remux-") {
                video.info.tracks.push(("data".into(), "tmcd".into()));
                video.info.track_indices.push(2);
                video.set_choice(Choice::Remux(easycat_core::Container::Mkv));
                if name.ends_with("ready") {
                    video.remux_excluded.push(2);
                }
            }
            let image = eframe::icon_data::from_png_bytes(include_bytes!(
                "../../assets/approved-mockup.png"
            ))
            .unwrap();
            let (w, h) = if portrait { (144, 256) } else { (320, 180) };
            let mut pixels = Vec::new();
            for y in 0..h {
                for x in 0..w {
                    let sx = if portrait {
                        420 + x * 262 / w
                    } else {
                        28 + x * 1058 / w
                    };
                    let sy = 182 + y * 466 / h;
                    let i = (sy * image.width as usize + sx) * 4;
                    pixels.push(Color32::from_rgb(
                        image.rgba[i],
                        image.rgba[i + 1],
                        image.rgba[i + 2],
                    ));
                }
            }
            let texture = ctx.load_texture(
                "demo-video",
                egui::ColorImage {
                    size: [w, h],
                    pixels,
                },
                egui::TextureOptions::LINEAR,
            );
            video.preview = Some(texture.clone());
            video.thumbs = vec![Some(texture); 20];
            app.video = Some(video);
        }
        app.exporting = exporting;
        app.progress = 0.42;
        if name == "easycat-refining" {
            app.measure = Some(Measure {
                id: 1,
                stop: Arc::new(AtomicBool::new(false)),
                done: 0,
                total: 1,
            });
        }
        let mut textures = HashMap::new();
        let mut last = None;
        for pass in 0..3 {
            if name == "easycat-switch-operation" {
                let video = app.video.as_mut().unwrap();
                video.set_choice(if pass == 0 {
                    Choice::Remux(easycat_core::Container::Mkv)
                } else {
                    Choice::Preset(0)
                });
            }
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(
                        Pos2::ZERO,
                        egui::vec2(width as f32, height as f32),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    app.top_bar(ctx);
                    app.bottom_bar(ctx);
                    app.export_panel(ctx);
                    app.center(ctx);
                },
            );
            apply_textures(&mut textures, &output);
            if pass == 2 {
                let header = Rect::from_min_size(Pos2::ZERO, egui::vec2(width as f32, 88.0));
                assert!(
                    output.shapes.iter().any(|shape| {
                        if let egui::Shape::Mesh(mesh) = &shape.shape {
                            mesh.texture_id == app.brand_art.as_ref().unwrap().id()
                                && shape.clip_rect.contains_rect(header)
                                && mesh.calc_bounds().contains_rect(header)
                        } else {
                            false
                        }
                    }),
                    "header artwork must cover the full panel without an empty strip: {name}"
                );
            }
            if loaded && pass == 2 {
                assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if ["Start here", "End here", "Reset"].contains(&text.galley.text()))));
                let illustration = theme::art_uv(28.0, 182.0, 1058.0, 466.0);
                let background = output.shapes.iter().any(|shape| {
                    matches!(&shape.shape, egui::Shape::Mesh(mesh) if Some(mesh.texture_id) == app.header.as_ref().map(|t| t.id()) && !mesh.vertices.is_empty() && mesh.vertices.iter().all(|v| illustration.expand(0.001).contains(egui::pos2(v.uv.x, v.uv.y))))
                });
                assert_eq!(
                    background,
                    name == "easycat-portrait",
                    "side background: {name}"
                );
                assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text() == "What would you like to do?" && shape.clip_rect.contains_rect(Rect::from_min_size(text.pos, text.galley.size())))), "heading is not visible: {name}");
                if !exporting {
                    for label in ["START", "END"] {
                        assert!(
                            output.shapes.iter().any(|shape| {
                                if let egui::Shape::Text(text) = &shape.shape {
                                    let bounds = Rect::from_min_size(text.pos, text.galley.size());
                                    text.galley.text() == label
                                        && shape.clip_rect.contains_rect(bounds)
                                        && bounds.bottom() <= height as f32 - 64.0
                                } else {
                                    false
                                }
                            }),
                            "trim control is not visible without scrolling: {label}, {name}"
                        );
                    }
                }
                assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text() == "Estimate options")), "estimate options must not take space in the settings panel: {name}");
                if !name.starts_with("easycat-remux-") {
                    let mut hardware_bottom = None;
                    let mut estimate_top = None;
                    for shape in &output.shapes {
                        if let egui::Shape::Text(text) = &shape.shape {
                            let bounds = Rect::from_min_size(text.pos, text.galley.size());
                            if text.galley.text() == "Hardware acceleration" {
                                assert!(
                                    shape.clip_rect.contains_rect(bounds),
                                    "hardware control clipped: {name}"
                                );
                                hardware_bottom = Some(bounds.bottom());
                            }
                            if [
                                "Rough estimate",
                                "Requested size",
                                "Refining estimate…",
                                "Sample-based estimate",
                            ]
                            .contains(&text.galley.text())
                            {
                                estimate_top = Some(bounds.top());
                            }
                        }
                    }
                    if let (Some(bottom), Some(top)) = (hardware_bottom, estimate_top) {
                        assert!(
                            bottom <= top,
                            "settings overlap the estimate: {name}, {bottom} > {top}"
                        );
                    }
                }
                let expected = if exporting {
                    "Cancel export"
                } else {
                    "Export video"
                };
                assert!(
                    output.shapes.iter().any(|shape| {
                        if let egui::Shape::Text(text) = &shape.shape {
                            let bounds = Rect::from_min_size(text.pos, text.galley.size());
                            text.galley.text() == expected
                                && bounds.left() >= 0.0
                                && bounds.right() <= width as f32
                                && bounds.top() >= 0.0
                                && bounds.bottom() <= height as f32
                                && shape.clip_rect.contains_rect(bounds)
                        } else {
                            false
                        }
                    }),
                    "button is not visible: {name}"
                );
            }
            last = Some(output);
        }
        screenshot(&ctx, last.unwrap(), &textures, width, height, name);
    }
}

#[test]
fn fullscreen_preserves_selection_and_exits_even_without_video() {
    let ctx = egui::Context::default();
    theme::install(&ctx);
    let mut app = App::new(None, &ctx);
    let info = easycat_core::probe::parse_probe(r#"{"streams":[{"codec_type":"video","codec_name":"h264","width":1080,"height":1920,"r_frame_rate":"30/1"}],"format":{"duration":"42"}}"#).unwrap();
    let (tx, _rx) = channel();
    let mut video = Video::new("portrait.mp4".into(), info, vec![], tx, false);
    video.start = 5.0;
    video.end = 35.0;
    video.playhead = 12.0;
    video.preview = Some(ctx.load_texture(
        "portrait-fixture",
        egui::ColorImage::new([90, 160], theme::CORAL),
        egui::TextureOptions::LINEAR,
    ));
    app.video = Some(video);
    let mut textures = HashMap::new();
    let input = || egui::RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1280.0, 840.0))),
        ..Default::default()
    };
    let output = ctx.run(input(), |ctx| app.apply(ctx, Action::ToggleFullscreen));
    apply_textures(&mut textures, &output);
    assert!(app.preview_fullscreen);
    assert!(output.viewport_output[&egui::ViewportId::ROOT]
        .commands
        .iter()
        .any(|command| matches!(command, egui::ViewportCommand::Fullscreen(true))));
    let mut last = None;
    for _ in 0..3 {
        let output = ctx.run(input(), |ctx| {
            app.fullscreen_preview(ctx);
        });
        apply_textures(&mut textures, &output);
        last = Some(output);
    }
    let output = last.unwrap();
    assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text() == "Exit fullscreen · Esc")));
    assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text() == "Export video")));
    screenshot(&ctx, output, &textures, 1280, 840, "easycat-fullscreen");
    let video = app.video.as_ref().unwrap();
    assert_eq!((video.start, video.end, video.playhead), (5.0, 35.0, 12.0));
    // A failed file load must not trap the application in fullscreen.
    app.video = None;
    let output = ctx.run(input(), |ctx| app.apply(ctx, Action::ToggleFullscreen));
    assert!(!app.preview_fullscreen);
    assert!(output.viewport_output[&egui::ViewportId::ROOT]
        .commands
        .iter()
        .any(|command| matches!(command, egui::ViewportCommand::Fullscreen(false))));
}

#[test]
fn transport_skips_seconds_clamps_to_selection_and_adjusts_preview_volume() {
    let ctx = egui::Context::default();
    let mut app = App::new(None, &ctx);
    let info = easycat_core::probe::parse_probe(r#"{"streams":[{"codec_type":"video","codec_name":"h264","width":1080,"height":1920,"r_frame_rate":"30/1"}],"format":{"duration":"42"}}"#).unwrap();
    let (tx, _rx) = channel();
    let mut video = Video::new("fixture.mp4".into(), info, vec![], tx, false);
    video.start = 5.0;
    video.end = 35.0;
    video.playhead = 12.0;
    app.video = Some(video);
    app.apply(&ctx, Action::Skip(-5.0));
    assert_eq!(app.video.as_ref().unwrap().playhead, 7.0);
    app.apply(&ctx, Action::Skip(-5.0));
    assert_eq!(app.video.as_ref().unwrap().playhead, 5.0);
    app.apply(&ctx, Action::Skip(100.0));
    assert_eq!(app.video.as_ref().unwrap().playhead, 35.0);
    app.apply(&ctx, Action::Step(-1.0 / 30.0));
    assert!((app.video.as_ref().unwrap().playhead - (35.0 - 1.0 / 30.0)).abs() < 0.001);
    app.apply(&ctx, Action::SetVolume(35));
    assert_eq!(app.volume, 35);
    app.apply(&ctx, Action::ToggleMute);
    assert!(app.muted);
    app.apply(&ctx, Action::ToggleMute);
    assert!(!app.muted);
    assert_eq!(app.volume, 35);
}

#[test]
fn minimum_build_gate_waits_for_export_and_blocks_new_exports() {
    use crate::updates::{Release, Status, BUILD};
    use std::sync::atomic::Ordering;
    let ctx = egui::Context::default();
    let mut app = App::new(None, &ctx);
    let info = easycat_core::probe::parse_probe(r#"{"streams":[{"codec_type":"video","codec_name":"h264","width":160,"height":96}],"format":{"duration":"2"}}"#).unwrap();
    let (tx, _rx) = channel();
    app.video = Some(Video::new("fixture.mp4".into(), info, vec![], tx, false));
    app.updates.status = Status::Required(Release {
        latest_build: BUILD + 1,
        minimum_build: BUILD + 1,
        version: "0.1.0-preview.5".into(),
    });
    let cancel = Arc::new(AtomicBool::new(false));
    app.export_cancel = Some(cancel.clone());
    app.exporting = true;
    assert!(!app.enforce_minimum_build());
    assert!(!cancel.load(Ordering::Relaxed));
    assert!(app.exporting);
    app.exporting = false;
    let playback = Arc::new(AtomicBool::new(false));
    let measurement = Arc::new(AtomicBool::new(false));
    app.play = Some(Play {
        id: 1,
        stop: playback.clone(),
    });
    app.measure = Some(Measure {
        id: 1,
        stop: measurement.clone(),
        done: 0,
        total: 1,
    });
    app.preview_fullscreen = true;
    assert!(app.enforce_minimum_build());
    assert!(playback.load(Ordering::Relaxed));
    assert!(measurement.load(Ordering::Relaxed));
    assert!(!app.preview_fullscreen);
    app.start_export(&ctx);
    assert!(!app.exporting);
    app.updates.status = Status::Unavailable;
    assert!(!app.enforce_minimum_build());
}

#[test]
fn update_notice_keeps_export_and_settings_accessible_at_minimum_size() {
    let ctx = egui::Context::default();
    theme::install(&ctx);
    let mut app = App::new(None, &ctx);
    let info = easycat_core::probe::parse_probe(r#"{"streams":[{"codec_type":"video","codec_name":"h264","width":1920,"height":1080}],"format":{"duration":"42"}}"#).unwrap();
    let presets = presets::builtin(&info, &HashSet::from(["libx264".into(), "libx265".into()]));
    let (tx, _rx) = channel();
    let mut video = Video::new("fixture.mp4".into(), info, presets, tx, false);
    video.choice = Choice::Preset(0);
    app.video = Some(video);
    app.updates.status = crate::updates::Status::Available(crate::updates::Release {
        latest_build: 5,
        minimum_build: 4,
        version: "0.1.0-preview.5".into(),
    });
    for _ in 0..3 {
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1180.0, 850.0))),
                ..Default::default()
            },
            |ctx| {
                app.top_bar(ctx);
                app.bottom_bar(ctx);
                app.updates.notice(ctx, false);
                app.export_panel(ctx);
                app.center(ctx);
            },
        );
        let mut hardware = None;
        let mut estimate = None;
        for shape in &output.shapes {
            if let egui::Shape::Text(text) = &shape.shape {
                let bounds = Rect::from_min_size(text.pos, text.galley.size());
                if ["Export video", "Download update", "Hardware acceleration"]
                    .contains(&text.galley.text())
                {
                    assert!(
                        shape.clip_rect.contains_rect(bounds),
                        "clipped {}",
                        text.galley.text()
                    );
                }
                if text.galley.text() == "Hardware acceleration" {
                    hardware = Some(bounds.bottom());
                }
                if text.galley.text() == "Rough estimate" {
                    estimate = Some(bounds.top());
                }
            }
        }
        if let (Some(bottom), Some(top)) = (hardware, estimate) {
            assert!(bottom <= top, "update notice overlaps export settings");
        }
    }
}
