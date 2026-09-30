//! Tab navigation and tab lifecycle shortcuts (New tab, Close tab, Switch tabs).

use crate::app::App;
use crate::mode::Mode;
use eframe::egui;

pub fn handle_tab_shortcuts(app: &mut App, ctx: &egui::Context, now: f64) -> Option<bool> {
    // New Note (Ctrl+N)
    let ctrl_n = ctx.input(|i| {
        (i.modifiers.ctrl || i.modifiers.command) && !i.modifiers.shift && !i.modifiers.alt && i.key_pressed(egui::Key::N)
    });
    if ctrl_n {
        app.create_new_note(now);
        return Some(false);
    }

    // Close Tab (Ctrl+W)
    let ctrl_w = ctx.input(|i| i.modifiers.ctrl && !i.modifiers.alt && i.key_pressed(egui::Key::W));
    if ctrl_w {
        if app.misc.mode == Mode::Doc {
            app.close_doc_tab(app.tabs.active_doc_tab, now);
        } else {
            app.close_tab(app.tabs.active_tab, now);
        }
        return Some(false);
    }

    // Tab Navigation: Ctrl+Tab and Ctrl+Shift+Tab
    let ctrl_tab = ctx.input(|i| i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::Tab));
    let ctrl_shift_tab = ctx.input(|i| i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::Tab));

    if ctrl_shift_tab {
        if app.misc.mode == Mode::Doc {
            if !app.tabs.open_doc_tabs.is_empty() {
                let n = app.tabs.open_doc_tabs.len();
                let prev_tab = (app.tabs.active_doc_tab + n - 1) % n;
                app.switch_doc_tab(prev_tab, now);
            }
        } else if !app.open_notes.is_empty() {
            let n = app.open_notes.len();
            let prev_tab = (app.tabs.active_tab + n - 1) % n;
            app.switch_tab(prev_tab, now);
        }
        return Some(false);
    }

    if ctrl_tab {
        if app.misc.mode == Mode::Doc {
            if !app.tabs.open_doc_tabs.is_empty() {
                let next_tab = (app.tabs.active_doc_tab + 1) % app.tabs.open_doc_tabs.len();
                app.switch_doc_tab(next_tab, now);
            }
        } else if !app.open_notes.is_empty() {
            let next_tab = (app.tabs.active_tab + 1) % app.open_notes.len();
            app.switch_tab(next_tab, now);
        }
        return Some(false);
    }

    // Numeric Tab Switching: Ctrl+1 through Ctrl+9
    let numeric_tab = ctx.input(|i| {
        if !i.modifiers.ctrl || i.modifiers.shift || i.modifiers.alt {
            return None;
        }
        for (idx, key) in [
            egui::Key::Num1, egui::Key::Num2, egui::Key::Num3,
            egui::Key::Num4, egui::Key::Num5, egui::Key::Num6,
            egui::Key::Num7, egui::Key::Num8, egui::Key::Num9,
        ].iter().enumerate() {
            if i.key_pressed(*key) {
                return Some(idx);
            }
        }
        None
    });

    if let Some(target_idx) = numeric_tab {
        if app.misc.mode == Mode::Doc {
            if target_idx < app.tabs.open_doc_tabs.len() {
                app.switch_doc_tab(target_idx, now);
                return Some(false);
            }
        } else if target_idx < app.open_notes.len() {
            app.switch_tab(target_idx, now);
            return Some(false);
        }
    }

    None
}
