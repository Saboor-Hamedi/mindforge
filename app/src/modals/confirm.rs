//! Generic, sleek Confirmation dialog modal.

use crate::ui::theme::Theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Rect, Stroke};

#[allow(dead_code)]
pub struct ConfirmModalAction {
    pub confirmed: bool,
    pub should_close: bool,
}

#[allow(dead_code)]
pub fn render_confirm_modal(
    ui: &egui::Ui,
    painter: &egui::Painter,
    bounds: Rect,
    title: &str,
    message: &str,
    confirm_label: &str,
    theme: &Theme,
    just_opened: bool,
) -> ConfirmModalAction {
    let backdrop_alpha = if theme.is_light() { 90 } else { 160 };
    painter.rect_filled(bounds, 0.0, Color32::from_black_alpha(backdrop_alpha));

    let modal_w = 460.0;
    let modal_h = 175.0;
    let modal_rect = Rect::from_center_size(bounds.center(), vec2(modal_w, modal_h));

    painter.rect(
        modal_rect,
        10.0,
        theme.surface(),
        Stroke::new(1.0_f32, theme.border()),
        egui::StrokeKind::Inside,
    );

    let m_origin = modal_rect.min + vec2(24.0, 22.0);

    painter.text(
        m_origin,
        Align2::LEFT_TOP,
        title,
        FontId::monospace(15.0),
        theme.highlight,
    );

    painter.text(
        m_origin + vec2(0.0, 28.0),
        Align2::LEFT_TOP,
        message,
        FontId::monospace(12.5),
        theme.text,
    );

    let btn_h = 36.0;
    let btn_y = modal_rect.max.y - btn_h - 18.0;

    let confirm_w = 120.0;
    let cancel_w = 100.0;
    let confirm_rect = Rect::from_min_size(pos2(modal_rect.max.x - 24.0 - confirm_w, btn_y), vec2(confirm_w, btn_h));
    let cancel_rect = Rect::from_min_size(pos2(modal_rect.max.x - 24.0 - confirm_w - 12.0 - cancel_w, btn_y), vec2(cancel_w, btn_h));

    let cancel_hover = ui.rect_contains_pointer(cancel_rect);
    let confirm_hover = ui.rect_contains_pointer(confirm_rect);

    let (enter, esc) = if just_opened {
        (false, false)
    } else {
        ui.input(|i| (
            i.key_pressed(egui::Key::Enter),
            i.key_pressed(egui::Key::Escape),
        ))
    };

    let mut action = ConfirmModalAction {
        confirmed: false,
        should_close: false,
    };

    // Cancel button
    let (cancel_bg, cancel_stroke, cancel_fg) = if theme.is_light() {
        if cancel_hover {
            (Color32::from_rgb(228, 231, 238), theme.border(), theme.highlight)
        } else {
            (Color32::from_rgb(241, 243, 247), theme.border(), theme.muted)
        }
    } else {
        if cancel_hover {
            (Color32::from_rgb(32, 34, 44), Color32::from_gray(80), Color32::WHITE)
        } else {
            (Color32::from_rgb(24, 25, 32), Color32::from_gray(50), theme.muted)
        }
    };

    painter.rect(
        cancel_rect,
        10.0,
        cancel_bg,
        Stroke::new(1.0_f32, cancel_stroke),
        egui::StrokeKind::Inside,
    );
    painter.text(
        cancel_rect.center(),
        Align2::CENTER_CENTER,
        "Cancel (Esc)",
        FontId::monospace(11.5),
        cancel_fg,
    );

    // Confirm button
    let (confirm_bg, confirm_stroke, confirm_fg) = if theme.is_light() {
        if confirm_hover {
            (theme.accent, theme.highlight, Color32::WHITE)
        } else {
            (theme.accent, theme.accent, Color32::WHITE)
        }
    } else {
        if confirm_hover {
            (theme.accent, Color32::WHITE, theme.bg)
        } else {
            (theme.accent, theme.accent, theme.bg)
        }
    };

    painter.rect(
        confirm_rect,
        10.0,
        confirm_bg,
        Stroke::new(1.0_f32, confirm_stroke),
        egui::StrokeKind::Inside,
    );
    painter.text(
        confirm_rect.center(),
        Align2::CENTER_CENTER,
        confirm_label,
        FontId::monospace(12.0),
        confirm_fg,
    );

    if (confirm_hover && ui.input(|i| i.pointer.primary_clicked())) || enter {
        action.confirmed = true;
        action.should_close = true;
    } else if (cancel_hover && ui.input(|i| i.pointer.primary_clicked())) || esc {
        action.should_close = true;
    }

    action
}
