//! Global keyboard shortcuts, window events, and surface routing.

pub mod clipboard;
pub mod editing;
pub mod navigation;
pub mod panels;
pub mod tabs;
pub mod window;

pub use clipboard::{get_clipboard_text, get_win32_clipboard, set_win32_clipboard};
pub use window::window_shortcuts;

use crate::app::App;
use eframe::egui;

/// Processes global shortcuts (saving, note creation, modals, clipboard, undo/redo).
/// Returns `Some(typed)` if a global shortcut fully handled the frame, or `None` to continue to typing.
pub fn handle_global_shortcuts(app: &mut App, ctx: &egui::Context, now: f64) -> Option<bool> {
    if app.workspace.dialog.is_some() {
        if ctx.input(|i|i.key_pressed(egui::Key::Escape)) {
            app.workspace.dialog=None;
            app.workspace.dialog_error=None;
            app.workspace.dialog_focus_requested=false;
        }
        return Some(false);
    }
    if app.editor.language_selector.open {
        if ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
            app.editor.language_selector.open = false;
            app.editor.language_selector.query.clear();
        }
        return Some(false);
    }

    let palette_shortcut = ctx.input(|i| {
        let command = i.modifiers.ctrl || i.modifiers.command;
        command && !i.modifiers.alt && i.modifiers.shift && i.key_pressed(egui::Key::P)
    });

    // 0. Active Modal Input Priority:
    // When any modal (Search, Settings, Rename, Delete confirmation, Accent dropdown) is active,
    // absorb global shortcuts so that modal dialogs retain 100% focused input context.
    if app.modal.search_open && !palette_shortcut {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            app.modal.search_open = false;
        }
        return Some(false);
    }
    if app.modal.settings_open {
        return Some(false);
    }
    if app.modal.rename_open {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            app.modal.rename_open = false;
        }
        return Some(false);
    }
    if app.modal.delete_confirm_open {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            app.modal.delete_confirm_open = false;
            app.modal.pending_delete_note_id = None;
        }
        return Some(false);
    }
    if app.misc.accent_dropdown_open {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            app.misc.accent_dropdown_open = false;
        }
        return Some(false);
    }
    if app.services.wikilink_autocomplete.is_active {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            app.services.wikilink_autocomplete.dismiss(app.editor.ed.cur);
            return Some(false);
        }
    }
    if app.services.hover_wikilink.is_active() {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            app.services.hover_wikilink.dismiss();
            return Some(false);
        }
    }

    // 1. Panels and tools toggling (Terminal, AI Agent, Outline, Preview, Sidebar, F2, F3)
    if let Some(res) = panels::handle_panel_shortcuts(app, ctx, now) {
        return Some(res);
    }

    // 2. Tab management shortcuts (Ctrl+N, Ctrl+W, Ctrl+Tab, Ctrl+1..9)
    if let Some(res) = tabs::handle_tab_shortcuts(app, ctx, now) {
        return Some(res);
    }

    // 3. Editing shortcuts (Undo, Redo, Indent, Dedent, Checklist, Quick save, Zen mode)
    if let Some(res) = editing::handle_editing_shortcuts(app, ctx, now) {
        return Some(res);
    }

    // 4. Clipboard operations (Cut, Copy, Paste, Select All)
    if let Some(res) = clipboard::handle_clipboard_shortcuts(app, ctx, now) {
        return Some(res);
    }

    // 5. Surface & Navigation shortcuts (Sidebar, Doc sidebar, Scan view, Escape handling)
    if let Some(res) = navigation::handle_navigation_shortcuts(app, ctx, now) {
        return Some(res);
    }

    None
}
