//! Surface keyboard navigation (Sidebar, Documentation Sidebar, Scan View, and Escape dismissal).

use crate::app::App;
use crate::editor::backend::EditorBackend;
use crate::mode::Mode;
use eframe::egui;

pub fn handle_navigation_shortcuts(app: &mut App, ctx: &egui::Context, now: f64) -> Option<bool> {
    // 1. Sidebar Keyboard Navigation
    // Critical fix: guarded by !app.command_bar.in_command so typing ':' never gets consumed by the sidebar!
    if app.sidebar.open && app.sidebar.focused && !app.command_bar.in_command {
        let (sb_up, sb_down, sb_enter, sb_esc, sb_edit, sb_to_editor, sb_tab) = ctx.input(|i| (
            (!i.modifiers.ctrl && !i.modifiers.alt && i.key_pressed(egui::Key::K)) || i.key_pressed(egui::Key::ArrowUp),
            (!i.modifiers.ctrl && !i.modifiers.alt && i.key_pressed(egui::Key::J)) || i.key_pressed(egui::Key::ArrowDown),
            i.key_pressed(egui::Key::Enter),
            i.key_pressed(egui::Key::Escape),
            !i.modifiers.ctrl && !i.modifiers.alt && i.key_pressed(egui::Key::I),
            (!i.modifiers.alt && (i.key_pressed(egui::Key::L) || i.key_pressed(egui::Key::ArrowRight))),
            !i.modifiers.ctrl && !i.modifiers.alt && i.key_pressed(egui::Key::Tab),
        ));

        if sb_esc || sb_to_editor || sb_tab {
            app.sidebar.focused = false;
            app.set_status("Editor active (Ctrl+H to return to Sidebar)", now);
            return Some(false);
        }

        if sb_edit {
            app.sidebar.focused = false;
            if app.services.editor_controller.mode == crate::app::EditorInputMode::Vim {
                if let Some(backend) = app.services.vim_runtime.backend.as_mut() {
                    let _ = backend.handle_text("i");
                } else {
                    app.services.vim_runtime.queue_input(crate::vim::PendingVimInput::Text("i".into()));
                }
            }
            return Some(false);
        }

        if sb_down {
            if !app.notes.notes_list.is_empty() && app.sidebar.selected_idx + 1 < app.notes.notes_list.len() {
                app.sidebar.selected_idx += 1;
                app.sidebar.needs_scroll = true;
            }
            return Some(false);
        }

        if sb_up {
            if app.sidebar.selected_idx > 0 {
                app.sidebar.selected_idx -= 1;
                app.sidebar.needs_scroll = true;
            }
            return Some(false);
        }

        if sb_enter {
            if let Some(note) = app.notes.notes_list.get(app.sidebar.selected_idx).cloned() {
                app.load_note(note.id, note.topic, note.body, now);
                app.sidebar.focused = true;
                app.set_status("Loaded note (sidebar active — use j/k to move)", now);
            }
            return Some(false);
        }

        return Some(false);
    }

    // 2. Documentation Sidebar Navigation
    if app.misc.mode == Mode::Doc && app.tabs.doc_sidebar_focused && !app.command_bar.in_command {
        let (doc_up, doc_down, doc_enter, doc_esc, doc_to_reader) = ctx.input(|i| (
            (!i.modifiers.ctrl && !i.modifiers.alt && i.key_pressed(egui::Key::K)) || i.key_pressed(egui::Key::ArrowUp),
            (!i.modifiers.ctrl && !i.modifiers.alt && i.key_pressed(egui::Key::J)) || i.key_pressed(egui::Key::ArrowDown),
            i.key_pressed(egui::Key::Enter),
            i.key_pressed(egui::Key::Escape),
            (!i.modifiers.alt && (i.key_pressed(egui::Key::L) || i.key_pressed(egui::Key::ArrowRight))),
        ));

        if doc_esc {
            app.tabs.doc_sidebar_focused = false;
            app.set_status("Doc reader active (Esc returns to the sidebar)", now);
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
    if matches!(app.misc.mode, Mode::ScanReport | Mode::ScanHistory) && !app.command_bar.in_command {
        let (scan_up, scan_down, scan_esc) = ctx.input(|i| (
            (!i.modifiers.ctrl && !i.modifiers.alt && i.key_pressed(egui::Key::K)) || i.key_pressed(egui::Key::ArrowUp),
            (!i.modifiers.ctrl && !i.modifiers.alt && i.key_pressed(egui::Key::J)) || i.key_pressed(egui::Key::ArrowDown),
            i.key_pressed(egui::Key::Escape),
        ));

        if scan_esc {
            app.misc.mode = app.scan.prev_mode_before_scan;
            app.set_status("Exited Scanner", now);
            return Some(false);
        }

        if scan_down {
            app.scan.scan_report_scroll_y = (app.scan.scan_report_scroll_y + 40.0).clamp(0.0, 5000.0);
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
        if app.services.editor_controller.mode == crate::app::EditorInputMode::Vim && !app.command_bar.in_command {
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
