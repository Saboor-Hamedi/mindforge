//! Sub-picker lists and router for Command Palette.
//!
//! Handles the `>theme`, `>sound`, `>caret`, `>font`, `>mode`, `>luna`
//! sub-picker modes. Each sub-picker generates a list of `SearchItem`s
//! with fuzzy matching, active-state badges, and preview-on-Enter support.
//!
//! The `match_subpicker` function is the main entry point — it routes
//! the query to the appropriate sub-picker based on the prefix.

use crate::app::EditorInputMode;
use crate::caret::CaretKind;
use crate::services::font_manager::SUPPORTED_FONTS;
use crate::services::fuzzy::{fuzzy_match, PaletteAction, SearchItem};
use crate::lunaline::LunaStyle;
use crate::services::sound::SoundProfile;
use crate::ui::theme::ThemeKind;

/// Formats caret kind title display name.
pub fn caret_display_name(kind: CaretKind) -> &'static str {
    match kind {
        CaretKind::Block => "Block Caret",
        CaretKind::Beam => "Beam Caret",
        CaretKind::Underline => "Underline Caret",
        CaretKind::Candle => "Candle Flame Caret",
        CaretKind::Fire => "Fire & Micro-Embers",
        CaretKind::Water => "Water Ripple Beam",
        CaretKind::Snow => "Falling Snowflakes",
        CaretKind::Neon => "Neon Laser Aura",
        CaretKind::Rainbow => "Rainbow Spectrum",
        CaretKind::Electric => "Electric Spark",
        CaretKind::Comet => "Comet Trail",
        CaretKind::Matrix => "Matrix Glyph Stream",
        CaretKind::Ice => "Ice Crystals",
        CaretKind::Glitch => "Cyberpunk Glitch",
        CaretKind::Heartbeat => "Heartbeat Pulse",
    }
}

/// Generates search items for theme sub-picker (`>theme [query]`).
pub fn theme_picker_items(needle: &str, active_theme: ThemeKind) -> Vec<SearchItem> {
    let mut results = Vec::new();
    for &theme_kind in ThemeKind::ALL {
        let name = theme_kind.display_name();
        let key_name = theme_kind.name();
        let score = if needle.is_empty() {
            Some(100)
        } else {
            fuzzy_match(needle, name).or_else(|| fuzzy_match(needle, key_name))
        };

        if let Some(mut s) = score {
            let is_active = theme_kind == active_theme;
            if is_active {
                s += 200;
            }
            let snippet = if theme_kind.is_light() {
                "Crisp Paper • Light".to_string()
            } else {
                "Midnight Canvas • Dark".to_string()
            };
            let badge = if is_active {
                "✓ Active".to_string()
            } else {
                String::new()
            };
            results.push(SearchItem {
                id: 0,
                title: name.to_string(),
                snippet,
                score: s,
                badge,
                icon: "🎨",
                action: PaletteAction::ApplyTheme(theme_kind),
            });
        }
    }
    results.sort_by(|a, b| b.score.cmp(&a.score));
    results
}

/// Generates search items for sound sub-picker (`>sound [query]`).
pub fn sound_picker_items(needle: &str, active_sound: SoundProfile) -> Vec<SearchItem> {
    let mut results = Vec::new();
    for &profile in SoundProfile::ALL.iter() {
        let name = profile.name();
        let desc = profile.description();
        let score = if needle.is_empty() {
            Some(100)
        } else {
            fuzzy_match(needle, name).or_else(|| fuzzy_match(needle, desc))
        };

        if let Some(mut s) = score {
            let is_active = profile == active_sound;
            if is_active {
                s += 200;
            }
            let badge = if is_active {
                "✓ Active".to_string()
            } else {
                String::new()
            };
            let icon = if profile == SoundProfile::Off { "🔇" } else { "🔊" };
            results.push(SearchItem {
                id: 0,
                title: name.to_string(),
                snippet: desc.to_string(),
                score: s,
                badge,
                icon,
                action: PaletteAction::ApplySoundProfile(profile),
            });
        }
    }
    results.sort_by(|a, b| b.score.cmp(&a.score));
    results
}

/// Generates search items for caret sub-picker (`>caret [query]`).
pub fn caret_picker_items(needle: &str, active_caret: CaretKind) -> Vec<SearchItem> {
    let mut results = Vec::new();
    for &kind in CaretKind::ALL {
        let name = caret_display_name(kind);
        let desc = kind.description();
        let score = if needle.is_empty() {
            Some(100)
        } else {
            fuzzy_match(needle, name).or_else(|| fuzzy_match(needle, desc))
        };

        if let Some(mut s) = score {
            let is_active = kind == active_caret;
            if is_active {
                s += 200;
            }
            let badge = if is_active {
                "✓ Active".to_string()
            } else {
                String::new()
            };
            results.push(SearchItem {
                id: 0,
                title: name.to_string(),
                snippet: desc.to_string(),
                score: s,
                badge,
                icon: "✦",
                action: PaletteAction::ApplyCaretKind(kind),
            });
        }
    }
    results.sort_by(|a, b| b.score.cmp(&a.score));
    results
}

/// Generates search items for font sub-picker (`>font [query]`).
pub fn font_picker_items(needle: &str, active_font: &str) -> Vec<SearchItem> {
    let mut results = Vec::new();
    for font in SUPPORTED_FONTS.iter() {
        let name = font.display_name;
        let desc = font.description;
        let score = if needle.is_empty() {
            Some(100)
        } else {
            fuzzy_match(needle, name).or_else(|| fuzzy_match(needle, desc))
        };

        if let Some(mut s) = score {
            let is_active = active_font.eq_ignore_ascii_case(name)
                || (active_font.is_empty() && font.id == "jetbrains_mono");
            if is_active {
                s += 200;
            }
            let badge = if is_active {
                "✓ Active".to_string()
            } else {
                String::new()
            };
            results.push(SearchItem {
                id: 0,
                title: name.to_string(),
                snippet: desc.to_string(),
                score: s,
                badge,
                icon: "🔤",
                action: PaletteAction::ApplyFont(name.to_string()),
            });
        }
    }
    results.sort_by(|a, b| b.score.cmp(&a.score));
    results
}

/// Generates search items for editor mode sub-picker (`>mode [query]`).
pub fn mode_picker_items(needle: &str, active_mode: EditorInputMode) -> Vec<SearchItem> {
    let modes = [
        (
            EditorInputMode::Vim,
            "Vim Modal Motions",
            "Command, Insert, Visual, VisualLine • hjkl, text objects, operators",
            "⚡",
        ),
        (
            EditorInputMode::Hybrid,
            "Hybrid Cursor Mode",
            "Standard cursor & selection • Ctrl+D duplicate, instant writing",
            "✍",
        ),
    ];

    let mut results = Vec::new();
    for (mode, name, desc, icon) in modes {
        let score = if needle.is_empty() {
            Some(100)
        } else {
            fuzzy_match(needle, name).or_else(|| fuzzy_match(needle, desc))
        };

        if let Some(mut s) = score {
            let is_active = mode == active_mode;
            if is_active {
                s += 200;
            }
            let badge = if is_active {
                "✓ Active".to_string()
            } else {
                String::new()
            };
            results.push(SearchItem {
                id: 0,
                title: name.to_string(),
                snippet: desc.to_string(),
                score: s,
                badge,
                icon,
                action: PaletteAction::ApplyEditorMode(mode),
            });
        }
    }
    results.sort_by(|a, b| b.score.cmp(&a.score));
    results
}

/// Generates search items for LunaLine statusline sub-picker (`>luna [query]`).
pub fn luna_picker_items(needle: &str, active_style: LunaStyle) -> Vec<SearchItem> {
    let styles = [
        (LunaStyle::Pill, "Pill Capsules", "Discrete rounded capsules with subtle surface background"),
        (LunaStyle::Powerline, "Neovim Powerline", "Classic angled arrow chevrons connecting segments"),
        (LunaStyle::Floating, "Floating Island Pill", "Detached glassmorphic statusline floating above canvas"),
        (LunaStyle::Minimal, "Minimal Clean Typography", "Pure typographic statusline with subtle dot separators"),
    ];

    let mut results = Vec::new();
    for (style, name, desc) in styles {
        let score = if needle.is_empty() {
            Some(100)
        } else {
            fuzzy_match(needle, name).or_else(|| fuzzy_match(needle, desc))
        };

        if let Some(mut s) = score {
            let is_active = style == active_style;
            if is_active {
                s += 200;
            }
            let badge = if is_active {
                "✓ Active".to_string()
            } else {
                String::new()
            };
            results.push(SearchItem {
                id: 0,
                title: format!("LunaLine: {}", name),
                snippet: desc.to_string(),
                score: s,
                badge,
                icon: "🎨",
                action: PaletteAction::SetLunaStyle(style),
            });
        }
    }
    results.sort_by(|a, b| b.score.cmp(&a.score));
    results
}

/// Routes sub-picker commands like `>theme`, `>sound`, `>caret`, `>font`, `>mode`, `>luna`.
/// Returns `Some(items)` if handled by a sub-picker, or `None` if general `>...` command mode.
pub fn match_subpicker(
    raw: &str,
    active_theme: ThemeKind,
    active_sound: SoundProfile,
    active_caret: CaretKind,
    active_font: &str,
    active_mode: EditorInputMode,
    active_luna_style: LunaStyle,
) -> Option<Vec<SearchItem>> {
    if raw.starts_with(">theme") || raw.starts_with("> theme") {
        let needle = raw
            .strip_prefix(">theme")
            .or_else(|| raw.strip_prefix("> theme"))
            .unwrap_or("")
            .trim();
        return Some(theme_picker_items(needle, active_theme));
    }

    if raw.starts_with(">sound") || raw.starts_with("> sound") {
        let needle = raw
            .strip_prefix(">sound")
            .or_else(|| raw.strip_prefix("> sound"))
            .unwrap_or("")
            .trim();
        return Some(sound_picker_items(needle, active_sound));
    }

    if raw.starts_with(">caret") || raw.starts_with("> caret") {
        let needle = raw
            .strip_prefix(">caret")
            .or_else(|| raw.strip_prefix("> caret"))
            .unwrap_or("")
            .trim();
        return Some(caret_picker_items(needle, active_caret));
    }

    if raw.starts_with(">font") || raw.starts_with("> font") {
        let needle = raw
            .strip_prefix(">font")
            .or_else(|| raw.strip_prefix("> font"))
            .unwrap_or("")
            .trim();
        return Some(font_picker_items(needle, active_font));
    }

    if raw.starts_with(">mode") || raw.starts_with("> mode") {
        let needle = raw
            .strip_prefix(">mode")
            .or_else(|| raw.strip_prefix("> mode"))
            .unwrap_or("")
            .trim();
        return Some(mode_picker_items(needle, active_mode));
    }

    if raw.starts_with(">luna") || raw.starts_with("> luna") {
        let needle = raw
            .strip_prefix(">luna")
            .or_else(|| raw.strip_prefix("> luna"))
            .unwrap_or("")
            .trim();
        return Some(luna_picker_items(needle, active_luna_style));
    }

    None
}
