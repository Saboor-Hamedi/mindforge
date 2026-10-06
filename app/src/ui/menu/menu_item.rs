//! Individual menu item row component with aligned icons, labels, and shortcut columns.

use crate::ui::theme::Theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Response, Ui};

pub const ITEM_HEIGHT: f32 = 26.0;
pub const ICON_COL_W: f32 = 22.0;
pub const HORIZONTAL_PAD: f32 = 8.0;

/// Renders a single interactive menu item row.
/// Returns (Response, clicked_boolean).
pub fn render_menu_item(
    ui: &mut Ui,
    width: f32,
    label: &str,
    icon: Option<&str>,
    shortcut: Option<&str>,
    destructive: bool,
    disabled: bool,
    is_keyboard_selected: bool,
    theme: &Theme,
) -> (Response, bool) {
    let (rect, response) = ui.allocate_exact_size(
        vec2(width, ITEM_HEIGHT),
        if disabled {
            egui::Sense::hover()
        } else {
            egui::Sense::click()
        },
    );

    let is_hovered = response.hovered() || is_keyboard_selected;
    let clicked = response.clicked() && !disabled;

    // Hover background (subtle highlight, crisp 0px corners per fix.md)
    if is_hovered && !disabled {
        let hover_bg = if destructive {
            Color32::from_rgba_unmultiplied(239, 68, 68, 25)
        } else if theme.is_light() {
            Color32::from_rgba_unmultiplied(0, 0, 0, 14)
        } else {
            Color32::from_rgba_unmultiplied(255, 255, 255, 14)
        };
        ui.painter().rect_filled(rect, 0.0, hover_bg);
    }

    let painter = ui.painter().with_clip_rect(rect);

    // 1. Icon column (left-aligned)
    let icon_x = rect.min.x + HORIZONTAL_PAD;
    if let Some(ic) = icon {
        let icon_color = if disabled {
            theme.muted.gamma_multiply(0.5)
        } else if destructive && is_hovered {
            Color32::from_rgb(248, 113, 113)
        } else {
            theme.muted
        };

        painter.text(
            pos2(icon_x, rect.center().y),
            Align2::LEFT_CENTER,
            ic,
            FontId::proportional(12.5),
            icon_color,
        );
    }

    // 2. Text label (after icon column)
    let text_x = icon_x + ICON_COL_W;
    let text_color = if disabled {
        theme.muted.gamma_multiply(0.5)
    } else if destructive {
        if is_hovered {
            Color32::from_rgb(248, 113, 113)
        } else {
            Color32::from_rgb(239, 68, 68)
        }
    } else if is_hovered {
        theme.highlight
    } else {
        theme.text
    };

    painter.text(
        pos2(text_x, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(12.5),
        text_color,
    );

    // 3. Shortcut hint column (right-aligned)
    if let Some(sc) = shortcut {
        let sc_x = rect.max.x - HORIZONTAL_PAD;
        let sc_color = if disabled {
            theme.muted.gamma_multiply(0.4)
        } else {
            theme.muted.gamma_multiply(0.85)
        };

        painter.text(
            pos2(sc_x, rect.center().y),
            Align2::RIGHT_CENTER,
            sc,
            FontId::proportional(11.0),
            sc_color,
        );
    }

    (response, clicked)
}
