//! Editor mode typing, paste, and navigation routing for Hybrid mode and Doc mode.
//! Vim mode input is routed directly to the embedded Neovim backend.

use crate::app::{App, EditorInputMode};
use crate::mode::Mode;
use eframe::egui::{Key, Modifiers};

#[allow(dead_code)]
pub fn handle_editor_paste(app: &mut App, s: &str, now: f64) -> bool {
    if app.misc.mode != Mode::Normal && app.misc.mode != Mode::Doc {
        return false;
    }
    if matches!(app.misc.mode, Mode::Normal | Mode::Doc) && app.services.editor_controller.mode == EditorInputMode::Vim {
        return false;
    }

    let is_doc = app.misc.mode == Mode::Doc;
    let target_ed = if is_doc { &mut app.editor.doc_ed } else { &mut app.editor.ed };
    for c in s.chars() {
        if c != '\r' {
            target_ed.insert(c);
        }
    }
    if !s.is_empty() {
        app.misc.sound.play();
        app.misc.last_char_time = now;
        if !is_doc {
            app.editor.is_dirty = true;
        }
    }
    true
}

pub fn handle_editor_text(app: &mut App, s: &str, now: f64) -> bool {
    if matches!(app.misc.mode, Mode::Normal | Mode::Doc) && app.services.editor_controller.mode == EditorInputMode::Vim {
        return false;
    }

    if app.misc.mode != Mode::Normal && app.misc.mode != Mode::Doc {
        return false;
    }

    let is_doc = app.misc.mode == Mode::Doc;
    let target_ed = if is_doc { &mut app.editor.doc_ed } else { &mut app.editor.ed };
    for c in s.chars() {
        if c == '\n' || c == '\r' {
            continue;
        }
        if app.services.editor_controller.mode == EditorInputMode::Hybrid && app.services.hybrid.handle_char(target_ed, c) {
            app.misc.sound.play();
        } else {
            target_ed.insert(c);
            app.misc.sound.play();
        }
    }
    app.misc.last_char_time = now;
    if !is_doc {
        app.editor.is_dirty = true;
    }
    true
}

pub fn handle_editor_key(app: &mut App, key: Key, modifiers: Modifiers, now: f64) -> bool {
    let is_doc = app.misc.mode == Mode::Doc;

    if matches!(app.misc.mode, Mode::Normal | Mode::Doc) && app.services.editor_controller.mode == EditorInputMode::Vim {
        return false;
    }

    // Wikilink Navigation on Enter: when cursor is inside [[link]], hitting Enter follows it
    if app.misc.mode == Mode::Normal
        && !app.services.wikilink_autocomplete.is_active
        && key == Key::Enter
        && !modifiers.shift
        && !modifiers.alt
    {
        if modifiers.ctrl {
            let text = app.editor.ed.text();
            let links = crate::wikilink::extract_wikilinks(&text);
            if let Some(link) = links.into_iter().find(|l| app.editor.ed.cur >= l.start && app.editor.ed.cur <= l.end) {
                app.follow_wikilink(&link.target, now);
                app.services.hover_wikilink.clear();
                return true;
            }
        }
    }

    let target_ed = if is_doc { &mut app.editor.doc_ed } else { &mut app.editor.ed };

    // Hybrid mode specific key handling
    if app.services.editor_controller.mode == EditorInputMode::Hybrid {
        let buf_before = if !is_doc { Some(target_ed.buf.clone()) } else { None };
        if app.services.hybrid.handle_key(target_ed, key, modifiers) {
            app.misc.sound.play();
            app.misc.last_char_time = now;
            if let Some(ref before) = buf_before {
                if target_ed.buf != *before {
                    app.editor.is_dirty = true;
                }
            }
            return true;
        }
    }

    use Key::*;
    match key {
        H if modifiers.ctrl && !modifiers.alt && !modifiers.shift => {
            target_ed.backspace();
            if !is_doc { app.editor.is_dirty = true; }
            app.misc.sound.play();
            return true;
        }
        Enter if modifiers.ctrl || modifiers.command => {
            target_ed.insert_line_below();
            if !is_doc {
                app.editor.is_dirty = true;
            }
            app.misc.sound.play();
            app.misc.last_char_time = now;
            return true;
        }
        Enter => {
            target_ed.handle_enter();
            if !is_doc {
                app.editor.is_dirty = true;
            }
            app.misc.sound.play();
            app.misc.last_char_time = now;
            return true;
        }
        Backspace if modifiers.ctrl => {
            target_ed.delete_word();
            if !is_doc {
                app.editor.is_dirty = true;
            }
            app.misc.sound.play();
            return true;
        }
        Backspace => {
            target_ed.backspace();
            if !is_doc {
                app.editor.is_dirty = true;
            }
            app.misc.sound.play();
            return true;
        }
        Delete => {
            target_ed.delete();
            if !is_doc {
                app.editor.is_dirty = true;
            }
            app.misc.sound.play();
            return true;
        }
        ArrowLeft if modifiers.shift => {
            target_ed.left_select();
            return true;
        }
        ArrowLeft => {
            target_ed.left();
            return true;
        }
        ArrowRight if modifiers.shift => {
            target_ed.right_select();
            return true;
        }
        ArrowRight => {
            target_ed.right();
            return true;
        }
        ArrowUp if modifiers.shift => {
            target_ed.up_visual_select(&app.editor.visual_lines);
            return true;
        }
        ArrowUp => {
            target_ed.up_visual(&app.editor.visual_lines);
            return true;
        }
        ArrowDown if modifiers.shift => {
            target_ed.down_visual_select(&app.editor.visual_lines);
            return true;
        }
        ArrowDown => {
            target_ed.down_visual(&app.editor.visual_lines);
            return true;
        }
        Home if modifiers.shift => {
            target_ed.home_visual_select(&app.editor.visual_lines);
            return true;
        }
        Home => {
            target_ed.home_visual(&app.editor.visual_lines);
            return true;
        }
        End if modifiers.shift => {
            target_ed.end_visual_select(&app.editor.visual_lines);
            return true;
        }
        End => {
            target_ed.end_visual(&app.editor.visual_lines);
            return true;
        }
        PageUp if modifiers.shift => {
            target_ed.page_up_visual_select(&app.editor.visual_lines, 10);
            return true;
        }
        PageUp => {
            target_ed.page_up_visual(&app.editor.visual_lines, 10);
            return true;
        }
        PageDown if modifiers.shift => {
            target_ed.page_down_visual_select(&app.editor.visual_lines, 10);
            return true;
        }
        PageDown => {
            target_ed.page_down_visual(&app.editor.visual_lines, 10);
            return true;
        }
        _ => {}
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ctrl_h_edits_the_hybrid_notes_buffer(){
        let mut app=App::new();
        app.services.editor_controller.mode=EditorInputMode::Hybrid;
        app.editor.ed.set_text("abc");
        app.editor.ed.cur=3;
        assert!(handle_editor_key(&mut app,Key::H,Modifiers::CTRL,1.0));
        assert_eq!(app.editor.ed.text(),"ab");
        assert!(app.editor.is_dirty);
    }
}
