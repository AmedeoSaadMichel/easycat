// File: easycut-app/src/timeline.rs
//! Timeline con miniature, maniglie di inizio/fine taglio e indicatore di posizione.
use eframe::egui::{self, pos2, vec2, Color32, Rect, Sense, Stroke, TextureHandle};

const HEIGHT: f32 = 72.0;
const MIN_LEN: f64 = 0.1;
const HANDLE_W: f32 = 14.0;
const ACCENT: Color32 = Color32::from_rgb(255, 196, 0);

/// UV per riempire un riquadro senza deformare l'immagine (ritaglio centrato).
fn uv_cover(tile_aspect: f32, img_aspect: f32) -> Rect {
    if tile_aspect < img_aspect {
        let f = tile_aspect / img_aspect;
        Rect::from_min_max(pos2((1.0 - f) / 2.0, 0.0), pos2((1.0 + f) / 2.0, 1.0))
    } else {
        let f = img_aspect / tile_aspect;
        Rect::from_min_max(pos2(0.0, (1.0 - f) / 2.0), pos2(1.0, (1.0 + f) / 2.0))
    }
}

pub struct Range<'a> {
    pub start: &'a mut f64,
    pub end: &'a mut f64,
    pub playhead: &'a mut f64,
}

/// Disegna la timeline. Ritorna l'istante da mostrare in anteprima se è cambiato.
pub fn show(
    ui: &mut egui::Ui,
    thumbs: &[Option<TextureHandle>],
    img_aspect: f32,
    duration: f64,
    r: Range,
) -> Option<f64> {
    let (rect, resp) =
        ui.allocate_exact_size(vec2(ui.available_width(), HEIGHT), Sense::click_and_drag());
    let painter = ui.painter_at(rect.expand(6.0));
    painter.rect_filled(rect, 4.0, Color32::from_gray(28));

    let tile_w = rect.width() / thumbs.len().max(1) as f32;
    for (i, t) in thumbs.iter().enumerate() {
        if let Some(tex) = t {
            let tile = Rect::from_min_size(
                pos2(rect.left() + i as f32 * tile_w, rect.top()),
                vec2(tile_w, HEIGHT),
            );
            painter.image(
                tex.id(),
                tile,
                uv_cover(tile_w / HEIGHT, img_aspect),
                Color32::WHITE,
            );
        }
    }

    let x_of = |t: f64| rect.left() + (t / duration.max(0.001)) as f32 * rect.width();
    let t_of = |x: f32| ((x - rect.left()) / rect.width()).clamp(0.0, 1.0) as f64 * duration;
    let (xs, xe) = (x_of(*r.start), x_of(*r.end));

    let dim = Color32::from_black_alpha(170);
    painter.rect_filled(
        Rect::from_min_max(rect.min, pos2(xs, rect.bottom())),
        0.0,
        dim,
    );
    painter.rect_filled(Rect::from_min_max(pos2(xe, rect.top()), rect.max), 0.0, dim);
    painter.rect_stroke(
        Rect::from_min_max(pos2(xs, rect.top()), pos2(xe, rect.bottom())),
        0.0,
        Stroke::new(2.0_f32, ACCENT),
    );

    let handle =
        |x: f32| Rect::from_center_size(pos2(x, rect.center().y), vec2(HANDLE_W, HEIGHT + 10.0));
    let (hs, he) = (handle(xs), handle(xe));
    let rs = ui.interact(hs, ui.make_persistent_id("tl_start"), Sense::drag());
    let re = ui.interact(he, ui.make_persistent_id("tl_end"), Sense::drag());
    for (h, _) in [(hs, &rs), (he, &re)] {
        painter.rect_filled(h, 4.0, ACCENT);
        painter.rect_filled(
            Rect::from_center_size(h.center(), vec2(2.0, 22.0)),
            1.0,
            Color32::BLACK,
        );
    }
    if rs.hovered() || re.hovered() || rs.dragged() || re.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
    }

    let mut changed = None;
    if rs.dragged() {
        if let Some(p) = rs.interact_pointer_pos() {
            *r.start = t_of(p.x).min(*r.end - MIN_LEN).max(0.0);
            *r.playhead = *r.start;
            changed = Some(*r.start);
        }
    } else if re.dragged() {
        if let Some(p) = re.interact_pointer_pos() {
            *r.end = t_of(p.x).max(*r.start + MIN_LEN).min(duration);
            *r.playhead = *r.end;
            changed = Some(*r.end);
        }
    } else if resp.clicked() || resp.dragged() {
        if let Some(p) = resp.interact_pointer_pos() {
            *r.playhead = t_of(p.x);
            changed = Some(*r.playhead);
        }
    }

    let xp = x_of(*r.playhead);
    painter.line_segment(
        [pos2(xp, rect.top() - 4.0), pos2(xp, rect.bottom() + 4.0)],
        Stroke::new(2.0_f32, Color32::WHITE),
    );
    changed
}

pub fn fmt_time(secs: f64) -> String {
    let s = secs.max(0.0);
    format!("{}:{:04.1}", (s / 60.0) as u64, s % 60.0)
}
