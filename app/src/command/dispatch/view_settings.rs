//! View layout, UI toggles, and editor settings commands (:set, :nu, :preview, :zen, :tabs, :sidebar, :titlebar, :noh).

use crate::app::App;
use crate::services::db_worker::DbMsg;

pub fn handle(app: &mut App, cmd: &str, args: &str, _raw: &str, now: f64) -> bool {
    match cmd {
        "set" => {
            handle_set(app, args, now);
            true
        }
        "nu" | "number" => {
            app.editor.show_line_numbers = true;
            let _ = app.services.db_tx.send(DbMsg::SaveSetting {
                key: "line_numbers".into(),
                val: "true".into(),
            });
            app.set_status("Line numbers enabled", now);
            true
        }
        "syntax" | "syn" | "synatx" => {
            let arg = args.trim().to_lowercase();
            match arg.as_str() {
                "off" => {
                    if let Some(backend) = app.services.vim_runtime.backend.as_mut() {
                        let _ = backend.execute_command("syntax off");
                    }
                    app.set_status("Syntax highlighting disabled", now);
                }
                "on" | "enable" | "" => {
                    if let Some(backend) = app.services.vim_runtime.backend.as_mut() {
                        let _ = backend.execute_command("syntax enable");
                        let _ = backend.execute_command("setlocal filetype=markdown syntax=markdown");
                        backend.sync_theme(&app.misc.theme);
                    }
                    app.set_status("Syntax highlighting enabled", now);
                }
                other => {
                    if let Some(backend) = app.services.vim_runtime.backend.as_mut() {
                        let _ = backend.execute_command(&format!("syntax {}", other));
                        backend.sync_theme(&app.misc.theme);
                    }
                    app.set_status(format!("Syntax: {}", other), now);
                }
            }
            true
        }
        "nonu" | "nonumber" => {
            app.editor.show_line_numbers = false;
            let _ = app.services.db_tx.send(DbMsg::SaveSetting {
                key: "line_numbers".into(),
                val: "false".into(),
            });
            app.set_status("Line numbers hidden", now);
            true
        }
        "preview" | "p" => {
            app.editor.preview_open = true;
            app.right_pane.tab = crate::app::RightPaneTab::Preview;
            let _ = app.services.db_tx.send(DbMsg::SaveSetting {
                key: "preview".into(),
                val: "true".into(),
            });
            app.set_status("Live markdown preview opened (Ctrl+\\)", now);
            true
        }
        "nopreview" | "nop" => {
            app.editor.preview_open = false;
            let _ = app.services.db_tx.send(DbMsg::SaveSetting {
                key: "preview".into(),
                val: "false".into(),
            });
            app.set_status("Live markdown preview closed", now);
            true
        }
        "titlebar" | "tb" => {
            app.misc.show_titlebar = !app.misc.show_titlebar;
            let _ = app.services.db_tx.send(DbMsg::SaveSetting {
                key: "show_titlebar".into(),
                val: if app.misc.show_titlebar { "true" } else { "false" }.into(),
            });
            let msg = if app.misc.show_titlebar { "Titlebar visible" } else { "Titlebar hidden" };
            app.set_status(msg, now);
            true
        }
        "sidebar" | "sb" => {
            app.sidebar.open = !app.sidebar.open;
            let msg = if app.sidebar.open { "Sidebar opened" } else { "Sidebar closed" };
            app.set_status(msg, now);
            true
        }
        "tabs" | "tabbar" => {
            app.misc.show_tabs = !app.misc.show_tabs;
            let _ = app.services.db_tx.send(DbMsg::SaveSetting {
                key: "show_tabs".into(),
                val: if app.misc.show_tabs { "true" } else { "false" }.into(),
            });
            let msg = if app.misc.show_tabs { "Document tabs visible" } else { "Document tabs hidden" };
            app.set_status(msg, now);
            true
        }
        "ai" | "agent" => {
            if app.editor.preview_open && app.right_pane.tab == crate::app::RightPaneTab::AiAgent {
                app.editor.preview_open = false;
                let _ = app.services.db_tx.send(DbMsg::SaveSetting {
                    key: "preview".into(),
                    val: "false".into(),
                });
                app.set_status("AI Assistant closed", now);
            } else {
                app.editor.preview_open = true;
                app.right_pane.tab = crate::app::RightPaneTab::AiAgent;
                let _ = app.services.db_tx.send(DbMsg::SaveSetting {
                    key: "preview".into(),
                    val: "true".into(),
                });
                app.set_status("DeepSeek AI Assistant opened (Ctrl+Shift+I)", now);
            }
            true
        }
        "backlinks" | "bl" | "links" => {
            if app.editor.preview_open && app.right_pane.tab == crate::app::RightPaneTab::Backlinks {
                app.editor.preview_open = false;
                let _ = app.services.db_tx.send(DbMsg::SaveSetting {
                    key: "preview".into(),
                    val: "false".into(),
                });
                app.set_status("Backlinks panel closed", now);
            } else {
                app.editor.preview_open = true;
                app.right_pane.tab = crate::app::RightPaneTab::Backlinks;
                let _ = app.services.db_tx.send(DbMsg::SaveSetting {
                    key: "preview".into(),
                    val: "true".into(),
                });
                app.set_status("Backlinks panel opened (Ctrl+I)", now);
            }
            true
        }
        "outline" | "ol" | "headings" => {
            if app.editor.preview_open && app.right_pane.tab == crate::app::RightPaneTab::Outline {
                app.editor.preview_open = false;
                let _ = app.services.db_tx.send(DbMsg::SaveSetting {
                    key: "preview".into(),
                    val: "false".into(),
                });
                app.set_status("Outline panel closed", now);
            } else {
                app.editor.preview_open = true;
                app.right_pane.tab = crate::app::RightPaneTab::Outline;
                let _ = app.services.db_tx.send(DbMsg::SaveSetting {
                    key: "preview".into(),
                    val: "true".into(),
                });
                app.set_status("Outline panel opened (Ctrl+Shift+O)", now);
            }
            true
        }
        "zen" | "zenmode" => {
            match args.to_lowercase().trim() {
                "on" | "enable" | "1" | "true" => {
                    app.misc.zen_mode = true;
                    app.misc.show_titlebar = false;
                    app.misc.show_tabs = false;
                    app.sidebar.open = false;
                    app.editor.preview_open = false;
                }
                "off" | "disable" | "0" | "false" => {
                    app.misc.zen_mode = false;
                    app.misc.show_titlebar = true;
                    app.misc.show_tabs = true;
                }
                _ => {
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
                }
            }
            let _ = app.services.db_tx.send(DbMsg::SaveSetting {
                key: "zen_mode".into(),
                val: if app.misc.zen_mode { "true" } else { "false" }.into(),
            });
            let msg = if app.misc.zen_mode {
                "Zen Mode Enabled (Ctrl+. to exit)"
            } else {
                "Zen Mode Disabled"
            };
            app.set_status(msg, now);
            true
        }
        "dashboard" | "welcome" | "home" => {
            app.misc.show_welcome = !app.misc.show_welcome;
            let msg = if app.misc.show_welcome {
                "Welcome Dashboard opened"
            } else {
                "Returned to note editor"
            };
            app.set_status(msg, now);
            true
        }
        "blur" | "acrylic" | "mica" | "noblur" => {
            let eff = match cmd {
                "noblur" => crate::services::blur::BlurEffect::None,
                "acrylic" => crate::services::blur::BlurEffect::Acrylic,
                "mica" => crate::services::blur::BlurEffect::Mica,
                _ => match args.to_lowercase().trim() {
                    "off" | "disable" | "0" | "false" | "none" => crate::services::blur::BlurEffect::None,
                    "acrylic" => crate::services::blur::BlurEffect::Acrylic,
                    "mica" => crate::services::blur::BlurEffect::Mica,
                    "on" | "enable" | "1" | "true" => crate::services::blur::BlurEffect::Acrylic,
                    _ => {
                        if app.misc.blur_effect == crate::services::blur::BlurEffect::None {
                            crate::services::blur::BlurEffect::Acrylic
                        } else {
                            crate::services::blur::BlurEffect::None
                        }
                    }
                },
            };
            app.misc.blur_effect = eff;
            crate::services::blur::apply_window_blur(eff);
            let val = match eff {
                crate::services::blur::BlurEffect::Acrylic => "acrylic",
                crate::services::blur::BlurEffect::Mica => "mica",
                crate::services::blur::BlurEffect::None => "none",
            };
            let _ = app.services.db_tx.send(DbMsg::SaveSetting {
                key: "blur".into(),
                val: val.into(),
            });
            app.set_status(format!("Backdrop blur set to {:?}", eff), now);
            true
        }
        "live" | "inline" | "livepreview" => {
            app.editor.inline_mode = false;
            let _ = app.services.db_tx.send(DbMsg::SaveSetting {
                key: "inline_mode".into(),
                val: "false".into(),
            });
            app.set_status("Live inline Markdown is disabled; raw editing is active", now);
            true
        }
        "raw" | "source" => {
            app.editor.inline_mode = false;
            let _ = app.services.db_tx.send(DbMsg::SaveSetting {
                key: "inline_mode".into(),
                val: "false".into(),
            });
            app.set_status("📝 Raw Monospace Mode ENABLED (Ctrl+E to toggle)", now);
            true
        }
        "noh" | "nohl" | "nohlsearch" => {
            if let Some(backend) = app.services.vim_runtime.backend.as_mut() {
                let _ = backend.send_input(":noh<CR>");
            }
            app.set_status("Search highlighting cleared (:noh)", now);
            true
        }
        _ => false,
    }
}

fn handle_set(app: &mut App, args: &str, now: f64) {
    let opt = args.to_lowercase();
    match opt.as_str() {
        "nu" | "number" => {
            app.editor.show_line_numbers = true;
            let _ = app.services.db_tx.send(DbMsg::SaveSetting {
                key: "line_numbers".into(),
                val: "true".into(),
            });
            app.set_status("Line numbers enabled", now);
        }
        "nonu" | "nonumber" => {
            app.editor.show_line_numbers = false;
            let _ = app.services.db_tx.send(DbMsg::SaveSetting {
                key: "line_numbers".into(),
                val: "false".into(),
            });
            app.set_status("Line numbers hidden", now);
        }
        "preview" | "p" => {
            app.editor.preview_open = true;
            app.right_pane.tab = crate::app::RightPaneTab::Preview;
            let _ = app.services.db_tx.send(DbMsg::SaveSetting {
                key: "preview".into(),
                val: "true".into(),
            });
            app.set_status("Live markdown preview opened (Ctrl+\\)", now);
        }
        "nopreview" | "nop" => {
            app.editor.preview_open = false;
            let _ = app.services.db_tx.send(DbMsg::SaveSetting {
                key: "preview".into(),
                val: "false".into(),
            });
            app.set_status("Live markdown preview closed", now);
        }
        "titlebar" | "tb" => {
            app.misc.show_titlebar = true;
            let _ = app.services.db_tx.send(DbMsg::SaveSetting {
                key: "show_titlebar".into(),
                val: "true".into(),
            });
            app.set_status("Titlebar enabled", now);
        }
        "notitlebar" | "notb" => {
            app.misc.show_titlebar = false;
            let _ = app.services.db_tx.send(DbMsg::SaveSetting {
                key: "show_titlebar".into(),
                val: "false".into(),
            });
            app.set_status("Titlebar hidden", now);
        }
        "sidebar" | "sb" => {
            app.sidebar.open = true;
            app.set_status("Sidebar opened", now);
        }
        "nosidebar" | "nosb" => {
            app.sidebar.open = false;
            app.set_status("Sidebar hidden", now);
        }
        "tabs" | "tabbar" => {
            app.misc.show_tabs = true;
            let _ = app.services.db_tx.send(DbMsg::SaveSetting {
                key: "show_tabs".into(),
                val: "true".into(),
            });
            app.set_status("Document tabs visible", now);
        }
        "notabs" | "notabbar" => {
            app.misc.show_tabs = false;
            let _ = app.services.db_tx.send(DbMsg::SaveSetting {
                key: "show_tabs".into(),
                val: "false".into(),
            });
            app.set_status("Document tabs hidden", now);
        }
        "ai" | "agent" => {
            app.editor.preview_open = true;
            app.right_pane.tab = crate::app::RightPaneTab::AiAgent;
            let _ = app.services.db_tx.send(DbMsg::SaveSetting {
                key: "preview".into(),
                val: "true".into(),
            });
            app.set_status("DeepSeek AI Assistant opened (Ctrl+Shift+I)", now);
        }
        "noai" | "noagent" => {
            if app.right_pane.tab == crate::app::RightPaneTab::AiAgent {
                app.editor.preview_open = false;
                let _ = app.services.db_tx.send(DbMsg::SaveSetting {
                    key: "preview".into(),
                    val: "false".into(),
                });
            }
            app.set_status("AI Assistant closed", now);
        }
        "backlinks" | "bl" | "links" => {
            app.editor.preview_open = true;
            app.right_pane.tab = crate::app::RightPaneTab::Backlinks;
            let _ = app.services.db_tx.send(DbMsg::SaveSetting {
                key: "preview".into(),
                val: "true".into(),
            });
            app.set_status("Backlinks panel opened (Ctrl+I)", now);
        }
        "nobacklinks" | "nobl" | "nolinks" => {
            if app.right_pane.tab == crate::app::RightPaneTab::Backlinks {
                app.editor.preview_open = false;
                let _ = app.services.db_tx.send(DbMsg::SaveSetting {
                    key: "preview".into(),
                    val: "false".into(),
                });
            }
            app.set_status("Backlinks panel closed", now);
        }
        "outline" | "ol" | "headings" => {
            app.editor.preview_open = true;
            app.right_pane.tab = crate::app::RightPaneTab::Outline;
            let _ = app.services.db_tx.send(DbMsg::SaveSetting {
                key: "preview".into(),
                val: "true".into(),
            });
            app.set_status("Outline panel opened (Ctrl+Shift+O)", now);
        }
        "nooutline" | "nool" | "noheadings" => {
            if app.right_pane.tab == crate::app::RightPaneTab::Outline {
                app.editor.preview_open = false;
                let _ = app.services.db_tx.send(DbMsg::SaveSetting {
                    key: "preview".into(),
                    val: "false".into(),
                });
            }
            app.set_status("Outline panel closed", now);
        }
        _ => {
            // Forward unknown settings options to Neovim in Vim mode
            if app.services.editor_controller.mode == crate::app::EditorInputMode::Vim {
                if let Some(backend) = app.services.vim_runtime.backend.as_mut() {
                    match backend.execute_command(&format!("set {}", args)) {
                        Ok(out) => {
                            if !out.is_empty() {
                                app.set_status(out, now);
                            }
                        }
                        Err(e) => {
                            app.set_status(format!("Vim set error: {}", e), now);
                        }
                    }
                    return;
                }
            }
            app.set_status(format!("Unknown option: :set {}. Valid: nu, nonu, preview, nopreview, titlebar, sidebar, tabs, ai, backlinks, outline, zen", args), now);
        }
    }
}
