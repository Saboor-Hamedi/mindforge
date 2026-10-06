//! Right pane split tab coordinator: Markdown live preview and DeepSeek AI Assistant.

use super::{App, RightPaneTab};
use crate::mode::Mode;
use crate::view_editor::render_markdown_preview;
use eframe::egui::{pos2, Painter, Rect, Ui};

impl App {
    /// Renders the right split pane tabs: Markdown Preview, DeepSeek AI Assistant, Backlinks, or Outline.
    pub fn render_right_pane_tabs(
        &mut self,
        ui: &mut Ui,
        painter: &Painter,
        preview_rect_opt: Option<Rect>,
        any_modal_open: bool,
        ed_font_size: f32,
        now: f64,
    ) {
        if let Some(p_rect) = preview_rect_opt {
            painter.rect_filled(p_rect, 0.0, self.misc.theme.surface());

            let r_header_h = crate::view_editor::TAB_ROW_H;
            let r_header_rect =
                Rect::from_min_max(p_rect.min, pos2(p_rect.max.x, p_rect.min.y + r_header_h));
            let r_content_rect =
                Rect::from_min_max(pos2(p_rect.min.x, p_rect.min.y + r_header_h), p_rect.max);

            // Backlinks are cached by active note in RightSidebarState. Avoid
            // scanning every Markdown note during every frame.
            let target_key = (
                self.notes.active_note_title.clone(),
                self.notes.active_note_id,
            );
            if self
                .right_pane
                .sidebar_state
                .cached_backlinks_target
                .as_ref()
                != Some(&target_key)
            {
                self.right_pane.sidebar_state.cached_backlinks = crate::wikilink::find_backlinks(
                    &self.notes.active_note_title,
                    &self.notes.notes_list,
                    self.notes.active_note_id,
                );
                self.right_pane.sidebar_state.cached_backlinks_target = Some(target_key);
            }

            let header_action = crate::view_editor::preview::render_right_pane_header(
                ui,
                painter,
                r_header_rect,
                self.right_pane.tab,
                &self.misc.theme,
                self.right_pane.sidebar_state.cached_backlinks.len(),
            );
            if let Some(action) = header_action {
                match action {
                    crate::view_editor::preview::RightPaneAction::SelectTab(tab) => {
                        self.right_pane.tab = tab;
                        if tab == RightPaneTab::AiAgent {
                            self.right_pane.ai_focus_requested = true;
                            self.services.agent_state.is_open = true;
                        }
                    }
                    crate::view_editor::preview::RightPaneAction::Close => {
                        self.editor.preview_open = false;
                        self.services.agent_state.is_open = false;
                        let _ = self.services.db_tx.send(
                            crate::services::db_worker::DbMsg::SaveSetting {
                                key: "preview".into(),
                                val: "false".into(),
                            },
                        );
                    }
                }
            }

            match self.right_pane.tab {
                RightPaneTab::Preview => {
                    let is_markdown = self.active_language()
                        == crate::language::FileLanguage::Markdown
                        || self.misc.mode == Mode::Doc;
                    if is_markdown {
                        let note_text = if self.misc.mode == Mode::Doc {
                            self.editor.doc_ed.text()
                        } else {
                            self.editor.ed.text()
                        };
                        render_markdown_preview(
                            ui,
                            painter,
                            r_content_rect,
                            &note_text,
                            &mut self.editor.preview_scroll_y,
                            &self.misc.theme,
                            ed_font_size,
                            any_modal_open
                                || self.editor.dragging_splitter
                                || self.sidebar.dragging_splitter,
                        );
                    } else {
                        ui.allocate_new_ui(
                            eframe::egui::UiBuilder::new()
                                .max_rect(r_content_rect)
                                .layout(eframe::egui::Layout::centered_and_justified(
                                    eframe::egui::Direction::TopDown,
                                )),
                            |ui| {
                                ui.label(
                                    eframe::egui::RichText::new(
                                        "Markdown preview is available for Markdown files",
                                    )
                                    .color(self.misc.theme.muted)
                                    .size(12.5),
                                );
                            },
                        );
                    }
                }
                RightPaneTab::AiAgent => {
                    self.services.agent_state.is_open = true;
                    let cur_text = if self.misc.mode == Mode::Doc {
                        self.editor.doc_ed.text()
                    } else {
                        self.editor.ed.text()
                    };
                    let active_note_info = if self.misc.mode == Mode::Doc {
                        let doc_title = crate::ui::docs::BRAIN_DOCS
                            .get(self.tabs.active_doc_idx)
                            .map(|d| d.title)
                            .unwrap_or("Documentation");
                        Some((doc_title, cur_text.as_str()))
                    } else if let Some(n) = self
                        .notes
                        .notes_list
                        .iter()
                        .find(|n| Some(n.id) == self.notes.active_note_id)
                    {
                        Some((n.topic.as_str(), cur_text.as_str()))
                    } else {
                        None
                    };
                    let req_focus = self.right_pane.ai_focus_requested;
                    self.right_pane.ai_focus_requested = false;
                    crate::agent::deepseek_ui::render_ai_pane(
                        ui,
                        painter,
                        r_content_rect,
                        &mut self.services.agent_state,
                        &self.notes.notes_list,
                        active_note_info,
                        &self.misc.theme,
                        self.misc.font_size,
                        req_focus,
                        any_modal_open
                            || self.editor.dragging_splitter
                            || self.sidebar.dragging_splitter,
                        self.misc.opacity,
                    );
                }
                RightPaneTab::Backlinks => {
                    let action = crate::rightsidebar::backlinks::render_backlinks_panel(
                        ui,
                        r_content_rect,
                        &self.right_pane.sidebar_state.cached_backlinks,
                        &mut self.right_pane.sidebar_state.backlinks_selected_idx,
                        &self.misc.theme,
                    );
                    if let Some(act) = action {
                        match act {
                            crate::rightsidebar::backlinks::BacklinkAction::OpenNote {
                                id,
                                title,
                            } => {
                                if id > 0 {
                                    self.open_note_by_id(id, now);
                                } else if let Some(note) = crate::wikilink::resolve_wikilink(
                                    &title,
                                    &self.notes.notes_list,
                                ) {
                                    self.open_note_by_id(note.id, now);
                                } else {
                                    self.create_new_note(now);
                                    crate::notes::rename_active_note(self, &title, now);
                                }
                            }
                        }
                    }
                }
                RightPaneTab::Outline => {
                    self.right_pane.sidebar_state.cached_headings =
                        crate::rightsidebar::outline::extract_outline_headings(&self.editor.ed);
                    let action = crate::rightsidebar::outline::render_outline_panel(
                        ui,
                        r_content_rect,
                        &self.right_pane.sidebar_state.cached_headings,
                        &mut self.right_pane.sidebar_state.outline_selected_idx,
                        &self.misc.theme,
                        self.editor.ed.cur,
                    );
                    if let Some(act) = action {
                        match act {
                            crate::rightsidebar::outline::OutlineAction::JumpToChar(pos) => {
                                let target_pos = pos.min(self.editor.ed.buf.len());
                                self.editor.ed.cur = target_pos;
                                self.editor.ed.desired_col = None;
                                let (row, _) = self.editor.ed.row_col_of(target_pos);
                                let line_h = self.misc.font_size * 1.55;
                                self.editor.scroll_y = (row as f32 * line_h - 40.0).max(0.0);
                            }
                        }
                    }
                }
            }
        }
    }
}
