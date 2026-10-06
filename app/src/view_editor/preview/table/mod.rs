//! Preview table rendering module providing 1:1 visual parity with Editor tables.
//!
//! Reuses the exact container stroke, header accent tint, alternating row stripes,
//! hairline divider lines, and cell padding metrics established by the inline editor.

use crate::ui::theme::Theme;
use crate::view_editor::inline::elements::{
    cell_color, render_table_block_decorations, render_table_row_decorations, table_metrics,
};
use crate::view_editor::preview::build_inline_job;
use eframe::egui::{pos2, vec2, Painter, Rect, Ui};

/// Renders a markdown table with exact 1:1 visual parity to the editor's table layout.
/// Matches container borders, header pill background, alternating row tints,
/// and exact font & padding metrics.
///
/// Returns the vertical height consumed by the rendered table.
pub fn render_preview_table(
    ui: &Ui,
    painter: &Painter,
    content_painter: &Painter,
    start_x: f32,
    current_y: f32,
    max_text_w: f32,
    font_size: f32,
    headers: &[String],
    rows: &[Vec<String>],
    theme: &Theme,
    table_idx: usize,
    viewport_rect: Rect,
) -> f32 {
    let col_count = headers.len().max(1);
    let table_w = max_text_w;

    let min_col_w = (font_size * 7.5).max(95.0);
    // Expand columns to evenly fill table_w, or enforce min_col_w when multiple columns overflow
    let cell_pad_x = 10.0;
    let col_w = ((table_w - cell_pad_x * 2.0) / col_count as f32).max(min_col_w);
    let total_content_w = col_w * col_count as f32 + cell_pad_x * 2.0;
    let max_scroll_x = (total_content_w - table_w).max(0.0);
    let needs_h_scroll = max_scroll_x > 0.0;

    // 1. Precompute header galleys and height matching editor table_metrics
    let (_, header_min_h) = table_metrics(font_size, true, false, false);
    let header_color = cell_color(theme, true);
    let header_galleys: Vec<_> = headers
        .iter()
        .map(|h_text| {
            let job = build_inline_job(
                h_text,
                font_size * 0.95,
                header_color,
                theme,
                col_w - cell_pad_x * 2.0,
            );
            painter.layout_job(job)
        })
        .collect();
    let header_h = header_galleys
        .iter()
        .map(|g| g.size().y + 8.0)
        .fold(header_min_h, f32::max);

    // 2. Precompute data row galleys and heights matching editor table_metrics
    let (_, row_min_h) = table_metrics(font_size, false, false, false);
    let row_text_color = cell_color(theme, false);
    let mut row_galleys_list: Vec<Vec<_>> = Vec::with_capacity(rows.len());
    let mut row_heights: Vec<f32> = Vec::with_capacity(rows.len());

    for row in rows {
        let mut row_galleys = Vec::with_capacity(col_count);
        let mut max_cell_h = row_min_h;
        for c_idx in 0..col_count {
            let cell_text = row.get(c_idx).map(|s| s.as_str()).unwrap_or("");
            let job = build_inline_job(
                cell_text,
                font_size * 0.90,
                row_text_color,
                theme,
                col_w - cell_pad_x * 2.0,
            );
            let galley = painter.layout_job(job);
            if galley.size().y + 6.0 > max_cell_h {
                max_cell_h = galley.size().y + 6.0;
            }
            row_galleys.push(galley);
        }
        row_heights.push(max_cell_h);
        row_galleys_list.push(row_galleys);
    }

    let total_table_h = header_h + row_heights.iter().sum::<f32>();

    // Viewport frustum culling
    if current_y + total_table_h >= viewport_rect.min.y && current_y <= viewport_rect.max.y {
        let table_rect =
            Rect::from_min_size(pos2(start_x, current_y), vec2(table_w, total_table_h));
        let header_rect = Rect::from_min_size(pos2(start_x, current_y), vec2(table_w, header_h));

        // Horizontal scrolling state & input handling (invisible scrollbar)
        let scroll_id = ui.id().with(("preview_table_scroll_x", table_idx));
        let mut scroll_x: f32 = ui.data(|d| d.get_temp(scroll_id).unwrap_or(0.0));
        if needs_h_scroll && ui.rect_contains_pointer(table_rect) {
            let h_delta = ui.input(|i| {
                if i.modifiers.shift {
                    if i.smooth_scroll_delta.y.abs() > 0.001 {
                        i.smooth_scroll_delta.y
                    } else {
                        i.raw_scroll_delta.y * 0.5
                    }
                } else if i.smooth_scroll_delta.x.abs() > 0.001 {
                    i.smooth_scroll_delta.x
                } else if i.raw_scroll_delta.x.abs() > 0.001 {
                    i.raw_scroll_delta.x * 0.5
                } else {
                    0.0
                }
            });
            if h_delta != 0.0 {
                scroll_x = (scroll_x - h_delta).clamp(0.0, max_scroll_x);
                ui.data_mut(|d| d.insert_temp(scroll_id, scroll_x));
                ui.ctx().request_repaint();
            }

            // Mouse horizontal drag handling
            if ui.input(|i| i.pointer.is_decidedly_dragging()) {
                let drag_x = ui.input(|i| i.pointer.delta().x);
                if drag_x != 0.0 {
                    scroll_x = (scroll_x - drag_x).clamp(0.0, max_scroll_x);
                    ui.data_mut(|d| d.insert_temp(scroll_id, scroll_x));
                    ui.ctx().request_repaint();
                }
            }
        }
        scroll_x = scroll_x.clamp(0.0, max_scroll_x);

        // Outer container border and header decorations matching editor styling directly (DRY)
        render_table_block_decorations(content_painter, table_rect, Some(header_rect), theme);

        // Strictly clip table cell drawing so zoomed or horizontally scrolled text never bleeds past table boundary
        let table_painter = content_painter.with_clip_rect(table_rect.intersect(viewport_rect));

        // Render header cell galleys
        for (c_idx, galley) in header_galleys.iter().enumerate() {
            let cx = start_x - scroll_x + cell_pad_x + (c_idx as f32 * col_w);
            let cy = current_y + ((header_h - galley.size().y) * 0.5).max(4.0);
            table_painter.galley(pos2(cx, cy), galley.clone(), header_color);
        }

        // Render data rows
        let mut row_y = current_y + header_h;
        for (r_idx, (r_galleys, &rh)) in row_galleys_list.iter().zip(row_heights.iter()).enumerate()
        {
            let row_rect = Rect::from_min_size(pos2(start_x, row_y), vec2(table_w, rh));
            let is_last = r_idx + 1 == rows.len();

            // Alternating tint & hairline divider matching editor styling directly (DRY)
            render_table_row_decorations(&table_painter, row_rect, theme, r_idx, is_last);

            // Render cell galleys
            for (c_idx, galley) in r_galleys.iter().enumerate() {
                let cx = start_x - scroll_x + cell_pad_x + (c_idx as f32 * col_w);
                let cy = row_y + ((rh - galley.size().y) * 0.5).max(3.0);
                table_painter.galley(pos2(cx, cy), galley.clone(), row_text_color);
            }

            row_y += rh;
        }
    }

    total_table_h + 10.0
}
