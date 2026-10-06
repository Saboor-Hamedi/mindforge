//! Hybrid typing engine — modern IDE power editing with word navigation,
//! line manipulation, auto-pairing/wrapping, and smart indentation.

use crate::editor::Editor;
use eframe::egui::{Key, Modifiers};

#[derive(Default, Debug, Clone)]
pub struct HybridEngine {
    pub auto_pairing_enabled: bool,
}

impl HybridEngine {
    pub fn new() -> Self {
        Self {
            auto_pairing_enabled: true,
        }
    }

    /// Handles keyboard shortcuts for hybrid mode.
    /// Returns `true` if the event was consumed.
    pub fn handle_key(&mut self, ed: &mut Editor, key: Key, modifiers: Modifiers) -> bool {
        let ctrl = modifiers.ctrl || modifiers.command;
        let alt = modifiers.alt;
        let shift = modifiers.shift;

        // ── Alt + Arrow line moving ───────────────────────────────────────────
        if alt && !ctrl {
            match key {
                Key::ArrowUp => {
                    ed.move_line_up();
                    return true;
                }
                Key::ArrowDown => {
                    ed.move_line_down();
                    return true;
                }
                _ => {}
            }
        }

        // ── Ctrl + Shift + Arrow: Word selection ──────────────────────────────
        if ctrl && shift && !alt {
            match key {
                Key::ArrowLeft => {
                    ed.word_left_select();
                    return true;
                }
                Key::ArrowRight => {
                    ed.word_right_select();
                    return true;
                }
                _ => {}
            }
        }

        // ── Ctrl + Arrow: Word navigation ─────────────────────────────────────
        if ctrl && !shift && !alt {
            match key {
                Key::ArrowLeft => {
                    ed.word_left();
                    return true;
                }
                Key::ArrowRight => {
                    ed.word_right();
                    return true;
                }
                Key::Delete => {
                    ed.delete_word_forward();
                    return true;
                }
                Key::D => {
                    ed.duplicate_line();
                    return true;
                }
                Key::OpenBracket => {
                    ed.dedent_line();
                    return true;
                }
                Key::CloseBracket => {
                    ed.indent_line();
                    return true;
                }
                Key::Enter => {
                    ed.insert_line_below();
                    return true;
                }
                Key::Z => {
                    ed.undo();
                    return true;
                }
                Key::Y => {
                    ed.redo();
                    return true;
                }
                _ => {}
            }
        }

        // ── Ctrl + Shift + Z: Redo ───────────────────────────────────────────
        if ctrl && shift && !alt && key == Key::Z {
            ed.redo();
            return true;
        }

        // ── Tab / Shift+Tab indentation ───────────────────────────────────────
        if key == Key::Tab && !ctrl && !alt {
            if ed.table_nav_tab(!shift) {
                return true;
            }
            if shift {
                ed.dedent();
            } else {
                ed.indent();
            }
            return true;
        }

        false
    }

    /// Handles typed characters for auto-pairing and auto-closing.
    /// Returns `true` if auto-paired or skipped, `false` to fall back to normal character insertion.
    pub fn handle_char(&mut self, ed: &mut Editor, c: char) -> bool {
        if !self.auto_pairing_enabled {
            return false;
        }

        // Closing char skip: if cursor is directly before the matching close char, step over it
        if ed.cur < ed.buf.len() && ed.buf[ed.cur] == c && matches!(c, ')' | ']' | '}' | '"' | '\'' | '`') {
            ed.cur += 1;
            ed.selection = None;
            return true;
        }

        match c {
            '(' => {
                ed.auto_pair('(', ')');
                true
            }
            '[' => {
                ed.auto_pair('[', ']');
                true
            }
            '{' => {
                ed.auto_pair('{', '}');
                true
            }
            '"' => {
                ed.auto_pair('"', '"');
                true
            }
            '\'' => {
                ed.auto_pair('\'', '\'');
                true
            }
            '`' => {
                ed.auto_pair('`', '`');
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hybrid_word_navigation() {
        let mut ed = Editor::new();
        ed.insert_str("alpha beta gamma");
        let mut hybrid = HybridEngine::new();

        hybrid.handle_key(&mut ed, Key::ArrowLeft, Modifiers::CTRL);
        assert_eq!(ed.cur, 11); // start of "gamma"

        hybrid.handle_key(&mut ed, Key::ArrowLeft, Modifiers::CTRL);
        assert_eq!(ed.cur, 6); // start of "beta"

        hybrid.handle_key(&mut ed, Key::ArrowRight, Modifiers::CTRL);
        assert_eq!(ed.cur, 11); // start of "gamma"
    }

    #[test]
    fn test_hybrid_auto_pairing() {
        let mut ed = Editor::new();
        let mut hybrid = HybridEngine::new();

        assert!(hybrid.handle_char(&mut ed, '('));
        assert_eq!(ed.text(), "()");
        assert_eq!(ed.cur, 1);

        // Stepping over ')'
        assert!(hybrid.handle_char(&mut ed, ')'));
        assert_eq!(ed.text(), "()");
        assert_eq!(ed.cur, 2);
    }

    #[test]
    fn test_hybrid_wrap_selection() {
        let mut ed = Editor::new();
        ed.insert_str("hello");
        ed.select_all();
        let mut hybrid = HybridEngine::new();

        assert!(hybrid.handle_char(&mut ed, '"'));
        assert_eq!(ed.text(), "\"hello\"");
    }

    #[test]
    fn test_hybrid_ctrl_d_duplicate_line() {
        let mut ed = Editor::new();
        ed.insert_str("line 1\nline 2");
        ed.cur = 2; // on line 1
        let mut hybrid = HybridEngine::new();

        assert!(hybrid.handle_key(&mut ed, Key::D, Modifiers::CTRL));
        assert_eq!(ed.text(), "line 1\nline 1\nline 2");
    }

    #[test]
    fn test_hybrid_ctrl_enter_inserts_line_below() {
        let mut ed = Editor::new();
        for c in "hello".chars() {
            ed.insert(c);
        }
        let mut hybrid = HybridEngine::new();

        assert!(hybrid.handle_key(&mut ed, Key::Enter, Modifiers::CTRL));
        assert_eq!(ed.text(), "hello\n");
        assert_eq!(ed.cur, 6);

        // Undo should cleanly revert the Ctrl+Enter newline
        assert!(hybrid.handle_key(&mut ed, Key::Z, Modifiers::CTRL));
        assert_eq!(ed.text(), "hello");
    }

    #[test]
    fn test_hybrid_typing_burst_undo() {
        let mut ed = Editor::new();
        ed.insert_str("start ");
        for c in "world".chars() {
            ed.insert(c);
        }
        assert_eq!(ed.text(), "start world");

        let mut hybrid = HybridEngine::new();
        assert!(hybrid.handle_key(&mut ed, Key::Z, Modifiers::CTRL));
        assert_eq!(ed.text(), "start ");
    }
}
