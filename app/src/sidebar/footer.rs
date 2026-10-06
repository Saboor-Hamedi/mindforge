//! Sidebar footer component: Round settings icon button with shortcut tooltip.

use crate::sidebar::SidebarAction;
use crate::ui::theme::Theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Rect, Stroke, Ui};

/// Renders the bottom footer of the sidebar inside the allocated footer_rect.
pub fn render_sidebar_footer(
    ui: &mut Ui,
    painter: &egui::Painter,
    footer_rect: Rect,
    theme: &Theme,
    any_modal_open: bool,
) -> Option<SidebarAction> {
    let mut action = None;

    let btn_size = 28.0;
    let btn_rect = Rect::from_min_size(
        pos2(
            footer_rect.min.x + 2.0,
            footer_rect.center().y - btn_size * 0.5,
        ),
        vec2(btn_size, btn_size),
    );

    let sense = if any_modal_open {
        egui::Sense::hover()
    } else {
        egui::Sense::click()
    };
    let resp = ui.allocate_rect(btn_rect, sense);
    let center = btn_rect.center();
    let is_hovered = !any_modal_open && (resp.hovered() || ui.rect_contains_pointer(btn_rect));

    let bg_color = if is_hovered {
        if theme.is_light() {
            Color32::from_rgba_unmultiplied(0, 0, 0, 14)
        } else {
            Color32::from_rgba_unmultiplied(255, 255, 255, 16)
        }
    } else {
        Color32::TRANSPARENT
    };
    let stroke = if is_hovered {
        Stroke::new(1.0_f32, theme.border())
    } else {
        Stroke::NONE
    };
    painter.rect(btn_rect, 4.0, bg_color, stroke, egui::StrokeKind::Inside);

    // Integrated gear icon centered with secondary text color
    painter.text(
        center,
        Align2::CENTER_CENTER,
        "⚙",
        FontId::proportional(15.0),
        if is_hovered { theme.text } else { theme.muted },
    );

    // Hover tooltip showing shortcut
    let resp = resp.on_hover_text("Settings (Ctrl+,)");

    if !any_modal_open
        && (resp.clicked() || (is_hovered && ui.input(|inp| inp.pointer.primary_clicked())))
    {
        action = Some(SidebarAction::OpenSettings);
    }

    action
}
