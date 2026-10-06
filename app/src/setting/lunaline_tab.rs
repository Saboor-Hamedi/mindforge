//! LunaLine settings tab — customizable presets, color themes, and component toggles.

use crate::lunaline::{render_lunaline_preview, LunaColorMode, LunaLineConfig, LunaStyle};
use crate::ui::theme::Theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Pos2, Rect, Stroke};

pub fn render_lunaline_tab(
    ui: &egui::Ui,
    painter: &egui::Painter,
    panel_rect: Rect,
    p_origin: Pos2,
    config: &mut LunaLineConfig,
    theme: &Theme,
    opacity: f32,
    on_save_setting: &mut dyn FnMut(&str, &str),
) {
    let available_w = (panel_rect.width() - 56.0).max(300.0);

    // ── Header ──────────────────────────────────────────────────────────────
    painter.text(
        p_origin,
        Align2::LEFT_TOP,
        "LUNALINE STATUSBAR",
        FontId::proportional(14.0),
        theme.highlight,
    );
    painter.text(
        p_origin + vec2(0.0, 20.0),
        Align2::LEFT_TOP,
        "Modular statusline under the editor · Inspired by Neovim lualine",
        FontId::proportional(11.5),
        theme.muted,
    );

    // ── Live Interactive Preview Box ─────────────────────────────────────────
    let preview_y = p_origin.y + 44.0;
    let preview_h = 36.0;
    let preview_rect =
        Rect::from_min_size(pos2(p_origin.x, preview_y), vec2(available_w, preview_h));

    // Render interactive live preview
    render_lunaline_preview(ui, painter, preview_rect, config, theme, opacity);

    // ── 1. Style Presets (4 Chips) ───────────────────────────────────────────
    let style_title_y = preview_y + preview_h + 16.0;
    painter.text(
        pos2(p_origin.x, style_title_y),
        Align2::LEFT_TOP,
        "VISUAL STYLE PRESETS",
        FontId::proportional(11.5),
        theme.highlight,
    );

    let style_start_y = style_title_y + 20.0;
    let col_gap = 10.0;
    let style_col_w = ((available_w - 3.0 * col_gap) / 4.0).floor().max(70.0);
    let style_h = 48.0;

    for (idx, &style) in LunaStyle::ALL.iter().enumerate() {
        let chip_rect = Rect::from_min_size(
            pos2(
                p_origin.x + idx as f32 * (style_col_w + col_gap),
                style_start_y,
            ),
            vec2(style_col_w, style_h),
        );
        let is_selected = config.style == style;
        let hovered = ui.rect_contains_pointer(chip_rect);

        if hovered {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            if ui.input(|i| i.pointer.primary_clicked()) {
                config.style = style;
                if let Ok(json) = serde_json::to_string(config) {
                    on_save_setting("lunaline_config", &json);
                }
            }
        }

        let stroke = if is_selected {
            Stroke::new(1.2, theme.accent)
        } else if hovered {
            Stroke::new(1.0, theme.border().lerp_to_gamma(theme.accent, 0.4))
        } else {
            Stroke::new(1.0, theme.border())
        };

        painter.rect(
            chip_rect,
            6.0,
            if is_selected {
                theme.surface().lerp_to_gamma(theme.accent, 0.08)
            } else {
                theme.surface()
            },
            stroke,
            egui::StrokeKind::Inside,
        );

        // Style name
        painter.text(
            pos2(chip_rect.min.x + 10.0, chip_rect.min.y + 12.0),
            Align2::LEFT_TOP,
            style.name(),
            FontId::monospace(12.0),
            if is_selected {
                theme.accent
            } else {
                theme.text
            },
        );

        // Subtitle
        let short_desc = match style {
            LunaStyle::Pill => "Capsules",
            LunaStyle::Powerline => "Chevrons",
            LunaStyle::Floating => "Island",
            LunaStyle::Minimal => "Typographic",
        };
        painter.text(
            pos2(chip_rect.min.x + 10.0, chip_rect.min.y + 28.0),
            Align2::LEFT_TOP,
            short_desc,
            FontId::proportional(10.0),
            theme.muted,
        );
    }

    // ── 2. Color Theme Selector (3 Chips) ────────────────────────────────────
    let color_title_y = style_start_y + style_h + 16.0;
    painter.text(
        pos2(p_origin.x, color_title_y),
        Align2::LEFT_TOP,
        "COLOR THEME",
        FontId::proportional(11.5),
        theme.highlight,
    );

    let color_start_y = color_title_y + 20.0;
    let color_col_w = ((available_w - 2.0 * col_gap) / 3.0).floor().max(90.0);
    let color_h = 42.0;

    for (idx, &mode) in LunaColorMode::ALL.iter().enumerate() {
        let chip_rect = Rect::from_min_size(
            pos2(
                p_origin.x + idx as f32 * (color_col_w + col_gap),
                color_start_y,
            ),
            vec2(color_col_w, color_h),
        );
        let is_selected = config.color_mode == mode;
        let hovered = ui.rect_contains_pointer(chip_rect);

        if hovered {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            if ui.input(|i| i.pointer.primary_clicked()) {
                config.color_mode = mode;
                if let Ok(json) = serde_json::to_string(config) {
                    on_save_setting("lunaline_config", &json);
                }
            }
        }

        let stroke = if is_selected {
            Stroke::new(1.2, theme.accent)
        } else if hovered {
            Stroke::new(1.0, theme.border().lerp_to_gamma(theme.accent, 0.4))
        } else {
            Stroke::new(1.0, theme.border())
        };

        painter.rect(
            chip_rect,
            6.0,
            if is_selected {
                theme.surface().lerp_to_gamma(theme.accent, 0.08)
            } else {
                theme.surface()
            },
            stroke,
            egui::StrokeKind::Inside,
        );

        // Color dot indicator
        let dot_color = match mode {
            LunaColorMode::Dynamic => Color32::from_rgb(168, 85, 247),
            LunaColorMode::ThemeAccent => theme.accent,
            LunaColorMode::Monochrome => Color32::from_rgb(180, 185, 195),
        };
        painter.circle_filled(
            pos2(chip_rect.min.x + 16.0, chip_rect.center().y),
            4.5,
            dot_color,
        );

        painter.text(
            pos2(chip_rect.min.x + 28.0, chip_rect.center().y),
            Align2::LEFT_CENTER,
            mode.name(),
            FontId::monospace(11.5),
            if is_selected {
                theme.accent
            } else {
                theme.text
            },
        );
    }

    // ── 3. Component Toggles Grid (2 Columns) ────────────────────────────────
    let toggles_title_y = color_start_y + color_h + 16.0;
    painter.text(
        pos2(p_origin.x, toggles_title_y),
        Align2::LEFT_TOP,
        "COMPONENTS & SEGMENTS",
        FontId::proportional(11.5),
        theme.highlight,
    );

    let toggles_start_y = toggles_title_y + 22.0;
    let row_h = 32.0;
    let col_w = (available_w - 20.0) * 0.5;
    let pill_w = 40.0;

    let mut toggle_items = [
        (
            "Mode Badge (NORMAL/INSERT)",
            &mut config.show_mode,
            "lunaline_mode",
        ),
        (
            "Note Info & Dirty Dot",
            &mut config.show_file_info,
            "lunaline_file_info",
        ),
        (
            "Word Count",
            &mut config.show_word_count,
            "lunaline_word_count",
        ),
        (
            "Reading Time Estimate",
            &mut config.show_reading_time,
            "lunaline_reading_time",
        ),
        (
            "Cursor Position (Ln, Col)",
            &mut config.show_cursor_pos,
            "lunaline_cursor_pos",
        ),
        (
            "Document Progress (%)",
            &mut config.show_progress,
            "lunaline_progress",
        ),
        (
            "Character Count",
            &mut config.show_char_count,
            "lunaline_char_count",
        ),
        (
            "File Encoding (UTF-8)",
            &mut config.show_encoding,
            "lunaline_encoding",
        ),
        (
            "AI Agent Pill Button",
            &mut config.show_ai_button,
            "lunaline_ai_button",
        ),
        (
            "Line Ending Format [LF]",
            &mut config.show_line_ending,
            "lunaline_line_ending",
        ),
    ];

    let mut changed = false;

    for (idx, (label, val_ref, id_salt)) in toggle_items.iter_mut().enumerate() {
        let col = idx % 2;
        let row = idx / 2;
        let x = p_origin.x + col as f32 * (col_w + 20.0);
        let y = toggles_start_y + row as f32 * row_h;

        let toggle_pos = pos2(x + col_w - pill_w, y + (row_h - 22.0) * 0.5);

        if crate::ui_components::toggle::render_toggle_with_label(
            ui, painter, toggle_pos, label, val_ref, theme, id_salt,
        ) {
            changed = true;
        }
    }

    if changed {
        if let Ok(json) = serde_json::to_string(config) {
            on_save_setting("lunaline_config", &json);
        }
    }
}
