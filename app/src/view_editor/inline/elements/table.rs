//! Markdown table styling and decorations for preview.

use crate::ui::theme::Theme;
use eframe::egui::{Color32, CornerRadius, FontId, Painter, Rect, Stroke, StrokeKind};

/// Returns font and line height metrics for table rows.
pub fn table_metrics(
    base_font_size: f32,
    is_header: bool,
    is_separator: bool,
    is_active: bool,
) -> (FontId, f32) {
    if is_separator {
        if is_active {
            (
                crate::services::font_manager::editor_font_id(base_font_size * 0.9),
                (base_font_size * 1.4).round(),
            )
        } else {
            (
                crate::services::font_manager::editor_font_id(base_font_size * 0.7),
                1.0,
            )
        }
    } else if is_active {
        (
            crate::services::font_manager::editor_font_id(base_font_size),
            (base_font_size * 1.55).round(),
        )
    } else if is_header {
        (
            crate::services::font_manager::editor_font_id(base_font_size * 0.95),
            (base_font_size * 1.6).round() + 8.0,
        )
    } else {
        (
            crate::services::font_manager::editor_font_id(base_font_size * 0.90),
            (base_font_size * 1.5).round() + 6.0,
        )
    }
}

/// Returns the cell text color.
pub fn cell_color(theme: &Theme, _is_header: bool) -> Color32 {
    theme.text
}

/// Renders table outer card stroke, header highlight pill, and alternating row backgrounds.
pub fn render_table_block_decorations(
    painter: &Painter,
    table_rect: Rect,
    header_rect: Option<Rect>,
    theme: &Theme,
) {
    // 1. Sleek rounded container matching preview.rs (4.0 radius, 1.0 stroke)
    painter.rect_stroke(
        table_rect,
        4.0,
        Stroke::new(1.0_f32, theme.border()),
        StrokeKind::Inside,
    );

    // 2. Header row background pill (no bottom border line)
    if let Some(h_rect) = header_rect {
        let is_single_row = table_rect.height() <= h_rect.height() + 2.0;
        let header_bg =
            Color32::from_rgba_unmultiplied(theme.text.r(), theme.text.g(), theme.text.b(), 14);
        let corner_radius = if is_single_row {
            CornerRadius::same(4)
        } else {
            CornerRadius {
                nw: 4,
                ne: 4,
                sw: 0,
                se: 0,
            }
        };
        painter.rect_filled(h_rect, corner_radius, header_bg);
    }
}

/// Renders alternating row background and row bottom divider line for a table data row.
pub fn render_table_row_decorations(
    painter: &Painter,
    row_rect: Rect,
    theme: &Theme,
    row_idx: usize,
    is_last: bool,
) {
    // Alternating tint on odd rows
    if row_idx % 2 == 1 {
        painter.rect_filled(
            row_rect,
            0.0,
            Color32::from_rgba_unmultiplied(theme.muted.r(), theme.muted.g(), theme.muted.b(), 10),
        );
    }

    // Hairline divider between rows
    if !is_last {
        let divider_color = Color32::from_rgba_unmultiplied(
            theme.border().r(),
            theme.border().g(),
            theme.border().b(),
            80,
        );
        painter.line_segment(
            [row_rect.left_bottom(), row_rect.right_bottom()],
            Stroke::new(0.8_f32, divider_color),
        );
    }
}
