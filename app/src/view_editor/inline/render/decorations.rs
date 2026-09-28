//! Container card and block decoration rendering for inline code blocks and tables.

use crate::editor::Editor;
use crate::ui::theme::Theme;
use crate::view_editor::inline::elements::{
    code_block_copy_button_rect, render_code_block_card, render_table_block_decorations,
};
use crate::view_editor::inline::types::{InlineEditorLayout, InlineLineKind};
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Pos2, Rect};

/// Renders unified background cards, headers, and interactive copy buttons for multi-line code blocks.
pub fn render_code_block_containers(
    ui: &egui::Ui,
    editor_painter: &egui::Painter,
    layout: &InlineEditorLayout,
    ed: &Editor,
    ed_origin: Pos2,
    editor_rect: Rect,
    text_left: f32,
    content_right: f32,
    theme: &Theme,
) {
    let mut blk_idx = 0;
    while blk_idx < layout.lines.len() {
        if matches!(layout.lines[blk_idx].kind, InlineLineKind::CodeFence(_) | InlineLineKind::CodeLine) {
            let start_idx = blk_idx;
            let mut end_idx = blk_idx;
            let mut fence_lang: Option<String> = None;

            if let InlineLineKind::CodeFence(ref l) = layout.lines[blk_idx].kind {
                if !l.is_empty() {
                    fence_lang = Some(l.clone());
                }
            }

            while end_idx + 1 < layout.lines.len()
                && matches!(layout.lines[end_idx + 1].kind, InlineLineKind::CodeFence(_) | InlineLineKind::CodeLine)
            {
                end_idx += 1;
                if matches!(layout.lines[end_idx].kind, InlineLineKind::CodeFence(_)) {
                    break;
                }
            }

            let top_y = ed_origin.y + layout.lines[start_idx].y_offset;
            let bottom_y = ed_origin.y + layout.lines[end_idx].y_offset + layout.lines[end_idx].height;

            let is_start_active = ed.cur >= layout.lines[start_idx].char_start
                && ed.cur <= layout.lines[start_idx].char_end;
            let display_lang = if is_start_active {
                None
            } else {
                fence_lang.as_deref()
            };

            if bottom_y >= editor_rect.min.y && top_y <= editor_rect.max.y {
                render_code_block_card(
                    editor_painter,
                    top_y,
                    bottom_y,
                    text_left,
                    content_right,
                    theme,
                    display_lang,
                );

                // Code block copy button: zero background, zero border matching preview.rs
                let copy_id = ui.id().with(("inline_code_block_copy", start_idx));
                let current_time = ui.input(|i| i.time);
                let last_copied: Option<f64> = ui.data(|d| d.get_temp(copy_id));
                let is_copied = last_copied.map_or(false, |t| current_time - t < 1.0);

                let btn_rect = code_block_copy_button_rect(content_right, top_y);

                let is_btn_hovered = ui.rect_contains_pointer(btn_rect.expand(3.0));
                if is_btn_hovered {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if is_btn_hovered && ui.input(|i| i.pointer.primary_clicked()) {
                    let mut code_text = String::new();
                    for k in (start_idx + 1)..end_idx {
                        let l_chars = &layout.lines[k];
                        let raw_line: String = ed.buf[l_chars.char_start..l_chars.char_end].iter().collect();
                        code_text.push_str(&raw_line);
                        code_text.push('\n');
                    }
                    ui.ctx().copy_text(code_text);
                    ui.data_mut(|d| d.insert_temp(copy_id, current_time));
                    ui.ctx().request_repaint();
                }
                if is_copied {
                    let elapsed = current_time - last_copied.unwrap();
                    let remaining = 1.0 - elapsed;
                    if remaining > 0.0 {
                        ui.ctx().request_repaint_after(std::time::Duration::from_millis((remaining * 1000.0) as u64 + 20));
                    }
                }

                let (btn_text, btn_color) = if is_copied {
                    ("✓", Color32::from_rgb(60, 200, 110))
                } else if is_btn_hovered {
                    ("Copy", theme.text)
                } else {
                    ("Copy", theme.muted)
                };

                editor_painter.text(
                    btn_rect.center(),
                    Align2::CENTER_CENTER,
                    btn_text,
                    FontId::monospace(if is_copied { 11.5 } else { 9.5 }),
                    btn_color,
                );
            }

            blk_idx = end_idx + 1;
        } else {
            blk_idx += 1;
        }
    }
}

/// Renders unified background container cards and headers for markdown tables.
pub fn render_table_containers(
    editor_painter: &egui::Painter,
    layout: &InlineEditorLayout,
    ed_origin: Pos2,
    editor_rect: Rect,
    text_left: f32,
    table_w: f32,
    theme: &Theme,
) {
    let mut tbl_idx = 0;
    while tbl_idx < layout.lines.len() {
        if matches!(layout.lines[tbl_idx].kind, InlineLineKind::TableRow(_)) {
            let start_idx = tbl_idx;
            let mut end_idx = tbl_idx;
            while end_idx + 1 < layout.lines.len()
                && matches!(layout.lines[end_idx + 1].kind, InlineLineKind::TableRow(_))
            {
                end_idx += 1;
            }

            let top_y = ed_origin.y + layout.lines[start_idx].y_offset;
            let bottom_y = ed_origin.y + layout.lines[end_idx].y_offset + layout.lines[end_idx].height;

            if bottom_y >= editor_rect.min.y && top_y <= editor_rect.max.y {
                let table_rect = Rect::from_min_max(
                    pos2(text_left, top_y),
                    pos2(text_left + table_w, bottom_y),
                );

                let mut header_rect = None;
                if let InlineLineKind::TableRow(ref info) = layout.lines[start_idx].kind {
                    if info.is_header {
                        let h_h = layout.lines[start_idx].height;
                        header_rect = Some(Rect::from_min_size(pos2(text_left, top_y), vec2(table_w, h_h)));
                    }
                }

                render_table_block_decorations(
                    editor_painter,
                    table_rect,
                    header_rect,
                    theme,
                );
            }

            tbl_idx = end_idx + 1;
        } else {
            tbl_idx += 1;
        }
    }
}
