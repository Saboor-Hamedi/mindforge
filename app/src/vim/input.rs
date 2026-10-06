//! Input forwarding for the embedded Neovim editor.

use crate::app::App;
use crate::editor::{backend::EditorBackend, events::EditorKeyEvent};
use crate::mode::Mode;
use crate::vim::PendingVimInput;
use eframe::egui::{self, Event};

/// Route an event to Neovim when its editor surface owns focus.
pub fn handle_event(
    app: &mut App,
    event: &Event,
    now: f64,
    has_text_event: bool,
    has_colon_text: bool,
) -> Option<bool> {
    let owns_input = matches!(app.misc.mode, Mode::Normal | Mode::Doc)
        && (!app.misc.show_welcome || app.misc.mode == Mode::Doc)
        && (!app.open_notes.is_empty() || app.misc.mode == Mode::Doc)
        && !app.modal.search_open
        && !app.modal.settings_open
        && !app.modal.rename_open
        && !app.modal.delete_confirm_open
        && !app.misc.accent_dropdown_open
        && !app.services.wikilink_autocomplete.is_active;
    if !owns_input {
        return None;
    }

    let typed = match event {
        Event::Paste(text) => {
            send_paste(app, text, now);
            true
        }
        Event::Text(text) => {
            send_text(app, text, now);
            true
        }
        Event::Key {
            key,
            pressed: true,
            modifiers,
            ..
        } => {
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
                    || matches!(
                        key,
                        egui::Key::Escape
                            | egui::Key::Enter
                            | egui::Key::Tab
                            | egui::Key::Backspace
                            | egui::Key::Delete
                            | egui::Key::ArrowLeft
                            | egui::Key::ArrowRight
                            | egui::Key::ArrowUp
                            | egui::Key::ArrowDown
                            | egui::Key::Home
                            | egui::Key::End
                            | egui::Key::PageUp
                            | egui::Key::PageDown
                    );
                if is_special || modifiers.ctrl || modifiers.command || modifiers.alt {
                    send_key(
                        app,
                        EditorKeyEvent {
                            key: *key,
                            modifiers: *modifiers,
                        },
                        now,
                    );
                    true
                } else if !has_text_event {
                    if let Some(text) = literal_key_text(*key, modifiers.shift) {
                        send_text(app, &text, now);
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
        }
        _ => false,
    };
    Some(typed)
}

fn literal_key_text(key: egui::Key, shifted: bool) -> Option<String> {
    let name = format!("{key:?}");
    if name.len() == 1 && name.as_bytes()[0].is_ascii_alphabetic() {
        return Some(if shifted {
            name.to_uppercase()
        } else {
            name.to_lowercase()
        });
    }
    let pair = match name.as_str() {
        "OpenBracket" => ('[', '{'),
        "CloseBracket" => (']', '}'),
        "Semicolon" => (';', ':'),
        "Comma" => (',', '<'),
        "Period" => ('.', '>'),
        "Slash" => ('/', '?'),
        "Backslash" => ('\\', '|'),
        "Backtick" => ('`', '~'),
        "Minus" => ('-', '_'),
        "Equals" => ('=', '+'),
        "Quote" => ('\'', '"'),
        _ => return None,
    };
    Some(if shifted { pair.1 } else { pair.0 }.to_string())
}

fn send_text(app: &mut App, text: &str, now: f64) {
    if !text.is_empty() {
        app.misc.sound.play();
        app.misc.last_char_time = now;
        let normal_mode = app
            .services
            .vim_runtime
            .backend
            .as_ref()
            .is_some_and(|backend| !backend.is_insert_mode());
        if normal_mode {
            if app.misc.showcmd.is_pending
                && app.misc.showcmd.kind == crate::ui::showcmd::ShowCmdKind::Keystroke
            {
                let sequence = format!("{}{}", app.misc.showcmd.text, text);
                app.misc.showcmd.record_action(&sequence, now);
            } else if matches!(text, "d" | "c" | "y" | "g" | "z") {
                app.misc.showcmd.set_pending(text, now);
            } else {
                app.misc.showcmd.record_action(text, now);
            }
        }
    }
    if let Some(backend) = app.services.vim_runtime.backend.as_mut() {
        if let Err(error) = backend.handle_text(text) {
            app.set_status(&format!("Neovim input failed: {error}"), now);
        }
    } else if !app
        .services
        .vim_runtime
        .queue_input(PendingVimInput::Text(text.into()))
    {
        app.set_status(
            "Neovim startup is taking too long; input queue is full",
            now,
        );
    }
}

fn send_paste(app: &mut App, text: &str, now: f64) {
    if !text.is_empty() {
        app.misc.sound.play();
        app.misc.last_char_time = now;
    }
    if let Some(backend) = app.services.vim_runtime.backend.as_mut() {
        if let Err(error) = backend.paste(text) {
            app.set_status(&format!("Neovim input failed: {error}"), now);
        }
    } else if !app
        .services
        .vim_runtime
        .queue_input(PendingVimInput::Paste(text.into()))
    {
        app.set_status(
            "Neovim startup is taking too long; input queue is full",
            now,
        );
    }
}

fn send_key(app: &mut App, event: EditorKeyEvent, now: f64) {
    app.misc.sound.play();
    app.misc.last_char_time = now;
    if app
        .services
        .vim_runtime
        .backend
        .as_ref()
        .is_some_and(|backend| !backend.is_insert_mode())
    {
        app.misc
            .showcmd
            .record_action(&format!("{:?}", event.key), now);
    }
    if let Some(backend) = app.services.vim_runtime.backend.as_mut() {
        if let Err(error) = backend.handle_key(event) {
            app.set_status(&format!("Neovim input failed: {error}"), now);
        }
    } else if !app
        .services
        .vim_runtime
        .queue_input(PendingVimInput::Key(event))
    {
        app.set_status(
            "Neovim startup is taking too long; input queue is full",
            now,
        );
    }
}
