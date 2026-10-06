//! Vim mode info, Lua script execution, and native Neovim MessagePack-RPC command forwarding.

use crate::app::App;
use crate::mode::Mode;

pub fn handle(app: &mut App, cmd: &str, args: &str, _raw: &str, now: f64) -> bool {
    match cmd {
        "vim" | "mode" => {
            app.set_status("MindForge runs on Neovim (Modal Engine active)", now);
            true
        }
        "lua" => {
            let lua_code = args.trim();
            if lua_code.is_empty() {
                app.set_status("Usage: :lua <lua expression or statement>", now);
                return true;
            }
            if let Some(backend) = app.services.vim_runtime.backend.as_mut() {
                match backend.execute_lua(lua_code) {
                    Ok(output) => {
                        let msg = if output.is_empty() {
                            "Lua executed successfully".to_string()
                        } else {
                            format!("=> {}", output)
                        };
                        app.set_status(msg, now);
                    }
                    Err(err) => {
                        app.set_status(format!("Lua error: {}", err), now);
                    }
                }
            } else {
                app.set_status("Neovim RPC engine is initializing...", now);
            }
            true
        }
        "source" | "so" => {
            let file_path = args.trim();
            if file_path.is_empty() {
                app.set_status("Usage: :source <path/to/script.vim or script.lua>", now);
                return true;
            }
            if let Some(backend) = app.services.vim_runtime.backend.as_mut() {
                match backend.execute_command(&format!("source {}", file_path)) {
                    Ok(out) => {
                        let msg = if out.is_empty() {
                            format!("Sourced: {}", file_path)
                        } else {
                            out
                        };
                        app.set_status(msg, now);
                    }
                    Err(err) => {
                        app.set_status(format!("Source error: {}", err), now);
                    }
                }
            } else {
                app.set_status("Neovim RPC engine is initializing...", now);
            }
            true
        }
        _ => false,
    }
}

/// Fallback handler: forward native Ex-commands directly through Neovim MessagePack-RPC.
pub fn handle_fallback(app: &mut App, cmd: &str, raw: &str, now: f64) {
    if let Some(backend) = app.services.vim_runtime.backend.as_mut() {
        // First check if user typed Lua syntax directly (e.g. `vim.opt.wrap` or `vim.api...`)
        let is_lua = raw.starts_with("vim.") || raw.starts_with("require(");
        let result = if is_lua {
            backend.execute_lua(raw)
        } else {
            backend.execute_command(raw)
        };

        match result {
            Ok(output) => {
                // Force Neovim to redraw and sync modified text immediately (:sort, :%s, :g/.../d, :.,$d)
                let _ = backend.execute_command("redraw");
                if let Some(new_text) = backend.force_sync_text() {
                    let current_text = if app.misc.mode == Mode::Doc {
                        app.editor.doc_ed.text()
                    } else {
                        app.editor.ed.text()
                    };
                    if new_text != current_text {
                        if app.misc.mode == Mode::Doc {
                            let cur = app.editor.doc_ed.cur;
                            app.editor.doc_ed.set_text(&new_text);
                            app.editor.doc_ed.cur = cur.min(app.editor.doc_ed.buf.len());
                        } else {
                            let cur = app.editor.ed.cur;
                            app.editor.ed.set_text(&new_text);
                            app.editor.ed.cur = cur.min(app.editor.ed.buf.len());
                            app.editor.is_dirty = true;
                            app.misc.last_char_time = now;
                            app.sync_active_tab();
                        }
                    }
                }
                if !output.is_empty() {
                    app.set_status(output, now);
                }
            }
            Err(err) => {
                // Clean error reporting: do NOT send raw keys into Neovim input stream,
                // which causes interactive prompt deadlocks, keystroke pollution, or exits.
                if !err.is_empty() {
                    app.set_status(format!("Vim: {}", err), now);
                }
            }
        }
    } else {
        app.set_status(
            format!("Unknown command: :{}. Type :help for documentation", cmd),
            now,
        );
    }
}
