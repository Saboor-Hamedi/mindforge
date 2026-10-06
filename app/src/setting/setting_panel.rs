//! Settings panel feature router.
//!
//! Renders the active setting tab's feature panel (appearance, sound, editor mode,
//! keybindings, shortcuts, AI configuration, backups, updates, and status line).

use super::setting_tab::SettingTab;
use crate::caret::Caret;
use crate::services::sound::SoundEngine;
use crate::services::updater::UpdateManager;
use crate::ui::theme::Theme;
use eframe::egui::{self, vec2, Rect};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingPanelAction {
    TriggerBackup,
    CheckUpdates,
    DownloadUpdate,
    RestartToApply,
}

/// Renders the main content panel where all settings features and configuration appear.
pub fn render_setting_panel(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    panel_rect: Rect,
    active_tab: SettingTab,
    caret: &mut Caret,
    sound: &mut SoundEngine,
    theme: &mut Theme,
    backup_dir: &mut String,
    last_backup_status: Option<&str>,
    updater: &UpdateManager,
    api_key_enc: &mut String,
    deepseek_model: &mut String,
    keymap: &mut super::keymap::VimKeymap,
    keybind_capture: &mut Option<super::keymap::KeybindCapture>,
    selected_font: &mut String,
    font_size: &mut f32,
    opacity: &mut f32,
    blur_effect: &mut crate::services::blur::BlurEffect,
    lunaline_config: &mut crate::lunaline::LunaLineConfig,
    on_save_setting: &mut dyn FnMut(&str, &str),
) -> Option<SettingPanelAction> {
    // Clip all painting strictly to the panel rect — nothing bleeds over modal border
    let painter = painter.with_clip_rect(panel_rect.shrink(1.0));
    let painter = &painter;
    let p_origin = panel_rect.min + vec2(28.0, 24.0);

    match active_tab {
        SettingTab::Carets => {
            super::carets::render_carets_tab(
                ui,
                painter,
                panel_rect,
                p_origin,
                caret,
                theme,
                on_save_setting,
            );
            None
        }
        SettingTab::Fonts => {
            super::setting_font::render_font_settings(
                ui,
                painter,
                panel_rect,
                p_origin,
                selected_font,
                font_size,
                theme,
                on_save_setting,
            );
            None
        }
        SettingTab::Sounds => {
            super::sound::render_sounds_tab(
                ui,
                painter,
                panel_rect,
                p_origin,
                sound,
                theme,
                on_save_setting,
            );
            None
        }
        SettingTab::Theme => {
            super::theme::render_theme_tab(
                ui,
                painter,
                panel_rect,
                p_origin,
                theme,
                opacity,
                blur_effect,
                on_save_setting,
            );
            None
        }
        SettingTab::Shortcuts => {
            super::shortcuts::render_shortcuts_tab(ui, painter, panel_rect, p_origin, theme);
            None
        }
        SettingTab::Keybindings => {
            super::keybindings_tab::render_keybindings_tab(
                ui,
                painter,
                panel_rect,
                p_origin,
                theme,
                keymap,
                keybind_capture,
            );
            None
        }
        SettingTab::Backup => super::backup::render_backup_tab(
            ui,
            painter,
            panel_rect,
            p_origin,
            backup_dir,
            last_backup_status,
            theme,
            on_save_setting,
        ),
        SettingTab::Updates => {
            super::updates::render_updates_tab(ui, painter, panel_rect, p_origin, updater, theme)
        }
        SettingTab::Ai => {
            super::ai_engine::render_ai_tab(
                ui,
                painter,
                panel_rect,
                p_origin,
                api_key_enc,
                deepseek_model,
                theme,
                on_save_setting,
            );
            None
        }
        SettingTab::LunaLine => {
            super::lunaline_tab::render_lunaline_tab(
                ui,
                painter,
                panel_rect,
                p_origin,
                lunaline_config,
                theme,
                *opacity,
                on_save_setting,
            );
            None
        }
    }
}
