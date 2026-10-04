//! Spotlight-style Search and Command Palette modal dialog.

use crate::services::fuzzy::SearchItem;
use crate::ui::theme::Theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Rect, Stroke};

pub struct SearchModalAction {
    pub selected_item: Option<SearchItem>,
    pub should_close: bool,
    pub new_query: Option<String>,
}

pub fn render_search_modal(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    bounds: Rect,
    query: &mut String,
    results: &[SearchItem],
    selected_idx: &mut usize,
    theme: &Theme,
    just_opened: bool,
    opened_at: f64,
    now: f64,
) -> SearchModalAction {
    let mut action = SearchModalAction {
        selected_item: None,
        should_close: false,
        new_query: None,
    };

    let is_theme_picker = query.starts_with(">theme") || query.starts_with("> theme");
    let is_sound_picker = query.starts_with(">sound") || query.starts_with("> sound");
    let is_caret_picker = query.starts_with(">caret") || query.starts_with("> caret");
    let is_font_picker = query.starts_with(">font") || query.starts_with("> font");
    let is_mode_picker = query.starts_with(">mode") || query.starts_with("> mode");
    let is_luna_picker = query.starts_with(">luna") || query.starts_with("> luna");
    let is_cmd_mode = query.starts_with('>');
    let (icon_str, hint_str) = if is_theme_picker {
        ("🎨", "Search themes (↑↓/Ctrl+J/K to navigate  ·  Enter to apply live)...")
    } else if is_sound_picker {
        ("🔊", "Search sounds (↑↓/Ctrl+J/K to navigate  ·  Enter to preview live)...")
    } else if is_caret_picker {
        ("✦", "Search caret styles (↑↓/Ctrl+J/K to navigate  ·  Enter to apply live)...")
    } else if is_font_picker {
        ("🔤", "Search font families (↑↓/Ctrl+J/K to navigate  ·  Enter to apply live)...")
    } else if is_mode_picker {
        ("⚡", "Switch editor mode (↑↓/Ctrl+J/K to navigate  ·  Enter to apply)...")
    } else if is_luna_picker {
        ("🎨", "Search LunaLine styles (↑↓/Ctrl+J/K to navigate  ·  Enter to apply)...")
    } else if is_cmd_mode {
        ("⚡", "Type a command or setting (↑↓/Ctrl+J/K to navigate  ·  Enter to run)...")
    } else {
        ("🔍", "Search notes or type > for commands (Ctrl+Shift+P)...")
    };

    // Dimmed translucent backdrop
    let backdrop_alpha = if theme.is_light() { 90 } else { 160 };
    painter.rect_filled(bounds, 0.0, Color32::from_black_alpha(backdrop_alpha));

    // Spotlight layout: positioned towards the top (~18% from window top)
    let modal_w = 620.0f32.min(bounds.width() - 32.0);
    let modal_top = bounds.min.y + (bounds.height() * 0.18).clamp(65.0, 140.0);
    let modal_x = bounds.min.x + (bounds.width() - modal_w) * 0.5;

    let is_querying = !query.trim().is_empty();
    let bar_h = 54.0;
    let has_results = !results.is_empty();
    let show_results = is_querying || has_results;
    let visible_items = results.len().min(6);

    let modal_h = if !show_results {
        bar_h
    } else if results.is_empty() {
        bar_h + 1.0 + 52.0 + 32.0 // bar + divider + empty state + footer
    } else {
        bar_h + 1.0 + (visible_items as f32 * 40.0) + 12.0 + 34.0 // bar + divider + items + gap + footer
    };

    let modal_rect = Rect::from_min_size(pos2(modal_x, modal_top), vec2(modal_w, modal_h));

    // Click outside dismisses modal (Mac Spotlight behavior, debounced to ignore opening click)
    let outside_click = !just_opened
        && (now - opened_at) > 0.35
        && ui.input(|i| i.pointer.primary_clicked());
    if outside_click {
        if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
            if !modal_rect.contains(pos) {
                action.should_close = true;
            }
        }
    }

    // Modern macOS Spotlight container: surface matching active theme with smooth rounded corners
    let glass_bg = theme.surface();
    let glass_border = Stroke::new(1.0_f32, theme.border());
    painter.rect(modal_rect, 10.0, glass_bg, glass_border, egui::StrokeKind::Inside);

    // ── Search Bar Input Row ────────────────────────────────────────────────
    let bar_center_y = modal_rect.min.y + bar_h * 0.5;

    // Search icon vertically centered as modern vector graphic
    let search_icon_rect = Rect::from_center_size(pos2(modal_rect.min.x + 24.0, bar_center_y), vec2(16.0, 16.0));
    crate::ui_components::render_vector_icon(painter, icon_str, search_icon_rect, theme.accent);

    // Escape shortcut text on far right of search bar, vertically centered (borderless typography, no background box)
    painter.text(
        pos2(modal_rect.max.x - 24.0, bar_center_y),
        Align2::RIGHT_CENTER,
        "esc",
        FontId::monospace(11.0),
        theme.muted,
    );

    // Keyboard navigation: ArrowUp/Down and vim-style Ctrl+K/J before TextEdit consumes them
    let (nav_up, nav_down, nav_enter, nav_esc, switch_to_cmd, switch_to_notes) = ui.input_mut(|i| {
        let ctrl_shift_p = (i.modifiers.ctrl || i.modifiers.command)
            && i.modifiers.shift
            && i.key_pressed(egui::Key::P);
        let ctrl_p = (i.modifiers.ctrl || i.modifiers.command)
            && !i.modifiers.shift
            && i.key_pressed(egui::Key::P);

        let ctrl_k = i.modifiers.ctrl && !i.modifiers.shift && !i.modifiers.alt
            && i.key_pressed(egui::Key::K);
        let ctrl_j = i.modifiers.ctrl && !i.modifiers.shift && !i.modifiers.alt
            && i.key_pressed(egui::Key::J);
        let up   = i.key_pressed(egui::Key::ArrowUp)   || ctrl_k;
        let down = i.key_pressed(egui::Key::ArrowDown) || ctrl_j;
        let enter = i.key_pressed(egui::Key::Enter);
        let esc   = i.key_pressed(egui::Key::Escape);

        if i.key_pressed(egui::Key::ArrowUp) {
            i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp);
        }
        if i.key_pressed(egui::Key::ArrowDown) {
            i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown);
        }
        if ctrl_k {
            i.consume_key(egui::Modifiers::CTRL, egui::Key::K);
        }
        if ctrl_j {
            i.consume_key(egui::Modifiers::CTRL, egui::Key::J);
        }
        if ctrl_shift_p {
            i.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::P);
            i.consume_key(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::P);
        }
        if ctrl_p {
            i.consume_key(egui::Modifiers::CTRL, egui::Key::P);
            i.consume_key(egui::Modifiers::COMMAND, egui::Key::P);
        }

        (up, down, enter, esc, ctrl_shift_p, ctrl_p)
    });

    if switch_to_cmd {
        *query = ">".to_string();
        *selected_idx = 0;
        action.new_query = Some(">".to_string());
    } else if switch_to_notes && is_cmd_mode {
        *query = String::new();
        *selected_idx = 0;
        action.new_query = Some(String::new());
    }

    if nav_up && !results.is_empty() {
        if *selected_idx > 0 {
            *selected_idx -= 1;
        } else {
            *selected_idx = results.len().saturating_sub(1);
        }
    }
    if nav_down && !results.is_empty() {
        if *selected_idx + 1 < results.len() {
            *selected_idx += 1;
        } else {
            *selected_idx = 0;
        }
    }
    if nav_enter {
        if let Some(item) = results.get(*selected_idx) {
            match &item.action {
                crate::services::fuzzy::PaletteAction::OpenThemePicker => {
                    *query = ">theme ".to_string();
                    *selected_idx = 0;
                    action.new_query = Some(">theme ".to_string());
                    action.should_close = false;
                }
                crate::services::fuzzy::PaletteAction::ApplyTheme(_) => {
                    action.selected_item = Some(item.clone());
                    action.should_close = false;
                }
                crate::services::fuzzy::PaletteAction::ShowSoundPicker => {
                    *query = ">sound ".to_string();
                    *selected_idx = 0;
                    action.new_query = Some(">sound ".to_string());
                    action.should_close = false;
                }
                crate::services::fuzzy::PaletteAction::ApplySoundProfile(_) => {
                    action.selected_item = Some(item.clone());
                    action.should_close = false;
                }
                crate::services::fuzzy::PaletteAction::OpenCaretPicker => {
                    *query = ">caret ".to_string();
                    *selected_idx = 0;
                    action.new_query = Some(">caret ".to_string());
                    action.should_close = false;
                }
                crate::services::fuzzy::PaletteAction::ApplyCaretKind(_) => {
                    action.selected_item = Some(item.clone());
                    action.should_close = false;
                }
                crate::services::fuzzy::PaletteAction::OpenFontPicker => {
                    *query = ">font ".to_string();
                    *selected_idx = 0;
                    action.new_query = Some(">font ".to_string());
                    action.should_close = false;
                }
                crate::services::fuzzy::PaletteAction::ApplyFont(_) => {
                    action.selected_item = Some(item.clone());
                    action.should_close = false;
                }
                crate::services::fuzzy::PaletteAction::OpenModePicker => {
                    *query = ">mode ".to_string();
                    *selected_idx = 0;
                    action.new_query = Some(">mode ".to_string());
                    action.should_close = false;
                }
                crate::services::fuzzy::PaletteAction::ApplyEditorMode(_) => {
                    action.selected_item = Some(item.clone());
                    action.should_close = false;
                }
                _ => {
                    action.selected_item = Some(item.clone());
                    action.should_close = true;
                }
            }
        }
    }
    if nav_esc {
        if is_theme_picker || is_sound_picker || is_caret_picker || is_font_picker || is_mode_picker || is_luna_picker {
            *query = ">".to_string();
            *selected_idx = 0;
            action.new_query = Some(">".to_string());
            action.should_close = false;
        } else {
            action.should_close = true;
        }
    }

    // Single-line text input vertically aligned with the search icon
    let input_h = 24.0;
    let edit_rect = Rect::from_min_size(
        pos2(modal_rect.min.x + 48.0, bar_center_y - input_h * 0.5),
        vec2(modal_w - 48.0 - 54.0, input_h),
    );

    let response = ui.put(
        edit_rect,
        egui::TextEdit::singleline(query)
            .font(FontId::monospace(14.0))
            .text_color(theme.text)
            .hint_text(hint_str)
            .margin(vec2(0.0, 2.0))
            .frame(false),
    );
    if response.changed() {
        *selected_idx = 0;
        action.new_query = Some(query.clone());
    }

    if just_opened || switch_to_cmd {
        response.request_focus();
        let end_idx = query.chars().count();
        let mut state = egui::TextEdit::load_state(ui.ctx(), response.id).unwrap_or_default();
        state.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(end_idx))));
        state.store(ui.ctx(), response.id);
    } else if !response.has_focus() && !ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        response.request_focus();
    }

    // ── Expanded Results ──────────────────────────────────────────────────────────
    if show_results {
        let div_y = modal_rect.min.y + bar_h;
        painter.line_segment(
            [pos2(modal_rect.min.x, div_y), pos2(modal_rect.max.x, div_y)],
            Stroke::new(1.0_f32, theme.border()),
        );

        let results_y = div_y + 8.0;

        if results.is_empty() {
            painter.text(
                pos2(modal_rect.center().x, results_y + 20.0),
                Align2::CENTER_CENTER,
                format!("No matching notes found for \"{}\"", query.trim()),
                FontId::monospace(12.5),
                theme.muted,
            );
        } else {
            let window_start = if *selected_idx >= visible_items {
                *selected_idx + 1 - visible_items
            } else {
                0
            };
            let window_end = (window_start + visible_items).min(results.len());

            for (render_idx, actual_idx) in (window_start..window_end).enumerate() {
                let item = &results[actual_idx];
                let item_rect = Rect::from_min_size(
                    pos2(modal_rect.min.x + 8.0, results_y + render_idx as f32 * 40.0),
                    vec2(modal_w - 16.0, 36.0),
                );
                let is_selected = actual_idx == *selected_idx;
                let is_hovered = ui.rect_contains_pointer(item_rect);

                if is_selected || is_hovered {
                    let sel_bg = if is_selected {
                        if theme.is_light() {
                            Color32::from_rgba_unmultiplied(theme.accent.r(), theme.accent.g(), theme.accent.b(), 14)
                        } else {
                            Color32::from_rgba_unmultiplied(theme.accent.r(), theme.accent.g(), theme.accent.b(), 20)
                        }
                    } else if theme.is_light() {
                        Color32::from_rgba_unmultiplied(0, 0, 0, 8)
                    } else {
                        Color32::from_rgba_unmultiplied(255, 255, 255, 8)
                    };
                    painter.rect_filled(item_rect, 6.0, sel_bg);
                }

                if is_hovered && ui.input(|i| i.pointer.primary_clicked()) {
                    *selected_idx = actual_idx;
                    match &item.action {
                        crate::services::fuzzy::PaletteAction::OpenThemePicker => {
                            *query = ">theme ".to_string();
                            *selected_idx = 0;
                            action.new_query = Some(">theme ".to_string());
                            action.should_close = false;
                        }
                        crate::services::fuzzy::PaletteAction::ApplyTheme(_) => {
                            action.selected_item = Some(item.clone());
                            action.should_close = false;
                        }
                        crate::services::fuzzy::PaletteAction::ShowSoundPicker => {
                            *query = ">sound ".to_string();
                            *selected_idx = 0;
                            action.new_query = Some(">sound ".to_string());
                            action.should_close = false;
                        }
                        crate::services::fuzzy::PaletteAction::ApplySoundProfile(_) => {
                            action.selected_item = Some(item.clone());
                            action.should_close = false;
                        }
                        crate::services::fuzzy::PaletteAction::OpenCaretPicker => {
                            *query = ">caret ".to_string();
                            *selected_idx = 0;
                            action.new_query = Some(">caret ".to_string());
                            action.should_close = false;
                        }
                        crate::services::fuzzy::PaletteAction::ApplyCaretKind(_) => {
                            action.selected_item = Some(item.clone());
                            action.should_close = false;
                        }
                        crate::services::fuzzy::PaletteAction::OpenFontPicker => {
                            *query = ">font ".to_string();
                            *selected_idx = 0;
                            action.new_query = Some(">font ".to_string());
                            action.should_close = false;
                        }
                        crate::services::fuzzy::PaletteAction::ApplyFont(_) => {
                            action.selected_item = Some(item.clone());
                            action.should_close = false;
                        }
                        crate::services::fuzzy::PaletteAction::OpenModePicker => {
                            *query = ">mode ".to_string();
                            *selected_idx = 0;
                            action.new_query = Some(">mode ".to_string());
                            action.should_close = false;
                        }
                        crate::services::fuzzy::PaletteAction::ApplyEditorMode(_) => {
                            action.selected_item = Some(item.clone());
                            action.should_close = false;
                        }
                        _ => {
                            action.selected_item = Some(item.clone());
                            action.should_close = true;
                        }
                    }
                }

                // Left Icon
                let icon_rect = Rect::from_center_size(
                    pos2(item_rect.min.x + 22.0, item_rect.center().y),
                    vec2(14.0, 14.0),
                );
                crate::ui_components::render_vector_icon(
                    painter,
                    item.icon,
                    icon_rect,
                    if is_selected { theme.accent } else { theme.muted },
                );

                // Note / Command Title & Snippet
                let title_x = item_rect.min.x + 42.0;
                let title_color = if is_selected {
                    theme.text
                } else {
                    theme.text.lerp_to_gamma(theme.muted, 0.15)
                };

                let title_display = crate::ui::truncate_with_ellipsis(&item.title, 45);

                if !item.snippet.is_empty() && is_selected {
                    painter.text(
                        pos2(title_x, item_rect.min.y + 4.0),
                        Align2::LEFT_TOP,
                        title_display,
                        FontId::monospace(12.5),
                        title_color,
                    );
                    let short_snip = crate::ui::truncate_with_ellipsis(&item.snippet, 53);
                    painter.text(
                        pos2(title_x, item_rect.min.y + 19.0),
                        Align2::LEFT_TOP,
                        short_snip,
                        FontId::monospace(10.0),
                        theme.muted,
                    );
                } else {
                    painter.text(
                        pos2(title_x, item_rect.center().y),
                        Align2::LEFT_CENTER,
                        title_display,
                        FontId::monospace(12.5),
                        title_color,
                    );
                }

                if !item.badge.is_empty() {
                    let badge_color = if is_selected {
                        theme.accent
                    } else if item.badge.contains("Active") {
                        theme.accent
                    } else {
                        theme.muted
                    };
                    painter.text(
                        pos2(item_rect.max.x - 16.0, item_rect.center().y),
                        Align2::RIGHT_CENTER,
                        &item.badge,
                        FontId::monospace(11.0),
                        badge_color,
                    );
                }
            }
        }

        // Minimalist footer bar
        let footer_y = modal_rect.max.y - 28.0;
        painter.line_segment(
            [pos2(modal_rect.min.x + 16.0, footer_y - 4.0), pos2(modal_rect.max.x - 16.0, footer_y - 4.0)],
            Stroke::new(1.0_f32, theme.border()),
        );
        let footer_hint = if is_theme_picker {
            "↑↓ / Ctrl+J/K  ·  ↵ Apply Theme Live  ·  esc → Commands"
        } else if is_sound_picker {
            "↑↓ / Ctrl+J/K  ·  ↵ Preview Sound Live  ·  esc → Commands"
        } else if is_caret_picker {
            "↑↓ / Ctrl+J/K  ·  ↵ Apply Caret Live  ·  esc → Commands"
        } else if is_font_picker {
            "↑↓ / Ctrl+J/K  ·  ↵ Apply Font Live  ·  esc → Commands"
        } else if is_mode_picker {
            "↑↓ / Ctrl+J/K  ·  ↵ Switch Editor Mode  ·  esc → Commands"
        } else if is_luna_picker {
            "↑↓ / Ctrl+J/K  ·  ↵ Apply LunaLine Style  ·  esc → Commands"
        } else if is_cmd_mode {
            "↑↓ / Ctrl+J/K  ·  ↵ Run Command  ·  Ctrl+P → Notes  ·  esc Close"
        } else {
            "↑↓ / Ctrl+J/K  ·  ↵ Open Note  ·  type > or Ctrl+Shift+P for Commands  ·  esc Close"
        };
        painter.text(
            pos2(modal_rect.min.x + 18.0, footer_y + 1.0),
            Align2::LEFT_TOP,
            footer_hint,
            FontId::monospace(10.5),
            theme.muted,
        );
    }

    action
}
