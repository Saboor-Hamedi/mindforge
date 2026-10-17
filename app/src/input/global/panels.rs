//! Panels, sidebars, tools, and dock toggling shortcuts.

use crate::app::App;
use crate::mode::Mode;
use eframe::egui;

pub fn handle_panel_shortcuts(app: &mut App, ctx: &egui::Context, now: f64) -> Option<bool> {
    let is_panel_nav_active = app.editor.preview_open
        && (app.misc.mode == Mode::Normal || app.misc.mode == Mode::Doc)
        && (app.right_pane.tab == crate::app::RightPaneTab::Preview
            || app.right_pane.tab == crate::app::RightPaneTab::Backlinks
            || app.right_pane.tab == crate::app::RightPaneTab::Outline);

    // While the Neovim completion popup is open, Ctrl+J/K move the selection
    // instead of toggling the terminal.
    if !app.command_bar.in_command
        && app
            .services
            .vim_runtime
            .backend
            .as_ref()
            .is_some_and(|b| b.popup_visible())
    {
        let step = ctx.input_mut(|i| {
            if i.consume_key(egui::Modifiers::CTRL, egui::Key::J) {
                Some("<C-n>")
            } else if i.consume_key(egui::Modifiers::CTRL, egui::Key::K) {
                Some("<C-p>")
            } else {
                None
            }
        });
        if let Some(keys) = step {
            if let Some(backend) = app.services.vim_runtime.backend.as_mut() {
                let _ = backend.send_input(keys);
            }
            return Some(false);
        }
    }

    // Terminal Toggle: Ctrl+J or Ctrl+Backtick
    let toggle_term = !app.command_bar.in_command
        && !is_panel_nav_active
        && ctx.input(|i| {
            (i.modifiers.ctrl
                && !i.modifiers.shift
                && !i.modifiers.alt
                && i.key_pressed(egui::Key::J))
                || (i.modifiers.ctrl && i.key_pressed(egui::Key::Backtick))
        });
    if toggle_term {
        app.terminal.open = !app.terminal.open;
        if app.terminal.open {
            app.terminal.prev_mode_before_term = app.misc.mode;
            app.terminal.focused = true;
            app.set_status(
                "Terminal opened (Ctrl+J to toggle, click editor or Esc to edit)",
                now,
            );
        } else {
            app.terminal.focused = false;
            app.set_status("Terminal closed", now);
        }
        return Some(false);
    }

    // AI Agent Right Pane Toggle: Ctrl+Shift+I
    let toggle_ai = ctx.input(|i| {
        (i.modifiers.ctrl || i.modifiers.command)
            && i.modifiers.shift
            && !i.modifiers.alt
            && i.key_pressed(egui::Key::I)
    });
    if toggle_ai && !app.command_bar.in_command {
        if app.editor.preview_open && app.right_pane.tab == crate::app::RightPaneTab::AiAgent {
            app.editor.preview_open = false;
            app.services.agent_state.is_open = false;
            ctx.memory_mut(|m| m.surrender_focus(egui::Id::new("deepseek_prompt_input")));
            app.set_status("AI Agent closed", now);
        } else {
            app.editor.preview_open = true;
            app.services.agent_state.is_open = true;
            app.right_pane.tab = crate::app::RightPaneTab::AiAgent;
            app.set_status("AI Agent opened (Ctrl+Shift+I)", now);
        }
        return Some(false);
    }

    // Split Preview Toggle: Ctrl+\
    let toggle_preview = ctx.input(|i| {
        (i.modifiers.ctrl || i.modifiers.command)
            && !i.modifiers.alt
            && !i.modifiers.shift
            && i.key_pressed(egui::Key::Backslash)
    });
    if toggle_preview && !app.command_bar.in_command {
        if app.editor.preview_open && app.right_pane.tab == crate::app::RightPaneTab::Preview {
            app.editor.preview_open = false;
            app.set_status("Split Preview closed (Ctrl+\\)", now);
        } else {
            app.editor.preview_open = true;
            app.right_pane.tab = crate::app::RightPaneTab::Preview;
            app.set_status("Split Preview opened (Ctrl+\\)", now);
        }
        return Some(false);
    }

    // Outline Toggle: Ctrl+Shift+O
    let toggle_outline = ctx.input(|i| {
        (i.modifiers.ctrl || i.modifiers.command)
            && i.modifiers.shift
            && !i.modifiers.alt
            && i.key_pressed(egui::Key::O)
    });
    if toggle_outline && !app.command_bar.in_command {
        if app.editor.preview_open && app.right_pane.tab == crate::app::RightPaneTab::Outline {
            app.editor.preview_open = false;
            app.set_status("Outline panel closed", now);
        } else {
            app.editor.preview_open = true;
            app.right_pane.tab = crate::app::RightPaneTab::Outline;
            app.set_status("Outline panel opened (Ctrl+Shift+O)", now);
        }
        return Some(false);
    }

    // AI Assistant Input Priority: when user is focused on the AI prompt textarea
    if !app.command_bar.in_command
        && app.editor.preview_open
        && app.right_pane.tab == crate::app::RightPaneTab::AiAgent
    {
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

    // Sidebar Toggle: Ctrl+B
    let ctrl_b = ctx.input(|i| {
        i.modifiers.ctrl && !i.modifiers.alt && !i.modifiers.shift && i.key_pressed(egui::Key::B)
    });
    if ctrl_b && !app.command_bar.in_command {
        app.sidebar.open = !app.sidebar.open;
        if app.sidebar.open {
            if app.misc.mode == Mode::Doc {
                app.tabs.doc_sidebar_focused = true;
            } else {
                app.sidebar.focused = true;
            }
            app.set_status("Sidebar opened", now);
        } else {
            app.sidebar.focused = false;
            app.tabs.doc_sidebar_focused = false;
            app.set_status("Sidebar closed", now);
        }
        let _ = app
            .services
            .db_tx
            .send(crate::services::settings_store::StorageMsg::SaveSetting {
                key: "sidebar".into(),
                val: if app.sidebar.open {
                    "true".into()
                } else {
                    "false".into()
                },
            });
        return Some(false);
    }

    // Sidebar F2: Rename selected file or folder in the Rename modal.
    let f2 = ctx.input(|i| i.key_pressed(egui::Key::F2));
    if f2 && !app.command_bar.in_command && app.sidebar.open && app.sidebar.focused {
        let target = app
            .workspace
            .selected_path
            .clone()
            .or_else(|| app.workspace.selected_items.iter().next().cloned());
        if let Some(path) = target {
            app.modal.pending_rename_path = Some(path.clone());
            app.modal.rename_input = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            app.modal.rename_open = true;
            app.modal.rename_just_opened = true;
            return Some(false);
        } else if !app.notes.notes_list.is_empty() {
            app.modal.pending_rename_path = None;
            app.modal.rename_input = app.notes.active_note_title.clone();
            app.modal.rename_open = true;
            app.modal.rename_just_opened = true;
            return Some(false);
        }
    }

    // Toggle Documentation Mode: F2.
    // While the embedded Neovim editor owns input, F2 must reach it so the
    // documented <F2> LSP rename works; other LSP keys (K, gd, gr, ...) are
    // forwarded for the same reason.
    let neovim_owns_keys = app.services.vim_runtime.backend.is_some()
        && matches!(app.misc.mode, Mode::Normal | Mode::Doc);
    if f2 && !app.command_bar.in_command && !neovim_owns_keys {
        if app.misc.mode == Mode::Doc {
            app.misc.mode = Mode::Normal;
            app.set_status("Returned to Notes Editor", now);
        } else {
            app.open_docs_mode(now);
        }
        return Some(false);
    }

    // Toggle Web Scanner: F3
    let f3 = ctx.input(|i| i.key_pressed(egui::Key::F3));
    if f3 && !app.command_bar.in_command {
        if matches!(app.misc.mode, Mode::ScanReport | Mode::ScanHistory) {
            app.misc.mode = app.scan.prev_mode_before_scan;
            app.set_status("Exited Scanner", now);
        } else {
            app.scan.prev_mode_before_scan = app.misc.mode;
            app.scan.active_scan_result = None;
            app.scan.active_scan_error = None;
            app.scan.scan_report_scroll_y = 0.0;
            app.misc.mode = Mode::ScanReport;
            app.set_status("Scanner ready — run :scan <url>", now);
        }
        return Some(false);
    }

    // Focus Sidebar Files: Ctrl+H — always select the file that's open on the
    // active tab so the user sees where they are in the tree.
    let ctrl_h = ctx.input(|i| {
        i.modifiers.ctrl && !i.modifiers.shift && !i.modifiers.alt && i.key_pressed(egui::Key::H)
    });
    if ctrl_h && !app.command_bar.in_command {
        if app.misc.mode == Mode::Doc {
            ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::H));
            app.tabs.doc_sidebar_focused = true;
            app.set_status("Doc sidebar focused (Ctrl+L for reader)", now);
            return Some(false);
        } else if app.sidebar.open {
            ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::H));
            app.sidebar.focused = true;
            let mut revealed = false;
            if let Some(tab) = app.open_notes.get(app.tabs.active_tab) {
                if let Some(path) = &tab.file_path {
                    app.workspace.selected_path = Some(path.clone());
                    app.workspace.reveal(path);
                    app.persist_workspace_expansion();
                    app.sidebar.needs_scroll = true;
                    revealed = true;
                }
            }
            if !revealed && app.workspace.selected_path.is_none() {
                if let Some(first) = app.workspace.entries.first() {
                    let first_path = first.path.clone();
                    app.workspace.selected_path = Some(first_path.clone());
                    app.workspace.selected_items.clear();
                    app.workspace.selected_items.insert(first_path.clone());
                    app.workspace.selection_anchor = Some(first_path);
                    app.sidebar.needs_scroll = true;
                }
            } else if let Some(path) = &app.workspace.selected_path {
                app.workspace.selected_items.clear();
                app.workspace.selected_items.insert(path.clone());
                app.workspace.selection_anchor = Some(path.clone());
                app.sidebar.needs_scroll = true;
            }
            app.set_status("Sidebar focused (Ctrl+L for editor, j/k to navigate)", now);
            return Some(false);
        }
    }

    // Panel Navigation: Ctrl+L (Focus Editor / Right Pane)
    let ctrl_l = ctx.input(|i| {
        i.modifiers.ctrl && !i.modifiers.shift && !i.modifiers.alt && i.key_pressed(egui::Key::L)
    });
    if ctrl_l && !app.command_bar.in_command {
        ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::L));
        if app.sidebar.open && (app.sidebar.focused || app.tabs.doc_sidebar_focused) {
            if app.misc.mode == Mode::Doc {
                app.tabs.doc_sidebar_focused = false;
                app.set_status("Doc reader focused (Ctrl+H for sidebar)", now);
            } else {
                app.sidebar.focused = false;
                app.misc.caret.gliding = false;
                app.set_status("Editor focused (Ctrl+H for sidebar)", now);
            }
            return Some(false);
        } else if is_panel_nav_active {
            app.sidebar.focused = false;
            app.set_status("Right pane focused (Ctrl+H for editor/sidebar)", now);
            return Some(false);
        } else if app.sidebar.open {
            app.sidebar.focused = false;
            app.misc.caret.gliding = false;
            app.set_status("Editor focused (Ctrl+H for sidebar)", now);
            return Some(false);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ctrl_h_is_not_consumed_as_a_sidebar_shortcut() {
        let mut app = App::new();
        app.sidebar.open = false;
        let ctx = egui::Context::default();
        let mut result = None;
        let raw = egui::RawInput {
            modifiers: egui::Modifiers::CTRL,
            events: vec![egui::Event::Key {
                key: egui::Key::H,
                physical_key: Some(egui::Key::H),
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::CTRL,
            }],
            ..Default::default()
        };
        let _ = ctx.run(raw, |ctx| {
            result = handle_panel_shortcuts(&mut app, ctx, 1.0);
        });
        assert_eq!(result, None);
    }

    #[test]
    fn test_ctrl_h_focuses_open_sidebar_and_ctrl_l_returns_to_editor() {
        let mut app = App::new();
        app.sidebar.open = true;
        app.sidebar.focused = false;
        let ctx = egui::Context::default();

        // 1. Press Ctrl+H -> moves focus to sidebar
        let raw_h = egui::RawInput {
            modifiers: egui::Modifiers::CTRL,
            events: vec![egui::Event::Key {
                key: egui::Key::H,
                physical_key: Some(egui::Key::H),
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::CTRL,
            }],
            ..Default::default()
        };
        let mut res_h = None;
        let _ = ctx.run(raw_h, |ctx| {
            res_h = handle_panel_shortcuts(&mut app, ctx, 1.0);
        });
        assert_eq!(res_h, Some(false));
        assert!(app.sidebar.focused);

        // 2. Press Ctrl+L -> moves focus back to editor
        let raw_l = egui::RawInput {
            modifiers: egui::Modifiers::CTRL,
            events: vec![egui::Event::Key {
                key: egui::Key::L,
                physical_key: Some(egui::Key::L),
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::CTRL,
            }],
            ..Default::default()
        };
        let mut res_l = None;
        let _ = ctx.run(raw_l, |ctx| {
            res_l = handle_panel_shortcuts(&mut app, ctx, 1.0);
        });
        assert_eq!(res_l, Some(false));
        assert!(!app.sidebar.focused);
    }
}
