//! UI utility modules for MINDFORGE's interface layer.
//!
//! These modules provide reusable UI infrastructure — theming, zoom,
//! documentation, and panels — used by multiple views and panels
//! throughout the application. Terminal integration lives in `crate::terminal`.

pub mod docs;
pub mod help_panel;
pub mod menu;
pub mod palette;
pub mod showcmd;
pub mod theme;
pub mod zoom;

/// Truncates text to at most `max_chars` Unicode scalar values, adding an
/// ellipsis when the value is longer. The limit includes the ellipsis.
pub fn truncate_with_ellipsis(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let prefix: String = chars.by_ref().take(max_chars.saturating_sub(3)).collect();
    if chars.next().is_some() {
        format!("{prefix}{}", ".".repeat(max_chars.min(3)))
    } else {
        value.to_owned()
    }
}
