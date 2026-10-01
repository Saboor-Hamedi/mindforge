//! Editor panes layout, splitters, tab strips, and view modes.

use super::{App, EditorInputMode};
use crate::editor::backend::EditorBackend;
use crate::mode::Mode;
use crate::view_editor::{render_editor_body, render_inline_editor};
use eframe::egui::{self, pos2, vec2, Color32, FontId, Rect, Stroke, Ui};

impl App {
    /// Renders the central workspace area: editor body, split panes, tab bars, right pane (preview/AI), terminal, and alternative views.
    pub fn render_editor_panes(
        &mut self,
        ui: &mut Ui,
        painter: &egui::Painter,
        bounds: Rect,
        editor_panel_rect: Rect,
        accent_anchor_rect: Rect,
        any_modal_open: bool,
        dt: f32,
        now: f64,
        typed: bool,
    ) {
        // Docked bottom terminal layout: splits editor_panel_rect vertically so terminal sits under editor & preview
        let (top_panel_rect, bottom_terminal_rect, term_splitter_rect_opt) = if self.terminal.open && (self.misc.mode == Mode::Normal || self.misc.mode == Mode::Doc) {
            let total_h = editor_panel_rect.height();
            let divider_h = 10.0;
            let tab_bar_h = crate::view_editor::TAB_ROW_H;
            let min_top_h = tab_bar_h + 60.0;
            let available_h = (total_h - divider_h).max(min_top_h + 80.0);
            let min_h = 80.0f32;
            let max_h = (total_h - divider_h - min_top_h).max(min_h);
            let term_h = (available_h * self.terminal.split_ratio).clamp(min_h, max_h);

            let top_split_y = (editor_panel_rect.max.y - term_h - divider_h).max(editor_panel_rect.min.y + min_top_h);
            let top_rect = Rect::from_min_max(
                editor_panel_rect.min,
                pos2(editor_panel_rect.max.x, top_split_y),
            );
            let divider_rect = Rect::from_min_max(
                pos2(editor_panel_rect.min.x, top_rect.max.y),
                pos2(editor_panel_rect.max.x, top_rect.max.y + divider_h),
            );
            let bottom_rect = Rect::from_min_max(
                pos2(editor_panel_rect.min.x, divider_rect.max.y),
                editor_panel_rect.max,
            );
            (top_rect, Some(bottom_rect), Some(divider_rect))
        } else {
            (editor_panel_rect, None, None)
        };

        // Detached Editor & Preview Panel Surface (subtle card background & border derived from theme)
        let is_preview_active = self.editor.preview_open && (self.misc.mode == Mode::Normal || self.misc.mode == Mode::Doc);
        let divider_w = 11.0;
        let available_w = (top_panel_rect.width() - divider_w).max(300.0);
        let min_w = 150.0f32;
        let min_right_w = 150.0f32;
        let max_w = (available_w - min_right_w).max(min_w);
        let left_w = if is_preview_active {
            (available_w * self.editor.split_ratio).clamp(min_w, max_w)
        } else {
            top_panel_rect.width()
        };

        let ed_card_rect = Rect::from_min_max(
            top_panel_rect.min,
            pos2(top_panel_rect.min.x + left_w, top_panel_rect.max.y),
        );
        let preview_card_rect = if is_preview_active {
            Some(Rect::from_min_max(
                pos2(ed_card_rect.max.x + divider_w, top_panel_rect.min.y),
                top_panel_rect.max,
            ))
        } else {
            None
        };

        // Draw card surfaces: preserve desktop blur & window opacity across editor and preview
        let is_editor_focused = !self.sidebar.open || (!self.sidebar.focused && !self.tabs.doc_sidebar_focused);
        let ed_stroke = if self.sidebar.open && is_editor_focused {
            Stroke::new(1.0, self.misc.theme.accent.gamma_multiply(0.40))
        } else {
            Stroke::new(1.0, self.misc.theme.border().gamma_multiply(0.60))
        };
        let ed_bg = if self.misc.blur_effect == crate::services::blur::BlurEffect::None || self.misc.opacity >= 0.99 {
            self.misc.theme.bg
        } else {
            let alpha = ((self.misc.opacity * 255.0) as u8).max(225);
            Color32::from_rgba_unmultiplied(
                self.misc.theme.bg.r(),
                self.misc.theme.bg.g(),
                self.misc.theme.bg.b(),
                alpha,
            )
        };
        painter.rect(
            ed_card_rect,
            5.0,
            ed_bg,
            ed_stroke,
            egui::StrokeKind::Inside,
        );

        if let Some(p_card) = preview_card_rect {
            let p_alpha = ((self.misc.opacity * 255.0) as u8).saturating_sub(15).max(30);
            let p_bg = Color32::from_rgba_unmultiplied(
                self.misc.theme.surface().r(),
                self.misc.theme.surface().g(),
                self.misc.theme.surface().b(),
                p_alpha,
            );
            painter.rect(
                p_card,
                5.0,
                p_bg,
                Stroke::new(1.0, self.misc.theme.border().gamma_multiply(0.60)),
                egui::StrokeKind::Inside,
            );
        }

        // Tab strip on the editor card
        let tab_bar_rect = Rect::from_min_max(
            ed_card_rect.min,
            pos2(ed_card_rect.max.x, ed_card_rect.min.y + crate::view_editor::TAB_ROW_H),
        );

        let tab_occluded_rect = if self.modal.settings_open
            || self.modal.search_open
            || self.modal.help_open
            || self.modal.rename_open
            || self.modal.delete_confirm_open
        {
            Some(bounds)
        } else if self.misc.accent_dropdown_open {
            let min_x = (accent_anchor_rect.max.x - 320.0).max(8.0);
            let min_y = accent_anchor_rect.max.y + 4.0;
            Some(Rect::from_min_size(pos2(min_x, min_y), vec2(320.0, 400.0)))
        } else {
            None
        };

        let are_tabs_visible = self.misc.show_tabs;
        if are_tabs_visible {
            if self.misc.mode == Mode::Normal {
                self.sync_active_tab();
                let tab_items: Vec<crate::view_editor::TabItem> = self
                    .open_notes
                    .iter()
                    .enumerate()
                    .map(|(idx, note)| crate::view_editor::TabItem {
                        title: &note.title,
                        is_dirty: note.is_dirty,
                        is_active: idx == self.tabs.active_tab,
                    })
                    .collect();

                let active_changed = self.tabs.active_tab != self.tabs.last_active_tab;
                if active_changed {
                    self.tabs.last_active_tab = self.tabs.active_tab;
                }

                if let Some(action) = crate::view_editor::render_tab_bar(
                    ui,
                    painter,
                    tab_bar_rect,
                    &tab_items,
                    &self.misc.theme,
                    &mut self.tabs.tab_scroll_offset,
                    active_changed,
                    tab_occluded_rect,
                ) {
                    match action {
                        crate::view_editor::TabAction::Select(idx) => {
                            self.switch_tab(idx, now);
                        }
                        crate::view_editor::TabAction::Close(idx) => {
                            self.close_tab(idx, now);
                        }
                    }
                }
            } else if self.misc.mode == Mode::Doc {
                let docs = crate::ui::docs::get_docs();
                let tab_items: Vec<crate::view_editor::TabItem> = self
                    .tabs.open_doc_tabs
                    .iter()
                    .enumerate()
                    .map(|(idx, &doc_idx)| {
                        let title = docs.get(doc_idx).map(|d| d.title).unwrap_or("Guide");
                        crate::view_editor::TabItem {
                            title,
                            is_dirty: false,
                            is_active: idx == self.tabs.active_doc_tab,
                        }
                    })
                    .collect();

                let active_doc_changed = self.tabs.active_doc_tab != self.tabs.last_active_doc_tab;
                if active_doc_changed {
                    self.tabs.last_active_doc_tab = self.tabs.active_doc_tab;
                }

                if let Some(action) = crate::view_editor::render_tab_bar(
                    ui,
                    painter,
                    tab_bar_rect,
                    &tab_items,
                    &self.misc.theme,
                    &mut self.tabs.doc_tab_scroll_offset,
                    active_doc_changed,
                    tab_occluded_rect,
                ) {
                    match action {
                        crate::view_editor::TabAction::Select(idx) => {
                            self.switch_doc_tab(idx, now);
                        }
                        crate::view_editor::TabAction::Close(idx) => {
                            self.close_doc_tab(idx, now);
                        }
                    }
                }
            } else if self.misc.mode == Mode::Help {
                let tab_items = [crate::view_editor::TabItem {
                    title: "⚡ Quick Start",
                    is_dirty: false,
                    is_active: true,
                }];

                if let Some(action) = crate::view_editor::render_tab_bar(
                    ui,
                    painter,
                    tab_bar_rect,
                    &tab_items,
                    &self.misc.theme,
                    &mut self.modal.help_tab_scroll_offset,
                    false,
                    tab_occluded_rect,
                ) {
                    match action {
                        crate::view_editor::TabAction::Select(_) => {}
                        crate::view_editor::TabAction::Close(_) => {
                            self.misc.mode = Mode::Normal;
                            self.set_status("Closed Quick Start", now);
                        }
                    }
                }
            }
        }

        // Body area below tab strip (for editor, gutter, preview, help)
        let has_active_tabs = match self.misc.mode {
            Mode::Normal => !self.open_notes.is_empty(),
            Mode::Doc => !self.tabs.open_doc_tabs.is_empty(),
            Mode::Help => true,
            _ => false,
        };

        let body_rect = if are_tabs_visible && has_active_tabs {
            let body_min_y = tab_bar_rect.max.y;
            let body_max_y = top_panel_rect.max.y.max(body_min_y + 30.0);
            Rect::from_min_max(
                pos2(top_panel_rect.min.x, body_min_y),
                pos2(top_panel_rect.max.x, body_max_y),
            )
        } else {
            top_panel_rect
        };

        // Split Editor & Preview panes setup inside body_rect
        let (actual_editor_rect, preview_rect_opt, divider_rect_opt) = if is_preview_active {
            let total_w = top_panel_rect.width();
            let available_w = (total_w - divider_w).max(300.0);
            let min_w = 150.0f32;
            let min_right_w = 150.0f32;
            let max_w = (available_w - min_right_w).max(min_w);
            let left_w = (available_w * self.editor.split_ratio).clamp(min_w, max_w);

            let left_rect = Rect::from_min_max(
                body_rect.min,
                pos2(top_panel_rect.min.x + left_w, body_rect.max.y),
            );
            let divider_rect = Rect::from_min_max(
                pos2(top_panel_rect.min.x + left_w, top_panel_rect.min.y),
                pos2(top_panel_rect.min.x + left_w + divider_w, top_panel_rect.max.y),
            );
            let right_rect = Rect::from_min_max(
                pos2(divider_rect.max.x, top_panel_rect.min.y),
                top_panel_rect.max,
            );
            (left_rect, Some(right_rect), Some(divider_rect))
        } else {
            (body_rect, None, None)
        };
        self.editor.last_editor_rect = Some(actual_editor_rect);

        let modals_open = self.modal.settings_open
            || self.modal.search_open
            || self.modal.help_open
            || self.modal.rename_open
            || self.modal.delete_confirm_open
            || self.misc.accent_dropdown_open
            || self.editor.dragging_splitter
            || self.sidebar.dragging_splitter
            || self.terminal.dragging_splitter;

        let show_dashboard = self.misc.mode == Mode::Normal && (self.misc.show_welcome || self.open_notes.is_empty());

        if (self.misc.mode == Mode::Normal || self.misc.mode == Mode::Doc) && !show_dashboard {
            if self.misc.zoom.handle_input(ui, actual_editor_rect, now, modals_open) {
                if self.misc.zoom.level == 1.0 {
                    self.set_status("Editor zoom reset to 100% (Ctrl+0)", now);
                }
            }
        }

        let (ed_font_size, ed_cw, ed_lh) = self.misc.zoom.editor_metrics(self.misc.font_size, ui.ctx());
        self.editor.last_ed_font_size = Some(ed_font_size);

        // Keep visual lines updated to exact editor width
        let target_ed = if self.misc.mode == Mode::Doc { &self.editor.doc_ed } else { &self.editor.ed };
        let effective_editor_w = actual_editor_rect.width();

        // In Vim mode Neovim owns the editor surface — skip the inline layout
        // computation entirely; it's expensive and serves no purpose here.
        let in_vim_mode = self.services.editor_controller.mode == EditorInputMode::Vim
            && matches!(self.misc.mode, Mode::Normal | Mode::Doc);

        self.editor.visual_lines = if in_vim_mode {
            vec![crate::types::VisualLine { char_start: 0, char_end: target_ed.buf.len() }]
        } else if self.editor.inline_mode {
            let gutter_w = if self.editor.show_line_numbers {
                let total_lines = (target_ed.buf.iter().filter(|&&c| c == '\n').count() + 1).max(1);
                let digits = total_lines.to_string().len().max(2);
                (digits as f32 * (ed_font_size * 0.55) + 14.0).max(28.0)
            } else {
                0.0
            };
            let pad_x = if self.editor.show_line_numbers { 16.0 } else { 24.0 };
            let pad_y = 10.0;
            let safe_w = effective_editor_w;
            let effective_gutter_w = if safe_w > gutter_w + 40.0 { gutter_w } else { 0.0 };
            let text_left = (actual_editor_rect.min.x + effective_gutter_w + pad_x).min(actual_editor_rect.max.x);
            let curr_scroll_y = if self.misc.mode == Mode::Doc { self.editor.doc_scroll_y } else { self.editor.scroll_y };
            let ed_origin = pos2(text_left, actual_editor_rect.min.y - curr_scroll_y + pad_y);
            self.editor.last_ed_origin = Some(ed_origin);

            let wrap_w = (effective_editor_w - effective_gutter_w - pad_x - 24.0).max(120.0);
            let inline_layout = crate::view_editor::inline::compute_inline_layout_ctx(
                ui.ctx(),
                target_ed,
                wrap_w,
                ed_font_size,
                &self.misc.theme,
                text_left,
            );
            inline_layout.compute_visual_lines()
        } else {
            let total_lines = (target_ed.buf.iter().filter(|&&c| c == '\n').count() + 1).max(1);
            let digits = total_lines.to_string().len().max(2);
            let gutter_space = if self.editor.show_line_numbers {
                (digits as f32 * ed_cw + 10.0).max(22.0) + 14.0
            } else {
                22.0
            };
            let text_area_w = (effective_editor_w - gutter_space - 10.0).max(100.0);
            let max_cols = (text_area_w / ed_cw).floor().max(15.0) as usize;
            target_ed.compute_visual_lines(max_cols)
        };

        // Active View rendering delegated to dedicated view modules
        match self.misc.mode {
            Mode::Normal | Mode::Doc => {
                let original_caret_kind = self.misc.caret.kind;
                let active_vim_mode = if self.services.editor_controller.mode == EditorInputMode::Vim {
                    self.services.vim_runtime.backend.as_ref().map(|b| b.grid.mode.as_str())
                } else {
                    None
                };
                self.misc.caret.kind = crate::caret::resolve_caret_kind(
                    self.services.editor_controller.mode,
                    active_vim_mode,
                    original_caret_kind,
                );

                if let (Some(_), Some(divider_rect)) = (preview_rect_opt, divider_rect_opt) {
                    let available_w = (top_panel_rect.width() - divider_w).max(200.0);
                    let panel_top = top_panel_rect.min.y;
                    let panel_bottom = top_panel_rect.max.y;
                    let mid_x = divider_rect.center().x;
                    let knob_mid = pos2(mid_x, (panel_top + panel_bottom) * 0.5);
                    let is_dragging = self.editor.dragging_splitter;
                    let knob_w = if is_dragging { 6.0 } else { 4.0 };
                    let knob_h = 36.0;
                    let knob_rect = Rect::from_center_size(knob_mid, vec2(knob_w, knob_h));
                    let knob_hit_rect = Rect::from_center_size(knob_mid, vec2(16.0, 44.0));

                    let is_knob_hovered = !any_modal_open && ui.rect_contains_pointer(knob_hit_rect);

                    let primary_down = ui.input(|i| i.pointer.primary_down());
                    let primary_pressed = ui.input(|i| i.pointer.primary_clicked() || i.pointer.button_pressed(egui::PointerButton::Primary));

                    if is_knob_hovered && primary_pressed {
                        self.editor.dragging_splitter = true;
                    }

                    if self.editor.dragging_splitter {
                        if primary_down {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeColumn);
                            if let Some(pos) = ui.input(|i| i.pointer.interact_pos().or_else(|| i.pointer.hover_pos())) {
                                let raw_ratio = (pos.x - top_panel_rect.min.x - divider_w * 0.5) / available_w;
                                let min_ratio = (150.0f32 / available_w).min(0.45);
                                let max_ratio = (1.0 - 150.0f32 / available_w).max(min_ratio);
                                self.editor.split_ratio = raw_ratio.clamp(min_ratio, max_ratio);
                            }
                        } else {
                            self.editor.dragging_splitter = false;
                        }
                    } else if is_knob_hovered {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeColumn);
                    }

                    let is_active = is_knob_hovered || self.editor.dragging_splitter;

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
                            self.misc.theme.accent
                        } else {
                            Color32::from_rgba_unmultiplied(self.misc.theme.muted.r(), self.misc.theme.muted.g(), self.misc.theme.muted.b(), 100)
                        },
                    );

                    let grip_color = self.misc.theme.bg;
                    for dy in [-5.0, 0.0, 5.0] {
                        painter.line_segment(
                            [pos2(knob_mid.x - 1.2, knob_mid.y + dy), pos2(knob_mid.x + 1.2, knob_mid.y + dy)],
                            Stroke::new(1.0_f32, grip_color),
                        );
                    }
                } else {
                    self.editor.dragging_splitter = false;
                }

                let (target_ed_mut, target_scroll_y) = if self.misc.mode == Mode::Doc {
                    (&mut self.editor.doc_ed, &mut self.editor.doc_scroll_y)
                } else {
                    (&mut self.editor.ed, &mut self.editor.scroll_y)
                };

                let search_matches: Option<(&[usize], usize)> = None;

                if show_dashboard {
                    if let Some(dash_action) = crate::views::dashboard::render_welcome_dashboard(
                        ui,
                        painter,
                        actual_editor_rect,
                        &self.misc.theme,
                        self.notes.total_notes_count,
                        modals_open,
                    ) {
                        match dash_action {
                            crate::views::dashboard::DashboardAction::NewNote => {
                                self.misc.show_welcome = false;
                                self.create_new_note(now);
                            }
                            crate::views::dashboard::DashboardAction::FindNote => {
                                self.modal.search_open = true;
                                self.modal.search_just_opened = true;
                                self.modal.search_opened_at = now;
                                self.modal.search_query.clear();
                                self.modal.search_selected = 0;
                                self.update_search_results();
                            }
                            crate::views::dashboard::DashboardAction::OpenRecent(id) => {
                                self.misc.show_welcome = false;
                                self.open_note_by_id(id, now);
                            }
                            crate::views::dashboard::DashboardAction::OpenTerminal => {
                                self.terminal.open = true;
                                self.terminal.focused = true;
                            }
                            crate::views::dashboard::DashboardAction::OpenAi => {
                                self.editor.preview_open = true;
                                self.right_pane.tab = crate::app::RightPaneTab::AiAgent;
                                self.services.agent_state.is_open = true;
                            }
                            crate::views::dashboard::DashboardAction::OpenDocs => {
                                self.misc.show_welcome = false;
                                self.open_docs_mode(now);
                            }
                            crate::views::dashboard::DashboardAction::OpenSettings => {
                                self.modal.settings_open = true;
                                self.modal.settings_just_opened = true;
                                self.modal.settings_opened_at = now;
                            }
                            crate::views::dashboard::DashboardAction::ToggleZen => {
                                self.misc.zen_mode = !self.misc.zen_mode;
                                if self.misc.zen_mode {
                                    self.misc.show_titlebar = false;
                                    self.misc.show_tabs = false;
                                    self.sidebar.open = false;
                                    self.editor.preview_open = false;
                                } else {
                                    self.misc.show_titlebar = true;
                                    self.misc.show_tabs = true;
                                }
                                let _ = self.services.db_tx.send(crate::services::db_worker::DbMsg::SaveSetting {
                                    key: "zen_mode".into(),
                                    val: if self.misc.zen_mode { "true" } else { "false" }.into(),
                                });
                            }
                            crate::views::dashboard::DashboardAction::Quit => {
                                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                        }
                    }
                } else if self.services.editor_controller.mode == EditorInputMode::Vim
                    && matches!(self.misc.mode, Mode::Normal | Mode::Doc)
                    && self.services.vim_runtime.backend.is_none()
                    && self.services.vim_runtime.start_error.is_some()
                {
                    ui.vertical_centered(|ui| {
                        if let Some(error) = &self.services.vim_runtime.start_error {
                            ui.colored_label(self.misc.theme.highlight, format!("Neovim could not start: {error}"));
                        }
                    });
                } else if matches!(self.misc.mode, Mode::Normal | Mode::Doc)
                    && self.services.vim_runtime.backend.is_some()
                    && self.services.editor_controller.mode == EditorInputMode::Vim
                {
                    if let Some(backend) = self.services.vim_runtime.backend.as_mut() {
                        backend.render_in_rect(
                            ui,
                            actual_editor_rect,
                            ed_font_size,
                            ed_cw,
                            ed_lh,
                            &self.misc.theme,
                            &mut self.misc.caret,
                            dt,
                            typed,
                            self.editor.show_line_numbers,
                        );
                    }
                } else if self.editor.inline_mode {
                    let mut dummy_dirty = false;
                    let target_dirty = if self.misc.mode == Mode::Doc {
                        &mut dummy_dirty
                    } else {
                        &mut self.editor.is_dirty
                    };
                    render_inline_editor(
                        ui,
                        painter,
                        bounds,
                        actual_editor_rect,
                        target_ed_mut,
                        &mut self.misc.caret,
                        target_scroll_y,
                        &self.misc.theme,
                        ed_font_size,
                        dt,
                        now,
                        typed,
                        any_modal_open
                            || self.editor.dragging_splitter
                            || self.sidebar.dragging_splitter
                            || self.services.hover_wikilink.is_mouse_inside_popup,
                        search_matches,
                        self.editor.show_line_numbers,
                        &mut self.misc.sound,
                        target_dirty,
                    );
                } else {
                    render_editor_body(
                        ui,
                        painter,
                        bounds,
                        actual_editor_rect,
                        target_ed_mut,
                        &self.editor.visual_lines,
                        &mut self.misc.caret,
                        target_scroll_y,
                        &self.misc.theme,
                        ed_font_size,
                        ed_cw,
                        ed_lh,
                        dt,
                        now,
                        typed,
                        any_modal_open
                            || self.editor.dragging_splitter
                            || self.sidebar.dragging_splitter
                            || self.services.hover_wikilink.is_mouse_inside_popup,
                        search_matches,
                        self.editor.show_line_numbers,
                        active_vim_mode,
                    );
                }
                self.misc.caret.kind = original_caret_kind;

                // Center-editor Zoom Percentage HUD
                if !show_dashboard {
                    self.misc.zoom.render_hud(ui, painter, actual_editor_rect, &self.misc.theme, now);
                }

                // Render Right Pane (Markdown Preview, AI Agent, Backlinks, Outline) side-by-side if active
                self.render_right_pane_tabs(ui, painter, preview_rect_opt, any_modal_open, ed_font_size, now);

                // Floating Keystroke Card (shown in both Vim and Hybrid modes)
                if !show_dashboard {
                    let card_anchor = pos2(actual_editor_rect.max.x - 16.0, actual_editor_rect.max.y - 20.0);
                    self.misc.showcmd.render_card(painter, card_anchor, &self.misc.theme, now);
                }

                if ui.rect_contains_pointer(actual_editor_rect) && ui.input(|i| i.pointer.primary_clicked()) {
                    self.terminal.focused = false;
                    ui.memory_mut(|m| m.surrender_focus(egui::Id::new("deepseek_prompt_input")));
                }

                // Render Bottom-Docked Embedded Terminal Drawer
                self.render_terminal_drawer(
                    ui,
                    painter,
                    editor_panel_rect,
                    bottom_terminal_rect,
                    term_splitter_rect_opt,
                    now,
                );
            }
            Mode::Help => {
                let action = crate::ui::help_panel::render_help_tab_view(
                    ui,
                    painter,
                    body_rect,
                    &mut self.modal.help_scroll_y,
                    &self.misc.theme,
                    self.misc.font_size,
                );
                if action.should_close {
                    self.misc.mode = Mode::Normal;
                    self.set_status("Closed Quick Start", now);
                }
            }
            Mode::Stats => {
                self.render_stats_pane(ui, editor_panel_rect);
            }
            Mode::ScanReport | Mode::ScanHistory => {
                self.render_scan_panes(ui, painter, editor_panel_rect);
            }
            Mode::Terminal => {
                if self.terminal.pane.is_none() {
                    self.terminal.pane = crate::ui::terminal_pane::TerminalPane::spawn(ui.ctx(), &self.misc.theme).ok();
                }
                if let Some(ref mut pane) = self.terminal.pane {
                    let action = pane.ui(ui, editor_panel_rect, &self.misc.theme, self.misc.font_size, true, self.misc.opacity);
                    if action == crate::ui::terminal_pane::TerminalAction::Close {
                        self.misc.mode = self.terminal.prev_mode_before_term;
                        self.set_status("Exited terminal", now);
                        ui.ctx().request_repaint();
                    }
                } else {
                    painter.text(
                        editor_panel_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "Failed to initialize terminal session.",
                        FontId::monospace(self.misc.font_size),
                        self.misc.theme.muted,
                    );
                }
            }
        }
    }
}
