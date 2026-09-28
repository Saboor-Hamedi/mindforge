//! Keyboard shortcut routing, text input, and command/editor event dispatch.

pub mod editor;
pub mod global;

pub use global::window_shortcuts;

use crate::app::App;
use crate::mode::Mode;
use eframe::egui;

/// Processes all keyboard shortcuts and text typing.
/// Returns true if a character or text edit occurred in the editor.
pub fn handle_input(app: &mut App, ctx: &egui::Context, now: f64) -> bool {
    // Route the two palette shortcuts from the raw key event before Vim input
    // or focused widgets can consume them. Ctrl+P searches notes; Ctrl+Shift+P
    // opens built-in commands even when the notes database is empty.
    let (ctrl_p, ctrl_shift_p) = ctx.input(|i| {
        let pressed = |wanted: egui::Key, shifted: bool| {
            i.events.iter().any(|event| matches!(event,
                egui::Event::Key { key, pressed: true, repeat: false, modifiers, .. }
                    if *key == wanted
                        && (modifiers.ctrl || modifiers.command)
                        && modifiers.shift == shifted
                        && !modifiers.alt
            ))
        };
        (pressed(egui::Key::P, false), pressed(egui::Key::P, true))
    });
    let other_modal_open = app.settings_open
        || app.rename_open
        || app.delete_confirm_open
        || app.accent_dropdown_open;
    if !other_modal_open && ctrl_shift_p {
        if app.search_open && app.search_query.starts_with('>') {
            app.search_open = false;
        } else {
            app.search_open = true;
            app.search_query = ">".to_string();
            app.search_selected = 0;
            app.search_just_opened = true;
            app.update_search_results();
        }
        return false;
    }
    if !other_modal_open && ctrl_p {
        if app.search_open && !app.search_query.starts_with('>') {
            app.search_open = false;
        } else {
            app.search_open = true;
            app.search_query.clear();
            app.search_selected = 0;
            app.search_just_opened = true;
            app.update_search_results();
        }
        return false;
    }

    // 1. Check global shortcuts (window close, undo/redo, modal triggers, clipboard, tabs)
    if let Some(typed) = global::handle_global_shortcuts(app, ctx, now) {
        return typed;
    }

    let mut typed = false;
    let vim_normal_mode = app.editor_controller.mode == crate::app::EditorInputMode::Vim
        && app.vim_runtime.backend.as_ref().is_none_or(|backend| !backend.is_insert_mode());

    // 2. Dispatch events for either command bar, focused terminal, or active editor
    ctx.input(|i| {
        // Some keyboard layouts/backends report Shift+; as a key event without
        // the corresponding text event. Neovim needs the resulting ':' text to
        // enter its command line, while avoiding a duplicate when egui already
        // supplied the character normally.
        let has_colon_text = i.events.iter().any(|event| {
            matches!(event, egui::Event::Text(text) if text.contains(':'))
        });
        let has_text_event = i.events.iter().any(|event| matches!(event, egui::Event::Text(text) if !text.is_empty()));
        for ev in &i.events {
            let command_surface_active = app.mode == Mode::Normal
                && !app.show_welcome
                && !app.open_notes.is_empty()
                && !app.terminal_focused
                && !app.search_open
                && !app.settings_open
                && !app.rename_open
                && !app.delete_confirm_open
                && !app.accent_dropdown_open
                && !app.wikilink_autocomplete.is_active;
            let command_prefix = match ev {
                egui::Event::Text(text) if text == ":"
                    && (app.editor_controller.mode != crate::app::EditorInputMode::Vim || vim_normal_mode) => Some(':'),
                egui::Event::Text(text) if text == "/" && vim_normal_mode => Some('/'),
                egui::Event::Text(text) if text == "?"
                    && vim_normal_mode => Some('?'),
                egui::Event::Key { key: egui::Key::Semicolon, pressed: true, modifiers, .. }
                    if modifiers.shift && !modifiers.ctrl && !modifiers.alt && !has_colon_text => Some(':'),
                egui::Event::Key { key: egui::Key::Slash, pressed: true, modifiers, .. }
                    if !has_text_event && !modifiers.ctrl && !modifiers.alt
                        && vim_normal_mode =>
                    Some(if modifiers.shift { '?' } else { '/' }),
                _ => None,
            };
            if !app.in_command && command_surface_active && command_prefix.is_some() {
                app.in_command = true;
                app.cmd_prefix = command_prefix.unwrap_or(':');
                app.cmd_ed.clear();
                app.cmd_selected_idx = 0;
                app.cmd_navigated = false;
                if app.cmd_prefix == ':' {
                    app.showcmd.set_command("", now);
                } else {
                    app.showcmd.set_search(&app.cmd_prefix.to_string(), "", now);
                }
                typed = true;
                continue;
            }
            if app.in_command {
                match ev {
                    egui::Event::Paste(s) => {
                        crate::command::input::handle_command_paste(app, s, now);
                        typed = true;
                    }
                    egui::Event::Text(s) => {
                        crate::command::input::handle_command_text(app, s, now);
                        typed = true;
                    }
                    egui::Event::Key { key, pressed: true, modifiers, .. } => {
                        crate::command::input::handle_command_key(app, *key, *modifiers, now);
                        typed = true;
                    }
                    _ => {}
                }
            } else if app.terminal_open && app.terminal_focused {
                if let egui::Event::Key { key: egui::Key::Escape, pressed: true, modifiers, .. } = ev {
                    if !modifiers.ctrl && !modifiers.shift && !modifiers.alt {
                        app.terminal_focused = false;
                        app.set_status("Editor focused (Ctrl+J to return to terminal)", now);
                        continue;
                    }
                }
                if let Some(ref mut pane) = app.term_pane {
                    pane.feed_event(ev, i.modifiers);
                }
            } else if let Some(vim_typed) = crate::vim::input::handle_event(app, ev, now, has_text_event, has_colon_text) {
                typed |= vim_typed;
            } else if app.mode == Mode::Normal && (app.show_welcome || app.open_notes.is_empty()) {
                // When on Welcome dashboard, hotkeys are handled directly by the dashboard or modals
                if app.editor_controller.mode == crate::app::EditorInputMode::Vim {
                    if let egui::Event::Text(ref s) = ev {
                        if s == ":" {
                            app.in_command = true;
                            app.cmd_ed.clear();
                            app.cmd_selected_idx = 0;
                            app.cmd_navigated = false;
                            app.showcmd.set_command("", now);
                            typed = true;
                        }
                    }
                }
            } else {
                match ev {
                    egui::Event::Paste(s) => {
                        if editor::handle_editor_paste(app, s, now) {
                            typed = true;
                        }
                    }
                    egui::Event::Text(s) => {
                        if editor::handle_editor_text(app, s, now) {
                            typed = true;
                        }
                    }
                    egui::Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } => {
                        if editor::handle_editor_key(app, *key, *modifiers, now) {
                            typed = true;
                        }
                    }
                    _ => {}
                }
            }
        }
    });

    if typed && !app.open_notes.is_empty() {
        app.show_welcome = false;
    }

    typed
}
