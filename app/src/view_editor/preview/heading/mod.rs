//! Preview heading rendering module with proportional typography and generous line height.
//!
//! Provides document-grade reading rhythm with unified theme palette colors,
//! distinct from the monospace CLI editor.

use crate::ui::theme::Theme;
use eframe::egui::{pos2, Color32, FontId, Painter, Rect};

/// Returns proportional font sizing and line height metrics for preview headings.
pub fn preview_heading_metrics(level: u8, base_font_size: f32) -> (FontId, f32) {
    match level {
        1 => (crate::services::font_manager::editor_font_id(base_font_size * 1.50), (base_font_size * 2.1).round()),
        2 => (crate::services::font_manager::editor_font_id(base_font_size * 1.30), (base_font_size * 1.85).round()),
        3 => (crate::services::font_manager::editor_font_id(base_font_size * 1.18), (base_font_size * 1.70).round()),
        _ => (crate::services::font_manager::editor_font_id(base_font_size * 1.08), (base_font_size * 1.60).round()),
    }
}

/// Returns the heading text color resolving strictly to the active theme's palette.
pub fn preview_heading_color(_level: u8, theme: &Theme) -> Color32 {
    theme.text
}

/// Renders a markdown heading (levels 1..=6) with proportional typography and comfortable spacing.
///
/// Returns the vertical height consumed by the rendered heading block (including margins).
pub fn render_preview_heading(
    painter: &Painter,
    content_painter: &Painter,
    start_x: f32,
    current_y: f32,
    max_text_w: f32,
    font_size: f32,
    level: u8,
    text: &str,
    theme: &Theme,
    b_idx: usize,
    viewport_rect: Rect,
) -> f32 {
    let (font, line_h) = preview_heading_metrics(level, font_size);
    let color = preview_heading_color(level, theme);

    // Vertical top margin before heading (suppressed for the very first block)
    let margin_top = if b_idx == 0 {
        0.0
    } else {
        match level {
            1 => 12.0,
            2 => 10.0,
            3 => 8.0,
            _ => 6.0,
        }
    };

    let galley = painter.layout(text.to_string(), font, color, max_text_w);
    let block_y = current_y + margin_top;
    let actual_h = line_h.max(galley.size().y);

    // Viewport frustum culling
    if block_y + actual_h >= viewport_rect.min.y && block_y <= viewport_rect.max.y {
        let y_pad = ((actual_h - galley.size().y) * 0.5).round().max(0.0);
        content_painter.galley(pos2(start_x, block_y + y_pad), galley, color);
    }

    let margin_bottom = match level {
        1 => 8.0,
        2 => 6.0,
        3 => 5.0,
        _ => 4.0,
    };

    margin_top + actual_h + margin_bottom
}
