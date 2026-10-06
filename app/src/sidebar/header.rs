//! Sidebar header component: MindForge branding and Stats navigation.

use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Rect, Stroke};
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

    let content_w = sidebar_w - 32.0;

    // 1. Branding Title: Crisp left-aligned header
    painter.text(
        pos2(sb_origin.x + 2.0, sb_origin.y),
        Align2::LEFT_TOP,
        "MINDFORGE",
        FontId::proportional(14.0),
        theme.accent,
    );

    // 2. Navigation: 📊 Stats toggle button (compact, IDE-styled)
    let btn_rect = Rect::from_min_size(
        sb_origin + vec2(0.0, 24.0),
        vec2(content_w, 24.0),
    );
    let is_sel = active_mode_idx == 1;
    let is_hovered = !any_modal_open && ui.rect_contains_pointer(btn_rect);

    if is_hovered || is_sel {
        let btn_bg = if is_sel {
            Color32::from_rgba_unmultiplied(theme.accent.r(), theme.accent.g(), theme.accent.b(), 28)
        } else if theme.is_light() {
            Color32::from_rgba_unmultiplied(0, 0, 0, 10)
        } else {
            Color32::from_rgba_unmultiplied(255, 255, 255, 12)
        };
        painter.rect_filled(btn_rect, 4.0, btn_bg);
        if is_hovered && !any_modal_open && ui.input(|inp| inp.pointer.primary_clicked()) {
            action = Some(SidebarAction::SwitchMode(if is_sel { 0 } else { 1 }));
        }
    }

    painter.text(
        pos2(btn_rect.min.x + 8.0, btn_rect.center().y),
        Align2::LEFT_CENTER,
        "📊 Stats & Activity",
        FontId::proportional(12.0),
        if is_sel { theme.accent } else { theme.text },
    );

    // 3. Crisp horizontal separator dividing Header/Stats from File Explorer
    let sep_y = sb_origin.y + 54.0;
    painter.line_segment(
        [pos2(sb_origin.x, sep_y), pos2(sb_origin.x + content_w, sep_y)],
        Stroke::new(1.0, theme.border().gamma_multiply(0.35)),
    );

    action
}
