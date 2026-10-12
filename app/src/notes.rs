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
        let (path, title) = if let Some(root) = app.workspace.root.as_ref() {
            let active_lang = app.active_language();
            let default_ext = active_lang.default_extension();
            let file_name = if app.notes.active_note_title.trim().is_empty() {
                format!("Untitled.{}", default_ext)
            } else {
                let t = app.notes.active_note_title.trim();
                if std::path::Path::new(t).extension().is_some() {
                    t.to_string()
                } else {
                    format!("{t}.{}", default_ext)
                }
            };
            let user_title_trimmed = app.notes.active_note_title.trim();
            let is_explicit_title = !user_title_trimmed.is_empty()
                && !user_title_trimmed.eq_ignore_ascii_case("untitled")
                && !user_title_trimmed.eq_ignore_ascii_case("untitled note");
            let target = root.join(&file_name);
            if is_explicit_title && target.exists() {
                app.set_status(
                    format!("File '{}' already exists and cannot be created", file_name),
                    now,
                );
                return;
            }
            let mut target = target;
            let mut suffix = 2;
            let has_ext = std::path::Path::new(&file_name).extension().is_some();
            while target.exists() {
                target = if has_ext {
                    let stem = std::path::Path::new(&file_name)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or(&file_name);
                    let ext = std::path::Path::new(&file_name)
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("");
                    root.join(format!("{stem}_{suffix}.{ext}"))
                } else {
                    root.join(format!("{file_name}_{suffix}.{}", default_ext))
                };
                suffix += 1;
            }
            let display_title = target
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            (target, display_title)
        } else {
            let picker = rfd::FileDialog::new();
            let active_lang = app.active_language();
            let default_ext = active_lang.default_extension();
            let suggested = if app.notes.active_note_title.trim().is_empty() {
                format!("Untitled.{}", default_ext)
            } else {
                let t = app.notes.active_note_title.trim();
                if std::path::Path::new(t).extension().is_some() {
                    t.to_string()
                } else {
                    format!("{t}.{}", default_ext)
                }
            };
            let Some(p) = picker.set_file_name(suggested).save_file() else {
                app.set_status("Save cancelled", now);
                return;
            };
            let display_title = p
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            (p, display_title)
        };

        match std::fs::write(&path, content.as_bytes()) {
            Ok(()) => {
                let path = crate::workspace::normalized_path(&path);
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
                if app.workspace.root.is_some() {
                    let _ = app.workspace.refresh();
                    app.workspace.rebuild_search_index();
                    app.workspace.reveal(&path);
                    app.workspace.selected_path = Some(path.clone());
                    app.workspace.selected_items.clear();
                    app.workspace.selected_items.insert(path.clone());
                    app.workspace.selection_anchor = Some(path.clone());
                    app.persist_workspace_expansion();
                }
                if let Some(backend) = app.services.vim_runtime.backend.as_mut() {
                    backend.mark_saved();
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
    let file_name = if topic.ends_with(".md") {
        topic.clone()
    } else {
        format!("{}.md", topic)
    };
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
    if let Some(tab) = app.open_notes.get(app.tabs.active_tab) {
        if let Some(path) = tab.file_path.clone() {
            let file_name_disp = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let active_idx = app.tabs.active_tab;
            app.close_tab(active_idx, now);
            match crate::workspace::delete_path(&path) {
                Ok(()) => {
                    let _ = app.workspace.refresh();
                    app.set_status(format!("Deleted {}", file_name_disp), now);
                }
                Err(e) => {
                    app.set_status(format!("Failed to delete {}: {}", file_name_disp, e), now);
                }
            }
            return;
        }
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

/// Renames the active note and updates in-memory notes_list or filesystem file.
pub fn rename_active_note(app: &mut App, new_title: &str, now: f64) {
    let trimmed = new_title.trim().to_string();
    if trimmed.is_empty() {
        return;
    }

    // If the active tab has an existing file path on the filesystem, perform a real file rename
    if let Some(tab) = app.open_notes.get(app.tabs.active_tab) {
        if let Some(old_path) = tab.file_path.clone() {
            let parent = old_path
                .parent()
                .unwrap_or_else(|| std::path::Path::new(""));
            let new_path = parent.join(&trimmed);
            if new_path == old_path {
                return;
            }
            if new_path.exists() {
                app.set_status(format!("Cannot rename: '{}' already exists", trimmed), now);
                return;
            }

            match crate::workspace::rename_path(&old_path, &new_path) {
                Ok(()) => {
                    let new_path = crate::workspace::normalized_path(&new_path);
                    app.remap_workspace_prefix(&old_path, &new_path);

                    let versions: Vec<_> = app
                        .file_versions
                        .iter()
                        .map(|(p, v)| (p.clone(), *v))
                        .collect();
                    for (path, version) in versions {
                        if path.starts_with(&old_path) {
                            if let Ok(suffix) = path.strip_prefix(&old_path) {
                                app.file_versions.remove(&path);
                                app.file_versions.insert(new_path.join(suffix), version);
                            }
                        }
                    }

                    if let Some(tab) = app.open_notes.get_mut(app.tabs.active_tab) {
                        tab.file_path = Some(new_path.clone());
                        tab.title = trimmed.clone();
                    }
                    app.notes.active_note_title = trimmed.clone();

                    if let Some(backend) = app.services.vim_runtime.backend.as_mut() {
                        let _ = backend.set_document(&app.editor.ed.text(), app.editor.ed.cur, 0);
                    }

                    let _ = app.workspace.refresh();
                    app.workspace.rebuild_search_index();
                    app.workspace.reveal(&new_path);
                    app.workspace.selected_path = Some(new_path.clone());
                    app.workspace.selected_items.clear();
                    app.workspace.selected_items.insert(new_path.clone());
                    app.workspace.selection_anchor = Some(new_path.clone());
                    app.persist_workspace_expansion();
                    app.sync_active_tab();
                    app.save_open_tabs();

                    app.set_status(format!("Renamed to {}", trimmed), now);
                }
                Err(e) => {
                    app.set_status(format!("Rename failed: {}", e), now);
                }
            }
            return;
        }
    }

    if let Some(id) = app.notes.active_note_id {
        app.notes.active_note_title = trimmed.clone();
        if let Some(n) = app.notes.notes_list.iter_mut().find(|n| n.id == id) {
            n.topic = trimmed.clone();
        }
        app.sync_active_tab();
        app.save_open_tabs();
        app.set_status("Renamed", now);
    } else {
        if let Some(root) = app.workspace.root.as_ref() {
            let active_lang = app.active_language();
            let default_ext = active_lang.default_extension();
            let file_name = if std::path::Path::new(&trimmed).extension().is_some() {
                trimmed.clone()
            } else {
                format!("{trimmed}.{default_ext}")
            };
            let target = root.join(&file_name);
            if target.exists() {
                app.set_status(
                    format!("File '{}' already exists in workspace", file_name),
                    now,
                );
                return;
            }
            let content = app.editor.ed.text();
            if let Err(err) = std::fs::write(&target, &content) {
                app.set_status(format!("Failed to create file: {err}"), now);
                return;
            }
            if let Some(tab) = app.open_notes.get_mut(app.tabs.active_tab) {
                tab.file_path = Some(target.clone());
                tab.title = file_name.clone();
                tab.is_dirty = false;
            }
            app.notes.active_note_title = file_name.clone();
            app.editor.is_dirty = false;
            if let Ok(version) = crate::workspace::file_fingerprint(&target) {
                app.file_versions.insert(target.clone(), version);
            }
            app.workspace.reveal(&target);
            let _ = app.workspace.refresh();
            app.workspace.rebuild_search_index();
            app.persist_workspace_expansion();
            app.sync_active_tab();
            app.save_open_tabs();
            app.set_status(format!("Created and saved {}", file_name), now);
            return;
        }

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
        app.services.lunaline_config.style,
    );
    let indexed_paths: Vec<std::path::PathBuf> = Vec::new();
    app.modal
        .search_results
        .extend(crate::services::fuzzy::search_workspace(
            query,
            app.workspace.root.as_deref(),
            &app.workspace.search_entries,
            &indexed_paths,
        ));
    app.modal
        .search_results
        .sort_by(|a, b| b.score.cmp(&a.score));
    if app.modal.search_selected >= app.modal.search_results.len() {
        app.modal.search_selected = 0;
    }
}
