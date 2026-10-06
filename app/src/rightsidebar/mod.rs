//! Right Sidebar containing Outline (H1-H6) and Backlinks inspectors with sleek tabs.

pub mod backlinks;
pub mod outline;

use crate::editor::Editor;
use crate::ui::theme::Theme;
use crate::wikilink::BacklinkItem;
use backlinks::{render_backlinks_panel, BacklinkAction};
use eframe::egui::{pos2, Align2, FontId, Rect, Stroke, Ui};
use outline::{extract_outline_headings, render_outline_panel, OutlineAction, OutlineHeading};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RightSidebarTab {
    #[default]
    Outline,
    Backlinks,
}

#[derive(Debug, Clone, Default)]
pub struct RightSidebarState {
    pub active_tab: RightSidebarTab,
    pub outline_selected_idx: usize,
    pub backlinks_selected_idx: usize,
    pub cached_headings: Vec<OutlineHeading>,
    pub cached_backlinks: Vec<BacklinkItem>,
    pub cached_backlinks_target: Option<(String, Option<i64>)>,
}

pub enum RightSidebarAction {
    JumpToChar(usize),
    OpenNote { id: i64, title: String },
}

/// Renders the sleek Right Sidebar with Outline and Backlinks tabs.
pub fn render_right_sidebar(
    ui: &mut Ui,
    rect: Rect,
    state: &mut RightSidebarState,
    ed: &Editor,
    theme: &Theme,
    opacity: f32,
    active_note_title: &str,
    notes_list: &[core::Note],
    active_note_id: Option<i64>,
) -> Option<RightSidebarAction> {
    // Build the backlinks list only when the selected note changes. This
    // function runs on every UI frame; rescanning every note here stalls input.
    let target_key = (active_note_title.to_owned(), active_note_id);
    if state.cached_backlinks_target.as_ref() != Some(&target_key) {
        state.cached_backlinks =
            crate::wikilink::find_backlinks(active_note_title, notes_list, active_note_id);
        state.cached_backlinks_target = Some(target_key);
    }

    let painter = ui.painter();

    // 1. Surface background & left border divider
    let bg_color = if opacity < 1.0 {
        theme.surface()
    } else {
        theme.bg
    };
    painter.rect_filled(rect, 0.0, bg_color);
    painter.line_segment(
        [rect.left_top(), rect.left_bottom()],
        Stroke::new(1.0_f32, theme.border()),
    );

    // 2. Sleek Dual-Tab Header Bar (Outline & Backlinks)
    let tab_bar_h = 32.0;
    let tab_bar_rect = Rect::from_min_max(rect.min, pos2(rect.max.x, rect.min.y + tab_bar_h));

    painter.line_segment(
        [
            pos2(tab_bar_rect.min.x, tab_bar_rect.max.y),
            pos2(tab_bar_rect.max.x, tab_bar_rect.max.y),
        ],
        Stroke::new(1.0_f32, theme.border()),
    );

    let tab_w = tab_bar_rect.width() * 0.5;

    // Outline Tab Button
    let outline_rect = Rect::from_min_max(
        tab_bar_rect.min,
        pos2(tab_bar_rect.min.x + tab_w, tab_bar_rect.max.y),
    );
    let outline_hovered = ui.rect_contains_pointer(outline_rect);
    let is_outline_active = state.active_tab == RightSidebarTab::Outline;

    if outline_hovered && ui.input(|i| i.pointer.primary_clicked()) {
        state.active_tab = RightSidebarTab::Outline;
    }

    let outline_text_color = if is_outline_active || outline_hovered {
        theme.accent
    } else {
        theme.muted
    };

    painter.text(
        outline_rect.center(),
        Align2::CENTER_CENTER,
        "Outline",
        FontId::monospace(11.5),
        outline_text_color,
    );

    if is_outline_active {
        // Active indicator line on bottom
        painter.line_segment(
            [
                pos2(outline_rect.min.x + 12.0, outline_rect.max.y - 1.5),
                pos2(outline_rect.max.x - 12.0, outline_rect.max.y - 1.5),
            ],
            Stroke::new(2.0_f32, theme.accent),
        );
    }

    // Backlinks Tab Button
    let backlinks_rect = Rect::from_min_max(
        pos2(tab_bar_rect.min.x + tab_w, tab_bar_rect.min.y),
        tab_bar_rect.max,
    );
    let backlinks_hovered = ui.rect_contains_pointer(backlinks_rect);
    let is_backlinks_active = state.active_tab == RightSidebarTab::Backlinks;

    if backlinks_hovered && ui.input(|i| i.pointer.primary_clicked()) {
        state.active_tab = RightSidebarTab::Backlinks;
    }

    let backlinks_text_color = if is_backlinks_active || backlinks_hovered {
        theme.accent
    } else {
        theme.muted
    };

    // Label with count if > 0
    let bl_label = if !state.cached_backlinks.is_empty() {
        format!("Links ({})", state.cached_backlinks.len())
    } else {
        "Backlinks".to_string()
    };

    painter.text(
        backlinks_rect.center(),
        Align2::CENTER_CENTER,
        bl_label,
        FontId::monospace(11.5),
        backlinks_text_color,
    );

    if is_backlinks_active {
        painter.line_segment(
            [
                pos2(backlinks_rect.min.x + 12.0, backlinks_rect.max.y - 1.5),
                pos2(backlinks_rect.max.x - 12.0, backlinks_rect.max.y - 1.5),
            ],
            Stroke::new(2.0_f32, theme.accent),
        );
    }

    // 3. Tab Content Area
    let content_rect = Rect::from_min_max(
        pos2(rect.min.x + 4.0, tab_bar_rect.max.y + 4.0),
        pos2(rect.max.x - 4.0, rect.max.y - 4.0),
    );

    match state.active_tab {
        RightSidebarTab::Outline => {
            // Refresh outline headings from current editor text
            state.cached_headings = extract_outline_headings(ed);
            let action = render_outline_panel(
                ui,
                content_rect,
                &state.cached_headings,
                &mut state.outline_selected_idx,
                theme,
                ed.cur,
            );
            action.map(|a| match a {
                OutlineAction::JumpToChar(pos) => RightSidebarAction::JumpToChar(pos),
            })
        }
        RightSidebarTab::Backlinks => {
            let action = render_backlinks_panel(
                ui,
                content_rect,
                &state.cached_backlinks,
                &mut state.backlinks_selected_idx,
                theme,
            );
            action.map(|a| match a {
                BacklinkAction::OpenNote { id, title } => {
                    RightSidebarAction::OpenNote { id, title }
                }
            })
        }
    }
}
