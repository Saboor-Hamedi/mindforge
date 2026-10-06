//! Editing shortcuts (Undo, Redo, Indent, Dedent, Checklist, Mode toggle, Quick save, Zen mode).

use crate::app::App;
use crate::mode::Mode;
use eframe::egui;

pub fn handle_editing_shortcuts(app: &mut App, ctx: &egui::Context, now: f64) -> Option<bool> {
    // Undo (Ctrl+Z)
    let ctrl_z =
        ctx.input(|i| i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::Z));
    if ctrl_z {
        if app.services.editor_controller.mode == crate::app::EditorInputMode::Vim
            && !app.command_bar.in_command
        {
            return None;
        }
        if app.command_bar.in_command {
            if app.editor.cmd_ed.undo() {
                app.misc.sound.play();
                app.set_status("Undo", now);
                return Some(true);
            }
        } else {
            let changed = if app.misc.mode == Mode::Doc {
                app.editor.doc_ed.undo()
            } else {
                app.editor.ed.undo()
            };
            if changed {
                if app.misc.mode != Mode::Doc {
                    app.editor.is_dirty = true;
                }
                app.misc.sound.play();
                app.set_status("Undo", now);
                return Some(true);
            }
        }
        return Some(false);
    }

    // Redo (Ctrl+Y or Ctrl+Shift+Z)
    let ctrl_y = ctx.input(|i| {
        (i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::Y))
            || (i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::Z))
    });
    if ctrl_y {
        if app.services.editor_controller.mode == crate::app::EditorInputMode::Vim
            && !app.command_bar.in_command
        {
            return None;
        }
        if app.command_bar.in_command {
            if app.editor.cmd_ed.redo() {
                app.misc.sound.play();
                app.set_status("Redo", now);
                return Some(true);
            }
        } else {
            let changed = if app.misc.mode == Mode::Doc {
                app.editor.doc_ed.redo()
            } else {
                app.editor.ed.redo()
            };
            if changed {
                if app.misc.mode != Mode::Doc {
                    app.editor.is_dirty = true;
                }
                app.misc.sound.play();
                app.set_status("Redo", now);
                return Some(true);
            }
        }
        return Some(false);
    }

    // Indent / Dedent (Ctrl+] / Ctrl+[)
    let ctrl_indent = ctx.input(|i| {
        i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::CloseBracket)
    });
    let ctrl_dedent = ctx
        .input(|i| i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::OpenBracket));
    if ctrl_indent {
        if app.services.editor_controller.mode == crate::app::EditorInputMode::Vim
            && matches!(app.misc.mode, Mode::Normal | Mode::Doc)
            && !app.command_bar.in_command
        {
            return None;
        }
        if !app.command_bar.in_command {
            if app.misc.mode == Mode::Doc {
                app.editor.doc_ed.indent_line();
            } else {
                app.editor.ed.indent_line();
                app.editor.is_dirty = true;
            }
            app.misc.sound.play();
            app.set_status("Indented line (Ctrl + ])", now);
            return Some(true);
        }
    }
    if ctrl_dedent {
        if app.services.editor_controller.mode == crate::app::EditorInputMode::Vim
            && matches!(app.misc.mode, Mode::Normal | Mode::Doc)
            && !app.command_bar.in_command
        {
            return None;
        }
        if !app.command_bar.in_command {
            if app.misc.mode == Mode::Doc {
                app.editor.doc_ed.dedent_line();
            } else {
                app.editor.ed.dedent_line();
                app.editor.is_dirty = true;
            }
            app.misc.sound.play();
            app.set_status("Dedented line (Ctrl + [)", now);
            return Some(true);
        }
    }

    // Move lines up / down (Alt+Up / Alt+Down)
    let alt_up = ctx.input(|i| {
        i.modifiers.alt
            && !i.modifiers.ctrl
            && !i.modifiers.command
            && !i.modifiers.shift
            && i.key_pressed(egui::Key::ArrowUp)
    });
    let alt_down = ctx.input(|i| {
        i.modifiers.alt
            && !i.modifiers.ctrl
            && !i.modifiers.command
            && !i.modifiers.shift
            && i.key_pressed(egui::Key::ArrowDown)
    });
    if alt_up {
        if app.services.editor_controller.mode == crate::app::EditorInputMode::Vim
            && matches!(app.misc.mode, Mode::Normal | Mode::Doc)
            && !app.command_bar.in_command
        {
            return None;
        }
        if !app.command_bar.in_command {
            if app.misc.mode == Mode::Doc {
                app.editor.doc_ed.move_line_up();
            } else {
                app.editor.ed.move_line_up();
                app.editor.is_dirty = true;
            }
            app.misc.sound.play();
            return Some(true);
        }
    }
    if alt_down {
        if app.services.editor_controller.mode == crate::app::EditorInputMode::Vim
            && matches!(app.misc.mode, Mode::Normal | Mode::Doc)
            && !app.command_bar.in_command
        {
            return None;
        }
        if !app.command_bar.in_command {
            if app.misc.mode == Mode::Doc {
                app.editor.doc_ed.move_line_down();
            } else {
                app.editor.ed.move_line_down();
                app.editor.is_dirty = true;
            }
            app.misc.sound.play();
            return Some(true);
        }
    }

    // Toggle Checklist Item (Ctrl+Shift+X)
    let ctrl_shift_x = ctx.input(|i| {
        (i.modifiers.ctrl || i.modifiers.command)
            && i.modifiers.shift
            && !i.modifiers.alt
            && i.key_pressed(egui::Key::X)
    });
    if ctrl_shift_x && !app.command_bar.in_command {
        let (target_ed_mut, _) = if app.misc.mode == Mode::Doc {
            (&mut app.editor.doc_ed, &mut app.editor.doc_scroll_y)
        } else {
            (&mut app.editor.ed, &mut app.editor.scroll_y)
        };
        if target_ed_mut.toggle_checklist() {
            if app.misc.mode != Mode::Doc {
                app.editor.is_dirty = true;
            }
            app.misc.sound.play();
            app.set_status("Toggled checklist item (Ctrl+Shift+X)", now);
            return Some(true);
        }
    }

    // Mode Toggle (Ctrl+E) between Vim and Hybrid
    let ctrl_e = ctx.input(|i| {
        (i.modifiers.ctrl || i.modifiers.command)
            && !i.modifiers.alt
            && !i.modifiers.shift
            && i.key_pressed(egui::Key::E)
    });
    if ctrl_e {
        let new_mode = match app.services.editor_controller.mode {
            crate::app::EditorInputMode::Vim => crate::app::EditorInputMode::Hybrid,
            crate::app::EditorInputMode::Hybrid => crate::app::EditorInputMode::Vim,
        };
        app.services.editor_controller.set_mode(new_mode);
        let mode_name = match new_mode {
            crate::app::EditorInputMode::Vim => "Vim",
            crate::app::EditorInputMode::Hybrid => "Hybrid",
        };
        app.set_status(&format!("Switched to {} mode", mode_name), now);
        return Some(false);
    }

    // Quick Save (Ctrl+S)
    let ctrl_s = ctx.input(|i| {
        (i.modifiers.ctrl || i.modifiers.command)
            && !i.modifiers.shift
            && !i.modifiers.alt
            && i.key_pressed(egui::Key::S)
    });
    if ctrl_s {
        if app.misc.mode == Mode::Doc {
            app.set_status(
                "Practice edits are temporary and never change the source documentation.",
                now,
            );
            return Some(false);
        }
        app.quick_save_active_note(now);
        return Some(false);
    }

    // Zen Mode Toggle: Ctrl+Shift+F
    let toggle_zen = ctx.input(|i| {
        (i.modifiers.ctrl || i.modifiers.command)
            && i.modifiers.shift
            && !i.modifiers.alt
            && i.key_pressed(egui::Key::F)
    });
    if toggle_zen {
        let is_zen = !app.sidebar.open && !app.editor.preview_open && !app.misc.show_titlebar;
        if is_zen {
            app.sidebar.open = true;
            app.misc.show_titlebar = true;
            app.set_status("Exited Zen Mode (Ctrl+Shift+F)", now);
        } else {
            app.sidebar.open = false;
            app.editor.preview_open = false;
            app.misc.show_titlebar = false;
            app.set_status(
                "Zen Mode: distraction-free writing (Ctrl+Shift+F to exit)",
                now,
            );
        }
        return Some(false);
    }

    None
}
