//! Blockquote styling for preview.

use crate::ui::theme::Theme;
use eframe::egui::Color32;

/// Returns the quote text color.
pub fn quote_color(theme: &Theme) -> Color32 {
    if theme.is_light() {
        Color32::from_rgb(
            ((theme.text.r() as u16 * 7 + theme.muted.r() as u16 * 3) / 10) as u8,
            ((theme.text.g() as u16 * 7 + theme.muted.g() as u16 * 3) / 10) as u8,
            ((theme.text.b() as u16 * 7 + theme.muted.b() as u16 * 3) / 10) as u8,
        )
    } else {
        Color32::from_rgb(
            ((theme.text.r() as u16 * 8 + theme.muted.r() as u16 * 2) / 10) as u8,
            ((theme.text.g() as u16 * 8 + theme.muted.g() as u16 * 2) / 10) as u8,
            ((theme.text.b() as u16 * 8 + theme.muted.b() as u16 * 2) / 10) as u8,
        )
    }
}
