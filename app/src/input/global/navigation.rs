//! Surface keyboard navigation (Sidebar, Documentation Sidebar, Scan View, and Escape dismissal).

use crate::app::App;
use crate::editor::backend::EditorBackend;
use crate::mode::Mode;
use eframe::egui;

pub fn handle_navigation_shortcuts(app: &mut App, ctx: &egui::Context, now: f64) -> Option<bool> {
    // 1. Sidebar Keyboard Navigation
    // Guarded by !app.command_bar.in_command so typing ':' is never consumed by the sidebar.
    if app.sidebar.open && app.sidebar.focused && !app.command_bar.in_command {
        let (sb_up, sb_down, sb_left, sb_right, sb_enter, sb_esc, sb_edit, sb_to_editor, sb_tab) =
            ctx.input(|i| {
                (
                    (!i.modifiers.ctrl && !i.modifiers.alt && i.key_pressed(egui::Key::K))
                        || i.key_pressed(egui::Key::ArrowUp),
                    (!i.modifiers.ctrl && !i.modifiers.alt && i.key_pressed(egui::Key::J))
                        || i.key_pressed(egui::Key::ArrowDown),
                    !i.modifiers.ctrl && !i.modifiers.alt && i.key_pressed(egui::Key::ArrowLeft),
                    !i.modifiers.ctrl && !i.modifiers.alt && i.key_pressed(egui::Key::ArrowRight),
                    !i.modifiers.ctrl
                        && !i.modifiers.command
                        && i.key_pressed(egui::Key::Enter),
                    i.key_pressed(egui::Key::Escape),
                    !i.modifiers.ctrl && !i.modifiers.alt && i.key_pressed(egui::Key::I),
                    (i.modifiers.ctrl
                        && !i.modifiers.shift
                        && !i.modifiers.alt
                        && i.key_pressed(egui::Key::L))
                        || (!i.modifiers.ctrl && !i.modifiers.alt && i.key_pressed(egui::Key::L)),
                    !i.modifiers.ctrl && !i.modifiers.alt && i.key_pressed(egui::Key::Tab),
                )
            });

        if sb_esc || sb_to_editor || sb_tab {
            app.sidebar.focused = false;
            app.misc.caret.gliding = false;
            app.set_status("Editor focused (Ctrl+H for sidebar)", now);
            return Some(false);
        }

        if sb_edit {
            app.sidebar.focused = false;
            app.misc.caret.gliding = false;
            if let Some(backend) = app.services.vim_runtime.backend.as_mut() {
                let _ = backend.handle_text("i");
            } else {
                app.services
                    .vim_runtime
                    .queue_input(crate::vim::PendingVimInput::Text("i".into()));
            }
            return Some(false);
        }

        if sb_down {
            if app.workspace.root.is_some() && !app.workspace.entries.is_empty() {
                let query_lower = app.sidebar.filter_query.trim().to_lowercase();
                let visible_indices: Vec<usize> = app
                    .workspace
                    .entries
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| {
                        if query_lower.is_empty() {
                            true
                        } else {
                            e.path
                                .file_name()
                                .map(|f| f.to_string_lossy().to_lowercase().contains(&query_lower))
                                .unwrap_or(false)
                        }
                    })
                    .map(|(i, _)| i)
                    .collect();

                if !visible_indices.is_empty() {
                    let current_pos = app.workspace.selected_path.as_ref().and_then(|cur| {
                        visible_indices.iter().position(|&idx| {
                            crate::workspace::same_path(&app.workspace.entries[idx].path, cur)
                        })
                    });
                    let new_pos = match current_pos {
                        Some(pos) => (pos + 1).min(visible_indices.len() - 1),
                        None => 0,
                    };
                    let entry_idx = visible_indices[new_pos];
                    let path = app.workspace.entries[entry_idx].path.clone();
                    app.workspace.selected_path = Some(path.clone());
                    app.workspace.selected_items.clear();
                    app.workspace.selected_items.insert(path.clone());
                    app.workspace.selection_anchor = Some(path);
                    app.sidebar.needs_scroll = true;
                }
            } else if !app.notes.notes_list.is_empty() {
                if app.sidebar.selected_idx + 1 < app.notes.notes_list.len() {
                    app.sidebar.selected_idx += 1;
                    app.sidebar.needs_scroll = true;
                }
            }
            return Some(false);
        }

        if sb_up {
            if app.workspace.root.is_some() && !app.workspace.entries.is_empty() {
                let query_lower = app.sidebar.filter_query.trim().to_lowercase();
                let visible_indices: Vec<usize> = app
                    .workspace
                    .entries
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| {
                        if query_lower.is_empty() {
                            true
                        } else {
                            e.path
                                .file_name()
                                .map(|f| f.to_string_lossy().to_lowercase().contains(&query_lower))
                                .unwrap_or(false)
                        }
                    })
                    .map(|(i, _)| i)
                    .collect();

                if !visible_indices.is_empty() {
                    let current_pos = app.workspace.selected_path.as_ref().and_then(|cur| {
                        visible_indices.iter().position(|&idx| {
                            crate::workspace::same_path(&app.workspace.entries[idx].path, cur)
                        })
                    });
                    let new_pos = match current_pos {
                        Some(pos) => pos.saturating_sub(1),
                        None => visible_indices.len().saturating_sub(1),
                    };
                    let entry_idx = visible_indices[new_pos];
                    let path = app.workspace.entries[entry_idx].path.clone();
                    app.workspace.selected_path = Some(path.clone());
                    app.workspace.selected_items.clear();
                    app.workspace.selected_items.insert(path.clone());
                    app.workspace.selection_anchor = Some(path);
                    app.sidebar.needs_scroll = true;
                }
            } else if !app.notes.notes_list.is_empty() {
                if app.sidebar.selected_idx > 0 {
                    app.sidebar.selected_idx -= 1;
                    app.sidebar.needs_scroll = true;
                }
            }
            return Some(false);
        }

        if sb_left && app.workspace.root.is_some() {
            if let Some(entry_path) = app.workspace.selected_path.clone() {
                if app.workspace.expanded.contains(&entry_path) {
                    app.workspace.toggle(&entry_path);
                    app.persist_workspace_expansion();
                    app.sidebar.needs_scroll = true;
                } else if let Some(parent) = entry_path.parent() {
                    if app
                        .workspace
                        .entries
                        .iter()
                        .any(|e| crate::workspace::same_path(&e.path, parent))
                    {
                        let parent_path = parent.to_path_buf();
                        app.workspace.selected_path = Some(parent_path.clone());
                        app.workspace.selected_items.clear();
                        app.workspace.selected_items.insert(parent_path.clone());
                        app.workspace.selection_anchor = Some(parent_path);
                        app.sidebar.needs_scroll = true;
                    }
                }
            }
            return Some(false);
        }

        if sb_right && app.workspace.root.is_some() {
            if let Some(entry_path) = app.workspace.selected_path.clone() {
                let is_dir = app
                    .workspace
                    .entries
                    .iter()
                    .find(|e| crate::workspace::same_path(&e.path, &entry_path))
                    .map(|e| e.directory)
                    .unwrap_or_else(|| entry_path.is_dir());
                if is_dir && !app.workspace.expanded.contains(&entry_path) {
                    app.workspace.toggle(&entry_path);
                    app.persist_workspace_expansion();
                    app.sidebar.needs_scroll = true;
                    return Some(false);
                }
            }
        }

        if sb_enter {
            if app.workspace.root.is_some() {
                if let Some(entry_path) = app.workspace.selected_path.clone() {
                    let is_dir = app
                        .workspace
                        .entries
                        .iter()
                        .find(|e| crate::workspace::same_path(&e.path, &entry_path))
                        .map(|e| e.directory)
                        .unwrap_or_else(|| entry_path.is_dir());
                    if is_dir {
                        app.workspace.toggle(&entry_path);
                        app.persist_workspace_expansion();
                        app.sidebar.needs_scroll = true;
                    } else {
                        app.open_file_path(entry_path, now);
                        app.sidebar.focused = false;
                        app.set_status("Opened file (Ctrl+H: sidebar)", now);
                    }
                }
            } else if let Some(note) = app.notes.notes_list.get(app.sidebar.selected_idx).cloned() {
                app.load_note(note.id, note.topic, note.body, now);
                app.sidebar.focused = false;
                app.set_status("Loaded note (Ctrl+H: sidebar)", now);
            }
            return Some(false);
        }

        return Some(false);
    }

    // 2. Documentation Sidebar Navigation
    if app.misc.mode == Mode::Doc && app.tabs.doc_sidebar_focused && !app.command_bar.in_command {
        let (doc_up, doc_down, doc_enter, doc_esc, doc_edit, doc_to_reader) = ctx.input(|i| {
            (
                (!i.modifiers.ctrl && !i.modifiers.alt && i.key_pressed(egui::Key::K))
                    || i.key_pressed(egui::Key::ArrowUp),
                (!i.modifiers.ctrl && !i.modifiers.alt && i.key_pressed(egui::Key::J))
                    || i.key_pressed(egui::Key::ArrowDown),
                !i.modifiers.ctrl && !i.modifiers.command && i.key_pressed(egui::Key::Enter),
                i.key_pressed(egui::Key::Escape),
                !i.modifiers.ctrl && !i.modifiers.alt && i.key_pressed(egui::Key::I),
                (!i.modifiers.alt
                    && (i.key_pressed(egui::Key::L) || i.key_pressed(egui::Key::ArrowRight))),
            )
        });

        if doc_esc {
            app.tabs.doc_sidebar_focused = false;
            app.set_status("Doc reader active (Esc returns to the sidebar)", now);
            return Some(false);
        }

        if doc_edit {
            app.tabs.doc_sidebar_focused = false;
            if let Some(backend) = app.services.vim_runtime.backend.as_mut() {
                let _ = backend.handle_text("i");
            } else {
                app.services
                    .vim_runtime
                    .queue_input(crate::vim::PendingVimInput::Text("i".into()));
            }
            return Some(false);
        }

        if doc_to_reader {
            app.tabs.doc_sidebar_focused = false;
            app.set_status("Doc reader active (Ctrl+H: sidebar)", now);
            return Some(false);
        }

        let num_docs = crate::ui::docs::get_docs().len();
        if doc_down {
            if num_docs > 0 && app.tabs.doc_selected_idx + 1 < num_docs {
                app.tabs.doc_selected_idx += 1;
            }
            return Some(false);
        }

        if doc_up {
            if app.tabs.doc_selected_idx > 0 {
                app.tabs.doc_selected_idx -= 1;
            }
            return Some(false);
        }

        if doc_enter {
            app.load_doc_by_index(app.tabs.doc_selected_idx, now);
            app.tabs.doc_sidebar_focused = false;
            return Some(false);
        }

        return Some(false);
    }

    // 3. Scan View Navigation
    if matches!(app.misc.mode, Mode::ScanReport | Mode::ScanHistory) && !app.command_bar.in_command
    {
        let (scan_up, scan_down, scan_esc) = ctx.input(|i| {
            (
                (!i.modifiers.ctrl && !i.modifiers.alt && i.key_pressed(egui::Key::K))
                    || i.key_pressed(egui::Key::ArrowUp),
                (!i.modifiers.ctrl && !i.modifiers.alt && i.key_pressed(egui::Key::J))
                    || i.key_pressed(egui::Key::ArrowDown),
                i.key_pressed(egui::Key::Escape),
            )
        });

        if scan_esc {
            app.misc.mode = app.scan.prev_mode_before_scan;
            app.set_status("Exited Scanner", now);
            return Some(false);
        }

        if scan_down {
            app.scan.scan_report_scroll_y =
                (app.scan.scan_report_scroll_y + 40.0).clamp(0.0, 5000.0);
            return Some(false);
        }

        if scan_up {
            app.scan.scan_report_scroll_y = (app.scan.scan_report_scroll_y - 40.0).max(0.0);
            return Some(false);
        }
    }

    // 4. Global Escape Dismissal
    let esc = ctx.input(|i| i.key_pressed(egui::Key::Escape));
    if esc {
        if !app.command_bar.in_command {
            return None;
        }
        app.misc.showcmd.clear();
        let selection_cleared = if app.misc.mode == Mode::Doc {
            if app.editor.doc_ed.has_selection() {
                app.editor.doc_ed.clear_selection();
                true
            } else {
                false
            }
        } else if app.editor.ed.has_selection() {
            app.editor.ed.clear_selection();
            true
        } else {
            false
        };
        if selection_cleared {
            return Some(true);
        }
        if app.command_bar.in_command {
            app.command_bar.in_command = false;
            app.misc.showcmd.clear();
            return Some(false);
        }
        if matches!(app.misc.mode, Mode::ScanReport | Mode::ScanHistory) {
            app.misc.mode = app.scan.prev_mode_before_scan;
            return Some(false);
        }
        if app.misc.mode == Mode::Doc {
            return Some(false);
        }
        if app.misc.mode != Mode::Normal {
            app.misc.mode = Mode::Normal;
            return Some(false);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_sidebar_jk_and_arrows_navigate_workspace_files() {
        let mut app = App::new();
        app.sidebar.open = true;
        app.sidebar.focused = true;
        app.workspace.root = Some(PathBuf::from("/workspace"));
        app.workspace.entries = vec![
            crate::workspace::Entry {
                path: PathBuf::from("/workspace/a.rs"),
                depth: 1,
                directory: false,
            },
            crate::workspace::Entry {
                path: PathBuf::from("/workspace/b.rs"),
                depth: 1,
                directory: false,
            },
            crate::workspace::Entry {
                path: PathBuf::from("/workspace/c.rs"),
                depth: 1,
                directory: false,
            },
        ];
        app.workspace.selected_path = Some(PathBuf::from("/workspace/a.rs"));

        let ctx = egui::Context::default();

        // 1. Press 'j' -> moves selection to b.rs
        let raw_j = egui::RawInput {
            events: vec![egui::Event::Key {
                key: egui::Key::J,
                physical_key: Some(egui::Key::J),
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        };
        let _ = ctx.run(raw_j, |ctx| {
            handle_navigation_shortcuts(&mut app, ctx, 1.0);
        });
        assert_eq!(
            app.workspace.selected_path.as_deref(),
            Some(std::path::Path::new("/workspace/b.rs"))
        );
        assert!(app
            .workspace
            .selected_items
            .contains(std::path::Path::new("/workspace/b.rs")));

        // 2. Press ArrowDown -> moves selection to c.rs
        let raw_down = egui::RawInput {
            events: vec![egui::Event::Key {
                key: egui::Key::ArrowDown,
                physical_key: Some(egui::Key::ArrowDown),
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        };
        let _ = ctx.run(raw_down, |ctx| {
            handle_navigation_shortcuts(&mut app, ctx, 1.0);
        });
        assert_eq!(
            app.workspace.selected_path.as_deref(),
            Some(std::path::Path::new("/workspace/c.rs"))
        );

        // 3. Press 'k' -> moves selection back to b.rs
        let raw_k = egui::RawInput {
            events: vec![egui::Event::Key {
                key: egui::Key::K,
                physical_key: Some(egui::Key::K),
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        };
        let _ = ctx.run(raw_k, |ctx| {
            handle_navigation_shortcuts(&mut app, ctx, 1.0);
        });
        assert_eq!(
            app.workspace.selected_path.as_deref(),
            Some(std::path::Path::new("/workspace/b.rs"))
        );

        // 4. Press ArrowUp -> moves selection back to a.rs
        let raw_up = egui::RawInput {
            events: vec![egui::Event::Key {
                key: egui::Key::ArrowUp,
                physical_key: Some(egui::Key::ArrowUp),
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        };
        let _ = ctx.run(raw_up, |ctx| {
            handle_navigation_shortcuts(&mut app, ctx, 1.0);
        });
        assert_eq!(
            app.workspace.selected_path.as_deref(),
            Some(std::path::Path::new("/workspace/a.rs"))
        );
    }

    #[test]
    fn test_sidebar_enter_toggles_directory() {
        let mut app = App::new();
        app.sidebar.open = true;
        app.sidebar.focused = true;
        let root = PathBuf::from("/workspace");
        let dir = PathBuf::from("/workspace/src");
        app.workspace.root = Some(root.clone());
        app.workspace.entries = vec![crate::workspace::Entry {
            path: dir.clone(),
            depth: 1,
            directory: true,
        }];
        app.workspace.selected_path = Some(dir.clone());
        assert!(!app.workspace.expanded.contains(&dir));

        let ctx = egui::Context::default();
        let raw_enter = egui::RawInput {
            events: vec![egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: Some(egui::Key::Enter),
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        };
        let _ = ctx.run(raw_enter, |ctx| {
            handle_navigation_shortcuts(&mut app, ctx, 1.0);
        });
        assert!(app.workspace.expanded.contains(&dir));
        // Focus stays on sidebar when toggling a folder
        assert!(app.sidebar.focused);
    }
}
