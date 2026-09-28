//! Modal dialogs and overlay menus (Preferences, Search, Rename, Delete confirmation, Accent picker).

use super::App;
use crate::services::db_worker::DbMsg;
use crate::mode::Mode;
use crate::modals::{render_delete_confirm_modal, render_rename_modal, render_search_modal};
use crate::settings::{render_setting_panel, render_setting_tabs, SettingPanelAction};
use eframe::egui::{self, pos2, Color32, Rect, Stroke, Ui};

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
        if self.settings_open {
            let backdrop_alpha = if self.theme.is_light() { 90 } else { 160 };
            painter.rect_filled(bounds, 0.0, Color32::from_black_alpha(backdrop_alpha));

            let modal_w = (bounds.width() - 80.0).clamp(640.0, 880.0);
            let modal_h = (bounds.height() - 80.0).clamp(480.0, 680.0);
            let modal_rect = Rect::from_center_size(bounds.center(), eframe::egui::vec2(modal_w, modal_h));

            // Surface container
            painter.rect(
                modal_rect,
                5.0,
                self.theme.surface(),
                Stroke::new(1.0_f32, self.theme.border()),
                egui::StrokeKind::Inside,
            );

            // Left tab strip
            let tab_w = 180.0;
            let tabs_rect = Rect::from_min_max(
                modal_rect.min,
                pos2(modal_rect.min.x + tab_w, modal_rect.max.y),
            );
            let panel_rect = Rect::from_min_max(
                pos2(modal_rect.min.x + tab_w, modal_rect.min.y),
                modal_rect.max,
            );

            render_setting_tabs(
                ui,
                painter,
                tabs_rect,
                &mut self.active_setting_tab,
                &self.theme,
            );

            let db_tx_clone = self.db_tx.clone();
            let mut on_save = |key: &str, val: &str| {
                let _ = db_tx_clone.send(DbMsg::SaveSetting {
                    key: key.to_string(),
                    val: val.to_string(),
                });
            };

            let prev_font = self.selected_font.clone();
            let p_action = render_setting_panel(
                ui,
                painter,
                panel_rect,
                self.active_setting_tab,
                &mut self.editor_controller.mode,
                &mut self.caret,
                &mut self.sound,
                &mut self.theme,
                &mut self.backup_dir,
                self.last_backup_status.as_deref(),
                &self.updater,
                &mut self.agent_state.deepseek_api_key_enc,
                &mut self.agent_state.deepseek_model,
                &mut self.keymap,
                &mut self.keybind_capture,
                &mut self.selected_font,
                &mut self.font_size,
                &mut self.opacity,
                &mut self.blur_effect,
                &mut self.lunaline_config,
                &mut on_save,
            );

            if self.selected_font != prev_font {
                self.cell = None;
            }

            if let Some(act) = p_action {
                match act {
                    SettingPanelAction::TriggerBackup => {
                        self.trigger_backup(now);
                    }
                    SettingPanelAction::CheckUpdates => {
                        self.updater.check_for_updates(env!("CARGO_PKG_VERSION"));
                    }
                    SettingPanelAction::DownloadUpdate => {
                        self.updater.start_download();
                    }
                    SettingPanelAction::RestartToApply => {
                        let _ = self.updater.restart_and_apply();
                    }
                }
            }

            // Close preferences on Escape key or outside click (ignoring the click that opened the modal)
            let escape = ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
            let outside_click = !self.settings_just_opened
                && ui.input(|i| i.pointer.primary_clicked())
                && !ui.rect_contains_pointer(modal_rect);
            self.settings_just_opened = false;

            if escape {
                if self.keybind_capture.is_some() {
                    self.keybind_capture = None;
                } else {
                    self.settings_open = false;
                    self.set_status("Preferences closed", now);
                }
            } else if outside_click {
                self.settings_open = false;
                self.set_status("Preferences closed", now);
            }
        }

        // 2. Fuzzy Search & Command Palette Modal (Ctrl+P / Ctrl+Shift+P)
        if self.search_open {
            let act = render_search_modal(
                ui,
                painter,
                bounds,
                &mut self.search_query,
                &self.search_results,
                &mut self.search_selected,
                &self.theme,
                self.search_just_opened,
            );
            self.search_just_opened = false;
            if let Some(ref new_q) = act.new_query {
                self.search_query = new_q.clone();
                self.search_selected = 0;
                self.update_search_results();
            }
            if let Some(item) = act.selected_item {
                match item.action {
                    crate::services::fuzzy::PaletteAction::OpenNote(id) => {
                        self.search_open = false;
                        if let Some(ref db) = self.db {
                            if let Ok(Some(n)) = db.get_note(id) {
                                self.load_note(n.id, n.topic, n.body, now);
                            }
                        }
                    }
                    crate::services::fuzzy::PaletteAction::ApplyTheme(theme_kind) => {
                        self.theme = crate::ui::theme::Theme::from_kind(theme_kind);
                        let _ = self.db_tx.send(DbMsg::SaveSetting {
                            key: "theme".into(),
                            val: theme_kind.name().into(),
                        });
                        self.set_status(&format!("Switched to {} theme", theme_kind.display_name()), now);
                        // Live in-place update so active checkmark updates without moving!
                        self.update_search_results();
                    }
                    crate::services::fuzzy::PaletteAction::OpenThemePicker => {
                        self.search_query = ">theme ".to_string();
                        self.search_selected = 0;
                        self.update_search_results();
                    }
                    crate::services::fuzzy::PaletteAction::OpenSetting(tab) => {
                        self.search_open = false;
                        self.active_setting_tab = tab;
                        self.settings_open = true;
                        self.settings_just_opened = true;
                    }
                    crate::services::fuzzy::PaletteAction::ToggleSidebar => {
                        self.search_open = false;
                        self.sidebar_open = !self.sidebar_open;
                        self.set_status(if self.sidebar_open { "Sidebar opened" } else { "Sidebar closed" }, now);
                    }
                    crate::services::fuzzy::PaletteAction::ToggleRightSidebar => {
                        self.search_open = false;
                        self.preview_open = !self.preview_open;
                        let val = if self.preview_open { "true" } else { "false" };
                        let _ = self.db_tx.send(DbMsg::SaveSetting {
                            key: "preview".into(),
                            val: val.into(),
                        });
                        self.set_status(if self.preview_open { "Right Pane opened" } else { "Right Pane closed" }, now);
                    }
                    crate::services::fuzzy::PaletteAction::ToggleBacklinks => {
                        self.search_open = false;
                        if self.preview_open && self.right_pane_tab == crate::app::RightPaneTab::Backlinks {
                            self.preview_open = false;
                            let _ = self.db_tx.send(DbMsg::SaveSetting {
                                key: "preview".into(),
                                val: "false".into(),
                            });
                            self.set_status("Backlinks panel closed", now);
                        } else {
                            self.preview_open = true;
                            self.right_pane_tab = crate::app::RightPaneTab::Backlinks;
                            let _ = self.db_tx.send(DbMsg::SaveSetting {
                                key: "preview".into(),
                                val: "true".into(),
                            });
                            self.set_status("Backlinks panel opened (Ctrl+I)", now);
                        }
                    }
                    crate::services::fuzzy::PaletteAction::ToggleOutline => {
                        self.search_open = false;
                        if self.preview_open && self.right_pane_tab == crate::app::RightPaneTab::Outline {
                            self.preview_open = false;
                            let _ = self.db_tx.send(DbMsg::SaveSetting {
                                key: "preview".into(),
                                val: "false".into(),
                            });
                            self.set_status("Outline panel closed", now);
                        } else {
                            self.preview_open = true;
                            self.right_pane_tab = crate::app::RightPaneTab::Outline;
                            let _ = self.db_tx.send(DbMsg::SaveSetting {
                                key: "preview".into(),
                                val: "true".into(),
                            });
                            self.set_status("Outline panel opened (Ctrl+Shift+O)", now);
                        }
                    }
                    crate::services::fuzzy::PaletteAction::TogglePreview => {
                        self.search_open = false;
                        self.preview_open = !self.preview_open;
                        let val = if self.preview_open { "true" } else { "false" };
                        let _ = self.db_tx.send(DbMsg::SaveSetting {
                            key: "preview".into(),
                            val: val.into(),
                        });
                        self.set_status(if self.preview_open { "Preview ON" } else { "Preview OFF" }, now);
                    }
                    crate::services::fuzzy::PaletteAction::ToggleAi => {
                        self.search_open = false;
                        self.preview_open = true;
                        self.right_pane_tab = crate::app::RightPaneTab::AiAgent;
                        self.ai_focus_requested = true;
                        self.agent_state.is_open = true;
                        ui.memory_mut(|m| m.request_focus(egui::Id::new("deepseek_prompt_input")));
                        self.set_status("AI Assistant opened", now);
                    }
                    crate::services::fuzzy::PaletteAction::ToggleTerminal => {
                        self.search_open = false;
                        self.terminal_open = !self.terminal_open;
                        self.set_status(if self.terminal_open { "Terminal docked (:term)" } else { "Terminal closed" }, now);
                    }
                    crate::services::fuzzy::PaletteAction::ToggleZen => {
                        self.search_open = false;
                        self.zen_mode = !self.zen_mode;
                        if self.zen_mode {
                            self.show_titlebar = false;
                            self.show_tabs = false;
                            self.sidebar_open = false;
                            self.preview_open = false;
                        } else {
                            self.show_titlebar = true;
                            self.show_tabs = true;
                        }
                        self.set_status(if self.zen_mode { "Zen Mode ON (Ctrl+.)" } else { "Zen Mode OFF" }, now);
                    }
                    crate::services::fuzzy::PaletteAction::ToggleTitlebar => {
                        self.search_open = false;
                        self.show_titlebar = !self.show_titlebar;
                    }
                    crate::services::fuzzy::PaletteAction::ToggleTabs => {
                        self.search_open = false;
                        self.show_tabs = !self.show_tabs;
                    }
                    crate::services::fuzzy::PaletteAction::NewNote => {
                        self.search_open = false;
                        self.create_new_note(now);
                    }
                    crate::services::fuzzy::PaletteAction::QuickSave => {
                        self.search_open = false;
                        self.quick_save_active_note(now);
                    }
                    crate::services::fuzzy::PaletteAction::RenameNote => {
                        self.search_open = false;
                        self.rename_open = true;
                        self.rename_input = self.active_note_title.clone();
                        self.rename_just_opened = true;
                    }
                    crate::services::fuzzy::PaletteAction::DeleteNote => {
                        self.search_open = false;
                        self.delete_confirm_open = true;
                        self.delete_just_opened = true;
                    }
                    crate::services::fuzzy::PaletteAction::ToggleChecklist => {
                        self.search_open = false;
                        let (target_ed_mut, _) = if self.mode == Mode::Doc {
                            (&mut self.doc_ed, &mut self.doc_scroll_y)
                        } else {
                            (&mut self.ed, &mut self.scroll_y)
                        };
                        if target_ed_mut.toggle_checklist() {
                            self.is_dirty = true;
                            self.sound.play();
                            self.set_status("Toggled checklist item (Ctrl+Shift+X)", now);
                        }
                    }
                    crate::services::fuzzy::PaletteAction::CloseTab => {
                        self.search_open = false;
                        if self.mode == Mode::Doc {
                            self.close_doc_tab(self.active_doc_tab, now);
                        } else {
                            self.close_tab(self.active_tab, now);
                        }
                    }
                    crate::services::fuzzy::PaletteAction::ImportWorkspace => {
                        self.search_open = false;
                        self.workspace_importer.is_modal_open = true;
                    }
                    crate::services::fuzzy::PaletteAction::RunScan => {
                        self.search_open = false;
                        self.in_command = true;
                        self.cmd_ed.set_text(":scan ");
                        self.cmd_ed.cur = 6;
                    }
                    crate::services::fuzzy::PaletteAction::ScanHistory => {
                        self.search_open = false;
                        self.mode = Mode::ScanHistory;
                    }
                    crate::services::fuzzy::PaletteAction::OpenHelp => {
                        self.search_open = false;
                        self.mode = Mode::Doc;
                        self.active_doc_idx = 0;
                    }
                    crate::services::fuzzy::PaletteAction::SetLunaStyle(style) => {
                        self.search_open = false;
                        self.lunaline_config.style = style;
                        if let Ok(json) = serde_json::to_string(&self.lunaline_config) {
                            let _ = self.db_tx.send(DbMsg::SaveSetting {
                                key: "lunaline_config".into(),
                                val: json,
                            });
                        }
                        self.set_status(&format!("LunaLine style set to {}", style.name()), now);
                    }
                    crate::services::fuzzy::PaletteAction::SetLunaColor(color_mode) => {
                        self.search_open = false;
                        self.lunaline_config.color_mode = color_mode;
                        if let Ok(json) = serde_json::to_string(&self.lunaline_config) {
                            let _ = self.db_tx.send(DbMsg::SaveSetting {
                                key: "lunaline_config".into(),
                                val: json,
                            });
                        }
                        self.set_status(&format!("LunaLine color set to {}", color_mode.name()), now);
                    }
                    crate::services::fuzzy::PaletteAction::ShowSoundPicker => {
                        // handled in modals.rs (sets query to >sound), no-op here
                    }
                    crate::services::fuzzy::PaletteAction::ApplySoundProfile(profile) => {
                        // Apply live — keep modal open so user can audition other profiles
                        self.sound.profile = profile;
                        self.sound.play(); // play a key sound so user hears the new profile immediately
                        let _ = self.db_tx.send(DbMsg::SaveSetting {
                            key: "sound".into(),
                            val: profile.name().to_lowercase(),
                        });
                        // Refresh results so ✓ Active badge moves to new selection
                        crate::notes::update_search_results(self);
                        self.set_status(&format!("Sound profile: {}", profile.name()), now);
                    }
                    crate::services::fuzzy::PaletteAction::OpenCaretPicker => {
                        self.search_query = ">caret ".to_string();
                        self.search_selected = 0;
                        self.update_search_results();
                    }
                    crate::services::fuzzy::PaletteAction::ApplyCaretKind(kind) => {
                        self.caret.kind = kind;
                        self.caret.last_type = now;
                        let _ = self.db_tx.send(DbMsg::SaveSetting {
                            key: "caret".into(),
                            val: kind.name().into(),
                        });
                        crate::notes::update_search_results(self);
                        self.set_status(&format!("Caret style: {}", crate::ui::palette::caret_display_name(kind)), now);
                    }
                    crate::services::fuzzy::PaletteAction::OpenFontPicker => {
                        self.search_query = ">font ".to_string();
                        self.search_selected = 0;
                        self.update_search_results();
                    }
                    crate::services::fuzzy::PaletteAction::ApplyFont(font_name) => {
                        self.selected_font = font_name.clone();
                        crate::services::font_manager::apply_font(ui.ctx(), &self.selected_font);
                        self.cell = None;
                        let _ = self.db_tx.send(DbMsg::SaveSetting {
                            key: "selected_font".into(),
                            val: self.selected_font.clone(),
                        });
                        crate::notes::update_search_results(self);
                        self.set_status(&format!("Font family: {}", self.selected_font), now);
                    }
                    crate::services::fuzzy::PaletteAction::OpenModePicker => {
                        self.search_query = ">mode ".to_string();
                        self.search_selected = 0;
                        self.update_search_results();
                    }
                    crate::services::fuzzy::PaletteAction::ApplyEditorMode(mode) => {
                        self.editor_controller.mode = mode;
                        self.vim_runtime.start_error = None;
                        let mode_str = match mode {
                            crate::app::EditorInputMode::Vim => "vim",
                            crate::app::EditorInputMode::Hybrid => "hybrid",
                        };
                        let _ = self.db_tx.send(DbMsg::SaveSetting {
                            key: "editor_mode".into(),
                            val: mode_str.into(),
                        });
                        crate::notes::update_search_results(self);
                        self.set_status(&format!("Editor mode: {}", if mode == crate::app::EditorInputMode::Vim { "Vim" } else { "Hybrid" }), now);
                    }
                }
            }
            if act.should_close {
                self.search_open = false;
            }
        }

        // 3. Rename Document Modal (Ctrl+R)
        if self.rename_open {
            let act = render_rename_modal(
                ui,
                painter,
                bounds,
                &mut self.rename_input,
                &self.theme,
                self.rename_just_opened,
            );
            self.rename_just_opened = false;
            if let Some(new_title) = act.confirmed_title {
                self.rename_open = false;
                self.rename_active_note(&new_title, now);
            }
            if act.should_close {
                self.rename_open = false;
            }
        }

        // 4. Delete Confirmation Modal
        if self.delete_confirm_open {
            let target_title = if let Some(del_id) = self.pending_delete_note_id {
                self.notes_list.iter().find(|n| n.id == del_id).map(|n| n.topic.as_str()).unwrap_or("this note")
            } else {
                &self.active_note_title
            };
            let act = render_delete_confirm_modal(
                ui,
                painter,
                bounds,
                target_title,
                &self.theme,
                self.delete_just_opened,
            );
            self.delete_just_opened = false;
            if act.confirmed {
                self.delete_confirm_open = false;
                if let Some(del_id) = self.pending_delete_note_id.take() {
                    let _ = self.db_tx.send(DbMsg::DeleteNote { id: del_id });
                    self.notes_list.retain(|n| n.id != del_id);
                    if self.active_note_id == Some(del_id) {
                        self.delete_active_note(now);
                    } else {
                        self.open_notes.retain(|t| t.id != del_id);
                        if self.active_tab >= self.open_notes.len() && !self.open_notes.is_empty() {
                            self.active_tab = self.open_notes.len() - 1;
                        }
                        self.save_open_tabs();
                    }
                    self.set_status("Note deleted", now);
                } else {
                    self.delete_active_note(now);
                }
            }
            if act.should_close {
                self.delete_confirm_open = false;
                self.pending_delete_note_id = None;
            }
        }

        // 5. Accent Picker Dropdown
        if self.accent_dropdown_open {
            let default_theme = crate::ui::theme::Theme::from_kind(self.theme.kind);
            let action = crate::accent::render_accent_dropdown(
                ui,
                painter,
                accent_anchor_rect,
                &mut self.accent_overrides,
                &self.theme,
                &default_theme,
                &mut self.opacity,
                &mut self.blur_effect,
                &mut |key: &str, val: &str| {
                    let _ = self.db_tx.send(crate::services::db_worker::DbMsg::SaveSetting {
                        key: key.into(),
                        val: val.into(),
                    });
                },
            );
            if let Some(act) = action {
                match act {
                    crate::accent::AccentAction::Changed => {
                        self.accent_overrides.apply(&mut self.theme);
                        if let Some(ref db) = self.db {
                            let _ = self.accent_overrides.save_to_db(db);
                        }
                    }
                    crate::accent::AccentAction::ResetAll => {
                        self.accent_overrides.clear();
                        self.theme = crate::ui::theme::Theme::from_kind(self.theme.kind);
                        if let Some(ref db) = self.db {
                            let _ = self.accent_overrides.save_to_db(db);
                        }
                    }
                    crate::accent::AccentAction::Close => {
                        self.accent_dropdown_open = false;
                    }
                }
            }
        }

        // 6. Workspace / Obsidian Vault Import Modal
        if self.workspace_importer.is_modal_open {
            let act = crate::workspace_import::render_import_modal(
                ui,
                painter,
                bounds,
                &mut self.workspace_importer,
                &self.theme,
            );
            if matches!(act, crate::workspace_import::ImportModalAction::RefreshNotes) {
                self.reload_db_state();
            }
        }
    }
}
