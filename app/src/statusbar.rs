//! Dedicated bottom command & status bar dock.

use crate::ui::theme::Theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Rect, Stroke};

#[allow(dead_code)]
pub fn render_bottom_dock(
    ui: &egui::Ui,
    painter: &egui::Painter,
    dock_rect: Rect,
    _content_left_margin: f32,
    in_command: bool,
    cmd_text: &str,
    cmd_cur: usize,
    cmd_selection: Option<(usize, usize)>,
    status_msg: &str,
    status_time: f64,
    now: f64,
    cursor_row: usize,
    cursor_col: usize,
    total_words: usize,
    mode_badge: Option<&str>,
    search_prompt: Option<(&str, &str, usize)>,
    theme: &Theme,
    is_ai_open: bool,
    opacity: f32,
) -> bool {
    let mut toggle_ai = false;
    let accent = theme.accent;
    let muted = theme.muted;

    // Seamless bottom dock matching the editor canvas (translucent so desktop blur shows through)
    let dock_alpha = (opacity * 255.0) as u8;
    let dock_bg = Color32::from_rgba_unmultiplied(
        theme.bg.r(),
        theme.bg.g(),
        theme.bg.b(),
        dock_alpha,
    );
    painter.rect(
        dock_rect,
        5.0,
        dock_bg,
        Stroke::NONE,
        egui::StrokeKind::Inside,
    );

    let cmd_x = dock_rect.min.x + 14.0;
    let cmd_y = dock_rect.center().y - 8.0;
    let mut text_x = cmd_x;

    // Right side stats: Line, Col, word count (padded before resize knob)
    let stats = if dock_rect.width() > 640.0 {
        format!("Ln {}, Col {}  ·  {} words", cursor_row, cursor_col, total_words)
    } else {
        format!("Ln {}, Col {}", cursor_row, cursor_col)
    };
    let stats_galley = painter.layout_no_wrap(stats, FontId::monospace(12.0), theme.muted);
    let stats_pos = pos2(dock_rect.max.x - 34.0, dock_rect.center().y - 7.0);
    let stats_left_x = stats_pos.x - stats_galley.size().x;

    // AI Agent Button on right side (clean text, no background or border)
    let ai_btn_w = 70.0;
    let ai_btn_h = 22.0;
    let ai_btn_rect = Rect::from_center_size(
        pos2(stats_left_x - (ai_btn_w * 0.5 + 12.0), dock_rect.center().y),
        vec2(ai_btn_w, ai_btn_h),
    );
    let is_ai_hovered = ui.rect_contains_pointer(ai_btn_rect);
    if is_ai_hovered {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if is_ai_hovered && ui.input(|i| i.pointer.primary_clicked()) {
        toggle_ai = true;
    }

    let ai_color = if is_ai_open || is_ai_hovered {
        theme.accent
    } else {
        theme.muted
    };
    painter.text(
        ai_btn_rect.center(),
        Align2::CENTER_CENTER,
        "AI Agent",
        FontId::monospace(11.0),
        ai_color,
    );

    let max_cmd_x = (ai_btn_rect.min.x - 16.0).max(cmd_x + 120.0);
    let cmd_input_left = cmd_x + 46.0;

    if in_command {
        // [:CMD] label (clean text, no background highlight or border)
        painter.text(
            pos2(cmd_x, dock_rect.center().y),
            Align2::LEFT_CENTER,
            ":CMD",
            FontId::monospace(11.0),
            accent,
        );

        let font = FontId::monospace(14.0);
        let cmd_avail_w = (max_cmd_x - cmd_input_left).max(40.0);

        let cur_clamped = cmd_cur.min(cmd_text.len());
        let mut valid_cur = cur_clamped;
        while !cmd_text.is_char_boundary(valid_cur) && valid_cur > 0 {
            valid_cur -= 1;
        }
        let before_cur = &cmd_text[..valid_cur];
        let cursor_offset_x = painter.layout_no_wrap(before_cur.to_string(), font.clone(), theme.text).size().x;
        let total_text_w = painter.layout_no_wrap(cmd_text.to_string(), font.clone(), theme.text).size().x;

        // Auto-scroll offset so cursor is always within view and long commands push left
        let scroll_x = if total_text_w > cmd_avail_w {
            let max_scroll = (total_text_w - cmd_avail_w + 24.0).max(0.0);
            (cursor_offset_x - (cmd_avail_w - 24.0)).clamp(0.0, max_scroll)
        } else {
            0.0
        };

        let cmd_clip_rect = Rect::from_min_max(
            pos2(cmd_input_left, dock_rect.min.y),
            pos2(max_cmd_x, dock_rect.max.y),
        );
        let cmd_painter = painter.with_clip_rect(cmd_clip_rect);
        let text_origin = pos2(cmd_input_left - scroll_x, cmd_y);

        // Draw selection background if text is selected
        if let Some((start, end)) = cmd_selection {
            let s_min = start.min(end).min(cmd_text.len());
            let s_max = start.max(end).min(cmd_text.len());
            let mut valid_min = s_min;
            while !cmd_text.is_char_boundary(valid_min) && valid_min > 0 {
                valid_min -= 1;
            }
            let mut valid_max = s_max;
            while !cmd_text.is_char_boundary(valid_max) && valid_max > 0 {
                valid_max -= 1;
            }
            let prefix = &cmd_text[..valid_min];
            let selected_part = &cmd_text[valid_min..valid_max];
            let x_off = cmd_painter.layout_no_wrap(prefix.to_string(), font.clone(), theme.text).size().x;
            let sel_w = cmd_painter.layout_no_wrap(selected_part.to_string(), font.clone(), theme.text).size().x;
            cmd_painter.rect_filled(
                Rect::from_min_size(pos2(text_origin.x + x_off, text_origin.y), vec2(sel_w, 18.0)),
                2.0,
                Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 90),
            );
        }

        // Draw command line text
        let galley = cmd_painter.layout_no_wrap(cmd_text.to_string(), font.clone(), theme.text);
        cmd_painter.galley(text_origin, galley, theme.text);

        // Draw cursor beam at the exact cmd_cur position
        let cursor_x = text_origin.x + cursor_offset_x;
        let blink = ((now * 2.5).sin() > -0.2) as i32 != 0;
        if blink {
            cmd_painter.rect_filled(
                Rect::from_min_size(pos2(cursor_x, text_origin.y), vec2(2.0, 16.0)),
                1.0,
                accent,
            );
        }
    } else if let Some((symbol, query, match_count)) = search_prompt {
        // [SEARCH] label (clean text, no background highlight or border)
        let badge_label = if symbol == "?" { "? SEARCH" } else { "/ SEARCH" };
        painter.text(
            pos2(cmd_x, dock_rect.center().y),
            Align2::LEFT_CENTER,
            badge_label,
            FontId::monospace(11.0),
            accent,
        );

        let search_left = cmd_x + 64.0;
        let search_clip_rect = Rect::from_min_max(
            pos2(search_left, dock_rect.min.y),
            pos2(max_cmd_x, dock_rect.max.y),
        );
        let search_painter = painter.with_clip_rect(search_clip_rect);
        let query_display = format!("{}{}_", symbol, query);
        search_painter.text(
            pos2(search_left, cmd_y),
            Align2::LEFT_TOP,
            query_display,
            FontId::monospace(14.0),
            theme.text,
        );

        if !query.is_empty() {
            let count_info = if match_count == 0 {
                "(no matches)".to_string()
            } else {
                format!("({} matches)", match_count)
            };
            let query_w = (query.len() + 2) as f32 * 8.5;
            search_painter.text(
                pos2(search_left + 4.0 + query_w, cmd_y + 2.0),
                Align2::LEFT_TOP,
                count_info,
                FontId::monospace(11.0),
                muted,
            );
        }
    } else {
        if let Some(badge) = mode_badge {
            let font = FontId::monospace(11.0);
            let text_color = if badge == "INSERT" {
                accent
            } else if badge.starts_with("VISUAL") {
                Color32::from_rgb(224, 108, 117)
            } else {
                accent
            };

            let layout = painter.layout_no_wrap(badge.to_string(), font.clone(), text_color);
            painter.text(
                pos2(cmd_x, dock_rect.center().y),
                Align2::LEFT_CENTER,
                badge,
                font,
                text_color,
            );
            text_x = cmd_x + layout.size().x + 14.0;
        }

        if !status_msg.is_empty() && (now - status_time) < 3.0 {
            let status_clip = Rect::from_min_max(
                pos2(text_x, dock_rect.min.y),
                pos2(max_cmd_x, dock_rect.max.y),
            );
            let status_painter = painter.with_clip_rect(status_clip);
            status_painter.text(
                pos2(text_x, cmd_y),
                Align2::LEFT_TOP,
                status_msg,
                FontId::monospace(12.0),
                muted,
            );
        }
    }

    // Interactive window resize knob in the bottom-right corner
    let knob_size = 28.0;
    let knob_rect = Rect::from_min_max(
        pos2(dock_rect.max.x - knob_size, dock_rect.max.y - knob_size),
        dock_rect.max,
    );
    let is_knob_hovered = ui.rect_contains_pointer(knob_rect);

    if is_knob_hovered {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeSouthEast);
    }

    if is_knob_hovered && ui.input(|i| i.pointer.primary_down() || i.pointer.primary_pressed()) {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::BeginResize(egui::ResizeDirection::SouthEast));
    }

    let knob_color = if is_knob_hovered {
        accent
    } else {
        theme.muted
    };

    // Tactile diagonal gripper ridges
    for &d in &[5.0, 9.0, 13.0, 17.0] {
        painter.line_segment(
            [
                pos2(dock_rect.max.x - d, dock_rect.max.y - 4.0),
                pos2(dock_rect.max.x - 4.0, dock_rect.max.y - d),
            ],
            Stroke::new(1.4_f32, knob_color),
        );
    }

    // Draw right side stats
    painter.galley(
        pos2(stats_left_x, stats_pos.y),
        stats_galley,
        theme.muted,
    );

    // Empty-space statusbar drag: lets users grab and move the borderless window from the dock
    let is_empty_dock_hovered = ui.rect_contains_pointer(dock_rect) && !is_ai_hovered && !is_knob_hovered && !in_command;
    if is_empty_dock_hovered && ui.input(|i| i.pointer.primary_down()) {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }

    toggle_ai
}
