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
pub fn heading_color(level: u8, theme: &Theme) -> Color32 {
    let is_light = theme.is_light();
    match level {
        1 => theme.accent,
        2 => if is_light { Color32::from_rgb(217, 119, 6) } else { Color32::from_rgb(229, 192, 123) },
        3 => if is_light { Color32::from_rgb(2, 132, 199) } else { Color32::from_rgb(97, 175, 239) },
        4 => if is_light { Color32::from_rgb(22, 163, 74) } else { Color32::from_rgb(152, 195, 121) },
        5 => if is_light { Color32::from_rgb(147, 51, 234) } else { Color32::from_rgb(198, 120, 221) },
        _ => if is_light { Color32::from_rgb(13, 148, 136) } else { Color32::from_rgb(86, 182, 194) },
    }
}
