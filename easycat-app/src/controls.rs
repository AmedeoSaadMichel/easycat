// File: easycat-app/src/controls.rs
//! Playback controls (native drawn icons) and keyboard shortcuts.
use crate::theme;
use eframe::egui::{self, pos2, vec2, Color32, Key, Pos2, Rect, Sense, Shape, Stroke};

#[derive(Clone, Copy, PartialEq)]
pub enum Action {
    None,
    TogglePlay,
    ToggleFullscreen,
    /// Seek to a timestamp; resume if `true` and playback is active.
    Seek(f64, bool),
    /// Move the playhead by `dt` seconds and pause.
    Step(f64),
    ToStart,
    ToEnd,
    ToggleMute,
    SetVolume(u8),
    Skip(f64),
    SetStart,
    SetEnd,
}

#[derive(Clone, Copy)]
enum Icon {
    Start,
    Back,
    Play,
    Pause,
    Fwd,
    End,
    Fullscreen,
    Speaker(bool),
}

fn tri(painter: &egui::Painter, pts: [Pos2; 3], c: Color32) {
    painter.add(Shape::convex_polygon(pts.to_vec(), c, Stroke::NONE));
}

fn icon_button(ui: &mut egui::Ui, icon: Icon, tip: &str, wide: bool) -> bool {
    let base = ui.spacing().interact_size.x;
    let size = vec2(if wide { base * 1.47 } else { base }, 32.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let vis = ui.style().interact(&resp);
    let painter = ui.painter();
    painter.rect(
        rect,
        if wide { 16.0 } else { 8.0 },
        if wide { theme::LILAC } else { vis.bg_fill },
        vis.bg_stroke,
    );
    let c = if wide { theme::BG } else { vis.fg_stroke.color };
    let (x, y) = (rect.center().x, rect.center().y);
    let bar = |dx: f32| Rect::from_center_size(pos2(x + dx, y), vec2(3.0, 14.0));
    match icon {
        Icon::Speaker(muted) => {
            painter.rect_filled(
                Rect::from_center_size(pos2(x - 7.0, y), vec2(5.0, 8.0)),
                1.0,
                c,
            );
            tri(
                painter,
                [
                    pos2(x - 6.0, y),
                    pos2(x + 1.0, y - 9.0),
                    pos2(x + 1.0, y + 9.0),
                ],
                c,
            );
            if muted {
                painter.line_segment(
                    [pos2(x + 4.0, y - 5.0), pos2(x + 12.0, y + 5.0)],
                    Stroke::new(1.5_f32, c),
                );
                painter.line_segment(
                    [pos2(x + 4.0, y + 5.0), pos2(x + 12.0, y - 5.0)],
                    Stroke::new(1.5_f32, c),
                );
            }
            for offset in [5.0, 10.0].into_iter().filter(|_| !muted) {
                painter.add(Shape::line(
                    vec![
                        pos2(x + offset - 2.0, y - 7.0),
                        pos2(x + offset + 1.0, y),
                        pos2(x + offset - 2.0, y + 7.0),
                    ],
                    Stroke::new(1.5_f32, c),
                ));
            }
        }
        Icon::Fullscreen => {
            for sx in [-1.0, 1.0] {
                for sy in [-1.0, 1.0] {
                    let corner = pos2(x + sx * 9.0, y + sy * 9.0);
                    painter.add(Shape::line(
                        vec![
                            corner - vec2(sx * 6.0, 0.0),
                            corner,
                            corner - vec2(0.0, sy * 6.0),
                        ],
                        Stroke::new(1.8_f32, c),
                    ));
                }
            }
        }
        Icon::Play => tri(
            painter,
            [
                pos2(x - 5.0, y - 9.0),
                pos2(x - 5.0, y + 9.0),
                pos2(x + 9.0, y),
            ],
            c,
        ),
        Icon::Pause => {
            painter.rect_filled(bar(-5.0), 1.0, c);
            painter.rect_filled(bar(5.0), 1.0, c);
        }
        Icon::Back => tri(
            painter,
            [
                pos2(x + 6.0, y - 8.0),
                pos2(x + 6.0, y + 8.0),
                pos2(x - 7.0, y),
            ],
            c,
        ),
        Icon::Fwd => tri(
            painter,
            [
                pos2(x - 6.0, y - 8.0),
                pos2(x - 6.0, y + 8.0),
                pos2(x + 7.0, y),
            ],
            c,
        ),
        Icon::Start => {
            painter.rect_filled(bar(-8.0), 1.0, c);
            tri(
                painter,
                [
                    pos2(x + 8.0, y - 8.0),
                    pos2(x + 8.0, y + 8.0),
                    pos2(x - 4.0, y),
                ],
                c,
            );
        }
        Icon::End => {
            painter.rect_filled(bar(8.0), 1.0, c);
            tri(
                painter,
                [
                    pos2(x - 8.0, y - 8.0),
                    pos2(x - 8.0, y + 8.0),
                    pos2(x + 4.0, y),
                ],
                c,
            );
        }
    }
    resp.on_hover_text(tip).clicked()
}

/// Playback buttons; return the requested action.
pub fn show(ui: &mut egui::Ui, playing: bool, muted: bool, volume: &mut u8, _frame: f64) -> Action {
    let mut act = Action::None;
    let width = ui.available_width();
    let button = if width < 600.0 { 28.0 } else { 38.0 };
    let gap = 4.0;
    let transport_width = button * 5.47 + gap * 4.0;
    let (row, _) = ui.allocate_exact_size(vec2(width, 32.0), Sense::hover());
    let transport = Rect::from_center_size(row.center(), vec2(transport_width, 32.0));
    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(transport), |ui| {
        ui.spacing_mut().item_spacing.x = gap;
        ui.spacing_mut().interact_size.x = button;
        ui.horizontal(|ui| {
            if icon_button(ui, Icon::Start, "Go to selection start", false) {
                act = Action::ToStart;
            }
            if icon_button(ui, Icon::Back, "Back 5 seconds (← = previous frame)", false) {
                act = Action::Skip(-5.0);
            }
            let (icon, tip) = if playing {
                (Icon::Pause, "Pause (Space)")
            } else {
                (Icon::Play, "Play (Space)")
            };
            if icon_button(ui, icon, tip, true) {
                act = Action::TogglePlay;
            }
            if icon_button(ui, Icon::Fwd, "Forward 5 seconds (→ = next frame)", false) {
                act = Action::Skip(5.0);
            }
            if icon_button(ui, Icon::End, "Go to selection end", false) {
                act = Action::ToEnd;
            }
        });
    });
    let audio = Rect::from_min_max(row.min, pos2(transport.left() - 8.0, row.bottom()));
    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(audio), |ui| {
        ui.spacing_mut().interact_size.x = button;
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.spacing_mut().slider_width = (audio.width() - button - 4.0).clamp(24.0, 100.0);
        ui.horizontal(|ui| {
            if icon_button(
                ui,
                Icon::Speaker(muted || *volume == 0),
                if muted { "Unmute" } else { "Mute" },
                false,
            ) {
                act = Action::ToggleMute;
            }
            let slider = ui
                .add(egui::Slider::new(volume, 0..=100).show_value(false))
                .on_hover_text(format!("Volume: {volume}%"));
            if slider.drag_stopped() || (slider.changed() && !slider.dragged()) {
                act = Action::SetVolume(*volume);
            }
        });
    });
    let fullscreen = Rect::from_min_size(pos2(row.right() - button, row.top()), vec2(button, 32.0));
    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(fullscreen), |ui| {
        ui.spacing_mut().interact_size.x = button;
        if icon_button(
            ui,
            Icon::Fullscreen,
            "Toggle fullscreen (F) · Exit with Esc",
            false,
        ) {
            act = Action::ToggleFullscreen;
        }
    });
    act
}

/// Shortcuts: Space, arrows, I (start), O (end); ignored while typing.
pub fn shortcuts(ctx: &egui::Context, frame: f64) -> Action {
    if ctx.wants_keyboard_input() {
        return Action::None;
    }
    ctx.input(|i| {
        let step = if i.modifiers.shift { 1.0 } else { frame };
        if i.key_pressed(Key::F) {
            Action::ToggleFullscreen
        } else if i.key_pressed(Key::Space) {
            Action::TogglePlay
        } else if i.key_pressed(Key::ArrowLeft) {
            Action::Step(-step)
        } else if i.key_pressed(Key::ArrowRight) {
            Action::Step(step)
        } else if i.key_pressed(Key::I) {
            Action::SetStart
        } else if i.key_pressed(Key::O) {
            Action::SetEnd
        } else {
            Action::None
        }
    })
}

/// Escape always exits the preview, even if a control still owns keyboard focus.
pub fn fullscreen_shortcuts(ctx: &egui::Context, frame: f64) -> Action {
    if ctx.input(|i| i.key_pressed(Key::Escape)) {
        Action::ToggleFullscreen
    } else {
        shortcuts(ctx, frame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn play_button_stays_centered_beside_volume_at_compact_and_wide_sizes() {
        for width in [400.0, 870.0] {
            let ctx = egui::Context::default();
            let mut volume = 75;
            let mut action = Action::None;
            for events in [
                vec![],
                vec![
                    egui::Event::PointerMoved(pos2(width / 2.0, 16.0)),
                    egui::Event::PointerButton {
                        pos: pos2(width / 2.0, 16.0),
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: Default::default(),
                    },
                ],
                vec![egui::Event::PointerButton {
                    pos: pos2(width / 2.0, 16.0),
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: Default::default(),
                }],
            ] {
                let _ = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(width, 100.0))),
                        events,
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default()
                            .frame(egui::Frame::none())
                            .show(ctx, |ui| {
                                action = show(ui, false, false, &mut volume, 1.0 / 30.0);
                                assert!(ui.cursor().top() <= 40.0, "transport must occupy one row");
                            });
                    },
                );
            }
            assert!(action == Action::TogglePlay);
            assert_eq!(volume, 75);
        }
    }

    #[test]
    fn fullscreen_keyboard_shortcuts() {
        let ctx = egui::Context::default();
        for key in [Key::F, Key::Escape] {
            let _ = ctx.run(
                egui::RawInput {
                    events: vec![egui::Event::Key {
                        key,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: Default::default(),
                    }],
                    ..Default::default()
                },
                |ctx| {
                    assert!(fullscreen_shortcuts(ctx, 1.0 / 30.0) == Action::ToggleFullscreen);
                },
            );
        }
    }
}
