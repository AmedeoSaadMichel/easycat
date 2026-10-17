// File: easycut-app/src/window_chrome.rs
//! Illustrated titlebar with native macOS buttons and themed controls elsewhere.
use crate::{theme, window_resize};
use eframe::egui::{self, pos2, vec2, Rect, Sense, ViewportCommand};

pub fn viewport() -> egui::ViewportBuilder {
    viewport_for(cfg!(target_os = "macos"))
}

fn viewport_for(macos: bool) -> egui::ViewportBuilder {
    let viewport = egui::ViewportBuilder::default()
        .with_resizable(true)
        .with_inner_size(window_resize::MIN_SIZE)
        .with_min_inner_size(window_resize::MIN_SIZE)
        .with_clamp_size_to_monitor_size(true);
    if macos {
        viewport
            .with_decorations(true)
            .with_fullsize_content_view(true)
            .with_titlebar_shown(false)
            .with_title_shown(false)
            .with_titlebar_buttons_shown(true)
    } else {
        viewport.with_decorations(false)
    }
}

pub fn drag_region(ui: &mut egui::Ui, rect: Rect) {
    let mut rect = rect.shrink(7.0);
    if cfg!(target_os = "macos") {
        rect.min.x += 76.0;
    }
    let response = ui.interact(rect, ui.id().with("titlebar_drag"), Sense::click_and_drag());
    if response.double_clicked() {
        let maximized = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
        ui.ctx()
            .send_viewport_cmd(ViewportCommand::Maximized(!maximized));
    } else if response.drag_started() {
        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
    }
}

pub fn controls(ui: &mut egui::Ui, header: Rect) {
    if !cfg!(target_os = "macos") {
        controls_for(ui, header);
    }
}

fn controls_for(ui: &mut egui::Ui, header: Rect) {
    let maximized = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
    for (index, tooltip) in [
        "Minimize",
        if maximized { "Restore" } else { "Maximize" },
        "Close",
    ]
    .into_iter()
    .enumerate()
    {
        let rect = Rect::from_min_size(
            pos2(
                header.right() - 116.0 + index as f32 * 36.0,
                header.top() + 7.0,
            ),
            vec2(32.0, 24.0),
        );
        let response = ui
            .interact(
                rect,
                ui.id().with(("window_control", index)),
                Sense::click(),
            )
            .on_hover_text(tooltip);
        let painter = ui.painter();
        if response.hovered() {
            painter.rect_filled(
                rect,
                5.0,
                if index == 2 {
                    theme::CORAL
                } else {
                    theme::SURFACE
                },
            );
        }
        let center = rect.center();
        let stroke = egui::Stroke::new(1.5_f32, theme::CREAM);
        match index {
            0 => {
                painter.line_segment([center - vec2(5.0, 0.0), center + vec2(5.0, 0.0)], stroke);
            }
            1 => {
                painter.rect_stroke(Rect::from_center_size(center, vec2(10.0, 9.0)), 0.0, stroke);
            }
            _ => {
                painter.line_segment([center - vec2(5.0, 5.0), center + vec2(5.0, 5.0)], stroke);
                painter.line_segment([center + vec2(-5.0, 5.0), center + vec2(5.0, -5.0)], stroke);
            }
        }
        if response.clicked() {
            ui.ctx().send_viewport_cmd(match index {
                0 => ViewportCommand::Minimized(true),
                1 => ViewportCommand::Maximized(!maximized),
                _ => ViewportCommand::Close,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn illustrated_header_can_drag_the_native_window() {
        let ctx = egui::Context::default();
        let point = pos2(400.0, 50.0);
        let mut dragged = false;
        for events in [
            vec![],
            vec![
                egui::Event::PointerMoved(point),
                egui::Event::PointerButton {
                    pos: point,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
            ],
            vec![egui::Event::PointerMoved(point + vec2(20.0, 0.0))],
        ] {
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(720.0, 480.0))),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        drag_region(ui, Rect::from_min_size(egui::Pos2::ZERO, vec2(720.0, 88.0)))
                    });
                },
            );
            dragged |= output.viewport_output[&egui::ViewportId::ROOT]
                .commands
                .iter()
                .any(|command| matches!(command, ViewportCommand::StartDrag));
        }
        assert!(dragged);
    }

    #[test]
    fn titlebar_configuration_is_themed_on_all_platforms() {
        let mac = viewport_for(true);
        assert_eq!(mac.titlebar_shown, Some(false));
        assert_eq!(mac.fullsize_content_view, Some(true));
        assert_eq!(mac.titlebar_buttons_shown, Some(true));
        assert_eq!(mac.title_shown, Some(false));
        let other = viewport_for(false);
        assert_eq!(other.decorations, Some(false));
        assert_eq!(other.resizable, Some(true));
    }
    #[test]
    fn custom_window_buttons_send_native_commands() {
        for index in 0..3 {
            let ctx = egui::Context::default();
            let mut commands = vec![];
            let point = pos2(720.0 - 100.0 + index as f32 * 36.0, 19.0);
            for events in [
                vec![],
                vec![
                    egui::Event::PointerMoved(point),
                    egui::Event::PointerButton {
                        pos: point,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: Default::default(),
                    },
                ],
                vec![egui::Event::PointerButton {
                    pos: point,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: Default::default(),
                }],
            ] {
                let output = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(
                            egui::Pos2::ZERO,
                            vec2(720.0, 480.0),
                        )),
                        events,
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| {
                            controls_for(
                                ui,
                                Rect::from_min_size(egui::Pos2::ZERO, vec2(720.0, 88.0)),
                            )
                        });
                    },
                );
                commands.extend(
                    output.viewport_output[&egui::ViewportId::ROOT]
                        .commands
                        .clone(),
                );
            }
            assert!(commands.iter().any(|command| matches!(
                (index, command),
                (0, ViewportCommand::Minimized(true))
                    | (1, ViewportCommand::Maximized(true))
                    | (2, ViewportCommand::Close)
            )));
        }
    }
}
