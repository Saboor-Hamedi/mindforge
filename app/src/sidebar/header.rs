//! Sidebar header component: MindForge branding and Stats navigation.

use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Rect};
use crate::sidebar::SidebarAction;

use crate::ui::theme::Theme;

/// Renders the top header of the sidebar: branding, shortcut hint, and Stats view button.
pub fn render_sidebar_header(
    ui: &egui::Ui,
    painter: &egui::Painter,
    sb_origin: egui::Pos2,
    sidebar_w: f32,
    active_mode_idx: usize,
    theme: &Theme,
    any_modal_open: bool,
) -> Option<SidebarAction> {
    let mut action = None;

    // 1. Branding Title: Centered horizontally in the sidebar
    let center_x = sb_origin.x + (sidebar_w - 32.0) * 0.5;
    painter.text(
        pos2(center_x, sb_origin.y),
        Align2::CENTER_TOP,
        "MINDFORGE",
        FontId::proportional(15.0),
        theme.accent,
    );

    // 2. Navigation: Only 📊 Stats (Notes is handled directly by Documents explorer below)
    let btn_rect = Rect::from_min_size(
        sb_origin + vec2(0.0, 28.0),
        vec2(sidebar_w - 32.0, 26.0),
    );
    let is_sel = active_mode_idx == 1;
    let is_hovered = !any_modal_open && ui.rect_contains_pointer(btn_rect);

    if is_hovered || is_sel {
        let btn_bg = if is_sel {
            Color32::from_rgba_unmultiplied(theme.accent.r(), theme.accent.g(), theme.accent.b(), 32)
        } else if theme.is_light() {
            Color32::from_rgba_unmultiplied(0, 0, 0, 13)
        } else {
            Color32::from_rgba_unmultiplied(255, 255, 255, 14)
        };
        painter.rect_filled(btn_rect, 5.0, btn_bg);
        if is_hovered && !any_modal_open && ui.input(|inp| inp.pointer.primary_clicked()) {
            action = Some(SidebarAction::SwitchMode(1));
        }
    }

    painter.text(
        pos2(btn_rect.min.x + 10.0, btn_rect.center().y),
        Align2::LEFT_CENTER,
        "📊 Stats",
        FontId::proportional(12.5),
        if is_sel { theme.accent } else { theme.text },
    );

    action
}
