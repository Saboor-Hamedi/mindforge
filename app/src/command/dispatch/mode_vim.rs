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
            if app.editor.is_dirty {
                let is_cfg_or_script = app
                    .open_notes
                    .get(app.tabs.active_tab)
                    .and_then(|t| t.file_path.as_ref())
                    .and_then(|p| p.extension())
                    .and_then(|e| e.to_str())
                    .is_some_and(|ext| {
                        ext.eq_ignore_ascii_case("lua") || ext.eq_ignore_ascii_case("vim")
                    });
                if is_cfg_or_script {
                    crate::notes::quick_save_active_note(app, now);
                }
            }
            let file_path = args.trim();
            if file_path.is_empty() || file_path == "%" {
                let active_script = app
                    .open_notes
                    .get(app.tabs.active_tab)
                    .and_then(|t| t.file_path.clone())
                    .filter(|p| {
                        p.extension()
                            .and_then(|e| e.to_str())
                            .is_some_and(|ext| {
                                ext.eq_ignore_ascii_case("lua") || ext.eq_ignore_ascii_case("vim")
                            })
                            && !p
                                .file_name()
                                .and_then(|n| n.to_str())
                                .is_some_and(|n| n.eq_ignore_ascii_case("init.lua"))
                    });
                if let Some(backend) = app.services.vim_runtime.backend.as_mut() {
                    if let Some(script_path) = active_script {
                        let escaped = script_path.to_string_lossy().replace('\\', "/");
                        let _ = backend.execute_command(&format!("silent! source {}", escaped));
                    }
                    match backend.reload_user_config() {
                        Ok(_) => {
                            if let Some(ln) = backend.line_numbers_enabled {
                                app.editor.show_line_numbers = ln;
                            }
                            app.set_status("Reloaded Neovim configuration (init.lua)", now);
                        }
                        Err(err) => {
                            app.set_status(format!("Source error: {}", err), now);
                        }
                    }
                } else {
                    app.set_status("Neovim RPC engine is initializing...", now);
                }
                return true;
            }
            if let Some(backend) = app.services.vim_runtime.backend.as_mut() {
                match backend.execute_command(&format!("source {}", file_path)) {
                    Ok(out) => {
                        let _ = backend.reload_user_config();
                        if let Some(ln) = backend.line_numbers_enabled {
                            app.editor.show_line_numbers = ln;
                        }
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
        "init" | "config" | "initconfig" | "nvimconfig" | "init.lua" => {
            if let Some(appdata) = std::env::var_os("APPDATA") {
                let path = std::path::PathBuf::from(appdata)
                    .join("mindforge")
                    .join("mindforge")
                    .join("nvim")
                    .join("init.lua");
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                if !path.exists() {
                    let _ = std::fs::write(&path, crate::vim::bootstrap::DEFAULT_USER_INIT_LUA);
                }
                app.open_file_path(path, now);
                app.set_status("Opened nvim/init.lua", now);
            }
            true
        }
        "lspconfig" => {
            if let Some(appdata) = std::env::var_os("APPDATA") {
                let path = std::path::PathBuf::from(appdata)
                    .join("mindforge")
                    .join("mindforge")
                    .join("nvim")
                    .join("lsp.lua");
                if !path.exists() {
                    if let Some(backend) = app.services.vim_runtime.backend.as_mut() {
                        let _ = backend.execute_lua(
                            "pcall(function() require('mindforge.lsp.config').open() end)",
                        );
                    }
                }
                if path.exists() {
                    app.open_file_path(path, now);
                    return true;
                }
            }
            false
        }
        "formatconfig" => {
            if let Some(appdata) = std::env::var_os("APPDATA") {
                let path = std::path::PathBuf::from(appdata)
                    .join("mindforge")
                    .join("mindforge")
                    .join("nvim")
                    .join("formatters.lua");
                if !path.exists() {
                    if let Some(backend) = app.services.vim_runtime.backend.as_mut() {
                        let _ = backend.execute_lua(
                            "pcall(function() require('mindforge.lsp.format').open_config() end)",
                        );
                    }
                }
                if path.exists() {
                    app.open_file_path(path, now);
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}

/// Canonicalizes lowercase custom command names to their Neovim user-command form
/// while preserving any arguments after the command word.
fn canonicalize_custom_command(cmd_lower: &str, raw: &str) -> String {
    let canonical = match cmd_lower {
        "format" => Some("Format"),
        "formatconfig" => Some("FormatConfig"),
        "init" => Some("Init"),
        "initconfig" => Some("InitConfig"),
        "config" => Some("Config"),
        "lspconfig" => Some("LspConfig"),
        "lspinstall" => Some("LspInstall"),
        "lspuninstall" => Some("LspUninstall"),
        "lsplist" => Some("LspList"),
        "lspinfo" => Some("LspInfo"),
        "lsplog" => Some("LspLog"),
        "lsprestart" => Some("LspRestart"),
        "rename" => Some("Rename"),
        "codeaction" => Some("CodeAction"),
        "references" => Some("References"),
        "hover" => Some("Hover"),
        "definition" => Some("Definition"),
        _ => None,
    };
    if let Some(target) = canonical {
        let trimmed = raw.trim_start();
        let rest = trimmed
            .split_once(char::is_whitespace)
            .map(|(_, args)| args.trim_start())
            .unwrap_or("");
        if rest.is_empty() {
            target.to_string()
        } else {
            format!("{target} {rest}")
        }
    } else {
        raw.to_string()
    }
}

/// Fallback handler: forward native Ex-commands directly through Neovim MessagePack-RPC.
pub fn handle_fallback(app: &mut App, cmd: &str, raw: &str, now: f64) {
    if let Some(backend) = app.services.vim_runtime.backend.as_mut() {
        // First check if user typed Lua syntax directly (e.g. `vim.opt.wrap` or `vim.api...`)
        let is_lua = raw.starts_with("vim.") || raw.starts_with("require(");
        let mapped_raw = canonicalize_custom_command(cmd, raw);
        let result = if is_lua {
            backend.execute_lua(raw)
        } else {
            backend.execute_command(&mapped_raw)
        };

        match result {
            Ok(output) => {
                // Force Neovim to redraw and sync modified text immediately (:sort, :%s, :g/.../d, :.,$d)
                let _ = backend.execute_command("redraw");
                if let Some(ln) = backend.line_numbers_enabled {
                    app.editor.show_line_numbers = ln;
                }
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
