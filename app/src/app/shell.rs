//! Application shell: titlebar, layout frame, splitters, bottom dock, and sidebar interactions.

use super::{App, EditorInputMode, OpenNote, RightPaneTab};
use crate::mode::Mode;
use crate::sidebar::{render_sidebar, SidebarAction};
use eframe::egui::{self, pos2, vec2, Color32, Rect, Stroke, Ui};

impl App {
    /// Renders the entire application frame: window frame, titlebar, splitters, editor panes, sidebar, and bottom dock.
    pub fn draw(&mut self, ui: &mut Ui, dt: f32, now: f64, typed: bool) {
        let (_cw, _lh) = self.cell_size(ui.ctx());
        let painter = ui.painter().clone();
        let bounds = ui.max_rect();

        // 5px Rounded window background frame
        let win_bg = Color32::from_rgba_unmultiplied(
            self.theme.bg.r(),
            self.theme.bg.g(),
            self.theme.bg.b(),
            (self.opacity * 255.0) as u8,
        );
        painter.rect(
            bounds,
            5.0,
            win_bg,
            Stroke::new(1.0_f32, Color32::from_rgb(32, 34, 40)),
            egui::StrokeKind::Inside,
        );

        // Apply Acrylic / Mica backdrop blur on first frame
        if self.first_frame {
            crate::services::blur::apply_window_blur(self.blur_effect);
        }

        // Full Control Window Dragging:
        // 1. Alt + Left-Click Drag anywhere on the canvas (Linux / Blender / Neovim GUI convention)
        if ui.input(|i| i.modifiers.alt && i.pointer.primary_down()) {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }

        let is_titlebar_visible = self.show_titlebar;

        // 2. Invisible top-edge grab strip (top 7px) whenever titlebar is hidden
        if !is_titlebar_visible {
            if let Some(pos) = ui.input(|i| i.pointer.interact_pos().or_else(|| i.pointer.hover_pos())) {
                if pos.y <= bounds.min.y + 7.0 && ui.input(|i| i.pointer.primary_down()) {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
            }
        }
        let sidebar_visible = self.sidebar_open;
        let layout = crate::layout::compute_modular_layout_ex(
            bounds,
            sidebar_visible,
            self.sidebar_width,
            false,
            0.0,
            is_titlebar_visible,
            true,
        );
        let titlebar_rect = layout.titlebar_rect;
        let cmd_bar_rect = layout.cmd_bar_rect;
        let editor_panel_rect = layout.editor_panel_rect;

        // Full-width modern Titlebar (hidden if show_titlebar is false or in Zen mode)
        let (titlebar_action, accent_anchor_rect) = if is_titlebar_visible {
            let (header_title, header_dirty) = match self.mode {
                Mode::Doc => {
                    let doc_title = crate::ui::docs::get_docs()
                        .get(self.active_doc_idx)
                        .map(|d| d.title)
                        .unwrap_or("Documentation");
                    (format!("📖 {}", doc_title), false)
                }
                Mode::Normal => {
                    if self.open_notes.is_empty() || self.show_welcome {
                        ("MindForge".to_string(), false)
                    } else {
                        (self.active_note_title.clone(), self.is_dirty)
                    }
                }
                Mode::Help => ("✦ Quick Start Guide".to_string(), false),
                Mode::Stats => ("📊 Daily Story & Statistics".to_string(), false),
                Mode::ScanReport | Mode::ScanHistory => ("🌐 Security Scanner".to_string(), false),
                Mode::Terminal => ("💻 Embedded Terminal".to_string(), false),
            };

            let (action, anchor) = crate::view_editor::render_full_titlebar(
                ui,
                &painter,
                titlebar_rect,
                &header_title,
                header_dirty,
                &self.theme,
                self.accent_dropdown_open,
                self.opacity,
            );
            (action, anchor)
        } else {
            (None, Rect::NOTHING)
        };

        if let Some(crate::view_editor::TitlebarAction::ToggleAccentDropdown) = titlebar_action {
            self.accent_dropdown_open = !self.accent_dropdown_open;
        }

        let pointer_pos = ui.input(|i| i.pointer.hover_pos().or_else(|| i.pointer.interact_pos()));
        let is_pointer_over_ai = self.agent_state.is_open && self.agent_state.window_rect.map_or(false, |r| {
            pointer_pos.map_or(false, |p| r.contains(p))
        });

        // Sidebar Splitter Divider & Knob
        let any_modal_open = self.settings_open
            || self.search_open
            || self.help_open
            || self.rename_open
            || self.delete_confirm_open
            || self.accent_dropdown_open
            || self.workspace_importer.is_modal_open
            || is_pointer_over_ai;

        if let Some(center_x) = layout.splitter_center_x {
            let panel_top = editor_panel_rect.min.y;
            let panel_bottom = editor_panel_rect.max.y;
            let knob_mid = pos2(center_x, (panel_top + panel_bottom) * 0.5);
            let is_dragging = self.is_dragging_sidebar_splitter;
            let knob_w = if is_dragging { 6.0 } else { 4.0 };
            let knob_h = 36.0;
            let knob_rect = Rect::from_center_size(knob_mid, vec2(knob_w, knob_h));
            // Generous interactive hit area specifically on the knob
            let knob_hit_rect = Rect::from_center_size(knob_mid, vec2(16.0, 44.0));

            let is_knob_hovered = !any_modal_open && ui.rect_contains_pointer(knob_hit_rect);
            let primary_down = ui.input(|i| i.pointer.primary_down());
            let primary_pressed = ui.input(|i| i.pointer.primary_clicked() || i.pointer.button_pressed(egui::PointerButton::Primary));

            if is_knob_hovered && primary_pressed {
                self.is_dragging_sidebar_splitter = true;
            }

            if self.is_dragging_sidebar_splitter {
                if primary_down {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeColumn);
                    if let Some(pos) = ui.input(|i| i.pointer.interact_pos().or_else(|| i.pointer.hover_pos())) {
                        let drag_x = pos.x - bounds.min.x - crate::layout::GAP;
                        if drag_x < 70.0 {
                            self.sidebar_open = false;
                            self.is_dragging_sidebar_splitter = false;
                        } else {
                            let max_sb = (bounds.width() - 200.0).clamp(crate::layout::MIN_SIDEBAR_W, crate::layout::MAX_SIDEBAR_W);
                            let new_w = drag_x.clamp(crate::layout::MIN_SIDEBAR_W, max_sb);
                            self.sidebar_width = new_w;
                        }
                    }
                } else {
                    self.is_dragging_sidebar_splitter = false;
                    let _ = self.db_tx.send(crate::services::db_worker::DbMsg::SaveSetting {
                        key: "sidebar_w".into(),
                        val: self.sidebar_width.to_string(),
                    });
                }
            } else if is_knob_hovered {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeColumn);
            }

            let is_active = is_knob_hovered || self.is_dragging_sidebar_splitter;

            // Render tactile knob only — no harsh full-height line
            let active_knob_rect = if is_active {
                Rect::from_center_size(knob_mid, vec2(6.0, 38.0))
            } else {
                knob_rect
            };
            painter.rect_filled(
                active_knob_rect,
                2.5,
                if is_active {
                    self.theme.accent
                } else {
                    Color32::from_rgba_unmultiplied(self.theme.muted.r(), self.theme.muted.g(), self.theme.muted.b(), 100)
                },
            );

            let grip_color = self.theme.bg;
            for dy in [-5.0, 0.0, 5.0] {
                painter.line_segment(
                    [pos2(knob_mid.x - 1.2, knob_mid.y + dy), pos2(knob_mid.x + 1.2, knob_mid.y + dy)],
                    Stroke::new(1.0_f32, grip_color),
                );
            }
        }

        // Render Doc Sidebar if in Documentation mode
        if self.mode == Mode::Doc && self.sidebar_open {
            if let Some(sb_rect) = layout.sidebar_rect {
                let doc_action = crate::ui::docs::render_doc_sidebar(
                    ui,
                    &painter,
                    sb_rect,
                    self.active_doc_idx,
                    self.doc_selected_idx,
                    self.doc_sidebar_focused,
                    &self.theme,
                    self.opacity,
                    any_modal_open,
                );
                if let Some(action) = doc_action {
                    match action {
                        crate::ui::docs::DocSidebarAction::SelectDoc(idx) => {
                            self.load_doc_by_index(idx, now);
                            self.doc_sidebar_focused = true;
                        }
                        crate::ui::docs::DocSidebarAction::ToggleSidebar => {
                            self.sidebar_open = !self.sidebar_open;
                            self.doc_sidebar_focused = self.sidebar_open;
                            let msg = if self.sidebar_open {
                                "Documentation sidebar opened"
                            } else {
                                "Documentation sidebar collapsed into full-width reader (Ctrl+B to reopen)"
                            };
                            self.set_status(msg, now);
                        }
                        crate::ui::docs::DocSidebarAction::BackToEditor => {
                            self.mode = Mode::Normal;
                            self.set_status("Switched to Notes Editor", now);
                        }
                        crate::ui::docs::DocSidebarAction::OpenSettings => {
                            self.settings_open = true;
                            self.settings_just_opened = true;
                        }
                    }
                }
            }
        }

        // Render Editor & Split Panes (Delegated to panes.rs)
        self.render_editor_panes(
            ui,
            &painter,
            bounds,
            editor_panel_rect,
            accent_anchor_rect,
            any_modal_open,
            dt,
            now,
            typed,
        );



        // Update ShowCmd card timeout
        if self.editor_controller.mode == EditorInputMode::Vim {
            self.showcmd.update(now);
        }

        // Bottom Dock (active editing mode badge, status feedback, word stats)
        let (row, col) = if self.mode == Mode::Normal
            && self.editor_controller.mode == EditorInputMode::Vim
        {
            self.vim_runtime
                .backend
                .as_ref()
                .map(|backend| (backend.grid.cursor.row, backend.grid.cursor.column))
                .unwrap_or((0, 0))
        } else if self.mode == Mode::Doc {
            self.doc_ed.visual_row_col(&self.visual_lines)
        } else {
            self.ed.visual_row_col(&self.visual_lines)
        };
        let (total_rows, word_count, total_chars) = if self.mode == Mode::Normal
            && self.editor_controller.mode == EditorInputMode::Vim
        {
            self.vim_runtime
                .backend
                .as_ref()
                .map(crate::vim::VimBackend::document_stats)
                .unwrap_or((0, 0, 0))
        } else if self.mode == Mode::Doc {
            let text = self.doc_ed.text();
            (text.lines().count(), text.split_whitespace().count(), text.len())
        } else {
            let text = self.ed.text();
            (text.lines().count(), text.split_whitespace().count(), text.len())
        };
        let mode_badge_str = match self.editor_controller.mode {
            EditorInputMode::Vim => self.vim_runtime.backend.as_ref().map(|backend| backend.grid.mode.to_uppercase()).unwrap_or_else(|| "STARTING NVIM".into()),
            EditorInputMode::Hybrid => "HYBRID".to_string(),
        };

        let search_prompt: Option<(&str, &str, usize)> = None;

        let active_title = if self.mode == Mode::Doc {
            crate::ui::docs::BRAIN_DOCS.get(self.active_doc_idx).map(|d| d.title).unwrap_or("Documentation")
        } else {
            self.active_note_title.as_str()
        };

        let is_ai_active = self.preview_open && self.right_pane_tab == RightPaneTab::AiAgent;
        let toggle_ai = crate::lunaline::render_lunaline(crate::lunaline::LunaLineRenderParams {
            ui,
            painter: &painter,
            dock_rect: cmd_bar_rect,
            in_command: self.in_command,
            cmd_prefix: self.cmd_prefix,
            cmd_text: &self.cmd_ed.text(),
            cmd_cur: self.cmd_ed.cur,
            cmd_selection: self.cmd_ed.selected_range(),
            status_msg: &self.status_msg,
            status_time: self.status_time,
            now,
            cursor_row: row + 1,
            cursor_col: col + 1,
            total_rows,
            total_words: word_count,
            total_chars,
            active_note_title: active_title,
            is_dirty: if self.mode == Mode::Doc { false } else { self.is_dirty },
            is_doc: self.mode == Mode::Doc,
            mode_badge: Some(mode_badge_str.as_str()),
            search_prompt,
            theme: &self.theme,
            opacity: self.opacity,
            is_ai_open: is_ai_active,
            config: &self.lunaline_config,
        });
        if toggle_ai {
            if self.preview_open && self.right_pane_tab == RightPaneTab::AiAgent {
                self.preview_open = false;
                self.agent_state.is_open = false;
                ui.memory_mut(|m| m.surrender_focus(egui::Id::new("deepseek_prompt_input")));
                self.set_status("AI Agent closed", now);
            } else {
                self.preview_open = true;
                self.right_pane_tab = RightPaneTab::AiAgent;
                self.ai_focus_requested = true;
                self.agent_state.is_open = true;
                ui.memory_mut(|m| m.request_focus(egui::Id::new("deepseek_prompt_input")));
                self.set_status("AI Assistant opened (Ctrl+Shift+I to toggle)", now);
            }
        }


        // Sleek Sidebar (Ctrl+B)
        if self.sidebar_open && self.mode != Mode::Doc {
            if let Some(sb_rect) = layout.sidebar_rect {
                let active_mode_idx = match self.mode {
                    Mode::Normal => 0,
                    Mode::Stats => 1,
                    Mode::Doc | Mode::Help | Mode::ScanReport | Mode::ScanHistory | Mode::Terminal => 0,
                };
                let action = render_sidebar(
                    ui,
                    &painter,
                    sb_rect,
                    active_mode_idx,
                    self.active_note_id,
                    &self.notes_list,
                    self.sidebar_notes_limit,
                    self.total_notes_count,
                    self.is_dirty,
                    &self.theme,
                    self.sidebar_selected_idx,
                    self.sidebar_focused,
                    self.opacity,
                    self.sidebar_needs_scroll,
                    any_modal_open,
                );
                self.sidebar_needs_scroll = false;

                if !any_modal_open && ui.input(|i| i.pointer.primary_clicked()) {
                    if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                        if sb_rect.contains(pos) {
                            self.sidebar_focused = true;
                        } else if editor_panel_rect.contains(pos) {
                            self.sidebar_focused = false;
                        }
                    }
                }

                if let Some(act) = action {
                    if !any_modal_open {
                        match act {
                        SidebarAction::SwitchMode(idx) => {
                            match idx {
                                0 => self.mode = Mode::Normal,
                                1 => {
                                    self.reload_db_state();
                                    self.mode = Mode::Stats;
                                }
                                _ => {}
                            }
                        }
                        SidebarAction::LoadNote { id, topic, body, index } => {
                            self.load_note(id, topic, body, now);
                            self.sidebar_selected_idx = index;
                            self.sidebar_focused = true;
                        }
                        SidebarAction::NewNote => {
                            if let Some(cur) = self.open_notes.get_mut(self.active_tab) {
                                cur.editor = self.ed.clone();
                                cur.title = self.active_note_title.clone();
                                cur.scroll_y = self.scroll_y;
                                cur.is_dirty = self.is_dirty;
                            }
                            self.active_note_id = None;
                            self.save_active_note_id();
                            self.active_note_title = "Untitled Note".to_string();
                            self.ed.clear();
                            self.mode = Mode::Normal;
                            self.is_dirty = false;
                            self.scroll_y = 0.0;
                            self.open_notes.push(OpenNote {
                                id: 0,
                                title: "Untitled Note".to_string(),
                                editor: self.ed.clone(),
                                scroll_y: 0.0,
                                is_dirty: false,
                            });
                            self.active_tab = self.open_notes.len() - 1;
                            self.save_open_tabs();
                            self.set_status("Created new note", now);
                        }
                        SidebarAction::DeleteNote(id) => {
                            self.pending_delete_note_id = Some(id);
                            self.delete_confirm_open = true;
                            self.delete_just_opened = true;
                        }
                        SidebarAction::ToggleNotesLimit => {
                            self.sidebar_notes_limit = if self.sidebar_notes_limit >= 100 { 50 } else { 100 };
                            self.reload_db_state();
                        }
                        SidebarAction::OpenSettings => {
                            self.settings_open = true;
                            self.settings_just_opened = true;
                        }
                    }
                    }
                }
            }
        }


        // Command Autocomplete Popup (Pops above bottom dock, rendered on top of sidebar)
        crate::command::command_suggestion::render_command_suggestions_overlay(
            self,
            ui,
            &painter,
            cmd_bar_rect,
            now,
        );

        // Drag-and-drop hover indicator overlay
        crate::workspace_import::render_hover_indicator(ui.ctx(), &painter, bounds, &self.theme);

        // Wikilink Autocomplete & Hover Preview Popups
        if self.mode == Mode::Normal && !any_modal_open {
            let are_tabs_visible = self.show_tabs && !self.zen_mode;
            let tab_bar_h = if are_tabs_visible && !self.open_notes.is_empty() {
                crate::view_editor::TAB_ROW_H
            } else {
                0.0
            };
            let body_min_y = editor_panel_rect.min.y + tab_bar_h;
            let body_rect = Rect::from_min_max(
                pos2(editor_panel_rect.min.x, body_min_y),
                editor_panel_rect.max,
            );

            let is_preview_active = (self.preview_open
                || self.right_pane_tab == crate::app::RightPaneTab::AiAgent)
                && !self.show_welcome
                && !self.open_notes.is_empty();

            let actual_editor_rect = self.last_editor_rect.unwrap_or_else(|| {
                if is_preview_active {
                    let divider_w = 11.0;
                    let total_w = editor_panel_rect.width();
                    let available_w = (total_w - divider_w).max(300.0);
                    let min_w = 150.0f32;
                    let min_right_w = 150.0f32;
                    let max_w = (available_w - min_right_w).max(min_w);
                    let left_w = (available_w * self.split_ratio).clamp(min_w, max_w);
                    Rect::from_min_max(body_rect.min, pos2(editor_panel_rect.min.x + left_w, body_rect.max.y))
                } else {
                    body_rect
                }
            });

            let effective_font_size = self.last_ed_font_size.unwrap_or(self.font_size);
            let gutter_w = if self.show_line_numbers {
                let total_lines = (self.ed.buf.iter().filter(|&&c| c == '\n').count() + 1).max(1);
                let digits = total_lines.to_string().len().max(2);
                (digits as f32 * (effective_font_size * 0.55) + 14.0).max(28.0)
            } else {
                0.0
            };
            let pad_x = if self.show_line_numbers { 16.0 } else { 24.0 };
            let pad_y = 10.0;
            let effective_gutter_w = if actual_editor_rect.width() > gutter_w + 40.0 { gutter_w } else { 0.0 };
            let text_left = (actual_editor_rect.min.x + effective_gutter_w + pad_x).min(actual_editor_rect.max.x);
            let ed_origin = self.last_ed_origin.unwrap_or_else(|| pos2(text_left, actual_editor_rect.min.y - self.scroll_y + pad_y));
            // Wiki link interaction uses the same raw monospace grid as the
            // editor. Do not build an inline Markdown layout in this per-frame
            // input path.
            let cell_w = painter
                .layout_no_wrap(
                    "M".to_owned(),
                    crate::services::font_manager::editor_font_id(effective_font_size),
                    egui::Color32::WHITE,
                )
                .size()
                .x
                .max(1.0);
            let line_h = effective_font_size * 1.5;

            let is_inserting = match self.editor_controller.mode {
                EditorInputMode::Vim => self.vim_runtime.backend.as_ref().is_some_and(|backend| backend.is_insert_mode()),
                EditorInputMode::Hybrid => true,
            };

            if is_inserting {
                self.wikilink_autocomplete.check_trigger(&self.ed, &self.notes_list);

                if self.wikilink_autocomplete.is_active {
                    let idx = self.wikilink_autocomplete.trigger_start;
                    if let Some(line) = self.visual_lines.iter().find(|line| idx >= line.char_start && idx <= line.char_end) {
                        let row = self.visual_lines.iter().position(|candidate| std::ptr::eq(candidate, line)).unwrap_or(0);
                        let col = idx.saturating_sub(line.char_start);
                        self.wikilink_autocomplete.trigger_screen_pos = pos2(
                            ed_origin.x + col as f32 * cell_w,
                            ed_origin.y + row as f32 * line_h + line_h + 2.0,
                        );
                        self.wikilink_autocomplete.trigger_line_height = line_h;
                    }
                }

                if let Some(crate::wikilink::wikilink_autocompletion::AutocompleteAction::Inserted { inserted_text: _ }) =
                    crate::wikilink::wikilink_autocompletion::render_wikilink_autocomplete(
                        ui,
                        &painter,
                        &mut self.wikilink_autocomplete,
                        &mut self.ed,
                        &self.theme,
                        bounds,
                    )
                {
                    // Autocomplete is an application feature. Commit its edit to
                    // Neovim at this explicit boundary so the Vim buffer remains
                    // the single editing source of truth.
                    if self.editor_controller.mode == EditorInputMode::Vim {
                        let (row, column) = self.ed.row_col();
                        if let Some(backend) = self.vim_runtime.backend.as_mut() {
                            if let Err(error) = backend.set_document(&self.ed.text(), row, column) {
                                self.set_status(&format!("Could not sync autocomplete to Neovim: {error}"), now);
                            }
                        }
                    }
                    self.is_dirty = true;
                    self.sound.play();
                }
            } else {
                self.wikilink_autocomplete.clear();
            }

            if let Some(pos) = pointer_pos {
                let pointer_changed = self.hover_wikilink.last_pointer_pos != Some(pos);
                if actual_editor_rect.contains(pos) && (pointer_changed || ui.input(|i| i.pointer.primary_clicked() || i.pointer.button_pressed(egui::PointerButton::Primary))) {
                    self.hover_wikilink.last_pointer_pos = Some(pos);
                    let text = self.ed.text();
                    let links = crate::wikilink::extract_wikilinks(&text);
                    let mut found_hover = None;

                    let clicked_row = ((pos.y - ed_origin.y) / line_h).floor().max(0.0) as usize;
                    let clicked_col = ((pos.x - ed_origin.x).max(0.0) / cell_w).floor() as usize;
                    let visual_line = self.visual_lines.get(clicked_row);
                    let char_idx = visual_line.map(|line| line.char_start + clicked_col).unwrap_or(self.ed.buf.len());

                    for link in &links {
                        let is_hit = char_idx >= link.start && char_idx <= link.end;

                        if is_hit {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            let row = self.visual_lines.iter().position(|line| link.start >= line.char_start && link.start <= line.char_end).unwrap_or(0);
                            let line = &self.visual_lines[row];
                            let anchor = pos2(ed_origin.x + link.start.saturating_sub(line.char_start) as f32 * cell_w, ed_origin.y + (row + 1) as f32 * line_h);
                            found_hover = Some((link.target.clone(), anchor));

                            if ui.input(|i| i.pointer.primary_clicked() || i.pointer.button_pressed(egui::PointerButton::Primary)) {
                                if let Some(note) = crate::wikilink::resolve_wikilink(&link.target, &self.notes_list) {
                                    self.open_note_by_id(note.id, now);
                                } else {
                                    self.create_new_note(now);
                                    crate::notes::rename_active_note(self, &link.target, now);
                                }
                            }
                            break;
                        }
                    }

                    if let Some((target, anchor)) = found_hover {
                        self.hover_wikilink.update_hover(&target, anchor, &self.notes_list, now);
                    } else {
                        self.hover_wikilink.pending_target = None;
                        self.hover_wikilink.dismissed_target = None;

                        // Safe bridge corridor between link anchor and popup card
                        let in_bridge = if let Some(popup) = self.hover_wikilink.popup_rect {
                            let bridge = Rect::from_min_max(
                                pos2(popup.min.x.min(self.hover_wikilink.anchor_pos.x - 24.0), (self.hover_wikilink.anchor_pos.y - 24.0).min(popup.min.y)),
                                pos2(popup.max.x.max(self.hover_wikilink.anchor_pos.x + 80.0), popup.max.y + 10.0),
                            );
                            bridge.contains(pos)
                        } else {
                            false
                        };

                        if in_bridge || self.hover_wikilink.is_mouse_inside_popup {
                            self.hover_wikilink.last_hover_time = now;
                        } else if self.hover_wikilink.is_active() {
                            // 400ms grace window for mouse travel
                            if now - self.hover_wikilink.last_hover_time > 0.40 {
                                self.hover_wikilink.clear();
                            }
                        }
                    }
                } else if !self.hover_wikilink.is_mouse_inside_popup {
                    if self.hover_wikilink.is_active() {
                        if now - self.hover_wikilink.last_hover_time > 0.40 {
                            self.hover_wikilink.clear();
                        }
                    } else {
                        self.hover_wikilink.pending_target = None;
                        self.hover_wikilink.clear();
                    }
                }
            } else if !self.hover_wikilink.is_mouse_inside_popup {
                if self.hover_wikilink.is_active() {
                    if now - self.hover_wikilink.last_hover_time > 0.40 {
                        self.hover_wikilink.clear();
                    }
                } else {
                    self.hover_wikilink.pending_target = None;
                    self.hover_wikilink.clear();
                }
            }

            // Keyboard navigation / typing in editor dismisses hover preview
            if self.hover_wikilink.is_active() && !self.hover_wikilink.is_mouse_inside_popup {
                let key_active = ui.input(|i| {
                    !i.events.is_empty()
                        && i.events.iter().any(|e| matches!(e, egui::Event::Key { .. } | egui::Event::Text(_)))
                });
                if key_active {
                    self.hover_wikilink.dismiss();
                }
            }
        }

        if let Some(act) = crate::wikilink::hover_wikilink::render_hover_wikilink_popup(
            ui,
            &painter,
            &mut self.hover_wikilink,
            &self.theme,
            self.font_size,
            bounds,
        ) {
            match act {
                crate::wikilink::hover_wikilink::HoverWikiLinkAction::OpenNote { id, title } => {
                    if let Some(note_id) = id {
                        self.open_note_by_id(note_id, now);
                    } else {
                        self.create_new_note(now);
                        crate::notes::rename_active_note(self, &title, now);
                    }
                }
            }
        }

        // Render Modal dialogs (Preferences, Search, Rename, Delete, Accent dropdown, Workspace Import)
        self.render_modals(ui, &painter, bounds, accent_anchor_rect, now);

        // Poll DeepSeek background worker for any completed responses
        self.agent_state.poll_response();
    }
}
