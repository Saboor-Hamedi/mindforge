//! Command line dispatcher for vim-like commands (:w, :r, :d, :mode, :lua, :sound, :caret, :theme, :stats, :quit).
//!
//! Organized into clean domain-driven command modules:
//! - `types`: Command catalog and metadata
//! - `file_ops`: Note lifecycle and file persistence (:w, :r, :d, :new, :clear, :backup, :export, :import)
//! - `view_settings`: Layout and display toggles (:set, :nu, :preview, :zen, :tabs, :sidebar, :titlebar, :noh)
//! - `appearance`: Theme, typography, sound, caret, and opacity (:theme, :sound, :caret, :font, :opacity)
//! - `mode_vim`: Modal switches, Lua script execution, and native Neovim RPC forwarding (:vim, :mode, :lua)
//! - `tools`: Terminal, WebScan security auditor, stats, docs, help, and lifecycle (:term, :scan, :scans, :doc, :quit)

pub mod appearance;
pub mod file_ops;
pub mod mode_vim;
pub mod tools;
pub mod types;
pub mod view_settings;

use crate::app::App;
pub use types::{CommandInfo, COMMAND_CATALOG};

/// Executes an input command line string within the application context.
pub fn execute_command(app: &mut App, raw: &str, now: f64) {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return;
    }
    let trimmed = trimmed.strip_prefix(':').unwrap_or(trimmed).trim();
    if trimmed.is_empty() {
        return;
    }
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    let cmd = parts[0].to_lowercase();
    let args = if parts.len() > 1 {
        trimmed[parts[0].len()..].trim()
    } else {
        ""
    };

    // 1. Try file and document operations
    if file_ops::handle(app, &cmd, args, trimmed, now) {
        return;
    }

    // 2. Try view settings and layout toggles
    if view_settings::handle(app, &cmd, args, trimmed, now) {
        return;
    }

    // 3. Try appearance, themes, sounds, and carets
    if appearance::handle(app, &cmd, args, trimmed, now) {
        return;
    }

    // 4. Try integrated tools, scanner, docs, and lifecycle
    if tools::handle(app, &cmd, args, trimmed, now) {
        return;
    }

    // 5. Try mode switches and explicit Lua commands
    if mode_vim::handle(app, &cmd, args, trimmed, now) {
        return;
    }

    // 6. Fallback: forward to native Neovim MessagePack-RPC in Vim mode
    mode_vim::handle_fallback(app, &cmd, trimmed, now);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backlinks_and_outline_command_catalog() {
        assert!(COMMAND_CATALOG.iter().any(|c| c.name == "backlinks"));
        assert!(COMMAND_CATALOG.iter().any(|c| c.name == "outline"));
        assert!(COMMAND_CATALOG.iter().any(|c| c.name == "lua"));
    }
}
