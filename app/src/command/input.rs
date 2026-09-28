//! Command bar (`:`) input handling (typing, navigation, backspace, and execution).

use crate::app::App;
use crate::command::command_suggestion;
use eframe::egui::{Key, Modifiers};

fn update_command_hud(app: &mut App, now: f64) {
    if app.cmd_prefix == ':' {
        app.showcmd.set_command(&app.cmd_ed.text(), now);
    } else {
        app.showcmd.set_search(&app.cmd_prefix.to_string(), &app.cmd_ed.text(), now);
    }
}

fn send_search_to_vim(app: &mut App, prefix: char, query: &str, now: f64) {
    if query.is_empty() {
        return;
    }
    if let Some(backend) = app.vim_runtime.backend.as_mut() {
        if let Err(error) = backend.send_input(&format!("{}{}<CR>", prefix, query)) {
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
            app.cmd_ed.insert(c);
        }
    }
    app.cmd_selected_idx = 0;
    app.cmd_navigated = false;
    update_command_hud(app, now);
    app.last_char_time = now;
}

/// Handles character typing into the command bar buffer.
pub fn handle_command_text(app: &mut App, s: &str, now: f64) {
    for c in s.chars() {
        app.cmd_ed.insert(c);
    }
    app.cmd_selected_idx = 0;
    app.cmd_navigated = false;
    update_command_hud(app, now);
    app.last_char_time = now;
}

/// Dispatches keystrokes in command mode: autocompletion, cursor movement, editing.
pub fn handle_command_key(app: &mut App, key: Key, modifiers: Modifiers, now: f64) {
    if app.cmd_prefix != ':' && key == Key::Enter {
        let prefix = app.cmd_prefix;
        let query = app.cmd_ed.text();
        app.in_command = false;
        app.cmd_ed.clear();
        app.cmd_prefix = ':';
        app.showcmd.record_action(&format!("{}{}", prefix, query), now);
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
            app.in_command = false;
            app.cmd_navigated = false;
            app.cmd_selected_idx = 0;
            app.cmd_ed.clear();
            app.showcmd.clear();
            app.cmd_prefix = ':';
        }
        Key::Backspace if modifiers.ctrl => {
            app.cmd_ed.delete_word();
            app.cmd_selected_idx = 0;
            app.cmd_navigated = false;
            update_command_hud(app, now);
        }
        Key::Backspace => {
            if app.cmd_ed.cur == 0 && !app.cmd_ed.has_selection() {
                app.in_command = false;
                app.cmd_selected_idx = 0;
                app.cmd_navigated = false;
                app.showcmd.clear();
                app.cmd_prefix = ':';
            } else {
                app.cmd_ed.backspace();
                app.cmd_selected_idx = 0;
                app.cmd_navigated = false;
                update_command_hud(app, now);
            }
        }
        Key::Delete if modifiers.ctrl => {
            app.cmd_ed.delete_word_forward();
            app.cmd_selected_idx = 0;
            app.cmd_navigated = false;
            update_command_hud(app, now);
        }
        Key::Delete => {
            app.cmd_ed.delete();
            app.cmd_selected_idx = 0;
            app.cmd_navigated = false;
            update_command_hud(app, now);
        }
        Key::ArrowLeft if modifiers.ctrl && modifiers.shift => {
            app.cmd_ed.word_left_select();
        }
        Key::ArrowLeft if modifiers.ctrl => {
            app.cmd_ed.word_left();
        }
        Key::ArrowLeft if modifiers.shift => {
            app.cmd_ed.left_select();
        }
        Key::ArrowLeft => {
            app.cmd_ed.left();
        }
        Key::ArrowRight if modifiers.ctrl && modifiers.shift => {
            app.cmd_ed.word_right_select();
        }
        Key::ArrowRight if modifiers.ctrl => {
            app.cmd_ed.word_right();
        }
        Key::ArrowRight if modifiers.shift => {
            app.cmd_ed.right_select();
        }
        Key::ArrowRight => {
            app.cmd_ed.right();
        }
        Key::Home => {
            app.cmd_ed.home();
        }
        Key::End => {
            app.cmd_ed.end();
        }
        Key::A if modifiers.ctrl => {
            app.cmd_ed.select_all();
        }
        Key::C if modifiers.ctrl => {
            if let Some(t) = app.cmd_ed.selected_text() {
                app.clipboard_text = Some(t.clone());
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
        app.in_command = true;
        handle_command_text(&mut app, "set nu", 0.0);
        assert_eq!(app.cmd_ed.text(), "set nu");
        assert_eq!(app.showcmd.text, ":set nu");

        handle_command_key(&mut app, Key::Backspace, Modifiers::NONE, 0.0);
        assert_eq!(app.cmd_ed.text(), "set n");

        handle_command_key(&mut app, Key::Escape, Modifiers::NONE, 0.0);
        assert!(!app.in_command);
        assert_eq!(app.cmd_ed.text(), "");
    }

    #[test]
    fn test_command_colon_input_no_freeze() {
        let mut app = App::new();
        app.in_command = true;
        handle_command_text(&mut app, ":", 0.0);
        assert_eq!(app.cmd_ed.text(), ":");
        handle_command_text(&mut app, "w", 0.0);
        assert_eq!(app.cmd_ed.text(), ":w");
    }

    #[test]
    fn test_command_navigation_and_tab_complete() {
        let mut app = App::new();
        app.in_command = true;
        handle_command_text(&mut app, "s", 0.0);

        // Ctrl+J navigates down
        handle_command_key(&mut app, Key::J, Modifiers::CTRL, 0.0);
        assert!(app.cmd_navigated);

        // Tab completes the selection
        handle_command_key(&mut app, Key::Tab, Modifiers::NONE, 0.0);
        assert!(!app.cmd_ed.text().is_empty());
        assert!(app.cmd_ed.text().starts_with("s"));

        // Enter records to history
        handle_command_key(&mut app, Key::Enter, Modifiers::NONE, 0.0);
        assert!(!app.in_command);
        assert!(!app.command_history.is_empty());
    }
}
