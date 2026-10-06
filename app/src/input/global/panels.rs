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
            .send(crate::services::db_worker::DbMsg::SaveSetting {
                key: "sidebar".into(),
                val: if app.sidebar.open {
                    "true".into()
                } else {
                    "false".into()
                },
            });
        return Some(false);
    }

    // Toggle Documentation Mode: F2
    let f2 = ctx.input(|i| i.key_pressed(egui::Key::F2));
    if f2 && !app.command_bar.in_command {
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

    // Panel Navigation: Ctrl+L (Focus Editor / Right Pane)
    let ctrl_l = ctx.input(|i| {
        i.modifiers.ctrl && !i.modifiers.shift && !i.modifiers.alt && i.key_pressed(egui::Key::L)
    });
    if ctrl_l && !app.command_bar.in_command {
        if app.sidebar.open && (app.sidebar.focused || app.tabs.doc_sidebar_focused) {
            if app.misc.mode == Mode::Doc {
                app.tabs.doc_sidebar_focused = false;
                app.set_status("Doc reader focused (Ctrl+H for sidebar)", now);
            } else {
                app.sidebar.focused = false;
                app.set_status("Editor focused (Ctrl+H for sidebar)", now);
            }
            return Some(false);
        } else if is_panel_nav_active {
            app.sidebar.focused = false;
            app.set_status("Right pane focused (Ctrl+H for editor/sidebar)", now);
            return Some(false);
        } else if app.sidebar.open {
            app.sidebar.focused = false;
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
        let ctx = egui::Context::default();
        let mut result = None;
        let raw = egui::RawInput {
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
}
