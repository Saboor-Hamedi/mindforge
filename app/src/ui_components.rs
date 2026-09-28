#[path = "ui_components/toggle.rs"]
pub mod toggle;
pub use toggle::{render_toggle, render_toggle_with_label};

#[path = "ui_components/icons.rs"]
pub mod icons;
pub use icons::render_vector_icon;

use eframe::egui::{self, vec2, Align2, Color32, FontId, Pos2, Rect};
use crate::ui::theme::Theme;

/// Renders a close button centered at `center` with width/height `size`.
pub fn render_close_button(
    ui: &egui::Ui,
    painter: &egui::Painter,
    center: Pos2,
    size: f32,
    theme: &Theme,
    id_salt: impl std::hash::Hash,
) -> bool {
    let hit_rect = Rect::from_center_size(center, vec2(size, size));
    render_close_button_rect(ui, painter, hit_rect, theme, id_salt)
}

/// Renders a sleek close button matching tab close styling (4px radius, soft tint hover, × glyph).
pub fn render_close_button_rect(
    ui: &egui::Ui,
    painter: &egui::Painter,
    hit_rect: Rect,
    theme: &Theme,
    _id_salt: impl std::hash::Hash,
) -> bool {
    let hovered = ui.rect_contains_pointer(hit_rect);

    if hovered {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        let bg_color = if theme.is_light() {
            Color32::from_rgba_unmultiplied(0, 0, 0, 16)
        } else {
            Color32::from_rgba_unmultiplied(255, 255, 255, 20)
        };
        painter.rect_filled(hit_rect, 4.0, bg_color);
    }

    let close_color = if hovered {
        Color32::from_rgb(235, 90, 90)
    } else {
        theme.muted
    };

    painter.text(
        hit_rect.center(),
        Align2::CENTER_CENTER,
        "×",
        FontId::proportional(13.0),
        close_color,
    );

    hovered && ui.input(|i| i.pointer.primary_clicked())
}

