//! Delete Note confirmation modal dialog.

use crate::ui::theme::Theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Rect, Stroke};

#[allow(dead_code)]
pub struct DeleteModalAction {
    pub confirmed: bool,
    pub should_close: bool,
}

#[allow(dead_code)]
pub fn render_delete_confirm_modal(
    ui: &egui::Ui,
    painter: &egui::Painter,
    bounds: Rect,
    doc_title: &str,
    theme: &Theme,
    just_opened: bool,
) -> DeleteModalAction {
    // Dimmed background overlay
    let backdrop_alpha = if theme.is_light() { 90 } else { 160 };
    painter.rect_filled(bounds, 0.0, Color32::from_black_alpha(backdrop_alpha));

    let modal_w = 480.0;
    let modal_h = 190.0;
    let modal_rect = Rect::from_center_size(bounds.center(), vec2(modal_w, modal_h));

    // Surface container with 10px rounded corners and subtle border
    painter.rect(
        modal_rect,
        10.0,
        theme.surface(),
        Stroke::new(1.0_f32, theme.border()),
        egui::StrokeKind::Inside,
    );

    let m_origin = modal_rect.min + vec2(24.0, 22.0);

    // Red warning pill badge
    let badge_rect = Rect::from_min_size(m_origin, vec2(54.0, 20.0));
    let (badge_bg, badge_text_col) = if theme.is_light() {
        (Color32::from_rgb(254, 226, 226), Color32::from_rgb(185, 28, 28))
    } else {
        (Color32::from_rgb(48, 20, 24), Color32::from_rgb(255, 100, 110))
    };
    painter.rect_filled(badge_rect, 6.0, badge_bg);
    painter.text(
        badge_rect.center(),
        Align2::CENTER_CENTER,
        "DELETE",
        FontId::monospace(10.0),
        badge_text_col,
    );

    // Modal Title
    painter.text(
        m_origin + vec2(64.0, 1.0),
        Align2::LEFT_TOP,
        "Delete Note",
        FontId::monospace(15.0),
        theme.highlight,
    );

    // Truncate note title cleanly if long so it never overflows the container
    let safe_title = if doc_title.trim().is_empty() {
        "Untitled Note".to_string()
    } else if doc_title.chars().count() > 36 {
        let truncated: String = doc_title.chars().take(36).collect();
        format!("{}...", truncated)
    } else {
        doc_title.to_string()
    };

    // Body text - cleanly spaced across dedicated rows
    painter.text(
        m_origin + vec2(0.0, 32.0),
        Align2::LEFT_TOP,
        "Permanently delete this document?",
        FontId::monospace(12.5),
        theme.text,
    );

    let doc_highlight_col = if theme.is_light() {
        Color32::from_rgb(190, 24, 38)
    } else {
        Color32::from_rgb(255, 130, 140)
    };
    painter.text(
        m_origin + vec2(0.0, 52.0),
        Align2::LEFT_TOP,
        format!("\"{}\"", safe_title),
        FontId::monospace(12.0),
        doc_highlight_col,
    );

    painter.text(
        m_origin + vec2(0.0, 72.0),
        Align2::LEFT_TOP,
        "This action cannot be undone.",
        FontId::monospace(11.0),
        theme.muted,
    );

    // Buttons: Cancel (Esc) & Delete (Enter) with 10px rounded styling
    let btn_h = 36.0;
    let btn_y = modal_rect.max.y - btn_h - 18.0;

    let delete_w = 130.0;
    let cancel_w = 105.0;
    let delete_rect = Rect::from_min_size(pos2(modal_rect.max.x - 24.0 - delete_w, btn_y), vec2(delete_w, btn_h));
    let cancel_rect = Rect::from_min_size(pos2(modal_rect.max.x - 24.0 - delete_w - 12.0 - cancel_w, btn_y), vec2(cancel_w, btn_h));

    let cancel_hover = ui.rect_contains_pointer(cancel_rect);
    let delete_hover = ui.rect_contains_pointer(delete_rect);

    let (enter, esc) = if just_opened {
        (false, false)
    } else {
        ui.input(|i| (
            i.key_pressed(egui::Key::Enter),
            i.key_pressed(egui::Key::Escape),
        ))
    };

    let mut action = DeleteModalAction {
        confirmed: false,
        should_close: false,
    };

    // Cancel button
    let (cancel_bg, cancel_stroke, cancel_fg) = if theme.is_light() {
        if cancel_hover {
            (Color32::from_rgb(228, 231, 238), theme.border(), theme.highlight)
        } else {
            (Color32::from_rgb(241, 243, 247), theme.border(), theme.text)
        }
    } else {
        if cancel_hover {
            (Color32::from_rgb(32, 34, 44), Color32::from_gray(80), Color32::WHITE)
        } else {
            (Color32::from_rgb(24, 25, 32), Color32::from_gray(50), Color32::from_gray(180))
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

    // Delete button (Destructive red)
    let (del_bg, del_stroke, del_fg) = if theme.is_light() {
        if delete_hover {
            (Color32::from_rgb(220, 38, 38), Color32::from_rgb(185, 28, 28), Color32::WHITE)
        } else {
            (Color32::from_rgb(239, 68, 68), Color32::from_rgb(220, 38, 38), Color32::WHITE)
        }
    } else {
        if delete_hover {
            (Color32::from_rgb(75, 22, 28), Color32::from_rgb(220, 60, 70), Color32::from_rgb(255, 140, 150))
        } else {
            (Color32::from_rgb(52, 16, 20), Color32::from_rgb(160, 45, 55), Color32::from_rgb(255, 140, 150))
        }
    };

    painter.rect(
        delete_rect,
        10.0,
        del_bg,
        Stroke::new(1.0_f32, del_stroke),
        egui::StrokeKind::Inside,
    );
    painter.text(
        delete_rect.center(),
        Align2::CENTER_CENTER,
        "Delete (Enter)",
        FontId::monospace(11.5),
        del_fg,
    );

    if (delete_hover && ui.input(|i| i.pointer.primary_clicked())) || enter {
        action.confirmed = true;
        action.should_close = true;
    } else if (cancel_hover && ui.input(|i| i.pointer.primary_clicked())) || esc {
        action.should_close = true;
    }

    action
}
