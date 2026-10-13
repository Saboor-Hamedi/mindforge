//! MindForge Explorer icon system.
//!
//! Provides a coherent, professional icon language for files, folders,
//! and extensions with clean visual distinction.

use std::path::Path;

/// Returns `(caret_indicator, icon_glyph)` for a filesystem entry.
pub fn get_explorer_icon(
    path: &Path,
    is_dir: bool,
    expanded: bool,
) -> (&'static str, &'static str) {
    if is_dir {
        if expanded {
            ("▾", "📂")
        } else {
            ("▸", "📁")
        }
    } else {
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");

        let icon = match ext.to_ascii_lowercase().as_str() {
            "rs" => "🦀",
            "md" | "markdown" => "📝",
            "py" | "pyw" | "python" => "🐍",
            "js" | "mjs" | "cjs" => "⚡",
            "jsx" => "⚛",
            "ts" | "mts" => "🔷",
            "tsx" => "⚛",
            "json" | "jsonc" => "⚙",
            "toml" | "yaml" | "yml" | "ini" => "⚙",
            "html" | "htm" => "🌐",
            "css" | "scss" | "sass" | "less" => "🎨",
            "sh" | "bash" | "zsh" => "💻",
            "sql" => "🗄",
            "txt" | "log" => "📄",
            _ => {
                let fn_lower = file_name.to_ascii_lowercase();
                if fn_lower.starts_with(".git") {
                    "🌿"
                } else if fn_lower.starts_with(".lumina") || fn_lower.starts_with(".mindforge") {
                    "✨"
                } else if fn_lower.starts_with(".env") {
                    "🔒"
                } else if fn_lower.starts_with('.') {
                    "⚙"
                } else {
                    "📄"
                }
            }
        };
        ("", icon)
    }
}
