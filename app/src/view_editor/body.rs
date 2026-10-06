//! Editor text body layout, scrolling, selection, search highlights, and rendering.

use super::ligatures::render_line_with_ligatures;
use crate::caret::Caret;
use crate::editor::{Editor, VisualLine};
use crate::ui::theme::Theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Pos2, Rect, Stroke};

/// Renders the editor body with soft-wrapped visual lines, smooth scrolling, caret animation, and interactive scrollbar.
pub fn render_editor_body(
    ui: &egui::Ui,
    painter: &egui::Painter,
    _window_bounds: Rect,
    editor_rect: Rect,
    ed: &mut Editor,
    visual_lines: &[VisualLine],
    caret: &mut Caret,
    scroll_y: &mut f32,
    theme: &Theme,
    font_size: f32,
    cw: f32,
    lh: f32,
    dt: f32,
    now: f64,
    mut typed: bool,
    block_scroll: bool,
    search_matches: Option<(&[usize], usize)>,
    show_line_numbers: bool,
    language: Option<crate::language::FileLanguage>,
    vim_mode: Option<&str>,
) {
    let font = crate::services::font_manager::editor_font_id(font_size);
    let font_h = painter.layout_no_wrap("M".to_owned(), font.clone(), Color32::WHITE).size().y;
    let y_pad = ((lh - font_h) * 0.5).round().max(0.0);
    let stroke_w = (font_size * 0.088).clamp(1.2, 1.8);
    let visible_h = editor_rect.height().max(0.0);
    if visible_h < 10.0 {
        return;
    }

    let total_content_h = visual_lines.len() as f32 * lh;
    let max_scroll = (total_content_h + 16.0 - visible_h + lh * 4.0).max(0.0);

    // Mouse wheel scrolling: prioritize smooth scroll delta, fallback to scaled raw delta
    if !block_scroll && ui.rect_contains_pointer(editor_rect) {
        let is_ctrl = ui.input(|i| i.modifiers.ctrl || i.modifiers.command);
        if !is_ctrl {
            let scroll_delta = ui.input(|i| {
                if i.raw_scroll_delta.y.abs() > 0.0 {
                    i.raw_scroll_delta.y
                } else {
                    i.smooth_scroll_delta.y
                }
            });
            if scroll_delta.abs() > 0.0 {
                *scroll_y = (*scroll_y - scroll_delta).clamp(0.0, max_scroll);
            }
        }
    }

    let gutter_w = if show_line_numbers {
        let total_lines = visual_lines.len().max(1);
        let digits = total_lines.to_string().len().max(2);
        (digits as f32 * (font_size * 0.55) + 14.0).max(28.0)
    } else {
        0.0
    };

    let pad_x = if show_line_numbers { 16.0 } else { 24.0 };
    let pad_y = 10.0;
    let safe_w = editor_rect.width();
    let effective_gutter_w = if safe_w > gutter_w + 40.0 { gutter_w } else { 0.0 };
    let text_left = (editor_rect.min.x + effective_gutter_w + pad_x).min(editor_rect.max.x);
    let ed_origin = pos2(text_left, editor_rect.min.y - *scroll_y + pad_y);

    // Direct mouse click in editor moves cursor to clicked visual row and col.
    // Restricted strictly to text area (excluding gutter) and blocked when splitter is dragging.
    let text_area_rect = Rect::from_min_max(
        pos2(text_left, editor_rect.min.y),
        pos2(editor_rect.max.x.max(text_left + 1.0), editor_rect.max.y),
    );
    let pos_to_char = |pos: Pos2| -> usize {
        let clicked_row = ((pos.y - ed_origin.y) / lh).floor() as isize;
        if clicked_row >= 0 && (clicked_row as usize) < visual_lines.len() {
            let r = clicked_row as usize;
            let line = &visual_lines[r];
            let clicked_col = (((pos.x - ed_origin.x).max(0.0)) / cw).round() as usize;
            let line_len = line.char_end.saturating_sub(line.char_start);
            line.char_start + clicked_col.min(line_len)
        } else if clicked_row >= visual_lines.len() as isize {
            ed.buf.len()
        } else {
            0
        }
    };

    if !block_scroll && ui.rect_contains_pointer(text_area_rect) {
        if ui.input(|i| i.pointer.button_double_clicked(egui::PointerButton::Primary)) {
            if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                let clicked_char = pos_to_char(pos);
                ed.select_word_at(clicked_char);
                ed.desired_col = None;
                typed = true;
            }
        } else if ui.input(|i| i.pointer.primary_pressed()) {
            if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                let clicked_char = pos_to_char(pos);
                let is_shift = ui.input(|i| i.modifiers.shift);
                if is_shift {
                    if ed.selection.is_none() {
                        ed.selection = Some(ed.cur);
                    }
                    ed.cur = clicked_char;
                } else {
                    ed.cur = clicked_char;
                    ed.selection = None;
                }
                ed.desired_col = None;
                typed = true;
            }
        } else if ui.input(|i| i.pointer.is_decidedly_dragging() && i.pointer.primary_down()) {
            if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                let drag_char = pos_to_char(pos);
                if ed.selection.is_none() {
                    ed.selection = Some(ed.cur);
                }
                ed.cur = drag_char;
                ed.desired_col = None;
                typed = true;
            }
        } else if ui.input(|i| i.pointer.primary_clicked()) {
            if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                let clicked_char = pos_to_char(pos);
                let is_shift = ui.input(|i| i.modifiers.shift);
                if is_shift {
                    if ed.selection.is_none() {
                        ed.selection = Some(ed.cur);
                    }
                    ed.cur = clicked_char;
                } else {
                    ed.cur = clicked_char;
                    ed.selection = None;
                }
                ed.desired_col = None;
                typed = true;
            }
        }
    }

    // Caret position in visual lines
    let (row, _) = ed.visual_row_col(visual_lines);
    let current_line = &visual_lines[row];
    let col = crate::editor::caret_cell(ed.cur, current_line, vim_mode);
    let caret_y_in_content = row as f32 * lh;

    // Auto-scroll / caret follow: ONLY when user is typing or navigating by keyboard!
    if typed {
        // Pin smoothly at bottom edge when moving downwards (accounting for bottom padding)
        if caret_y_in_content + lh + pad_y > *scroll_y + visible_h {
            *scroll_y = (caret_y_in_content + lh + pad_y - visible_h).clamp(0.0, max_scroll);
        }
        // Pin smoothly at top edge when moving upwards
        if caret_y_in_content < *scroll_y {
            *scroll_y = caret_y_in_content.clamp(0.0, max_scroll);
        }
    }

    // Clip drawing strictly to editor bounds
    let editor_painter = painter.with_clip_rect(editor_rect);

    let sel_range = ed.selected_range();
    let sel_color = if theme.is_light() {
        Color32::from_rgba_unmultiplied(
            theme.accent.r(),
            theme.accent.g(),
            theme.accent.b(),
            55,
        )
    } else {
        Color32::from_rgba_unmultiplied(
            theme.accent.r(),
            theme.accent.g(),
            theme.accent.b(),
            40,
        )
    };

    // Caret placement and animation (allow living caret embers/flames to extend into the 5px top gap without clipping)
    let caret_clip = Rect::from_min_max(
        pos2(editor_rect.min.x, editor_rect.min.y - 5.0),
        editor_rect.max,
    );
    let caret_painter = painter.with_clip_rect(caret_clip);
    let caret_x = ed_origin.x + col as f32 * cw;
    let target = pos2(caret_x, ed_origin.y + row as f32 * lh);
    caret.update(dt, target, typed, now, cw, lh);
    caret.paint(&caret_painter, cw, lh, now, theme.accent, theme.is_light());

    let is_block = caret.kind == crate::caret::CaretKind::Block;

    // Frustum culling: render only lines intersecting the visible viewport
    let start_row = if lh > 0.0 {
        ((editor_rect.min.y - ed_origin.y) / lh).floor().max(0.0) as usize
    } else {
        0
    };
    let end_row = if lh > 0.0 {
        (((editor_rect.max.y - ed_origin.y) / lh).ceil() as usize + 1).min(visual_lines.len())
    } else {
        visual_lines.len()
    };
    let start_row = start_row.min(visual_lines.len());
    let end_row = end_row.max(start_row).min(visual_lines.len());

    for r in start_row..end_row {
        let line = &visual_lines[r];
        let line_y = ed_origin.y + r as f32 * lh;

        // Render selection highlight background behind text on this line
        if let Some((sel_start, sel_end)) = sel_range {
            if line.char_start == line.char_end {
                // Empty line gap between paragraphs (\n\n):
                // Highlight a visible block only when the active selection spans across this empty line.
                if sel_start <= line.char_start && sel_end > line.char_end {
                    let sel_w = cw.max(12.0);
                    let highlight_rect = Rect::from_min_size(pos2(ed_origin.x, line_y), vec2(sel_w, lh + 0.5));
                    editor_painter.rect_filled(highlight_rect, 0.0, sel_color);
                }
            } else {
                let intersect_start = sel_start.max(line.char_start);
                let intersect_end = sel_end.min(line.char_end);
                if intersect_start < intersect_end {
                    let start_col = intersect_start - line.char_start;
                    let end_col = intersect_end - line.char_start;
                    let sel_x = ed_origin.x + start_col as f32 * cw;
                    let mut sel_w = (end_col - start_col) as f32 * cw;
                    // If selection extends past this line's content (e.g. across newline to subsequent lines),
                    // extend the highlight to visually show that the newline / trailing whitespace is selected.
                    if sel_end > line.char_end {
                        sel_w += cw.max(10.0);
                    }
                    let highlight_rect = Rect::from_min_size(pos2(sel_x, line_y), vec2(sel_w, lh + 0.5));
                    editor_painter.rect_filled(highlight_rect, 0.0, sel_color);
                }
            }
        }

        // Render in-buffer search match highlights behind text on this line
        if let Some((matches, q_len)) = search_matches {
            if q_len > 0 {
                for &m_idx in matches {
                    if m_idx + q_len <= line.char_start || m_idx >= line.char_end {
                        continue;
                    }
                    let match_start = m_idx.max(line.char_start);
                    let match_end = (m_idx + q_len).min(line.char_end);
                    if match_start < match_end {
                        let start_col = match_start - line.char_start;
                        let end_col = match_end - line.char_start;
                        let match_x = ed_origin.x + start_col as f32 * cw;
                        let match_w = (end_col - start_col) as f32 * cw;
                        let is_current = m_idx == ed.cur;
                        let match_color = if is_current {
                            theme.accent
                        } else {
                            theme.highlight
                        };
                        let underline_y = line_y + lh - 2.0;
                        editor_painter.line_segment(
                            [pos2(match_x, underline_y), pos2(match_x + match_w, underline_y)],
                            Stroke::new(if is_current { 1.5_f32 } else { 1.0_f32 }, match_color),
                        );
                    }
                }
            }
        }

        let text_y = line_y + y_pad;
        let y_mid = line_y + (lh * 0.5).round();

        let line_len = line.char_end.saturating_sub(line.char_start);
        let block_col = if r == row && is_block && line_len > 0 {
            Some(col.min(line_len))
        } else {
            None
        };
        let start = line.char_start.min(ed.buf.len());
        let end = line.char_end.min(ed.buf.len()).max(start);
        let line_chars = &ed.buf[start..end];
        if let Some(language) = language {
            let source_line: String = line_chars.iter().collect();
            let job = super::preview::syntax::highlight_editor_code_line(
                &source_line,
                language.label(),
                font_size,
                theme,
            );
            let galley = editor_painter.layout_job(job);
            editor_painter.galley(pos2(ed_origin.x, text_y), galley, theme.text);
        } else {
            render_line_with_ligatures(
                &editor_painter,
                ed_origin.x,
                text_y,
                line_y,
                y_mid,
                line_chars,
                &font,
                theme.text,
                cw,
                lh,
                stroke_w,
                block_col,
            );
        }
    }

    // Render line number gutter on the left side if enabled
    if show_line_numbers && gutter_w > 0.0 {
        let gutter_rect = Rect::from_min_max(
            editor_rect.min,
            pos2(editor_rect.min.x + gutter_w, editor_rect.max.y),
        );
        let gutter_painter = painter.with_clip_rect(gutter_rect);
        let num_font = FontId::monospace(font_size * 0.82);

        // Match Neovim's `number relativenumber`: show the absolute number on
        // the cursor line and distance from it on every other physical line.
        let current_line_start = visual_lines
            .get(row)
            .map(|line| line.char_start)
            .unwrap_or(0)
            .min(ed.buf.len());
        let current_physical_line = ed.row_col_of(current_line_start).0 + 1;

        // Compute starting physical line for start_row
        let start_char = visual_lines.get(start_row).map_or(0, |l| l.char_start);
        let mut physical_line = ed.row_col_of(start_char).0 + 1;

        for r in start_row..end_row {
            let v_line = &visual_lines[r];
            let line_y = ed_origin.y + r as f32 * lh;
            let is_new_physical = r == 0 || (v_line.char_start > 0 && ed.buf.get(v_line.char_start.saturating_sub(1)) == Some(&'\n'));

            if r > start_row && is_new_physical {
                physical_line += 1;
            }

            if line_y + lh >= editor_rect.min.y && line_y <= editor_rect.max.y {
                let is_current = physical_line == current_physical_line;
                if is_new_physical {
                    let num_str = if is_current {
                        physical_line.to_string()
                    } else {
                        physical_line.abs_diff(current_physical_line).to_string()
                    };
                    let color = if is_current {
                        theme.accent
                    } else if theme.is_light() {
                        theme.muted
                    } else {
                        Color32::from_rgba_unmultiplied(
                            theme.muted.r(),
                            theme.muted.g(),
                            theme.muted.b(),
                            100,
                        )
                    };
                    gutter_painter.text(
                        pos2(gutter_rect.max.x - 4.0, line_y + y_pad),
                        Align2::RIGHT_TOP,
                        num_str,
                        num_font.clone(),
                        color,
                    );
                } else {
                    // Wrapped continuation line indicator
                    let wrap_color = if is_current {
                        theme.accent
                    } else {
                        Color32::from_rgba_unmultiplied(
                            theme.muted.r(),
                            theme.muted.g(),
                            theme.muted.b(),
                            if theme.is_light() { 130 } else { 40 },
                        )
                    };
                    gutter_painter.text(
                        pos2(gutter_rect.max.x - 4.0, line_y + y_pad),
                        Align2::RIGHT_TOP,
                        "·",
                        num_font.clone(),
                        wrap_color,
                    );
                }
            }
        }
    }

    // Scrollbar thumb is kept invisible per UI design (mouse wheel and keyboard scrolling remain active).
}
