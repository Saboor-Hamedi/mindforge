//! Tab bar widget for multi-note and documentation views.
//!
//! Renders as the top strip inside the editor panel card.

use crate::ui::theme::Theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Rect, Stroke};

pub const TAB_ROW_H: f32 = 35.0;

pub enum TabAction {
    Select(usize),
    Close(usize),
}

pub struct TabItem<'a> {
    pub title: &'a str,
    pub is_dirty: bool,
    pub is_active: bool,
}

/// Renders the tab bar strip at the top of the editor panel.
pub fn render_tab_bar(
    ui: &egui::Ui,
    painter: &egui::Painter,
    tab_bar_rect: Rect,
    tabs: &[TabItem],
    theme: &Theme,
    scroll_offset: &mut f32,
    active_changed: bool,
    occluded_rect: Option<Rect>,
) -> Option<TabAction> {
    if tabs.is_empty() {
        return None;
    }

    let mut action = None;

    let tab_gap = 6.0;
    let initial_pad = 6.0;
    let chip_margin_y = 4.0;
    let tab_h = tab_bar_rect.height() - chip_margin_y * 2.0;
    let primary_clicked = ui.input(|i| i.pointer.primary_clicked());
    let mouse_pos = ui.input(|i| i.pointer.interact_pos());
    let is_occluded = occluded_rect.map_or(false, |r| mouse_pos.map_or(false, |p| r.contains(p)));
    let mouse_in_bar = !is_occluded && ui.rect_contains_pointer(tab_bar_rect);

    // 1. Measure all tabs and calculate relative positions
    let mut tab_widths: Vec<f32> = Vec::with_capacity(tabs.len());
    let mut display_titles: Vec<String> = Vec::with_capacity(tabs.len());

    for tab in tabs {
        let title_clean = if tab.title.trim().is_empty() {
            "Untitled"
        } else {
            tab.title
        };
        let max_chars = 22;
        let display_title = if title_clean.chars().count() > max_chars {
            format!(
                "{}...",
                title_clean
                    .chars()
                    .take(max_chars.saturating_sub(3))
                    .collect::<String>()
            )
        } else {
            title_clean.to_string()
        };

        let text_w = display_title.len() as f32 * 7.0;
        let tab_w = (text_w + 46.0).clamp(94.0, 210.0);
        tab_widths.push(tab_w);
        display_titles.push(display_title);
    }

    // Relative start and end offsets from content origin (initial_pad = first tab left margin)
    let mut tab_positions: Vec<(f32, f32)> = Vec::with_capacity(tabs.len());
    let mut current_offset = initial_pad;
    for &w in &tab_widths {
        tab_positions.push((current_offset, current_offset + w));
        current_offset += w + tab_gap;
    }
    let total_content_w = (current_offset - tab_gap + initial_pad).max(0.0);

    let visible_tab_area = tab_bar_rect;
    let viewport_w = visible_tab_area.width().max(1.0);
    let max_scroll = (total_content_w - viewport_w).max(0.0);

    let clip_painter = painter.with_clip_rect(visible_tab_area);

    // 2. Fast mouse wheel scrolling across tab bar
    if mouse_in_bar {
        let raw_scroll = ui.input(|i| {
            if i.raw_scroll_delta.x.abs() > 0.1 || i.raw_scroll_delta.y.abs() > 0.1 {
                i.raw_scroll_delta
            } else {
                i.smooth_scroll_delta
            }
        });
        let delta = if raw_scroll.x.abs() > 0.1 {
            raw_scroll.x
        } else if raw_scroll.y.abs() > 0.1 {
            raw_scroll.y
        } else {
            0.0
        };
        if delta.abs() > 0.1 {
            *scroll_offset -= delta * 2.0;
        }
    }

    // 3. Auto-scroll ONLY when active tab changed (never snap back during free user scrolling)
    if active_changed {
        if let Some(active_idx) = tabs.iter().position(|t| t.is_active) {
            let (active_start, active_end) = tab_positions[active_idx];
            if active_end > *scroll_offset + viewport_w {
                *scroll_offset = (active_end - viewport_w + 12.0).min(max_scroll);
            }
            if active_start < *scroll_offset {
                *scroll_offset = active_start.max(0.0);
            }
        }
    }

    *scroll_offset = (*scroll_offset).clamp(0.0, max_scroll);

    // 4. Render visible tabs as discrete floating chips
    for (idx, tab) in tabs.iter().enumerate() {
        let (rel_start, rel_end) = tab_positions[idx];
        let tab_w = rel_end - rel_start;
        let tab_x = visible_tab_area.min.x + rel_start - *scroll_offset;

        let tab_rect = Rect::from_min_size(
            pos2(tab_x, tab_bar_rect.min.y + chip_margin_y),
            vec2(tab_w, tab_h),
        );

        // Skip completely off-screen tabs without breaking the loop
        if tab_rect.max.x < visible_tab_area.min.x || tab_rect.min.x > visible_tab_area.max.x {
            continue;
        }

        let is_tab_hovered = mouse_in_bar && ui.rect_contains_pointer(tab_rect);

        // Close button: compact rounded chip button inside tab
        let btn_w = 18.0;
        let btn_h = 18.0;
        let close_rect = Rect::from_center_size(
            pos2(tab_rect.max.x - 14.0, tab_rect.center().y),
            vec2(btn_w, btn_h),
        );
        let is_close_hovered = mouse_in_bar && ui.rect_contains_pointer(close_rect);
        let show_close = is_tab_hovered;

        // Tab chip surface styling: single crisp active signal (bottom accent pill)
        if tab.is_active {
            let accent_w = (tab_rect.width() - 20.0).max(18.0);
            let accent_bar = Rect::from_center_size(
                pos2(tab_rect.center().x, tab_rect.max.y - 1.5),
                vec2(accent_w, 2.0),
            );
            clip_painter.rect_filled(accent_bar, 1.0, theme.accent);
        } else if is_tab_hovered {
            // Subtle, soft hover background
            let bg = if theme.is_light() {
                Color32::from_rgba_unmultiplied(0, 0, 0, 10)
            } else {
                Color32::from_rgba_unmultiplied(255, 255, 255, 12)
            };
            clip_painter.rect_filled(tab_rect, 5.0, bg);
        }

        // Title and dirty indicator
        let text_color = if tab.is_active {
            theme.text
        } else if is_tab_hovered {
            theme.text.lerp_to_gamma(theme.muted, 0.3)
        } else {
            theme.muted
        };

        let label_pos = pos2(tab_rect.min.x + 10.0, tab_rect.center().y);
        clip_painter.text(
            label_pos,
            Align2::LEFT_CENTER,
            &display_titles[idx],
            FontId::proportional(12.0),
            text_color,
        );

        // Keep the dirty mark and close control in one stable slot. Hover swaps
        // the mark for the close icon, so the title never shifts or collides.
        if show_close {
            let close_color = if is_close_hovered {
                Color32::from_rgb(235, 90, 90)
            } else {
                theme.muted
            };

            if is_close_hovered {
                clip_painter.rect_filled(
                    close_rect,
                    4.0,
                    if theme.is_light() {
                        Color32::from_rgba_unmultiplied(0, 0, 0, 16)
                    } else {
                        Color32::from_rgba_unmultiplied(255, 255, 255, 20)
                    },
                );
            }

            clip_painter.text(
                close_rect.center(),
                Align2::CENTER_CENTER,
                "×",
                FontId::proportional(13.0),
                close_color,
            );
        } else if tab.is_dirty {
            clip_painter.circle_filled(
                close_rect.center(),
                3.0,
                if tab.is_active {
                    theme.accent
                } else {
                    theme.muted
                },
            );
        }

        // Mouse interaction
        if primary_clicked && mouse_in_bar {
            if let Some(pos) = mouse_pos {
                if visible_tab_area.contains(pos) {
                    if show_close && close_rect.contains(pos) {
                        action = Some(TabAction::Close(idx));
                    } else if tab_rect.contains(pos) {
                        action = Some(TabAction::Select(idx));
                    }
                }
            }
        }

        // Small divider tick between tabs
        if !tab.is_active && idx + 1 < tabs.len() && !tabs[idx + 1].is_active {
            let tick_x = tab_rect.max.x + 1.0;
            let tick_mid_y = tab_bar_rect.center().y;
            clip_painter.line_segment(
                [
                    pos2(tick_x, tick_mid_y - 6.0),
                    pos2(tick_x, tick_mid_y + 6.0),
                ],
                Stroke::new(1.0_f32, theme.border()),
            );
        }
    }

    action
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tab_row_height_constant() {
        assert_eq!(TAB_ROW_H, 35.0);
    }

    #[test]
    fn test_tab_item_properties() {
        let item = TabItem {
            title: "Rust Architecture",
            is_dirty: true,
            is_active: false,
        };
        assert_eq!(item.title, "Rust Architecture");
        assert!(item.is_dirty);
        assert!(!item.is_active);
    }

    #[test]
    fn test_tab_auto_scroll_clamps() {
        let mut scroll = 100.0f32;
        let viewport_w = 400.0f32;
        let total_w = 300.0f32;
        let max_scroll = (total_w - viewport_w).max(0.0);
        scroll = scroll.clamp(0.0, max_scroll);
        assert_eq!(scroll, 0.0);
    }
}
