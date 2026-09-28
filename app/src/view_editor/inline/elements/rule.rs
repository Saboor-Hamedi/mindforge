//! Horizontal divider rule element rendering and metrics.

use crate::ui::theme::Theme;
use eframe::egui::{pos2, FontId, Painter, Stroke};

/// Returns font and line height metrics for horizontal rules.
#[allow(dead_code)]
pub fn rule_metrics(base_font_size: f32) -> (FontId, f32) {
    (FontId::monospace(base_font_size * 0.8), 18.0)
}

/// Renders a crisp horizontal rule divider when the line is inactive.
pub fn render_horizontal_rule(
    painter: &Painter,
    line_y: f32,
    line_h: f32,
    text_left: f32,
    content_right: f32,
    theme: &Theme,
    is_active: bool,
) {
    if !is_active {
        let mid_y = (line_y + line_h * 0.5).round();
        painter.line_segment(
            [pos2(text_left - 4.0, mid_y), pos2(content_right - 8.0, mid_y)],
            Stroke::new(1.0_f32, theme.border()),
        );
    }
}
