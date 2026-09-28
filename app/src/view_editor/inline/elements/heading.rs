//! Heading styling metrics and decorations.
//!
//! Provides seamless, smooth typography matching the normal editor's baseline,
//! avoiding jarring vertical jumps or oversized proportional fonts.

use crate::ui::theme::Theme;
use eframe::egui::{Color32, FontId};

/// Returns font sizing and line height metrics for a given heading level (1..=6).
/// Uses monospace typography with subtle, tasteful scale factors for seamless editor transitions.
pub fn heading_metrics(level: u8, base_font_size: f32) -> (FontId, f32) {
    match level {
        1 => (crate::services::font_manager::editor_font_id(base_font_size * 1.18), (base_font_size * 1.70).round()),
        2 => (crate::services::font_manager::editor_font_id(base_font_size * 1.12), (base_font_size * 1.62).round()),
        3 => (crate::services::font_manager::editor_font_id(base_font_size * 1.06), (base_font_size * 1.58).round()),
        _ => (crate::services::font_manager::editor_font_id(base_font_size), (base_font_size * 1.55).round()),
    }
}

/// Returns the primary text color for a given heading level.
pub fn heading_color(_level: u8, theme: &Theme) -> Color32 {
    theme.text
}
