// File: easycat-app/src/panels.rs
//! Three primary operations and settings for the selected operation.
use crate::{
    theme,
    video::{Choice, Video},
};
use easycat_core::{Container, Speed};
use eframe::egui::{self, RichText, Sense, Stroke};

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

fn card(ui: &mut egui::Ui, selected: bool, title: &str, subtitle: &str, icon: usize) -> bool {
    let response = egui::Frame::none()
        .fill(if selected {
            theme::MINT.gamma_multiply(0.12)
        } else {
            theme::SURFACE
        })
        .stroke(Stroke::new(
            if selected { 2.0_f32 } else { 1.0_f32 },
            if selected { theme::MINT } else { theme::LINE },
        ))
        .rounding(12.0)
        .inner_margin(4.0)
        .show(ui, |ui| {
            ui.set_min_width((ui.available_width() - 2.0).max(0.0));
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), Sense::hover());
                theme::operation_icon(ui.painter(), rect, icon);
                ui.vertical(|ui| {
                    ui.set_max_width((ui.available_width() - 22.0).max(100.0));
                    ui.label(
                        RichText::new(title)
                            .family(egui::FontFamily::Name("display".into()))
                            .size(17.0),
                    );
                    ui.label(RichText::new(subtitle).color(theme::MUTED).small());
                });
            });
            let center = egui::pos2(ui.max_rect().right() - 9.0, ui.min_rect().center().y);
            ui.painter().circle_stroke(
                center,
                8.0,
                Stroke::new(1.8_f32, if selected { theme::MINT } else { theme::LILAC }),
            );
            if selected {
                ui.painter().circle_filled(center, 4.5, theme::MINT);
            }
        })
        .response;
    response
        .interact(Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
}

pub fn size_line(ui: &mut egui::Ui, est: u64, base: u64) {
    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new(format!("≈ {}", fmt_bytes(est)))
                .family(egui::FontFamily::Name("display".into()))
                .size(if ui.ctx().screen_rect().height() < 600.0 {
                    26.0
                } else {
                    36.0
                })
                .strong(),
        );
        if base > 0 {
            let pct = (1.0 - est as f64 / base as f64) * 100.0;
            if pct.abs() >= 0.5 {
                let text = if pct > 0.0 {
                    format!("−{pct:.0}%")
                } else {
                    format!("+{:.0}%", -pct)
                };
                ui.label(
                    RichText::new(text)
                        .color(if pct > 0.0 { theme::MINT } else { theme::CORAL })
                        .strong(),
                );
            }
        }
    });
}

const OPERATIONS: [&str; 3] = ["Trim only", "Change container", "Compress"];

/// Shapes offered for the crop rectangle; `None` is free.
const CROP_SHAPES: [(&str, Option<(u32, u32)>); 5] = [
    ("Free", None),
    ("1:1", Some((1, 1))),
    ("16:9", Some((16, 9))),
    ("9:16", Some((9, 16))),
    ("4:5", Some((4, 5))),
];

fn operation_index(choice: Choice) -> usize {
    match choice {
        Choice::Copy => 0,
        Choice::Remux(_) => 1,
        Choice::Preset(_) | Choice::Target => 2,
    }
}

pub enum PanelAction {
    None,
    MeasureAll,
    MeasureOne(usize),
}

pub fn show(ui: &mut egui::Ui, v: &mut Video) {
    let compact = ui.ctx().screen_rect().height() < 820.0;
    if compact {
        let mut operation = operation_index(v.choice);
        // Crop shares the row with the operation: space is tight here.
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("operation")
                .width((ui.available_width() - 120.0).max(120.0))
                .selected_text(OPERATIONS[operation])
                .show_ui(ui, |ui| {
                    for (index, name) in OPERATIONS.into_iter().enumerate() {
                        ui.selectable_value(&mut operation, index, name);
                    }
                });
            crop_toggle(ui, v);
        });
        let current = operation_index(v.choice);
        if operation != current {
            let choice = match operation {
                0 => Choice::Copy,
                1 => Choice::Remux(
                    [Container::Mp4, Container::Mkv, Container::Webm]
                        .into_iter()
                        .find(|c| easycat_core::job::validate_remux(&v.info, *c).is_ok())
                        .unwrap_or(Container::Mkv),
                ),
                _ if v.presets.is_empty() => Choice::Target,
                _ => Choice::Preset(0),
            };
            v.set_choice(choice);
        }
    } else {
        // Keep the cards close so the settings below stay on screen.
        ui.spacing_mut().item_spacing.y = 4.0;
        if card(
            ui,
            v.choice == Choice::Copy,
            "Trim only",
            "Fast, no quality loss",
            0,
        ) {
            v.set_choice(Choice::Copy);
        }
        let remux = matches!(v.choice, Choice::Remux(_));
        if card(ui, remux, "Change container", "Same quality, new format", 1) && !remux {
            let containers = [Container::Mp4, Container::Mkv, Container::Webm];
            let preferred = containers
                .into_iter()
                .find(|c| easycat_core::job::validate_remux(&v.info, *c).is_ok())
                .or_else(|| {
                    containers.into_iter().find(|c| {
                        easycat_core::job::remux_issues(&v.info, *c)
                            .iter()
                            .all(|issue| issue.removable)
                    })
                })
                .unwrap_or(Container::Mkv);
            v.set_choice(Choice::Remux(preferred));
        }
        let compressing = matches!(v.choice, Choice::Preset(_) | Choice::Target);
        if card(ui, compressing, "Compress", "A smaller file", 2) && !compressing {
            v.set_choice(if v.presets.is_empty() {
                Choice::Target
            } else {
                Choice::Preset(0)
            });
        }
    }
    if !compact {
        ui.separator();
        crop_toggle(ui, v);
    }
    crop_options(ui, v, compact);
    match v.choice {
        Choice::Copy if v.crop_on => {
            ui.label(
                RichText::new(format!(".{} · re-encoded to apply the crop", v.out_ext()))
                    .color(theme::MINT),
            );
        }
        Choice::Copy => {
            ui.label(
                RichText::new(format!(
                    ".{} · original quality and resolution",
                    v.copy_ext()
                ))
                .color(theme::MINT),
            );
            ui.label(
                RichText::new("Cuts snap to keyframes, not individual frames.")
                    .small()
                    .color(theme::MUTED),
            );
        }
        Choice::Remux(container) => {
            if !compact {
                ui.label("Output format");
            }
            ui.horizontal_wrapped(|ui| {
                for c in [Container::Mp4, Container::Mkv, Container::Webm] {
                    // Cropping re-encodes to H.264/H.265, which WebM cannot hold.
                    if v.crop_on && c == Container::Webm {
                        continue;
                    }
                    if ui
                        .add(
                            egui::Button::new(c.ext().to_uppercase())
                                .selected(v.choice == Choice::Remux(c)),
                        )
                        .clicked()
                    {
                        v.set_choice(Choice::Remux(c));
                    }
                }
            });
            let mut tracks = |ui: &mut egui::Ui| {
                let issues = easycat_core::job::remux_issues(&v.info, container);
                if issues.is_empty() {
                    ui.label(
                        RichText::new("All tracks can be copied without re-encoding.")
                            .small()
                            .color(theme::MINT),
                    );
                } else {
                    ui.label(
                        RichText::new("Incompatible tracks")
                            .strong()
                            .color(theme::CORAL),
                    );
                    for issue in issues {
                        ui.label(issue.description());
                        if issue.removable {
                            let mut excluded = v.remux_excluded.contains(&issue.index);
                            if ui
                                .checkbox(&mut excluded, "Exclude this optional track")
                                .changed()
                            {
                                if excluded {
                                    v.remux_excluded.push(issue.index);
                                } else {
                                    v.remux_excluded.retain(|index| *index != issue.index);
                                }
                            }
                            if excluded {
                                ui.label(
                                    RichText::new("This track will not be in the output.")
                                        .small()
                                        .color(theme::CORAL),
                                );
                            }
                        } else {
                            ui.label(RichText::new("Video/audio cannot be excluded. Choose another container or use Compress.").small().color(theme::CORAL));
                        }
                    }
                }
                ui.label(RichText::new("All video and audio tracks are preserved. Only explicitly selected optional tracks are omitted. Trims use keyframes.").small().color(theme::MUTED));
            };
            ui.menu_button("Track compatibility", |ui| tracks(ui));
        }
        Choice::Preset(_) | Choice::Target => {
            if !compact {
                ui.label("Export profile");
            }
            let selected = match v.choice {
                Choice::Preset(i) => v.presets[i].name,
                _ => "Target size",
            };
            egui::ComboBox::from_id_salt("profile")
                .selected_text(selected)
                .width(ui.available_width() - 8.0)
                .show_ui(ui, |ui| {
                    for i in 0..v.presets.len() {
                        if ui
                            .selectable_label(v.choice == Choice::Preset(i), v.presets[i].name)
                            .clicked()
                        {
                            v.set_choice(Choice::Preset(i));
                        }
                    }
                    if ui
                        .selectable_label(v.choice == Choice::Target, "Target size")
                        .clicked()
                    {
                        v.set_choice(Choice::Target);
                    }
                });
            if let Choice::Preset(i) = v.choice {
                if !compact {
                    ui.label(RichText::new(v.preset_label(i)).small().color(theme::MUTED))
                        .on_hover_text(v.presets[i].description);
                }
            } else {
                ui.horizontal(|ui| {
                    ui.label("Size");
                    ui.add(
                        egui::DragValue::new(&mut v.target_mb)
                            .range(0.5..=100_000.0)
                            .speed(0.5)
                            .suffix(" MB"),
                    );
                });
                if !compact {
                    ui.label(
                        RichText::new("Quality adjusts to the requested size.")
                            .small()
                            .color(theme::MUTED),
                    );
                }
            }
            if !compact {
                ui.label("Speed");
            }
            egui::ComboBox::from_id_salt("speed")
                .selected_text(v.speed.label())
                .width(ui.available_width() - 8.0)
                .show_ui(ui, |ui| {
                    for speed in [Speed::Fast, Speed::Balanced, Speed::Small] {
                        ui.selectable_value(&mut v.speed, speed, speed.label());
                    }
                });
            ui.add_enabled(cfg!(target_os = "macos"), egui::Checkbox::new(&mut v.hardware, "Hardware acceleration"))
                .on_hover_text("Hardware export is currently available on macOS. Windows and Linux use software encoding.");
        }
    }
}

/// Crop sits beside the trim selection: it combines with any operation. The
/// rectangle is dragged on the preview; the shape is picked here.
fn crop_toggle(ui: &mut egui::Ui, v: &mut Video) {
    let label = if v.crop_on {
        format!("Crop {}×{}", v.crop.width, v.crop.height)
    } else {
        "Crop".into()
    };
    if ui.checkbox(&mut v.crop_on, label).changed() {
        v.remux_excluded.clear();
    }
}

fn crop_options(ui: &mut egui::Ui, v: &mut Video, compact: bool) {
    if !v.crop_on {
        return;
    }
    if compact {
        // A narrow panel would wrap six buttons onto two rows.
        let selected = CROP_SHAPES
            .iter()
            .find(|(_, ratio)| *ratio == v.crop_ratio)
            .map_or("Free", |(name, _)| *name);
        egui::ComboBox::from_id_salt("crop-shape")
            .selected_text(selected)
            .width(ui.available_width() - 8.0)
            .show_ui(ui, |ui| {
                for (name, ratio) in CROP_SHAPES {
                    if ui.selectable_label(v.crop_ratio == ratio, name).clicked() {
                        v.set_crop_ratio(ratio);
                    }
                }
                if ui.selectable_label(false, "Full frame").clicked() {
                    v.reset_crop();
                }
            });
    } else {
        ui.horizontal_wrapped(|ui| {
            // Six buttons in one row: keep them snug.
            ui.spacing_mut().item_spacing = egui::vec2(4.0, 2.0);
            ui.spacing_mut().button_padding = egui::vec2(6.0, 3.0);
            for (name, ratio) in CROP_SHAPES {
                if ui
                    .add(egui::Button::new(name).selected(v.crop_ratio == ratio))
                    .clicked()
                {
                    v.set_crop_ratio(ratio);
                }
            }
            if ui.button("Full frame").clicked() {
                v.reset_crop();
            }
        });
    }
    if matches!(v.choice, Choice::Copy | Choice::Remux(_)) {
        egui::ComboBox::from_id_salt("speed")
            .selected_text(v.speed.label())
            .width(ui.available_width() - 8.0)
            .show_ui(ui, |ui| {
                for speed in [Speed::Fast, Speed::Balanced, Speed::Small] {
                    ui.selectable_value(&mut v.speed, speed, speed.label());
                }
            });
    }
}

pub fn estimate_options(ui: &mut egui::Ui, v: &Video, measuring: bool) -> PanelAction {
    let mut action = PanelAction::None;
    if measuring {
        ui.label("Measurement in progress…");
    } else {
        if let Choice::Preset(i) = v.choice {
            if ui.button("Refine this estimate").clicked() {
                action = PanelAction::MeasureOne(i);
                ui.close_menu();
            }
        }
        if ui.button("Measure all profiles").clicked() {
            action = PanelAction::MeasureAll;
            ui.close_menu();
        }
    }
    action
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crop_shapes_start_with_free_and_are_distinct() {
        assert_eq!(CROP_SHAPES[0].1, None);
        for (i, a) in CROP_SHAPES.iter().enumerate() {
            for b in &CROP_SHAPES[i + 1..] {
                assert_ne!(a.1, b.1);
            }
        }
    }

    #[test]
    fn every_choice_maps_to_one_operation_card() {
        assert_eq!(operation_index(Choice::Copy), 0);
        assert_eq!(operation_index(Choice::Remux(Container::Mp4)), 1);
        assert_eq!(operation_index(Choice::Target), 2);
        assert_eq!(operation_index(Choice::Preset(0)), 2);
    }
}
