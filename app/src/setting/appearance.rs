//! Unified Appearance settings: themes, fonts, carets, and window blur effects.

pub use super::carets::{caret_dot_color, render_carets_tab};
pub use super::setting_font::render_font_settings;
pub use super::theme::render_theme_tab;

use crate::caret::Caret;
use crate::services::blur::BlurEffect;
use crate::ui::theme::Theme;
use eframe::egui::{self, Pos2, Rect, Ui};

/// Dispatches rendering for appearance sub-components.
pub fn render_appearance_section(
    ui: &mut Ui,
    painter: &egui::Painter,
    panel_rect: Rect,
    p_origin: Pos2,
    theme: &mut Theme,
    opacity: &mut f32,
    blur_effect: &mut BlurEffect,
    _selected_font: &mut String,
    _font_size: &mut f32,
    _caret: &mut Caret,
    on_save_setting: &mut dyn FnMut(&str, &str),
) {
    render_theme_tab(
        ui,
        painter,
        panel_rect,
        p_origin,
        theme,
        opacity,
        blur_effect,
        on_save_setting,
    );
}
