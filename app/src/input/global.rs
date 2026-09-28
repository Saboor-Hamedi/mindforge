//! Global keyboard shortcuts, clipboard actions, and window resize/drag handlers.

use crate::app::App;
use crate::editor::backend::EditorBackend;
use crate::mode::Mode;
use eframe::egui;

/// Processes global shortcuts (saving, note creation, modals, clipboard, undo/redo).
/// Returns `Some(typed)` if a global shortcut fully handled the frame, or `None` to continue to typing.
pub fn handle_global_shortcuts(app: &mut App, ctx: &egui::Context, now: f64) -> Option<bool> {
    let palette_shortcut = ctx.input(|i| {
        let command = i.modifiers.ctrl || i.modifiers.command;
        command && !i.modifiers.alt && i.modifiers.shift && i.key_pressed(egui::Key::P)
    });
    // 0. Active Modal Input Priority:
    // When any modal (Search, Settings, Rename, Delete confirmation, Accent dropdown) is active,
    // absorb global shortcuts so that modal dialogs retain 100% focused input context.
    // This prevents global actions (e.g., terminal toggle Ctrl+J, new note Ctrl+N, AI toggle Ctrl+Shift+I)
    // from interrupting or conflicting with modal interaction.
    if app.modal.search_open && !palette_shortcut {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            app.modal.search_open = false;
        }
        return Some(false);
    }
    if app.modal.settings_open {
        // Modal input priority: let the settings modal and keybindings tab manage Escape & key capture
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

    // Global terminal toggle shortcut: Ctrl+J or Ctrl+` (Backtick / Tilde)
    // IMPORTANT: Suppressed when wikilink autocomplete, hover wikilink, or right pane lists (Outline/Backlinks) are active,
    // so Ctrl+J navigates items / scrolls preview instead of toggling terminal.
    let is_panel_nav_active = app.services.wikilink_autocomplete.is_active
        || app.services.hover_wikilink.is_active()
        || (app.editor.preview_open && (
            app.right_pane.tab == crate::app::RightPaneTab::Backlinks
            || app.right_pane.tab == crate::app::RightPaneTab::Outline
        ));

    let toggle_term = !app.command_bar.in_command
        && !is_panel_nav_active
        && ctx.input(|i| {
            (i.modifiers.ctrl && !i.modifiers.shift && !i.modifiers.alt && i.key_pressed(egui::Key::J))
                || (i.modifiers.ctrl && i.key_pressed(egui::Key::Backtick))
        });
    if toggle_term {
        app.terminal.open = !app.terminal.open;
        if app.terminal.open {
            app.terminal.focused = true;
            app.set_status("Terminal opened (Ctrl+J to toggle, click editor or Esc to edit)", now);
        } else {
            app.terminal.focused = false;
            app.set_status("Terminal closed", now);
        }
        return Some(false);
    }

    // Global AI Agent right pane toggle: Ctrl+Shift+I
    let toggle_ai = ctx.input(|i| {
        (i.modifiers.ctrl || i.modifiers.command) && i.modifiers.shift && !i.modifiers.alt && i.key_pressed(egui::Key::I)
    });
    if toggle_ai {
        if app.editor.preview_open && app.right_pane.tab == crate::app::RightPaneTab::AiAgent {
            app.editor.preview_open = false;
            app.services.agent_state.is_open = false;
            ctx.memory_mut(|m| m.surrender_focus(egui::Id::new("deepseek_prompt_input")));
            app.set_status("AI Agent closed", now);
        } else {
            app.editor.preview_open = true;
            app.right_pane.tab = crate::app::RightPaneTab::AiAgent;
            app.right_pane.ai_focus_requested = true;
            app.services.agent_state.is_open = true;
            ctx.memory_mut(|m| m.request_focus(egui::Id::new("deepseek_prompt_input")));
            app.set_status("AI Assistant opened (Ctrl+Shift+I to toggle)", now);
        }
        return Some(false);
    }

    // Toggle Backlinks Panel in Right Pane: Ctrl+I
    let ctrl_i = ctx.input(|i| {
        (i.modifiers.ctrl || i.modifiers.command)
            && !i.modifiers.shift
            && !i.modifiers.alt
            && i.key_pressed(egui::Key::I)
    });
    if ctrl_i {
        if app.editor.preview_open && app.right_pane.tab == crate::app::RightPaneTab::Backlinks {
            app.editor.preview_open = false;
            let _ = app.services.db_tx.send(crate::services::db_worker::DbMsg::SaveSetting {
                key: "preview".into(),
                val: "false".into(),
            });
            app.set_status("Backlinks panel closed (Ctrl+I)", now);
        } else {
            app.editor.preview_open = true;
            app.right_pane.tab = crate::app::RightPaneTab::Backlinks;
            let _ = app.services.db_tx.send(crate::services::db_worker::DbMsg::SaveSetting {
                key: "preview".into(),
                val: "true".into(),
            });
            app.set_status("Backlinks panel opened (Ctrl+I)", now);
        }
        return Some(false);
    }

    // Toggle Outline Panel in Right Pane: Ctrl+Shift+O
    let ctrl_shift_o = ctx.input(|i| {
        (i.modifiers.ctrl || i.modifiers.command)
            && i.modifiers.shift
            && !i.modifiers.alt
            && i.key_pressed(egui::Key::O)
    });
    if ctrl_shift_o {
        if app.editor.preview_open && app.right_pane.tab == crate::app::RightPaneTab::Outline {
            app.editor.preview_open = false;
            let _ = app.services.db_tx.send(crate::services::db_worker::DbMsg::SaveSetting {
                key: "preview".into(),
                val: "false".into(),
            });
            app.set_status("Outline panel closed (Ctrl+Shift+O)", now);
        } else {
            app.editor.preview_open = true;
            app.right_pane.tab = crate::app::RightPaneTab::Outline;
            let _ = app.services.db_tx.send(crate::services::db_worker::DbMsg::SaveSetting {
                key: "preview".into(),
                val: "true".into(),
            });
            app.set_status("Outline panel opened (Ctrl+Shift+O)", now);
        }
        return Some(false);
    }

    // AI Assistant input priority: when user is focused on the AI prompt textarea,
    // absorb typing and shortcuts so input goes exclusively into the prompt and doesn't trigger the editor or sidebar.
    if app.editor.preview_open && app.right_pane.tab == crate::app::RightPaneTab::AiAgent {
        let ai_input_id = egui::Id::new("deepseek_prompt_input");
        let is_ai_focused = app.services.agent_state.is_input_focused
            || ctx.memory(|m| m.has_focus(ai_input_id))
            || ctx.wants_keyboard_input();

        if is_ai_focused {
            app.sidebar.focused = false;
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                ctx.memory_mut(|m| m.surrender_focus(ai_input_id));
                app.services.agent_state.is_input_focused = false;
            }
            return Some(false);
        }
    }

    // When Vim search is active, bypass ALL global shortcuts so every keystroke
    // flows through as a Text event into the search buffer (fixes missing chars).
    // Terminal input priority: when terminal dock is open and focused, protect shell control sequences
    // (Ctrl+C, Ctrl+D, Ctrl+Z, etc.) so they pass directly to the terminal PTY instead of triggering editor commands.
    if app.terminal.open && app.terminal.focused {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            app.terminal.focused = false;
            app.set_status("Editor focused (Ctrl+J to return to terminal)", now);
            return Some(false);
        }
        let is_ctrl = ctx.input(|i| i.modifiers.ctrl);
        let key_c = ctx.input(|i| i.key_pressed(egui::Key::C));
        let key_d = ctx.input(|i| i.key_pressed(egui::Key::D));
        let key_z = ctx.input(|i| i.key_pressed(egui::Key::Z));
        let key_l = ctx.input(|i| i.key_pressed(egui::Key::L));
        if is_ctrl && (key_c || key_d || key_z || key_l) {
            return None;
        }
    }

    // Help Tab Input Isolation & Navigation (single Quick Start tab)
    if app.misc.mode == Mode::Help {
        let (help_esc, scroll_up, scroll_down) = ctx.input(|i| (
            i.key_pressed(egui::Key::Escape),
            (!i.modifiers.ctrl && (i.key_pressed(egui::Key::K) || i.key_pressed(egui::Key::ArrowUp) || i.key_pressed(egui::Key::PageUp))),
            (!i.modifiers.ctrl && (i.key_pressed(egui::Key::J) || i.key_pressed(egui::Key::ArrowDown) || i.key_pressed(egui::Key::PageDown))),
        ));

        if help_esc {
            app.misc.mode = Mode::Normal;
            app.set_status("Returned to notes", now);
            return Some(false);
        }

        if scroll_down {
            app.modal.help_scroll_y = (app.modal.help_scroll_y + 45.0).max(0.0);
            return Some(false);
        }
        if scroll_up {
            app.modal.help_scroll_y = (app.modal.help_scroll_y - 45.0).max(0.0);
            return Some(false);
        }
    }

    // Global Keyboard Shortcuts
    let (ctrl_s, ctrl_n, ctrl_r, ctrl_p, ctrl_shift_p, ctrl_comma, ctrl_b, escape, ctrl_backslash) = ctx.input(|i| (
        i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::S),
        i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::N),
        i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::R),
        i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::P),
        (i.modifiers.ctrl || i.modifiers.command) && i.modifiers.shift && i.key_pressed(egui::Key::P),
        i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::Comma),
        i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::B),
        i.key_pressed(egui::Key::Escape),
        (i.modifiers.ctrl && (i.key_pressed(egui::Key::Backslash) || i.key_pressed(egui::Key::Pipe)))
            || (i.modifiers.alt && i.key_pressed(egui::Key::Backslash)),
    ));

    // Delete Note (Ctrl+Shift+D or Ctrl+Shift+Delete)
    let ctrl_shift_d = ctx.input(|i| {
        (i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::D))
            || (i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::Delete))
    });

    let ctrl_close = ctx.input(|i| {
        (i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::W))
            || (i.modifiers.ctrl && i.key_pressed(egui::Key::Q))
    });
    if ctrl_close {
        if app.editor.is_dirty && app.misc.mode == Mode::Normal {
            app.quick_save_active_note(now);
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        return Some(false);
    }

    // Undo / Redo
    let ctrl_z = ctx.input(|i| i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::Z));
    let ctrl_redo = ctx.input(|i| {
        (i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::Z))
            || (i.modifiers.ctrl && i.key_pressed(egui::Key::Y))
    });
    if ctrl_z {
        if app.services.editor_controller.mode == crate::app::EditorInputMode::Vim && app.misc.mode == Mode::Normal && !app.command_bar.in_command {
            let modifiers = ctx.input(|i| i.modifiers);
            if let Some(backend) = app.services.vim_runtime.backend.as_mut() {
                let _ = backend.handle_key(crate::editor::events::EditorKeyEvent { key: egui::Key::Z, modifiers });
            } else {
                app.services.vim_runtime.queue_input(crate::vim::PendingVimInput::Key(
                    crate::editor::events::EditorKeyEvent { key: egui::Key::Z, modifiers },
                ));
            }
            return Some(false);
        } else if app.command_bar.in_command {
            app.editor.cmd_ed.undo();
        } else if app.editor.ed.undo() {
            app.editor.is_dirty = true;
            app.misc.sound.play();
            app.set_status("Undo", now);
            return Some(true);
        }
        return Some(false);
    }
    if ctrl_redo {
        if app.services.editor_controller.mode == crate::app::EditorInputMode::Vim && app.misc.mode == Mode::Normal && !app.command_bar.in_command {
            let modifiers = ctx.input(|i| i.modifiers);
            let key = if modifiers.shift { egui::Key::Z } else { egui::Key::Y };
            if let Some(backend) = app.services.vim_runtime.backend.as_mut() {
                let _ = backend.handle_key(crate::editor::events::EditorKeyEvent { key, modifiers });
            } else {
                app.services.vim_runtime.queue_input(crate::vim::PendingVimInput::Key(
                    crate::editor::events::EditorKeyEvent { key, modifiers },
                ));
            }
            return Some(false);
        } else if app.command_bar.in_command {
            app.editor.cmd_ed.redo();
        } else if app.editor.ed.redo() {
            app.editor.is_dirty = true;
            app.misc.sound.play();
            app.set_status("Redo", now);
            return Some(true);
        }
        return Some(false);
    }

    // Move text left / right (Ctrl + [ to dedent left, Ctrl + ] to indent right)
    let ctrl_indent = ctx.input(|i| i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::CloseBracket));
    let ctrl_dedent = ctx.input(|i| i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::OpenBracket));
    if ctrl_indent {
        if app.services.editor_controller.mode == crate::app::EditorInputMode::Vim && !app.command_bar.in_command {
            return None;
        }
        if !app.command_bar.in_command {
            app.editor.ed.indent_line();
            app.editor.is_dirty = true;
            app.misc.sound.play();
            app.set_status("Indented line (Ctrl + ])", now);
            return Some(true);
        }
    }
    if ctrl_dedent {
        if app.services.editor_controller.mode == crate::app::EditorInputMode::Vim && !app.command_bar.in_command {
            return None;
        }
        if !app.command_bar.in_command {
            app.editor.ed.dedent_line();
            app.editor.is_dirty = true;
            app.misc.sound.play();
            app.set_status("Dedented line (Ctrl + [)", now);
            return Some(true);
        }
    }

    // Tab / Shift+Tab — polled directly so egui focus-cycling can never intercept it.
    let (tab_pressed, shift_tab_pressed) = ctx.input(|i| {
        let tab = !i.modifiers.ctrl && !i.modifiers.alt && i.key_pressed(egui::Key::Tab);
        (tab && !i.modifiers.shift, tab && i.modifiers.shift)
    });
    if (tab_pressed || shift_tab_pressed) && !app.command_bar.in_command {
        if app.misc.mode == Mode::Doc {
            if !app.sidebar.open {
                app.sidebar.open = true;
                app.tabs.doc_sidebar_focused = true;
            } else {
                app.tabs.doc_sidebar_focused = !app.tabs.doc_sidebar_focused;
            }
            let status = if app.tabs.doc_sidebar_focused {
                "Doc Sidebar active (j/k: move • Enter: read • Tab: reader)"
            } else {
                "Doc Reader active (Tab / Ctrl+B to return to Doc List)"
            };
            app.set_status(status, now);
            return Some(false);
        }
        if app.misc.mode == Mode::Normal {
            use crate::app::EditorInputMode;
            let nvim_normal = app.services.vim_runtime.backend.as_ref().is_some_and(|backend| backend.grid.mode.starts_with('n'));
            if app.sidebar.open && (app.sidebar.focused || (app.services.editor_controller.mode == EditorInputMode::Vim && nvim_normal)) {
                app.sidebar.focused = !app.sidebar.focused;
                let status = if app.sidebar.focused {
                    "Notes Sidebar active (j/k: move • Enter: load • Ctrl+L / l: editor)"
                } else {
                    "Editor active (Ctrl+H to return to Sidebar)"
                };
                app.set_status(status, now);
                return Some(false);
            }
            let modifiers = if shift_tab_pressed {
                let mut m = egui::Modifiers::default();
                m.shift = true;
                m
            } else {
                egui::Modifiers::default()
            };
            if app.services.editor_controller.mode == EditorInputMode::Vim {
                return None;
            }
            if app.editor.ed.table_nav_tab(!shift_tab_pressed) {
                // Navigated table cell or appended row
            } else if app.services.editor_controller.mode == EditorInputMode::Hybrid {
                app.services.hybrid.handle_key(&mut app.editor.ed, egui::Key::Tab, modifiers);
            } else if shift_tab_pressed {
                app.editor.ed.dedent();
            } else {
                app.editor.ed.indent();
            }
            app.editor.is_dirty = true;
            app.misc.sound.play();
            app.misc.last_char_time = now;
            return Some(true);
        }
    }

    // Select All (Ctrl+A)
    let ctrl_a = ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::A));
    if ctrl_a {
        if app.services.editor_controller.mode == crate::app::EditorInputMode::Vim && !app.command_bar.in_command {
            return None;
        }
        if app.command_bar.in_command {
            app.editor.cmd_ed.select_all();
        } else {
            app.editor.ed.select_all();
            return Some(true);
        }
        return Some(false);
    }

    // Clipboard Copy (Ctrl+C)
    let ctrl_c = ctx.input(|i| i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::C));
    if ctrl_c {
        if app.services.editor_controller.mode == crate::app::EditorInputMode::Vim && !app.command_bar.in_command {
            return None;
        }
        let text = if app.command_bar.in_command {
            app.editor.cmd_ed.selected_text()
        } else if app.modal.search_open {
            Some(app.modal.search_query.clone())
        } else if app.modal.rename_open {
            Some(app.modal.rename_input.clone())
        } else if app.misc.mode == Mode::Doc {
            app.editor.doc_ed.selected_text()
        } else {
            app.editor.ed.selected_text()
        };
        if let Some(t) = text {
            app.misc.clipboard_text = Some(t.clone());
            set_win32_clipboard(&t);
            ctx.copy_text(t);
            app.set_status("Copied selection", now);
        }
        return Some(false);
    }

    // Checklist Toggle Shortcut: Ctrl+Shift+X (friendly creation/toggle for paragraphs, bullets, tasks)
    let ctrl_shift_x = app.services.editor_controller.mode != crate::app::EditorInputMode::Vim
        && ctx.input(|i| {
            (i.modifiers.ctrl || i.modifiers.command)
                && i.modifiers.shift
                && !i.modifiers.alt
                && i.key_pressed(egui::Key::X)
        });
    if ctrl_shift_x {
        let (target_ed_mut, _) = if app.misc.mode == Mode::Doc {
            (&mut app.editor.doc_ed, &mut app.editor.doc_scroll_y)
        } else {
            (&mut app.editor.ed, &mut app.editor.scroll_y)
        };
        if target_ed_mut.toggle_checklist() {
            app.editor.is_dirty = true;
            app.misc.sound.play();
            app.set_status("Toggled checklist item (Ctrl+Shift+X)", now);
            return Some(true);
        }
    }

    // Clipboard Cut (Ctrl+X)
    let ctrl_x = ctx.input(|i| i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::X));
    if ctrl_x {
        if app.services.editor_controller.mode == crate::app::EditorInputMode::Vim && !app.command_bar.in_command {
            return None;
        }
        let text = if app.command_bar.in_command {
            let t = app.editor.cmd_ed.selected_text();
            app.editor.cmd_ed.delete_selection();
            t
        } else {
            let t = app.editor.ed.selected_text();
            if app.editor.ed.delete_selection() {
                app.editor.is_dirty = true;
                app.misc.sound.play();
                app.set_status("Cut selection", now);
            }
            t
        };
        if let Some(t) = text {
            app.misc.clipboard_text = Some(t.clone());
            set_win32_clipboard(&t);
            ctx.copy_text(t);
            return Some(true);
        }
        return Some(false);
    }

    // Clipboard Paste (Ctrl+V)
    let ctrl_v = ctx.input(|i| i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::V));
    if ctrl_v {
        if let Some(text) = get_clipboard_text(app) {
            if app.command_bar.in_command {
                crate::command::input::handle_command_paste(app, &text, now);
                return Some(true);
            } else if app.modal.search_open {
                app.modal.search_query.push_str(&text);
                app.modal.search_selected = 0;
                app.update_search_results();
                return Some(true);
            } else if app.modal.rename_open {
                app.modal.rename_input.push_str(&text);
                return Some(true);
            } else if app.services.editor_controller.mode == crate::app::EditorInputMode::Vim
                && app.misc.mode == Mode::Normal && !app.command_bar.in_command
            {
                if let Some(backend) = app.services.vim_runtime.backend.as_mut() {
                    let _ = backend.paste(&text);
                } else {
                    app.services.vim_runtime.queue_input(crate::vim::PendingVimInput::Paste(text));
                }
                return Some(true);
            } else if app.misc.mode == Mode::Normal {
                if crate::input::editor::handle_editor_paste(app, &text, now) {
                    return Some(true);
                }
            }
        }
        return Some(false);
    }

    if ctrl_s {
        if app.misc.mode == Mode::Doc {
            app.set_status("Documentation files are read-only (changes not saved).", now);
            return Some(false);
        }
        app.quick_save_active_note(now);
        return Some(false);
    }

    if ctrl_n {
        app.create_new_note(now);
        return Some(false);
    }

    let ctrl_w = ctx.input(|i| i.modifiers.ctrl && !i.modifiers.alt && i.key_pressed(egui::Key::W));
    if ctrl_w {
        if app.misc.mode == Mode::Doc {
            app.close_doc_tab(app.tabs.active_doc_tab, now);
        } else {
            app.close_tab(app.tabs.active_tab, now);
        }
        return Some(false);
    }

    // Direct tab jump with Ctrl+1 .. Ctrl+9
    if ctx.input(|i| i.modifiers.ctrl && !i.modifiers.alt) {
        let num_target = ctx.input(|i| {
            if i.key_pressed(egui::Key::Num1) { Some(0) }
            else if i.key_pressed(egui::Key::Num2) { Some(1) }
            else if i.key_pressed(egui::Key::Num3) { Some(2) }
            else if i.key_pressed(egui::Key::Num4) { Some(3) }
            else if i.key_pressed(egui::Key::Num5) { Some(4) }
            else if i.key_pressed(egui::Key::Num6) { Some(5) }
            else if i.key_pressed(egui::Key::Num7) { Some(6) }
            else if i.key_pressed(egui::Key::Num8) { Some(7) }
            else if i.key_pressed(egui::Key::Num9) { Some(8) }
            else { None }
        });
        if let Some(target_idx) = num_target {
            if app.misc.mode == Mode::Doc {
                if !app.tabs.open_doc_tabs.is_empty() {
                    let idx = target_idx.min(app.tabs.open_doc_tabs.len() - 1);
                    app.switch_doc_tab(idx, now);
                }
            } else if !app.open_notes.is_empty() {
                let idx = target_idx.min(app.open_notes.len() - 1);
                app.switch_tab(idx, now);
            }
            return Some(false);
        }
    }

    // Tab cycling with Ctrl+Tab (next) and Ctrl+Shift+Tab (previous)
    let ctrl_tab = ctx.input(|i| i.modifiers.ctrl && !i.modifiers.alt && i.key_pressed(egui::Key::Tab));
    if ctrl_tab {
        let is_shift = ctx.input(|i| i.modifiers.shift);
        if app.misc.mode == Mode::Doc {
            let n = app.tabs.open_doc_tabs.len();
            if n > 1 {
                let next = if is_shift {
                    if app.tabs.active_doc_tab == 0 { n - 1 } else { app.tabs.active_doc_tab - 1 }
                } else {
                    (app.tabs.active_doc_tab + 1) % n
                };
                app.switch_doc_tab(next, now);
            }
        } else {
            let n = app.open_notes.len();
            if n > 1 {
                let next = if is_shift {
                    if app.tabs.active_tab == 0 { n - 1 } else { app.tabs.active_tab - 1 }
                } else {
                    (app.tabs.active_tab + 1) % n
                };
                app.switch_tab(next, now);
            }
        }
        return Some(false);
    }

    if ctrl_r {
        if app.misc.mode == Mode::Doc {
            app.set_status("Documentation files are read-only and cannot be renamed.", now);
            return Some(false);
        }
        app.modal.rename_open = true;
        app.modal.rename_input = app.notes.active_note_title.clone();
        app.modal.rename_just_opened = true;
        return Some(false);
    }

    if ctrl_shift_d {
        if app.misc.mode == Mode::Doc {
            app.set_status("Documentation files cannot be deleted.", now);
            return Some(false);
        }
        app.modal.delete_confirm_open = true;
        app.modal.delete_just_opened = true;
        return Some(false);
    }

    if ctrl_comma {
        app.modal.settings_open = !app.modal.settings_open;
        app.modal.settings_just_opened = app.modal.settings_open;
        return Some(false);
    }

    if ctrl_shift_p {
        if app.modal.search_open && app.modal.search_query.starts_with('>') {
            app.modal.search_open = false;
        } else {
            app.modal.search_open = true;
            app.modal.search_query = ">".to_string();
            app.modal.search_selected = 0;
            app.modal.search_just_opened = true;
            app.update_search_results();
        }
        return Some(false);
    }

    if ctrl_p {
        if app.modal.search_open && !app.modal.search_query.starts_with('>') {
            app.modal.search_open = false;
        } else {
            app.modal.search_open = true;
            app.modal.search_query.clear();
            app.modal.search_selected = 0;
            app.modal.search_just_opened = true;
            app.update_search_results();
        }
        return Some(false);
    }

    if ctrl_backslash {
        if app.editor.preview_open && app.right_pane.tab == crate::app::RightPaneTab::Preview {
            app.editor.preview_open = false;
        } else {
            app.editor.preview_open = true;
            app.right_pane.tab = crate::app::RightPaneTab::Preview;
        }
        let val = if app.editor.preview_open { "true" } else { "false" };
        let _ = app.services.db_tx.send(crate::services::db_worker::DbMsg::SaveSetting {
            key: "preview".into(),
            val: val.into(),
        });
        let msg = if app.editor.preview_open {
            "Markdown Live Preview ON (Ctrl + \\ to toggle, drag center knob)"
        } else {
            "Markdown Live Preview OFF (Ctrl + \\)"
        };
        app.set_status(msg, now);
        return Some(false);
    }

    let ctrl_e = ctx.input(|i| (i.modifiers.ctrl || i.modifiers.command) && !i.modifiers.alt && !i.modifiers.shift && i.key_pressed(egui::Key::E));
    if ctrl_e {
        app.editor.inline_mode = false;
        app.set_status("Raw Markdown editing is active", now);
        return Some(false);
    }

    // Zen Mode Toggle (Ctrl + .)
    let ctrl_dot = ctx.input(|i| {
        (i.modifiers.ctrl || i.modifiers.command)
            && !i.modifiers.alt
            && !i.modifiers.shift
            && i.key_pressed(egui::Key::Period)
    });
    if ctrl_dot {
        app.misc.zen_mode = !app.misc.zen_mode;
        if app.misc.zen_mode {
            app.misc.show_titlebar = false;
            app.misc.show_tabs = false;
            app.sidebar.open = false;
            app.editor.preview_open = false;
        } else {
            app.misc.show_titlebar = true;
            app.misc.show_tabs = true;
        }
        let _ = app.services.db_tx.send(crate::services::db_worker::DbMsg::SaveSetting {
            key: "zen_mode".into(),
            val: if app.misc.zen_mode { "true" } else { "false" }.into(),
        });
        let _ = app.services.db_tx.send(crate::services::db_worker::DbMsg::SaveSetting {
            key: "show_titlebar".into(),
            val: if app.misc.show_titlebar { "true" } else { "false" }.into(),
        });
        let _ = app.services.db_tx.send(crate::services::db_worker::DbMsg::SaveSetting {
            key: "show_tabs".into(),
            val: if app.misc.show_tabs { "true" } else { "false" }.into(),
        });
        let _ = app.services.db_tx.send(crate::services::db_worker::DbMsg::SaveSetting {
            key: "sidebar".into(),
            val: if app.sidebar.open { "true" } else { "false" }.into(),
        });
        let _ = app.services.db_tx.send(crate::services::db_worker::DbMsg::SaveSetting {
            key: "preview".into(),
            val: if app.editor.preview_open { "true" } else { "false" }.into(),
        });
        let msg = if app.misc.zen_mode {
            "Zen Mode ON (Ctrl+. to toggle)"
        } else {
            "Zen Mode OFF (Ctrl+. to toggle)"
        };
        app.set_status(msg, now);
        return Some(false);
    }

    let ctrl_h = ctx.input(|i| {
        (i.modifiers.ctrl || i.modifiers.command)
            && !i.modifiers.shift
            && !i.modifiers.alt
            && i.key_pressed(egui::Key::H)
    });
    if ctrl_h && !app.command_bar.in_command {
        if app.misc.mode == Mode::Doc {
            app.sidebar.open = true;
            app.tabs.doc_sidebar_focused = true;
            app.set_status("Doc sidebar active (j/k: navigate • Enter: read • Ctrl+L: editor)", now);
        } else {
            app.sidebar.open = true;
            app.sidebar.focused = true;
            app.sidebar.needs_scroll = true;
            if let Some(cur_id) = app.notes.active_note_id {
                app.sidebar.selected_idx = app.notes.notes_list.iter().position(|n| n.id == cur_id).unwrap_or(0);
            }
            app.set_status("Sidebar active (j/k: move • Enter: load • Ctrl+L / l: editor)", now);
        }
        let _ = app.services.db_tx.send(crate::services::db_worker::DbMsg::SaveSetting {
            key: "sidebar".into(),
            val: "true".into(),
        });
        return Some(false);
    }

    if ctrl_b {
        if app.misc.mode == Mode::Doc {
            app.sidebar.open = !app.sidebar.open;
            app.tabs.doc_sidebar_focused = app.sidebar.open;
            let status = if app.sidebar.open {
                "Documentation sidebar opened (j/k: navigate • Enter: read • Ctrl+L: editor)"
            } else {
                "Doc sidebar closed"
            };
            app.set_status(status, now);
            let sb_val = if app.sidebar.open { "true" } else { "false" };
            let _ = app.services.db_tx.send(crate::services::db_worker::DbMsg::SaveSetting {
                key: "sidebar".into(),
                val: sb_val.into(),
            });
            return Some(false);
        }

        app.sidebar.open = !app.sidebar.open;
        app.sidebar.focused = app.sidebar.open;
        if app.sidebar.open {
            app.sidebar.needs_scroll = true;
            if let Some(cur_id) = app.notes.active_note_id {
                app.sidebar.selected_idx = app.notes.notes_list.iter().position(|n| n.id == cur_id).unwrap_or(0);
            } else {
                app.sidebar.selected_idx = 0;
            }
            app.set_status("Sidebar opened (j/k: move • Enter: load • Ctrl+L / l: editor)", now);
        } else {
            app.set_status("Sidebar closed", now);
        }
        let sb_val = if app.sidebar.open { "true" } else { "false" };
        let _ = app.services.db_tx.send(crate::services::db_worker::DbMsg::SaveSetting {
            key: "sidebar".into(),
            val: sb_val.into(),
        });
        return Some(false);
    }

    // Sidebar keyboard navigation (j/k to select, Enter to open, Esc/l/→/Tab to return to editor, i to edit)
    if app.sidebar.open && app.sidebar.focused {
        let (sb_up, sb_down, sb_enter, sb_esc, sb_edit, sb_to_editor, sb_tab) = ctx.input(|i| (
            (!i.modifiers.ctrl && !i.modifiers.alt && i.key_pressed(egui::Key::K)) || i.key_pressed(egui::Key::ArrowUp),
            (!i.modifiers.ctrl && !i.modifiers.alt && i.key_pressed(egui::Key::J)) || i.key_pressed(egui::Key::ArrowDown),
            i.key_pressed(egui::Key::Enter),
            i.key_pressed(egui::Key::Escape),
            !i.modifiers.ctrl && !i.modifiers.alt && i.key_pressed(egui::Key::I),
            (!i.modifiers.alt && (i.key_pressed(egui::Key::L) || i.key_pressed(egui::Key::ArrowRight))),
            !i.modifiers.ctrl && !i.modifiers.alt && i.key_pressed(egui::Key::Tab),
        ));

        if sb_esc || sb_to_editor || sb_tab {
            app.sidebar.focused = false;
            app.set_status("Editor active (Ctrl+H to return to Sidebar)", now);
            return Some(false);
        }

        if sb_edit {
            app.sidebar.focused = false;
            if app.services.editor_controller.mode == crate::app::EditorInputMode::Vim {
                if let Some(backend) = app.services.vim_runtime.backend.as_mut() {
                    let _ = backend.handle_text("i");
                } else {
                    app.services.vim_runtime.queue_input(crate::vim::PendingVimInput::Text("i".into()));
                }
            }
            return Some(false);
        }

        if sb_down {
            if !app.notes.notes_list.is_empty() && app.sidebar.selected_idx + 1 < app.notes.notes_list.len() {
                app.sidebar.selected_idx += 1;
                app.sidebar.needs_scroll = true;
            }
            return Some(false);
        }

        if sb_up {
            if app.sidebar.selected_idx > 0 {
                app.sidebar.selected_idx -= 1;
                app.sidebar.needs_scroll = true;
            }
            return Some(false);
        }

        if sb_enter {
            if let Some(note) = app.notes.notes_list.get(app.sidebar.selected_idx).cloned() {
                app.load_note(note.id, note.topic, note.body, now);
                // Keep focus on the sidebar and on the loaded note as requested!
                app.sidebar.focused = true;
                app.set_status("Loaded note (sidebar active — use j/k to move)", now);
            }
            return Some(false);
        }

        // When sidebar is focused, consume any text/keys so they do not type into editor
        return Some(false);
    }

    // Dedicated Documentation Sidebar keyboard navigation (j/k to select, Enter to open, Esc to exit)
    if app.misc.mode == Mode::Doc && app.tabs.doc_sidebar_focused && !app.command_bar.in_command {
        let (doc_up, doc_down, doc_enter, doc_esc, doc_to_reader) = ctx.input(|i| (
            (!i.modifiers.ctrl && !i.modifiers.alt && i.key_pressed(egui::Key::K)) || i.key_pressed(egui::Key::ArrowUp),
            (!i.modifiers.ctrl && !i.modifiers.alt && i.key_pressed(egui::Key::J)) || i.key_pressed(egui::Key::ArrowDown),
            i.key_pressed(egui::Key::Enter),
            i.key_pressed(egui::Key::Escape),
            (!i.modifiers.alt && (i.key_pressed(egui::Key::L) || i.key_pressed(egui::Key::ArrowRight))),
        ));

        if doc_esc {
            // Unfocus doc sidebar into reader without closing the documentation!
            app.tabs.doc_sidebar_focused = false;
            app.set_status("Doc reader active (Ctrl+H: sidebar • Esc: exit docs)", now);
            return Some(false);
        }

        if doc_to_reader {
            app.tabs.doc_sidebar_focused = false;
            app.set_status("Doc Reader active (Ctrl+H to return to Doc List)", now);
            return Some(false);
        }

        if doc_down {
            let total_docs = crate::ui::docs::BRAIN_DOCS.len();
            if total_docs > 0 && app.tabs.doc_selected_idx + 1 < total_docs {
                app.tabs.doc_selected_idx += 1;
            }
            return Some(false);
        }

        if doc_up {
            if app.tabs.doc_selected_idx > 0 {
                app.tabs.doc_selected_idx -= 1;
            }
            return Some(false);
        }

        if doc_enter {
            app.load_doc_by_index(app.tabs.doc_selected_idx, now);
            // Keep focus on the doc sidebar as requested!
            app.tabs.doc_sidebar_focused = true;
            return Some(false);
        }

        // When doc sidebar is focused, consume any text/keys so they do not leak into doc reader
        return Some(false);
    }

    // Help & Guidance Tab (F1)
    let trigger_help = ctx.input(|i| i.key_pressed(egui::Key::F1));
    if trigger_help && !app.command_bar.in_command {
        if app.misc.mode == Mode::Help {
            app.misc.mode = Mode::Normal;
            app.set_status("Returned to notes", now);
        } else {
            app.open_help_tab(now);
        }
        return Some(false);
    }

    if escape {
        if app.services.editor_controller.mode == crate::app::EditorInputMode::Vim
            && app.misc.mode == Mode::Normal && !app.command_bar.in_command
        {
            return None;
        }
        app.misc.showcmd.clear();
        if app.editor.ed.has_selection() {
            app.editor.ed.clear_selection();
            return Some(true);
        }
        if app.command_bar.in_command {
            app.command_bar.in_command = false;
            app.editor.cmd_ed.clear();
            app.command_bar.prefix = ':';
            if let Some(backend) = app.services.vim_runtime.backend.as_mut() {
                let _ = backend.clear_preview_search();
            }
            return Some(false);
        }
        if app.misc.mode == Mode::ScanReport || app.misc.mode == Mode::ScanHistory {
            app.misc.mode = app.scan.prev_mode_before_scan;
            return Some(false);
        }
        if app.misc.mode != Mode::Normal {
            app.misc.mode = Mode::Normal;
            return Some(false);
        }
    }

    // Scan views keyboard navigation
    if app.misc.mode == Mode::ScanReport && !app.command_bar.in_command {
        let q_pressed = ctx.input(|i| i.key_pressed(egui::Key::Q));
        if q_pressed {
            app.misc.mode = app.scan.prev_mode_before_scan;
            return Some(false);
        }
    }

    if app.misc.mode == Mode::ScanHistory && !app.command_bar.in_command {
        let up_pressed = ctx.input(|i| i.key_pressed(egui::Key::ArrowUp) || i.key_pressed(egui::Key::K));
        let down_pressed = ctx.input(|i| i.key_pressed(egui::Key::ArrowDown) || i.key_pressed(egui::Key::J));
        let enter_pressed = ctx.input(|i| i.key_pressed(egui::Key::Enter));

        if up_pressed {
            if app.scan.scan_history_selected > 0 {
                app.scan.scan_history_selected -= 1;
            }
            return Some(false);
        }
        if down_pressed {
            if app.scan.scan_history_selected + 1 < app.scan.past_scans.len() {
                app.scan.scan_history_selected += 1;
            }
            return Some(false);
        }
        if enter_pressed {
            if let Some(record) = app.scan.past_scans.get(app.scan.scan_history_selected) {
                let findings: Vec<webscan::Finding> = serde_json::from_str(&record.findings_json).unwrap_or_default();
                let result = webscan::ScanResult {
                    url: record.url.clone(),
                    status_code: 200,
                    response_time_ms: 0,
                    tls: None,
                    server_header: None,
                    page_size_bytes: 0,
                    note: record.note.clone(),
                    findings,
                };
                app.scan.active_scan_result = Some(result);
                app.scan.active_scan_error = None;
                app.scan.scan_report_scroll_y = 0.0;
                app.misc.mode = Mode::ScanReport;
            }
            return Some(false);
        }
    }

    // Suppress egui tab focus navigation so Tab key reaches the editor
    ctx.input_mut(|i| {
        i.consume_key(egui::Modifiers::NONE, egui::Key::Tab);
        i.consume_key(egui::Modifiers::SHIFT, egui::Key::Tab);
    });

    None
}

pub fn window_shortcuts(ctx: &egui::Context) {
    use egui::{CursorIcon, Key, ResizeDirection, ViewportCommand};
    let (drag, f11, quit, is_fs) = ctx.input(|i| (
        i.modifiers.alt && i.pointer.primary_pressed(),
        i.key_pressed(Key::F11),
        (i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(Key::W))
            || (i.modifiers.ctrl && i.key_pressed(Key::Q)),
        i.viewport().fullscreen.unwrap_or(false),
    ));
    if drag {
        ctx.send_viewport_cmd(ViewportCommand::StartDrag);
    }
    if f11 {
        ctx.send_viewport_cmd(ViewportCommand::Fullscreen(!is_fs));
    }
    if quit {
        ctx.send_viewport_cmd(ViewportCommand::Close);
    }

    // Borderless window edge & corner resize grips (6px borders)
    if !is_fs {
        let screen = ctx.screen_rect();
        if let Some(pos) = ctx.input(|i| i.pointer.latest_pos()) {
            let margin = 6.0;
            let on_right = pos.x >= screen.max.x - margin && pos.x <= screen.max.x + 2.0;
            let on_bottom = pos.y >= screen.max.y - margin && pos.y <= screen.max.y + 2.0;
            let on_left = pos.x <= screen.min.x + margin && pos.x >= screen.min.x - 2.0;

            let resize_dir = if on_right && on_bottom {
                Some((ResizeDirection::SouthEast, CursorIcon::ResizeSouthEast))
            } else if on_left && on_bottom {
                Some((ResizeDirection::SouthWest, CursorIcon::ResizeSouthWest))
            } else if on_right {
                Some((ResizeDirection::East, CursorIcon::ResizeEast))
            } else if on_bottom {
                Some((ResizeDirection::South, CursorIcon::ResizeSouth))
            } else if on_left {
                Some((ResizeDirection::West, CursorIcon::ResizeWest))
            } else {
                None
            };

            if let Some((dir, cursor)) = resize_dir {
                ctx.set_cursor_icon(cursor);
                if ctx.input(|i| i.pointer.primary_pressed()) {
                    ctx.send_viewport_cmd(ViewportCommand::BeginResize(dir));
                }
            }
        }
    }
}

#[cfg(target_os = "windows")]
pub fn set_win32_clipboard(text: &str) {
    #[link(name = "user32")]
    #[link(name = "kernel32")]
    extern "system" {
        fn OpenClipboard(hWndNewOwner: *mut std::ffi::c_void) -> i32;
        fn CloseClipboard() -> i32;
        fn EmptyClipboard() -> i32;
        fn SetClipboardData(uFormat: u32, hMem: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
        fn GlobalAlloc(uFlags: u32, dwBytes: usize) -> *mut std::ffi::c_void;
        fn GlobalLock(hMem: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
        fn GlobalUnlock(hMem: *mut std::ffi::c_void) -> i32;
        fn GlobalFree(hMem: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
    }
    const CF_UNICODETEXT: u32 = 13;
    const GMEM_MOVEABLE: u32 = 0x0002;

    let utf16: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let bytes = utf16.len() * std::mem::size_of::<u16>();

    unsafe {
        let mut opened = false;
        for _ in 0..10 {
            if OpenClipboard(std::ptr::null_mut()) != 0 {
                opened = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        if opened {
            EmptyClipboard();
            let hmem = GlobalAlloc(GMEM_MOVEABLE, bytes);
            if !hmem.is_null() {
                let ptr = GlobalLock(hmem) as *mut u16;
                if !ptr.is_null() {
                    std::ptr::copy_nonoverlapping(utf16.as_ptr(), ptr, utf16.len());
                    GlobalUnlock(hmem);
                    if SetClipboardData(CF_UNICODETEXT, hmem).is_null() {
                        GlobalFree(hmem);
                    }
                } else {
                    GlobalFree(hmem);
                }
            }
            CloseClipboard();
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn set_win32_clipboard(_text: &str) {}

#[cfg(target_os = "windows")]
pub fn get_win32_clipboard() -> Option<String> {
    #[link(name = "user32")]
    #[link(name = "kernel32")]
    extern "system" {
        fn OpenClipboard(hWndNewOwner: *mut std::ffi::c_void) -> i32;
        fn CloseClipboard() -> i32;
        fn GetClipboardData(uFormat: u32) -> *mut std::ffi::c_void;
        fn GlobalLock(hMem: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
        fn GlobalUnlock(hMem: *mut std::ffi::c_void) -> i32;
    }
    const CF_UNICODETEXT: u32 = 13;
    unsafe {
        let mut opened = false;
        for _ in 0..10 {
            if OpenClipboard(std::ptr::null_mut()) != 0 {
                opened = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        if opened {
            let handle = GetClipboardData(CF_UNICODETEXT);
            let mut result = None;
            if !handle.is_null() {
                let ptr = GlobalLock(handle) as *const u16;
                if !ptr.is_null() {
                    let mut len = 0;
                    while *ptr.add(len) != 0 {
                        len += 1;
                    }
                    let slice = std::slice::from_raw_parts(ptr, len);
                    result = String::from_utf16(slice).ok();
                    GlobalUnlock(handle);
                }
            }
            CloseClipboard();
            return result;
        }
    }
    None
}

#[cfg(not(target_os = "windows"))]
pub fn get_win32_clipboard() -> Option<String> {
    None
}

pub fn get_clipboard_text(app: &App) -> Option<String> {
    if let Some(text) = get_win32_clipboard() {
        if !text.is_empty() {
            return Some(text);
        }
    }
    if let Some(ref text) = app.misc.clipboard_text {
        if !text.is_empty() {
            return Some(text.clone());
        }
    }
    None
}
