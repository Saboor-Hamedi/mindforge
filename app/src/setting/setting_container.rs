//! Settings modal container window, backdrop overlay, card frame, and interaction routing.

use super::setting_panel::{render_setting_panel, SettingPanelAction};
use super::setting_tab::render_setting_tabs;
use crate::caret::Caret;
use crate::services::sound::SoundEngine;
use crate::services::updater::UpdateManager;
use crate::state::modal::ModalState;
use crate::ui::theme::Theme;
use eframe::egui::{self, pos2, Color32, Rect, Stroke, Ui};

/// Renders the complete settings container: backdrop, floating window card, tab strip, and feature panel.
/// Returns any requested panel action (e.g. backup, update check).
pub fn render_setting_container(
    ui: &mut Ui,
    painter: &egui::Painter,
    bounds: Rect,
    modal: &mut ModalState,
    caret: &mut Caret,
    sound: &mut SoundEngine,
    theme: &mut Theme,
    selected_font: &mut String,
    font_size: &mut f32,
    opacity: &mut f32,
    blur_effect: &mut crate::services::blur::BlurEffect,
    updater: &UpdateManager,
    deepseek_api_key_enc: &mut String,
    deepseek_model: &mut String,
    lunaline_config: &mut crate::lunaline::LunaLineConfig,
    now: f64,
    on_save: &mut dyn FnMut(&str, &str),
) -> Option<SettingPanelAction> {
    if !modal.settings_open {
        return None;
    }

    // 1. Semi-transparent backdrop overlay
    let backdrop_alpha = if theme.is_light() { 90 } else { 160 };
    painter.rect_filled(bounds, 0.0, Color32::from_black_alpha(backdrop_alpha));

    // 2. Centered modal card sizing
    let modal_w = (bounds.width() - 80.0).clamp(640.0, 880.0);
    let modal_h = (bounds.height() - 80.0).clamp(480.0, 680.0);
    let modal_rect = Rect::from_center_size(bounds.center(), eframe::egui::vec2(modal_w, modal_h));

    // Surface container
    painter.rect(
        modal_rect,
        5.0,
        theme.surface(),
        Stroke::new(1.0_f32, theme.border()),
        egui::StrokeKind::Inside,
    );

    // 3. Tab strip on the left, feature panel on the right
    let tab_w = 180.0;
    let tabs_rect = Rect::from_min_max(
        modal_rect.min,
        pos2(modal_rect.min.x + tab_w, modal_rect.max.y),
    );
    let panel_rect = Rect::from_min_max(
        pos2(modal_rect.min.x + tab_w, modal_rect.min.y),
        modal_rect.max,
    );

    render_setting_tabs(ui, painter, tabs_rect, &mut modal.active_setting_tab, theme);

    let panel_action = render_setting_panel(
        ui,
        painter,
        panel_rect,
        modal.active_setting_tab,
        caret,
        sound,
        theme,
        &mut modal.backup_dir,
        modal.last_backup_status.as_deref(),
        updater,
        deepseek_api_key_enc,
        deepseek_model,
        &mut modal.keymap,
        &mut modal.keybind_capture,
        selected_font,
        font_size,
        opacity,
        blur_effect,
        lunaline_config,
        on_save,
    );

    // 4. Modal closure on Escape key or clicking outside
    let escape = ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
    let outside_click = !modal.settings_just_opened
        && (now - modal.settings_opened_at) > 0.35
        && ui.input(|i| i.pointer.primary_clicked())
        && !ui.rect_contains_pointer(modal_rect);
    modal.settings_just_opened = false;

    if escape {
        if modal.keybind_capture.is_some() {
            modal.keybind_capture = None;
        } else {
            modal.settings_open = false;
        }
    } else if outside_click {
        modal.settings_open = false;
    }

    panel_action
}
