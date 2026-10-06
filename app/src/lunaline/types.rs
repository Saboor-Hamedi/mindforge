//! LunaLine type definitions, visual styles, color modes, and user configuration.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LunaStyle {
    /// Modern rounded pill capsules with discrete segment backgrounds (Zed / Linear / modern macOS)
    Pill,
    /// Angled powerline chevrons connecting segments (Classic Neovim / tmux lualine)
    Powerline,
    /// Detached floating island pill with subtle glassmorphic glow
    Floating,
    /// Clean borderless typographic style with subtle dot separators
    Minimal,
}

impl LunaStyle {
    pub const ALL: [LunaStyle; 4] = [
        LunaStyle::Pill,
        LunaStyle::Powerline,
        LunaStyle::Floating,
        LunaStyle::Minimal,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            LunaStyle::Pill => "Pill",
            LunaStyle::Powerline => "Powerline",
            LunaStyle::Floating => "Floating",
            LunaStyle::Minimal => "Minimal",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            LunaStyle::Pill => "Modern rounded capsules with subtle pill backgrounds",
            LunaStyle::Powerline => "Classic Neovim / tmux angled chevron arrows",
            LunaStyle::Floating => "Detached glassmorphic floating island",
            LunaStyle::Minimal => "Pure clean typography with subtle dot separators",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LunaColorMode {
    /// Dynamic mode-reactive accents (Normal = Green, Insert = Amber, Visual = Violet, Command = Sky Blue)
    Dynamic,
    /// Always matches current application theme accent
    ThemeAccent,
    /// Sleek stealth monochrome grayscale
    Monochrome,
}

impl LunaColorMode {
    pub const ALL: [LunaColorMode; 3] = [
        LunaColorMode::Dynamic,
        LunaColorMode::ThemeAccent,
        LunaColorMode::Monochrome,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            LunaColorMode::Dynamic => "Dynamic",
            LunaColorMode::ThemeAccent => "Theme Accent",
            LunaColorMode::Monochrome => "Monochrome",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            LunaColorMode::Dynamic => {
                "Vibrant mode-responsive colors (Normal, Insert, Visual, Cmd)"
            }
            LunaColorMode::ThemeAccent => {
                "Harmonizes directly with your active Lumina theme accent"
            }
            LunaColorMode::Monochrome => "Minimalist stealth grayscale tones with maximum clarity",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LunaLineConfig {
    pub enabled: bool,
    pub style: LunaStyle,
    pub color_mode: LunaColorMode,
    pub show_mode: bool,
    pub show_file_info: bool,
    pub show_word_count: bool,
    pub show_char_count: bool,
    pub show_reading_time: bool,
    pub show_cursor_pos: bool,
    pub show_progress: bool,
    pub show_encoding: bool,
    pub show_ai_button: bool,
    pub show_line_ending: bool,
}

impl Default for LunaLineConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            style: LunaStyle::Pill,
            color_mode: LunaColorMode::Dynamic,
            show_mode: true,
            show_file_info: true,
            show_word_count: true,
            show_char_count: false,
            show_reading_time: true,
            show_cursor_pos: true,
            show_progress: true,
            show_encoding: true,
            show_ai_button: true,
            show_line_ending: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lunaline_defaults_and_serde_roundtrip() {
        let default_cfg = LunaLineConfig::default();
        assert!(default_cfg.enabled);
        assert_eq!(default_cfg.style, LunaStyle::Pill);
        assert_eq!(default_cfg.color_mode, LunaColorMode::Dynamic);
        assert!(default_cfg.show_mode);
        assert!(default_cfg.show_file_info);
        assert!(default_cfg.show_word_count);
        assert!(!default_cfg.show_char_count);
        assert!(default_cfg.show_reading_time);
        assert!(default_cfg.show_cursor_pos);
        assert!(default_cfg.show_progress);
        assert!(default_cfg.show_encoding);
        assert!(default_cfg.show_ai_button);

        let json = serde_json::to_string(&default_cfg).expect("serialize default LunaLineConfig");
        let deserialized: LunaLineConfig =
            serde_json::from_str(&json).expect("deserialize LunaLineConfig");
        assert_eq!(default_cfg, deserialized);
    }

    #[test]
    fn test_lunaline_style_variants() {
        for style in LunaStyle::ALL {
            assert!(!style.name().is_empty());
            assert!(!style.description().is_empty());
        }
    }

    #[test]
    fn test_lunaline_color_modes() {
        for mode in LunaColorMode::ALL {
            assert!(!mode.name().is_empty());
            assert!(!mode.description().is_empty());
        }
    }
}
