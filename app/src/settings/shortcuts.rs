//! Keyboard shortcuts reference table tab.

use crate::theme::Theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Pos2, Rect, Stroke};

pub fn render_shortcuts_tab(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    panel_rect: Rect,
    p_origin: Pos2,
    theme: &Theme,
) {
    // ── 1. Header: Title & Search Bar ────────────────────────────────────────
    painter.text(
        p_origin + vec2(0.0, 4.0),
        Align2::LEFT_TOP,
        "KEYBOARD SHORTCUTS",
        FontId::proportional(15.0),
        theme.highlight,
    );

    // Search Bar matching Keybindings tab
    let search_id = egui::Id::new("shortcuts_search_query");
    let mut search_query = ui
        .ctx()
        .data_mut(|d| d.get_temp::<String>(search_id))
        .unwrap_or_default();

    let search_w = 200.0f32.min(panel_rect.width() - 40.0);
    let search_rect = Rect::from_min_size(
        pos2(panel_rect.max.x - search_w - 20.0, p_origin.y),
        vec2(search_w, 28.0),
    );

    let edit_id = search_id.with("text_edit");
    let is_search_focused = ui.memory(|m| m.has_focus(edit_id));
    let is_search_hover = ui.rect_contains_pointer(search_rect);

    let border_stroke = if is_search_focused {
        Stroke::new(1.0, theme.accent)
    } else if is_search_hover {
        Stroke::new(1.0, theme.border().lerp_to_gamma(theme.accent, 0.35))
    } else {
        Stroke::new(1.0, theme.border())
    };

    painter.rect(
        search_rect,
        4.0,
        theme.surface(),
        border_stroke,
        egui::StrokeKind::Inside,
    );
    painter.text(
        pos2(search_rect.min.x + 8.0, search_rect.center().y),
        Align2::LEFT_CENTER,
        "🔍",
        FontId::proportional(11.0),
        theme.muted,
    );

    let right_pad = if search_query.is_empty() { 6.0 } else { 24.0 };
    let text_rect = Rect::from_min_max(
        pos2(search_rect.min.x + 26.0, search_rect.min.y + 4.0),
        pos2(search_rect.max.x - right_pad, search_rect.max.y - 4.0),
    );
    let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(text_rect));
    let edit_resp = child_ui.add(
        egui::TextEdit::singleline(&mut search_query)
            .id(edit_id)
            .hint_text("Search shortcuts...")
            .frame(false)
            .font(FontId::proportional(12.0))
            .text_color(theme.text),
    );
    if edit_resp.changed() {
        ui.ctx().data_mut(|d| d.insert_temp(search_id, search_query.clone()));
    }

    let search_resp = ui.allocate_rect(search_rect, egui::Sense::click());
    if search_resp.clicked() && !is_search_focused {
        ui.memory_mut(|m| m.request_focus(edit_id));
    }

    if !search_query.is_empty() {
        let clear_rect = Rect::from_center_size(
            pos2(search_rect.max.x - 14.0, search_rect.center().y),
            vec2(16.0, 16.0),
        );
        let clear_hover = ui.rect_contains_pointer(clear_rect);
        if clear_hover {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            if ui.input(|i| i.pointer.primary_clicked()) {
                search_query.clear();
                ui.ctx().data_mut(|d| d.insert_temp(search_id, String::new()));
                ui.ctx().request_repaint();
            }
        }
        painter.text(
            clear_rect.center(),
            Align2::CENTER_CENTER,
            "×",
            FontId::proportional(12.0),
            if clear_hover { theme.text } else { theme.muted },
        );
    }

    // ── 2. Table Headers ─────────────────────────────────────────────────────
    let table_header_y = p_origin.y + 34.0;
    let pad_x = 20.0;
    let row_left = panel_rect.min.x + pad_x;
    let row_w = panel_rect.width() - (pad_x * 2.0);

    painter.text(
        pos2(row_left + 8.0, table_header_y + 4.0),
        Align2::LEFT_TOP,
        "COMMAND",
        FontId::proportional(11.0),
        theme.muted,
    );
    painter.text(
        pos2(row_left + row_w - 110.0, table_header_y + 4.0),
        Align2::LEFT_TOP,
        "SHORTCUT",
        FontId::proportional(11.0),
        theme.muted,
    );
    painter.line_segment(
        [
            pos2(row_left, table_header_y + 22.0),
            pos2(row_left + row_w, table_header_y + 22.0),
        ],
        Stroke::new(1.0, theme.border()),
    );

    // ── 3. Groups & Filtered Items ───────────────────────────────────────────
    let groups: &[(&str, &[(&str, &str)])] = &[
        ("DOCUMENT", &[
            ("Ctrl + N",      "Create new note"),
            ("Ctrl + S",      "Save active document"),
            ("Ctrl + R",      "Rename document"),
            ("Ctrl + [ / ]",  "Move text left / right (dedent/indent)"),
            ("Ctrl + D",      "Duplicate line below"),
            ("Ctrl+Shift+X",  "Toggle checklist - [ ] <-> - [x]"),
            ("Ctrl + E",      "Toggle live markdown / raw monospace"),
        ]),
        ("NAVIGATION", &[
            ("Ctrl + P",      "Fuzzy search across notes"),
            ("Ctrl+Shift+P",  "Command Palette (commands & settings via >)"),
            ("Ctrl + B",      "Toggle notes sidebar (open / close)"),
            ("Ctrl + H",      "Focus notes sidebar (left)"),
            ("Ctrl + L / l",  "Focus editor (right)"),
            ("Ctrl + I",      "Toggle backlinks sidebar"),
            ("Ctrl+Shift+O",  "Toggle outline (H1-H6) sidebar"),
            ("Ctrl + .",      "Toggle Zen mode (distraction-free editor)"),
            ("Ctrl + J",      "Toggle interactive terminal dock"),
            ("Ctrl + ,",      "Open preferences & keybindings"),
            ("Esc",           "Dismiss modal / return to Normal mode"),
        ]),
        ("VIM & SHOWCMD HUD", &[
            ("h / j / k / l", "Home-row cursor navigation"),
            ("ci\" / da(",    "Text object editing (tracked in HUD)"),
            ("/pattern",      "Live buffer search (FIND HUD badge)"),
            (":w / :doc",     "Command palette (CMD HUD badge)"),
            ("v / V",         "Visual character / line selection"),
        ]),
        ("TABLES & BLOCKS", &[
            ("Tab",           "Next table cell / auto-append row"),
            ("Shift + Tab",   "Previous table cell / dedent line"),
            ("Enter",         "Add table row / list item continuation"),
            ("Ctrl + Enter",  "Exit code block or markdown table"),
        ]),
        ("SYSTEM", &[
            ("Ctrl+Shift+W",  "Close window"),
            (":help",         "Command bar reference"),
            (":stats",        "Daily writing statistics"),
            ("Preferences",   "Caret styles, blinking & animations (:set)"),
        ]),
    ];

    let query_lower = search_query.trim().to_lowercase();
    let row_h = 36.0;
    let header_h = 24.0;
    let list_top = table_header_y + 26.0;

    // Filter items by search query
    let filtered_groups: Vec<(&str, Vec<(&str, &str)>)> = groups
        .iter()
        .filter_map(|(g_label, items)| {
            let matches: Vec<(&str, &str)> = items
                .iter()
                .filter(|(k, d)| {
                    if query_lower.is_empty() {
                        true
                    } else {
                        k.to_lowercase().contains(&query_lower)
                            || d.to_lowercase().contains(&query_lower)
                            || g_label.to_lowercase().contains(&query_lower)
                    }
                })
                .copied()
                .collect();
            if matches.is_empty() {
                None
            } else {
                Some((*g_label, matches))
            }
        })
        .collect();

    let total_matches: usize = filtered_groups.iter().map(|(_, items)| items.len()).sum();
    let content_h = filtered_groups.len() as f32 * header_h
        + total_matches as f32 * row_h
        + 42.0 // config footer
        + 24.0;

    let visible_h = (panel_rect.max.y - list_top).max(0.0);
    let max_scroll = (content_h - visible_h).max(0.0);

    let scroll_id = egui::Id::new("shortcuts_tab_scroll");
    let mut scroll = ui.ctx().data_mut(|d| d.get_temp::<f32>(scroll_id)).unwrap_or(0.0);

    // Responsive wheel scroll matching keybindings tab
    let pointer_over_panel = ui.rect_contains_pointer(Rect::from_min_max(
        pos2(panel_rect.min.x, list_top),
        panel_rect.max,
    ));
    if pointer_over_panel {
        let wheel = ui.input(|i| i.raw_scroll_delta.y);
        if wheel != 0.0 {
            scroll = (scroll - wheel).clamp(0.0, max_scroll);
            ui.ctx().data_mut(|d| d.insert_temp(scroll_id, scroll));
        }
    }

    let clip_rect = Rect::from_min_max(pos2(panel_rect.min.x, list_top), panel_rect.max);
    let list_painter = painter.with_clip_rect(clip_rect);
    let hover_pos = ui.input(|i| i.pointer.hover_pos()).unwrap_or_default();

    if filtered_groups.is_empty() {
        list_painter.text(
            pos2(panel_rect.center().x, list_top + 40.0),
            Align2::CENTER_CENTER,
            "No shortcuts found matching your search.",
            FontId::proportional(13.0),
            theme.muted,
        );
        return;
    }

    let mut cur_y = list_top - scroll;

    for (g_label, items) in &filtered_groups {
        // Group Header Line
        if cur_y + header_h >= clip_rect.min.y && cur_y <= clip_rect.max.y {
            list_painter.text(
                pos2(row_left + 8.0, cur_y + header_h * 0.5),
                Align2::LEFT_CENTER,
                *g_label,
                FontId::proportional(10.5),
                theme.muted,
            );
            let label_end_x = row_left + 8.0 + (g_label.len() as f32 * 6.5) + 12.0;
            list_painter.line_segment(
                [pos2(label_end_x, cur_y + header_h * 0.5), pos2(row_left + row_w - 8.0, cur_y + header_h * 0.5)],
                Stroke::new(1.0, theme.border()),
            );
        }
        cur_y += header_h;

        for (key, desc) in items {
            let row_rect = Rect::from_min_size(pos2(row_left, cur_y), vec2(row_w, row_h));
            let in_view = cur_y + row_h >= clip_rect.min.y && cur_y <= clip_rect.max.y;

            if in_view {
                // Background-less clean row: no background or border on the row/label
                list_painter.text(
                    pos2(row_rect.min.x + 8.0, row_rect.center().y),
                    Align2::LEFT_CENTER,
                    *desc,
                    FontId::proportional(13.0),
                    theme.text,
                );

                // Keycap badge on the Right: matching Keybindings tab style
                let font_key = FontId::monospace(11.0);
                let text_layout = list_painter.layout_no_wrap(key.to_string(), font_key.clone(), theme.accent);
                let badge_w = (text_layout.size().x + 18.0).max(64.0);
                let badge_h = 24.0;
                let badge_rect = Rect::from_min_size(
                    pos2(row_rect.max.x - badge_w - 8.0, row_rect.center().y - badge_h * 0.5),
                    vec2(badge_w, badge_h),
                );

                let badge_hover = clip_rect.contains(hover_pos) && ui.rect_contains_pointer(badge_rect);

                list_painter.rect(
                    badge_rect,
                    3.0,
                    if badge_hover { theme.surface() } else { theme.bg },
                    Stroke::new(1.0, if badge_hover { theme.accent } else { theme.border() }),
                    egui::StrokeKind::Inside,
                );
                list_painter.text(
                    badge_rect.center(),
                    Align2::CENTER_CENTER,
                    *key,
                    font_key,
                    if badge_hover { theme.highlight } else { theme.accent },
                );
            }

            cur_y += row_h;
        }

        cur_y += 6.0;
    }

    // Keymap file info footer (pointing to keymap.json in settings directory)
    let keymap_path = crate::settings::keymap::VimKeymap::get_keymap_path();
    let footer_rect = Rect::from_min_size(pos2(row_left, cur_y + 8.0), vec2(row_w, 28.0));
    let footer_in_view = footer_rect.max.y >= clip_rect.min.y && footer_rect.min.y <= clip_rect.max.y;

    if footer_in_view {
        list_painter.rect(
            footer_rect,
            4.0,
            theme.surface(),
            Stroke::new(1.0, theme.border()),
            egui::StrokeKind::Inside,
        );
        let path_display = keymap_path.to_string_lossy();
        let short_path = if path_display.len() > 42 {
            format!("...{}", &path_display[path_display.len().saturating_sub(40)..])
        } else {
            path_display.to_string()
        };
        list_painter.text(
            pos2(footer_rect.min.x + 10.0, footer_rect.center().y),
            Align2::LEFT_CENTER,
            format!("⚙ Config: {}", short_path),
            FontId::proportional(11.5),
            theme.muted,
        );
        let open_btn = Rect::from_min_size(pos2(footer_rect.max.x - 96.0, footer_rect.min.y + 4.0), vec2(90.0, 20.0));
        let open_hover = clip_rect.contains(hover_pos) && ui.rect_contains_pointer(open_btn);
        let btn_bg = if open_hover {
            theme.surface().lerp_to_gamma(theme.accent, 0.15)
        } else {
            theme.bg
        };
        list_painter.rect(
            open_btn,
            3.0,
            btn_bg,
            Stroke::new(1.0, if open_hover { theme.accent } else { theme.border() }),
            egui::StrokeKind::Inside,
        );
        list_painter.text(
            open_btn.center(),
            Align2::CENTER_CENTER,
            "Open Folder",
            FontId::proportional(11.0),
            if open_hover { theme.accent } else { theme.text },
        );
        if open_hover && ui.input(|i| i.pointer.primary_clicked()) {
            if let Some(parent) = keymap_path.parent() {
                let _ = std::process::Command::new("explorer").arg(parent).spawn();
            }
        }
    }

    // Sleek custom scrollbar matching Keybindings tab
    if max_scroll > 0.0 {
        let track_x = panel_rect.max.x - 8.0;
        let track = Rect::from_min_max(pos2(track_x, list_top), pos2(track_x + 3.0, panel_rect.max.y - 8.0));
        let track_color = if theme.is_light() {
            Color32::from_rgba_unmultiplied(0, 0, 0, 15)
        } else {
            Color32::from_rgba_unmultiplied(255, 255, 255, 15)
        };
        painter.rect_filled(track, 1.5, track_color);

        let thumb_h = (visible_h * visible_h / content_h).clamp(24.0, visible_h);
        let thumb_y = list_top + (scroll / max_scroll) * (visible_h - thumb_h - 8.0);
        let thumb = Rect::from_min_size(pos2(track_x, thumb_y), vec2(3.0, thumb_h));
        let thumb_color = if theme.is_light() {
            Color32::from_rgba_unmultiplied(0, 0, 0, 60)
        } else {
            Color32::from_rgba_unmultiplied(255, 255, 255, 60)
        };
        painter.rect_filled(thumb, 1.5, thumb_color);
    }
}
