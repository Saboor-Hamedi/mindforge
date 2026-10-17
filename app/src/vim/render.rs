//! Neovim Grid Surface Renderer.
//!
//! # Purpose
//! Converts Neovim UI/grid state into MindForge's egui editor surface.
//!
//! # Architecture & Responsibilities
//! - Computes exact row galleys with syntax highlighting and text layouts.
//! - Renders contiguous selection highlights (Visual mode) without vertical gaps.
//! - Renders gutter line numbers aligned with MindForge's typography.
//! - Renders caret positioning, animations, gliding, and cursor styles.
//! - Displays auto-completion popup menus and LSP documentation hover panes.
//!
//! # Invariants & Non-Goals
//! - Must NOT mutate authoritative buffer lines.
//! - Must NOT perform blocking RPC or synchronous IPC.
//! - Must NOT implement Vim command semantics.

use super::backend::VimBackend;
use crate::editor::backend::EditorBackend;
use crate::editor::events::EditorMouseEvent;
use crate::ui::theme::Theme;
use eframe::egui::{self, Align2, Color32, Id, Pos2, Rect, Sense, Stroke, Ui};
use rmpv::Value;
use std::sync::Arc;
use std::time::Duration;

pub(crate) fn rgb(value: u32) -> Color32 {
    Color32::from_rgb((value >> 16) as u8, (value >> 8) as u8, value as u8)
}

/// Plain-text form of LSP documentation: drops Markdown code fences and
/// inline backticks, caps the length, and trims blank edges.
pub(crate) fn clean_documentation(body: &str) -> String {
    let text: Vec<&str> = body
        .lines()
        .filter(|line| !line.trim_start().starts_with("```"))
        .collect();
    let joined = text.join("\n").replace('`', "").replace('\t', "  ");
    let trimmed = joined.trim();
    if trimmed.chars().count() > 1200 {
        trimmed
            .chars()
            .take(1199)
            .chain(std::iter::once('…'))
            .collect()
    } else {
        trimmed.to_string()
    }
}

pub(crate) fn popupmenu_items(value: &Value) -> Vec<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_array)
        .filter_map(|item| item.first().and_then(Value::as_str))
        .map(str::to_owned)
        .collect()
}

pub(crate) fn popupmenu_info(value: &Value) -> Vec<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_array)
        .map(|item| item.get(3).and_then(Value::as_str).unwrap_or("").to_owned())
        .collect()
}

/// Maps the LSP completion kind label to a short glyph column.
pub(crate) fn popupmenu_meta(value: &Value) -> Vec<(String, String)> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_array)
        .map(|item| {
            let text = |i: usize| item.get(i).and_then(Value::as_str).unwrap_or("");
            let kind = match text(1) {
                "Text" => "t",
                "Method" | "Function" | "Constructor" => "ƒ",
                "Field" | "Property" => "p",
                "Variable" | "Value" | "Unit" => "x",
                "Class" | "TypeParameter" => "C",
                "Interface" => "I",
                "Module" | "Folder" => "M",
                "Struct" => "S",
                "Enum" | "EnumMember" => "E",
                "Constant" => "c",
                "Keyword" => "k",
                "Snippet" => "▣",
                "Event" => "e",
                "Operator" => "±",
                "Reference" | "Color" => "&",
                "File" => "f",
                "" => "",
                _ => "·",
            };
            (kind.to_owned(), text(2).to_owned())
        })
        .collect()
}

impl VimBackend {
    /// Selects completion item index and accepts it (mouse click on the popup).
    pub(crate) fn accept_popup_item(&mut self, index: usize) {
        let total = self.grid.popup_items.len() as i64;
        if index as i64 >= total {
            return;
        }
        let current = self
            .grid
            .popup_selected
            .map_or(-1, |selected| selected as i64);
        let _ = self.client.notify(
            "nvim_exec_lua",
            vec![
                Value::from(concat!(
                    "local index, current = ...; ",
                    "local select_item = vim.api.nvim_select_popupmenu_item; ",
                    "if select_item then local ok = pcall(select_item, index, true, true, {}); if ok then return end end; ",
                    "local steps = current >= 0 and (index - current) or (index + 1); ",
                    "local key = steps >= 0 and '<C-n>' or '<C-p>'; ",
                    "local keys = string.rep(key, math.abs(steps)) .. '<C-y>'; ",
                    "vim.api.nvim_feedkeys(vim.api.nvim_replace_termcodes(keys, true, false, true), 'n', false)"
                )),
                Value::Array(vec![Value::from(index as i64), Value::from(current)]),
            ],
        );
    }

    /// Paints Neovim's grid strictly inside the application's existing editor content rectangle.
    pub(crate) fn render_in_rect_impl(
        &mut self,
        ui: &mut Ui,
        rect: Rect,
        font_size: f32,
        cell_width: f32,
        row_height: f32,
        theme: &Theme,
        caret: &mut crate::caret::Caret,
        dt: f32,
        typed: bool,
        show_line_numbers: bool,
    ) {
        self.tick();

        // Expire stale messages/mode text so they never linger on screen (and
        // so a leftover message can no longer interfere with layout).
        if let Some(at) = self.grid.message_at {
            if at.elapsed() >= Duration::from_secs(5) {
                self.grid.message.clear();
                self.grid.message_at = None;
                self.grid.message_is_error = false;
                self.grid.message_copied_at = None;
            } else {
                ui.ctx().request_repaint_after(Duration::from_millis(200));
            }
        }

        let font = crate::services::font_manager::editor_font_id(font_size);
        if self.layout_font_size != font_size.to_bits() {
            self.row_layouts.clear();
            self.layout_font_size = font_size.to_bits();
        }

        let nvim_row_height = row_height.max(1.0);
        let rows = self.grid.cells.len();
        let grid_is_empty = rows == 0
            || self.grid.cells.iter().all(|row| {
                row.iter().all(|cell| cell.text.trim().is_empty())
            });
        let detected_gutter_cols = self
            .grid
            .cells
            .iter()
            .map(|r| {
                r.iter()
                    .take_while(|c| {
                        self.grid
                            .highlights
                            .get(&c.highlight)
                            .is_some_and(|s| s.is_gutter)
                    })
                    .count()
            })
            .max()
            .unwrap_or(0);

        let show_line_numbers = if !grid_is_empty && !self.grid.highlights.is_empty() {
            detected_gutter_cols > 0
        } else {
            self.line_numbers_enabled.unwrap_or(show_line_numbers)
        };
        const NUMBER_WIDTH: usize = 4;
        let gutter_cols = if show_line_numbers {
            detected_gutter_cols.max(NUMBER_WIDTH)
        } else {
            0
        };
        let gutter_w = if show_line_numbers {
            (gutter_cols as f32 * (font_size * 0.55) + 14.0).max(28.0)
        } else {
            0.0
        };
        let effective_gutter_w = if rect.width() > gutter_w + 40.0 {
            gutter_w
        } else {
            0.0
        };
        let pad_x = if show_line_numbers { 16.0 } else { 24.0 };
        let text_left = rect.min.x + effective_gutter_w + pad_x;

        let number_columns = if show_line_numbers && effective_gutter_w > 0.0 {
            if !grid_is_empty && !self.grid.highlights.is_empty() {
                detected_gutter_cols
            } else {
                NUMBER_WIDTH
            }
        } else {
            0
        };
        if self.last_number_columns != number_columns {
            self.last_number_columns = number_columns;
            self.row_layouts.clear();
        }

        let origin = Pos2::new(
            text_left - number_columns as f32 * cell_width,
            rect.min.y + 10.0,
        );
        let content_rect = Rect::from_min_max(origin, rect.max);
        let width = ((rect.max.x - text_left) / cell_width.max(1.0))
            .floor()
            .max(1.0) as usize
            + number_columns;
        let height = (content_rect.height() / nvim_row_height).floor().max(1.0) as usize;
        let now = ui.input(|i| i.time);

        if (width, height) != self.last_size {
            if self.pending_size != Some((width, height)) {
                self.pending_size = Some((width, height));
                self.resize_pending_since = Some(now);
            }
        } else {
            self.pending_size = None;
            self.resize_pending_since = None;
        }

        if let (Some(size), Some(since)) = (self.pending_size, self.resize_pending_since) {
            if now - since >= 0.05 {
                if let Err(error) = self.client.resize(size.0, size.1) {
                    self.error = Some(error);
                } else {
                    self.last_size = size;
                    self.pending_size = None;
                    self.resize_pending_since = None;
                }
            }
        }

        let response = ui.interact(
            rect,
            Id::new("neovim_editor_surface"),
            Sense::click_and_drag(),
        );
        if response.clicked() {
            response.request_focus();
        }

        let painter = ui.painter().with_clip_rect(rect);

        let current_theme_colors = (theme.accent, theme.text, theme.bg, theme.highlight);
        if self.cached_theme != Some(current_theme_colors) {
            self.cached_theme = Some(current_theme_colors);
            self.sync_theme(theme);
        }

        if self.grid.width == 0 || self.grid.height == 0 {
            painter.text(
                Pos2::new(origin.x + number_columns as f32 * cell_width, origin.y),
                Align2::LEFT_TOP,
                self.error.as_deref().unwrap_or("Starting Neovim…"),
                font.clone(),
                theme.text,
            );
        } else if grid_is_empty {
            if !self.lines.is_empty() && (self.lines.len() > 1 || !self.lines[0].is_empty()) {
                let pending = self.lines.join("\n");
                render_pending_text(
                    &painter,
                    rect,
                    &pending,
                    font_size,
                    nvim_row_height,
                    theme,
                    show_line_numbers,
                );
            }
        } else {
            // Lay out every row Neovim actually sent (clipped to the rect by the
            // painter) rather than clamping to the locally-computed height. Clamping
            // could hide the last row and thus the caret when the grid was briefly a
            // row taller than our estimate.
            self.row_layouts.resize_with(rows, || None);

            for row_idx in 0..rows {
            let row = &self.grid.cells[row_idx];
            // Never render a Neovim statusline row (e.g. bottom filename/ruler bar).
            let row_is_statusline = !row.is_empty()
                && row.iter().any(|c| {
                    self.grid
                        .highlights
                        .get(&c.highlight)
                        .is_some_and(|s| s.is_statusline)
                });
            if row_is_statusline {
                continue;
            }
            let revision = self.grid.row_revision.get(row_idx).copied().unwrap_or(0);
            // Every grid row is real buffer text (messages/cmdline are handled
            // out-of-band via `ext_messages`/`ext_cmdline`), so each row is laid
            // out and positioned identically.
            let row_font = font.clone();
            if self.row_layouts[row_idx]
                .as_ref()
                .is_none_or(|(cached_revision, _, _)| *cached_revision != revision)
            {
                let visible_count = width.saturating_sub(number_columns);
                let visible_slice = if number_columns < row.len() {
                    let end = (number_columns + visible_count).min(row.len());
                    &row[number_columns..end]
                } else {
                    &[]
                };

                let row_chars: Vec<char> = visible_slice
                    .iter()
                    .map(|c| c.text.chars().next().unwrap_or(' '))
                    .collect();

                let mut row_ligatures = Vec::new();
                let mut i = 0;
                while i < row_chars.len() {
                    if let Some(lig) = crate::view_editor::ligatures::detect_ligature(&row_chars, i) {
                        let len = lig.char_len();
                        let cell = &visible_slice[i];
                        let mut foreground = theme.text;
                        if let Some(style) = self.grid.highlights.get(&cell.highlight) {
                            if style.reverse {
                                foreground = style.background.map(rgb).unwrap_or(theme.bg);
                            } else if let Some(color) = style.foreground {
                                foreground = rgb(color);
                            }
                        }
                        row_ligatures.push(super::backend::RowLigature {
                            col_offset: i,
                            kind: lig,
                            color: foreground,
                        });
                        i += len;
                    } else {
                        i += 1;
                    }
                }

                let mut job = egui::text::LayoutJob::default();
                for (col_offset, cell) in visible_slice.iter().enumerate() {
                    let col_idx = number_columns + col_offset;
                    let is_cursor_cell =
                        row_idx == self.grid.cursor.row && col_idx == self.grid.cursor.column;
                    let mut foreground = theme.text;
                    let mut format = egui::TextFormat {
                        font_id: row_font.clone(),
                        color: foreground,
                        background: Color32::TRANSPARENT,
                        ..Default::default()
                    };
                    let mut is_stl = false;
                    if let Some(style) = self.grid.highlights.get(&cell.highlight) {
                        if style.is_statusline {
                            is_stl = true;
                        } else if style.reverse {
                            if !is_cursor_cell || caret.kind == crate::caret::CaretKind::Block {
                                foreground = style.background.map(rgb).unwrap_or(theme.bg);
                            }
                        } else if let Some(color) = style.foreground {
                            foreground = rgb(color);
                        }
                    }
                    format.color = foreground;

                    let in_ligature = row_ligatures.iter().any(|lig| {
                        col_offset >= lig.col_offset && col_offset < lig.col_offset + lig.kind.char_len()
                    });
                    if in_ligature || is_stl {
                        format.color = Color32::TRANSPARENT;
                        job.append(" ", 0.0, format);
                    } else {
                        job.append(&cell.text, 0.0, format);
                    }
                }
                self.row_layouts[row_idx] = Some((revision, painter.layout_job(job), row_ligatures));
            }
            let font_row_height = self
                .row_layouts
                .iter()
                .filter_map(|opt| opt.as_ref().map(|(_, g, _)| g.size().y))
                .next()
                .unwrap_or(font_size * 1.25);
            let y_offset = ((nvim_row_height - font_row_height) * 0.5).max(0.0).round();

            let row_top = origin.y + row_idx as f32 * nvim_row_height;
            let row_h = nvim_row_height + 0.5;

            let mut span_start: Option<(usize, Color32)> = None;
            for (col_offset, cell) in row
                .iter()
                .skip(number_columns)
                .take(width.saturating_sub(number_columns))
                .enumerate()
            {
                let col_idx = number_columns + col_offset;
                let is_cursor_cell =
                    row_idx == self.grid.cursor.row && col_idx == self.grid.cursor.column;
                let mut bg_color = None;
                if let Some(style) = self.grid.highlights.get(&cell.highlight) {
                    if style.is_statusline {
                        bg_color = None;
                    } else if style.reverse {
                        if !is_cursor_cell || caret.kind == crate::caret::CaretKind::Block {
                            bg_color = Some(style.foreground.map(rgb).unwrap_or(theme.text));
                        }
                    } else if let Some(color) = style.background {
                        let c = rgb(color);
                        let is_sel = style.is_visual
                            || (c.r() == theme.highlight.r()
                                && c.g() == theme.highlight.g()
                                && c.b() == theme.highlight.b());
                        if is_sel {
                            let alpha = if theme.is_light() { 55 } else { 40 };
                            bg_color =
                                Some(Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha));
                        } else {
                            bg_color = Some(c);
                        }
                    }
                }
                match (span_start, bg_color) {
                    (Some((_start_col, cur_bg)), Some(new_bg)) if cur_bg == new_bg => {}
                    (Some((start_col, cur_bg)), _) => {
                        let x1 = text_left + start_col as f32 * cell_width;
                        let x2 = text_left + col_offset as f32 * cell_width;
                        painter.rect_filled(
                            Rect::from_min_max(
                                Pos2::new(x1, row_top),
                                Pos2::new(x2, row_top + row_h),
                            ),
                            0.0,
                            cur_bg,
                        );
                        span_start = bg_color.map(|c| (col_offset, c));
                    }
                    (None, Some(new_bg)) => {
                        span_start = Some((col_offset, new_bg));
                    }
                    (None, None) => {}
                }
            }
            if let Some((start_col, cur_bg)) = span_start {
                let max_col = width.saturating_sub(number_columns);
                let x1 = text_left + start_col as f32 * cell_width;
                let x2 = text_left + max_col as f32 * cell_width;
                painter.rect_filled(
                    Rect::from_min_max(Pos2::new(x1, row_top), Pos2::new(x2, row_top + row_h)),
                    0.0,
                    cur_bg,
                );
            }

            if let Some((_, galley, ligatures)) = &self.row_layouts[row_idx] {
                let y_pos = origin.y + row_idx as f32 * nvim_row_height + y_offset;
                painter.galley(Pos2::new(text_left, y_pos), Arc::clone(galley), theme.text);

                for lig in ligatures {
                    let x_start = text_left + lig.col_offset as f32 * cell_width;
                    let y_mid = row_top + nvim_row_height * 0.5;
                    crate::view_editor::ligatures::draw_ligature(
                        &painter,
                        lig.kind,
                        x_start,
                        y_mid,
                        cell_width,
                        Stroke::new(1.35, lig.color),
                    );
                }
            }

            for (col_offset, cell) in row
                .iter()
                .skip(number_columns)
                .take(width.saturating_sub(number_columns))
                .enumerate()
            {
                let Some(style) = self.grid.highlights.get(&cell.highlight) else {
                    continue;
                };
                if !style.underline {
                    continue;
                }
                let x = text_left + col_offset as f32 * cell_width;
                let y = row_top + nvim_row_height - 1.0;
                let color = style.special.map(rgb).unwrap_or(theme.accent);
                painter.line_segment(
                    [Pos2::new(x, y), Pos2::new(x + cell_width, y)],
                    Stroke::new(1.0, color),
                );
            }
            if number_columns > 0 {
                let number = row
                    .iter()
                    .take(number_columns)
                    .filter(|cell| {
                        self.grid
                            .highlights
                            .get(&cell.highlight)
                            .is_some_and(|s| s.is_gutter)
                    })
                    .map(|cell| cell.text.as_str())
                    .collect::<String>();
                let number = number.trim();
                let current = row_idx == self.grid.cursor.row;
                let number_color = if current {
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
                let gutter_painter = painter.with_clip_rect(Rect::from_min_max(
                    rect.min,
                    Pos2::new(rect.min.x + gutter_w, rect.max.y),
                ));
                let label = if number.is_empty() {
                    let has_text = row
                        .iter()
                        .skip(number_columns)
                        .any(|cell| !cell.text.trim().is_empty());
                    if has_text {
                        "·"
                    } else {
                        ""
                    }
                } else {
                    number
                };
                if !label.is_empty() {
                    gutter_painter.text(
                        Pos2::new(
                            rect.min.x + gutter_w - 4.0,
                            origin.y + row_idx as f32 * nvim_row_height + y_offset,
                        ),
                        Align2::RIGHT_TOP,
                        label,
                        crate::services::font_manager::editor_font_id(font_size * 0.82),
                        number_color,
                    );
                }
            }
        }
        }

        let font_row_height = self
            .row_layouts
            .iter()
            .filter_map(|opt| opt.as_ref().map(|(_, g, _)| g.size().y))
            .next()
            .unwrap_or(font_size * 1.25);
        let y_offset = ((nvim_row_height - font_row_height) * 0.5).max(0.0).round();

        let cursor_row = self.grid.cursor.row;
        let cursor_column = self.grid.cursor.column;
        if cursor_row < rows && cursor_column < width {
            let is_visual = self.grid.mode == "visual"
                || self.grid.mode.starts_with('v')
                || self.grid.mode.starts_with('V')
                || self.grid.mode == "\x16";
            let visual_offset_x = if is_visual && caret.kind != crate::caret::CaretKind::Block {
                let next_col = cursor_column + 1;
                let current_highlight = self
                    .grid
                    .cells
                    .get(cursor_row)
                    .and_then(|r| r.get(cursor_column))
                    .map(|c| c.highlight)
                    .unwrap_or(0);
                let next_is_same_highlight = if next_col < width {
                    self.grid
                        .cells
                        .get(cursor_row)
                        .and_then(|r| r.get(next_col))
                        .map(|c| c.highlight)
                        == Some(current_highlight)
                } else {
                    false
                };
                if next_is_same_highlight {
                    0.0
                } else {
                    cell_width
                }
            } else {
                0.0
            };
            let cursor_x = origin.x + cursor_column as f32 * cell_width + visual_offset_x;
            let cursor_y = origin.y + cursor_row as f32 * nvim_row_height;
            let target = Pos2::new(cursor_x, cursor_y);
            if !self.cursor_render_initialized {
                caret.pos = target;
                self.cursor_render_initialized = true;
                response.request_focus();
            }
            if self.is_insert_mode() || self.grid.mode.starts_with('r') {
                caret.pos = target;
                caret.gliding = false;
            }
            caret.update(
                dt,
                target,
                typed && (self.is_insert_mode() || self.grid.mode.starts_with('r')),
                now,
                cell_width,
                nvim_row_height,
            );
            caret.paint(
                &painter,
                cell_width,
                nvim_row_height,
                now,
                theme.accent,
                theme.is_light(),
            );

            if caret.kind == crate::caret::CaretKind::Block {
                if let Some(cell) = self
                    .grid
                    .cells
                    .get(cursor_row)
                    .and_then(|row| row.get(cursor_column))
                {
                    if !cell.text.trim().is_empty() {
                        painter.text(
                            Pos2::new(caret.pos.x, caret.pos.y + y_offset),
                            Align2::LEFT_TOP,
                            &cell.text,
                            font.clone(),
                            theme.text,
                        );
                    }
                }
            }
        }

        // Neovim's command line (`:` / `/`) arrives through `ext_cmdline`, not as
        // grid text. Without drawing it here it would be invisible while Neovim
        // is in command mode.
        let has_cmdline = !self.grid.command_line.is_empty();
        if has_cmdline {
            let fixed_font = crate::services::font_manager::editor_font_id(13.0);
            painter.text(
                Pos2::new(text_left, rect.max.y - nvim_row_height),
                Align2::LEFT_TOP,
                &self.grid.command_line,
                fixed_font,
                theme.text,
            );
        }

        if !self.grid.message.is_empty() {
            let fixed_font = crate::services::font_manager::editor_font_id(11.5);
            let icon_font = crate::services::font_manager::editor_font_id(11.0);

            let msg_text = &self.grid.message;
            let is_error = self.grid.message_is_error;
            let is_copied = self
                .grid
                .message_copied_at
                .is_some_and(|t| t.elapsed() < Duration::from_millis(1500));

            // Subtle semantic colors
            let prefix_color = if is_error {
                Color32::from_rgb(235, 105, 105)
            } else {
                theme.accent.gamma_multiply(0.85)
            };
            let text_color = if is_error {
                Color32::from_rgb(245, 145, 145)
            } else {
                theme.text.lerp_to_gamma(theme.muted, 0.25)
            };

            // Subtle Copy button text & vector icon
            let copy_label = if is_copied { "Copied" } else { "Copy" };
            let copy_color = if is_copied {
                theme.accent
            } else {
                theme.muted.lerp_to_gamma(theme.text, 0.35)
            };
            let copy_galley =
                painter.layout_no_wrap(copy_label.to_string(), icon_font.clone(), copy_color);
            let icon_w = 11.0;
            let copy_btn_w = icon_w + 4.0 + copy_galley.size().x + 12.0;
            let copy_btn_h = 18.0;
            let prefix_w = 10.0;

            // Layout single-line message with layout_no_wrap so it never wraps or collides with Copy
            let max_text_w =
                (rect.width() - prefix_w - copy_btn_w - 48.0).clamp(80.0, 620.0);
            let single_line_msg = msg_text
                .lines()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            let approx_char_w = (font_size * 0.52).max(6.0);
            let max_chars = ((max_text_w / approx_char_w).floor() as usize).max(16);
            let display_msg = if single_line_msg.chars().count() > max_chars {
                let kept: String = single_line_msg
                    .chars()
                    .take(max_chars.saturating_sub(3))
                    .collect();
                format!("{kept}...")
            } else {
                single_line_msg
            };
            let msg_galley =
                painter.layout_no_wrap(display_msg, fixed_font.clone(), text_color);

            let pill_h = (msg_galley.size().y + 8.0).max(24.0);
            let pill_w = (prefix_w + msg_galley.size().x + copy_btn_w + 28.0)
                .min(rect.width() - 16.0);

            // Horizontally centered near the bottom of the editor
            let pill_bottom_gap = if has_cmdline {
                nvim_row_height + 10.0
            } else {
                10.0
            };
            let pill_y = (rect.max.y - pill_bottom_gap - pill_h).max(rect.min.y + 4.0);
            let min_x = rect.min.x + 8.0;
            let max_x = (rect.max.x - pill_w - 8.0).max(min_x);
            let pill_x = (rect.center().x - pill_w * 0.5).clamp(min_x, max_x);

            let pill_rect =
                Rect::from_min_size(Pos2::new(pill_x, pill_y), egui::vec2(pill_w, pill_h));

            // Subtle translucent floating capsule
            let pill_bg = if theme.is_light() {
                Color32::from_rgba_unmultiplied(248, 250, 252, 225)
            } else {
                Color32::from_rgba_unmultiplied(22, 24, 30, 215)
            };
            let pill_border = if is_error {
                Color32::from_rgba_unmultiplied(235, 95, 95, 110)
            } else {
                theme.border().gamma_multiply(0.45)
            };

            painter.rect(
                pill_rect,
                12.0,
                pill_bg,
                Stroke::new(0.8, pill_border),
                egui::StrokeKind::Inside,
            );

            // Draw crisp vector status indicator dot
            let content_y = pill_rect.center().y;
            let dot_center = Pos2::new(pill_rect.min.x + 12.0, content_y);
            painter.circle_filled(dot_center, 3.2, prefix_color);

            // Draw message text
            let text_x = pill_rect.min.x + 10.0 + prefix_w;
            let text_pos = Pos2::new(text_x, content_y - msg_galley.size().y * 0.5);
            painter.galley(text_pos, msg_galley, text_color);

            // Subtle Copy chip on the right
            let copy_btn_rect = Rect::from_center_size(
                Pos2::new(pill_rect.max.x - copy_btn_w * 0.5 - 5.0, content_y),
                egui::vec2(copy_btn_w, copy_btn_h),
            );

            let copy_hovered = ui.rect_contains_pointer(copy_btn_rect);
            if copy_hovered {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }

            let btn_bg = if is_copied {
                theme.accent.gamma_multiply(0.16)
            } else if copy_hovered {
                theme.highlight.gamma_multiply(0.25)
            } else {
                Color32::TRANSPARENT
            };
            let btn_stroke = if is_copied {
                Stroke::new(0.8, theme.accent.gamma_multiply(0.75))
            } else if copy_hovered {
                Stroke::new(0.8, theme.border().gamma_multiply(0.65))
            } else {
                Stroke::NONE
            };

            painter.rect(
                copy_btn_rect,
                9.0,
                btn_bg,
                btn_stroke,
                egui::StrokeKind::Inside,
            );

            // Vector copy / checkmark icon inside Copy chip
            let icon_x = copy_btn_rect.min.x + 6.0;
            let icon_stroke = Stroke::new(1.15, copy_color);
            if is_copied {
                let p1 = Pos2::new(icon_x + 1.5, content_y + 0.5);
                let p2 = Pos2::new(icon_x + 4.2, content_y + 3.0);
                let p3 = Pos2::new(icon_x + 9.2, content_y - 2.8);
                painter.line_segment([p1, p2], icon_stroke);
                painter.line_segment([p2, p3], icon_stroke);
            } else {
                let back_sheet = Rect::from_min_size(
                    Pos2::new(icon_x + 1.0, content_y - 4.8),
                    egui::vec2(6.2, 7.2),
                );
                let front_sheet = Rect::from_min_size(
                    Pos2::new(icon_x + 3.2, content_y - 2.6),
                    egui::vec2(6.2, 7.2),
                );
                painter.rect_stroke(back_sheet, 1.2, icon_stroke, egui::StrokeKind::Inside);
                painter.rect_filled(front_sheet, 1.2, pill_bg);
                painter.rect_stroke(front_sheet, 1.2, icon_stroke, egui::StrokeKind::Inside);
            }

            let copy_text_pos = Pos2::new(
                icon_x + icon_w + 3.0,
                copy_btn_rect.center().y - copy_galley.size().y * 0.5,
            );
            painter.galley(copy_text_pos, copy_galley, copy_color);

            // Check click on copy button
            let primary_clicked = ui.input(|i| {
                i.pointer.primary_clicked()
                    || i.pointer.button_clicked(egui::PointerButton::Primary)
            });
            if copy_hovered && primary_clicked {
                ui.ctx().copy_text(self.grid.message.clone());
                self.grid.message_copied_at = Some(std::time::Instant::now());
                ui.ctx().request_repaint();
            }
        }

        // Popup completion menu
        let mut popup_hit: Option<(Rect, usize)> = None;
        let hover_pos = response.hover_pos();
        if !self.grid.popup_items.is_empty() {
            const MAX_ROWS: usize = 12;
            let total = self.grid.popup_items.len();
            let visible = total.min(MAX_ROWS);
            let selected = self.grid.popup_selected;
            let start = selected
                .map_or(0, |s| (s + 1).saturating_sub(visible))
                .min(total - visible);
            const DETAIL_CHAR: f32 = 6.4;
            let label_chars = self
                .grid
                .popup_items
                .iter()
                .map(|s| s.chars().count())
                .max()
                .unwrap_or(0);
            let detail_chars = self
                .grid
                .popup_meta
                .iter()
                .map(|(_, e)| e.chars().count())
                .max()
                .unwrap_or(0)
                .min(40);
            let wanted =
                30.0 + label_chars as f32 * cell_width + 28.0 + detail_chars as f32 * DETAIL_CHAR;
            let popup_width = wanted.clamp(200.0, 520.0).min(rect.width());
            let popup_height = (visible as f32 * nvim_row_height).min(rect.height());
            let (popup_row, popup_col) = self
                .grid
                .popup_anchor
                .unwrap_or((self.grid.cursor.row, self.grid.cursor.column));
            let popup_x = (origin.x + popup_col as f32 * cell_width)
                .clamp(origin.x, (rect.max.x - popup_width).max(origin.x));
            let below_y = origin.y + (popup_row + 1) as f32 * nvim_row_height;
            let popup_y = if below_y + popup_height <= rect.max.y {
                below_y
            } else {
                (origin.y + popup_row as f32 * nvim_row_height - popup_height).max(origin.y)
            };
            let popup_rect = Rect::from_min_size(
                Pos2::new(popup_x, popup_y),
                egui::vec2(popup_width, popup_height),
            );
            popup_hit = Some((popup_rect, start));
            let small = crate::services::font_manager::editor_font_id(11.0);
            painter.rect_filled(
                popup_rect.translate(egui::vec2(0.0, 4.0)),
                8.0,
                Color32::from_black_alpha(65),
            );
            painter.rect(
                popup_rect,
                8.0,
                theme.surface(),
                Stroke::new(1.0, theme.border()),
                egui::StrokeKind::Inside,
            );
            for (row, index) in (start..start + visible).enumerate() {
                let item = &self.grid.popup_items[index];
                let (kind, extra) = self
                    .grid
                    .popup_meta
                    .get(index)
                    .map(|(k, e)| (k.as_str(), e.as_str()))
                    .unwrap_or(("", ""));
                let y = popup_y + row as f32 * nvim_row_height;
                let hovered = hover_pos.is_some_and(|p| {
                    p.x >= popup_rect.min.x
                        && p.x <= popup_rect.max.x
                        && p.y >= y
                        && p.y < y + nvim_row_height
                });
                let is_selected = selected == Some(index);
                if is_selected || hovered {
                    let fill = if is_selected {
                        theme.accent.gamma_multiply(0.25)
                    } else {
                        theme.surface().lerp_to_gamma(theme.accent, 0.12)
                    };
                    painter.rect_filled(
                        Rect::from_min_size(
                            Pos2::new(popup_rect.min.x + 2.0, y + 1.0),
                            egui::vec2(popup_rect.width() - 4.0, nvim_row_height - 2.0),
                        ),
                        4.0,
                        fill,
                    );
                }
                let icon_color = match kind {
                    "ƒ" => Color32::from_rgb(120, 180, 255),
                    "p" | "x" => Color32::from_rgb(255, 180, 100),
                    "C" | "S" | "E" | "I" => Color32::from_rgb(255, 215, 100),
                    "k" => Color32::from_rgb(200, 120, 255),
                    "▣" => Color32::from_rgb(100, 220, 180),
                    _ => theme.muted,
                };
                painter.text(
                    Pos2::new(popup_rect.min.x + 8.0, y + 2.0),
                    Align2::LEFT_TOP,
                    kind,
                    small.clone(),
                    icon_color,
                );
                let label_color = if is_selected {
                    theme.highlight
                } else {
                    theme.text
                };
                painter.text(
                    Pos2::new(popup_rect.min.x + 24.0, y + 2.0),
                    Align2::LEFT_TOP,
                    item,
                    font.clone(),
                    label_color,
                );
                if !extra.is_empty() {
                    let extra_w = extra.chars().count() as f32 * DETAIL_CHAR;
                    let extra_x = (popup_rect.max.x - 8.0 - extra_w).max(popup_rect.min.x + 120.0);
                    painter.text(
                        Pos2::new(extra_x, y + 2.0),
                        Align2::LEFT_TOP,
                        extra,
                        small.clone(),
                        theme.muted,
                    );
                }
            }
            let doc = selected
                .and_then(|sel| self.grid.popup_info.get(sel))
                .map(|info| clean_documentation(info))
                .unwrap_or_default();
            if !doc.is_empty() {
                let space_right = rect.max.x - popup_rect.max.x - 6.0;
                let space_left = popup_rect.min.x - rect.min.x - 6.0;
                let on_right = space_right >= 200.0 || space_right >= space_left;
                let pane_width =
                    (if on_right { space_right } else { space_left }).clamp(120.0, 360.0);
                if pane_width >= 120.0 {
                    let galley = painter.layout(
                        doc,
                        small.clone(),
                        theme.text.linear_multiply(0.85),
                        pane_width - 20.0,
                    );
                    let max_height = (nvim_row_height * 14.0).min(rect.height());
                    let pane_height = (galley.size().y + 16.0).min(max_height);
                    let pane_x = if on_right {
                        popup_rect.max.x + 6.0
                    } else {
                        popup_rect.min.x - pane_width - 6.0
                    };
                    let pane_y = popup_y.min((rect.max.y - pane_height).max(rect.min.y));
                    let pane = Rect::from_min_size(
                        Pos2::new(pane_x, pane_y),
                        egui::vec2(pane_width, pane_height),
                    );
                    painter.rect_filled(
                        pane.translate(egui::vec2(0.0, 4.0)),
                        8.0,
                        Color32::from_black_alpha(65),
                    );
                    painter.rect(
                        pane,
                        8.0,
                        theme.surface(),
                        Stroke::new(1.0, theme.border()),
                        egui::StrokeKind::Inside,
                    );
                    painter.with_clip_rect(pane.shrink(1.0)).galley(
                        Pos2::new(pane.min.x + 10.0, pane.min.y + 8.0),
                        galley,
                        theme.text,
                    );
                }
            }
            if total > visible {
                let track = Rect::from_min_max(
                    Pos2::new(popup_rect.max.x - 4.0, popup_rect.min.y + 4.0),
                    Pos2::new(popup_rect.max.x - 2.0, popup_rect.max.y - 4.0),
                );
                let thumb_h = (track.height() * visible as f32 / total as f32).max(12.0);
                let top = track.min.y
                    + (track.height() - thumb_h) * start as f32 / (total - visible).max(1) as f32;
                painter.rect_filled(
                    Rect::from_min_size(Pos2::new(track.min.x, top), egui::vec2(2.0, thumb_h)),
                    1.0,
                    theme.text.linear_multiply(0.25),
                );
            }
            if popup_rect.contains(hover_pos.unwrap_or(Pos2::ZERO)) {
                let delta = ui.input(|i| i.raw_scroll_delta.y);
                if delta != 0.0 {
                    let _ = self
                        .client
                        .input(if delta > 0.0 { "<C-p>" } else { "<C-n>" });
                    ui.input_mut(|i| i.raw_scroll_delta = egui::Vec2::ZERO);
                }
            }
        }
        let mut panel_clicked = false;
        if self.lsp_panel.open {
            let click = response
                .clicked()
                .then(|| response.interact_pointer_pos())
                .flatten();
            let small = crate::services::font_manager::editor_font_id(11.0);
            if let Some(index) =
                self.lsp_panel
                    .paint(&painter, rect, nvim_row_height, &font, &small, theme, click)
            {
                if let super::lsp_panel::PanelAction::Run(keys) = self.lsp_panel.activate_row(index)
                {
                    let _ = self.client.input(&keys);
                }
            }
            panel_clicked =
                click.is_some_and(|p| self.lsp_panel.contains(rect, nvim_row_height, p));
            if response.hovered() {
                let delta = ui.input(|i| i.raw_scroll_delta.y);
                if delta != 0.0 {
                    self.lsp_panel.scroll(if delta > 0.0 { -1 } else { 1 });
                }
            }
        }
        if let Some(error) = &self.error {
            painter.text(origin, Align2::LEFT_TOP, error, font, theme.highlight);
        }

        // Mouse interactions
        let mut mouse_actions = Vec::new();
        if let Some(pos) = response
            .interact_pointer_pos()
            .filter(|_| !self.lsp_panel.open && !panel_clicked)
        {
            let row = ((pos.y - origin.y) / nvim_row_height).floor().max(0.0) as usize;
            let column = ((pos.x - origin.x) / cell_width).floor().max(0.0) as usize;
            if let Some(index) =
                popup_hit
                    .filter(|_| response.clicked())
                    .and_then(|(popup_rect, start)| {
                        popup_rect.contains(pos).then(|| {
                            start + ((pos.y - popup_rect.min.y) / nvim_row_height) as usize
                        })
                    })
            {
                self.accept_popup_item(index);
            } else if response.drag_started() {
                mouse_actions.push(EditorMouseEvent::Press {
                    row,
                    column,
                    button: 0,
                });
            } else if response.drag_stopped() {
                mouse_actions.push(EditorMouseEvent::Release {
                    row,
                    column,
                    button: 0,
                });
            } else if response.dragged() {
                mouse_actions.push(EditorMouseEvent::Drag { row, column });
            } else if response.clicked() {
                mouse_actions.push(EditorMouseEvent::Press {
                    row,
                    column,
                    button: 0,
                });
                mouse_actions.push(EditorMouseEvent::Release {
                    row,
                    column,
                    button: 0,
                });
            }
        }
        if response.hovered() && !self.lsp_panel.open {
            let delta = ui.input(|i| {
                if i.raw_scroll_delta.y.abs() > 0.0 {
                    i.raw_scroll_delta.y
                } else {
                    i.smooth_scroll_delta.y
                }
            });
            if delta != 0.0 {
                // Consume the wheel so the delta cannot also move the tab strip,
                // terminal, or any other surface underneath the pointer.
                ui.input_mut(|i| {
                    i.raw_scroll_delta = egui::Vec2::ZERO;
                    i.smooth_scroll_delta = egui::Vec2::ZERO;
                });
                // In Visual mode a scroll drags the selection endpoint and can
                // select the whole buffer in an instant, so we never scroll
                // there — the selection stays exactly as the user made it.
                let is_visual = self.grid.mode == "visual"
                    || self.grid.mode.starts_with('v')
                    || self.grid.mode.starts_with('V')
                    || self.grid.mode == "\x16";
                // Only scroll when the buffer is taller than the visible grid,
                // so a short file (e.g. 15 lines) does not slide the viewport.
                if !is_visual && self.lines.len() > height {
                    let steps = ((delta.abs() / nvim_row_height).ceil() as i32).clamp(1, 8);
                    // Neovim's native scroll keeps the caret on its text line
                    // while there is room and only pushes it at the window edge.
                    // <C-Y> shows earlier lines (wheel up), <C-E> later lines.
                    let key = if delta > 0.0 { "<C-y>" } else { "<C-e>" };
                    let _ = self.client.input(&key.repeat(steps as usize));
                }
            }
        }
        for event in mouse_actions {
            let _ = self.handle_mouse(event);
        }
    }
}

/// Lightweight plain-text editor surface used while the Neovim backend is still
/// starting (or re-attaching) so opening a note never shows a blank editor.
///
/// It mirrors the grid layout (`mod.rs` gutter + top padding + row height) so
/// there is no visible jump when the real grid takes over. It is read-only and
/// purely visual — no input handling, no mutation of authoritative state.
pub fn render_pending_text(
    painter: &egui::Painter,
    rect: Rect,
    text: &str,
    font_size: f32,
    row_height: f32,
    theme: &Theme,
    show_line_numbers: bool,
) {
    const NUMBER_WIDTH: usize = 4;
    let font = crate::services::font_manager::editor_font_id(font_size);
    let number_font = crate::services::font_manager::editor_font_id(font_size * 0.82);
    let gutter_w = if show_line_numbers {
        (NUMBER_WIDTH as f32 * (font_size * 0.55) + 14.0).max(28.0)
    } else {
        0.0
    };
    let effective_gutter_w = if rect.width() > gutter_w + 40.0 {
        gutter_w
    } else {
        0.0
    };
    let pad_x = if show_line_numbers { 16.0 } else { 24.0 };
    let text_left = rect.min.x + effective_gutter_w + pad_x;
    let row_height = row_height.max(1.0);

    let clip = painter.with_clip_rect(rect);
    let mut y = rect.min.y + 10.0;
    for (index, line) in text.lines().enumerate() {
        if y > rect.max.y {
            break;
        }
        if show_line_numbers && effective_gutter_w > 0.0 {
            clip.text(
                egui::pos2(rect.min.x + effective_gutter_w - 4.0, y),
                Align2::RIGHT_TOP,
                (index + 1).to_string(),
                number_font.clone(),
                theme.muted,
            );
        }
        clip.text(
            egui::pos2(text_left, y),
            Align2::LEFT_TOP,
            line,
            font.clone(),
            theme.text,
        );
        y += row_height;
    }
}
