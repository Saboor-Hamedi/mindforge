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
    // Route critical modal shortcuts (Ctrl+P, Ctrl+Shift+P, Ctrl+,) from the raw key event
    // before Vim input or focused widgets can consume them.
    let (ctrl_p, ctrl_shift_p, ctrl_comma) = ctx.input(|i| {
        let pressed = |wanted: egui::Key, shifted: bool| {
            i.events.iter().any(|event| matches!(event,
                egui::Event::Key { key, pressed: true, repeat: false, modifiers, .. }
                    if *key == wanted
                        && (modifiers.ctrl || modifiers.command)
                        && modifiers.shift == shifted
                        && !modifiers.alt
            ))
        };
        (
            pressed(egui::Key::P, false),
            pressed(egui::Key::P, true),
            pressed(egui::Key::Comma, false),
        )
    });

    if ctrl_comma {
        if app.modal.settings_open {
            app.modal.settings_open = false;
        } else {
            app.modal.close_all();
            app.modal.settings_open = true;
            app.modal.settings_just_opened = true;
            app.modal.settings_opened_at = now;
        }
        return false;
    }
    let other_modal_open = app.modal.settings_open
        || app.modal.rename_open
        || app.modal.delete_confirm_open
        || app.misc.accent_dropdown_open;
    if !other_modal_open && ctrl_shift_p {
        if app.modal.search_open && app.modal.search_query.starts_with('>') {
            app.modal.search_open = false;
        } else {
            app.modal.search_open = true;
            app.modal.search_query = ">".to_string();
            app.modal.search_selected = 0;
            app.modal.search_just_opened = true;
            app.modal.search_opened_at = now;
            app.update_search_results();
        }
        return false;
    }
    if !other_modal_open && ctrl_p {
        if app.modal.search_open && !app.modal.search_query.starts_with('>') {
            app.modal.search_open = false;
        } else {
            app.modal.search_open = true;
            app.modal.search_query.clear();
            app.modal.search_selected = 0;
            app.modal.search_just_opened = true;
            app.modal.search_opened_at = now;
            app.update_search_results();
        }
        return false;
    }

    // 1. Check global shortcuts (window close, undo/redo, modal triggers, clipboard, tabs)
    if let Some(typed) = global::handle_global_shortcuts(app, ctx, now) {
        return typed;
    }

    let mut typed = false;
    let vim_normal_mode = app.services.editor_controller.mode == crate::app::EditorInputMode::Vim
        && app.services.vim_runtime.backend.as_ref().is_none_or(|backend| !backend.is_insert_mode());

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
            let command_surface_active = matches!(app.misc.mode, Mode::Normal | Mode::Doc | Mode::ScanReport | Mode::ScanHistory | Mode::Stats)
                && !app.terminal.focused
                && !app.modal.search_open
                && !app.modal.settings_open
                && !app.modal.rename_open
                && !app.modal.delete_confirm_open
                && !app.misc.accent_dropdown_open
                && !app.services.wikilink_autocomplete.is_active;
            let command_prefix = match ev {
                egui::Event::Text(text) if text == ":"
                    && (app.services.editor_controller.mode != crate::app::EditorInputMode::Vim
                        || app.misc.mode != Mode::Normal
                        || app.misc.show_welcome
                        || app.open_notes.is_empty()
                        || vim_normal_mode) => Some(':'),
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
            if !app.command_bar.in_command && command_surface_active && command_prefix.is_some() {
                app.command_bar.in_command = true;
                app.command_bar.prefix = command_prefix.unwrap_or(':');
                app.editor.cmd_ed.clear();
                app.command_bar.selected_idx = 0;
                app.command_bar.navigated = false;
                if app.command_bar.prefix == ':' {
                    app.misc.showcmd.set_command("", now);
                } else {
                    app.misc.showcmd.set_search(&app.command_bar.prefix.to_string(), "", now);
                }
                typed = true;
                continue;
            }
            if app.command_bar.in_command {
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
            } else if app.terminal.open && app.terminal.focused {
                if let egui::Event::Key { key: egui::Key::Escape, pressed: true, modifiers, .. } = ev {
                    if !modifiers.ctrl && !modifiers.shift && !modifiers.alt {
                        app.terminal.focused = false;
                        app.set_status("Editor focused (Ctrl+J to return to terminal)", now);
                        continue;
                    }
                }
                if let Some(ref mut pane) = app.terminal.pane {
                    pane.feed_event(ev, i.modifiers);
                }
            } else if let Some(vim_typed) = crate::vim::input::handle_event(app, ev, now, has_text_event, has_colon_text) {
                typed |= vim_typed;
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
        app.misc.show_welcome = false;
    }

    typed
}
