//! Shared sleek toggle switch component.

use crate::ui::theme::Theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Pos2, Rect, Stroke};

/// Linearly interpolates two Color32 values.
fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).clamp(0.0, 255.0) as u8;
    Color32::from_rgba_unmultiplied(
        l(a.r(), b.r()),
        l(a.g(), b.g()),
        l(a.b(), b.b()),
        l(a.a(), b.a()),
    )
}

/// Renders a sleek, animated pill toggle switch.
/// Returns `true` if clicked and value was toggled.
pub fn render_toggle(
    ui: &egui::Ui,
    painter: &egui::Painter,
    rect: Rect,
    value: &mut bool,
    theme: &Theme,
    id_salt: impl std::hash::Hash,
) -> bool {
    let hovered = ui.rect_contains_pointer(rect);
    let on = *value;
    let anim_t = ui
        .ctx()
        .animate_bool_with_time(egui::Id::new(("toggle_anim", id_salt)), on, 0.15);

    if hovered {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    let h = rect.height();
    let r = h * 0.5;

    // Track background & stroke
    let off_bg = theme.surface();
    let on_bg = Color32::from_rgba_unmultiplied(
        theme.accent.r(),
        theme.accent.g(),
        theme.accent.b(),
        if theme.is_light() { 55 } else { 40 },
    );
    let track_bg = lerp_color(off_bg, on_bg, anim_t);

    let off_stroke = theme.border();
    let on_stroke = theme.accent;
    let stroke_color = lerp_color(off_stroke, on_stroke, anim_t);

    painter.rect(
        rect,
        r,
        track_bg,
        Stroke::new(1.0_f32, stroke_color),
        egui::StrokeKind::Inside,
    );

    // Animated knob
    let knob_r = (r - 2.5).max(3.0);
    let min_x = rect.min.x + knob_r + 3.0;
    let max_x = rect.max.x - knob_r - 3.0;
    let knob_x = min_x + (max_x - min_x) * anim_t;

    let knob_color = lerp_color(theme.muted, theme.accent, anim_t);
    painter.circle_filled(pos2(knob_x, rect.center().y), knob_r, knob_color);

    if hovered && ui.input(|i| i.pointer.primary_clicked()) {
        *value = !*value;
        true
    } else {
        false
    }
}

/// Renders a toggle switch preceded by an aligned right-anchored label.
pub fn render_toggle_with_label(
    ui: &egui::Ui,
    painter: &egui::Painter,
    pill_pos: Pos2,
    label: &str,
    value: &mut bool,
    theme: &Theme,
    id_salt: impl std::hash::Hash,
) -> bool {
    let pill_w = 40.0;
    let pill_h = 22.0;
    let pill_rect = Rect::from_min_size(pill_pos, vec2(pill_w, pill_h));

    painter.text(
        pos2(pill_rect.min.x - 10.0, pill_rect.center().y),
        Align2::RIGHT_CENTER,
        label,
        FontId::proportional(12.5),
        theme.text,
    );

    render_toggle(ui, painter, pill_rect, value, theme, id_salt)
}
