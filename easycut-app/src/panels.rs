// File: easycut-app/src/panels.rs
//! Pannello dei formati: ogni scelta mostra la dimensione stimata del file.
use crate::video::{Choice, Video};
use easycut_core::Speed;
use eframe::egui::{self, Color32, RichText, Sense};

pub fn fmt_bytes(b: u64) -> String {
    let b = b as f64;
    if b >= 1e9 {
        format!("{:.2} GB", b / 1e9)
    } else if b >= 1e6 {
        format!("{:.1} MB", b / 1e6)
    } else {
        format!("{:.0} KB", b / 1e3)
    }
}

/// Un riquadro cliccabile. Ritorna true se è stato cliccato.
fn card(ui: &mut egui::Ui, selected: bool, body: impl FnOnce(&mut egui::Ui)) -> bool {
    let fill = if selected {
        ui.visuals().selection.bg_fill.gamma_multiply(0.35)
    } else {
        ui.visuals().faint_bg_color
    };
    let stroke = if selected {
        ui.visuals().selection.stroke
    } else {
        ui.visuals().widgets.noninteractive.bg_stroke
    };
    let r = egui::Frame::group(ui.style())
        .fill(fill)
        .stroke(stroke)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            body(ui);
        });
    let resp = r.response.interact(Sense::click());
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp.clicked()
}

fn size_line(ui: &mut egui::Ui, est: u64, base: u64) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("~{}", fmt_bytes(est)))
                .size(20.0)
                .strong(),
        );
        if base > 0 {
            let pct = (1.0 - est as f64 / base as f64) * 100.0;
            let (txt, col) = if pct >= 0.5 {
                (format!("−{pct:.0}%"), Color32::from_rgb(80, 200, 120))
            } else {
                (format!("+{:.0}%", -pct), Color32::from_rgb(230, 150, 60))
            };
            ui.label(RichText::new(txt).color(col).strong());
        }
    });
}

pub enum PanelAction {
    None,
    MeasureAll,
    MeasureOne(usize),
}

/// `measuring` = (completati, totali) se una misurazione è in corso.
pub fn show(ui: &mut egui::Ui, v: &mut Video, measuring: Option<(usize, usize)>) -> PanelAction {
    let mut action = PanelAction::None;
    ui.heading("Formato e compressione");
    ui.label(
        RichText::new("Le dimensioni sono stime; il file reale può differire.")
            .small()
            .weak(),
    );
    ui.checkbox(&mut v.hardware, "Accelerazione hardware (più veloce)");
    ui.horizontal(|ui| {
        ui.label("Velocità:");
        for speed in [Speed::Fast, Speed::Balanced, Speed::Small] {
            ui.selectable_value(&mut v.speed, speed, speed.label());
        }
    })
    .response
    .on_hover_text(
        "Veloce: esporta prima, file un po' più grande.\nPiù piccolo: più lento, file più leggero.\nCon l'encoder hardware la differenza è minima.",
    );
    ui.horizontal(|ui| {
        let busy = measuring.is_some();
        if ui
            .add_enabled(
                !busy && !v.presets.is_empty(),
                egui::Button::new("Misura le dimensioni reali"),
            )
            .on_hover_text(
                "Codifica brevi campioni del tratto scelto per una stima molto più fedele",
            )
            .clicked()
        {
            action = PanelAction::MeasureAll;
        }
        if let Some((done, total)) = measuring {
            ui.spinner();
            ui.label(format!("{done}/{total}"));
        }
    });
    ui.add_space(4.0);

    let base = v.original_slice_bytes();
    egui::ScrollArea::vertical().show(ui, |ui| {
        if card(ui, v.choice == Choice::Copy, |ui| {
            ui.label(RichText::new("Solo taglio").strong());
            ui.label(RichText::new("Senza ricodifica: istantaneo, nessuna perdita. Il taglio cade sul keyframe più vicino.").small());
            ui.label(RichText::new(format!(".{} · stessa qualità", v.copy_ext())).small().weak());
            size_line(ui, v.estimate_for(Choice::Copy), base);
        }) {
            v.set_choice(Choice::Copy);
        }

        for i in 0..v.presets.len() {
            let selected_here = v.choice == Choice::Preset(i);
            let (name, desc, label) = {
                let p = &v.presets[i];
                (p.name, p.description, v.preset_label(i))
            };
            if card(ui, v.choice == Choice::Preset(i), |ui| {
                ui.label(RichText::new(name).strong());
                ui.label(RichText::new(desc).small());
                ui.label(RichText::new(label).small().weak());
                size_line(ui, v.estimate_for(Choice::Preset(i)), base);
                if v.measured.contains_key(&i) {
                    ui.label(RichText::new("misurata").small().color(Color32::from_rgb(80, 200, 120)));
                } else if selected_here
                    && measuring.is_none()
                    && ui.small_button("Misura questo").clicked()
                {
                    action = PanelAction::MeasureOne(i);
                }
            }) {
                v.set_choice(Choice::Preset(i));
            }
        }

        let sel = v.choice == Choice::Target;
        if card(ui, sel, |ui| {
            ui.label(RichText::new("Dimensione precisa").strong());
            ui.label(RichText::new("Scegli quanto deve pesare il file; la qualità si adatta.").small());
            ui.horizontal(|ui| {
                ui.add(
                    egui::DragValue::new(&mut v.target_mb)
                        .range(0.5..=100_000.0)
                        .speed(0.5)
                        .suffix(" MB"),
                );
                ui.label(RichText::new("H.265 · MP4").small().weak());
            });
            if sel {
                size_line(ui, v.estimate_for(Choice::Target), base);
            }
        }) {
            v.set_choice(Choice::Target);
        }
    });
    action
}
