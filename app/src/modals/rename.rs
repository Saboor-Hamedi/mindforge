//! Modern, sleek floating Rename modal dialog.
//! Minimalist and distraction-free: only the sleek input modal container, with no extra buttons or labels.

use crate::ui::theme::Theme;
use eframe::egui::{self, vec2, Color32, FontId, Rect, Stroke};

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
    let backdrop_alpha = if theme.is_light() { 70 } else { 140 };
    painter.rect_filled(bounds, 0.0, Color32::from_black_alpha(backdrop_alpha));

    // Sleek floating modal — just a little bigger than the input field itself
    let modal_w = 460.0;
    let modal_h = 46.0;
    let center_pos = egui::pos2(bounds.center().x, bounds.center().y - 40.0);
    let modal_rect = Rect::from_center_size(center_pos, vec2(modal_w, modal_h));

    // Clicking outside closes the modal (ignoring the frame it was opened on)
    let clicked_outside = !just_opened
        && ui.input(|i| i.pointer.primary_clicked())
        && !modal_rect.contains(ui.input(|i| i.pointer.interact_pos().unwrap_or_default()));

    // Sleek modal container with 10px rounded corners and accent border
    let modal_bg = if theme.is_light() {
        Color32::from_rgb(255, 255, 255)
    } else {
        theme.surface()
    };
    painter.rect(
        modal_rect,
        10.0,
        modal_bg,
        Stroke::new(1.2_f32, theme.accent),
        egui::StrokeKind::Inside,
    );

    // Inner input area vertically centered and aligned on the left
    let input_h = 24.0;
    let input_y = modal_rect.center().y - input_h * 0.5;
    let edit_rect = Rect::from_min_size(
        egui::pos2(modal_rect.min.x + 18.0, input_y),
        vec2(modal_w - 36.0, input_h),
    );
    let response = ui.put(
        edit_rect,
        egui::TextEdit::singleline(input_text)
            .font(FontId::monospace(14.0))
            .text_color(theme.text)
            .margin(egui::Margin::symmetric(0, 2))
            .frame(false),
    );

    if just_opened {
        response.request_focus();
        let mut state =
            egui::text_edit::TextEditState::load(ui.ctx(), response.id).unwrap_or_default();
        let char_count = input_text.chars().count();
        state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::two(
                egui::text::CCursor::new(0),
                egui::text::CCursor::new(char_count),
            )));
        state.store(ui.ctx(), response.id);
    } else if !response.has_focus() && !ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        response.request_focus();
    }

    let (enter, esc) = if just_opened {
        (false, false)
    } else {
        ui.input(|i| {
            (
                i.key_pressed(egui::Key::Enter),
                i.key_pressed(egui::Key::Escape),
            )
        })
    };

    let mut action = RenameModalAction {
        confirmed_title: None,
        should_close: false,
    };

    if enter {
        let trimmed = input_text.trim();
        if !trimmed.is_empty() {
            action.confirmed_title = Some(trimmed.to_string());
        }
        action.should_close = true;
    } else if esc || clicked_outside {
        action.should_close = true;
    }

    action
}
