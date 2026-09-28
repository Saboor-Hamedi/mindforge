//! Input forwarding for the embedded Neovim editor.

use crate::app::{App, EditorInputMode};
use crate::editor::{backend::EditorBackend, events::EditorKeyEvent};
use crate::mode::Mode;
use crate::vim::PendingVimInput;
use eframe::egui::{self, Event};

/// Route an event to Neovim when its editor surface owns focus.
/// `None` means the event belongs to another app surface or to Hybrid.
pub fn handle_event(app: &mut App, event: &Event, now: f64, has_colon_text: bool) -> Option<bool> {
    let owns_input = app.mode == Mode::Normal
        && app.editor_controller.mode == EditorInputMode::Vim
        && !app.show_welcome
        && !app.open_notes.is_empty()
        && !app.search_open
        && !app.settings_open
        && !app.rename_open
        && !app.delete_confirm_open
        && !app.accent_dropdown_open
        && !app.wikilink_autocomplete.is_active;
    if !owns_input {
        return None;
    }

    let typed = match event {
        Event::Paste(text) => {
            if let Some(backend) = app.vim_runtime.backend.as_mut() {
                if let Err(error) = backend.paste(text) {
                    app.set_status(&format!("Neovim input failed: {error}"), now);
                }
            } else if !app.vim_runtime.queue_input(PendingVimInput::Paste(text.clone())) {
                app.set_status("Neovim startup is taking too long; input queue is full", now);
            }
            true
        }
        Event::Text(text) => {
            if let Some(backend) = app.vim_runtime.backend.as_mut() {
                if let Err(error) = backend.handle_text(text) {
                    app.set_status(&format!("Neovim input failed: {error}"), now);
                }
            } else if !app.vim_runtime.queue_input(PendingVimInput::Text(text.clone())) {
                app.set_status("Neovim startup is taking too long; input queue is full", now);
            }
            true
        }
        Event::Key { key, pressed: true, modifiers, .. } => {
            if *key == egui::Key::Semicolon
                && modifiers.shift
                && !modifiers.ctrl
                && !modifiers.alt
                && !has_colon_text
            {
                send_text(app, ":", now);
                true
            } else {
                let name = format!("{key:?}");
                let is_function_key = name
                    .strip_prefix('F')
                    .and_then(|digits| digits.parse::<u8>().ok())
                    .is_some_and(|number| (1..=35).contains(&number));
                let is_special = is_function_key
                    || matches!(key, egui::Key::Escape | egui::Key::Enter | egui::Key::Tab
                        | egui::Key::Backspace | egui::Key::Delete | egui::Key::ArrowLeft
                        | egui::Key::ArrowRight | egui::Key::ArrowUp | egui::Key::ArrowDown
                        | egui::Key::Home | egui::Key::End | egui::Key::PageUp | egui::Key::PageDown);
                if is_special || modifiers.ctrl || modifiers.command || modifiers.alt {
                    send_key(app, EditorKeyEvent { key: *key, modifiers: *modifiers }, now);
                    true
                } else {
                    false
                }
            }
        }
        _ => false,
    };
    Some(typed)
}

fn send_text(app: &mut App, text: &str, now: f64) {
    if let Some(backend) = app.vim_runtime.backend.as_mut() {
        if let Err(error) = backend.handle_text(text) {
            app.set_status(&format!("Neovim input failed: {error}"), now);
        }
    } else if !app.vim_runtime.queue_input(PendingVimInput::Text(text.into())) {
        app.set_status("Neovim startup is taking too long; input queue is full", now);
    }
}

fn send_key(app: &mut App, event: EditorKeyEvent, now: f64) {
    if let Some(backend) = app.vim_runtime.backend.as_mut() {
        if let Err(error) = backend.handle_key(event) {
            app.set_status(&format!("Neovim input failed: {error}"), now);
        }
    } else if !app.vim_runtime.queue_input(PendingVimInput::Key(event)) {
        app.set_status("Neovim startup is taking too long; input queue is full", now);
    }
}
