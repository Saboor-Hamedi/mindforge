//! Subtle horizontal menu separator line component.

use eframe::egui::{self, pos2, Color32, Stroke, Ui};

pub const SEPARATOR_TOTAL_H: f32 = 7.0;

/// Renders a crisp 1px separator line inside a menu.
pub fn render_menu_separator(ui: &mut Ui, width: f32, color: Color32) {
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(width, SEPARATOR_TOTAL_H), egui::Sense::hover());

    let y = rect.center().y;
    let margin = 6.0;
    ui.painter().line_segment(
        [pos2(rect.min.x + margin, y), pos2(rect.max.x - margin, y)],
        Stroke::new(1.0, color),
    );
}
