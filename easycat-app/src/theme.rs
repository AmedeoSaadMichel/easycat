// File: easycat-app/src/theme.rs
//! easycat identity: cosmic night, pastel colors and a three-eyed mascot.
use eframe::egui::{self, pos2, vec2, Color32, FontId, Rect, RichText, Shape, Stroke};

pub const BG: Color32 = Color32::from_rgb(25, 17, 35);
pub const PANEL: Color32 = Color32::from_rgb(33, 23, 44);
pub const SURFACE: Color32 = Color32::from_rgb(43, 30, 56);
pub const LILAC: Color32 = Color32::from_rgb(187, 146, 229);
pub const CREAM: Color32 = Color32::from_rgb(255, 239, 185);
pub const MUTED: Color32 = Color32::from_rgb(190, 165, 211);
pub const MINT: Color32 = Color32::from_rgb(137, 225, 180);
pub const CORAL: Color32 = Color32::from_rgb(255, 122, 155);
pub const YELLOW: Color32 = Color32::from_rgb(255, 225, 103);
pub const LINE: Color32 = Color32::from_rgb(89, 65, 109);

pub fn install(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "lilita".into(),
        egui::FontData::from_static(include_bytes!("../assets/LilitaOne-Regular.ttf")),
    );
    let mut display = vec!["lilita".into()];
    display.extend(
        fonts
            .families
            .get(&egui::FontFamily::Proportional)
            .cloned()
            .unwrap_or_default(),
    );
    fonts
        .families
        .insert(egui::FontFamily::Name("display".into()), display);
    ctx.set_fonts(fonts);
    let mut style = (*ctx.style()).clone();
    let mut v = egui::Visuals::dark();
    v.panel_fill = PANEL;
    v.window_fill = BG;
    v.extreme_bg_color = BG;
    v.faint_bg_color = SURFACE;
    v.override_text_color = Some(CREAM);
    v.selection.bg_fill = MINT.gamma_multiply(0.25);
    v.selection.stroke = Stroke::new(1.5_f32, MINT);
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, LINE);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, MUTED);
    for widget in [
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
    ] {
        widget.rounding = egui::Rounding::same(9.0);
        widget.fg_stroke = Stroke::new(1.0_f32, CREAM);
    }
    v.widgets.inactive.bg_fill = SURFACE;
    v.widgets.inactive.weak_bg_fill = SURFACE;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, LINE);
    v.widgets.hovered.bg_fill = Color32::from_rgb(65, 43, 82);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, LILAC);
    v.widgets.active.bg_fill = Color32::from_rgb(82, 53, 101);
    v.widgets.active.bg_stroke = Stroke::new(1.5_f32, MINT);
    style.visuals = v;
    style.spacing.item_spacing = vec2(10.0, 8.0);
    style.spacing.button_padding = vec2(14.0, 9.0);
    style.spacing.interact_size.y = 34.0;
    style
        .text_styles
        .insert(egui::TextStyle::Body, FontId::proportional(15.0));
    style
        .text_styles
        .insert(egui::TextStyle::Button, FontId::proportional(15.0));
    style.text_styles.insert(
        egui::TextStyle::Heading,
        FontId::new(24.0, egui::FontFamily::Name("display".into())),
    );
    style
        .text_styles
        .insert(egui::TextStyle::Small, FontId::proportional(12.0));
    ctx.set_style(style);
}

pub fn primary(text: &str) -> egui::Button<'_> {
    egui::Button::new(
        RichText::new(text)
            .family(egui::FontFamily::Name("display".into()))
            .size(22.0)
            .strong()
            .color(BG),
    )
    .fill(YELLOW)
    .rounding(12.0)
    .min_size(vec2(180.0, 50.0))
}

pub fn cat(p: &egui::Painter, rect: Rect) {
    let c = rect.center();
    let s = rect.width().min(rect.height()) / 2.0;
    let point = |x: f32, y: f32| c + vec2(x * s, y * s);
    let ink = Stroke::new(2.0_f32, BG);
    p.add(Shape::convex_polygon(
        vec![
            point(-0.82, -0.1),
            point(-0.85, -0.95),
            point(-0.27, -0.5),
            point(0.27, -0.5),
            point(0.85, -0.95),
            point(0.82, -0.1),
        ],
        CORAL,
        ink,
    ));
    p.add(Shape::ellipse_filled(
        c + vec2(0.0, s * 0.1),
        vec2(s * 0.91, s * 0.72),
        CORAL,
    ));
    for (x, y, scale) in [(-0.38, -0.01, 1.0), (0.38, -0.01, 1.0), (0.0, -0.47, 0.72)] {
        let eye = point(x, y);
        p.add(Shape::ellipse_filled(
            eye,
            vec2(s * 0.23 * scale, s * 0.26 * scale),
            CREAM,
        ));
        p.line_segment(
            [
                eye - vec2(0.0, s * 0.18 * scale),
                eye + vec2(0.0, s * 0.18 * scale),
            ],
            Stroke::new(3.0_f32, BG),
        );
    }
    p.circle_filled(point(0.0, 0.31), s * 0.065, BG);
    for side in [-1.0, 1.0] {
        p.line_segment([point(side * 0.58, 0.28), point(side * 1.05, 0.18)], ink);
        p.line_segment([point(side * 0.58, 0.4), point(side * 1.05, 0.47)], ink);
    }
}

pub fn stars(p: &egui::Painter, rect: Rect) {
    for (x, y, size) in [
        (0.58, 0.22, 4.0),
        (0.72, 0.65, 3.0),
        (0.88, 0.28, 5.0),
        (0.46, 0.8, 2.0),
    ] {
        let c = pos2(
            rect.left() + rect.width() * x,
            rect.top() + rect.height() * y,
        );
        p.line_segment(
            [c - vec2(size, 0.0), c + vec2(size, 0.0)],
            Stroke::new(1.0_f32, YELLOW),
        );
        p.line_segment(
            [c - vec2(0.0, size * 1.6), c + vec2(0.0, size * 1.6)],
            Stroke::new(1.0_f32, YELLOW),
        );
    }
}

/// Limit texture dimensions before the first native frame.
pub fn load_header(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    load_artwork(
        ctx,
        "mockup",
        include_bytes!("../assets/approved-mockup.png"),
    )
}

pub fn load_brand(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    load_artwork(
        ctx,
        "retina-brand",
        include_bytes!("../assets/retina-brand.png"),
    )
}

pub fn load_decorations(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    load_artwork(
        ctx,
        "decorations-v2",
        include_bytes!("../assets/decorations-v2.png"),
    )
}

pub fn load_footer(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    load_artwork(
        ctx,
        "footer-panorama-v3",
        include_bytes!("../assets/footer-panorama-v3.png"),
    )
}

fn load_artwork(ctx: &egui::Context, name: &str, bytes: &[u8]) -> Option<egui::TextureHandle> {
    let image = eframe::icon_data::from_png_bytes(bytes).ok()?;
    let limit = ctx.input(|i| i.max_texture_side).min(2048);
    let width = (image.width as usize).min(limit);
    let height = ((image.height as usize * width) / image.width as usize).max(1);
    if width == image.width as usize && height == image.height as usize {
        return Some(ctx.load_texture(
            name,
            egui::ColorImage::from_rgba_unmultiplied([width, height], &image.rgba),
            egui::TextureOptions::LINEAR,
        ));
    }
    let mut pixels = Vec::with_capacity(width * height);
    for y in 0..height {
        for x in 0..width {
            let sx = ((x as f32 + 0.5) * image.width as f32 / width as f32 - 0.5).max(0.0);
            let sy = ((y as f32 + 0.5) * image.height as f32 / height as f32 - 0.5).max(0.0);
            let (x0, y0) = (sx.floor() as usize, sy.floor() as usize);
            let (fx, fy) = (sx.fract(), sy.fract());
            let mut rgba = [0.0; 4];
            for (dx, dy, weight) in [
                (0, 0, (1.0 - fx) * (1.0 - fy)),
                (1, 0, fx * (1.0 - fy)),
                (0, 1, (1.0 - fx) * fy),
                (1, 1, fx * fy),
            ] {
                let i = ((y0 + dy).min(image.height as usize - 1) * image.width as usize
                    + (x0 + dx).min(image.width as usize - 1))
                    * 4;
                for (channel, value) in rgba.iter_mut().enumerate() {
                    *value += image.rgba[i + channel] as f32 * weight;
                }
            }
            pixels.push(Color32::from_rgba_unmultiplied(
                rgba[0].round() as u8,
                rgba[1].round() as u8,
                rgba[2].round() as u8,
                rgba[3].round() as u8,
            ));
        }
    }
    Some(ctx.load_texture(
        name,
        egui::ColorImage {
            size: [width, height],
            pixels,
        },
        egui::TextureOptions::LINEAR,
    ))
}

/// Original mockup coordinates: sample the approved artwork directly.
pub fn art_uv(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_min_max(
        pos2(x / 1586.0, y / 992.0),
        pos2((x + width) / 1586.0, (y + height) / 992.0),
    )
}

pub fn video_rect(canvas: Rect, video: egui::Vec2) -> Rect {
    let scale = (canvas.width() / video.x.max(1.0)).min(canvas.height() / video.y.max(1.0));
    Rect::from_center_size(canvas.center(), video * scale)
}

/// Mockup artwork behind portrait video instead of black side borders.
pub fn video_background(p: &egui::Painter, texture: &egui::TextureHandle, canvas: Rect) {
    let mut uv = art_uv(28.0, 182.0, 1058.0, 466.0);
    let source_aspect = 1058.0 / 466.0;
    let target_aspect = canvas.width() / canvas.height().max(1.0);
    let mut size = uv.size();
    if target_aspect > source_aspect {
        size.y *= source_aspect / target_aspect;
    } else {
        size.x *= target_aspect / source_aspect;
    }
    uv = Rect::from_center_size(uv.center(), size);
    p.image(texture.id(), canvas, uv, Color32::WHITE);
}

pub fn paw(p: &egui::Painter, center: egui::Pos2, color: Color32) {
    p.add(Shape::ellipse_filled(
        center + vec2(0.0, 5.0),
        vec2(8.0, 6.5),
        color,
    ));
    for (x, y) in [(-10.0, -2.0), (-4.0, -9.0), (4.0, -9.0), (10.0, -2.0)] {
        p.add(Shape::ellipse_filled(
            center + vec2(x, y),
            vec2(3.4, 4.5),
            color,
        ));
    }
}

pub fn operation_icon(p: &egui::Painter, rect: Rect, operation: usize) {
    let c = rect.center();
    let stroke = Stroke::new(2.0_f32, CREAM);
    if operation == 0 {
        for side in [-1.0, 1.0] {
            p.circle_stroke(c + vec2(side * 7.0, 9.0), 4.0, stroke);
            p.line_segment(
                [c + vec2(side * 5.0, 5.0), c + vec2(-side * 10.0, -12.0)],
                stroke,
            );
        }
    } else {
        let r = Rect::from_center_size(c, vec2(22.0, 28.0));
        if operation == 1 {
            p.add(Shape::closed_line(
                vec![
                    r.left_top(),
                    r.right_top() - vec2(7.0, 0.0),
                    r.right_top() + vec2(0.0, 7.0),
                    r.right_bottom(),
                    r.left_bottom(),
                ],
                stroke,
            ));
            p.line_segment(
                [
                    r.right_top() - vec2(7.0, 0.0),
                    r.right_top() + vec2(-7.0, 7.0),
                ],
                stroke,
            );
            p.line_segment(
                [
                    r.right_top() + vec2(-7.0, 7.0),
                    r.right_top() + vec2(0.0, 7.0),
                ],
                stroke,
            );
        } else {
            p.add(Shape::line(
                vec![
                    c + vec2(-12.0, 1.0),
                    c + vec2(-12.0, 13.0),
                    c + vec2(12.0, 13.0),
                    c + vec2(12.0, 1.0),
                ],
                stroke,
            ));
            let arrow = Stroke::new(2.5_f32, MINT);
            p.line_segment([c - vec2(0.0, 15.0), c + vec2(0.0, 3.0)], arrow);
            p.add(Shape::line(
                vec![
                    c + vec2(-6.0, -3.0),
                    c + vec2(0.0, 3.0),
                    c + vec2(6.0, -3.0),
                ],
                arrow,
            ));
        }
    }
}

/// Crop a texture region to cover a target without stretching, preserving its bottom edge.
pub fn cover_uv(region: Rect, texture: egui::Vec2, target: egui::Vec2) -> Rect {
    let source = region.size() * texture;
    let aspect = target.x / target.y.max(1.0);
    let source_aspect = source.x / source.y.max(1.0);
    if aspect > source_aspect {
        let height = region.height() * source_aspect / aspect;
        Rect::from_min_max(pos2(region.left(), region.bottom() - height), region.max)
    } else {
        Rect::from_center_size(
            region.center(),
            vec2(region.width() * aspect / source_aspect, region.height()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn header_cover_preserves_artwork_proportions() {
        let texture = vec2(2172.0, 724.0);
        let region = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 0.345));
        for target in [vec2(1280.0, 88.0), vec2(800.0, 88.0)] {
            let uv = cover_uv(region, texture, target);
            assert!(region.contains_rect(uv));
            let pixels = uv.size() * texture;
            assert!((pixels.x / pixels.y - target.x / target.y).abs() < 0.001);
        }
    }

    #[test]
    fn portrait_preview_is_centered_and_uncropped() {
        let canvas = Rect::from_min_size(pos2(20.0, 40.0), vec2(800.0, 450.0));
        let video = video_rect(canvas, vec2(2160.0, 3840.0));
        assert_eq!(video.center(), canvas.center());
        assert!((video.height() - 450.0).abs() < 0.01);
        assert!((video.width() / video.height() - 9.0 / 16.0).abs() < 0.001);
        assert!(video.left() > canvas.left() && video.right() < canvas.right());
        assert_eq!(video_rect(canvas, vec2(1920.0, 1080.0)), canvas);
    }
}
