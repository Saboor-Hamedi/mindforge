//! VSCode-style dynamic keybinding configuration settings tab.

use crate::settings::keymap::{
    key_stroke_display, KeyStroke, KeybindCapture, KeymapMode, VimKeymap,
};
use crate::settings::types::{InsertPosition, VimAction, VimMotion, VimOperator};
use crate::ui::theme::Theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Id, Key, Pos2, Rect, Sense, Stroke};

pub fn render_keybindings_tab(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    panel_rect: Rect,
    p_origin: Pos2,
    theme: &Theme,
    keymap: &mut VimKeymap,
    capture: &mut Option<KeybindCapture>,
) {
    // ── 1. Escape Cancellation: Consumed so settings modal stays open ────────
    if capture.is_some() {
        let esc = ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
        if esc {
            *capture = None;
            return;
        }
    }

    // ── 2. Enter Confirmation: Commits staged keybinding ─────────────────────
    if let Some(cap) = capture.as_mut() {
        let enter = ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
        if enter {
            if let Some(new_stroke) = cap.staged_stroke.take() {
                keymap.rebind(
                    cap.mode,
                    cap.old_stroke.as_ref(),
                    new_stroke,
                    cap.action.clone(),
                );
            }
            *capture = None;
            return;
        }
    }

    // ── 3. Real Keystroke Capture (ignores lone Escape and Enter) ─────────────
    if let Some(cap) = capture.as_mut() {
        let pressed = ui.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } if *key != Key::Escape && *key != Key::Enter => {
                    Some(KeyStroke::from_key(*key, *modifiers))
                }
                _ => None,
            })
        });
        if let Some(stroke) = pressed {
            cap.staged_stroke = Some(stroke);
        }
    }

    // ── 4. Header: Title & "Reset All Defaults" Button ───────────────────────
    painter.text(
        p_origin + vec2(0.0, 4.0),
        Align2::LEFT_TOP,
        "KEYBINDINGS",
        FontId::proportional(15.0),
        theme.highlight,
    );

    // ── 0. Calculate Recorder Modal Geometry Upfront ────────────────────────
    let modal_w = 460.0f32.min(panel_rect.width() - 40.0);
    let modal_h = 215.0;
    let modal_rect = Rect::from_center_size(panel_rect.center(), vec2(modal_w, modal_h));
    let pointer_in_modal = capture.is_some() && ui.rect_contains_pointer(modal_rect);

    // Global Reset Button
    let reset_all_w = 165.0;
    let reset_all_h = 28.0;
    let reset_all_rect = Rect::from_min_size(
        pos2(panel_rect.max.x - reset_all_w - 20.0, p_origin.y),
        vec2(reset_all_w, reset_all_h),
    );
    let reset_all_id = Id::new("keymap_reset_all_defaults");
    let reset_all_resp = ui.interact(reset_all_rect, reset_all_id, Sense::click());
    let reset_all_hov = reset_all_resp.hovered() && !pointer_in_modal;

    painter.rect(
        reset_all_rect,
        4.0,
        if reset_all_hov {
            theme.surface()
        } else {
            theme.bg
        },
        Stroke::new(
            1.0_f32,
            if reset_all_hov {
                theme.accent
            } else {
                theme.border()
            },
        ),
        egui::StrokeKind::Inside,
    );
    painter.text(
        pos2(reset_all_rect.min.x + 12.0, reset_all_rect.center().y),
        Align2::LEFT_CENTER,
        "↺",
        FontId::monospace(13.0),
        if reset_all_hov {
            theme.highlight
        } else {
            theme.muted
        },
    );
    painter.text(
        pos2(reset_all_rect.min.x + 28.0, reset_all_rect.center().y),
        Align2::LEFT_CENTER,
        "Reset All Defaults",
        FontId::proportional(11.5),
        if reset_all_hov {
            theme.highlight
        } else {
            theme.muted
        },
    );
    if reset_all_resp.clicked() && !pointer_in_modal {
        *keymap = VimKeymap::new_standard();
        let _ = keymap.save_to_file();
        *capture = None;
    }

    // ── 5. Mode Selector: Normal Mode vs Visual Mode ─────────────────────────
    let mode_id = Id::new("keybindings_tab_mode");
    let mut active_mode: KeymapMode = ui
        .ctx()
        .data_mut(|d| d.get_temp(mode_id))
        .unwrap_or(KeymapMode::Normal);

    let mode_toggle_y = p_origin.y + 34.0;
    let normal_btn_rect = Rect::from_min_size(
        pos2(panel_rect.min.x + 20.0, mode_toggle_y),
        vec2(110.0, 26.0),
    );
    let visual_btn_rect = Rect::from_min_size(
        pos2(normal_btn_rect.max.x + 8.0, mode_toggle_y),
        vec2(110.0, 26.0),
    );

    let normal_resp = ui.interact(
        normal_btn_rect,
        Id::new("keymap_mode_normal"),
        Sense::click(),
    );
    if normal_resp.clicked() && !pointer_in_modal {
        active_mode = KeymapMode::Normal;
        ui.ctx().data_mut(|d| d.insert_temp(mode_id, active_mode));
        *capture = None;
    }
    let is_normal = active_mode == KeymapMode::Normal;
    let normal_bg = if normal_resp.hovered() && !pointer_in_modal {
        theme.surface()
    } else {
        theme.bg
    };
    painter.rect(
        normal_btn_rect,
        4.0,
        normal_bg,
        Stroke::new(
            1.0_f32,
            if is_normal {
                theme.accent
            } else {
                theme.border()
            },
        ),
        egui::StrokeKind::Inside,
    );
    painter.text(
        normal_btn_rect.center(),
        Align2::CENTER_CENTER,
        "Normal Mode",
        FontId::proportional(12.0),
        if is_normal { theme.accent } else { theme.muted },
    );

    let visual_resp = ui.interact(
        visual_btn_rect,
        Id::new("keymap_mode_visual"),
        Sense::click(),
    );
    if visual_resp.clicked() && !pointer_in_modal {
        active_mode = KeymapMode::Visual;
        ui.ctx().data_mut(|d| d.insert_temp(mode_id, active_mode));
        *capture = None;
    }
    let is_visual = active_mode == KeymapMode::Visual;
    let visual_bg = if visual_resp.hovered() && !pointer_in_modal {
        theme.surface()
    } else {
        theme.bg
    };
    painter.rect(
        visual_btn_rect,
        4.0,
        visual_bg,
        Stroke::new(
            1.0_f32,
            if is_visual {
                theme.accent
            } else {
                theme.border()
            },
        ),
        egui::StrokeKind::Inside,
    );
    painter.text(
        visual_btn_rect.center(),
        Align2::CENTER_CENTER,
        "Visual Mode",
        FontId::proportional(12.0),
        if is_visual { theme.accent } else { theme.muted },
    );

    // ── 6. Search Bar ────────────────────────────────────────────────────────
    let search_id = Id::new("keybindings_search_query");
    let mut search_query = ui
        .ctx()
        .data_mut(|d| d.get_temp::<String>(search_id))
        .unwrap_or_default();

    let search_x = visual_btn_rect.max.x + 16.0;
    let search_w = (panel_rect.max.x - 20.0 - search_x).max(140.0);
    let search_rect = Rect::from_min_size(pos2(search_x, mode_toggle_y), vec2(search_w, 26.0));

    painter.rect(
        search_rect,
        4.0,
        theme.surface(),
        Stroke::new(1.0_f32, theme.border()),
        egui::StrokeKind::Inside,
    );
    painter.text(
        pos2(search_rect.min.x + 8.0, search_rect.center().y),
        Align2::LEFT_CENTER,
        "🔍",
        FontId::proportional(11.0),
        theme.muted,
    );

    let text_rect = Rect::from_min_max(
        pos2(search_rect.min.x + 26.0, search_rect.min.y),
        pos2(search_rect.max.x - 6.0, search_rect.max.y),
    );
    let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(text_rect));
    let edit_resp = child_ui.add(
        egui::TextEdit::singleline(&mut search_query)
            .hint_text("Search shortcuts...")
            .frame(false)
            .font(FontId::proportional(12.0))
            .text_color(theme.text),
    );
    if edit_resp.changed() {
        ui.ctx()
            .data_mut(|d| d.insert_temp(search_id, search_query.clone()));
    }

    // ── 7. Snapshot Current Keymap Entries ───────────────────────────────────
    let current_entries: Vec<(KeyStroke, VimAction)> = match active_mode {
        KeymapMode::Normal => keymap
            .normal
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
        KeymapMode::Visual => keymap
            .visual
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
    };

    let mut all_actions = canonical_actions_for_mode(active_mode);
    for (_, act) in &current_entries {
        if !all_actions.contains(act) {
            all_actions.push(act.clone());
        }
    }

    // Filter by search query
    let query_lower = search_query.trim().to_lowercase();
    let filtered_actions: Vec<VimAction> = all_actions
        .into_iter()
        .filter(|act| {
            if query_lower.is_empty() {
                return true;
            }
            let name = action_display_name(act).to_lowercase();
            if name.contains(&query_lower) {
                return true;
            }
            for (k, a) in &current_entries {
                if a == act {
                    let k_disp = key_stroke_display(k).to_lowercase();
                    if k_disp.contains(&query_lower) {
                        return true;
                    }
                }
            }
            false
        })
        .collect();

    // ── 8. Table Headers ─────────────────────────────────────────────────────
    let table_header_y = mode_toggle_y + 34.0;
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
        pos2(row_left + row_w - 280.0, table_header_y + 4.0),
        Align2::LEFT_TOP,
        "KEYBINDINGS",
        FontId::proportional(11.0),
        theme.muted,
    );
    painter.line_segment(
        [
            pos2(row_left, table_header_y + 22.0),
            pos2(row_left + row_w, table_header_y + 22.0),
        ],
        Stroke::new(1.0_f32, theme.border()),
    );

    // ── 9. Scrollable List of Actions ────────────────────────────────────────
    let list_top = table_header_y + 26.0;
    let row_h = 40.0;
    let content_h = filtered_actions.len() as f32 * row_h + 30.0;
    let visible_h = (panel_rect.max.y - list_top).max(0.0);
    let max_scroll = (content_h - visible_h).max(0.0);

    let scroll_id = Id::new(("keybindings_tab_scroll", active_mode));
    let mut scroll = ui
        .ctx()
        .data_mut(|d| d.get_temp::<f32>(scroll_id))
        .unwrap_or(0.0);

    let pointer_over_list = ui.rect_contains_pointer(Rect::from_min_max(
        pos2(panel_rect.min.x, list_top),
        panel_rect.max,
    ));
    if pointer_over_list && !pointer_in_modal {
        let wheel = ui.input(|i| i.raw_scroll_delta.y);
        if wheel != 0.0 {
            scroll = (scroll - wheel).clamp(0.0, max_scroll);
            ui.ctx().data_mut(|d| d.insert_temp(scroll_id, scroll));
        }
    }

    let clip_rect = Rect::from_min_max(pos2(panel_rect.min.x, list_top), panel_rect.max);
    let row_painter = painter.with_clip_rect(clip_rect);

    if filtered_actions.is_empty() {
        row_painter.text(
            pos2(panel_rect.center().x, list_top + 40.0),
            Align2::CENTER_CENTER,
            "No keybindings found matching your search.",
            FontId::proportional(13.0),
            theme.muted,
        );
        return;
    }

    let mut stroke_to_unbind: Option<KeyStroke> = None;
    let mut action_to_reset_default: Option<VimAction> = None;
    let mut next_capture: Option<Option<KeybindCapture>> = None;
    let mut stroke_to_commit_before_switch: Option<(
        KeymapMode,
        Option<KeyStroke>,
        KeyStroke,
        VimAction,
    )> = None;

    for (idx, action) in filtered_actions.iter().enumerate() {
        let y = list_top - scroll + idx as f32 * row_h;
        if y + row_h < list_top || y > panel_rect.max.y {
            continue;
        }

        let is_capturing_this = capture
            .as_ref()
            .map(|c| c.mode == active_mode && c.action == *action)
            .unwrap_or(false);

        let row_rect = Rect::from_min_size(pos2(row_left, y), vec2(row_w, row_h - 4.0));

        if is_capturing_this {
            let row_bg = Color32::from_rgba_unmultiplied(
                theme.accent.r(),
                theme.accent.g(),
                theme.accent.b(),
                26,
            );
            row_painter.rect_filled(row_rect, 4.0, row_bg);
        }

        // Command Name (Left column)
        let action_name = action_display_name(action);
        row_painter.text(
            pos2(row_rect.min.x + 12.0, row_rect.center().y),
            Align2::LEFT_CENTER,
            action_name,
            FontId::proportional(13.0),
            if is_capturing_this {
                theme.highlight
            } else {
                theme.text
            },
        );

        // Right side: Individual Key Slots + Add Button + Individual Reset Button
        let mut bound_strokes: Vec<KeyStroke> = current_entries
            .iter()
            .filter(|(_, act)| act == action)
            .map(|(k, _)| k.clone())
            .collect();
        bound_strokes.sort_by(|a, b| key_stroke_display(a).cmp(&key_stroke_display(b)));

        let mut right_x = row_rect.max.x - 12.0;

        // 1. [↺ Reset] Button (shown only when this action differs from default)
        let is_modified = is_action_modified(active_mode, action, &bound_strokes);
        if is_modified {
            let reset_w = 26.0;
            let reset_rect = Rect::from_min_size(
                pos2(right_x - reset_w, row_rect.center().y - 12.0),
                vec2(reset_w, 24.0),
            );
            let reset_id = Id::new(("btn_reset_indiv", idx));
            let reset_resp = ui.interact(reset_rect, reset_id, Sense::click());
            let reset_hov = reset_resp.hovered() && !pointer_in_modal;

            row_painter.rect(
                reset_rect,
                3.0,
                if reset_hov {
                    theme.surface()
                } else {
                    Color32::TRANSPARENT
                },
                Stroke::new(
                    1.0_f32,
                    if reset_hov {
                        theme.accent
                    } else {
                        theme.border()
                    },
                ),
                egui::StrokeKind::Inside,
            );
            row_painter.text(
                reset_rect.center(),
                Align2::CENTER_CENTER,
                "↺",
                FontId::proportional(12.5),
                if reset_hov {
                    theme.highlight
                } else {
                    theme.muted
                },
            );
            if reset_resp.clicked() && !pointer_in_modal {
                action_to_reset_default = Some(action.clone());
            }
            right_x -= reset_w + 6.0;
        }

        // 2. [+] Add Keybinding Button
        let add_w = 24.0;
        let add_rect = Rect::from_min_size(
            pos2(right_x - add_w, row_rect.center().y - 12.0),
            vec2(add_w, 24.0),
        );
        let add_id = Id::new(("btn_add_key", idx));
        let add_resp = ui.interact(add_rect, add_id, Sense::click());
        let add_hov = add_resp.hovered() && !pointer_in_modal;

        row_painter.rect(
            add_rect,
            3.0,
            if add_hov {
                theme.surface()
            } else {
                Color32::TRANSPARENT
            },
            Stroke::new(
                1.0_f32,
                if add_hov {
                    theme.accent
                } else {
                    theme.border()
                },
            ),
            egui::StrokeKind::Inside,
        );
        row_painter.text(
            add_rect.center(),
            Align2::CENTER_CENTER,
            "+",
            FontId::monospace(13.0),
            if add_hov {
                theme.highlight
            } else {
                theme.muted
            },
        );
        if add_resp.clicked() && !pointer_in_modal {
            if let Some(ref cap) = capture.as_ref() {
                if let Some(ref staged) = cap.staged_stroke {
                    stroke_to_commit_before_switch = Some((
                        cap.mode,
                        cap.old_stroke.clone(),
                        staged.clone(),
                        cap.action.clone(),
                    ));
                }
            }
            next_capture = Some(Some(KeybindCapture {
                mode: active_mode,
                action: action.clone(),
                action_label: action_name.to_string(),
                old_stroke: None,
                staged_stroke: None,
            }));
        }
        right_x -= add_w + 6.0;

        // 3. Render each key as its OWN clean individual slot
        if bound_strokes.is_empty() {
            let unbound_w = 64.0;
            let unbound_rect = Rect::from_min_size(
                pos2(right_x - unbound_w, row_rect.center().y - 12.0),
                vec2(unbound_w, 24.0),
            );
            row_painter.text(
                unbound_rect.center(),
                Align2::CENTER_CENTER,
                "None",
                FontId::monospace(11.0),
                theme.muted,
            );
        } else {
            // Render individual pills in reverse so they line up right-to-left
            for stroke in bound_strokes.iter().rev() {
                let disp = key_stroke_display(stroke);
                let text_w = (disp.len() as f32 * 7.5).max(18.0);
                let pill_w = text_w + 24.0; // key text + delete '✕' button
                let pill_rect = Rect::from_min_size(
                    pos2(right_x - pill_w, row_rect.center().y - 12.0),
                    vec2(pill_w, 24.0),
                );

                let is_capturing_this_stroke = capture
                    .as_ref()
                    .map(|c| {
                        c.mode == active_mode
                            && c.action == *action
                            && c.old_stroke.as_ref() == Some(stroke)
                    })
                    .unwrap_or(false);

                let slot_id = Id::new(("key_slot", active_mode, &disp));
                let slot_resp = ui.interact(pill_rect, slot_id, Sense::click());
                let slot_hov = slot_resp.hovered() && !pointer_in_modal;

                row_painter.rect(
                    pill_rect,
                    3.0,
                    if is_capturing_this_stroke {
                        Color32::from_rgba_unmultiplied(
                            theme.accent.r(),
                            theme.accent.g(),
                            theme.accent.b(),
                            35,
                        )
                    } else if slot_hov {
                        theme.surface()
                    } else {
                        theme.bg
                    },
                    Stroke::new(
                        1.0_f32,
                        if is_capturing_this_stroke || slot_hov {
                            theme.accent
                        } else {
                            theme.border()
                        },
                    ),
                    egui::StrokeKind::Inside,
                );

                row_painter.text(
                    pos2(pill_rect.min.x + 6.0, pill_rect.center().y),
                    Align2::LEFT_CENTER,
                    &disp,
                    FontId::monospace(11.0),
                    if is_capturing_this_stroke {
                        theme.highlight
                    } else {
                        theme.text
                    },
                );

                // Clicking the key text area edits this specific key
                if slot_resp.clicked() && !pointer_in_modal {
                    if let Some(ref cap) = capture.as_ref() {
                        if let Some(ref staged) = cap.staged_stroke {
                            stroke_to_commit_before_switch = Some((
                                cap.mode,
                                cap.old_stroke.clone(),
                                staged.clone(),
                                cap.action.clone(),
                            ));
                        }
                    }
                    next_capture = Some(Some(KeybindCapture {
                        mode: active_mode,
                        action: action.clone(),
                        action_label: action_name.to_string(),
                        old_stroke: Some(stroke.clone()),
                        staged_stroke: None,
                    }));
                }

                // Small delete '✕' icon inside the pill
                let del_rect = Rect::from_min_size(
                    pos2(pill_rect.max.x - 16.0, pill_rect.min.y),
                    vec2(16.0, 24.0),
                );
                let del_id = Id::new(("btn_del_key", active_mode, &disp));
                let del_resp = ui.interact(del_rect, del_id, Sense::click());
                let del_hov = del_resp.hovered() && !pointer_in_modal;

                row_painter.text(
                    del_rect.center(),
                    Align2::CENTER_CENTER,
                    "✕",
                    FontId::monospace(9.5),
                    if del_hov {
                        Color32::from_rgb(220, 70, 70)
                    } else {
                        theme.muted
                    },
                );
                if del_resp.clicked() && !pointer_in_modal {
                    stroke_to_unbind = Some(stroke.clone());
                }

                right_x -= pill_w + 4.0;
            }
        }
    }

    // ── 10. Process Unbind, Individual Reset, and Seamless Switch Actions ────
    if let Some(stroke) = stroke_to_unbind {
        keymap.unbind(active_mode, &stroke);
    }

    if let Some((mode, old_stroke, new_stroke, action)) = stroke_to_commit_before_switch {
        keymap.rebind(mode, old_stroke.as_ref(), new_stroke, action);
    }

    if let Some(action) = action_to_reset_default {
        let def_strokes = default_strokes_for_action(active_mode, &action);
        // Clear all current keys for this action
        let strokes: Vec<KeyStroke> = match active_mode {
            KeymapMode::Normal => keymap
                .normal
                .iter()
                .filter(|(_, act)| **act == action)
                .map(|(k, _)| k.clone())
                .collect(),
            KeymapMode::Visual => keymap
                .visual
                .iter()
                .filter(|(_, act)| **act == action)
                .map(|(k, _)| k.clone())
                .collect(),
        };
        for s in strokes {
            let map = match active_mode {
                KeymapMode::Normal => &mut keymap.normal,
                KeymapMode::Visual => &mut keymap.visual,
            };
            map.remove(&s);
        }
        // Insert defaults
        for s in def_strokes {
            let map = match active_mode {
                KeymapMode::Normal => &mut keymap.normal,
                KeymapMode::Visual => &mut keymap.visual,
            };
            map.insert(s, action.clone());
        }
        let _ = keymap.save_to_file();
    }

    // ── 11. Centered Delete-Modal-Styled Keybinding Recorder Overlay (No Border) ──
    let mut close_recorder = false;
    let mut commit_from_modal: Option<(KeymapMode, Option<KeyStroke>, KeyStroke, VimAction)> = None;

    if let Some(ref cap) = capture.as_ref() {
        // Dimmed background overlay matching delete modal
        let backdrop_alpha = if theme.is_light() { 90 } else { 160 };
        painter.rect_filled(panel_rect, 0.0, Color32::from_black_alpha(backdrop_alpha));

        // Surface container with NO BORDER (Stroke::NONE) matching delete modal
        painter.rect(
            modal_rect,
            5.0,
            theme.surface(),
            Stroke::NONE,
            egui::StrokeKind::Inside,
        );

        let m_origin = modal_rect.min + vec2(24.0, 22.0);

        // Header: Pill Badge + Modal Title
        let badge_w = if cap.old_stroke.is_some() { 68.0 } else { 60.0 };
        let badge_rect = Rect::from_min_size(m_origin, vec2(badge_w, 20.0));
        let (badge_bg, badge_text_col) = if theme.is_light() {
            (
                Color32::from_rgba_unmultiplied(
                    theme.accent.r(),
                    theme.accent.g(),
                    theme.accent.b(),
                    26,
                ),
                theme.accent,
            )
        } else {
            (
                Color32::from_rgba_unmultiplied(
                    theme.accent.r(),
                    theme.accent.g(),
                    theme.accent.b(),
                    36,
                ),
                theme.highlight,
            )
        };
        painter.rect_filled(badge_rect, 4.0, badge_bg);
        painter.text(
            badge_rect.center(),
            Align2::CENTER_CENTER,
            if cap.old_stroke.is_some() {
                "EDIT KEY"
            } else {
                "NEW KEY"
            },
            FontId::monospace(10.0),
            badge_text_col,
        );

        painter.text(
            m_origin + vec2(badge_w + 10.0, 1.0),
            Align2::LEFT_TOP,
            if cap.old_stroke.is_some() {
                "Edit Keybinding"
            } else {
                "Add Keybinding"
            },
            FontId::monospace(14.5),
            theme.highlight,
        );

        // Body: Command Name & Context
        painter.text(
            m_origin + vec2(0.0, 28.0),
            Align2::LEFT_TOP,
            &cap.action_label,
            FontId::monospace(13.0),
            theme.text,
        );
        let sub_desc = if let Some(ref old) = cap.old_stroke {
            format!("Replacing current key \"{}\"", key_stroke_display(old))
        } else {
            "Assign an additional alternative shortcut".to_string()
        };
        painter.text(
            m_origin + vec2(0.0, 46.0),
            Align2::LEFT_TOP,
            &sub_desc,
            FontId::monospace(11.0),
            theme.muted,
        );

        // Single-Line Input Box for Key Combination
        let input_rect =
            Rect::from_min_size(m_origin + vec2(0.0, 68.0), vec2(modal_w - 48.0, 34.0));
        let input_bg = if theme.is_light() {
            Color32::from_rgb(255, 255, 255)
        } else {
            theme.bg
        };
        painter.rect(
            input_rect,
            5.0,
            input_bg,
            Stroke::new(
                1.0_f32,
                if cap.staged_stroke.is_some() {
                    theme.accent
                } else {
                    theme.border()
                },
            ),
            egui::StrokeKind::Inside,
        );

        let input_display = if let Some(ref staged) = cap.staged_stroke {
            key_stroke_display(staged)
        } else {
            "Press desired key combination...".to_string()
        };
        painter.text(
            input_rect.center(),
            Align2::CENTER_CENTER,
            &input_display,
            FontId::monospace(13.5),
            if cap.staged_stroke.is_some() {
                theme.highlight
            } else {
                theme.muted
            },
        );

        // Conflict Detection Check
        let mut conflict_name: Option<&'static str> = None;
        if let Some(ref staged) = cap.staged_stroke {
            for (k, act) in &current_entries {
                if k == staged && act != &cap.action {
                    conflict_name = Some(action_display_name(act));
                    break;
                }
            }
        }

        let msg_y = input_rect.max.y + 6.0;
        if let Some(c_name) = conflict_name {
            let warn = format!("⚠️ Already bound to \"{c_name}\" (Enter will reassign)");
            painter.text(
                pos2(m_origin.x, msg_y),
                Align2::LEFT_TOP,
                &warn,
                FontId::monospace(11.0),
                Color32::from_rgb(240, 160, 40),
            );
        } else if cap.staged_stroke.is_some() {
            painter.text(
                pos2(m_origin.x, msg_y),
                Align2::LEFT_TOP,
                "✓ Key available. Press Enter or click Save.",
                FontId::monospace(11.0),
                Color32::from_rgb(80, 200, 120),
            );
        } else {
            painter.text(
                pos2(m_origin.x, msg_y),
                Align2::LEFT_TOP,
                "Listening for keystroke... (e.g. k, Up, Ctrl+Shift+X)",
                FontId::monospace(11.0),
                theme.muted,
            );
        }

        // Action Buttons at bottom: Cancel (Esc) & Save (Enter), styled identically to Delete Modal
        let btn_h = 32.0;
        let btn_y = modal_rect.max.y - btn_h - 18.0;
        let save_w = 125.0;
        let cancel_w = 110.0;

        let save_rect = Rect::from_min_size(
            pos2(modal_rect.max.x - 24.0 - save_w, btn_y),
            vec2(save_w, btn_h),
        );
        let cancel_rect = Rect::from_min_size(
            pos2(modal_rect.max.x - 24.0 - save_w - 12.0 - cancel_w, btn_y),
            vec2(cancel_w, btn_h),
        );

        let cancel_hover = ui.rect_contains_pointer(cancel_rect);
        let save_hover = ui.rect_contains_pointer(save_rect);

        // Cancel button
        let (cancel_bg, cancel_stroke, cancel_fg) = if theme.is_light() {
            if cancel_hover {
                (
                    Color32::from_rgb(228, 231, 238),
                    theme.border(),
                    theme.highlight,
                )
            } else {
                (Color32::from_rgb(241, 243, 247), theme.border(), theme.text)
            }
        } else {
            if cancel_hover {
                (
                    Color32::from_rgb(28, 30, 38),
                    Color32::from_gray(80),
                    Color32::WHITE,
                )
            } else {
                (
                    Color32::from_rgb(22, 23, 28),
                    Color32::from_gray(50),
                    Color32::from_gray(180),
                )
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

        // Save button
        let has_staged = cap.staged_stroke.is_some();
        let (save_bg, save_stroke, save_fg) = if theme.is_light() {
            if !has_staged {
                (
                    Color32::from_rgb(241, 243, 247),
                    theme.border(),
                    theme.muted,
                )
            } else if save_hover {
                (
                    Color32::from_rgb(37, 99, 235),
                    Color32::from_rgb(29, 78, 216),
                    Color32::WHITE,
                )
            } else {
                (theme.accent, theme.accent, Color32::WHITE)
            }
        } else {
            if !has_staged {
                (
                    Color32::from_rgb(22, 23, 28),
                    Color32::from_gray(50),
                    Color32::from_gray(120),
                )
            } else if save_hover {
                (theme.highlight, theme.highlight, Color32::WHITE)
            } else {
                (theme.accent, theme.accent, Color32::WHITE)
            }
        };

        painter.rect(
            save_rect,
            5.0,
            save_bg,
            Stroke::new(1.0_f32, save_stroke),
            egui::StrokeKind::Inside,
        );
        painter.text(
            save_rect.center(),
            Align2::CENTER_CENTER,
            "Save (Enter)",
            FontId::monospace(11.5),
            save_fg,
        );

        let cancel_clicked = cancel_hover && ui.input(|i| i.pointer.primary_clicked());
        let save_clicked = has_staged && save_hover && ui.input(|i| i.pointer.primary_clicked());

        if cancel_clicked {
            close_recorder = true;
        } else if save_clicked {
            if let Some(new_stroke) = cap.staged_stroke.clone() {
                commit_from_modal = Some((
                    cap.mode,
                    cap.old_stroke.clone(),
                    new_stroke,
                    cap.action.clone(),
                ));
            }
        }
    }

    // Dismiss recorder when clicking outside the recorder card (Mac Spotlight/Delete modal behavior)
    let primary_clicked = ui.input(|i| i.pointer.primary_clicked());
    let click_pos = ui.input(|i| i.pointer.interact_pos());
    let clicked_outside_modal = capture.is_some()
        && primary_clicked
        && click_pos.map(|p| !modal_rect.contains(p)).unwrap_or(false);

    if clicked_outside_modal && next_capture.is_none() {
        close_recorder = true;
    }

    if close_recorder {
        *capture = None;
    } else if let Some(new_cap) = next_capture {
        *capture = new_cap;
    }

    if let Some((mode, old_stroke, new_stroke, action)) = commit_from_modal {
        keymap.rebind(mode, old_stroke.as_ref(), new_stroke, action);
        *capture = None;
    }

    // Sleek custom scrollbar matching Shortcuts tab
    if max_scroll > 0.0 {
        let track_x = panel_rect.max.x - 8.0;
        let track = Rect::from_min_max(
            pos2(track_x, list_top),
            pos2(track_x + 3.0, panel_rect.max.y - 8.0),
        );
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

fn is_action_modified(mode: KeymapMode, action: &VimAction, current_keys: &[KeyStroke]) -> bool {
    let mut default_keys = default_strokes_for_action(mode, action);
    let mut curr = current_keys.to_vec();
    default_keys.sort_by(|a, b| key_stroke_display(a).cmp(&key_stroke_display(b)));
    curr.sort_by(|a, b| key_stroke_display(a).cmp(&key_stroke_display(b)));
    default_keys != curr
}

fn default_strokes_for_action(mode: KeymapMode, action: &VimAction) -> Vec<KeyStroke> {
    let standard = VimKeymap::new_standard();
    let map = match mode {
        KeymapMode::Normal => &standard.normal,
        KeymapMode::Visual => &standard.visual,
    };
    map.iter()
        .filter(|(_, act)| *act == action)
        .map(|(k, _)| k.clone())
        .collect()
}

fn action_display_name(action: &VimAction) -> &'static str {
    match action {
        VimAction::Motion(VimMotion::Left) => "Move Left",
        VimAction::Motion(VimMotion::Right) => "Move Right",
        VimAction::Motion(VimMotion::UpVisual) => "Move Up",
        VimAction::Motion(VimMotion::DownVisual) => "Move Down",
        VimAction::Motion(VimMotion::WordForward) => "Next Word",
        VimAction::Motion(VimMotion::WordBackward) => "Previous Word",
        VimAction::Motion(VimMotion::LineStart) => "Line Start",
        VimAction::Motion(VimMotion::LineEnd) => "Line End",
        VimAction::Motion(VimMotion::BufferStart) => "Top of Buffer",
        VimAction::Motion(VimMotion::BufferEnd) => "Bottom of Buffer",
        VimAction::ToggleTaskCheckbox => "Toggle Task Checkbox",
        VimAction::DeleteChar => "Delete Character",
        VimAction::Undo => "Undo",
        VimAction::Redo => "Redo",
        VimAction::Paste { before: false } => "Paste After",
        VimAction::Paste { before: true } => "Paste Before",
        VimAction::DuplicateLine => "Duplicate Line",
        VimAction::EnterInsert(InsertPosition::AtCursor) => "Insert Mode",
        VimAction::EnterInsert(InsertPosition::AfterCursor) => "Append",
        VimAction::EnterInsert(InsertPosition::LineStart) => "Insert at Line Start",
        VimAction::EnterInsert(InsertPosition::LineEnd) => "Append at Line End",
        VimAction::EnterInsert(InsertPosition::LineBelow) => "Open Line Below",
        VimAction::EnterInsert(InsertPosition::LineAbove) => "Open Line Above",
        VimAction::EnterVisual { is_line: false } => "Visual Mode",
        VimAction::EnterVisual { is_line: true } => "Visual Line Mode",
        VimAction::EnterSearch { backward: false } => "Search Forward",
        VimAction::EnterSearch { backward: true } => "Search Backward",
        VimAction::RepeatSearch { reverse: false } => "Repeat Search",
        VimAction::RepeatSearch { reverse: true } => "Repeat Search Backward",
        VimAction::Operator(VimOperator::Delete) => "Delete Operator",
        VimAction::Operator(VimOperator::Yank) => "Yank (Copy) Operator",
        VimAction::Operator(VimOperator::Change) => "Change Operator",
        _ => "Other Action",
    }
}

fn canonical_actions_for_mode(mode: KeymapMode) -> Vec<VimAction> {
    match mode {
        KeymapMode::Normal => vec![
            VimAction::ToggleTaskCheckbox,
            VimAction::Motion(VimMotion::Left),
            VimAction::Motion(VimMotion::Right),
            VimAction::Motion(VimMotion::UpVisual),
            VimAction::Motion(VimMotion::DownVisual),
            VimAction::Motion(VimMotion::WordForward),
            VimAction::Motion(VimMotion::WordBackward),
            VimAction::Motion(VimMotion::LineStart),
            VimAction::Motion(VimMotion::LineEnd),
            VimAction::Motion(VimMotion::BufferStart),
            VimAction::Motion(VimMotion::BufferEnd),
            VimAction::DeleteChar,
            VimAction::Undo,
            VimAction::Redo,
            VimAction::Paste { before: false },
            VimAction::Paste { before: true },
            VimAction::DuplicateLine,
            VimAction::EnterInsert(InsertPosition::AtCursor),
            VimAction::EnterInsert(InsertPosition::AfterCursor),
            VimAction::EnterInsert(InsertPosition::LineStart),
            VimAction::EnterInsert(InsertPosition::LineEnd),
            VimAction::EnterInsert(InsertPosition::LineBelow),
            VimAction::EnterInsert(InsertPosition::LineAbove),
            VimAction::EnterVisual { is_line: false },
            VimAction::EnterVisual { is_line: true },
            VimAction::EnterSearch { backward: false },
            VimAction::EnterSearch { backward: true },
            VimAction::RepeatSearch { reverse: false },
            VimAction::RepeatSearch { reverse: true },
            VimAction::Operator(VimOperator::Delete),
            VimAction::Operator(VimOperator::Yank),
            VimAction::Operator(VimOperator::Change),
        ],
        KeymapMode::Visual => vec![
            VimAction::ToggleTaskCheckbox,
            VimAction::Motion(VimMotion::Left),
            VimAction::Motion(VimMotion::Right),
            VimAction::Motion(VimMotion::UpVisual),
            VimAction::Motion(VimMotion::DownVisual),
            VimAction::Motion(VimMotion::WordForward),
            VimAction::Motion(VimMotion::WordBackward),
            VimAction::Motion(VimMotion::LineStart),
            VimAction::Motion(VimMotion::LineEnd),
            VimAction::Motion(VimMotion::BufferStart),
            VimAction::Motion(VimMotion::BufferEnd),
            VimAction::Operator(VimOperator::Yank),
            VimAction::Operator(VimOperator::Delete),
            VimAction::Operator(VimOperator::Change),
        ],
    }
}
