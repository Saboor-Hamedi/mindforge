//! LunaLine rendering engine — renders sleek, modular statusline under the editor.

use super::types::{LunaColorMode, LunaLineConfig, LunaStyle};
use crate::language::{FileLanguage, LanguageSelection, LanguageSelectorState};
use crate::ui::theme::Theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Rect, Shape, Stroke};

pub struct LunaLineRenderParams<'a> {
    pub ui: &'a egui::Ui,
    pub painter: &'a egui::Painter,
    pub dock_rect: Rect,
    pub in_command: bool,
    pub cmd_prefix: char,
    pub cmd_text: &'a str,
    pub cmd_cur: usize,
    pub cmd_selection: Option<(usize, usize)>,
    pub status_msg: &'a str,
    pub status_time: f64,
    /// Label of background work in progress; shows a spinner and a progress bar.
    pub busy: Option<&'a str>,
    pub now: f64,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub total_rows: usize,
    pub total_words: usize,
    pub total_chars: usize,
    pub active_note_title: &'a str,
    pub is_dirty: bool,
    pub is_doc: bool,
    pub mode_badge: Option<&'a str>,
    pub search_prompt: Option<(&'a str, &'a str, usize)>,
    pub theme: &'a Theme,
    pub opacity: f32,
    pub is_ai_open: bool,
    pub config: &'a LunaLineConfig,
    pub language: FileLanguage,
    pub detected_language: FileLanguage,
    pub language_override: Option<FileLanguage>,
    pub language_selector: &'a mut LanguageSelectorState,
}

#[derive(Debug, Default)]
pub struct LunaLineAction {
    pub toggle_ai: bool,
    pub language_selection: Option<LanguageSelection>,
}

/// Resolves mode colors `(bg, fg)` based on the active mode string and color configuration.
pub fn get_mode_colors(mode: &str, color_mode: LunaColorMode, theme: &Theme) -> (Color32, Color32) {
    match color_mode {
        LunaColorMode::Dynamic => {
            let m = mode.to_uppercase();
            if m == "INSERT" {
                // Warm amber / golden orange
                (Color32::from_rgb(230, 145, 20), Color32::from_rgb(18, 18, 20))
            } else if m.starts_with("VISUAL") || m == "V-LINE" {
                // Royal purple / magenta
                (Color32::from_rgb(168, 85, 247), Color32::WHITE)
            } else if m.starts_with("CMD") || m.contains("SEARCH") {
                // Ocean sky blue
                (Color32::from_rgb(14, 165, 233), Color32::WHITE)
            } else if m == "HYBRID" {
                // Cyan / teal
                (Color32::from_rgb(20, 184, 166), Color32::from_rgb(18, 20, 24))
            } else if m.starts_with("DOC") {
                // Indigo
                (Color32::from_rgb(99, 102, 241), Color32::WHITE)
            } else {
                // NORMAL
                (theme.accent, if theme.is_light() { Color32::WHITE } else { Color32::from_rgb(16, 18, 20) })
            }
        }
        LunaColorMode::ThemeAccent => {
            (theme.accent, if theme.is_light() { Color32::WHITE } else { Color32::from_rgb(16, 18, 20) })
        }
        LunaColorMode::Monochrome => {
            if theme.is_light() {
                (Color32::from_rgb(45, 48, 55), Color32::WHITE)
            } else {
                (Color32::from_rgb(220, 225, 235), Color32::from_rgb(24, 26, 30))
            }
        }
    }
}

/// Renders the complete LunaLine dock.
pub fn render_lunaline(mut params: LunaLineRenderParams) -> LunaLineAction {
    let ui = params.ui;
    let painter = params.painter;
    let dock_rect = params.dock_rect;
    let theme = params.theme;
    let config = params.config;
    let now = params.now;
    let mut action = LunaLineAction::default();

    if !config.enabled || dock_rect.height() < 10.0 {
        return action;
    }

    let dock_alpha = (params.opacity * 255.0) as u8;
    let base_dock_bg = Color32::from_rgba_unmultiplied(
        theme.bg.r(),
        theme.bg.g(),
        theme.bg.b(),
        dock_alpha,
    );

    // ── 1. Render Outer Dock Surface ─────────────────────────────────────────
    let actual_bar_rect = match config.style {
        LunaStyle::Floating => {
            let inset = 4.0;
            Rect::from_min_max(
                pos2(dock_rect.min.x + inset, dock_rect.min.y + 2.0),
                pos2(dock_rect.max.x - inset, dock_rect.max.y - 2.0),
            )
        }
        _ => dock_rect,
    };

    match config.style {
        LunaStyle::Floating => {
            painter.rect(
                actual_bar_rect,
                8.0,
                theme.surface(),
                Stroke::new(1.0_f32, theme.border()),
                egui::StrokeKind::Inside,
            );
        }
        LunaStyle::Pill => {
            painter.rect(
                actual_bar_rect,
                5.0,
                base_dock_bg,
                Stroke::new(0.5_f32, theme.border().gamma_multiply(0.4)),
                egui::StrokeKind::Inside,
            );
        }
        LunaStyle::Powerline => {
            painter.rect(
                actual_bar_rect,
                3.0,
                Color32::from_rgba_unmultiplied(theme.surface().r(), theme.surface().g(), theme.surface().b(), dock_alpha),
                Stroke::NONE,
                egui::StrokeKind::Inside,
            );
        }
        LunaStyle::Minimal => {
            painter.rect(
                actual_bar_rect,
                0.0,
                base_dock_bg,
                Stroke::NONE,
                egui::StrokeKind::Inside,
            );
        }
    }

    let bar_center_y = actual_bar_rect.center().y;

    // ── 2. Interactive Resize Gripper on Bottom-Right ────────────────────────
    let knob_size = 28.0;
    let knob_rect = Rect::from_min_max(
        pos2(actual_bar_rect.max.x - knob_size, actual_bar_rect.max.y - knob_size),
        actual_bar_rect.max,
    );
    let is_knob_hovered = ui.rect_contains_pointer(knob_rect);

    if is_knob_hovered {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeSouthEast);
    }
    if is_knob_hovered && ui.input(|i| i.pointer.primary_down() || i.pointer.primary_clicked()) {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::BeginResize(egui::ResizeDirection::SouthEast));
    }

    let knob_color = if is_knob_hovered { theme.accent } else { theme.muted };
    for &d in &[5.0, 9.0, 13.0, 17.0] {
        painter.line_segment(
            [
                pos2(actual_bar_rect.max.x - d, actual_bar_rect.max.y - 4.0),
                pos2(actual_bar_rect.max.x - 4.0, actual_bar_rect.max.y - d),
            ],
            Stroke::new(1.3_f32, knob_color),
        );
    }

    // ── 3. Right-Side Component Assembly ─────────────────────────────────────
    let mut right_x = actual_bar_rect.max.x - 32.0; // Padded before resize knob
    let is_wide = actual_bar_rect.width() > 640.0;
    let font_info = FontId::monospace(11.0);

    // AI Agent Button
    let mut is_ai_hovered = false;
    if config.show_ai_button {
        let ai_w = 76.0;
        let ai_h = 22.0;
        right_x -= ai_w;
        let ai_btn_rect = Rect::from_min_size(pos2(right_x, bar_center_y - ai_h * 0.5), vec2(ai_w, ai_h));
        is_ai_hovered = ui.rect_contains_pointer(ai_btn_rect);

        if is_ai_hovered {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if is_ai_hovered && ui.input(|i| i.pointer.primary_clicked()) {
            action.toggle_ai = true;
        }

        let (ai_bg, ai_fg) = if params.is_ai_open {
            (Color32::from_rgba_unmultiplied(theme.accent.r(), theme.accent.g(), theme.accent.b(), 40), theme.accent)
        } else if is_ai_hovered {
            (theme.surface(), theme.accent)
        } else {
            (Color32::TRANSPARENT, theme.muted)
        };

        if matches!(config.style, LunaStyle::Pill | LunaStyle::Floating) {
            painter.rect(
                ai_btn_rect,
                11.0,
                ai_bg,
                if params.is_ai_open || is_ai_hovered { Stroke::new(1.0_f32, theme.accent.gamma_multiply(0.7)) } else { Stroke::NONE },
                egui::StrokeKind::Inside,
            );
        }
        painter.text(
            ai_btn_rect.center(),
            Align2::CENTER_CENTER,
            "✦ AI Agent",
            FontId::monospace(10.5),
            ai_fg,
        );
        right_x -= 10.0;
    }

    // Encoding Badge (e.g. "UTF-8")
    if config.show_encoding && is_wide {
        let enc_str = if config.show_line_ending { "UTF-8 [LF]" } else { "UTF-8" };
        let enc_w = (enc_str.len() as f32 * 7.0 + 16.0).max(42.0);
        let enc_h = 20.0;
        right_x -= enc_w;
        let enc_rect = Rect::from_min_size(pos2(right_x, bar_center_y - enc_h * 0.5), vec2(enc_w, enc_h));

        if matches!(config.style, LunaStyle::Pill | LunaStyle::Floating) {
            painter.rect(enc_rect, 4.0, theme.surface(), Stroke::new(0.5_f32, theme.border()), egui::StrokeKind::Inside);
        }
        painter.text(enc_rect.center(), Align2::CENTER_CENTER, enc_str, font_info.clone(), theme.muted);
        right_x -= 8.0;
    }

    let language_label = params.language.label();
    let language_width = language_label.chars().count() as f32 * 7.4 + 25.0;
    right_x -= language_width;
    let language_rect = Rect::from_min_size(
        pos2(right_x, bar_center_y - 11.0),
        vec2(language_width, 22.0),
    );
    let language_hovered = ui.rect_contains_pointer(language_rect);
    if language_hovered {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        if ui.input(|input| input.pointer.primary_clicked()) {
            params.language_selector.open = !params.language_selector.open;
            params.language_selector.query.clear();
            params.language_selector.selected_index = params
                .language_override
                .and_then(|language| {
                    FileLanguage::ALL
                        .iter()
                        .position(|candidate| *candidate == language)
                        .map(|index| index + 1)
                })
                .unwrap_or(0);
            params.language_selector.focus_search = params.language_selector.open;
        }
    }
    painter.text(
        pos2(language_rect.min.x + 7.0, language_rect.center().y),
        Align2::LEFT_CENTER,
        language_label,
        font_info.clone(),
        theme.accent,
    );
    painter.text(
        pos2(language_rect.max.x - 7.0, language_rect.center().y),
        Align2::RIGHT_CENTER,
        "▾",
        font_info.clone(),
        if language_hovered { theme.accent } else { theme.muted },
    );
    right_x -= 8.0;

    // Progress Badge (e.g. "Top", "45%", "Bot")
    if config.show_progress {
        let progress_str = if params.cursor_row <= 1 {
            "Top".to_string()
        } else if params.cursor_row >= params.total_rows.max(1) {
            "Bot".to_string()
        } else {
            let pct = ((params.cursor_row as f32 / params.total_rows.max(1) as f32) * 100.0).round() as usize;
            format!("{}%", pct)
        };
        let prog_w = (progress_str.len() as f32 * 7.5 + 16.0).max(40.0);
        let prog_h = 20.0;
        right_x -= prog_w;
        let prog_rect = Rect::from_min_size(pos2(right_x, bar_center_y - prog_h * 0.5), vec2(prog_w, prog_h));

        if matches!(config.style, LunaStyle::Pill | LunaStyle::Floating) {
            painter.rect(prog_rect, 4.0, theme.surface(), Stroke::new(0.5_f32, theme.border()), egui::StrokeKind::Inside);
        }
        painter.text(prog_rect.center(), Align2::CENTER_CENTER, &progress_str, font_info.clone(), theme.muted);
        right_x -= 8.0;
    }

    // Cursor Position Badge ("Ln 14, Col 23")
    if config.show_cursor_pos {
        let pos_str = format!("Ln {}, Col {}", params.cursor_row, params.cursor_col);
        let pos_w = pos_str.len() as f32 * 7.2 + 16.0;
        let pos_h = 20.0;
        right_x -= pos_w;
        let pos_rect = Rect::from_min_size(pos2(right_x, bar_center_y - pos_h * 0.5), vec2(pos_w, pos_h));

        if matches!(config.style, LunaStyle::Pill | LunaStyle::Floating) {
            painter.rect(pos_rect, 4.0, theme.surface(), Stroke::new(0.5_f32, theme.border()), egui::StrokeKind::Inside);
        }
        painter.text(pos_rect.center(), Align2::CENTER_CENTER, &pos_str, font_info.clone(), theme.text);
        right_x -= 8.0;
    }

    // Word & Reading Time Metrics
    if (config.show_word_count || config.show_reading_time || config.show_char_count) && is_wide {
        let mut parts = Vec::new();
        if config.show_word_count {
            parts.push(format!("{} words", params.total_words));
        }
        if config.show_reading_time {
            let read_mins = (params.total_words / 200).max(1);
            parts.push(format!("{}m read", read_mins));
        }
        if config.show_char_count {
            parts.push(format!("{} chars", params.total_chars));
        }
        let metrics_str = parts.join(" · ");
        let metrics_w = metrics_str.len() as f32 * 7.0 + 16.0;
        let metrics_h = 20.0;
        right_x -= metrics_w;
        let metrics_rect = Rect::from_min_size(pos2(right_x, bar_center_y - metrics_h * 0.5), vec2(metrics_w, metrics_h));

        if matches!(config.style, LunaStyle::Pill | LunaStyle::Floating) {
            painter.rect(metrics_rect, 4.0, theme.surface(), Stroke::new(0.5_f32, theme.border()), egui::StrokeKind::Inside);
        }
        painter.text(metrics_rect.center(), Align2::CENTER_CENTER, &metrics_str, font_info.clone(), theme.muted);
        right_x -= 12.0;
    }

    let right_boundary_x = right_x.max(actual_bar_rect.min.x + 160.0);

    // ── 4. Left-Side Assembly (Mode, File Info, Command, Search, Status) ─────
    let mut left_x = actual_bar_rect.min.x + 10.0;
    let mode_str = params.mode_badge.unwrap_or("NORMAL");
    let (mode_bg, mode_fg) = get_mode_colors(mode_str, config.color_mode, theme);

    if params.in_command {
        // Active Command Mode Input Box
        let cmd_prompt = if params.cmd_prefix == ':' {
            ":CMD".to_string()
        } else {
            format!("{} FIND", params.cmd_prefix)
        };
        let prompt_w = cmd_prompt.len() as f32 * 7.5 + 16.0;
        let prompt_rect = Rect::from_min_size(pos2(left_x, bar_center_y - 11.0), vec2(prompt_w, 22.0));
        let (cmd_bg, cmd_fg) = get_mode_colors("CMD", config.color_mode, theme);

        painter.rect(prompt_rect, 5.0, cmd_bg, Stroke::NONE, egui::StrokeKind::Inside);
        painter.text(prompt_rect.center(), Align2::CENTER_CENTER, &cmd_prompt, FontId::monospace(11.0), cmd_fg);
        left_x += prompt_w + 10.0;

        let cmd_avail_w = (right_boundary_x - left_x - 10.0).max(40.0);
        let cmd_font = FontId::monospace(13.5);
        let cmd_clip_rect = Rect::from_min_max(pos2(left_x, actual_bar_rect.min.y), pos2(left_x + cmd_avail_w, actual_bar_rect.max.y));
        let cmd_painter = painter.with_clip_rect(cmd_clip_rect);

        let cur_clamped = params.cmd_cur.min(params.cmd_text.len());
        let mut valid_cur = cur_clamped;
        while !params.cmd_text.is_char_boundary(valid_cur) && valid_cur > 0 {
            valid_cur -= 1;
        }
        let before_cur = &params.cmd_text[..valid_cur];
        let cursor_offset_x = cmd_painter.layout_no_wrap(before_cur.to_string(), cmd_font.clone(), theme.text).size().x;
        let total_text_w = cmd_painter.layout_no_wrap(params.cmd_text.to_string(), cmd_font.clone(), theme.text).size().x;

        let scroll_x = if total_text_w > cmd_avail_w {
            let max_scroll = (total_text_w - cmd_avail_w + 24.0).max(0.0);
            (cursor_offset_x - (cmd_avail_w - 24.0)).clamp(0.0, max_scroll)
        } else {
            0.0
        };

        let text_origin = pos2(left_x - scroll_x, bar_center_y - 8.0);

        if let Some((start, end)) = params.cmd_selection {
            let s_min = start.min(end).min(params.cmd_text.len());
            let s_max = start.max(end).min(params.cmd_text.len());
            let mut valid_min = s_min;
            while !params.cmd_text.is_char_boundary(valid_min) && valid_min > 0 { valid_min -= 1; }
            let mut valid_max = s_max;
            while !params.cmd_text.is_char_boundary(valid_max) && valid_max > 0 { valid_max -= 1; }
            let prefix = &params.cmd_text[..valid_min];
            let selected_part = &params.cmd_text[valid_min..valid_max];
            let x_off = cmd_painter.layout_no_wrap(prefix.to_string(), cmd_font.clone(), theme.text).size().x;
            let sel_w = cmd_painter.layout_no_wrap(selected_part.to_string(), cmd_font.clone(), theme.text).size().x;
            cmd_painter.rect_filled(
                Rect::from_min_size(pos2(text_origin.x + x_off, text_origin.y), vec2(sel_w, 18.0)),
                2.0,
                Color32::from_rgba_unmultiplied(theme.accent.r(), theme.accent.g(), theme.accent.b(), 90),
            );
        }

        let galley = cmd_painter.layout_no_wrap(params.cmd_text.to_string(), cmd_font.clone(), theme.text);
        cmd_painter.galley(text_origin, galley, theme.text);

        let cursor_x = text_origin.x + cursor_offset_x;
        let blink = ((now * 2.5).sin() > -0.2) as i32 != 0;
        if blink {
            cmd_painter.rect_filled(
                Rect::from_min_size(pos2(cursor_x, text_origin.y), vec2(2.0, 16.0)),
                1.0,
                theme.accent,
            );
        }
    } else if let Some((symbol, query, match_count)) = params.search_prompt {
        // Active In-Buffer Search
        let prompt_str = if symbol == "?" { "? SEARCH" } else { "/ SEARCH" };
        let prompt_w = prompt_str.len() as f32 * 7.5 + 16.0;
        let prompt_rect = Rect::from_min_size(pos2(left_x, bar_center_y - 11.0), vec2(prompt_w, 22.0));
        let (s_bg, s_fg) = get_mode_colors("SEARCH", config.color_mode, theme);

        painter.rect(prompt_rect, 5.0, s_bg, Stroke::NONE, egui::StrokeKind::Inside);
        painter.text(prompt_rect.center(), Align2::CENTER_CENTER, prompt_str, FontId::monospace(11.0), s_fg);
        left_x += prompt_w + 10.0;

        let search_text = format!("{}{}_", symbol, query);
        painter.text(pos2(left_x, bar_center_y), Align2::LEFT_CENTER, search_text, FontId::monospace(13.0), theme.text);
        let q_w = query.len() as f32 * 8.0 + 20.0;
        let match_info = if match_count == 0 { "(no matches)".into() } else { format!("({} matches)", match_count) };
        painter.text(pos2(left_x + q_w, bar_center_y), Align2::LEFT_CENTER, match_info, font_info.clone(), theme.muted);
    } else {
        // Mode Badge
        if config.show_mode {
            let badge_w = mode_str.len() as f32 * 7.8 + 18.0;
            let badge_h = 22.0;
            let mode_rect = Rect::from_min_size(pos2(left_x, bar_center_y - badge_h * 0.5), vec2(badge_w, badge_h));

            match config.style {
                LunaStyle::Pill | LunaStyle::Floating => {
                    painter.rect(mode_rect, 11.0, mode_bg, Stroke::NONE, egui::StrokeKind::Inside);
                    painter.text(mode_rect.center(), Align2::CENTER_CENTER, mode_str, FontId::monospace(11.0), mode_fg);
                    left_x += badge_w + 8.0;
                }
                LunaStyle::Powerline => {
                    painter.rect(mode_rect, 2.0, mode_bg, Stroke::NONE, egui::StrokeKind::Inside);
                    painter.text(mode_rect.center(), Align2::CENTER_CENTER, mode_str, FontId::monospace(11.0), mode_fg);
                    // Chevron arrow pointing right
                    let arrow_w = 8.0;
                    let p1 = pos2(mode_rect.max.x, mode_rect.min.y);
                    let p2 = pos2(mode_rect.max.x + arrow_w, bar_center_y);
                    let p3 = pos2(mode_rect.max.x, mode_rect.max.y);
                    painter.add(Shape::convex_polygon(vec![p1, p2, p3], mode_bg, Stroke::NONE));
                    left_x += badge_w + arrow_w + 8.0;
                }
                LunaStyle::Minimal => {
                    painter.text(mode_rect.center(), Align2::CENTER_CENTER, mode_str, FontId::monospace(11.0), mode_bg);
                    left_x += badge_w + 4.0;
                    painter.text(pos2(left_x, bar_center_y), Align2::LEFT_CENTER, "·", font_info.clone(), theme.muted);
                    left_x += 10.0;
                }
            }
        }

        // Note Info & Dirty Dot
        if config.show_file_info && !params.active_note_title.is_empty() {
            let note_icon = if params.is_doc { "📖" } else { "📄" };
            let title_display = if params.active_note_title.len() > 28 {
                format!("{}…", &params.active_note_title[..26])
            } else {
                params.active_note_title.to_string()
            };
            let info_str = format!("{} {}", note_icon, title_display);
            let info_w = info_str.len() as f32 * 7.0 + if params.is_dirty { 26.0 } else { 16.0 };
            let info_h = 22.0;

            if left_x + info_w < right_boundary_x {
                let info_rect = Rect::from_min_size(pos2(left_x, bar_center_y - info_h * 0.5), vec2(info_w, info_h));

                if matches!(config.style, LunaStyle::Pill | LunaStyle::Floating) {
                    painter.rect(info_rect, 6.0, theme.surface(), Stroke::new(0.5_f32, theme.border()), egui::StrokeKind::Inside);
                }
                let text_anchor = pos2(info_rect.min.x + 8.0, bar_center_y);
                painter.text(text_anchor, Align2::LEFT_CENTER, &info_str, font_info.clone(), theme.text);

                if params.is_dirty {
                    let dot_pos = pos2(info_rect.max.x - 10.0, bar_center_y);
                    painter.circle_filled(dot_pos, 3.5, theme.accent);
                }
                left_x += info_w + 8.0;
            }
        }

        // Background work (installs, LSP indexing) takes the status slot.
        if let Some(label) = params.busy {
            let spinner_center = pos2(left_x + 8.0, bar_center_y);
            paint_spinner(painter, spinner_center, 6.0, now, theme.accent);
            let text_x = left_x + 22.0;
            let avail = (right_boundary_x - text_x - 12.0).max(0.0);
            if avail > 40.0 {
                let clip = Rect::from_min_max(pos2(text_x, actual_bar_rect.min.y), pos2(text_x + avail, actual_bar_rect.max.y));
                painter.with_clip_rect(clip).text(pos2(text_x, bar_center_y), Align2::LEFT_CENTER, label, font_info.clone(), theme.text);
            }
        } else if !params.status_msg.is_empty() && (now - params.status_time) < 3.5 {
            let fade_t = ((3.5 - (now - params.status_time)) / 0.5).clamp(0.0, 1.0) as f32;
            let status_alpha = (fade_t * 255.0) as u8;
            let status_color = Color32::from_rgba_unmultiplied(theme.text.r(), theme.text.g(), theme.text.b(), status_alpha);
            let avail_status_w = (right_boundary_x - left_x - 12.0).max(0.0);

            if avail_status_w > 40.0 {
                let status_clip = Rect::from_min_max(pos2(left_x, actual_bar_rect.min.y), pos2(left_x + avail_status_w, actual_bar_rect.max.y));
                let status_painter = painter.with_clip_rect(status_clip);
                status_painter.text(pos2(left_x, bar_center_y), Align2::LEFT_CENTER, params.status_msg, font_info.clone(), status_color);
            }
        }
    }

    // Indeterminate progress bar along the top edge of the dock.
    if params.busy.is_some() {
        let track = Rect::from_min_max(
            pos2(actual_bar_rect.min.x, actual_bar_rect.min.y),
            pos2(actual_bar_rect.max.x, actual_bar_rect.min.y + 2.0),
        );
        let clip = painter.with_clip_rect(track);
        clip.rect_filled(track, 0.0, theme.accent.gamma_multiply(0.18));
        let width = track.width();
        let seg = (width * 0.28).max(40.0);
        let phase = ((now * 0.9) % 1.0) as f32;
        let x = track.min.x - seg + phase * (width + seg);
        clip.rect_filled(
            Rect::from_min_size(pos2(x, track.min.y), vec2(seg, 2.0)),
            1.0,
            theme.accent,
        );
    }

    // ── 5. Empty-Space Window Dragging ────────────────────────────────────────
    if params.language_selector.open {
        let selector = &mut params.language_selector;
        let mut choices = vec![(None, "Auto Detect")];
        choices.extend(
            FileLanguage::ALL
                .into_iter()
                .map(|language| (Some(language), language.label())),
        );
        let query = selector.query.trim().to_ascii_lowercase();
        let filtered: Vec<_> = choices
            .into_iter()
            .filter(|(_, label)| query.is_empty() || label.to_ascii_lowercase().contains(&query))
            .collect();
        selector.selected_index = selector.selected_index.min(filtered.len().saturating_sub(1));

        if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
            selector.open = false;
        } else if !filtered.is_empty() {
            if ui.input(|input| input.key_pressed(egui::Key::ArrowDown)) {
                selector.selected_index = (selector.selected_index + 1).min(filtered.len() - 1);
            }
            if ui.input(|input| input.key_pressed(egui::Key::ArrowUp)) {
                selector.selected_index = selector.selected_index.saturating_sub(1);
            }
            if ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                let (language, _) = filtered[selector.selected_index];
                action.language_selection = Some(match language {
                    Some(language) => LanguageSelection::Language(language),
                    None => LanguageSelection::AutoDetect,
                });
                selector.open = false;
            }
        }

        if selector.open {
            let popup_width = 230.0;
            let popup_height = 300.0;
            let screen = ui.ctx().screen_rect();
            let max_x = (screen.max.x - popup_width - 8.0).max(screen.min.x + 8.0);
            let popup_pos = pos2(
                language_rect.min.x.clamp(screen.min.x + 8.0, max_x),
                (language_rect.min.y - popup_height - 8.0).max(screen.min.y + 8.0),
            );
            egui::Area::new(egui::Id::new("mindforge-language-selector"))
                .order(egui::Order::Foreground)
                .fixed_pos(popup_pos)
                .show(ui.ctx(), |ui| {
                    egui::Frame::popup(ui.style())
                        .fill(theme.surface())
                        .stroke(Stroke::new(1.0, theme.border()))
                        .show(ui, |ui| {
                            ui.set_min_width(popup_width);
                            let search = ui.add(
                                egui::TextEdit::singleline(&mut selector.query)
                                    .id_salt("mindforge-language-search")
                                    .hint_text("Search language...")
                                    .desired_width(popup_width - 16.0),
                            );
                            if selector.focus_search {
                                search.request_focus();
                                selector.focus_search = false;
                            }
                            ui.separator();
                            egui::ScrollArea::vertical()
                                .max_height(popup_height - 62.0)
                                .show(ui, |ui| {
                                    for (index, (language, label)) in filtered.iter().enumerate() {
                                        let is_selected = match language {
                                            Some(language) => params.language_override == Some(*language),
                                            None => params.language_override.is_none(),
                                        };
                                        let prefix = if is_selected { "✓ " } else { "  " };
                                        let color = if is_selected { theme.accent } else { theme.text };
                                        let row_label = if language.is_none() {
                                            format!("{label} ({})", params.detected_language.label())
                                        } else {
                                            (*label).to_string()
                                        };
                                        let response = ui.add_sized(
                                            [popup_width - 16.0, 23.0],
                                            egui::Button::new(
                                                egui::RichText::new(format!("{prefix}{row_label}")).color(color),
                                            )
                                            .fill(if index == selector.selected_index {
                                                theme.bg
                                            } else {
                                                Color32::TRANSPARENT
                                            })
                                            .stroke(Stroke::NONE),
                                        );
                                        if index == selector.selected_index {
                                            response.scroll_to_me(Some(egui::Align::Center));
                                        }
                                        if response.clicked() {
                                            action.language_selection = Some(match language {
                                                Some(language) => LanguageSelection::Language(*language),
                                                None => LanguageSelection::AutoDetect,
                                            });
                                            selector.open = false;
                                        }
                                    }
                                });
                        });
                });
        }
    }

    let is_empty_dock_hovered = ui.rect_contains_pointer(actual_bar_rect)
        && !is_ai_hovered
        && !language_hovered
        && !is_knob_hovered
        && !params.in_command;
    if is_empty_dock_hovered && ui.input(|i| i.pointer.primary_down()) {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }

    action
}

/// Small rotating arc used as an activity indicator.
fn paint_spinner(painter: &egui::Painter, center: egui::Pos2, radius: f32, now: f64, color: Color32) {
    let start = (now * 6.0) as f32;
    let points: Vec<egui::Pos2> = (0..=10)
        .map(|i| {
            let angle = start + i as f32 * 0.28;
            pos2(center.x + radius * angle.cos(), center.y + radius * angle.sin())
        })
        .collect();
    painter.add(Shape::line(points, Stroke::new(1.8, color)));
}
/// Renders a standalone mock preview of LunaLine in the Settings modal.
pub fn render_lunaline_preview(
    ui: &egui::Ui,
    painter: &egui::Painter,
    preview_rect: Rect,
    config: &LunaLineConfig,
    theme: &Theme,
    opacity: f32,
) {
    let mock_params = LunaLineRenderParams {
        ui,
        painter,
        dock_rect: preview_rect,
        in_command: false,
        cmd_prefix: ':',
        cmd_text: "",
        cmd_cur: 0,
        cmd_selection: None,
        status_msg: "Ready",
        status_time: 0.0,
        busy: None,
        now: 10.0,
        cursor_row: 14,
        cursor_col: 23,
        total_rows: 42,
        total_words: 340,
        total_chars: 1890,
        active_note_title: "MindForge Architecture",
        is_dirty: true,
        is_doc: false,
        mode_badge: Some("NORMAL"),
        search_prompt: None,
        theme,
        opacity,
        is_ai_open: false,
        config,
        language: FileLanguage::Markdown,
        detected_language: FileLanguage::Markdown,
        language_override: None,
        language_selector: &mut LanguageSelectorState::default(),
    };
    render_lunaline(mock_params);
}
