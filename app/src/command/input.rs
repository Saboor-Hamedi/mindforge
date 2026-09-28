//! Command bar (`:`) input handling (typing, navigation, backspace, and execution).

use crate::app::App;
use crate::command::command_suggestion;
use eframe::egui::{Key, Modifiers};

fn update_command_hud(app: &mut App, now: f64) {
    if app.command_bar.prefix == ':' {
        app.misc.showcmd.set_command(&app.editor.cmd_ed.text(), now);
    } else {
        app.misc.showcmd.set_search(&app.command_bar.prefix.to_string(), &app.editor.cmd_ed.text(), now);
        if let Some(backend) = app.services.vim_runtime.backend.as_mut() {
            if let Err(error) = backend.preview_search(&app.editor.cmd_ed.text()) {
                app.set_status(&format!("Neovim search preview failed: {error}"), now);
            }
        }
    }
}

fn send_search_to_vim(app: &mut App, prefix: char, query: &str, now: f64) {
    if query.is_empty() {
        return;
    }
    if let Some(backend) = app.services.vim_runtime.backend.as_mut() {
        if let Err(error) = backend.search(query, prefix == '?') {
            app.set_status(&format!("Neovim search failed: {error}"), now);
        }
    } else {
        app.set_status("Neovim is still starting; search was not sent", now);
    }
}

/// Handles pasting text into the command bar buffer.
pub fn handle_command_paste(app: &mut App, s: &str, now: f64) {
    for c in s.chars() {
        if c != '\n' && c != '\r' {
            app.editor.cmd_ed.insert(c);
        }
    }
    app.command_bar.selected_idx = 0;
    app.command_bar.navigated = false;
    update_command_hud(app, now);
    app.misc.last_char_time = now;
}

/// Handles character typing into the command bar buffer.
pub fn handle_command_text(app: &mut App, s: &str, now: f64) {
    for c in s.chars() {
        app.editor.cmd_ed.insert(c);
    }
    app.command_bar.selected_idx = 0;
    app.command_bar.navigated = false;
    update_command_hud(app, now);
    app.misc.last_char_time = now;
}

/// Dispatches keystrokes in command mode: autocompletion, cursor movement, editing.
pub fn handle_command_key(app: &mut App, key: Key, modifiers: Modifiers, now: f64) {
    if app.command_bar.prefix != ':' && key == Key::Enter {
        let prefix = app.command_bar.prefix;
        let query = app.editor.cmd_ed.text();
        app.command_bar.in_command = false;
        app.editor.cmd_ed.clear();
        app.command_bar.prefix = ':';
        app.misc.showcmd.record_action(&format!("{}{}", prefix, query), now);
        if let Some(backend) = app.services.vim_runtime.backend.as_mut() {
            let _ = backend.clear_preview_search();
        }
        send_search_to_vim(app, prefix, &query, now);
        return;
    }
    // 1. Suggestion navigation & execution (Enter, Tab, Ctrl+J/K, ArrowUp/Down)
    if command_suggestion::handle_suggestion_key(app, key, modifiers, now) {
        return;
    }

    // 2. Standard command line editing & navigation
    match key {
        Key::Escape => {
            app.command_bar.in_command = false;
            app.command_bar.navigated = false;
            app.command_bar.selected_idx = 0;
            app.editor.cmd_ed.clear();
            app.misc.showcmd.clear();
            app.command_bar.prefix = ':';
            if let Some(backend) = app.services.vim_runtime.backend.as_mut() {
                let _ = backend.clear_preview_search();
            }
        }
        Key::Backspace if modifiers.ctrl => {
            app.editor.cmd_ed.delete_word();
            app.command_bar.selected_idx = 0;
            app.command_bar.navigated = false;
            update_command_hud(app, now);
        }
        Key::Backspace => {
            if app.editor.cmd_ed.cur == 0 && !app.editor.cmd_ed.has_selection() {
                app.command_bar.in_command = false;
                app.command_bar.selected_idx = 0;
                app.command_bar.navigated = false;
                app.misc.showcmd.clear();
                app.command_bar.prefix = ':';
                if let Some(backend) = app.services.vim_runtime.backend.as_mut() {
                    let _ = backend.clear_preview_search();
                }
            } else {
                app.editor.cmd_ed.backspace();
                app.command_bar.selected_idx = 0;
                app.command_bar.navigated = false;
                update_command_hud(app, now);
            }
        }
        Key::Delete if modifiers.ctrl => {
            app.editor.cmd_ed.delete_word_forward();
            app.command_bar.selected_idx = 0;
            app.command_bar.navigated = false;
            update_command_hud(app, now);
        }
        Key::Delete => {
            app.editor.cmd_ed.delete();
            app.command_bar.selected_idx = 0;
            app.command_bar.navigated = false;
            update_command_hud(app, now);
        }
        Key::ArrowLeft if modifiers.ctrl && modifiers.shift => {
            app.editor.cmd_ed.word_left_select();
        }
        Key::ArrowLeft if modifiers.ctrl => {
            app.editor.cmd_ed.word_left();
        }
        Key::ArrowLeft if modifiers.shift => {
            app.editor.cmd_ed.left_select();
        }
        Key::ArrowLeft => {
            app.editor.cmd_ed.left();
        }
        Key::ArrowRight if modifiers.ctrl && modifiers.shift => {
            app.editor.cmd_ed.word_right_select();
        }
        Key::ArrowRight if modifiers.ctrl => {
            app.editor.cmd_ed.word_right();
        }
        Key::ArrowRight if modifiers.shift => {
            app.editor.cmd_ed.right_select();
        }
        Key::ArrowRight => {
            app.editor.cmd_ed.right();
        }
        Key::Home => {
            app.editor.cmd_ed.home();
        }
        Key::End => {
            app.editor.cmd_ed.end();
        }
        Key::A if modifiers.ctrl => {
            app.editor.cmd_ed.select_all();
        }
        Key::C if modifiers.ctrl => {
            if let Some(t) = app.editor.cmd_ed.selected_text() {
                app.misc.clipboard_text = Some(t.clone());
                crate::input::global::set_win32_clipboard(&t);
            }
        }
        Key::V if modifiers.ctrl => {
            if let Some(text) = crate::input::global::get_clipboard_text(app) {
                handle_command_paste(app, &text, now);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_text_and_backspace() {
        let mut app = App::new();
        app.command_bar.in_command = true;
        handle_command_text(&mut app, "set nu", 0.0);
        assert_eq!(app.editor.cmd_ed.text(), "set nu");
        assert_eq!(app.misc.showcmd.text, ":set nu");

        handle_command_key(&mut app, Key::Backspace, Modifiers::NONE, 0.0);
        assert_eq!(app.editor.cmd_ed.text(), "set n");

        handle_command_key(&mut app, Key::Escape, Modifiers::NONE, 0.0);
        assert!(!app.command_bar.in_command);
        assert_eq!(app.editor.cmd_ed.text(), "");
    }

    #[test]
    fn test_command_colon_input_no_freeze() {
        let mut app = App::new();
        app.command_bar.in_command = true;
        handle_command_text(&mut app, ":", 0.0);
        assert_eq!(app.editor.cmd_ed.text(), ":");
        handle_command_text(&mut app, "w", 0.0);
        assert_eq!(app.editor.cmd_ed.text(), ":w");
    }

    #[test]
    fn test_command_navigation_and_tab_complete() {
        let mut app = App::new();
        app.command_bar.in_command = true;
        handle_command_text(&mut app, "s", 0.0);

        // Ctrl+J navigates down
        handle_command_key(&mut app, Key::J, Modifiers::CTRL, 0.0);
        assert!(app.command_bar.navigated);

        // Tab completes the selection
        handle_command_key(&mut app, Key::Tab, Modifiers::NONE, 0.0);
        assert!(!app.editor.cmd_ed.text().is_empty());
        assert!(app.editor.cmd_ed.text().starts_with("s"));

        // Enter records to history
        handle_command_key(&mut app, Key::Enter, Modifiers::NONE, 0.0);
        assert!(!app.command_bar.in_command);
        assert!(!app.command_bar.history.is_empty());
    }
}
