//! Synchronous note persistence, live search filtering, and note operations.

use crate::app::App;
use chrono::Local;
use core::Note;

/// SQLite is the application's authoritative persistence layer in both editor
/// modes; Vim snapshots are synchronized from Neovim before this function runs.
pub fn quick_save_active_note(app: &mut App, now: f64) {
    let content = app.ed.text();
    if let Some(ref db) = app.db {
        if let Some(id) = app.active_note_id {
            let _ = db.update_note(id, &content);
            app.is_dirty = false;
            if let Some(backend) = app.vim_runtime.backend.as_mut() {
                backend.mark_saved();
            }
            app.last_saved_time = now;
            app.pending_edited += 1;
            if let Some(n) = app.notes_list.iter_mut().find(|n| n.id == id) {
                n.body = content.clone();
            }
            app.save_active_note_id();
            app.sync_active_tab();
            app.save_open_tabs();
            app.set_status("Saved", now);
        } else {
            let topic = if app.active_note_title.trim().is_empty() {
                "Untitled Note".to_string()
            } else {
                app.active_note_title.clone()
            };
            let dt = Local::now().naive_local();
            if let Ok(new_id) = db.add_note(&topic, &content, None, dt) {
                app.active_note_id = Some(new_id);
                app.save_active_note_id();
                app.is_dirty = false;
                app.last_saved_time = now;
                app.pending_created += 1;
                app.notes_list.insert(
                    0,
                    Note {
                        id: new_id,
                        topic: topic.clone(),
                        body: content,
                        struggled_with: None,
                        created_at: dt,
                    },
                );
                app.sync_active_tab();
                app.save_open_tabs();
                app.set_status("Saved", now);
            }
        }
    }
}

/// Deletes the active note from SQLite and in-memory notes_list.
pub fn delete_active_note(app: &mut App, now: f64) {
    if let Some(id) = app.active_note_id {
        if let Some(ref db) = app.db {
            let _ = db.delete_note(id);
        }
        let _ = app.db_tx.send(crate::services::db_worker::DbMsg::DeleteNote { id });
        app.notes_list.retain(|n| n.id != id);
        app.open_notes.retain(|n| n.id != id);
        if app.active_tab >= app.open_notes.len() && !app.open_notes.is_empty() {
            app.active_tab = app.open_notes.len() - 1;
        }
        app.save_open_tabs();
        app.active_note_id = None;
        app.save_active_note_id();
        app.active_note_title.clear();
        app.ed.clear();
        app.is_dirty = false;
        app.set_status("Deleted", now);
        app.reload_db_state();

        // Load the next available note if one exists
        if let Some(first) = app.notes_list.first() {
            let first_id = first.id;
            let topic = first.topic.clone();
            let body = first.body.clone();
            let clean = body.replace("\r\n", "\n").replace('\r', "\n");
            app.active_note_id = Some(first_id);
            app.save_active_note_id();
            app.active_note_title = topic;
            app.ed.set_text(&clean);
            let saved_cur = app.db.as_ref()
                .and_then(|db| db.get_setting(&format!("note_caret_{}", first_id)).ok().flatten())
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0);
            app.ed.cur = saved_cur.min(app.ed.buf.len());
            app.is_dirty = false;
            app.show_welcome = false;
        } else {
            app.open_notes.clear();
            app.active_note_id = None;
            app.save_active_note_id();
            app.active_note_title.clear();
            app.ed.clear();
            app.is_dirty = false;
            app.show_welcome = true;
            app.save_open_tabs();
        }
    } else {
        app.ed.clear();
        app.is_dirty = false;
        app.set_status("Cleared note", now);
    }
}

/// Renames the active note directly in SQLite and updates in-memory notes_list.
pub fn rename_active_note(app: &mut App, new_title: &str, now: f64) {
    let trimmed = new_title.trim().to_string();
    if trimmed.is_empty() {
        return;
    }
    app.active_note_title = trimmed.clone();
    if let Some(id) = app.active_note_id {
        if let Some(ref db) = app.db {
            let _ = db.rename_note(id, &trimmed);
        }
        if let Some(n) = app.notes_list.iter_mut().find(|n| n.id == id) {
            n.topic = trimmed.clone();
        }
        app.sync_active_tab();
        app.save_open_tabs();
        app.set_status("Renamed", now);
    } else {
        // Active note was newly created (e.g. via Ctrl+N) and not yet stored in SQLite.
        // Save it now so it immediately exists in the DB and appears in the sidebar!
        if let Some(ref db) = app.db {
            let dt = Local::now().naive_local();
            let content = app.ed.text();
            if let Ok(new_id) = db.add_note(&trimmed, &content, None, dt) {
                app.active_note_id = Some(new_id);
                app.save_active_note_id();
                app.pending_created += 1;
                app.notes_list.insert(
                    0,
                    Note {
                        id: new_id,
                        topic: trimmed.clone(),
                        body: content,
                        struggled_with: None,
                        created_at: dt,
                    },
                );
                app.sync_active_tab();
                app.save_open_tabs();
                app.set_status("Saved note", now);
            }
        }
    }
}

/// Updates fuzzy search results across notes, commands, themes, sound profiles, carets, fonts, and modes.
pub fn update_search_results(app: &mut App) {
    let query = app.search_query.trim();
    app.search_results = crate::services::fuzzy::search_palette(
        query,
        &app.notes_list,
        app.theme.kind,
        app.sound.profile,
        app.caret.kind,
        &app.selected_font,
        app.editor_controller.mode,
        app.lunaline_config.style,
    );
    if app.search_selected >= app.search_results.len() {
        app.search_selected = 0;
    }
}
