//! Application startup initialization, settings loading, and session persistence.

use super::{App, EditorInputMode, OpenNote};

/// One restorable tab, stored in order so notes, code files, and untitled tabs reopen together.
#[derive(serde::Serialize, serde::Deserialize)]
enum SavedTab {
    Note(i64),
    File(String),
    Untitled {
        title: String,
        #[serde(default)]
        content: String,
    },
}
use crate::caret::CaretKind;
use crate::editor::Editor;
use crate::services::db_worker::{spawn_db_worker, DbMsg};
use crate::services::sound::SoundProfile;
use crate::setting::SettingTab;
use crate::ui::theme::{Theme, ThemeKind};

pub fn default_backup_dir() -> std::path::PathBuf {
    if let Some(proj) = directories::ProjectDirs::from("com", "mindforge", "mindforge") {
        proj.data_local_dir().join("mindforge_backup")
    } else {
        std::path::PathBuf::from("mindforge_backup")
    }
}

impl App {
    pub fn new() -> Self {
        let tx = spawn_db_worker();

        let mut app = Self {
            workspace: crate::workspace::WorkspaceState::default(),
            file_versions: std::collections::HashMap::new(),
            editor: crate::state::editor::EditorState::new(),
            command_bar: crate::state::command_bar::CommandBarState::new(),
            misc: crate::state::misc::MiscState::new(),
            sidebar: crate::state::sidebar::SidebarState {
                open: false,
                focused: false,
                selected_idx: 0,
                width: crate::layout::DEFAULT_SIDEBAR_W,
                dragging_splitter: false,
                needs_scroll: false,
            },
            modal: crate::state::modal::ModalState {
                settings_open: false,
                settings_just_opened: false,
                settings_opened_at: 0.0,
                active_setting_tab: SettingTab::Carets,
                backup_dir: default_backup_dir().to_string_lossy().to_string(),
                last_backup_status: None,
                keybind_capture: None,
                keymap: crate::setting::keymap::VimKeymap::load_or_init(),
                search_open: false,
                search_query: String::new(),
                search_results: Vec::new(),
                search_selected: 0,
                search_just_opened: false,
                search_opened_at: 0.0,
                rename_open: false,
                rename_input: String::new(),
                rename_just_opened: false,
                delete_confirm_open: false,
                delete_just_opened: false,
                pending_delete_note_id: None,
                pending_delete_path: None,
                help_open: false,
                help_tab: 0,
                help_scroll_y: 0.0,
                help_tab_scroll_offset: 0.0,
            },
            notes: crate::state::notes::NotesState::new(),
            open_notes: Vec::new(),
            tabs: crate::state::tabs::TabsState::new(),
            scroll_y: 0.0,
            doc_scroll_y: 0.0,
            preview_scroll_y: 0.0,
            preview_open: false,
            inline_mode: false,
            split_ratio: 0.5,
            is_dragging_splitter: false,
            show_line_numbers: true,
            is_dirty: false,
            last_saved_time: 0.0,
            services: crate::state::services::ServicesState::new(tx),
            showcmd: crate::ui::showcmd::ShowCmdState::new(true),
            activity: crate::state::activity::ActivityState::new(),
            active_doc_idx: 0,
            doc_sidebar_focused: true,
            doc_selected_idx: 0,
            scan: crate::state::scan::ScanState::new(),
            terminal: crate::state::terminal::TerminalState::new(),
            right_pane: crate::state::right_pane::RightPaneState::new(),
            font_dirty: false,
            last_editor_rect: None,
            last_ed_origin: None,
            last_ed_font_size: None,
        };

        app.load_settings();
        let stored_settings = crate::services::db_worker::get_all_stored_settings();
        if let Some(path) = stored_settings.get("workspace_root") {
            app.open_workspace(std::path::PathBuf::from(path), 0.0);
        }

        // Restore active document and open tabs from settings.json
        let saved_tabs = stored_settings
            .get("open_tabs_v2")
            .and_then(|json| serde_json::from_str::<Vec<SavedTab>>(json).ok());
        if let Some(entries) = saved_tabs {
            for entry in entries {
                match entry {
                    SavedTab::Note(_id) => {}
                    SavedTab::File(path) => {
                        let path = std::path::PathBuf::from(path);
                        let Ok(content) = std::fs::read_to_string(&path) else {
                            continue;
                        };
                        if let Ok(version) = crate::workspace::file_fingerprint(&path) {
                            app.file_versions.insert(path.clone(), version);
                        }
                        let mut file_ed = Editor::new();
                        file_ed.set_text(&content.replace("\r\n", "\n").replace('\r', "\n"));
                        file_ed.clear_history();
                        app.open_notes.push(OpenNote {
                            id: 0,
                            title: path
                                .file_name()
                                .and_then(|name| name.to_str())
                                .unwrap_or("Untitled")
                                .to_string(),
                            editor: file_ed,
                            scroll_y: 0.0,
                            is_dirty: false,
                            file_path: Some(path),
                            language_override: None,
                        });
                    }
                    SavedTab::Untitled { title, content } => {
                        let mut ed = Editor::new();
                        ed.set_text(&content.replace("\r\n", "\n").replace('\r', "\n"));
                        ed.clear_history();
                        app.open_notes.push(OpenNote {
                            id: 0,
                            title,
                            editor: ed,
                            scroll_y: 0.0,
                            is_dirty: false,
                            file_path: None,
                            language_override: None,
                        });
                    }
                }
            }
            if app.open_notes.is_empty() {
                app.misc.show_welcome = true;
            } else {
                app.misc.show_welcome = false;
                app.misc.show_tabs = true;
                let saved_active = stored_settings
                    .get("open_tabs_v2_active")
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap_or(0);
                app.tabs.active_tab = saved_active.min(app.open_notes.len() - 1);
                app.tabs.last_active_tab = app.tabs.active_tab;
                let target = &app.open_notes[app.tabs.active_tab];
                app.notes.active_note_id = (target.id > 0).then_some(target.id);
                app.notes.active_note_title = target.title.clone();
                app.editor.ed = target.editor.clone();
                app.editor.scroll_y = target.scroll_y;
            }
        } else {
            app.misc.show_welcome = true;
        }

        // Restore daily activity and scan history
        let today_str = chrono::Local::now().date_naive().format("%Y-%m-%d").to_string();
        let recent = crate::services::db_worker::get_recent_activity(14);
        if let Some(act) = recent.iter().find(|a| a.date == today_str) {
            app.activity.today_activity = act.clone();
        }
        app.activity.activity_history = recent;
        app.activity.lifetime_activity = crate::services::db_worker::get_lifetime_activity();
        app.scan.past_scans = crate::services::db_worker::list_stored_scans();
        if let Some(json) = stored_settings.get("command_history") {
            if let Ok(hist) = serde_json::from_str::<Vec<String>>(json) {
                app.command_bar.history = hist;
            }
        }

        if app.open_notes.is_empty() {
            app.open_notes.push(OpenNote {
                id: 0,
                title: "Untitled Note".to_string(),
                editor: Editor::new(),
                scroll_y: 0.0,
                is_dirty: false,
                file_path: None,
                language_override: None,
            });
            app.tabs.active_tab = 0;
            app.tabs.last_active_tab = 0;
        }

        app
    }

    pub fn load_settings(&mut self) {
        let settings = crate::services::db_worker::get_all_stored_settings();
        let map: std::collections::BTreeMap<String, String> = settings.into_iter().collect();

        if let Some(c) = map.get("caret") {
            if let Some(kind) = CaretKind::parse(c) {
                self.misc.caret.kind = kind;
            }
        }
        if let Some(t) = map.get("theme") {
            if let Some(kind) = ThemeKind::parse(t) {
                self.misc.theme = Theme::from_kind(kind);
            }
        }
        self.misc.accent_overrides = crate::accent::AccentOverrides::load_from_settings(&map);
        self.misc.accent_overrides.apply(&mut self.misc.theme);
        if let Some(s) = map.get("sound") {
            if let Some(profile) = SoundProfile::parse(s) {
                self.misc.sound.profile = profile;
            }
        }
        if let Some(w) = map.get("caret_width") {
            if let Ok(val) = w.parse::<f32>() {
                self.misc.caret.width = val.clamp(1.0, 10.0);
            }
        }
        if let Some(anim) = map.get("caret_animations") {
            self.misc.caret.animations_enabled = anim != "off" && anim != "false";
        }
        if let Some(blink) = map.get("caret_blinking") {
            self.misc.caret.blink_enabled = blink != "off" && blink != "false";
        }
        if let Some(op) = map.get("opacity") {
            if let Ok(val) = op.parse::<f32>() {
                self.misc.opacity = val.clamp(0.2, 1.0);
            }
        }
        if let Some(sf) = map.get("selected_font") {
            self.misc.selected_font = sf.clone();
        }
        if let Some(bl) = map.get("blur") {
            self.misc.blur_effect = match bl.as_str() {
                "acrylic" => crate::services::blur::BlurEffect::Acrylic,
                "mica" => crate::services::blur::BlurEffect::Mica,
                "none" | "off" => crate::services::blur::BlurEffect::None,
                _ => crate::services::blur::BlurEffect::Acrylic,
            };
        }
        if let Some(zen) = map.get("zen_mode") {
            self.misc.zen_mode = zen == "true" || zen == "on" || zen == "1";
            if self.misc.zen_mode {
                self.misc.show_titlebar = false;
                self.misc.show_tabs = false;
                self.sidebar.open = false;
                self.editor.preview_open = false;
            }
        }
        if let Some(tb) = map.get("show_titlebar") {
            self.misc.show_titlebar = tb == "true" || tb == "1" || tb == "on";
        }
        if let Some(tabs) = map.get("show_tabs") {
            self.misc.show_tabs = tabs == "true" || tabs == "1" || tabs == "on";
        }
        if let Some(f) = map.get("font") {
            if let Ok(val) = f.parse::<f32>() {
                self.misc.font_size = val.clamp(12.0, 48.0);
                self.editor.cell = None;
            }
        }
        if let Some(sw) = map.get("sidebar_w") {
            if let Ok(val) = sw.parse::<f32>() {
                self.sidebar.width =
                    val.clamp(crate::layout::MIN_SIDEBAR_W, crate::layout::MAX_SIDEBAR_W);
            }
        }
        if let Some(b) = map.get("backup_dir") {
            self.modal.backup_dir = b.clone();
        }
        if let Some(m) = map.get("editor_mode") {
            if m == "vim" {
                self.services.editor_controller.mode = EditorInputMode::Vim;
            } else {
                self.services.editor_controller.mode = EditorInputMode::Hybrid;
            }
        }
        if let Some(s) = map.get("showcmd") {
            self.misc.showcmd.enabled = s != "off" && s != "false";
        }
        if let Some(ln) = map.get("line_numbers") {
            self.editor.show_line_numbers = ln != "off" && ln != "false";
        }
        if let Some(p) = map.get("preview") {
            self.editor.preview_open = p == "on" || p == "true";
        }
        self.editor.inline_mode = false;
        let _ = self.services.db_tx.send(DbMsg::SaveSetting {
            key: "inline_mode".into(),
            val: "false".into(),
        });
        if let Some(sb) = map.get("sidebar") {
            self.sidebar.open = sb == "on" || sb == "true";
        }
        if let Some(k) = map.get("deepseek_api_key_enc") {
            self.services.agent_state.deepseek_api_key_enc = k.clone();
        }
        if let Some(m) = map.get("deepseek_model") {
            self.services.agent_state.deepseek_model = m.clone();
        }
        if let Some(json) = map.get("lunaline_config") {
            if let Ok(cfg) = serde_json::from_str::<crate::lunaline::LunaLineConfig>(json) {
                self.services.lunaline_config = cfg;
            }
        }
    }

    pub fn save_active_note_id(&self) {
        let val = self
            .notes
            .active_note_id
            .map(|id| id.to_string())
            .unwrap_or_default();
        let _ = self.services.db_tx.send(DbMsg::SaveSetting {
            key: "last_active_note_id".to_string(),
            val,
        });
        self.save_caret_position();
    }

    pub fn save_caret_position(&self) {
        let _ = self.services.db_tx.send(DbMsg::SaveSetting {
            key: "last_caret_pos".to_string(),
            val: self.editor.ed.cur.to_string(),
        });
        let _ = self.services.db_tx.send(DbMsg::SaveSetting {
            key: "last_scroll_y".to_string(),
            val: self.editor.scroll_y.to_string(),
        });
        if let Some(id) = self.notes.active_note_id {
            let _ = self.services.db_tx.send(DbMsg::SaveSetting {
                key: format!("note_caret_{}", id),
                val: self.editor.ed.cur.to_string(),
            });
            let _ = self.services.db_tx.send(DbMsg::SaveSetting {
                key: format!("note_scroll_{}", id),
                val: self.editor.scroll_y.to_string(),
            });
        }
    }

    pub fn save_open_tabs(&self) {
        let mut saved = Vec::new();
        let mut active = 0;
        for (index, tab) in self.open_notes.iter().enumerate() {
            let is_active = index == self.tabs.active_tab;
            let entry = match &tab.file_path {
                Some(path) => SavedTab::File(path.to_string_lossy().into_owned()),
                None => SavedTab::Untitled {
                    title: tab.title.clone(),
                    content: if is_active {
                        self.editor.ed.text()
                    } else {
                        tab.editor.text()
                    },
                },
            };
            if is_active {
                active = saved.len();
            }
            saved.push(entry);
        }
        if let Ok(json) = serde_json::to_string(&saved) {
            let _ = self.services.db_tx.send(DbMsg::SaveSetting {
                key: "open_tabs_v2".into(),
                val: json,
            });
        }
        let _ = self.services.db_tx.send(DbMsg::SaveSetting {
            key: "open_tabs_v2_active".into(),
            val: active.to_string(),
        });
    }

    pub fn sync_save_session(&mut self) {
        if let Some(tab) = self.open_notes.get_mut(self.tabs.active_tab) {
            tab.editor = self.editor.ed.clone();
            tab.title = self.notes.active_note_title.clone();
            tab.scroll_y = self.editor.scroll_y;
            tab.is_dirty = self.editor.is_dirty;
        }
        for index in 0..self.open_notes.len() {
            let scratch = self
                .open_notes
                .get(index)
                .is_some_and(|tab| tab.file_path.is_none() && tab.id == 0 && tab.is_dirty);
            if scratch {
                if let Err(error) = self.persist_scratch_tab(index, 0.0) {
                    eprintln!("Failed to persist scratch tab: {error}");
                }
            }
        }
        for (index, tab) in self.open_notes.iter_mut().enumerate() {
            if let Some(path) = &tab.file_path {
                let is_active = index == self.tabs.active_tab;
                if tab.is_dirty || (is_active && self.editor.is_dirty) {
                    let expected = self.file_versions.get(path).copied();
                    if !path.exists()
                        || expected.is_some_and(|version| {
                            crate::workspace::file_fingerprint(path)
                                .is_ok_and(|actual| actual != version)
                        })
                    {
                        eprintln!(
                            "Skipped shutdown save for externally changed or deleted file {}",
                            path.display()
                        );
                        continue;
                    }
                    let content = if is_active {
                        self.editor.ed.text()
                    } else {
                        tab.editor.text()
                    };
                    match std::fs::write(path, content) {
                        Ok(()) => {
                            if let Ok(version) = crate::workspace::file_fingerprint(path) {
                                self.file_versions.insert(path.clone(), version);
                            }
                            tab.is_dirty = false;
                            if is_active {
                                self.editor.is_dirty = false;
                            }
                        }
                        Err(error) => {
                            eprintln!("Failed to save {} during shutdown: {error}", path.display())
                        }
                    }
                }
            }
        }
        let _ = self.services.db_tx.send(DbMsg::SaveSetting {
            key: "last_caret_pos".into(),
            val: self.editor.ed.cur.to_string(),
        });
        let _ = self.services.db_tx.send(DbMsg::SaveSetting {
            key: "last_scroll_y".into(),
            val: self.editor.scroll_y.to_string(),
        });
        let _ = self.services.db_tx.send(DbMsg::SaveSetting {
            key: "opacity".into(),
            val: format!("{:.2}", self.misc.opacity),
        });
        let blur_str = match self.misc.blur_effect {
            crate::services::blur::BlurEffect::Acrylic => "acrylic",
            crate::services::blur::BlurEffect::Mica => "mica",
            crate::services::blur::BlurEffect::None => "none",
        };
        let _ = self.services.db_tx.send(DbMsg::SaveSetting {
            key: "blur".into(),
            val: blur_str.into(),
        });
        if let Ok(json) = serde_json::to_string(&self.command_bar.history) {
            let _ = self.services.db_tx.send(DbMsg::SaveSetting {
                key: "command_history".into(),
                val: json,
            });
        }
        self.save_open_tabs();
    }

    pub fn trigger_backup(&mut self, now: f64) {
        let target = if !self.modal.backup_dir.is_empty() {
            let p = std::path::PathBuf::from(&self.modal.backup_dir);
            if p.file_name().and_then(|s| s.to_str()) == Some("mindforge_backup") {
                p
            } else {
                p.join("mindforge_backup")
            }
        } else {
            default_backup_dir()
        };
        let _ = std::fs::create_dir_all(&target);
        let mf_dir = crate::workspace::default_workspace_dir().join(".mindforge");
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
        let backup_dest = target.join(format!("mindforge_settings_{}", timestamp));
        let _ = std::fs::create_dir_all(&backup_dest);
        if let Ok(entries) = std::fs::read_dir(&mf_dir) {
            for entry in entries.flatten() {
                let _ = std::fs::copy(entry.path(), backup_dest.join(entry.file_name()));
            }
        }
        self.modal.last_backup_status = Some(format!(
            "Success ({})",
            chrono::Local::now().format("%H:%M:%S")
        ));
        self.set_status(format!("Backup saved: {}", backup_dest.display()), now);
    }
}
