//! MindForge Explorer icon system.
//!
//! Provides classic, clean file and folder icons for the workspace explorer.

use std::path::Path;

/// Returns `(caret_indicator, icon_glyph)` for a filesystem entry.
pub fn get_explorer_icon(
    _path: &Path,
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
        ("", "📄")
    }
}
