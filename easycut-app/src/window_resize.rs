// File: easycut-app/src/window_resize.rs
//! Explicit client-edge resizing, including macOS where BeginResize is unsupported.
use eframe::egui::{self, pos2, vec2, Pos2, Rect, Vec2};

pub const MIN_SIZE: Vec2 = vec2(1180.0, 850.0);

#[derive(Clone, Copy)]
struct Edge {
    west: bool,
    east: bool,
    north: bool,
    south: bool,
}

#[derive(Clone, Copy)]
struct Drag {
    edge: Edge,
    pointer: Pos2,
    position: Pos2,
    size: Vec2,
}

#[derive(Default)]
pub struct WindowResize {
    drag: Option<Drag>,
}

fn resized(drag: Drag, pointer: Pos2, maximum: Vec2) -> (Pos2, Vec2) {
    let delta = pointer - drag.pointer;
    let edge = drag.edge;
    let width = (drag.size.x
        + if edge.west {
            -delta.x
        } else if edge.east {
            delta.x
        } else {
            0.0
        })
    .clamp(MIN_SIZE.x, maximum.x.max(MIN_SIZE.x));
    let height = (drag.size.y
        + if edge.north {
            -delta.y
        } else if edge.south {
            delta.y
        } else {
            0.0
        })
    .clamp(MIN_SIZE.y, maximum.y.max(MIN_SIZE.y));
    let position = drag.position
        + vec2(
            if edge.west { drag.size.x - width } else { 0.0 },
            if edge.north {
                drag.size.y - height
            } else {
                0.0
            },
        );
    (position, vec2(width, height))
}

impl WindowResize {
    pub fn show(&mut self, ctx: &egui::Context) {
        let (viewport, pointer, pressed) = ctx.input(|i| {
            (
                i.viewport().clone(),
                i.pointer.interact_pos(),
                i.pointer.primary_down(),
            )
        });
        if viewport.fullscreen == Some(true) || viewport.maximized == Some(true) {
            self.drag = None;
            return;
        }
        let screen = ctx.screen_rect();
        let inner = viewport.inner_rect.unwrap_or(screen);
        let outer = viewport.outer_rect.unwrap_or(inner);
        if !pressed {
            self.drag = None;
        }
        let border = 7.0;
        let corner = 16.0;
        // Corners and edges have disjoint hit regions so normal controls remain usable.
        let regions = [
            (
                Rect::from_min_size(screen.min, vec2(corner, corner)),
                true,
                false,
                true,
                false,
            ),
            (
                Rect::from_min_size(
                    pos2(screen.right() - corner, screen.top()),
                    vec2(corner, corner),
                ),
                false,
                true,
                true,
                false,
            ),
            (
                Rect::from_min_size(
                    pos2(screen.left(), screen.bottom() - corner),
                    vec2(corner, corner),
                ),
                true,
                false,
                false,
                true,
            ),
            (
                Rect::from_min_size(screen.max - vec2(corner, corner), vec2(corner, corner)),
                false,
                true,
                false,
                true,
            ),
            (
                Rect::from_min_max(
                    pos2(screen.left() + corner, screen.top()),
                    pos2(screen.right() - corner, screen.top() + border),
                ),
                false,
                false,
                true,
                false,
            ),
            (
                Rect::from_min_max(
                    pos2(screen.left() + corner, screen.bottom() - border),
                    pos2(screen.right() - corner, screen.bottom()),
                ),
                false,
                false,
                false,
                true,
            ),
            (
                Rect::from_min_max(
                    pos2(screen.left(), screen.top() + corner),
                    pos2(screen.left() + border, screen.bottom() - corner),
                ),
                true,
                false,
                false,
                false,
            ),
            (
                Rect::from_min_max(
                    pos2(screen.right() - border, screen.top() + corner),
                    pos2(screen.right(), screen.bottom() - corner),
                ),
                false,
                true,
                false,
                false,
            ),
        ];
        for (index, (rect, west, east, north, south)) in regions.into_iter().enumerate() {
            let edge = Edge {
                west,
                east,
                north,
                south,
            };
            egui::Area::new(egui::Id::new(("window_resize", index)))
                .order(egui::Order::Foreground)
                .fixed_pos(rect.min)
                .default_size(rect.size())
                .constrain(false)
                .movable(false)
                .show(ctx, |ui| {
                    let (_, response) = ui.allocate_exact_size(rect.size(), egui::Sense::drag());
                    if response.hovered()
                        || response.dragged()
                        || pointer.is_some_and(|point| rect.contains(point))
                    {
                        ctx.set_cursor_icon(if (west && north) || (east && south) {
                            egui::CursorIcon::ResizeNwSe
                        } else if (west && south) || (east && north) {
                            egui::CursorIcon::ResizeNeSw
                        } else if west || east {
                            egui::CursorIcon::ResizeHorizontal
                        } else {
                            egui::CursorIcon::ResizeVertical
                        });
                    }
                    if self.drag.is_none() && ctx.input(|i| i.pointer.primary_pressed()) {
                        if let Some(start) = ctx.input(|i| i.pointer.press_origin()) {
                            if rect.contains(start) {
                                if cfg!(target_os = "macos") {
                                    self.drag = Some(Drag {
                                        edge,
                                        pointer: inner.min + start.to_vec2(),
                                        position: outer.min,
                                        size: screen.size(),
                                    });
                                } else {
                                    use egui::ResizeDirection as D;
                                    let direction = match (west, east, north, south) {
                                        (true, _, true, _) => D::NorthWest,
                                        (_, true, true, _) => D::NorthEast,
                                        (true, _, _, true) => D::SouthWest,
                                        (_, true, _, true) => D::SouthEast,
                                        (true, _, _, _) => D::West,
                                        (_, true, _, _) => D::East,
                                        (_, _, true, _) => D::North,
                                        _ => D::South,
                                    };
                                    ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(
                                        direction,
                                    ));
                                }
                            }
                        }
                    }
                });
        }
        if let (Some(drag), Some(pointer)) = (self.drag, pointer) {
            let chrome = outer.size() - inner.size();
            let maximum = viewport.monitor_size.unwrap_or(vec2(16_384.0, 16_384.0)) - chrome;
            let (position, size) = resized(drag, inner.min + pointer.to_vec2(), maximum);
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
            if drag.edge.west || drag.edge.north {
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(position));
            }
            ctx.request_repaint();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn opposite_edge_is_preserved_and_minimum_is_enforced() {
        let drag = Drag {
            edge: Edge {
                west: true,
                east: false,
                north: true,
                south: false,
            },
            pointer: pos2(100.0, 100.0),
            position: pos2(50.0, 50.0),
            size: vec2(1360.0, 1040.0),
        };
        let (position, size) = resized(drag, pos2(200.0, 150.0), vec2(1920.0, 1080.0));
        assert_eq!(size, vec2(1260.0, 990.0));
        assert_eq!(position, pos2(150.0, 100.0));
        assert_eq!(position + size, drag.position + drag.size);
        let (_, size) = resized(drag, pos2(2000.0, 2000.0), vec2(1920.0, 1080.0));
        assert_eq!(size, MIN_SIZE);
    }

    #[test]
    fn edge_drag_sends_real_native_resize_commands() {
        let ctx = egui::Context::default();
        let mut resize = WindowResize::default();
        let events = [
            vec![],
            vec![
                egui::Event::PointerMoved(pos2(1357.0, 400.0)),
                egui::Event::PointerButton {
                    pos: pos2(1357.0, 400.0),
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
            ],
            vec![egui::Event::PointerMoved(pos2(1300.0, 400.0))],
        ];
        let mut commands = vec![];
        for events in events {
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1360.0, 1040.0))),
                    events,
                    ..Default::default()
                },
                |ctx| resize.show(ctx),
            );
            commands.extend(
                output.viewport_output[&egui::ViewportId::ROOT]
                    .commands
                    .clone(),
            );
        }
        if cfg!(target_os = "macos") {
            assert!(commands.iter().any(|command| matches!(command, egui::ViewportCommand::InnerSize(size) if size.x < 1360.0)));
        } else {
            assert!(commands.iter().any(|command| matches!(
                command,
                egui::ViewportCommand::BeginResize(egui::ResizeDirection::East)
            )));
        }
    }
}
