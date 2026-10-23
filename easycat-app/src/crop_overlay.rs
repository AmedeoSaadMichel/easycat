// File: easycat-app/src/crop_overlay.rs
//! Crop rectangle drawn over the preview: geometry first, egui drawing after.
use crate::theme;
use easycat_core::Crop;
use eframe::egui::{self, Color32, CursorIcon, Pos2, Rect, Sense, Stroke, Vec2};

const MIN_SIDE: f64 = 16.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Handle {
    Move,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

fn even_down(v: f64) -> u32 {
    (v.max(0.0) as u32) & !1
}

/// Applies a drag of `(dx, dy)` video pixels. Always returns a rectangle that
/// fits the frame, with even sides and positions. `ratio` keeps the shape
/// while a corner is dragged. An impossible result leaves the crop unchanged.
pub fn drag(
    crop: Crop,
    handle: Handle,
    (dx, dy): (f64, f64),
    frame: (u32, u32),
    ratio: Option<(u32, u32)>,
) -> Crop {
    let (fw, fh) = (frame.0 as f64, frame.1 as f64);
    let (x, y) = (crop.x as f64, crop.y as f64);
    let (w, h) = (crop.width as f64, crop.height as f64);

    let candidate = if handle == Handle::Move {
        let nx = (x + dx).clamp(0.0, (fw - w).max(0.0));
        let ny = (y + dy).clamp(0.0, (fh - h).max(0.0));
        Crop {
            x: even_down(nx),
            y: even_down(ny),
            ..crop
        }
    } else {
        let (left, top) = matches!(handle, Handle::TopLeft | Handle::BottomLeft)
            .then_some((true, matches!(handle, Handle::TopLeft)))
            .unwrap_or((false, matches!(handle, Handle::TopRight)));
        // The corner opposite to the one being dragged stays where it is.
        let anchor = (if left { x + w } else { x }, if top { y + h } else { y });
        let moving = (
            (if left { x } else { x + w } + dx).clamp(0.0, fw),
            (if top { y } else { y + h } + dy).clamp(0.0, fh),
        );
        let (mut nw, mut nh) = ((moving.0 - anchor.0).abs(), (moving.1 - anchor.1).abs());
        // Dragging across the anchor flips the rectangle: treat it as too small.
        if (moving.0 < anchor.0) != left || (moving.1 < anchor.1) != top {
            (nw, nh) = (0.0, 0.0);
        }
        if let Some((rw, rh)) = ratio {
            let (rw, rh) = (rw.max(1) as f64, rh.max(1) as f64);
            let room_w = if left { anchor.0 } else { fw - anchor.0 };
            let room_h = if top { anchor.1 } else { fh - anchor.1 };
            nw = nw.min(room_w).min(room_h * rw / rh);
            nh = nw * rh / rw;
        }
        let nx = if left { anchor.0 - nw } else { anchor.0 };
        let ny = if top { anchor.1 - nh } else { anchor.1 };
        // Round the position first, then keep the far edge inside the frame.
        let (ex, ey) = (even_down(nx), even_down(ny));
        Crop {
            x: ex,
            y: ey,
            width: even_down(nw).min(frame.0.saturating_sub(ex) & !1),
            height: even_down(nh).min(frame.1.saturating_sub(ey) & !1),
        }
    };
    let ok = candidate.validate(frame.0, frame.1).is_ok()
        && candidate.width as f64 >= MIN_SIDE
        && candidate.height as f64 >= MIN_SIDE;
    if ok {
        candidate
    } else {
        crop
    }
}

/// Where the handles sit, in screen space.
fn corners(rect: Rect) -> [(Handle, Pos2); 4] {
    [
        (Handle::TopLeft, rect.left_top()),
        (Handle::TopRight, rect.right_top()),
        (Handle::BottomLeft, rect.left_bottom()),
        (Handle::BottomRight, rect.right_bottom()),
    ]
}

/// Draws the crop over the shown picture `image` and lets the user move and
/// resize it. Returns true when the rectangle changed.
pub fn show(
    ui: &mut egui::Ui,
    image: Rect,
    frame: (u32, u32),
    crop: &mut Crop,
    ratio: Option<(u32, u32)>,
    enabled: bool,
) -> bool {
    let scale = image.width() / frame.0.max(1) as f32;
    let to_screen = |c: &Crop| {
        Rect::from_min_size(
            image.min + Vec2::new(c.x as f32, c.y as f32) * scale,
            Vec2::new(c.width as f32, c.height as f32) * scale,
        )
    };
    let mut changed = false;
    let rect = to_screen(crop);

    if enabled {
        let body = ui.interact(rect.shrink(10.0), ui.id().with("crop-move"), Sense::drag());
        if body.hovered() || body.dragged() {
            ui.ctx().set_cursor_icon(CursorIcon::Grab);
        }
        if body.dragged() {
            let d = body.drag_delta() / scale;
            let next = drag(*crop, Handle::Move, (d.x as f64, d.y as f64), frame, ratio);
            changed |= next != *crop;
            *crop = next;
        }
        for (handle, pos) in corners(to_screen(crop)) {
            let area = Rect::from_center_size(pos, Vec2::splat(30.0));
            let response = ui.interact(
                area,
                ui.id().with(("crop-handle", handle as u8)),
                Sense::drag(),
            );
            if response.hovered() || response.dragged() {
                ui.ctx().set_cursor_icon(CursorIcon::ResizeNwSe);
            }
            if response.dragged() {
                let d = response.drag_delta() / scale;
                let next = drag(*crop, handle, (d.x as f64, d.y as f64), frame, ratio);
                changed |= next != *crop;
                *crop = next;
            }
        }
    }

    let rect = to_screen(crop);
    let p = ui.painter_at(image.expand(2.0));
    let dim = Color32::from_black_alpha(150);
    // Everything outside the crop is dimmed.
    p.rect_filled(
        Rect::from_min_max(image.min, Pos2::new(image.max.x, rect.min.y)),
        0.0,
        dim,
    );
    p.rect_filled(
        Rect::from_min_max(Pos2::new(image.min.x, rect.max.y), image.max),
        0.0,
        dim,
    );
    p.rect_filled(
        Rect::from_min_max(
            Pos2::new(image.min.x, rect.min.y),
            Pos2::new(rect.min.x, rect.max.y),
        ),
        0.0,
        dim,
    );
    p.rect_filled(
        Rect::from_min_max(
            Pos2::new(rect.max.x, rect.min.y),
            Pos2::new(image.max.x, rect.max.y),
        ),
        0.0,
        dim,
    );
    let grid = Stroke::new(1.0_f32, theme::CREAM.gamma_multiply(0.35));
    for i in 1..3 {
        let f = i as f32 / 3.0;
        p.line_segment(
            [
                Pos2::new(rect.min.x + rect.width() * f, rect.min.y),
                Pos2::new(rect.min.x + rect.width() * f, rect.max.y),
            ],
            grid,
        );
        p.line_segment(
            [
                Pos2::new(rect.min.x, rect.min.y + rect.height() * f),
                Pos2::new(rect.max.x, rect.min.y + rect.height() * f),
            ],
            grid,
        );
    }
    p.rect_stroke(rect, 0.0, Stroke::new(2.0_f32, theme::MINT));
    for (_, pos) in corners(rect) {
        p.circle_filled(pos, 6.0, theme::MINT);
        p.circle_stroke(pos, 6.0, Stroke::new(1.5_f32, theme::BG));
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME: (u32, u32) = (1920, 1080);

    fn crop(x: u32, y: u32, w: u32, h: u32) -> Crop {
        Crop {
            x,
            y,
            width: w,
            height: h,
        }
    }

    #[test]
    fn moving_stays_inside_the_frame_and_keeps_the_size() {
        let c = crop(100, 100, 400, 300);
        let moved = drag(c, Handle::Move, (50.0, -30.0), FRAME, None);
        assert_eq!(
            (moved.x, moved.y, moved.width, moved.height),
            (150, 70, 400, 300)
        );
        let far = drag(c, Handle::Move, (9000.0, 9000.0), FRAME, None);
        assert_eq!((far.x, far.y), (1520, 780));
        let before = drag(c, Handle::Move, (-9000.0, -9000.0), FRAME, None);
        assert_eq!((before.x, before.y), (0, 0));
    }

    #[test]
    fn resizing_a_corner_keeps_the_opposite_corner_fixed() {
        let c = crop(100, 100, 400, 300);
        let grown = drag(c, Handle::BottomRight, (100.0, 50.0), FRAME, None);
        assert_eq!(
            (grown.x, grown.y, grown.width, grown.height),
            (100, 100, 500, 350)
        );
        let shrunk = drag(c, Handle::TopLeft, (50.0, 40.0), FRAME, None);
        assert_eq!(
            (shrunk.x, shrunk.y, shrunk.width, shrunk.height),
            (150, 140, 350, 260)
        );
        // Opposite corner of the top-left drag is (500, 400): unchanged.
        assert_eq!(
            (shrunk.x + shrunk.width, shrunk.y + shrunk.height),
            (500, 400)
        );
    }

    #[test]
    fn a_ratio_lock_keeps_the_shape_while_resizing() {
        let c = Crop::centered(1920, 1080, 1, 1);
        let bigger = drag(c, Handle::BottomRight, (-100.0, 300.0), FRAME, Some((1, 1)));
        assert_eq!(bigger.width, bigger.height);
        let wide = Crop::centered(1920, 1080, 16, 9);
        let smaller = drag(wide, Handle::TopLeft, (200.0, 0.0), FRAME, Some((16, 9)));
        // Even sizes allow a rounding error of a pixel or two.
        let error = (smaller.width as i64 * 9 - smaller.height as i64 * 16).abs();
        assert!(error <= 32, "{smaller:?}");
    }

    #[test]
    fn results_are_always_even_and_inside_the_frame() {
        let c = crop(101 & !1, 100, 400, 300);
        for handle in [
            Handle::Move,
            Handle::TopLeft,
            Handle::TopRight,
            Handle::BottomLeft,
            Handle::BottomRight,
        ] {
            for (dx, dy) in [
                (13.7, -9.3),
                (-777.1, 55.5),
                (4000.0, 4000.0),
                (-4000.0, -4000.0),
            ] {
                for ratio in [None, Some((9, 16))] {
                    let next = drag(c, handle, (dx, dy), FRAME, ratio);
                    assert!(
                        next.validate(FRAME.0, FRAME.1).is_ok(),
                        "{handle:?} {dx},{dy}: {next:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn dragging_past_the_opposite_corner_or_below_the_minimum_changes_nothing() {
        let c = crop(100, 100, 400, 300);
        assert_eq!(drag(c, Handle::BottomRight, (-450.0, 0.0), FRAME, None), c);
        assert_eq!(drag(c, Handle::TopLeft, (395.0, 0.0), FRAME, None), c);
    }
}
