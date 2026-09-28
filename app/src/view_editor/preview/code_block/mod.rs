//! Preview code block rendering module providing 1:1 visual parity with Editor code blocks.
//!
//! Reuses the exact container card surface, sleek header bar, language badge,
//! copy button metrics, and line height rhythm established by the inline editor.

use crate::ui::theme::Theme;
use crate::view_editor::inline::elements::{code_block_copy_button_rect, render_code_block_card};
use crate::view_editor::preview::syntax::highlight_code_line;
use eframe::egui::{pos2, vec2, Align2, Color32, FontId, Painter, Rect, Ui};

/// Renders a fenced code block with exact 1:1 parity to the editor's code block card.
/// Matches card surface, header bar, language badge, copy button, syntax highlighting,
/// and line height rhythm.
///
/// Returns the vertical height consumed by the rendered code block.
pub fn render_preview_code_block(
    ui: &Ui,
    painter: &Painter,
    content_painter: &Painter,
    start_x: f32,
    current_y: f32,
    max_text_w: f32,
    font_size: f32,
    lang: &str,
    code: &str,
    theme: &Theme,
    code_block_idx: usize,
    viewport_rect: Rect,
) -> f32 {
    let lines: Vec<&str> = code.lines().collect();
    let line_count = lines.len().max(1);
    // Exact match to editor's line height for code
    let line_h = (font_size * 1.55).round();
    let has_header = !lang.is_empty();
    let pad_x = 14.0;
    let pad_top = if has_header { 28.0 } else { 12.0 };

    let avail_w = (max_text_w - pad_x * 2.0).max(10.0);
    let line_galleys: Vec<std::sync::Arc<eframe::egui::Galley>> = lines
        .iter()
        .map(|line_str| painter.layout_job(highlight_code_line(line_str, lang, font_size, theme)))
        .collect();
    let max_line_w = line_galleys.iter().map(|g| g.size().x).fold(0.0f32, f32::max);
    let max_scroll_x = (max_line_w - avail_w).max(0.0);
    let needs_h_scroll = max_scroll_x > 0.0;
    let pad_bottom = if needs_h_scroll { 16.0 } else { 12.0 };
    let block_h = pad_top + (line_count as f32 * line_h) + pad_bottom;

    if current_y + block_h >= viewport_rect.min.y && current_y <= viewport_rect.max.y {
        let code_rect = Rect::from_min_size(pos2(start_x, current_y), vec2(max_text_w, block_h));
        let card_left = start_x;
        let card_right = start_x + max_text_w;

        // 1. Unified cohesive code wrapper card matching editor card with sleek header bar
        render_code_block_card(
            content_painter,
            current_y,
            current_y + block_h,
            card_left,
            card_right,
            theme,
            if lang.is_empty() { None } else { Some(lang) },
        );

        // 2. Horizontal scrolling state & input handling
        let scroll_id = ui.id().with(("code_block_scroll_x", code_block_idx));
        let mut scroll_x: f32 = ui.data(|d| d.get_temp(scroll_id).unwrap_or(0.0));
        if ui.rect_contains_pointer(code_rect) && needs_h_scroll {
            let h_delta = ui.input(|i| {
                if i.modifiers.shift {
                    if i.smooth_scroll_delta.y.abs() > 0.001 {
                        i.smooth_scroll_delta.y
                    } else {
                        i.raw_scroll_delta.y * 0.5
                    }
                } else if i.smooth_scroll_delta.x.abs() > 0.001 {
                    i.smooth_scroll_delta.x
                } else {
                    i.raw_scroll_delta.x * 0.5
                }
            });
            if h_delta != 0.0 {
                scroll_x = (scroll_x - h_delta).clamp(0.0, max_scroll_x);
                ui.data_mut(|d| d.insert_temp(scroll_id, scroll_x));
                ui.ctx().request_repaint();
            }
        }
        scroll_x = scroll_x.clamp(0.0, max_scroll_x);

        // 3. Copy button logic & state matching editor
        let copy_id = ui.id().with(("code_block_copy", code_block_idx));
        let current_time = ui.input(|i| i.time);
        let last_copied: Option<f64> = ui.data(|d| d.get_temp(copy_id));
        let is_copied = last_copied.map_or(false, |t| current_time - t < 1.0);

        let btn_rect = code_block_copy_button_rect(card_right, current_y);
        let is_btn_hovered = ui.rect_contains_pointer(btn_rect);
        if is_btn_hovered {
            ui.ctx().set_cursor_icon(eframe::egui::CursorIcon::PointingHand);
        }
        if is_btn_hovered && ui.input(|i| i.pointer.primary_clicked()) {
            ui.ctx().copy_text(code.to_string());
            ui.data_mut(|d| d.insert_temp(copy_id, current_time));
            ui.ctx().request_repaint();
        }
        if is_copied {
            let elapsed = current_time - last_copied.unwrap();
            let remaining = 1.0 - elapsed;
            if remaining > 0.0 {
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(
                    (remaining * 1000.0) as u64 + 20,
                ));
            }
        }

        // Render Copy button text matching editor
        let (btn_text, btn_color) = if is_copied {
            ("✓", Color32::from_rgb(60, 200, 110))
        } else if is_btn_hovered {
            ("Copy", theme.text)
        } else {
            ("Copy", theme.muted)
        };
        content_painter.text(
            btn_rect.center(),
            Align2::CENTER_CENTER,
            btn_text,
            FontId::monospace(if is_copied { 11.5 } else { 9.5 }),
            btn_color,
        );

        // 4. Render code lines with horizontal scroll offset
        let code_clip = Rect::from_min_max(
            pos2(code_rect.min.x + pad_x, code_rect.min.y),
            pos2(code_rect.max.x - pad_x, code_rect.max.y),
        );
        let code_painter = content_painter.with_clip_rect(code_clip);

        let mut line_y = code_rect.min.y + pad_top;
        for galley in line_galleys {
            code_painter.galley(
                pos2(code_rect.min.x + pad_x - scroll_x, line_y),
                galley,
                theme.text,
            );
            line_y += line_h;
        }

        // 5. Interactive horizontal scrollbar at bottom when text overflows
        if needs_h_scroll {
            let track_h = 3.5;
            let track_y = code_rect.max.y - 7.0;
            let track_rect = Rect::from_min_size(
                pos2(code_rect.min.x + pad_x, track_y),
                vec2(avail_w, track_h),
            );

            let thumb_ratio = (avail_w / max_line_w).clamp(0.1, 1.0);
            let thumb_w = (avail_w * thumb_ratio).max(20.0);
            let scroll_ratio = if max_scroll_x > 0.0 {
                scroll_x / max_scroll_x
            } else {
                0.0
            };
            let thumb_x = track_rect.min.x + scroll_ratio * (avail_w - thumb_w);
            let thumb_rect = Rect::from_min_size(pos2(thumb_x, track_y), vec2(thumb_w, track_h));

            content_painter.rect_filled(
                track_rect,
                1.75,
                Color32::from_rgba_unmultiplied(theme.muted.r(), theme.muted.g(), theme.muted.b(), 30),
            );
            content_painter.rect_filled(
                thumb_rect,
                1.75,
                Color32::from_rgba_unmultiplied(theme.muted.r(), theme.muted.g(), theme.muted.b(), 110),
            );
        }
    }

    block_h + 12.0
}
