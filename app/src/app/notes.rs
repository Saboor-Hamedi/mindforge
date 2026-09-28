//! Active document state, tab management, and activity tracking.

use super::{App, OpenNote};
use crate::db_worker::DbMsg;
use crate::mode::Mode;
use crate::notes::{delete_active_note, quick_save_active_note, rename_active_note, update_search_results};
use chrono::Local;

impl App {
    pub fn sync_active_tab(&mut self) {
        if let Some(tab) = self.open_notes.get_mut(self.active_tab) {
            tab.title = self.active_note_title.clone();
            tab.is_dirty = self.is_dirty;
            tab.scroll_y = self.scroll_y;
            if let Some(id) = self.active_note_id {
                tab.id = id;
            }
        }
    }

    pub fn switch_tab(&mut self, new_idx: usize, now: f64) {
        if self.open_notes.is_empty() {
            return;
        }
        let new_idx = new_idx.min(self.open_notes.len() - 1);
        if new_idx == self.active_tab {
            return;
        }

        // 1. Sync current state into the active tab before switching
        if let Some(cur) = self.open_notes.get_mut(self.active_tab) {
            cur.editor = self.ed.clone();
            cur.title = self.active_note_title.clone();
            cur.scroll_y = self.scroll_y;
            cur.is_dirty = self.is_dirty;
            if let Some(cur_id) = self.active_note_id {
                cur.id = cur_id;
                if let Some(ref db) = self.db {
                    let _ = db.set_setting(&format!("note_caret_{}", cur_id), &self.ed.cur.to_string());
                    let _ = db.set_setting(&format!("note_scroll_{}", cur_id), &self.scroll_y.to_string());
                }
            }
        }

        // 2. Set new active tab index
        self.active_tab = new_idx;

        // 3. Load target tab state
        let target = &self.open_notes[self.active_tab];
        self.active_note_id = if target.id > 0 { Some(target.id) } else { None };
        self.active_note_title = target.title.clone();
        self.ed = target.editor.clone();
        self.scroll_y = target.scroll_y;
        self.is_dirty = target.is_dirty;
        self.save_active_note_id();
        self.save_open_tabs();
        self.mode = Mode::Normal;
        let msg = format!("Switched to {}", self.active_note_title);
        self.set_status(&msg, now);
    }

    pub fn close_tab(&mut self, idx: usize, now: f64) {
        if self.open_notes.is_empty() {
            self.show_welcome = true;
            return;
        }

        if self.open_notes.len() <= 1 {
            // Last tab closed: clear all open tabs and reveal the Welcome Dashboard
            self.open_notes.clear();
            self.active_note_id = None;
            self.save_active_note_id();
            self.active_note_title.clear();
            self.ed.clear();
            self.mode = Mode::Normal;
            self.is_dirty = false;
            self.scroll_y = 0.0;
            self.active_tab = 0;
            self.last_active_tab = 0;
            self.show_welcome = true;
            self.save_open_tabs();
            self.set_status("All tabs closed — Welcome to MindForge", now);
            return;
        }

        self.open_notes.remove(idx);
        if self.active_tab >= self.open_notes.len() {
            self.active_tab = self.open_notes.len() - 1;
        } else if idx < self.active_tab {
            self.active_tab = self.active_tab.saturating_sub(1);
        }

        let target = &self.open_notes[self.active_tab];
        self.active_note_id = if target.id > 0 { Some(target.id) } else { None };
        self.active_note_title = target.title.clone();
        self.ed = target.editor.clone();
        self.scroll_y = target.scroll_y;
        self.is_dirty = target.is_dirty;
        self.save_active_note_id();
        self.save_open_tabs();
        self.mode = Mode::Normal;
        let msg = format!("Closed tab; active: {}", self.active_note_title);
        self.set_status(&msg, now);
    }

    pub fn switch_doc_tab(&mut self, new_idx: usize, now: f64) {
        if self.open_doc_tabs.is_empty() {
            return;
        }
        let new_idx = new_idx.min(self.open_doc_tabs.len() - 1);
        if new_idx == self.active_doc_tab {
            return;
        }
        self.active_doc_tab = new_idx;
        let doc_idx = self.open_doc_tabs[self.active_doc_tab];
        self.load_doc_by_index(doc_idx, now);
    }

    pub fn close_doc_tab(&mut self, idx: usize, now: f64) {
        if self.open_doc_tabs.len() <= 1 {
            self.mode = Mode::Normal;
            self.set_status("Closed Documentation reader", now);
            return;
        }

        self.open_doc_tabs.remove(idx);
        if self.active_doc_tab >= self.open_doc_tabs.len() {
            self.active_doc_tab = self.open_doc_tabs.len() - 1;
        } else if idx < self.active_doc_tab {
            self.active_doc_tab = self.active_doc_tab.saturating_sub(1);
        }

        let doc_idx = self.open_doc_tabs[self.active_doc_tab];
        self.load_doc_by_index(doc_idx, now);
    }

    pub fn load_note(&mut self, id: i64, topic: String, body: String, now: f64) {
        self.show_welcome = false;
        self.sync_active_tab();

        // 1. Check if note is already open in an existing tab
        if let Some(existing_tab_idx) = self.open_notes.iter().position(|n| (id > 0 && n.id == id) || (id == 0 && n.title == topic)) {
            self.switch_tab(existing_tab_idx, now);
            return;
        }

        // 2. If current tab is pristine Untitled Note, reuse it
        let reuse_current = self.open_notes.len() == 1
            && self.open_notes[0].id == 0
            && !self.open_notes[0].is_dirty
            && self.open_notes[0].editor.text().trim().is_empty();

        self.active_note_id = Some(id);
        self.save_active_note_id();
        self.active_note_title = topic.clone();
        self.ed.clear();
        self.ed.insert_str(&body);
        self.ed.cur = 0;
        self.ed.clear_history();
        self.scroll_y = 0.0;
        self.is_dirty = false;
        self.mode = Mode::Normal;

        if let Some(ref db) = self.db {
            if let Ok(Some(c_str)) = db.get_setting(&format!("note_caret_{}", id)) {
                if let Ok(c) = c_str.parse::<usize>() {
                    self.ed.cur = c.min(self.ed.buf.len());
                }
            }
            if let Ok(Some(s_str)) = db.get_setting(&format!("note_scroll_{}", id)) {
                if let Ok(s) = s_str.parse::<f32>() {
                    self.scroll_y = s;
                }
            }
        }

        let new_tab = OpenNote {
            id,
            title: topic.clone(),
            editor: self.ed.clone(),
            scroll_y: self.scroll_y,
            is_dirty: false,
        };

        if reuse_current {
            self.open_notes[0] = new_tab;
            self.active_tab = 0;
        } else {
            self.open_notes.push(new_tab);
            self.active_tab = self.open_notes.len() - 1;
        }

        self.save_open_tabs();
        let msg = format!("Loaded note: {}", topic);
        self.set_status(&msg, now);
    }

    pub fn reload_db_state(&mut self) {
        if let Some(ref db) = self.db {
            let limit = self.sidebar_notes_limit;
            if let Ok(notes) = db.get_recent_notes(limit) {
                self.notes_list = notes;
            }
            if let Ok(count) = db.get_notes_count() {
                self.total_notes_count = count;
            }
            let today_str = Local::now().date_naive().format("%Y-%m-%d").to_string();
            if let Ok(recent) = db.get_recent_activity(14) {
                if let Some(act) = recent.iter().find(|a| a.date == today_str) {
                    self.today_activity = act.clone();
                }
                self.activity_history = recent;
            }
            if let Ok(life) = db.get_lifetime_activity() {
                self.lifetime_activity = life;
            }
            if let Ok(scans) = db.list_scans() {
                self.past_scans = scans;
            }
        }
    }

    pub fn flush_activity(&mut self, now: f64) {
        self.last_flush_time = now;
        if self.pending_secs < 0.1 && self.pending_keys == 0 && self.pending_words == 0 {
            return;
        }

        let secs = self.pending_secs;
        let keys = self.pending_keys;
        let words = self.pending_words;
        let created = self.pending_created;
        let edited = self.pending_edited;

        self.pending_secs = 0.0;
        self.pending_keys = 0;
        self.pending_words = 0;
        self.pending_created = 0;
        self.pending_edited = 0;

        let today_str = Local::now().date_naive().format("%Y-%m-%d").to_string();
        let _ = self.db_tx.send(DbMsg::FlushActivity {
            date: today_str,
            delta_secs: secs as u32,
            delta_keys: keys,
            delta_words: words,
            delta_created: created,
            delta_edited: edited,
        });
    }

    pub fn open_docs_mode(&mut self, now: f64) {
        self.mode = Mode::Doc;
        self.doc_sidebar_focused = true;
        self.sidebar_open = true;
        self.load_doc_by_index(self.active_doc_idx, now);
        self.set_status("Documentation reader opened (F2 to toggle)", now);
    }

    pub fn load_doc_by_index(&mut self, idx: usize, now: f64) {
        let docs = crate::docs::get_docs();
        if let Some(doc) = docs.get(idx) {
            self.active_doc_idx = idx;
            self.doc_selected_idx = idx;

            if !self.open_doc_tabs.contains(&idx) {
                self.open_doc_tabs.push(idx);
                self.active_doc_tab = self.open_doc_tabs.len() - 1;
            } else if let Some(tab_pos) = self.open_doc_tabs.iter().position(|&t| t == idx) {
                self.active_doc_tab = tab_pos;
            }

            self.doc_ed.clear();
            let body = crate::docs::format_doc_for_reader(doc.content);
            self.doc_ed.insert_str(&body);
            self.doc_ed.cur = 0;
            self.doc_ed.clear_history();
            self.doc_scroll_y = 0.0;
            let msg = format!("Viewing Documentation: {}", doc.title);
            self.set_status(&msg, now);
        }
    }

    pub fn open_help_tab(&mut self, now: f64) {
        self.mode = Mode::Help;
        self.help_scroll_y = 0.0;
        self.set_status("Opened Guidance & Help", now);
    }

    #[allow(dead_code)]
    pub fn open_help_tab_index(&mut self, tab_idx: usize, now: f64) {
        self.mode = Mode::Help;
        self.help_tab = tab_idx;
        self.help_scroll_y = 0.0;
        self.set_status("Opened Guidance & Help", now);
    }

    pub fn set_status(&mut self, msg: impl Into<String>, now: f64) {
        self.status_msg = msg.into();
        self.status_time = now;
    }

    pub fn quick_save_active_note(&mut self, now: f64) {
        quick_save_active_note(self, now);
    }

    pub fn delete_active_note(&mut self, now: f64) {
        delete_active_note(self, now);
    }

    pub fn rename_active_note(&mut self, new_title: &str, now: f64) {
        rename_active_note(self, new_title, now);
    }

    pub fn update_search_results(&mut self) {
        update_search_results(self);
    }

    pub fn create_new_note(&mut self, now: f64) {
        if self.is_dirty && self.mode == Mode::Normal {
            self.quick_save_active_note(now);
        }
        if !self.open_notes.is_empty() {
            if let Some(cur) = self.open_notes.get_mut(self.active_tab) {
                cur.editor = self.ed.clone();
                cur.title = self.active_note_title.clone();
                cur.scroll_y = self.scroll_y;
                cur.is_dirty = self.is_dirty;
            }
        }
        self.show_welcome = false;
        self.active_note_id = None;
        self.active_note_title = "Untitled Note".to_string();
        self.ed.clear();
        self.mode = Mode::Normal;
        self.is_dirty = false;
        self.scroll_y = 0.0;
        self.open_notes.push(crate::app::OpenNote {
            id: 0,
            title: "Untitled Note".to_string(),
            editor: self.ed.clone(),
            scroll_y: 0.0,
            is_dirty: false,
        });
        self.active_tab = self.open_notes.len() - 1;
        self.last_active_tab = self.active_tab;
        self.save_open_tabs();
        self.set_status("Created new note", now);
    }

    pub fn open_note_by_id(&mut self, id: i64, now: f64) {
        if id <= 0 {
            return;
        }

        // 1. If note is already open in an existing tab, switch directly to it
        if let Some(tab_idx) = self.open_notes.iter().position(|t| t.id == id) {
            self.switch_tab(tab_idx, now);
            return;
        }

        // 2. Load note from SQLite database
        if let Some(ref db) = self.db {
            if let Ok(Some(note)) = db.get_note(id) {
                self.load_note(note.id, note.topic, note.body, now);
                return;
            }
        }

        // 3. Fallback: check in self.notes_list cache
        if let Some(note) = self.notes_list.iter().find(|n| n.id == id).cloned() {
            self.load_note(note.id, note.topic, note.body, now);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_close_last_tab_reveals_welcome_dashboard() {
        let mut app = App::new();
        // Ensure starting with 1 tab
        app.open_notes.clear();
        app.create_new_note(1.0);
        assert_eq!(app.open_notes.len(), 1);
        assert!(!app.show_welcome);

        // Closing the only/last tab
        app.close_tab(0, 2.0);
        assert!(app.open_notes.is_empty(), "open_notes must be empty when last tab is closed");
        assert!(app.show_welcome, "show_welcome must be true when all tabs are closed");
        assert_eq!(app.active_note_id, None);
        assert!(app.active_note_title.is_empty());
    }

    #[test]
    fn test_create_new_note_clears_show_welcome() {
        let mut app = App::new();
        app.open_notes.clear();
        app.show_welcome = true;

        app.create_new_note(1.0);
        assert_eq!(app.open_notes.len(), 1);
        assert!(!app.show_welcome, "Creating a note must dismiss welcome dashboard");
        assert_eq!(app.active_note_title, "Untitled Note");
    }

    #[test]
    fn test_load_note_clears_show_welcome() {
        let mut app = App::new();
        app.open_notes.clear();
        app.show_welcome = true;

        app.load_note(999, "Obsidian Import".to_string(), "# Hello".to_string(), 1.0);
        assert_eq!(app.open_notes.len(), 1);
        assert!(!app.show_welcome, "Loading a note must dismiss welcome dashboard");
        assert_eq!(app.active_note_id, Some(999));
        assert_eq!(app.active_note_title, "Obsidian Import");
    }

    #[test]
    fn test_closing_one_of_multiple_tabs_keeps_remaining() {
        let mut app = App::new();
        app.open_notes.clear();
        app.create_new_note(1.0);
        app.create_new_note(2.0);
        assert_eq!(app.open_notes.len(), 2);
        assert!(!app.show_welcome);

        // Close active tab (tab 1)
        app.close_tab(1, 3.0);
        assert_eq!(app.open_notes.len(), 1);
        assert!(!app.show_welcome);
        assert_eq!(app.active_tab, 0);
    }
}
