//! Interactive task checkbox element rendering.

use crate::ui::theme::Theme;
use eframe::egui::{pos2, Color32, Painter, Pos2, Rect, Stroke};

/// Renders an interactive vector checkbox for markdown task list items (- [ ] / - [x]).
pub fn render_task_checkbox(
    painter: &Painter,
    box_rect: Rect,
    checked: bool,
    mouse_pos: Option<Pos2>,
    theme: &Theme,
) {
    let is_hovered = mouse_pos.map_or(false, |p| box_rect.expand(4.0).contains(p));
    let radius = 3.5;

    if checked {
        // Beautiful vibrant emerald task green (Apple / GitHub / Notion standard)
        let green_bg = Color32::from_rgb(46, 204, 113);
        painter.rect_filled(box_rect, radius, green_bg);
        // Crisp high-contrast white checkmark
        let check_color = Color32::WHITE;
        let p1 = pos2(box_rect.min.x + 3.2, box_rect.min.y + 7.2);
        let p2 = pos2(box_rect.min.x + 6.2, box_rect.min.y + 10.8);
        let p3 = pos2(box_rect.min.x + 11.5, box_rect.min.y + 3.8);
        painter.line_segment([p1, p2], Stroke::new(2.0_f32, check_color));
        painter.line_segment([p2, p3], Stroke::new(2.0_f32, check_color));
    } else {
        // Empty outlined box with smooth hover glow
        let border_color = if is_hovered { theme.accent } else { theme.border() };
        let fill_color = if is_hovered {
            Color32::from_rgba_unmultiplied(
                theme.accent.r(),
                theme.accent.g(),
                theme.accent.b(),
                30,
            )
        } else {
            Color32::TRANSPARENT
        };
        painter.rect(
            box_rect,
            radius,
            fill_color,
            Stroke::new(1.3_f32, border_color),
            eframe::egui::StrokeKind::Inside,
        );
    }
}
