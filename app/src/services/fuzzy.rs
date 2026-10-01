//! Fuzzy search utilities and Command Palette actions for MindForge.
//!
//! Provides the unified search and command routing system:
//! - `fuzzy_match` — Subsequence-based fuzzy string matching with scoring
//! - `search_palette` — Routes queries to sub-pickers or command palette
//! - `BUILTIN_COMMANDS` — Static catalog of all available commands
//! - `PaletteAction` — Enum of all actions the palette can trigger
//!
//! Scoring rewards: consecutive characters, word-boundary matches,
//! exact matches, and shorter haystacks (more relevant).

use crate::ui::theme::ThemeKind;
use crate::services::sound::SoundProfile;

/// All actions that can be triggered from the Command Palette or fuzzy search.
/// Each variant carries the data needed to execute the action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaletteAction {
    OpenNote(i64),
    OpenThemePicker,
    ApplyTheme(ThemeKind),
    ShowSoundPicker,
    ApplySoundProfile(SoundProfile),
    OpenCaretPicker,
    ApplyCaretKind(crate::caret::CaretKind),
    OpenFontPicker,
    ApplyFont(String),
    OpenModePicker,
    ApplyEditorMode(crate::app::EditorInputMode),
    OpenSetting(crate::setting::SettingTab),
    ToggleSidebar,
    ToggleRightSidebar,
    ToggleBacklinks,
    ToggleOutline,
    TogglePreview,
    ToggleAi,
    ToggleTerminal,
    ToggleZen,
    ToggleTitlebar,
    ToggleTabs,
    ToggleChecklist,
    CloseTab,
    NewNote,
    QuickSave,
    RenameNote,
    DeleteNote,
    ImportWorkspace,
    RunScan,
    ScanHistory,
    OpenHelp,
    SetLunaStyle(crate::lunaline::LunaStyle),
    SetLunaColor(crate::lunaline::LunaColorMode),
}

/// A single search result item displayed in the palette or search modal.
#[derive(Debug, Clone)]
pub struct SearchItem {
    /// Unique identifier (note ID, or 0 for built-in commands)
    pub id: i64,
    /// Display title shown as the primary line
    pub title: String,
    /// Secondary description text
    pub snippet: String,
    /// Fuzzy match score (higher = more relevant)
    pub score: i64,
    /// Short badge text (e.g. ">theme", "Ctrl+N", "Active")
    pub badge: String,
    /// Emoji or symbol icon displayed to the left
    pub icon: &'static str,
    /// Action to execute when this item is selected
    pub action: PaletteAction,
}

impl SearchItem {
    pub fn for_note(id: i64, title: String, snippet: String, score: i64) -> Self {
        Self {
            id,
            title,
            snippet,
            score,
            badge: String::new(),
            icon: "📄",
            action: PaletteAction::OpenNote(id),
        }
    }
}

pub struct BuiltinCommand {
    pub title: &'static str,
    pub snippet: &'static str,
    pub badge: &'static str,
    pub icon: &'static str,
    pub action: PaletteAction,
}

pub const BUILTIN_COMMANDS: &[BuiltinCommand] = &[
    // --- Theme, Audio & Caret Pickers ---
    BuiltinCommand {
        title: "Color Theme",
        snippet: "Browse and live-preview all 19 visual themes",
        badge: ">theme",
        icon: "🎨",
        action: PaletteAction::OpenThemePicker,
    },
    BuiltinCommand {
        title: "Typing Sound Profile",
        snippet: "Preview and switch mechanical keyboard sound profiles live",
        badge: ">sound",
        icon: "🔊",
        action: PaletteAction::ShowSoundPicker,
    },
    BuiltinCommand {
        title: "Caret Style & Cursor FX",
        snippet: "Preview and switch living animated cursor styles & physics live",
        badge: ">caret",
        icon: "✦",
        action: PaletteAction::OpenCaretPicker,
    },
    BuiltinCommand {
        title: "Preferences & Appearance",
        snippet: "Customize active color palette, window opacity, and blur",
        badge: "Ctrl+,",
        icon: "⚙",
        action: PaletteAction::OpenSetting(crate::setting::SettingTab::Theme),
    },
    BuiltinCommand {
        title: "Keyboard Shortcuts & Cheatsheet",
        snippet: "Review global shortcuts, editing commands, and markdown combos",
        badge: "F1",
        icon: "⚙",
        action: PaletteAction::OpenSetting(crate::setting::SettingTab::Shortcuts),
    },
    BuiltinCommand {
        title: "Editor Mode (Vim / Hybrid)",
        snippet: "Switch between modal Vim motions and intuitive Hybrid writing",
        badge: ">mode",
        icon: "⚙",
        action: PaletteAction::OpenModePicker,
    },
    BuiltinCommand {
        title: "Window Opacity & Transparency",
        snippet: "Adjust window opacity level from solid to translucent",
        badge: "Opacity",
        icon: "⚙",
        action: PaletteAction::OpenSetting(crate::setting::SettingTab::Theme),
    },
    BuiltinCommand {
        title: "Window Blur Effect (Acrylic / Mica / Off)",
        snippet: "Configure Windows desktop acrylic or mica glass blur",
        badge: "Blur",
        icon: "⚙",
        action: PaletteAction::OpenSetting(crate::setting::SettingTab::Theme),
    },

    // --- Carets & Typography Settings ---
    BuiltinCommand {
        title: "Carets & Cursor Styles",
        snippet: "Customize cursor animation, kind (Beam, Block, Neon), and width",
        badge: "Caret",
        icon: "⚙",
        action: PaletteAction::OpenSetting(crate::setting::SettingTab::Carets),
    },
    BuiltinCommand {
        title: "Fonts & Monospace Typography",
        snippet: "Select custom font family, ligature rendering, and font size",
        badge: ">font",
        icon: "⚙",
        action: PaletteAction::OpenFontPicker,
    },

    // --- Audio, Keybindings & System Settings ---
    BuiltinCommand {
        title: "Mechanical Typing Audio & Switches",
        snippet: "Switch mechanical switch audio profiles (Thocky, Clacky, Silent)",
        badge: "Audio",
        icon: "⚙",
        action: PaletteAction::OpenSetting(crate::setting::SettingTab::Sounds),
    },
    BuiltinCommand {
        title: "Custom Keybindings & Remapping",
        snippet: "Configure custom keybindings and inspect motion keymaps",
        badge: "Keymap",
        icon: "⚙",
        action: PaletteAction::OpenSetting(crate::setting::SettingTab::Keybindings),
    },
    BuiltinCommand {
        title: "Vault Backup & Data Safety",
        snippet: "Configure automated SQLite snapshots and export paths",
        badge: "Backup",
        icon: "⚙",
        action: PaletteAction::OpenSetting(crate::setting::SettingTab::Backup),
    },
    BuiltinCommand {
        title: "Check for App Updates",
        snippet: "Verify GitHub release packages and apply live updates",
        badge: "Updates",
        icon: "⚙",
        action: PaletteAction::OpenSetting(crate::setting::SettingTab::Updates),
    },
    BuiltinCommand {
        title: "AI Engine & DeepSeek",
        snippet: "Configure DeepSeek API key and model parameters",
        badge: "Ctrl+Shift+I",
        icon: "⚙",
        action: PaletteAction::OpenSetting(crate::setting::SettingTab::Ai),
    },

    // --- LunaLine Statusline Settings ---
    BuiltinCommand {
        title: "LunaLine Statusline",
        snippet: "Customize dock style (Pill, Powerline, Floating) & component toggles",
        badge: "LunaLine",
        icon: "⚙",
        action: PaletteAction::OpenSetting(crate::setting::SettingTab::LunaLine),
    },
    BuiltinCommand {
        title: "LunaLine Style - Modern Pill Capsules",
        snippet: "Discrete capsules with subtle rounded pill background",
        badge: "Pill",
        icon: "🎨",
        action: PaletteAction::SetLunaStyle(crate::lunaline::LunaStyle::Pill),
    },
    BuiltinCommand {
        title: "LunaLine Style - Neovim Powerline Chevrons",
        snippet: "Classic angled arrow chevrons connecting segments",
        badge: "Powerline",
        icon: "🎨",
        action: PaletteAction::SetLunaStyle(crate::lunaline::LunaStyle::Powerline),
    },
    BuiltinCommand {
        title: "LunaLine Style - Floating Island Pill",
        snippet: "Detached glassmorphic statusline floating above edge",
        badge: "Floating",
        icon: "🎨",
        action: PaletteAction::SetLunaStyle(crate::lunaline::LunaStyle::Floating),
    },
    BuiltinCommand {
        title: "LunaLine Style - Minimal Clean Typography",
        snippet: "Pure typographic statusline with subtle dot separators",
        badge: "Minimal",
        icon: "🎨",
        action: PaletteAction::SetLunaStyle(crate::lunaline::LunaStyle::Minimal),
    },
    BuiltinCommand {
        title: "LunaLine Color Mode - Dynamic Accents",
        snippet: "Mode-reactive colors (Normal, Insert, Visual, Command)",
        badge: "Dynamic",
        icon: "🎨",
        action: PaletteAction::SetLunaColor(crate::lunaline::LunaColorMode::Dynamic),
    },
    BuiltinCommand {
        title: "LunaLine Color Mode - Theme Accent",
        snippet: "Harmonizes directly with active Lumina theme accent",
        badge: "Accent",
        icon: "🎨",
        action: PaletteAction::SetLunaColor(crate::lunaline::LunaColorMode::ThemeAccent),
    },
    BuiltinCommand {
        title: "LunaLine Color Mode - Monochrome",
        snippet: "Stealth minimalist grayscale with maximum clarity",
        badge: "Mono",
        icon: "🎨",
        action: PaletteAction::SetLunaColor(crate::lunaline::LunaColorMode::Monochrome),
    },

    // --- View Settings & Navigation Shortcuts ---
    BuiltinCommand {
        title: "Toggle Sidebar Explorer",
        snippet: "Show or hide the document explorer drawer",
        badge: "Ctrl+B",
        icon: "👁",
        action: PaletteAction::ToggleSidebar,
    },
    BuiltinCommand {
        title: "Toggle Backlinks Sidebar",
        snippet: "Show or hide incoming backlinks and note reference panel",
        badge: "Ctrl+I",
        icon: "🔗",
        action: PaletteAction::ToggleBacklinks,
    },
    BuiltinCommand {
        title: "Toggle Outline Sidebar",
        snippet: "Show or hide the document outline (H1-H6) panel",
        badge: "Ctrl+Shift+O",
        icon: "📑",
        action: PaletteAction::ToggleOutline,
    },
    BuiltinCommand {
        title: "Toggle Markdown Live Preview",
        snippet: "Split or close side-by-side formatted preview",
        badge: "Ctrl+\\",
        icon: "👁",
        action: PaletteAction::TogglePreview,
    },
    BuiltinCommand {
        title: "Toggle Interactive Terminal",
        snippet: "Open bottom docked PowerShell / cmd shell session",
        badge: "Ctrl+J",
        icon: "👁",
        action: PaletteAction::ToggleTerminal,
    },
    BuiltinCommand {
        title: "Toggle Zen Focus Mode",
        snippet: "Distraction-free pure canvas writing environment",
        badge: "Ctrl+.",
        icon: "👁",
        action: PaletteAction::ToggleZen,
    },
    BuiltinCommand {
        title: "Toggle AI Writing Assistant",
        snippet: "Open DeepSeek Pro intelligent writing assistant",
        badge: "Ctrl+Shift+I",
        icon: "👁",
        action: PaletteAction::ToggleAi,
    },
    BuiltinCommand {
        title: "Toggle Window Titlebar",
        snippet: "Show or hide the top window titlebar",
        badge: "UI",
        icon: "👁",
        action: PaletteAction::ToggleTitlebar,
    },
    BuiltinCommand {
        title: "Toggle Document Tabs",
        snippet: "Show or hide editor document tab strip",
        badge: "UI",
        icon: "👁",
        action: PaletteAction::ToggleTabs,
    },

    // --- Document & Action Settings Shortcuts ---
    BuiltinCommand {
        title: "New Note Document",
        snippet: "Create a fresh empty markdown note",
        badge: "Ctrl+N",
        icon: "📄",
        action: PaletteAction::NewNote,
    },
    BuiltinCommand {
        title: "Quick Save Active Note",
        snippet: "Commit active note buffer immediately to SQLite",
        badge: "Ctrl+S",
        icon: "📄",
        action: PaletteAction::QuickSave,
    },
    BuiltinCommand {
        title: "Rename Active Note",
        snippet: "Change topic title of the active document",
        badge: "Ctrl+R",
        icon: "📄",
        action: PaletteAction::RenameNote,
    },
    BuiltinCommand {
        title: "Delete Active Note",
        snippet: "Permanently delete current note from local vault",
        badge: "Ctrl+Shift+D",
        icon: "📄",
        action: PaletteAction::DeleteNote,
    },
    BuiltinCommand {
        title: "Toggle Checklist Checkbox",
        snippet: "Toggle checklist checkbox between [ ] and [x]",
        badge: "Ctrl+Shift+X",
        icon: "📄",
        action: PaletteAction::ToggleChecklist,
    },
    BuiltinCommand {
        title: "Close Note Tab",
        snippet: "Close the currently active note or doc tab",
        badge: "Ctrl+W",
        icon: "📄",
        action: PaletteAction::CloseTab,
    },
    BuiltinCommand {
        title: "Settings & Preferences Modal",
        snippet: "Open full preferences dialog (theme, sounds, keys, AI, carets)",
        badge: "Ctrl+,",
        icon: "⚙",
        action: PaletteAction::OpenSetting(crate::setting::SettingTab::Theme),
    },
    BuiltinCommand {
        title: "Import Obsidian Vault or Folder",
        snippet: "Bulk-import local markdown files and vaults",
        badge: "Import",
        icon: "📄",
        action: PaletteAction::ImportWorkspace,
    },
    BuiltinCommand {
        title: "Run Web Security Scan",
        snippet: "Run automated security vulnerability scan against URL",
        badge: ":scan",
        icon: "⚡",
        action: PaletteAction::RunScan,
    },
    BuiltinCommand {
        title: "Security Scan History",
        snippet: "Review previous security vulnerability scan results",
        badge: ":scans",
        icon: "⚡",
        action: PaletteAction::ScanHistory,
    },
    BuiltinCommand {
        title: "Documentation & User Manual",
        snippet: "Browse built-in user guides, shortcuts, and tutorials",
        badge: "F1",
        icon: "📖",
        action: PaletteAction::OpenHelp,
    },
];

/// Computes a fuzzy match score between needle and haystack.
///
/// The needle must be a subsequence of the haystack (case-insensitive).
/// Scoring algorithm:
/// - +10 points per matched character
/// - +5 × consecutive streak for consecutive matches
/// - +15 bonus for matches at word boundaries (start, after space/`_`/`-`/`:`)
/// - +50 bonus for exact full-string match
/// - Penalty: up to -30 for longer haystacks (shorter = more relevant)
///
/// Returns `Some(score)` if all needle characters were matched, `None` otherwise.
pub fn fuzzy_match(needle: &str, haystack: &str) -> Option<i64> {
    if needle.is_empty() {
        return Some(0);
    }
    let needle_chars: Vec<char> = needle.to_lowercase().chars().collect();
    let haystack_chars: Vec<char> = haystack.to_lowercase().chars().collect();

    let mut n_idx = 0;
    let mut score = 0i64;
    let mut consecutive = 0i64;

    for (h_idx, &hc) in haystack_chars.iter().enumerate() {
        if hc == needle_chars[n_idx] {
            score += 10;
            if consecutive > 0 {
                score += consecutive * 5; // bonus for consecutive letters
            }
            if h_idx == 0 || haystack_chars[h_idx - 1].is_whitespace() || haystack_chars[h_idx - 1] == '_' || haystack_chars[h_idx - 1] == '-' || haystack_chars[h_idx - 1] == ':' {
                score += 15; // word boundary bonus
            }
            consecutive += 1;
            n_idx += 1;
            if n_idx == needle_chars.len() {
                if needle_chars.len() == haystack_chars.len() {
                    score += 50; // exact match bonus
                }
                score -= (haystack_chars.len() as i64).min(30);
                return Some(score);
            }
        } else {
            consecutive = 0;
        }
    }

    None
}

/// Unified Search & Command Palette router.
///
/// Handles three query modes:
/// 1. **Sub-picker mode** (`>theme`, `>sound`, `>caret`, `>font`, `>mode`, `>luna`)
///    → Delegates to `palette::match_subpicker` for live interactive selection
/// 2. **Command Palette mode** (`>[query]`)
///    → Fuzzy searches across `BUILTIN_COMMANDS` catalog
/// 3. **Note search mode** (`[query]`)
///    → Fuzzy searches across note titles and body content
///
/// Results are sorted by score (descending).
/// Extracts a clean, safe snippet around the matched query in the note body.
/// Uses Unicode character indices so multi-byte UTF-8 sequences (em-dashes, emojis, etc.)
/// are never split across byte boundaries.
pub fn extract_snippet(body: &str, query: &str) -> String {
    let clean = body.replace('\n', " ");
    let chars: Vec<char> = clean.chars().collect();
    let total_chars = chars.len();

    if query.is_empty() {
        if total_chars > 60 {
            let s: String = chars[..60].iter().collect();
            format!("{s}...")
        } else {
            clean
        }
    } else {
        let clean_lower: Vec<char> = clean.to_lowercase().chars().collect();
        let query_lower: Vec<char> = query.to_lowercase().chars().collect();

        let match_pos = if query_lower.is_empty() {
            None
        } else {
            clean_lower
                .windows(query_lower.len())
                .position(|window| window == query_lower.as_slice())
        };

        if let Some(pos) = match_pos {
            let start = pos.saturating_sub(20);
            let end = (pos + query_lower.len() + 40).min(total_chars);
            let s: String = chars[start..end].iter().collect();
            let prefix = if start > 0 { "..." } else { "" };
            let suffix = if end < total_chars { "..." } else { "" };
            format!("{prefix}{s}{suffix}")
        } else if total_chars > 60 {
            let s: String = chars[..60].iter().collect();
            format!("{s}...")
        } else {
            clean
        }
    }
}

pub fn search_palette(
    query_str: &str,
    notes: &[core::Note],
    active_theme: ThemeKind,
    active_sound: SoundProfile,
    active_caret: crate::caret::CaretKind,
    active_font: &str,
    active_mode: crate::app::EditorInputMode,
    active_luna_style: crate::lunaline::LunaStyle,
) -> Vec<SearchItem> {
    let raw = query_str.trim();

    // ── 1. Sub-Picker Modes (`>theme`, `>sound`, `>caret`, `>font`, `>mode`, `>luna`)
    if let Some(sub_items) = crate::ui::palette::match_subpicker(
        raw,
        active_theme,
        active_sound,
        active_caret,
        active_font,
        active_mode,
        active_luna_style,
    ) {
        return sub_items;
    }

    // ── 2. VS Code Command Palette Mode (`>...`) ─────────────────────────────
    if raw.starts_with('>') {
        let needle = raw.strip_prefix('>').unwrap_or("").trim();
        let mut results = Vec::new();

        for cmd in BUILTIN_COMMANDS {
            let score = if needle.is_empty() {
                Some(100)
            } else {
                let s_title = fuzzy_match(needle, cmd.title);
                let s_snip = fuzzy_match(needle, cmd.snippet);
                let s_badge = fuzzy_match(needle, cmd.badge);
                s_title.or(s_snip).or(s_badge)
            };

            if let Some(s) = score {
                results.push(SearchItem {
                    id: 0,
                    title: cmd.title.to_string(),
                    snippet: cmd.snippet.to_string(),
                    score: s,
                    badge: cmd.badge.to_string(),
                    icon: cmd.icon,
                    action: cmd.action.clone(),
                });
            }
        }
        results.sort_by(|a, b| b.score.cmp(&a.score));
        return results;
    }

    // ── 3. Notes Fuzzy Search Mode ───────────────────────────────────────────
    let mut results = Vec::new();
    for note in notes {
        let score_topic = fuzzy_match(raw, &note.topic);
        let score_body = fuzzy_match(raw, &note.body);
        if let Some(score) = score_topic.or(score_body) {
            let snippet = extract_snippet(&note.body, raw);

            results.push(SearchItem::for_note(
                note.id,
                note.topic.clone(),
                snippet,
                score,
            ));
        }
    }

    results.sort_by(|a, b| b.score.cmp(&a.score));
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzy_match() {
        assert!(fuzzy_match("rust", "The Rust Programming Language").is_some());
        assert!(fuzzy_match("rpl", "Rust Programming Language").is_some());
        assert!(fuzzy_match("xyz", "Rust Language").is_none());

        let score_prefix = fuzzy_match("rust", "Rust").unwrap();
        let score_sub = fuzzy_match("rust", "A long text about Rust").unwrap();
        assert!(score_prefix > score_sub);
    }

    #[test]
    fn test_command_palette_matching() {
        let def_font = "JetBrains Mono";
        let def_mode = crate::app::EditorInputMode::Vim;
        let def_luna = crate::lunaline::LunaStyle::Pill;

        let items = search_palette(">", &[], ThemeKind::TokyoNight, crate::services::sound::SoundProfile::Off, crate::caret::CaretKind::Beam, def_font, def_mode, def_luna);
        assert!(!items.is_empty());
        // Verify commands have NO "Settings: " prefix as requested
        assert!(items.iter().all(|i| !i.title.starts_with("Settings: ")));
        assert!(items.iter().any(|i| i.title.contains("Color Theme")));
        assert!(items.iter().any(|i| i.title.contains("Keyboard Shortcuts")));
        assert!(items.iter().any(|i| i.title.contains("Caret Style & Cursor FX")));

        let theme_filter = search_palette(">theme", &[], ThemeKind::TokyoNight, crate::services::sound::SoundProfile::Off, crate::caret::CaretKind::Beam, def_font, def_mode, def_luna);
        assert_eq!(theme_filter.len(), ThemeKind::ALL.len());
        let active = theme_filter.iter().find(|i| i.badge.contains("Active"));
        assert!(active.is_some());
        // Inactive themes must have empty badges
        let inactive_with_badge = theme_filter.iter().filter(|i| !i.badge.is_empty() && !i.badge.contains("Active")).count();
        assert_eq!(inactive_with_badge, 0);

        // Sound picker: >sound shows all profiles
        let sound_filter = search_palette(">sound", &[], ThemeKind::TokyoNight, crate::services::sound::SoundProfile::Thocky, crate::caret::CaretKind::Beam, def_font, def_mode, def_luna);
        assert_eq!(sound_filter.len(), crate::services::sound::SoundProfile::ALL.len());
        let active_sound = sound_filter.iter().find(|i| i.badge.contains("Active"));
        assert!(active_sound.is_some());
        assert_eq!(active_sound.unwrap().title, "Thocky");
        // Inactive sound profiles must have empty badges
        let inactive_sound_with_badge = sound_filter.iter().filter(|i| !i.badge.is_empty() && !i.badge.contains("Active")).count();
        assert_eq!(inactive_sound_with_badge, 0);

        // Caret picker: >caret shows all curated carets
        let caret_filter = search_palette(">caret", &[], ThemeKind::TokyoNight, crate::services::sound::SoundProfile::Off, crate::caret::CaretKind::Fire, def_font, def_mode, def_luna);
        assert_eq!(caret_filter.len(), crate::caret::CaretKind::ALL.len());
        let active_caret = caret_filter.iter().find(|i| i.badge.contains("Active"));
        assert!(active_caret.is_some());
        assert!(active_caret.unwrap().title.contains("Fire"));

        // Font picker: >font shows supported fonts
        let font_filter = search_palette(">font", &[], ThemeKind::TokyoNight, crate::services::sound::SoundProfile::Off, crate::caret::CaretKind::Beam, "JetBrains Mono", def_mode, def_luna);
        assert!(!font_filter.is_empty());
        assert!(font_filter.iter().any(|i| i.title == "JetBrains Mono" && i.badge.contains("Active")));

        // Mode picker: >mode shows modes
        let mode_filter = search_palette(">mode", &[], ThemeKind::TokyoNight, crate::services::sound::SoundProfile::Off, crate::caret::CaretKind::Beam, def_font, crate::app::EditorInputMode::Vim, def_luna);
        assert_eq!(mode_filter.len(), 2);
        assert!(mode_filter.iter().any(|i| i.title.contains("Vim") && i.badge.contains("Active")));

        // Note search items must have empty badges
        let sample_notes = vec![core::Note {
            id: 1,
            topic: "Architecture".to_string(),
            body: "MindForge system design".to_string(),
            struggled_with: None,
            created_at: chrono::NaiveDateTime::default(),
        }];
        let note_results = search_palette("Arch", &sample_notes, ThemeKind::TokyoNight, crate::services::sound::SoundProfile::Off, crate::caret::CaretKind::Beam, def_font, def_mode, def_luna);
        assert!(!note_results.is_empty());
        assert_eq!(note_results[0].badge, "");
    }

    #[test]
    fn test_extract_snippet_multibyte_safety() {
        // Multi-byte em-dash and unicode characters exactly like the user's crashing note
        let body = "prefix text before — em-dash and some unicode: 🚀 — and even more text following";
        let snippet = extract_snippet(body, "unicode");
        assert!(snippet.contains("unicode"));
        assert!(!snippet.is_empty());

        // Long text with em-dash near byte boundary
        let long_body = format!("{} — {}", "a".repeat(835), "action words");
        let snip2 = extract_snippet(&long_body, "action");
        assert!(snip2.contains("action"));
    }
}

