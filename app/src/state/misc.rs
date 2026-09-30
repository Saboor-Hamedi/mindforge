//! Miscellaneous UI state for MINDFORGE's global interface.
//!
//! Groups fields that don't fit into other categories:
//! - Welcome screen visibility
//! - Window opacity and blur effect
//! - Status message bar
//! - Zen mode and surface visibility toggles
//! - Accent color overrides
//! - Clipboard text cache
//! - Zoom state
//! - ShowCmd HUD state

use crate::caret::Caret;
use crate::mode::Mode;
use crate::services::blur::BlurEffect;
use crate::services::sound::SoundEngine;
use crate::ui::showcmd::ShowCmdState;
use crate::ui::theme::Theme;
use crate::ui::zoom::ZoomState;

/// Miscellaneous UI state that doesn't fit into other sub-structs.
pub struct MiscState {
    /// Whether the welcome dashboard is shown (no notes open)
    pub show_welcome: bool,
    /// Window opacity level (0.0–1.0)
    pub opacity: f32,
    /// Window blur effect (Acrylic, Mica, or None)
    pub blur_effect: BlurEffect,
    /// Current status message displayed in the status bar
    pub status_msg: String,
    /// Timestamp when the status message was set (for auto-hide)
    pub status_time: f64,
    /// Whether this is the first frame (for initialization)
    pub first_frame: bool,
    /// Timestamp of the last keystroke (for activity tracking)
    pub last_char_time: f64,
    /// Whether zen (distraction-free) mode is active
    pub zen_mode: bool,
    /// Whether the window titlebar is visible
    pub show_titlebar: bool,
    /// Whether the document tab strip is visible
    pub show_tabs: bool,
    /// Accent color overrides (user-customizable)
    pub accent_overrides: crate::accent::AccentOverrides,
    /// Whether the accent color dropdown is open
    pub accent_dropdown_open: bool,
    /// Cached clipboard text for paste operations
    pub clipboard_text: Option<String>,
    /// Editor zoom state (level + HUD)
    pub zoom: ZoomState,
    /// ShowCmd keystroke HUD state
    pub showcmd: ShowCmdState,
    /// Active caret style and animation state
    pub caret: Caret,
    /// Active color theme
    pub theme: Theme,
    /// Active sound engine for typing audio
    pub sound: SoundEngine,
    /// Current editor mode (Normal, Doc, Terminal, ScanReport, etc.)
    pub mode: Mode,
    /// Selected font name
    pub selected_font: String,
    /// Current font size in points
    pub font_size: f32,
}

impl Default for MiscState {
    fn default() -> Self {
        Self {
            show_welcome: false,
            opacity: 1.0,
            blur_effect: BlurEffect::Acrylic,
            status_msg: String::new(),
            status_time: 0.0,
            first_frame: true,
            last_char_time: -10.0,
            zen_mode: false,
            show_titlebar: true,
            show_tabs: true,
            accent_overrides: crate::accent::AccentOverrides::default(),
            accent_dropdown_open: false,
            clipboard_text: None,
            zoom: ZoomState::new(),
            showcmd: ShowCmdState::new(true),
            caret: Caret::new(crate::caret::CaretKind::Beam),
            theme: Theme::from_kind(crate::ui::theme::ThemeKind::Gruvbox),
            sound: SoundEngine::new(crate::services::sound::SoundProfile::Thocky),
            mode: Mode::Normal,
            selected_font: "JetBrains Mono".to_string(),
            font_size: 16.0,
        }
    }
}

impl MiscState {
    /// Creates a new misc state with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets a status message with the current timestamp.
    pub fn set_status(&mut self, msg: &str, now: f64) {
        self.status_msg = msg.to_string();
        self.status_time = now;
    }

    /// Toggles zen mode.
    pub fn toggle_zen(&mut self) {
        self.zen_mode = !self.zen_mode;
    }

    /// Toggles the titlebar visibility.
    pub fn toggle_titlebar(&mut self) {
        self.show_titlebar = !self.show_titlebar;
    }

    /// Toggles the tab strip visibility.
    pub fn toggle_tabs(&mut self) {
        self.show_tabs = !self.show_tabs;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_misc_toggles() {
        let mut misc = MiscState::new();
        assert!(!misc.zen_mode);
        misc.toggle_zen();
        assert!(misc.zen_mode);
        assert!(misc.show_titlebar);
        misc.toggle_titlebar();
        assert!(!misc.show_titlebar);
    }

    #[test]
    fn test_misc_status() {
        let mut misc = MiscState::new();
        misc.set_status("Hello", 123.0);
        assert_eq!(misc.status_msg, "Hello");
        assert_eq!(misc.status_time, 123.0);
    }
}
