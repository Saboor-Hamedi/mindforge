//! Neovim-style minimalist welcome dashboard and quick-action launcher.
//!
//! Renders the initial welcome screen users see on startup, featuring:
//! - ASCII art logo (when viewport is large enough)
//! - Quick-action buttons for common tasks (new note, find, terminal, etc.)
//! - Drag-and-drop hint for Obsidian vault import
//! - Responsive layout that adapts to available space
//!
//! The dashboard is purely presentational — it returns a `DashboardAction`
//! enum variant when the user clicks a button, and the caller decides what
//! to do with that action.

use crate::ui::theme::Theme;
use eframe::egui::{self, pos2, vec2, Align2, FontId, Rect};

/// Actions that can be triggered from the dashboard's quick-action buttons.
/// Returned by `render_welcome_dashboard` when the user interacts with a button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashboardAction {
    /// Create a new blank note (Ctrl+N)
    NewNote,
    /// Open the fuzzy search modal to find an existing note (Ctrl+P)
    FindNote,
    /// Open a recently accessed note by its ID
    OpenRecent(i64),
    /// Toggle the embedded terminal pane
    OpenTerminal,
    /// Toggle the AI assistant drawer
    OpenAi,
    /// Open the built-in documentation reader
    OpenDocs,
    /// Open the settings/preferences modal
    OpenSettings,
    /// Toggle distraction-free zen mode
    ToggleZen,
    /// Quit the application
    Quit,
}

const ASCII_LOGO: [&str; 6] = [
    r" __  __ _           _ _____                    ",
    r"|  \/  (_)_ __   __| |  ___|__  _ __ __ _  ___ ",
    r"| |\/| | | '_ \ / _` | |_ / _ \| '__/ _` |/ _ \",
    r"| |  | | | | | | (_| |  _| (_) | | | (_| |  __/",
    r"|_|  |_|_|_| |_|\__,_|_|  \___/|_|  \__, |\___|",
    r"                                    |___/      ",
];

/// Renders the welcome dashboard inside the given screen rectangle.
///
/// # Arguments
/// * `ui` — egui UI context for allocating widgets and handling input
/// * `painter` — egui painter for drawing text and shapes
/// * `rect` — Screen area to render within (clipped automatically)
/// * `theme` — Active color theme for consistent styling
/// * `total_notes` — Total note count displayed in the subtitle
/// * `modals_open` — When true, suppresses hover effects and clicks (modal is on top)
///
/// # Returns
/// `Some(DashboardAction)` if the user clicked an action button, `None` otherwise.
pub fn render_welcome_dashboard(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    rect: Rect,
    theme: &Theme,
    total_notes: usize,
    modals_open: bool,
) -> Option<DashboardAction> {
    if rect.width() < 120.0 || rect.height() < 80.0 {
        return None;
    }

    // Clip strictly to rect so NO text or shapes ever bleed outside into preview, sidebar, or terminal!
    let painter = painter.with_clip_rect(rect);
    let painter = &painter;

    let mut action = None;
    let center = rect.center();

    let use_ascii = rect.width() >= 520.0 && rect.height() >= 440.0;
    let btn_h = 26.0;
    let btn_gap = 5.0;

    let action_items: [(&str, &str, &str, DashboardAction); 7] = [
        ("N", "New Note", "Ctrl+N", DashboardAction::NewNote),
        ("P", "Find Note", "Ctrl+P", DashboardAction::FindNote),
        ("T", "Embedded Terminal", "Ctrl+J", DashboardAction::OpenTerminal),
        ("I", "AI Assistant", "Ctrl+Shift+I", DashboardAction::OpenAi),
        ("D", "Documentation", ":doc", DashboardAction::OpenDocs),
        ("S", "Preferences", "Ctrl+,", DashboardAction::OpenSettings),
        ("Z", "Zen Focus Mode", "Ctrl+.", DashboardAction::ToggleZen),
    ];

    // Determine how many buttons fit comfortably in the available height
    let max_buttons = if rect.height() < 240.0 {
        3
    } else if rect.height() < 320.0 {
        5
    } else {
        7
    };
    let visible_actions = &action_items[..max_buttons.min(action_items.len())];
    let total_buttons_h = visible_actions.len() as f32 * (btn_h + btn_gap);

    let (content_start_y, actions_start_y) = if use_ascii {
        let logo_line_h = 16.0;
        let total_logo_h = (ASCII_LOGO.len() as f32) * logo_line_h;
        let total_content_h = total_logo_h + 20.0 + total_buttons_h;
        let logo_start_y = (center.y - total_content_h * 0.48).max(rect.min.y + 10.0);

        for (idx, line) in ASCII_LOGO.iter().enumerate() {
            let y = logo_start_y + idx as f32 * logo_line_h;
            painter.text(
                pos2(center.x, y),
                Align2::CENTER_CENTER,
                *line,
                FontId::monospace(13.5),
                theme.accent,
            );
        }

        let sub_y = logo_start_y + total_logo_h + 8.0;
        let subtitle = format!("MINDFORGE  •  Fast, Minimalist Notes  •  {} notes", total_notes);
        painter.text(
            pos2(center.x, sub_y),
            Align2::CENTER_CENTER,
            subtitle,
            FontId::monospace(11.5),
            theme.muted,
        );

        (logo_start_y, sub_y + 22.0)
    } else {
        // Compact modern header when sidebar, preview, or terminal is open
        let total_content_h = 44.0 + total_buttons_h;
        let start_y = (center.y - total_content_h * 0.48).max(rect.min.y + 10.0);

        painter.text(
            pos2(center.x, start_y + 8.0),
            Align2::CENTER_CENTER,
            "MINDFORGE",
            FontId::monospace((rect.width() * 0.05).clamp(16.0, 22.0)),
            theme.accent,
        );

        let subtitle = if rect.width() > 340.0 {
            format!("Fast, Minimalist Notes  ·  {} notes", total_notes)
        } else {
            format!("{} notes", total_notes)
        };
        painter.text(
            pos2(center.x, start_y + 28.0),
            Align2::CENTER_CENTER,
            subtitle,
            FontId::monospace(11.0),
            theme.muted,
        );

        (start_y, start_y + 44.0)
    };

    let _ = content_start_y;
    let btn_w = 310.0f32.min(rect.width() - 32.0).max(120.0);

    // ── 2. Action Buttons (ZERO background, ZERO border) ───────────────────
    for (idx, (key, label, hint, act)) in visible_actions.iter().enumerate() {
        let btn_rect = Rect::from_center_size(
            pos2(center.x, actions_start_y + idx as f32 * (btn_h + btn_gap)),
            vec2(btn_w, btn_h),
        );
        let resp = ui.allocate_rect(btn_rect, egui::Sense::click());
        let hovered = resp.hovered() || ui.rect_contains_pointer(btn_rect);

        if !modals_open && hovered {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        // Bracket key trigger
        painter.text(
            pos2(btn_rect.min.x + 8.0, btn_rect.center().y),
            Align2::LEFT_CENTER,
            format!("[{}]", key),
            FontId::monospace(12.0),
            if hovered && !modals_open { theme.highlight } else { theme.accent },
        );

        // Action label
        painter.text(
            pos2(btn_rect.min.x + 42.0, btn_rect.center().y),
            Align2::LEFT_CENTER,
            *label,
            FontId::monospace(12.0),
            if hovered && !modals_open { theme.text } else { theme.muted },
        );

        // Shortcut hint (hidden if too narrow)
        if btn_w > 220.0 {
            painter.text(
                pos2(btn_rect.max.x - 8.0, btn_rect.center().y),
                Align2::RIGHT_CENTER,
                *hint,
                FontId::monospace(11.0),
                theme.border(),
            );
        }

        if !modals_open && (resp.clicked() || (hovered && ui.input(|i| i.pointer.primary_clicked()))) {
            action = Some(*act);
        }
    }

    // ── 3. Subtle Obsidian Vault Drag & Drop Hint (Zero background aesthetic) ───
    if rect.height() >= 330.0 {
        let hint_y = actions_start_y + visible_actions.len() as f32 * (btn_h + btn_gap) + 14.0;
        painter.text(
            pos2(center.x, hint_y),
            Align2::CENTER_CENTER,
            "💡 Drag & drop Obsidian vaults or Markdown folders anywhere to import",
            FontId::monospace(11.0),
            theme.muted,
        );
    }

    action
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ascii_logo_non_empty() {
        assert_eq!(ASCII_LOGO.len(), 6);
        for line in ASCII_LOGO {
            assert!(!line.trim().is_empty());
        }
    }

    #[test]
    fn test_dashboard_action_variants() {
        let act = DashboardAction::NewNote;
        assert_eq!(act, DashboardAction::NewNote);
        assert_ne!(act, DashboardAction::FindNote);
    }
}
