//! Markdown table rendering, cell column grid layout, and styling.

use super::super::types::TableAlign;
use crate::ui::theme::Theme;
use eframe::egui::{Color32, CornerRadius, FontId, Painter, Rect, Stroke, StrokeKind};

/// A parsed cell within a table row, recording its character slice and trimmed content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableCellSpan {
    /// Start index in line's chars (inclusive)
    pub start: usize,
    /// End index in line's chars (exclusive)
    pub end: usize,
    /// Trimmed content start index in line's chars
    pub content_start: usize,
    /// Trimmed content end index in line's chars
    pub content_end: usize,
}

/// Parses the cell character spans of a table row (e.g. `| Col 1 | Col 2 |`).
pub fn split_table_cells(chars: &[char]) -> Vec<TableCellSpan> {
    let mut cells = Vec::new();
    let n = chars.len();
    if n == 0 {
        return cells;
    }

    let mut start = 0;
    if chars[0] == '|' {
        start = 1;
    }

    let mut i = start;
    while i <= n {
        if i == n || chars[i] == '|' {
            let cell_start = start;
            let cell_end = i;

            if cell_end > cell_start || i < n {
                let mut content_start = cell_start;
                while content_start < cell_end && chars[content_start].is_whitespace() {
                    content_start += 1;
                }
                let mut content_end = cell_end;
                while content_end > content_start && chars[content_end - 1].is_whitespace() {
                    content_end -= 1;
                }

                // Avoid trailing empty cell after closing pipe at end of line
                if content_end > content_start || i < n {
                    cells.push(TableCellSpan {
                        start: cell_start,
                        end: cell_end,
                        content_start,
                        content_end,
                    });
                }
            }
            start = i + 1;
        }
        i += 1;
    }

    cells
}

/// Returns font and line height metrics for table rows.
pub fn table_metrics(base_font_size: f32, is_header: bool, is_separator: bool, is_active: bool) -> (FontId, f32) {
    if is_separator {
        if is_active {
            (crate::services::font_manager::editor_font_id(base_font_size * 0.9), (base_font_size * 1.4).round())
        } else {
            (crate::services::font_manager::editor_font_id(base_font_size * 0.7), 1.0)
        }
    } else if is_active {
        (crate::services::font_manager::editor_font_id(base_font_size), (base_font_size * 1.55).round())
    } else if is_header {
        (crate::services::font_manager::editor_font_id(base_font_size * 0.95), (base_font_size * 1.6).round() + 8.0)
    } else {
        (crate::services::font_manager::editor_font_id(base_font_size * 0.90), (base_font_size * 1.5).round() + 6.0)
    }
}

/// Returns the pipe delimiter `|` color.
pub fn pipe_color(theme: &Theme, is_active: bool) -> Color32 {
    if is_active {
        theme.text
    } else {
        Color32::from_rgba_unmultiplied(
            theme.border().r(),
            theme.border().g(),
            theme.border().b(),
            if theme.is_light() { 160 } else { 120 },
        )
    }
}

/// Returns the cell text color.
pub fn cell_color(theme: &Theme, _is_header: bool) -> Color32 {
    theme.text
}

/// Parses column alignments from a markdown separator line (e.g. `| :--- | :---: | ---: |`).
pub fn parse_aligns(sep_chars: &[char]) -> Vec<TableAlign> {
    let s: String = sep_chars.iter().collect();
    let trimmed = s.trim();
    let parts = trimmed.split('|');
    let mut aligns = Vec::new();

    for p in parts {
        let p_trim = p.trim();
        if p_trim.is_empty() {
            continue;
        }
        let starts = p_trim.starts_with(':');
        let ends = p_trim.ends_with(':');
        let align = match (starts, ends) {
            (true, true) => TableAlign::Center,
            (false, true) => TableAlign::Right,
            (true, false) => TableAlign::Left,
            (false, false) => TableAlign::None,
        };
        aligns.push(align);
    }

    aligns
}

/// Renders table outer card stroke, header highlight pill, and alternating row backgrounds
/// matching the exact styling and aesthetics of preview.rs.
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
        let header_bg = Color32::from_rgba_unmultiplied(
            theme.text.r(),
            theme.text.g(),
            theme.text.b(),
            14,
        );
        let corner_radius = if is_single_row {
            CornerRadius::same(4)
        } else {
            CornerRadius { nw: 4, ne: 4, sw: 0, se: 0 }
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
