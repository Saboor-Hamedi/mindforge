//! Modern, sleek Rename Document modal dialog.

use crate::ui::theme::Theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Rect, Stroke};

pub struct RenameModalAction {
    pub confirmed_title: Option<String>,
    pub should_close: bool,
}

pub fn render_rename_modal(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    bounds: Rect,
    input_text: &mut String,
    theme: &Theme,
    just_opened: bool,
) -> RenameModalAction {
    let backdrop_alpha = if theme.is_light() { 90 } else { 160 };
    painter.rect_filled(bounds, 0.0, Color32::from_black_alpha(backdrop_alpha));

    let modal_w = 480.0;
    let modal_h = 210.0;
    let modal_rect = Rect::from_center_size(bounds.center(), vec2(modal_w, modal_h));

    // Sleek modal surface container with 10px rounded corners
    painter.rect(
        modal_rect,
        10.0,
        theme.surface(),
        Stroke::new(1.0_f32, theme.border()),
        egui::StrokeKind::Inside,
    );

    let m_origin = modal_rect.min + vec2(24.0, 22.0);

    // Header badge
    let badge_rect = Rect::from_min_size(m_origin, vec2(58.0, 20.0));
    let (badge_bg, badge_fg) = if theme.is_light() {
        (Color32::from_rgb(230, 244, 255), theme.highlight)
    } else {
        (Color32::from_rgba_unmultiplied(theme.accent.r(), theme.accent.g(), theme.accent.b(), 35), theme.accent)
    };
    painter.rect_filled(badge_rect, 6.0, badge_bg);
    painter.text(
        badge_rect.center(),
        Align2::CENTER_CENTER,
        "RENAME",
        FontId::monospace(10.0),
        badge_fg,
    );

    // Title
    painter.text(
        m_origin + vec2(68.0, 1.0),
        Align2::LEFT_TOP,
        "Rename Document",
        FontId::monospace(15.0),
        theme.highlight,
    );

    // Subtitle
    painter.text(
        m_origin + vec2(0.0, 28.0),
        Align2::LEFT_TOP,
        "Enter a new title for this document",
        FontId::monospace(12.0),
        theme.muted,
    );

    // Input Field (Sleek 10px rounded corner)
    let input_rect = Rect::from_min_size(m_origin + vec2(0.0, 52.0), vec2(modal_w - 48.0, 36.0));
    let input_bg = if theme.is_light() {
        Color32::from_rgb(255, 255, 255)
    } else {
        theme.bg
    };
    painter.rect(
        input_rect,
        10.0,
        input_bg,
        Stroke::new(1.0_f32, theme.border()),
        egui::StrokeKind::Inside,
    );

    let edit_rect = input_rect.shrink2(vec2(12.0, 6.0));
    let response = ui.put(
        edit_rect,
        egui::TextEdit::singleline(input_text)
            .font(FontId::monospace(13.5))
            .text_color(theme.text)
            .frame(false),
    );

    if just_opened {
        response.request_focus();
        let mut state = egui::text_edit::TextEditState::load(ui.ctx(), response.id).unwrap_or_default();
        let char_count = input_text.chars().count();
        state.cursor.set_char_range(Some(egui::text::CCursorRange::two(
            egui::text::CCursor::new(0),
            egui::text::CCursor::new(char_count),
        )));
        state.store(ui.ctx(), response.id);
    } else if !response.has_focus() && !ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        response.request_focus();
    }

    let mut action = RenameModalAction {
        confirmed_title: None,
        should_close: false,
    };

    // Action Buttons: Cancel and Sleek OK / Rename button
    // The OK button is slightly bigger than the input (height 38px) with 10px rounding
    let btn_h = 38.0;
    let btn_y = modal_rect.max.y - btn_h - 18.0;

    let ok_w = 120.0;
    let cancel_w = 95.0;
    let ok_rect = Rect::from_min_size(pos2(modal_rect.max.x - 24.0 - ok_w, btn_y), vec2(ok_w, btn_h));
    let cancel_rect = Rect::from_min_size(pos2(modal_rect.max.x - 24.0 - ok_w - 12.0 - cancel_w, btn_y), vec2(cancel_w, btn_h));

    let ok_hover = ui.rect_contains_pointer(ok_rect);
    let cancel_hover = ui.rect_contains_pointer(cancel_rect);

    let (enter, esc) = if just_opened {
        (false, false)
    } else {
        ui.input(|i| (
            i.key_pressed(egui::Key::Enter),
            i.key_pressed(egui::Key::Escape),
        ))
    };

    // Cancel Button (clean muted button, rounded 10px)
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

    // Sleek OK / Rename Button (Accent colored, rounded 10px, slightly larger than input)
    let (ok_bg, ok_stroke, ok_fg) = if theme.is_light() {
        if ok_hover {
            (theme.accent, theme.highlight, Color32::WHITE)
        } else {
            (theme.accent, theme.accent, Color32::WHITE)
        }
    } else {
        if ok_hover {
            (theme.accent, Color32::WHITE, theme.bg)
        } else {
            (theme.accent, theme.accent, theme.bg)
        }
    };
    painter.rect(
        ok_rect,
        10.0,
        ok_bg,
        Stroke::new(1.0_f32, ok_stroke),
        egui::StrokeKind::Inside,
    );
    painter.text(
        ok_rect.center(),
        Align2::CENTER_CENTER,
        "OK (Enter)",
        FontId::monospace(12.5),
        ok_fg,
    );

    // Left subtle hint
    painter.text(
        pos2(m_origin.x, btn_y + btn_h * 0.5),
        Align2::LEFT_CENTER,
        "Press Enter to confirm",
        FontId::monospace(10.5),
        theme.muted,
    );

    let ok_clicked = (ok_hover && ui.input(|i| i.pointer.primary_clicked())) || enter;
    let cancel_clicked = (cancel_hover && ui.input(|i| i.pointer.primary_clicked())) || esc;

    if ok_clicked {
        let trimmed = input_text.trim();
        if !trimmed.is_empty() {
            action.confirmed_title = Some(trimmed.to_string());
        }
        action.should_close = true;
    } else if cancel_clicked {
        action.should_close = true;
    }

    action
}
