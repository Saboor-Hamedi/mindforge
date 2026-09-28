//! Split pane tab bar header and close button for the right-hand panel.

use crate::ui::theme::Theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Rect, Stroke};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RightPaneAction {
    SelectTab(crate::app::RightPaneTab),
    Close,
}

/// Renders the unified tab bar header at the top of the right-hand split pane.
/// Styled with the exact same height (35px), borders, and active tab indicator as the editor tabs.
pub fn render_right_pane_header(
    ui: &egui::Ui,
    painter: &egui::Painter,
    header_rect: Rect,
    active_tab: crate::app::RightPaneTab,
    theme: &Theme,
    backlinks_count: usize,
) -> Option<RightPaneAction> {
    let mut action = None;

    // Bottom divider line separating tabs from pane content (identical to editor tab bar)
    painter.line_segment(
        [
            pos2(header_rect.min.x, header_rect.max.y),
            pos2(header_rect.max.x, header_rect.max.y),
        ],
        Stroke::new(1.0_f32, theme.border()),
    );

    let chip_margin_y = 4.0;
    let tab_h = header_rect.height() - chip_margin_y * 2.0;
    let tab_y = header_rect.min.y + chip_margin_y;

    let bl_label = if backlinks_count > 0 {
        format!("Links ({})", backlinks_count)
    } else {
        "Backlinks".to_string()
    };

    let tabs: [(crate::app::RightPaneTab, &str); 4] = [
        (crate::app::RightPaneTab::Preview, "Preview"),
        (crate::app::RightPaneTab::AiAgent, "AI Agent"),
        (crate::app::RightPaneTab::Backlinks, bl_label.as_str()),
        (crate::app::RightPaneTab::Outline, "Outline"),
    ];

    let mut current_x = header_rect.min.x + 8.0;

    for (tab_kind, label) in tabs {
        let is_active = active_tab == tab_kind;
        let text_w = painter.layout_no_wrap(label.to_owned(), FontId::proportional(11.5), Color32::WHITE).size().x;
        let tab_w = (text_w + 34.0).max(78.0);
        let tab_rect = Rect::from_min_size(pos2(current_x, tab_y), vec2(tab_w, tab_h));
        let is_hovered = ui.rect_contains_pointer(tab_rect);

        if is_hovered {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        if is_active {
            // Discrete bottom accent indicator pill
            let accent_w = (tab_rect.width() - 16.0).max(18.0);
            let accent_bar = Rect::from_center_size(
                pos2(tab_rect.center().x, tab_rect.max.y - 1.5),
                vec2(accent_w, 2.0),
            );
            painter.rect_filled(accent_bar, 1.0, theme.accent);
        } else if is_hovered {
            let bg = if theme.is_light() {
                Color32::from_rgba_unmultiplied(0, 0, 0, 10)
            } else {
                Color32::from_rgba_unmultiplied(255, 255, 255, 12)
            };
            painter.rect_filled(tab_rect, 5.0, bg);
        }

        let text_color = if is_active {
            theme.text
        } else if is_hovered {
            theme.text.lerp_to_gamma(theme.muted, 0.3)
        } else {
            theme.muted
        };

        let icon_color = if is_active { theme.accent } else { text_color };
        let icon_center = pos2(tab_rect.min.x + 13.0, tab_rect.center().y);

        // Crisp vector icons: never depend on OS emoji fonts or unicode tofu
        match tab_kind {
            crate::app::RightPaneTab::Preview => {
                // Vector eye icon (almond outline + pupil)
                let stroke = Stroke::new(1.3_f32, icon_color);
                let p_l = pos2(icon_center.x - 5.5, icon_center.y);
                let p_r = pos2(icon_center.x + 5.5, icon_center.y);
                let p_t = pos2(icon_center.x, icon_center.y - 3.2);
                let p_b = pos2(icon_center.x, icon_center.y + 3.2);
                painter.line_segment([p_l, p_t], stroke);
                painter.line_segment([p_t, p_r], stroke);
                painter.line_segment([p_r, p_b], stroke);
                painter.line_segment([p_b, p_l], stroke);
                painter.circle_filled(icon_center, 1.5, icon_color);
            }
            crate::app::RightPaneTab::AiAgent => {
                // Vector AI 4-point sparkle star
                let c = icon_center;
                let pts = [
                    pos2(c.x, c.y - 5.0),
                    pos2(c.x + 1.3, c.y - 1.3),
                    pos2(c.x + 5.0, c.y),
                    pos2(c.x + 1.3, c.y + 1.3),
                    pos2(c.x, c.y + 5.0),
                    pos2(c.x - 1.3, c.y + 1.3),
                    pos2(c.x - 5.0, c.y),
                    pos2(c.x - 1.3, c.y - 1.3),
                ];
                painter.add(egui::Shape::convex_polygon(pts.to_vec(), icon_color, Stroke::NONE));
            }
            crate::app::RightPaneTab::Backlinks => {
                // Vector chain link icon
                let stroke = Stroke::new(1.4_f32, icon_color);
                let c = icon_center;
                painter.line_segment([pos2(c.x - 3.5, c.y + 3.5), pos2(c.x + 3.5, c.y - 3.5)], stroke);
                painter.circle_stroke(pos2(c.x - 2.5, c.y + 2.5), 2.5, stroke);
                painter.circle_stroke(pos2(c.x + 2.5, c.y - 2.5), 2.5, stroke);
            }
            crate::app::RightPaneTab::Outline => {
                // Vector outline list bars
                let stroke = Stroke::new(1.3_f32, icon_color);
                let c = icon_center;
                painter.line_segment([pos2(c.x - 4.5, c.y - 4.0), pos2(c.x + 4.5, c.y - 4.0)], stroke);
                painter.line_segment([pos2(c.x - 4.5, c.y), pos2(c.x + 2.5, c.y)], stroke);
                painter.line_segment([pos2(c.x - 4.5, c.y + 4.0), pos2(c.x + 0.5, c.y + 4.0)], stroke);
            }
        }

        painter.text(
            pos2(tab_rect.min.x + 24.0, tab_rect.center().y),
            Align2::LEFT_CENTER,
            label,
            FontId::proportional(11.5),
            text_color,
        );

        if is_hovered && ui.input(|i| i.pointer.primary_clicked()) {
            action = Some(RightPaneAction::SelectTab(tab_kind));
        }

        current_x += tab_w + 8.0;
    }

    // Close button (×) on far right of header
    let close_size = 20.0;
    let close_rect = Rect::from_center_size(
        pos2(header_rect.max.x - 18.0, header_rect.center().y),
        vec2(close_size, close_size),
    );
    if crate::ui_components::render_close_button_rect(ui, painter, close_rect, theme, "right_pane_close") {
        action = Some(RightPaneAction::Close);
    }

    action
}
