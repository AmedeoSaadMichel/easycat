// File: easycut-app/src/controls.rs
//! Controlli di riproduzione (icone disegnate a mano) e scorciatoie da tastiera.
use eframe::egui::{self, pos2, vec2, Color32, Key, Pos2, Rect, Sense, Shape, Stroke};

#[derive(Clone, Copy, PartialEq)]
pub enum Action {
    None,
    TogglePlay,
    /// Vai a un istante; se `true` e il video è in riproduzione, continua da lì.
    Seek(f64, bool),
    /// Sposta la posizione di `dt` secondi e mette in pausa.
    Step(f64),
    ToStart,
    ToEnd,
    ToggleMute,
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
}

fn tri(painter: &egui::Painter, pts: [Pos2; 3], c: Color32) {
    painter.add(Shape::convex_polygon(pts.to_vec(), c, Stroke::NONE));
}

fn icon_button(ui: &mut egui::Ui, icon: Icon, tip: &str, wide: bool) -> bool {
    let size = vec2(if wide { 56.0 } else { 38.0 }, 32.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let vis = ui.style().interact(&resp);
    let painter = ui.painter();
    painter.rect(rect, 6.0, vis.bg_fill, vis.bg_stroke);
    let c = vis.fg_stroke.color;
    let (x, y) = (rect.center().x, rect.center().y);
    let bar = |dx: f32| Rect::from_center_size(pos2(x + dx, y), vec2(3.0, 14.0));
    match icon {
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

/// Riga dei bottoni. Ritorna l'azione richiesta.
pub fn show(ui: &mut egui::Ui, playing: bool, muted: bool, frame: f64) -> Action {
    let mut act = Action::None;
    ui.horizontal(|ui| {
        ui.add_space(((ui.available_width() - 330.0) / 2.0).max(0.0));
        if icon_button(ui, Icon::Start, "Vai all'inizio del taglio", false) {
            act = Action::ToStart;
        }
        if icon_button(
            ui,
            Icon::Back,
            "Un fotogramma indietro (←, Maiusc = 1 s)",
            false,
        ) {
            act = Action::Step(-frame);
        }
        let (icon, tip) = if playing {
            (Icon::Pause, "Pausa (Spazio)")
        } else {
            (Icon::Play, "Riproduci (Spazio)")
        };
        if icon_button(ui, icon, tip, true) {
            act = Action::TogglePlay;
        }
        if icon_button(
            ui,
            Icon::Fwd,
            "Un fotogramma avanti (→, Maiusc = 1 s)",
            false,
        ) {
            act = Action::Step(frame);
        }
        if icon_button(ui, Icon::End, "Vai alla fine del taglio", false) {
            act = Action::ToEnd;
        }
        ui.add_space(8.0);
        if ui
            .selectable_label(!muted, "Audio")
            .on_hover_text("Attiva/disattiva l'audio")
            .clicked()
        {
            act = Action::ToggleMute;
        }
    });
    act
}

/// Scorciatoie: Spazio, ←/→, I (inizio), O (fine). Ignorate mentre si scrive.
pub fn shortcuts(ctx: &egui::Context, frame: f64) -> Action {
    if ctx.wants_keyboard_input() {
        return Action::None;
    }
    ctx.input(|i| {
        let step = if i.modifiers.shift { 1.0 } else { frame };
        if i.key_pressed(Key::Space) {
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
