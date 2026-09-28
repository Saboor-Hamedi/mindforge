//! Settings modal panels and tabs router.

pub mod ai_engine;
pub mod backup;
pub mod carets;
pub mod editor_mode;
pub mod keybindings_tab;
pub mod keymap;
pub mod types;
pub mod setting_font;
pub mod shortcuts;
pub mod sounds;
pub mod tabs;
pub mod theme;
pub mod updates;
pub mod lunaline_tab;

pub use tabs::{render_setting_tabs, SettingTab};

use crate::app::EditorInputMode;
use crate::caret::Caret;
use crate::sound::SoundEngine;
use crate::theme::Theme;
use crate::updater::UpdateManager;
use eframe::egui::{self, vec2, Rect};

pub enum SettingPanelAction {
    TriggerBackup,
    CheckUpdates,
    DownloadUpdate,
    RestartToApply,
}

pub fn render_setting_panel(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    panel_rect: Rect,
    active_tab: SettingTab,
    editor_input_mode: &mut EditorInputMode,
    caret: &mut Caret,
    sound: &mut SoundEngine,
    theme: &mut Theme,
    backup_dir: &mut String,
    last_backup_status: Option<&str>,
    updater: &UpdateManager,
    api_key_enc: &mut String,
    deepseek_model: &mut String,
    keymap: &mut crate::settings::keymap::VimKeymap,
    keybind_capture: &mut Option<crate::settings::keymap::KeybindCapture>,
    selected_font: &mut String,
    font_size: &mut f32,
    opacity: &mut f32,
    blur_effect: &mut crate::blur::BlurEffect,
    lunaline_config: &mut crate::lunaline::LunaLineConfig,
    on_save_setting: &mut dyn FnMut(&str, &str),
) -> Option<SettingPanelAction> {
    // Clip all painting strictly to the panel rect — nothing bleeds over modal border
    let painter = painter.with_clip_rect(panel_rect.shrink(1.0));
    let painter = &painter;
    let p_origin = panel_rect.min + vec2(28.0, 24.0);

    match active_tab {
        SettingTab::Carets => {
            carets::render_carets_tab(
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
            setting_font::render_font_settings(
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
        SettingTab::EditorMode => {
            editor_mode::render_editor_mode_tab(ui, painter, panel_rect, p_origin, editor_input_mode, theme, on_save_setting);
            None
        }
        SettingTab::Sounds => {
            sounds::render_sounds_tab(ui, painter, panel_rect, p_origin, sound, theme, on_save_setting);
            None
        }
        SettingTab::Theme => {
            theme::render_theme_tab(ui, painter, panel_rect, p_origin, theme, opacity, blur_effect, on_save_setting);
            None
        }
        SettingTab::Shortcuts => {
            shortcuts::render_shortcuts_tab(ui, painter, panel_rect, p_origin, theme);
            None
        }
        SettingTab::Keybindings => {
            keybindings_tab::render_keybindings_tab(ui, painter, panel_rect, p_origin, theme, keymap, keybind_capture);
            None
        }
        SettingTab::Backup => {
            backup::render_backup_tab(ui, painter, panel_rect, p_origin, backup_dir, last_backup_status, theme, on_save_setting)
        }
        SettingTab::Updates => {
            updates::render_updates_tab(ui, painter, panel_rect, p_origin, updater, theme)
        }
        SettingTab::Ai => {
            ai_engine::render_ai_tab(ui, painter, panel_rect, p_origin, api_key_enc, deepseek_model, theme, on_save_setting);
            None
        }
        SettingTab::LunaLine => {
            lunaline_tab::render_lunaline_tab(ui, painter, panel_rect, p_origin, lunaline_config, theme, *opacity, on_save_setting);
            None
        }
    }
}
