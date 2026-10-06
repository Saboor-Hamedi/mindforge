//! Modal dialogs and overlay menus (Preferences, Search, Rename, Delete confirmation, Accent picker).

use super::App;
use crate::modals::{render_confirm_modal, render_rename_modal, render_search_modal};
use crate::mode::Mode;
use crate::services::db_worker::DbMsg;
use crate::setting::{render_setting_container, SettingPanelAction};
use eframe::egui::{self, Rect, Ui};

impl App {
    /// Renders open modals (Preferences, Search, Rename, Delete, Accent Dropdown).
    pub fn render_modals(
        &mut self,
        ui: &mut Ui,
        painter: &egui::Painter,
        bounds: Rect,
        accent_anchor_rect: Rect,
        now: f64,
    ) {
        // 1. Two-Column Preferences Modal (Ctrl+,)
        if self.modal.settings_open {
            let db_tx_clone = self.services.db_tx.clone();
            let mut on_save = |key: &str, val: &str| {
                let _ = db_tx_clone.send(DbMsg::SaveSetting {
                    key: key.to_string(),
                    val: val.to_string(),
                });
            };

            let prev_font = self.misc.selected_font.clone();
            let p_action = render_setting_container(
                ui,
                painter,
                bounds,
                &mut self.modal,
                &mut self.misc.caret,
                &mut self.misc.sound,
                &mut self.misc.theme,
                &mut self.misc.selected_font,
                &mut self.misc.font_size,
                &mut self.misc.opacity,
                &mut self.misc.blur_effect,
                &self.services.updater,
                &mut self.services.agent_state.deepseek_api_key_enc,
                &mut self.services.agent_state.deepseek_model,
                &mut self.services.lunaline_config,
                now,
                &mut on_save,
            );

            if self.misc.selected_font != prev_font {
                self.editor.cell = None;
            }

            if let Some(act) = p_action {
                match act {
                    SettingPanelAction::TriggerBackup => {
                        self.trigger_backup(now);
                    }
                    SettingPanelAction::CheckUpdates => {
                        self.services
                            .updater
                            .check_for_updates(env!("CARGO_PKG_VERSION"));
                    }
                    SettingPanelAction::DownloadUpdate => {
                        self.services.updater.start_download();
                    }
                    SettingPanelAction::RestartToApply => {
                        let _ = self.services.updater.restart_and_apply();
                    }
                }
            }
        }

        // 2. Fuzzy Search & Command Palette Modal (Ctrl+P / Ctrl+Shift+P)
        if self.modal.search_open {
            let act = render_search_modal(
                ui,
                painter,
                bounds,
                &mut self.modal.search_query,
                &self.modal.search_results,
                &mut self.modal.search_selected,
                &self.misc.theme,
                self.modal.search_just_opened,
                self.modal.search_opened_at,
                now,
            );
            self.modal.search_just_opened = false;
            if let Some(ref new_q) = act.new_query {
                self.modal.search_query = new_q.clone();
                self.modal.search_selected = 0;
                self.update_search_results();
            }
            if let Some(item) = act.selected_item {
                match item.action {
                    crate::services::fuzzy::PaletteAction::OpenNote(id) => {
                        self.modal.search_open = false;
                        if let Some(n) = self.notes.notes_list.iter().find(|n| n.id == id).cloned()
                        {
                            self.load_note(n.id, n.topic, n.body, now);
                        }
                    }
                    crate::services::fuzzy::PaletteAction::OpenWorkspaceFile(path) => {
                        self.modal.search_open = false;
                        self.open_file_path(path, now);
                    }
                    crate::services::fuzzy::PaletteAction::OpenWorkspaceFolder(path) => {
                        self.modal.search_open = false;
                        if self.workspace.reveal(&path) {
                            self.persist_workspace_expansion();
                        }
                    }
                    crate::services::fuzzy::PaletteAction::ApplyTheme(theme_kind) => {
                        self.misc.theme = crate::ui::theme::Theme::from_kind(theme_kind);
                        self.misc.accent_overrides.apply(&mut self.misc.theme);
                        if let Some(backend) = self.services.vim_runtime.backend.as_mut() {
                            backend.sync_theme(&self.misc.theme);
                        }
                        let _ = self.services.db_tx.send(DbMsg::SaveSetting {
                            key: "theme".into(),
                            val: theme_kind.name().into(),
                        });
                        self.set_status(
                            &format!("Switched to {} theme", theme_kind.display_name()),
                            now,
                        );
                        // Live in-place update so active checkmark updates without moving!
                        self.update_search_results();
                    }
                    crate::services::fuzzy::PaletteAction::OpenThemePicker => {
                        self.modal.search_query = ">theme ".to_string();
                        self.modal.search_selected = 0;
                        self.update_search_results();
                    }
                    crate::services::fuzzy::PaletteAction::OpenSetting(tab) => {
                        self.modal.search_open = false;
                        self.modal.active_setting_tab = tab;
                        self.modal.settings_open = true;
                        self.modal.settings_just_opened = true;
                        self.modal.settings_opened_at = now;
                    }
                    crate::services::fuzzy::PaletteAction::ToggleSidebar => {
                        self.modal.search_open = false;
                        self.sidebar.open = !self.sidebar.open;
                        self.set_status(
                            if self.sidebar.open {
                                "Sidebar opened"
                            } else {
                                "Sidebar closed"
                            },
                            now,
                        );
                    }
                    crate::services::fuzzy::PaletteAction::ToggleRightSidebar => {
                        self.modal.search_open = false;
                        self.editor.preview_open = !self.editor.preview_open;
                        let val = if self.editor.preview_open {
                            "true"
                        } else {
                            "false"
                        };
                        let _ = self.services.db_tx.send(DbMsg::SaveSetting {
                            key: "preview".into(),
                            val: val.into(),
                        });
                        self.set_status(
                            if self.editor.preview_open {
                                "Right Pane opened"
                            } else {
                                "Right Pane closed"
                            },
                            now,
                        );
                    }
                    crate::services::fuzzy::PaletteAction::ToggleBacklinks => {
                        self.modal.search_open = false;
                        if self.editor.preview_open
                            && self.right_pane.tab == crate::app::RightPaneTab::Backlinks
                        {
                            self.editor.preview_open = false;
                            let _ = self.services.db_tx.send(DbMsg::SaveSetting {
                                key: "preview".into(),
                                val: "false".into(),
                            });
                            self.set_status("Backlinks panel closed", now);
                        } else {
                            self.editor.preview_open = true;
                            self.right_pane.tab = crate::app::RightPaneTab::Backlinks;
                            let _ = self.services.db_tx.send(DbMsg::SaveSetting {
                                key: "preview".into(),
                                val: "true".into(),
                            });
                            self.set_status("Backlinks panel opened (Ctrl+I)", now);
                        }
                    }
                    crate::services::fuzzy::PaletteAction::ToggleOutline => {
                        self.modal.search_open = false;
                        if self.editor.preview_open
                            && self.right_pane.tab == crate::app::RightPaneTab::Outline
                        {
                            self.editor.preview_open = false;
                            let _ = self.services.db_tx.send(DbMsg::SaveSetting {
                                key: "preview".into(),
                                val: "false".into(),
                            });
                            self.set_status("Outline panel closed", now);
                        } else {
                            self.editor.preview_open = true;
                            self.right_pane.tab = crate::app::RightPaneTab::Outline;
                            let _ = self.services.db_tx.send(DbMsg::SaveSetting {
                                key: "preview".into(),
                                val: "true".into(),
                            });
                            self.set_status("Outline panel opened (Ctrl+Shift+O)", now);
                        }
                    }
                    crate::services::fuzzy::PaletteAction::TogglePreview => {
                        self.modal.search_open = false;
                        self.editor.preview_open = !self.editor.preview_open;
                        let val = if self.editor.preview_open {
                            "true"
                        } else {
                            "false"
                        };
                        let _ = self.services.db_tx.send(DbMsg::SaveSetting {
                            key: "preview".into(),
                            val: val.into(),
                        });
                        self.set_status(
                            if self.editor.preview_open {
                                "Preview ON"
                            } else {
                                "Preview OFF"
                            },
                            now,
                        );
                    }
                    crate::services::fuzzy::PaletteAction::ToggleAi => {
                        self.modal.search_open = false;
                        self.editor.preview_open = true;
                        self.right_pane.tab = crate::app::RightPaneTab::AiAgent;
                        self.right_pane.ai_focus_requested = true;
                        self.services.agent_state.is_open = true;
                        ui.memory_mut(|m| m.request_focus(egui::Id::new("deepseek_prompt_input")));
                        self.set_status("AI Assistant opened", now);
                    }
                    crate::services::fuzzy::PaletteAction::ToggleTerminal => {
                        self.modal.search_open = false;
                        self.terminal.open = !self.terminal.open;
                        self.set_status(
                            if self.terminal.open {
                                "Terminal docked (:term)"
                            } else {
                                "Terminal closed"
                            },
                            now,
                        );
                    }
                    crate::services::fuzzy::PaletteAction::ToggleZen => {
                        self.modal.search_open = false;
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
                        self.set_status(
                            if self.misc.zen_mode {
                                "Zen Mode ON (Ctrl+.)"
                            } else {
                                "Zen Mode OFF"
                            },
                            now,
                        );
                    }
                    crate::services::fuzzy::PaletteAction::ToggleTitlebar => {
                        self.modal.search_open = false;
                        self.misc.show_titlebar = !self.misc.show_titlebar;
                    }
                    crate::services::fuzzy::PaletteAction::ToggleTabs => {
                        self.modal.search_open = false;
                        self.misc.show_tabs = !self.misc.show_tabs;
                    }
                    crate::services::fuzzy::PaletteAction::NewNote => {
                        self.modal.search_open = false;
                        self.create_new_note(now);
                    }
                    crate::services::fuzzy::PaletteAction::QuickSave => {
                        self.modal.search_open = false;
                        self.quick_save_active_note(now);
                    }
                    crate::services::fuzzy::PaletteAction::RenameNote => {
                        self.modal.search_open = false;
                        self.modal.rename_open = true;
                        self.modal.rename_input = self.notes.active_note_title.clone();
                        self.modal.rename_just_opened = true;
                    }
                    crate::services::fuzzy::PaletteAction::DeleteNote => {
                        self.modal.search_open = false;
                        self.modal.delete_confirm_open = true;
                        self.modal.delete_just_opened = true;
                    }
                    crate::services::fuzzy::PaletteAction::ToggleChecklist => {
                        self.modal.search_open = false;
                        let (target_ed_mut, _) = if self.misc.mode == Mode::Doc {
                            (&mut self.editor.doc_ed, &mut self.editor.doc_scroll_y)
                        } else {
                            (&mut self.editor.ed, &mut self.editor.scroll_y)
                        };
                        if target_ed_mut.toggle_checklist() {
                            self.editor.is_dirty = true;
                            self.misc.sound.play();
                            self.set_status("Toggled checklist item (Ctrl+Shift+X)", now);
                        }
                    }
                    crate::services::fuzzy::PaletteAction::CloseTab => {
                        self.modal.search_open = false;
                        if self.misc.mode == Mode::Doc {
                            self.close_doc_tab(self.tabs.active_doc_tab, now);
                        } else {
                            self.close_tab(self.tabs.active_tab, now);
                        }
                    }
                    crate::services::fuzzy::PaletteAction::ImportWorkspace => {
                        self.modal.search_open = false;
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            self.open_workspace(path, now);
                        }
                    }
                    crate::services::fuzzy::PaletteAction::RunScan => {
                        self.modal.search_open = false;
                        self.command_bar.in_command = true;
                        self.editor.cmd_ed.set_text(":scan ");
                        self.editor.cmd_ed.cur = 6;
                    }
                    crate::services::fuzzy::PaletteAction::ScanHistory => {
                        self.modal.search_open = false;
                        self.misc.mode = Mode::ScanHistory;
                    }
                    crate::services::fuzzy::PaletteAction::OpenHelp => {
                        self.modal.search_open = false;
                        self.tabs.active_doc_idx = 0;
                        self.open_docs_mode(now);
                    }
                    crate::services::fuzzy::PaletteAction::SetLunaStyle(style) => {
                        self.modal.search_open = false;
                        self.services.lunaline_config.style = style;
                        if let Ok(json) = serde_json::to_string(&self.services.lunaline_config) {
                            let _ = self.services.db_tx.send(DbMsg::SaveSetting {
                                key: "lunaline_config".into(),
                                val: json,
                            });
                        }
                        self.set_status(&format!("LunaLine style set to {}", style.name()), now);
                    }
                    crate::services::fuzzy::PaletteAction::SetLunaColor(color_mode) => {
                        self.modal.search_open = false;
                        self.services.lunaline_config.color_mode = color_mode;
                        if let Ok(json) = serde_json::to_string(&self.services.lunaline_config) {
                            let _ = self.services.db_tx.send(DbMsg::SaveSetting {
                                key: "lunaline_config".into(),
                                val: json,
                            });
                        }
                        self.set_status(
                            &format!("LunaLine color set to {}", color_mode.name()),
                            now,
                        );
                    }
                    crate::services::fuzzy::PaletteAction::ShowSoundPicker => {
                        // handled in modals.rs (sets query to >sound), no-op here
                    }
                    crate::services::fuzzy::PaletteAction::ApplySoundProfile(profile) => {
                        // Apply live — keep modal open so user can audition other profiles
                        self.misc.sound.profile = profile;
                        self.misc.sound.play(); // play a key sound so user hears the new profile immediately
                        let _ = self.services.db_tx.send(DbMsg::SaveSetting {
                            key: "sound".into(),
                            val: profile.name().to_lowercase(),
                        });
                        // Refresh results so ✓ Active badge moves to new selection
                        crate::notes::update_search_results(self);
                        self.set_status(&format!("Sound profile: {}", profile.name()), now);
                    }
                    crate::services::fuzzy::PaletteAction::OpenCaretPicker => {
                        self.modal.search_query = ">caret ".to_string();
                        self.modal.search_selected = 0;
                        self.update_search_results();
                    }
                    crate::services::fuzzy::PaletteAction::ApplyCaretKind(kind) => {
                        self.misc.caret.kind = kind;
                        self.misc.caret.last_type = now;
                        let _ = self.services.db_tx.send(DbMsg::SaveSetting {
                            key: "caret".into(),
                            val: kind.name().into(),
                        });
                        crate::notes::update_search_results(self);
                        self.set_status(
                            &format!(
                                "Caret style: {}",
                                crate::ui::palette::caret_display_name(kind)
                            ),
                            now,
                        );
                    }
                    crate::services::fuzzy::PaletteAction::OpenFontPicker => {
                        self.modal.search_query = ">font ".to_string();
                        self.modal.search_selected = 0;
                        self.update_search_results();
                    }
                    crate::services::fuzzy::PaletteAction::ApplyFont(font_name) => {
                        self.misc.selected_font = font_name.clone();
                        crate::services::font_manager::apply_font(
                            ui.ctx(),
                            &self.misc.selected_font,
                        );
                        self.editor.cell = None;
                        let _ = self.services.db_tx.send(DbMsg::SaveSetting {
                            key: "selected_font".into(),
                            val: self.misc.selected_font.clone(),
                        });
                        crate::notes::update_search_results(self);
                        self.set_status(&format!("Font family: {}", self.misc.selected_font), now);
                    }
                }
            }
            if act.should_close {
                self.modal.search_open = false;
            }
        }

        // 3. Rename Document Modal (Ctrl+R)
        if self.modal.rename_open {
            let act = render_rename_modal(
                ui,
                painter,
                bounds,
                &mut self.modal.rename_input,
                &self.misc.theme,
                self.modal.rename_just_opened,
            );
            self.modal.rename_just_opened = false;
            if let Some(new_title) = act.confirmed_title {
                self.modal.rename_open = false;
                self.rename_active_note(&new_title, now);
            }
            if act.should_close {
                self.modal.rename_open = false;
            }
        }

        // 4. Delete Confirmation Modal
        if self.modal.delete_confirm_open {
            let (target_title, is_folder) =
                if let Some(path) = self.modal.pending_delete_path.as_ref() {
                    let name = path
                        .file_name()
                        .unwrap_or(path.as_os_str())
                        .to_string_lossy()
                        .into_owned();
                    (name, path.is_dir())
                } else if let Some(del_id) = self.modal.pending_delete_note_id {
                    let name = self
                        .notes
                        .notes_list
                        .iter()
                        .find(|n| n.id == del_id)
                        .map(|n| n.topic.clone())
                        .unwrap_or_else(|| "this note".to_string());
                    (name, false)
                } else {
                    (self.notes.active_note_title.clone(), false)
                };

            let title = if is_folder {
                "Delete Folder"
            } else {
                "Delete File"
            };
            let message = format!(
                "Are you sure you want to permanently delete \"{}\"?\nThis action cannot be undone.",
                target_title
            );

            let act = render_confirm_modal(
                ui,
                painter,
                bounds,
                title,
                &message,
                "Delete (Enter)",
                &self.misc.theme,
                self.modal.delete_just_opened,
            );
            self.modal.delete_just_opened = false;
            if act.confirmed {
                self.modal.delete_confirm_open = false;
                if let Some(path) = self.modal.pending_delete_path.take() {
                    let to_close: Vec<usize> = self
                        .open_notes
                        .iter()
                        .enumerate()
                        .filter_map(|(idx, t)| {
                            if t.file_path.as_ref().is_some_and(|p| p.starts_with(&path)) {
                                Some(idx)
                            } else {
                                None
                            }
                        })
                        .collect();
                    for idx in to_close.into_iter().rev() {
                        self.close_tab(idx, now);
                    }
                    let file_name_disp = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned();
                    match crate::workspace::delete_path(&path) {
                        Ok(()) => {
                            let _ = self.workspace.refresh();
                            self.set_status(format!("Deleted {}", file_name_disp), now);
                        }
                        Err(e) => {
                            self.set_status(
                                format!("Failed to delete {}: {}", file_name_disp, e),
                                now,
                            );
                        }
                    }
                } else if let Some(del_id) = self.modal.pending_delete_note_id.take() {
                    let _ = self.services.db_tx.send(DbMsg::DeleteNote { id: del_id });
                    self.notes.notes_list.retain(|n| n.id != del_id);
                    if self.notes.active_note_id == Some(del_id) {
                        self.delete_active_note(now);
                    } else {
                        let to_close: Vec<usize> = self
                            .open_notes
                            .iter()
                            .enumerate()
                            .filter_map(|(idx, t)| if t.id == del_id { Some(idx) } else { None })
                            .collect();
                        for idx in to_close.into_iter().rev() {
                            self.close_tab(idx, now);
                        }
                    }
                    self.set_status("Note deleted", now);
                } else {
                    self.delete_active_note(now);
                }
            }
            if act.should_close {
                self.modal.delete_confirm_open = false;
                self.modal.pending_delete_note_id = None;
                self.modal.pending_delete_path = None;
            }
        }

        // 5. Accent Picker Dropdown
        if self.misc.accent_dropdown_open {
            let default_theme = crate::ui::theme::Theme::from_kind(self.misc.theme.kind);
            let action = crate::accent::render_accent_dropdown(
                ui,
                painter,
                accent_anchor_rect,
                &mut self.misc.accent_overrides,
                &self.misc.theme,
                &default_theme,
                &mut self.misc.opacity,
                &mut self.misc.blur_effect,
                &mut |key: &str, val: &str| {
                    let _ =
                        self.services
                            .db_tx
                            .send(crate::services::db_worker::DbMsg::SaveSetting {
                                key: key.into(),
                                val: val.into(),
                            });
                },
            );
            if let Some(act) = action {
                match act {
                    crate::accent::AccentAction::Changed => {
                        self.misc.accent_overrides.apply(&mut self.misc.theme);
                        self.misc
                            .accent_overrides
                            .save_to_settings(&self.services.db_tx);
                        if let Some(backend) = self.services.vim_runtime.backend.as_mut() {
                            backend.sync_theme(&self.misc.theme);
                        }
                        ui.ctx().request_repaint();
                    }
                    crate::accent::AccentAction::ResetAll => {
                        self.misc.accent_overrides.clear();
                        self.misc.theme = crate::ui::theme::Theme::from_kind(self.misc.theme.kind);
                        self.misc
                            .accent_overrides
                            .save_to_settings(&self.services.db_tx);
                        if let Some(backend) = self.services.vim_runtime.backend.as_mut() {
                            backend.sync_theme(&self.misc.theme);
                        }
                        ui.ctx().request_repaint();
                    }
                    crate::accent::AccentAction::Close => {
                        self.misc.accent_dropdown_open = false;
                    }
                }
            }
        }
    }
}
