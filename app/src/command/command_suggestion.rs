//! Command suggestion, autocomplete popup, and bash-like command history.
//!
//! Provides live fuzzy filtering over both historical commands and the built-in
//! command catalog, with keyboard navigation (Ctrl+J / Ctrl+K / Up / Down / Tab).

use crate::app::App;
use crate::command::dispatch::{execute_command, COMMAND_CATALOG};
use crate::ui::theme::Theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Key, Modifiers, Rect, Stroke};

/// A single matched command suggestion item.
#[derive(Clone, Debug, PartialEq)]
pub struct SuggestionItem {
    pub text: String,
    pub desc: String,
    pub is_history: bool,
    pub score: i64,
}

/// Appends a command to bash-like history.
///
/// Trims whitespace, strips leading ':', deduplicates previous occurrences,
/// and maintains the most recent entries up to `max_len`.
pub fn record_history(history: &mut Vec<String>, cmd: &str) {
    let clean = cmd.trim();
    let clean = clean.strip_prefix(':').unwrap_or(clean).trim();
    if clean.is_empty() {
        return;
    }

    // Remove existing match so the most recent execution moves to the end
    history.retain(|h| h.trim() != clean);

    history.push(clean.to_string());

    const MAX_HISTORY: usize = 200;
    if history.len() > MAX_HISTORY {
        let excess = history.len() - MAX_HISTORY;
        history.drain(0..excess);
    }
}

/// Returns ranked suggestions matching `query` from both history and the command catalog.
pub fn get_filtered_suggestions(query: &str, history: &[String]) -> Vec<SuggestionItem> {
    let raw_query = query.trim();
    let clean_query = raw_query.strip_prefix(':').unwrap_or(raw_query).trim().to_lowercase();

    let mut results: Vec<SuggestionItem> = Vec::new();
    let mut seen_commands = std::collections::HashSet::new();

    // 1. If empty query (just ':' typed), show recent history followed by common commands
    if clean_query.is_empty() {
        // Up to 5 most recent history items
        for cmd in history.iter().rev().take(5) {
            let desc = lookup_catalog_desc(cmd).unwrap_or("Recent history command");
            if seen_commands.insert(cmd.to_lowercase()) {
                results.push(SuggestionItem {
                    text: cmd.clone(),
                    desc: desc.to_string(),
                    is_history: true,
                    score: 100,
                });
            }
        }

        // Top default commands from catalog
        for entry in COMMAND_CATALOG.iter().take(6) {
            if seen_commands.insert(entry.name.to_lowercase()) {
                results.push(SuggestionItem {
                    text: entry.name.to_string(),
                    desc: entry.desc.to_string(),
                    is_history: false,
                    score: 50,
                });
            }
        }

        return results;
    }

    // 2. Non-empty query: search history first (higher priority for exact command sequences)
    for cmd in history.iter().rev() {
        let cmd_clean = cmd.trim().to_lowercase();
        if seen_commands.contains(&cmd_clean) {
            continue;
        }

        if let Some(score) = score_match(&clean_query, &cmd_clean) {
            seen_commands.insert(cmd_clean);
            let desc = lookup_catalog_desc(cmd).unwrap_or("History");
            results.push(SuggestionItem {
                text: cmd.clone(),
                desc: desc.to_string(),
                is_history: true,
                score: score + 15, // slight history bias
            });
        }
    }

    // 3. Search built-in catalog commands
    for entry in COMMAND_CATALOG {
        let name_lower = entry.name.to_lowercase();
        if seen_commands.contains(&name_lower) {
            continue;
        }

        if let Some(score) = score_match(&clean_query, &name_lower) {
            seen_commands.insert(name_lower);
            results.push(SuggestionItem {
                text: entry.name.to_string(),
                desc: entry.desc.to_string(),
                is_history: false,
                score,
            });
        }
    }

    // Sort descending by match score
    results.sort_by(|a, b| b.score.cmp(&a.score));

    // Cap at 7 results for a clean, non-cluttered popup
    results.truncate(7);
    results
}

/// Helper to compute match score: favors exact match > prefix match > fuzzy match.
fn score_match(query: &str, target: &str) -> Option<i64> {
    if target == query {
        return Some(2000);
    }
    if target.starts_with(query) {
        return Some(1000 + (target.len() as i64 * -2));
    }
    crate::services::fuzzy::fuzzy_match(query, target)
}

/// Finds the catalog description for a command name if available.
fn lookup_catalog_desc(cmd: &str) -> Option<&'static str> {
    let name = cmd.split_whitespace().next()?.to_lowercase();
    COMMAND_CATALOG
        .iter()
        .find(|c| c.name.eq_ignore_ascii_case(&name))
        .map(|c| c.desc)
}

/// Intercepts suggestion-specific keyboard shortcuts (navigation, completion, execution).
/// Returns `true` if the key was handled, allowing `input.rs` to remain clean and DRY.
pub fn handle_suggestion_key(app: &mut App, key: Key, modifiers: Modifiers, now: f64) -> bool {
    if app.command_bar.prefix != ':' {
        return false;
    }
    let suggestions = get_filtered_suggestions(&app.editor.cmd_ed.text(), &app.command_bar.history);

    match key {
        Key::Enter => {
            let cmd = if app.command_bar.navigated && !suggestions.is_empty() {
                if let Some(item) = suggestions.get(app.command_bar.selected_idx) {
                    item.text.clone()
                } else {
                    app.editor.cmd_ed.text()
                }
            } else {
                app.editor.cmd_ed.text()
            };
            app.command_bar.in_command = false;
            app.command_bar.navigated = false;
            app.command_bar.selected_idx = 0;
            app.editor.cmd_ed.clear();
            record_history(&mut app.command_bar.history, &cmd);
            app.misc.showcmd.record_action(&format!(":{}", cmd), now);
            execute_command(app, &cmd, now);
            true
        }
        Key::Tab => {
            if !suggestions.is_empty() {
                let idx = app.command_bar.selected_idx.min(suggestions.len() - 1);
                if let Some(item) = suggestions.get(idx) {
                    app.editor.cmd_ed.set_text(&item.text);
                    app.editor.cmd_ed.cur = app.editor.cmd_ed.buf.len();
                    app.command_bar.navigated = false;
                    app.misc.showcmd.set_command(&app.editor.cmd_ed.text(), now);
                }
            }
            true
        }
        // Down navigation: Ctrl+J or ArrowDown
        Key::J if modifiers.ctrl => {
            if !suggestions.is_empty() {
                app.command_bar.selected_idx = (app.command_bar.selected_idx + 1) % suggestions.len();
                app.command_bar.navigated = true;
            }
            true
        }
        Key::ArrowDown => {
            if !suggestions.is_empty() {
                app.command_bar.selected_idx = (app.command_bar.selected_idx + 1) % suggestions.len();
                app.command_bar.navigated = true;
            }
            true
        }
        // Up navigation: Ctrl+K or ArrowUp
        Key::K if modifiers.ctrl => {
            if !suggestions.is_empty() {
                if app.command_bar.selected_idx == 0 {
                    app.command_bar.selected_idx = suggestions.len().saturating_sub(1);
                } else {
                    app.command_bar.selected_idx -= 1;
                }
                app.command_bar.navigated = true;
            }
            true
        }
        Key::ArrowUp => {
            if !suggestions.is_empty() {
                if app.command_bar.selected_idx == 0 {
                    app.command_bar.selected_idx = suggestions.len().saturating_sub(1);
                } else {
                    app.command_bar.selected_idx -= 1;
                }
                app.command_bar.navigated = true;
            }
            true
        }
        _ => false,
    }
}

/// Action resulting from user interaction with the suggestions popup.
pub enum SuggestionAction {
    Select(usize),
    Execute(String),
}

/// Complete overlay rendering helper called cleanly from `shell.rs`.
/// Renders on top of all main UI elements and the sidebar.
pub fn render_command_suggestions_overlay(
    app: &mut App,
    ui: &egui::Ui,
    painter: &egui::Painter,
    dock_rect: Rect,
    now: f64,
) {
    if !app.command_bar.in_command || app.command_bar.prefix != ':' {
        return;
    }

    let suggestions = get_filtered_suggestions(&app.editor.cmd_ed.text(), &app.command_bar.history);
    if let Some(action) = render_command_suggestions(
        ui,
        painter,
        dock_rect,
        &suggestions,
        app.command_bar.selected_idx,
        &app.misc.theme,
    ) {
        match action {
            SuggestionAction::Select(idx) => {
                app.command_bar.selected_idx = idx;
                app.command_bar.navigated = true;
            }
            SuggestionAction::Execute(cmd) => {
                app.command_bar.in_command = false;
                app.command_bar.navigated = false;
                app.command_bar.selected_idx = 0;
                app.editor.cmd_ed.clear();
                record_history(&mut app.command_bar.history, &cmd);
                app.misc.showcmd.record_action(&format!(":{}", cmd), now);
                execute_command(app, &cmd, now);
            }
        }
    }
}

/// Renders the floating suggestion popup card above the command bar.
pub fn render_command_suggestions(
    ui: &egui::Ui,
    painter: &egui::Painter,
    dock_rect: Rect,
    suggestions: &[SuggestionItem],
    selected_idx: usize,
    theme: &Theme,
) -> Option<SuggestionAction> {
    if suggestions.is_empty() {
        return None;
    }

    let mut action = None;
    let item_h = 26.0f32;
    let header_h = 22.0f32;
    let pad_y = 6.0f32;
    let total_h = header_h + (suggestions.len() as f32 * item_h) + pad_y * 2.0;

    let popup_w = 340.0f32.min(dock_rect.width() - 32.0);
    let popup_x = (dock_rect.min.x + 14.0).min(dock_rect.max.x - popup_w - 14.0);
    let popup_y = dock_rect.min.y - total_h - 6.0;

    let popup_rect = Rect::from_min_size(pos2(popup_x, popup_y), vec2(popup_w, total_h));

    // Drop shadow
    let shadow_color = if theme.is_light() {
        Color32::from_black_alpha(28)
    } else {
        Color32::from_black_alpha(85)
    };
    painter.rect_filled(popup_rect.expand(4.0), 8.0, shadow_color);

    // Main capsule container
    painter.rect(
        popup_rect,
        6.0,
        theme.surface(),
        Stroke::new(1.0_f32, theme.border()),
        egui::StrokeKind::Inside,
    );

    // Header label & hint
    let header_rect = Rect::from_min_size(popup_rect.min + vec2(10.0, pad_y), vec2(popup_w - 20.0, header_h));
    painter.text(
        pos2(header_rect.min.x, header_rect.center().y),
        Align2::LEFT_CENTER,
        "COMMAND AUTOCOMPLETE",
        FontId::monospace(10.0),
        theme.muted,
    );
    painter.text(
        pos2(header_rect.max.x, header_rect.center().y),
        Align2::RIGHT_CENTER,
        "Ctrl+J/K or ↑↓",
        FontId::monospace(9.5),
        theme.muted,
    );

    // Separator line
    let sep_y = header_rect.max.y + 1.0;
    painter.line_segment(
        [pos2(popup_rect.min.x + 8.0, sep_y), pos2(popup_rect.max.x - 8.0, sep_y)],
        Stroke::new(1.0_f32, theme.border().linear_multiply(0.6)),
    );

    // Rows
    let list_top = sep_y + 3.0;
    for (i, item) in suggestions.iter().enumerate() {
        let row_rect = Rect::from_min_size(
            pos2(popup_rect.min.x + 4.0, list_top + (i as f32 * item_h)),
            vec2(popup_w - 8.0, item_h),
        );

        let is_selected = i == selected_idx;
        let is_hovered = ui.rect_contains_pointer(row_rect);

        if is_selected {
            let sel_bg = Color32::from_rgba_unmultiplied(
                theme.muted.r(),
                theme.muted.g(),
                theme.muted.b(),
                if theme.is_light() { 22 } else { 30 },
            );
            painter.rect(
                row_rect,
                4.0,
                sel_bg,
                Stroke::NONE,
                egui::StrokeKind::Inside,
            );
        } else if is_hovered {
            painter.rect_filled(
                row_rect,
                4.0,
                theme.bg.lerp_to_gamma(theme.surface(), 0.5),
            );
        }

        // Pointer click
        if is_hovered && ui.input(|inp| inp.pointer.primary_clicked()) {
            action = Some(SuggestionAction::Execute(item.text.clone()));
        } else if is_hovered && ui.input(|inp| inp.pointer.hover_pos().is_some()) {
            if !is_selected {
                action = Some(SuggestionAction::Select(i));
            }
        }

        // Left badge: HIST or CMD
        let (badge_txt, badge_color) = if item.is_history {
            ("HIST", Color32::from_rgb(180, 130, 40))
        } else {
            ("CMD", theme.accent)
        };
        let badge_rect = Rect::from_min_size(
            pos2(row_rect.min.x + 6.0, row_rect.center().y - 7.0),
            vec2(28.0, 14.0),
        );
        painter.text(
            badge_rect.center(),
            Align2::CENTER_CENTER,
            badge_txt,
            FontId::monospace(8.5),
            badge_color,
        );

        // Command text
        let cmd_display = format!(":{}", item.text);
        let text_color = if is_selected { theme.accent } else { theme.text };
        painter.text(
            pos2(badge_rect.max.x + 8.0, row_rect.center().y),
            Align2::LEFT_CENTER,
            cmd_display,
            FontId::monospace(12.5),
            text_color,
        );

        // Description text on right
        let desc_text = if item.desc.chars().count() > 24 {
            let s: String = item.desc.chars().take(22).collect();
            format!("{}…", s)
        } else {
            item.desc.clone()
        };
        painter.text(
            pos2(row_rect.max.x - 8.0, row_rect.center().y),
            Align2::RIGHT_CENTER,
            desc_text,
            FontId::monospace(10.5),
            theme.muted,
        );
    }

    action
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_history_dedup_and_clean() {
        let mut hist = Vec::new();
        record_history(&mut hist, ":w");
        record_history(&mut hist, "set nu");
        record_history(&mut hist, ":w"); // duplicate of first

        assert_eq!(hist.len(), 2);
        assert_eq!(hist[0], "set nu");
        assert_eq!(hist[1], "w");
    }

    #[test]
    fn test_empty_query_suggestions() {
        let hist = vec!["custom_cmd".to_string()];
        let results = get_filtered_suggestions("", &hist);
        assert!(!results.is_empty());
        assert_eq!(results[0].text, "custom_cmd");
        assert!(results[0].is_history);
    }

    #[test]
    fn test_filtered_suggestions_prefix_and_fuzzy() {
        let hist = vec!["set preview".to_string(), "w".to_string()];
        let results = get_filtered_suggestions("set", &hist);
        assert!(!results.is_empty());
        // Exact prefix match in history or catalog should rank highest
        assert!(results.iter().any(|r| r.text.starts_with("set")));
    }
}
