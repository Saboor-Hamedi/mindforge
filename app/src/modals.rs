use crate::fuzzy::SearchItem;
use crate::theme::Theme;
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

    // Click outside dismisses modal (Mac Spotlight behavior)
    if ui.input(|i| i.pointer.primary_clicked()) {
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
                crate::fuzzy::PaletteAction::OpenThemePicker => {
                    *query = ">theme ".to_string();
                    *selected_idx = 0;
                    action.new_query = Some(">theme ".to_string());
                    action.should_close = false;
                }
                crate::fuzzy::PaletteAction::ApplyTheme(_) => {
                    action.selected_item = Some(item.clone());
                    action.should_close = false; // live change theme without moving away!
                }
                crate::fuzzy::PaletteAction::ShowSoundPicker => {
                    *query = ">sound ".to_string();
                    *selected_idx = 0;
                    action.new_query = Some(">sound ".to_string());
                    action.should_close = false;
                }
                crate::fuzzy::PaletteAction::ApplySoundProfile(_) => {
                    action.selected_item = Some(item.clone());
                    action.should_close = false; // preview sound without closing picker!
                }
                crate::fuzzy::PaletteAction::OpenCaretPicker => {
                    *query = ">caret ".to_string();
                    *selected_idx = 0;
                    action.new_query = Some(">caret ".to_string());
                    action.should_close = false;
                }
                crate::fuzzy::PaletteAction::ApplyCaretKind(_) => {
                    action.selected_item = Some(item.clone());
                    action.should_close = false; // live preview caret without closing picker!
                }
                crate::fuzzy::PaletteAction::OpenFontPicker => {
                    *query = ">font ".to_string();
                    *selected_idx = 0;
                    action.new_query = Some(">font ".to_string());
                    action.should_close = false;
                }
                crate::fuzzy::PaletteAction::ApplyFont(_) => {
                    action.selected_item = Some(item.clone());
                    action.should_close = false; // live apply font without closing picker!
                }
                crate::fuzzy::PaletteAction::OpenModePicker => {
                    *query = ">mode ".to_string();
                    *selected_idx = 0;
                    action.new_query = Some(">mode ".to_string());
                    action.should_close = false;
                }
                crate::fuzzy::PaletteAction::ApplyEditorMode(_) => {
                    action.selected_item = Some(item.clone());
                    action.should_close = false; // switch mode without closing picker!
                }
                _ => {
                    action.selected_item = Some(item.clone());
                    action.should_close = true;
                }
            }
        }
    }
    if nav_esc {
        // If in any sub-picker mode (theme/sound/caret/font/mode/luna), return to > commands
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
        // Search and command results must follow edits to the query field,
        // including deleting `>` to return from commands to note search.
        *selected_idx = 0;
        action.new_query = Some(query.clone());
    }

    // Auto-focus immediately when modal opens and place cursor at the end (e.g. after '>')
    if just_opened || switch_to_cmd {
        response.request_focus();
        let end_idx = query.chars().count();
        let mut state = egui::TextEdit::load_state(ui.ctx(), response.id).unwrap_or_default();
        state.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(end_idx))));
        state.store(ui.ctx(), response.id);
    } else if !response.has_focus() && !ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        response.request_focus();
    }

    // ── Expanded Results (When querying or when notes exist) ──────────────────────────────
    if show_results {
        // Subtle divider separating search input from results
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
                        crate::fuzzy::PaletteAction::OpenThemePicker => {
                            *query = ">theme ".to_string();
                            *selected_idx = 0;
                            action.new_query = Some(">theme ".to_string());
                            action.should_close = false;
                        }
                        crate::fuzzy::PaletteAction::ApplyTheme(_) => {
                            action.selected_item = Some(item.clone());
                            action.should_close = false; // stay in theme picker
                        }
                        crate::fuzzy::PaletteAction::ShowSoundPicker => {
                            *query = ">sound ".to_string();
                            *selected_idx = 0;
                            action.new_query = Some(">sound ".to_string());
                            action.should_close = false;
                        }
                        crate::fuzzy::PaletteAction::ApplySoundProfile(_) => {
                            action.selected_item = Some(item.clone());
                            action.should_close = false; // stay in sound picker, play preview
                        }
                        crate::fuzzy::PaletteAction::OpenCaretPicker => {
                            *query = ">caret ".to_string();
                            *selected_idx = 0;
                            action.new_query = Some(">caret ".to_string());
                            action.should_close = false;
                        }
                        crate::fuzzy::PaletteAction::ApplyCaretKind(_) => {
                            action.selected_item = Some(item.clone());
                            action.should_close = false; // stay in caret picker, preview live
                        }
                        crate::fuzzy::PaletteAction::OpenFontPicker => {
                            *query = ">font ".to_string();
                            *selected_idx = 0;
                            action.new_query = Some(">font ".to_string());
                            action.should_close = false;
                        }
                        crate::fuzzy::PaletteAction::ApplyFont(_) => {
                            action.selected_item = Some(item.clone());
                            action.should_close = false;
                        }
                        crate::fuzzy::PaletteAction::OpenModePicker => {
                            *query = ">mode ".to_string();
                            *selected_idx = 0;
                            action.new_query = Some(">mode ".to_string());
                            action.should_close = false;
                        }
                        crate::fuzzy::PaletteAction::ApplyEditorMode(_) => {
                            action.selected_item = Some(item.clone());
                            action.should_close = false;
                        }
                        _ => {
                            action.selected_item = Some(item.clone());
                            action.should_close = true;
                        }
                    }
                }

                // Left Icon rendered as modern vector graphic
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

                let title_display = if item.title.len() > 42 {
                    format!("{}...", &item.title[..42])
                } else {
                    item.title.clone()
                };

                if !item.snippet.is_empty() && is_selected {
                    painter.text(
                        pos2(title_x, item_rect.min.y + 4.0),
                        Align2::LEFT_TOP,
                        title_display,
                        FontId::monospace(12.5),
                        title_color,
                    );
                    let short_snip = if item.snippet.len() > 50 {
                        format!("{}...", &item.snippet[..50])
                    } else {
                        item.snippet.clone()
                    };
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

                // Sleek borderless text badge on far right (NO background box, NO border!)
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

pub struct RenameModalAction {
    pub confirmed_title: Option<String>,
    pub should_close: bool,
}

pub fn render_rename_modal(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    bounds: Rect,
    input_text: &mut String,
    theme: &Theme,
    just_opened: bool,
) -> RenameModalAction {
    let backdrop_alpha = if theme.is_light() { 90 } else { 160 };
    painter.rect_filled(bounds, 0.0, Color32::from_black_alpha(backdrop_alpha));

    let modal_w = 460.0;
    let modal_h = 160.0;
    let modal_rect = Rect::from_center_size(bounds.center(), vec2(modal_w, modal_h));

    painter.rect(
        modal_rect,
        5.0,
        theme.surface(),
        Stroke::new(1.0_f32, theme.border()),
        egui::StrokeKind::Inside,
    );

    let m_origin = modal_rect.min + vec2(24.0, 20.0);
    painter.text(
        m_origin,
        Align2::LEFT_TOP,
        "Rename Document",
        FontId::monospace(15.0),
        theme.highlight,
    );
    painter.text(
        m_origin + vec2(0.0, 22.0),
        Align2::LEFT_TOP,
        "Enter a new title for this document",
        FontId::monospace(12.0),
        theme.muted,
    );

    let input_rect = Rect::from_min_size(m_origin + vec2(0.0, 48.0), vec2(modal_w - 48.0, 34.0));
    let input_bg = if theme.is_light() {
        Color32::from_rgb(255, 255, 255)
    } else {
        theme.bg
    };
    painter.rect(
        input_rect,
        5.0,
        input_bg,
        Stroke::new(1.0_f32, theme.border()),
        egui::StrokeKind::Inside,
    );

    let edit_rect = input_rect.shrink2(vec2(10.0, 6.0));
    let response = ui.put(
        edit_rect,
        egui::TextEdit::singleline(input_text)
            .font(FontId::monospace(14.0))
            .text_color(theme.text)
            .frame(false),
    );

    if just_opened {
        response.request_focus();
        let mut state = egui::text_edit::TextEditState::load(ui.ctx(), response.id).unwrap_or_default();
        let char_count = input_text.chars().count();
        state.cursor.set_char_range(Some(egui::text::CCursorRange::two(
            egui::text::CCursor::new(0),
            egui::text::CCursor::new(char_count),
        )));
        state.store(ui.ctx(), response.id);
    } else if !response.has_focus() && !ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        response.request_focus();
    }

    let mut action = RenameModalAction {
        confirmed_title: None,
        should_close: false,
    };

    let (enter, esc) = ui.input(|i| (
        i.key_pressed(egui::Key::Enter),
        i.key_pressed(egui::Key::Escape),
    ));

    if enter {
        let trimmed = input_text.trim();
        if !trimmed.is_empty() {
            action.confirmed_title = Some(trimmed.to_string());
        }
        action.should_close = true;
    } else if esc {
        action.should_close = true;
    }

    // Bottom subtle hint line
    painter.text(
        pos2(m_origin.x, modal_rect.max.y - 22.0),
        Align2::LEFT_TOP,
        "Enter to rename  ·  Esc to cancel",
        FontId::monospace(11.0),
        theme.accent,
    );

    action
}

pub struct DeleteModalAction {
    pub confirmed: bool,
    pub should_close: bool,
}

pub fn render_delete_confirm_modal(
    ui: &egui::Ui,
    painter: &egui::Painter,
    bounds: Rect,
    doc_title: &str,
    theme: &Theme,
    just_opened: bool,
) -> DeleteModalAction {
    // Dimmed background overlay
    let backdrop_alpha = if theme.is_light() { 90 } else { 160 };
    painter.rect_filled(bounds, 0.0, Color32::from_black_alpha(backdrop_alpha));

    let modal_w = 460.0;
    let modal_h = 175.0;
    let modal_rect = Rect::from_center_size(bounds.center(), vec2(modal_w, modal_h));

    // Surface container with subtle border
    painter.rect(
        modal_rect,
        5.0,
        theme.surface(),
        Stroke::new(1.0_f32, theme.border()),
        egui::StrokeKind::Inside,
    );

    let m_origin = modal_rect.min + vec2(24.0, 22.0);

    // Red warning pill badge
    let badge_rect = Rect::from_min_size(m_origin, vec2(54.0, 20.0));
    let (badge_bg, badge_text_col) = if theme.is_light() {
        (Color32::from_rgb(254, 226, 226), Color32::from_rgb(185, 28, 28))
    } else {
        (Color32::from_rgb(48, 20, 24), Color32::from_rgb(255, 100, 110))
    };
    painter.rect_filled(badge_rect, 4.0, badge_bg);
    painter.text(
        badge_rect.center(),
        Align2::CENTER_CENTER,
        "DELETE",
        FontId::monospace(10.0),
        badge_text_col,
    );

    // Modal Title
    painter.text(
        m_origin + vec2(64.0, 1.0),
        Align2::LEFT_TOP,
        "Delete Note",
        FontId::monospace(14.5),
        theme.highlight,
    );

    // Truncate note title cleanly if long so it never overflows the container
    let safe_title = if doc_title.trim().is_empty() {
        "Untitled Note".to_string()
    } else if doc_title.chars().count() > 36 {
        let truncated: String = doc_title.chars().take(36).collect();
        format!("{}...", truncated)
    } else {
        doc_title.to_string()
    };

    // Body text - cleanly spaced across dedicated rows
    painter.text(
        m_origin + vec2(0.0, 32.0),
        Align2::LEFT_TOP,
        "Permanently delete this document?",
        FontId::monospace(12.5),
        theme.text,
    );

    let doc_highlight_col = if theme.is_light() {
        Color32::from_rgb(190, 24, 38)
    } else {
        Color32::from_rgb(255, 130, 140)
    };
    painter.text(
        m_origin + vec2(0.0, 52.0),
        Align2::LEFT_TOP,
        format!("\"{}\"", safe_title),
        FontId::monospace(12.0),
        doc_highlight_col,
    );

    painter.text(
        m_origin + vec2(0.0, 72.0),
        Align2::LEFT_TOP,
        "This action cannot be undone.",
        FontId::monospace(11.0),
        theme.muted,
    );

    // Buttons: Cancel (Esc) & Delete (Enter)
    let btn_h = 32.0;
    let btn_y = modal_rect.max.y - btn_h - 18.0;

    let delete_w = 125.0;
    let cancel_w = 110.0;
    let delete_rect = Rect::from_min_size(pos2(modal_rect.max.x - 24.0 - delete_w, btn_y), vec2(delete_w, btn_h));
    let cancel_rect = Rect::from_min_size(pos2(modal_rect.max.x - 24.0 - delete_w - 12.0 - cancel_w, btn_y), vec2(cancel_w, btn_h));

    let cancel_hover = ui.rect_contains_pointer(cancel_rect);
    let delete_hover = ui.rect_contains_pointer(delete_rect);

    let (enter, esc) = if just_opened {
        (false, false)
    } else {
        ui.input(|i| (
            i.key_pressed(egui::Key::Enter),
            i.key_pressed(egui::Key::Escape),
        ))
    };

    let mut action = DeleteModalAction {
        confirmed: false,
        should_close: false,
    };

    // Cancel button
    let (cancel_bg, cancel_stroke, cancel_fg) = if theme.is_light() {
        if cancel_hover {
            (Color32::from_rgb(228, 231, 238), theme.border(), theme.highlight)
        } else {
            (Color32::from_rgb(241, 243, 247), theme.border(), theme.text)
        }
    } else {
        if cancel_hover {
            (Color32::from_rgb(28, 30, 38), Color32::from_gray(80), Color32::WHITE)
        } else {
            (Color32::from_rgb(22, 23, 28), Color32::from_gray(50), Color32::from_gray(180))
        }
    };

    painter.rect(
        cancel_rect,
        5.0,
        cancel_bg,
        Stroke::new(1.0_f32, cancel_stroke),
        egui::StrokeKind::Inside,
    );
    painter.text(
        cancel_rect.center(),
        Align2::CENTER_CENTER,
        "Cancel (Esc)",
        FontId::monospace(11.5),
        cancel_fg,
    );

    // Delete button (Destructive red)
    let (del_bg, del_stroke, del_fg) = if theme.is_light() {
        if delete_hover {
            (Color32::from_rgb(220, 38, 38), Color32::from_rgb(185, 28, 28), Color32::WHITE)
        } else {
            (Color32::from_rgb(239, 68, 68), Color32::from_rgb(220, 38, 38), Color32::WHITE)
        }
    } else {
        if delete_hover {
            (Color32::from_rgb(75, 22, 28), Color32::from_rgb(220, 60, 70), Color32::from_rgb(255, 140, 150))
        } else {
            (Color32::from_rgb(52, 16, 20), Color32::from_rgb(160, 45, 55), Color32::from_rgb(255, 140, 150))
        }
    };

    painter.rect(
        delete_rect,
        5.0,
        del_bg,
        Stroke::new(1.0_f32, del_stroke),
        egui::StrokeKind::Inside,
    );
    painter.text(
        delete_rect.center(),
        Align2::CENTER_CENTER,
        "Delete (Enter)",
        FontId::monospace(11.5),
        del_fg,
    );

    if (delete_hover && ui.input(|i| i.pointer.primary_clicked())) || enter {
        action.confirmed = true;
        action.should_close = true;
    } else if (cancel_hover && ui.input(|i| i.pointer.primary_clicked())) || esc {
        action.should_close = true;
    }

    action
}
