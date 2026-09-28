//! Right pane split tab coordinator: Markdown live preview and DeepSeek AI Assistant.

use super::{App, RightPaneTab};
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
            painter.rect_filled(p_rect, 0.0, self.theme.surface());

            let r_header_h = crate::view_editor::TAB_ROW_H;
            let r_header_rect = Rect::from_min_max(
                p_rect.min,
                pos2(p_rect.max.x, p_rect.min.y + r_header_h),
            );
            let r_content_rect = Rect::from_min_max(
                pos2(p_rect.min.x, p_rect.min.y + r_header_h),
                p_rect.max,
            );

            // Backlinks are cached by active note in RightSidebarState. Avoid
            // scanning every Markdown note during every frame.
            let target_key = (self.active_note_title.clone(), self.active_note_id);
            if self.right_sidebar_state.cached_backlinks_target.as_ref() != Some(&target_key) {
                self.right_sidebar_state.cached_backlinks = crate::wikilink::find_backlinks(
                    &self.active_note_title,
                    &self.notes_list,
                    self.active_note_id,
                );
                self.right_sidebar_state.cached_backlinks_target = Some(target_key);
            }

            let header_action = crate::view_editor::preview::render_right_pane_header(
                ui,
                painter,
                r_header_rect,
                self.right_pane_tab,
                &self.theme,
                self.right_sidebar_state.cached_backlinks.len(),
            );
            if let Some(action) = header_action {
                match action {
                    crate::view_editor::preview::RightPaneAction::SelectTab(tab) => {
                        self.right_pane_tab = tab;
                        if tab == RightPaneTab::AiAgent {
                            self.ai_focus_requested = true;
                            self.agent_state.is_open = true;
                        }
                    }
                    crate::view_editor::preview::RightPaneAction::Close => {
                        self.preview_open = false;
                        self.agent_state.is_open = false;
                        let _ = self.db_tx.send(crate::db_worker::DbMsg::SaveSetting {
                            key: "preview".into(),
                            val: "false".into(),
                        });
                    }
                }
            }

            match self.right_pane_tab {
                RightPaneTab::Preview => {
                    let note_text = self.ed.text();
                    render_markdown_preview(
                        ui,
                        painter,
                        r_content_rect,
                        &note_text,
                        &mut self.preview_scroll_y,
                        &self.theme,
                        ed_font_size,
                        any_modal_open
                            || self.is_dragging_splitter
                            || self.is_dragging_sidebar_splitter,
                    );
                }
                RightPaneTab::AiAgent => {
                    self.agent_state.is_open = true;
                    let cur_text = self.ed.text();
                    let active_note_info = if let Some(n) = self.notes_list.iter().find(|n| Some(n.id) == self.active_note_id) {
                        Some((n.topic.as_str(), cur_text.as_str()))
                    } else {
                        None
                    };
                    let req_focus = self.ai_focus_requested;
                    self.ai_focus_requested = false;
                    crate::agent::deepseek_ui::render_ai_pane(
                        ui,
                        painter,
                        r_content_rect,
                        &mut self.agent_state,
                        &self.notes_list,
                        active_note_info,
                        &self.theme,
                        self.font_size,
                        req_focus,
                        any_modal_open
                            || self.is_dragging_splitter
                            || self.is_dragging_sidebar_splitter,
                        self.opacity,
                    );
                }
                RightPaneTab::Backlinks => {
                    let action = crate::rightsidebar::backlinks::render_backlinks_panel(
                        ui,
                        r_content_rect,
                        &self.right_sidebar_state.cached_backlinks,
                        &mut self.right_sidebar_state.backlinks_selected_idx,
                        &self.theme,
                    );
                    if let Some(act) = action {
                        match act {
                            crate::rightsidebar::backlinks::BacklinkAction::OpenNote { id, title } => {
                                if id > 0 {
                                    self.open_note_by_id(id, now);
                                } else if let Some(note) = crate::wikilink::resolve_wikilink(&title, &self.notes_list) {
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
                    self.right_sidebar_state.cached_headings = crate::rightsidebar::outline::extract_outline_headings(&self.ed);
                    let action = crate::rightsidebar::outline::render_outline_panel(
                        ui,
                        r_content_rect,
                        &self.right_sidebar_state.cached_headings,
                        &mut self.right_sidebar_state.outline_selected_idx,
                        &self.theme,
                        self.ed.cur,
                    );
                    if let Some(act) = action {
                        match act {
                            crate::rightsidebar::outline::OutlineAction::JumpToChar(pos) => {
                                let target_pos = pos.min(self.ed.buf.len());
                                self.ed.cur = target_pos;
                                self.ed.desired_col = None;
                                let (row, _) = self.ed.row_col_of(target_pos);
                                let line_h = self.font_size * 1.55;
                                self.scroll_y = (row as f32 * line_h - 40.0).max(0.0);
                            }
                        }
                    }
                }
            }
        }
    }
}
