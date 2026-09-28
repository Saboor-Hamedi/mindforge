//! Application startup initialization, settings loading, and session persistence.

use super::{App, EditorInputMode, OpenNote, RightPaneTab};
use crate::caret::{Caret, CaretKind};
use crate::db_worker::{spawn_db_worker, DbMsg};
use crate::editor::{Editor, VisualLine};
use crate::hybrid::HybridEngine;
use crate::mode::Mode;
use crate::settings::SettingTab;
use crate::sound::{SoundEngine, SoundProfile};
use crate::theme::{Theme, ThemeKind};
use crate::updater::UpdateManager;

use chrono::Local;
use core::{DailyActivity, Database};

pub fn default_backup_dir() -> std::path::PathBuf {
    if let Some(proj) = directories::ProjectDirs::from("com", "mindforge", "mindforge") {
        proj.data_local_dir().join("mindforge_backup")
    } else {
        std::path::PathBuf::from("mindforge_backup")
    }
}

impl App {
    pub fn new() -> Self {
        // Open SQLite connection on the main thread first to safely apply any pending migrations
        let db = Database::open_default().ok();
        let tx = spawn_db_worker();

        let mut app = Self {
            ed: Editor::new(),
            doc_ed: Editor::new(),
            cmd_ed: Editor::new(),
            in_command: false,
            cmd_selected_idx: 0,
            cmd_navigated: false,
            command_history: Vec::new(),
            caret: Caret::new(CaretKind::Beam),
            theme: Theme::from_kind(ThemeKind::Green),
            sound: SoundEngine::new(SoundProfile::Thocky),
            font_size: 16.0,
            selected_font: "JetBrains Mono".to_string(),
            blur_effect: crate::blur::BlurEffect::Acrylic,
            show_welcome: false,
            zoom: crate::zoom::ZoomState::new(),
            right_pane_tab: RightPaneTab::Preview,
            ai_focus_requested: false,
            opacity: 1.0,
            last_char_time: -10.0,
            cell: None,
            visual_lines: vec![VisualLine { char_start: 0, char_end: 0 }],
            mode: Mode::Normal,
            status_msg: String::new(),
            status_time: 0.0,
            first_frame: true,
            sidebar_open: false,
            sidebar_focused: false,
            sidebar_selected_idx: 0,
            sidebar_width: crate::layout::DEFAULT_SIDEBAR_W,
            is_dragging_sidebar_splitter: false,
            sidebar_needs_scroll: false,
            settings_open: false,
            settings_just_opened: false,
            active_setting_tab: SettingTab::Carets,
            backup_dir: default_backup_dir().to_string_lossy().to_string(),
            last_backup_status: None,
            keybind_capture: None,
            keymap: crate::settings::keymap::VimKeymap::load_or_init(),
            search_open: false,
            search_query: String::new(),
            search_results: Vec::new(),
            search_selected: 0,
            search_just_opened: false,
            rename_open: false,
            rename_input: String::new(),
            rename_just_opened: false,
            delete_confirm_open: false,
            delete_just_opened: false,
            pending_delete_note_id: None,
            help_open: false,
            help_tab: 0,
            help_scroll_y: 0.0,
            help_tab_scroll_offset: 0.0,
            active_note_id: None,
            active_note_title: "Untitled Note".to_string(),
            notes_list: Vec::new(),
            sidebar_notes_limit: 50,
            total_notes_count: 0,
            open_notes: Vec::new(),
            active_tab: 0,
            last_active_tab: 0,
            open_doc_tabs: vec![0],
            active_doc_tab: 0,
            last_active_doc_tab: 0,
            tab_scroll_offset: 0.0,
            doc_tab_scroll_offset: 0.0,
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
            db,
            db_tx: tx,
            // Neovim is the default editing engine; Hybrid remains available
            // through the editor mode command/settings.
            editor_controller: crate::editor::controller::EditorController::new(EditorInputMode::Hybrid),
            vim_runtime: crate::vim::VimRuntime::default(),
            hybrid: HybridEngine::new(),
            showcmd: crate::showcmd::ShowCmdState::new(true),
            pending_secs: 0.0,
            pending_keys: 0,
            pending_words: 0,
            pending_created: 0,
            pending_edited: 0,
            last_flush_time: 0.0,
            today_activity: DailyActivity::default(),
            activity_history: Vec::new(),
            lifetime_activity: (0, 0, 0, 0),
            updater: UpdateManager::new(),
            active_doc_idx: 0,
            doc_sidebar_focused: true,
            doc_selected_idx: 0,
            prev_mode_before_scan: Mode::Normal,
            scan_in_progress: None,
            scan_rx: None,
            active_scan_result: None,
            active_scan_error: None,
            scan_report_scroll_y: 0.0,
            past_scans: Vec::new(),
            scan_history_selected: 0,
            scan_history_scroll_y: 0.0,
            accent_dropdown_open: false,
            accent_overrides: crate::accent::AccentOverrides::default(),
            terminal_open: false,
            terminal_split_ratio: 0.35,
            is_dragging_terminal_splitter: false,
            term_pane: None,
            terminal_focused: false,
            prev_mode_before_term: Mode::Normal,
            clipboard_text: None,
            agent_state: crate::agent::AgentState::new(),
            zen_mode: false,
            show_titlebar: true,
            show_tabs: true,
            workspace_importer: crate::workspace_import::WorkspaceImporter::new(),
            lunaline_config: crate::lunaline::LunaLineConfig::default(),
            right_sidebar_open: false,
            right_sidebar_width: 240.0,
            right_sidebar_state: crate::rightsidebar::RightSidebarState::default(),
            hover_wikilink: crate::wikilink::hover_wikilink::HoverWikiLinkState::default(),
            wikilink_autocomplete: crate::wikilink::wikilink_autocompletion::WikiLinkAutocompleteState::default(),
            font_dirty: false,
            last_editor_rect: None,
            last_ed_origin: None,
            last_ed_font_size: None,
        };

        app.load_settings();

        // Restore active document and open tabs from SQLite
        if let Some(ref db) = app.db {
            let limit = app.sidebar_notes_limit;
            if let Ok(notes) = db.get_recent_notes(limit) {
                app.notes_list = notes;
            }
            if let Ok(count) = db.get_notes_count() {
                app.total_notes_count = count;
            }

            let mut tabs_setting_present = false;
            let mut tabs_restored = false;
            if let Ok(Some(json)) = db.get_setting("open_note_tab_ids") {
                tabs_setting_present = true;
                if let Ok(ids) = serde_json::from_str::<Vec<i64>>(&json) {
                    for id in ids {
                        if let Ok(Some(note)) = db.get_note(id) {
                            let mut note_ed = Editor::new();
                            note_ed.insert_str(&note.body);
                            note_ed.cur = 0;
                            note_ed.clear_history();
                            let s_y = db.get_setting(&format!("note_scroll_{}", id))
                                .ok()
                                .flatten()
                                .and_then(|s| s.parse::<f32>().ok())
                                .unwrap_or(0.0);
                            app.open_notes.push(OpenNote {
                                id: note.id,
                                title: note.topic.clone(),
                                editor: note_ed,
                                scroll_y: s_y,
                                is_dirty: false,
                            });
                        }
                    }
                    if !app.open_notes.is_empty() {
                        tabs_restored = true;
                        app.show_welcome = false;
                        let saved_active = db.get_setting("open_note_active_tab")
                            .ok()
                            .flatten()
                            .and_then(|s| s.parse::<usize>().ok())
                            .unwrap_or(0);
                        app.active_tab = saved_active.min(app.open_notes.len() - 1);
                        app.last_active_tab = app.active_tab;
                        let target = &app.open_notes[app.active_tab];
                        app.active_note_id = Some(target.id);
                        app.active_note_title = target.title.clone();
                        app.ed = target.editor.clone();
                        app.scroll_y = target.scroll_y;
                        if let Ok(Some(c_str)) = db.get_setting(&format!("note_caret_{}", target.id)) {
                            if let Ok(c) = c_str.parse::<usize>() {
                                app.ed.cur = c.min(app.ed.buf.len());
                            }
                        }
                    } else {
                        // User explicitly closed all tabs in previous session
                        app.show_welcome = true;
                    }
                }
            }

            if !tabs_setting_present && !tabs_restored {
                let last_id = db.get_setting("last_active_note_id").ok().flatten().and_then(|s| s.parse::<i64>().ok());
                let note = match last_id {
                    Some(id) => db.get_note(id).ok().flatten(),
                    None => db.get_all_notes().ok().and_then(|l| l.into_iter().next()),
                };

                if let Some(n) = note {
                    app.active_note_id = Some(n.id);
                    app.active_note_title = n.topic.clone();
                    app.ed.insert_str(&n.body);
                    app.ed.cur = 0;
                    app.ed.clear_history();

                    if let Ok(Some(c_str)) = db.get_setting(&format!("note_caret_{}", n.id)) {
                        if let Ok(c) = c_str.parse::<usize>() {
                            app.ed.cur = c.min(app.ed.buf.len());
                        }
                    }
                    if let Ok(Some(s_str)) = db.get_setting(&format!("note_scroll_{}", n.id)) {
                        if let Ok(s) = s_str.parse::<f32>() {
                            app.scroll_y = s;
                        }
                    }

                    app.open_notes.push(OpenNote {
                        id: n.id,
                        title: n.topic,
                        editor: app.ed.clone(),
                        scroll_y: app.scroll_y,
                        is_dirty: false,
                    });
                    app.active_tab = 0;
                    app.last_active_tab = 0;
                    app.show_welcome = false;
                } else {
                    app.show_welcome = true;
                }
            }

            // Restore daily activity
            let today_str = Local::now().date_naive().format("%Y-%m-%d").to_string();
            if let Ok(recent) = db.get_recent_activity(14) {
                if let Some(act) = recent.iter().find(|a| a.date == today_str) {
                    app.today_activity = act.clone();
                }
                app.activity_history = recent;
            }
            if let Ok(life) = db.get_lifetime_activity() {
                app.lifetime_activity = life;
            }
            if let Ok(scans) = db.list_scans() {
                app.past_scans = scans;
            }
            if let Ok(Some(json)) = db.get_setting("command_history") {
                if let Ok(hist) = serde_json::from_str::<Vec<String>>(&json) {
                    app.command_history = hist;
                }
            }
        } else {
            app.open_notes.push(OpenNote {
                id: 0,
                title: "Untitled Note".to_string(),
                editor: Editor::new(),
                scroll_y: 0.0,
                is_dirty: false,
            });
            app.active_tab = 0;
            app.last_active_tab = 0;
        }

        app
    }

    pub fn load_settings(&mut self) {
        if let Some(db) = &self.db {
            if let Ok(Some(c)) = db.get_setting("caret") {
                if let Some(kind) = CaretKind::parse(&c) {
                    self.caret.kind = kind;
                }
            }
            if let Ok(Some(t)) = db.get_setting("theme") {
                if let Some(kind) = ThemeKind::parse(&t) {
                    self.theme = Theme::from_kind(kind);
                }
            }
            self.accent_overrides = crate::accent::AccentOverrides::load_from_db(db);
            self.accent_overrides.apply(&mut self.theme);
            if let Ok(Some(s)) = db.get_setting("sound") {
                if let Some(profile) = SoundProfile::parse(&s) {
                    self.sound.profile = profile;
                }
            }
            if let Ok(Some(w)) = db.get_setting("caret_width") {
                if let Ok(val) = w.parse::<f32>() {
                    self.caret.width = val.clamp(1.0, 10.0);
                }
            }
            if let Ok(Some(anim)) = db.get_setting("caret_animations") {
                self.caret.animations_enabled = anim != "off" && anim != "false";
            }
            if let Ok(Some(blink)) = db.get_setting("caret_blinking") {
                self.caret.blink_enabled = blink != "off" && blink != "false";
            }
            if let Ok(Some(op)) = db.get_setting("opacity") {
                if let Ok(val) = op.parse::<f32>() {
                    self.opacity = val.clamp(0.2, 1.0);
                }
            }
            if let Ok(Some(sf)) = db.get_setting("selected_font") {
                self.selected_font = sf;
            }
            if let Ok(Some(bl)) = db.get_setting("blur") {
                self.blur_effect = match bl.as_str() {
                    "acrylic" => crate::blur::BlurEffect::Acrylic,
                    "mica" => crate::blur::BlurEffect::Mica,
                    "none" | "off" => crate::blur::BlurEffect::None,
                    _ => crate::blur::BlurEffect::Acrylic,
                };
            }
            if let Ok(Some(zen)) = db.get_setting("zen_mode") {
                self.zen_mode = zen == "true" || zen == "on" || zen == "1";
                if self.zen_mode {
                    self.show_titlebar = false;
                    self.show_tabs = false;
                    self.sidebar_open = false;
                    self.preview_open = false;
                }
            }
            if let Ok(Some(tb)) = db.get_setting("show_titlebar") {
                self.show_titlebar = tb == "true" || tb == "1" || tb == "on";
            }
            if let Ok(Some(tabs)) = db.get_setting("show_tabs") {
                self.show_tabs = tabs == "true" || tabs == "1" || tabs == "on";
            }
            if let Ok(Some(f)) = db.get_setting("font") {
                if let Ok(val) = f.parse::<f32>() {
                    self.font_size = val.clamp(12.0, 48.0);
                    self.cell = None;
                }
            }
            if let Ok(Some(sw)) = db.get_setting("sidebar_w") {
                if let Ok(val) = sw.parse::<f32>() {
                    self.sidebar_width = val.clamp(crate::layout::MIN_SIDEBAR_W, crate::layout::MAX_SIDEBAR_W);
                }
            }
            if let Ok(Some(b)) = db.get_setting("backup_dir") {
                self.backup_dir = b;
            }
            if let Ok(Some(m)) = db.get_setting("editor_mode") {
                if m == "vim" {
                    self.editor_controller.mode = EditorInputMode::Vim;
                } else {
                    self.editor_controller.mode = EditorInputMode::Hybrid;
                }
            }
            if let Ok(Some(s)) = db.get_setting("showcmd") {
                self.showcmd.enabled = s != "off" && s != "false";
            }
            if let Ok(Some(ln)) = db.get_setting("line_numbers") {
                self.show_line_numbers = ln != "off" && ln != "false";
            }
            if let Ok(Some(p)) = db.get_setting("preview") {
                self.preview_open = p == "on" || p == "true";
            }
            // Live inline Markdown is disabled: its whole-document layout pass
            // was running on the UI thread and is not needed for raw editing.
            self.inline_mode = false;
            let _ = self.db_tx.send(DbMsg::SaveSetting {
                key: "inline_mode".into(),
                val: "false".into(),
            });
            if let Ok(Some(sb)) = db.get_setting("sidebar") {
                self.sidebar_open = sb == "on" || sb == "true";
            }
            if let Ok(Some(k)) = db.get_setting("deepseek_api_key_enc") {
                self.agent_state.deepseek_api_key_enc = k;
            }
            if let Ok(Some(m)) = db.get_setting("deepseek_model") {
                self.agent_state.deepseek_model = m;
            }
            if let Ok(Some(json)) = db.get_setting("lunaline_config") {
                if let Ok(cfg) = serde_json::from_str::<crate::lunaline::LunaLineConfig>(&json) {
                    self.lunaline_config = cfg;
                }
            }
        }

        // Check settings.json fallback and ensure settings.json file is populated
        if let Ok(path) = core::Database::get_db_path() {
            if let Some(parent) = path.parent() {
                let json_path = parent.join("settings.json");
                let mut map: std::collections::BTreeMap<String, String> = std::fs::read_to_string(&json_path)
                    .ok()
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default();

                if self.agent_state.deepseek_api_key_enc.is_empty() {
                    if let Some(key) = map.get("deepseek_api_key_enc") {
                        self.agent_state.deepseek_api_key_enc = key.clone();
                    }
                }
                if let Some(model) = map.get("deepseek_model") {
                    if self.agent_state.deepseek_model.is_empty() {
                        self.agent_state.deepseek_model = model.clone();
                    }
                }

                if !self.agent_state.deepseek_api_key_enc.is_empty() {
                    map.insert("deepseek_api_key_enc".into(), self.agent_state.deepseek_api_key_enc.clone());
                }
                map.insert("deepseek_model".into(), self.agent_state.deepseek_model.clone());
                if let Ok(s) = serde_json::to_string_pretty(&map) {
                    let _ = std::fs::write(json_path, s);
                }
            }
        }
    }

    pub fn save_active_note_id(&self) {
        let val = self.active_note_id.map(|id| id.to_string()).unwrap_or_default();
        let _ = self.db_tx.send(DbMsg::SaveSetting {
            key: "last_active_note_id".to_string(),
            val,
        });
        self.save_caret_position();
    }

    pub fn save_caret_position(&self) {
        let _ = self.db_tx.send(DbMsg::SaveSetting {
            key: "last_caret_pos".to_string(),
            val: self.ed.cur.to_string(),
        });
        let _ = self.db_tx.send(DbMsg::SaveSetting {
            key: "last_scroll_y".to_string(),
            val: self.scroll_y.to_string(),
        });
        if let Some(id) = self.active_note_id {
            let _ = self.db_tx.send(DbMsg::SaveSetting {
                key: format!("note_caret_{}", id),
                val: self.ed.cur.to_string(),
            });
            let _ = self.db_tx.send(DbMsg::SaveSetting {
                key: format!("note_scroll_{}", id),
                val: self.scroll_y.to_string(),
            });
        }
    }

    pub fn save_open_tabs(&self) {
        if let Some(ref db) = self.db {
            let tab_ids: Vec<i64> = self.open_notes.iter()
                .filter_map(|n| if n.id > 0 { Some(n.id) } else { None })
                .collect();
            if let Ok(json) = serde_json::to_string(&tab_ids) {
                let _ = db.set_setting("open_note_tab_ids", &json);
            }
            let _ = db.set_setting("open_note_active_tab", &self.active_tab.to_string());
        }
    }

    pub fn sync_save_session(&mut self) {
        let mut created_id = None;
        if let Some(ref db) = self.db {
            let _ = db.set_setting("last_caret_pos", &self.ed.cur.to_string());
            let _ = db.set_setting("last_scroll_y", &self.scroll_y.to_string());
            let _ = db.set_setting("opacity", &format!("{:.2}", self.opacity));
            let blur_str = match self.blur_effect {
                crate::blur::BlurEffect::Acrylic => "acrylic",
                crate::blur::BlurEffect::Mica => "mica",
                crate::blur::BlurEffect::None => "none",
            };
            let _ = db.set_setting("blur", blur_str);
            if let Ok(json) = serde_json::to_string(&self.command_history) {
                let _ = db.set_setting("command_history", &json);
            }
            if let Some(id) = self.active_note_id {
                let _ = db.set_setting("last_active_note_id", &id.to_string());
                let _ = db.set_setting(&format!("note_caret_{}", id), &self.ed.cur.to_string());
                let _ = db.set_setting(&format!("note_scroll_{}", id), &self.scroll_y.to_string());
                if self.is_dirty {
                    let _ = db.update_note(id, &self.ed.text());
                    self.is_dirty = false;
                }
            } else if self.is_dirty || !self.ed.text().trim().is_empty() {
                let topic = if self.active_note_title.trim().is_empty() {
                    "Untitled Note".to_string()
                } else {
                    self.active_note_title.clone()
                };
                let dt = Local::now().naive_local();
                let content = self.ed.text();
                if let Ok(new_id) = db.add_note(&topic, &content, None, dt) {
                    created_id = Some(new_id);
                    let _ = db.set_setting("last_active_note_id", &new_id.to_string());
                }
            } else {
                let _ = db.set_setting("last_active_note_id", "");
            }
        }
        if let Some(new_id) = created_id {
            self.active_note_id = Some(new_id);
            self.is_dirty = false;
            self.sync_active_tab();
        }
        self.save_open_tabs();
    }

    pub fn trigger_backup(&mut self, now: f64) {
        let target = if !self.backup_dir.is_empty() {
            let p = std::path::PathBuf::from(&self.backup_dir);
            if p.file_name().and_then(|s| s.to_str()) == Some("mindforge_backup") {
                p
            } else {
                p.join("mindforge_backup")
            }
        } else {
            default_backup_dir()
        };
        if let Some(ref db) = self.db {
            match db.backup(&target) {
                Ok(p) => {
                    self.last_backup_status = Some(format!(
                        "Success ({})",
                        chrono::Local::now().format("%H:%M:%S")
                    ));
                    self.set_status(format!("Backup saved: {}", p.display()), now);
                }
                Err(e) => {
                    self.last_backup_status = Some(format!("Failed: {}", e));
                    self.set_status(format!("Backup failed: {}", e), now);
                }
            }
        } else {
            self.set_status("Database not available for backup", now);
        }
    }
}
