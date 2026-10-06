//! Synchronous note persistence, live search filtering, and note operations.

use crate::app::App;

/// Saves filesystem documents to their actual paths; Vim snapshots are synchronized before saving.
pub fn quick_save_active_note(app: &mut App, now: f64) {
    crate::vim::runtime::sync_neovim_changes(app, now);
    let content = app.editor.ed.text();
    let file_path = app
        .open_notes
        .get(app.tabs.active_tab)
        .and_then(|tab| tab.file_path.clone());

    if let Some(path) = file_path {
        let expected = app.file_versions.get(&path).copied();
        if !path.exists() {
            app.set_status(
                "Save blocked: the file was removed or renamed outside MindForge",
                now,
            );
            return;
        }
        if let (Some(expected), Ok(actual)) = (expected, crate::workspace::file_fingerprint(&path))
        {
            if actual != expected {
                app.set_status("Save blocked: the file changed outside MindForge; close and reopen it to review the external version", now);
                return;
            }
        }
        match std::fs::write(&path, content) {
            Ok(()) => {
                if let Ok(version) = crate::workspace::file_fingerprint(&path) {
                    app.file_versions.insert(path.clone(), version);
                }
                app.editor.is_dirty = false;
                app.editor.last_saved_time = now;
                if let Some(backend) = app.services.vim_runtime.backend.as_mut() {
                    backend.mark_saved();
                }
                app.sync_active_tab();
                app.set_status(format!("Saved {}", app.display_file_path(&path)), now);
            }
            Err(error) => app.set_status(format!("Save failed: {error}"), now),
        }
        return;
    }

    if app.notes.active_note_id.is_none() {
        let mut picker = rfd::FileDialog::new();
        if let Some(root) = app.workspace.root.as_deref() {
            picker = picker.set_directory(root);
        }
        let suggested = if app.notes.active_note_title.trim().is_empty() {
            "Untitled.md".to_string()
        } else {
            format!("{}.md", app.notes.active_note_title.trim())
        };
        let Some(path) = picker.set_file_name(suggested).save_file() else {
            app.set_status("Save cancelled", now);
            return;
        };
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut file) => {
                use std::io::Write;
                if let Err(error) = file.write_all(content.as_bytes()) {
                    app.set_status(format!("Save failed: {error}"), now);
                    return;
                }
                let path = crate::workspace::normalized_path(&path);
                let title = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                if let Some(tab) = app.open_notes.get_mut(app.tabs.active_tab) {
                    tab.file_path = Some(path.clone());
                    tab.title = title.clone();
                    tab.id = 0;
                }
                app.notes.active_note_title = title;
                if let Ok(version) = crate::workspace::file_fingerprint(&path) {
                    app.file_versions.insert(path.clone(), version);
                }
                app.editor.is_dirty = false;
                app.editor.last_saved_time = now;
                app.sync_active_tab();
                app.save_open_tabs();
                if let Some(backend) = app.services.vim_runtime.backend.as_mut() {
                    backend.mark_saved();
                }
                if let Some(id) = app.notes.active_note_id {
                    if let Some(note) = app.notes.notes_list.iter_mut().find(|note| note.id == id) {
                        note.body = content;
                    }
                }
                app.set_status(format!("Saved {}", app.display_file_path(&path)), now);
            }
            Err(error) => app.set_status(format!("Could not create file: {error}"), now),
        }
        return;
    }

    let topic = if app.notes.active_note_title.trim().is_empty() {
        "Untitled".to_string()
    } else {
        app.notes.active_note_title.trim().to_string()
    };
    let file_name = if topic.ends_with(".md") { topic.clone() } else { format!("{}.md", topic) };
    let notes_dir = crate::workspace::default_workspace_dir().join("Notes");
    let _ = std::fs::create_dir_all(&notes_dir);
    let path = notes_dir.join(&file_name);
    if std::fs::write(&path, &content).is_ok() {
        if let Some(tab) = app.open_notes.get_mut(app.tabs.active_tab) {
            tab.file_path = Some(path.clone());
        }
        if let Ok(version) = crate::workspace::file_fingerprint(&path) {
            app.file_versions.insert(path, version);
        }
        app.editor.is_dirty = false;
        if let Some(backend) = app.services.vim_runtime.backend.as_mut() {
            backend.mark_saved();
        }
        app.editor.last_saved_time = now;
        app.activity.pending_edited += 1;
        if let Some(id) = app.notes.active_note_id {
            if let Some(note) = app.notes.notes_list.iter_mut().find(|note| note.id == id) {
                note.body = content;
            }
        }
        app.save_active_note_id();
        app.sync_active_tab();
        app.save_open_tabs();
        app.set_status("Saved", now);
    }
}

/// Deletes the active note from the filesystem and in-memory notes_list.
pub fn delete_active_note(app: &mut App, now: f64) {
    if app
        .open_notes
        .get(app.tabs.active_tab)
        .is_some_and(|tab| tab.file_path.is_some() && tab.id <= 0)
    {
        app.set_status("Code files cannot be deleted from MindForge", now);
        return;
    }
    if let Some(id) = app.notes.active_note_id {
        app.notes.notes_list.retain(|n| n.id != id);
        app.open_notes.retain(|n| n.id != id);
        if app.tabs.active_tab >= app.open_notes.len() && !app.open_notes.is_empty() {
            app.tabs.active_tab = app.open_notes.len() - 1;
        }
        app.save_open_tabs();
        app.notes.active_note_id = None;
        app.save_active_note_id();
        app.notes.active_note_title.clear();
        app.editor.ed.clear();
        app.editor.is_dirty = false;
        app.set_status("Deleted", now);
        app.reload_db_state();

        // Load the next available note if one exists
        if let Some(first) = app.notes.notes_list.first() {
            let first_id = first.id;
            let topic = first.topic.clone();
            let body = first.body.clone();
            let clean = body.replace("\r\n", "\n").replace('\r', "\n");
            app.notes.active_note_id = Some(first_id);
            app.save_active_note_id();
            app.notes.active_note_title = topic;
            app.editor.ed.set_text(&clean);
            app.editor.ed.cur = 0;
            app.editor.is_dirty = false;
            app.misc.show_welcome = false;
        } else {
            app.open_notes.clear();
            app.notes.active_note_id = None;
            app.save_active_note_id();
            app.notes.active_note_title.clear();
            app.editor.ed.clear();
            app.editor.is_dirty = false;
            app.misc.show_welcome = true;
            app.save_open_tabs();
        }
    } else {
        app.editor.ed.clear();
        app.editor.is_dirty = false;
        app.set_status("Cleared note", now);
    }
}

/// Renames the active note and updates in-memory notes_list.
pub fn rename_active_note(app: &mut App, new_title: &str, now: f64) {
    if app
        .open_notes
        .get(app.tabs.active_tab)
        .is_some_and(|tab| tab.file_path.is_some() && tab.id <= 0)
    {
        app.set_status("Code file names are managed by the filesystem", now);
        return;
    }
    let trimmed = new_title.trim().to_string();
    if trimmed.is_empty() {
        return;
    }
    app.notes.active_note_title = trimmed.clone();
    if let Some(id) = app.notes.active_note_id {
        if let Some(n) = app.notes.notes_list.iter_mut().find(|n| n.id == id) {
            n.topic = trimmed.clone();
        }
        app.sync_active_tab();
        app.save_open_tabs();
        app.set_status("Renamed", now);
    } else {
        app.notes.active_note_title = trimmed.clone();
        app.sync_active_tab();
        app.save_open_tabs();
        app.set_status(
            "Untitled document renamed; save it to create the filesystem file",
            now,
        );
    }
}

/// Updates fuzzy search results across notes, commands, themes, sound profiles, carets, fonts, and modes.
pub fn update_search_results(app: &mut App) {
    let query = app.modal.search_query.trim();
    app.modal.search_results = crate::services::fuzzy::search_palette(
        query,
        &app.notes.notes_list,
        app.misc.theme.kind,
        app.misc.sound.profile,
        app.misc.caret.kind,
        &app.misc.selected_font,
        app.services.editor_controller.mode,
        app.services.lunaline_config.style,
    );
    let indexed_paths: Vec<std::path::PathBuf> = Vec::new();
    app.modal.search_results.extend(crate::services::fuzzy::search_workspace(query,app.workspace.root.as_deref(),&app.workspace.search_entries,&indexed_paths));
    app.modal.search_results.sort_by(|a,b|b.score.cmp(&a.score));
    if app.modal.search_selected >= app.modal.search_results.len() {
        app.modal.search_selected = 0;
    }
}
