//! Reusable UI property components, controls, and layout helpers for settings tabs.

use crate::ui::theme::Theme;
use eframe::egui::{self, pos2, Align2, Color32, FontId, Pos2, Rect, Stroke, Ui};

/// Renders a section header with an uppercase category title and a subtle accent underline or dot.
pub fn render_section_header(
    painter: &egui::Painter,
    origin: Pos2,
    width: f32,
    title: &str,
    theme: &Theme,
) -> f32 {
    let title_font = FontId::proportional(12.0);
    let title_color = theme.accent;

    painter.text(
        origin,
        Align2::LEFT_TOP,
        title.to_uppercase(),
        title_font,
        title_color,
    );

    let line_y = origin.y + 18.0;
    let divider_color = Color32::from_rgba_unmultiplied(
        theme.border().r(),
        theme.border().g(),
        theme.border().b(),
        60,
    );
    painter.line_segment(
        [pos2(origin.x, line_y), pos2(origin.x + width, line_y)],
        Stroke::new(1.0, divider_color),
    );

    24.0 // height consumed
}

/// Renders a standardized property row with title, description, and an interactive child area.
pub fn render_property_row<R>(
    ui: &mut Ui,
    title: &str,
    description: &str,
    theme: &Theme,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> R {
    let mut result = None;
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new(title)
                    .size(13.0)
                    .color(theme.text)
                    .strong(),
            );
            if !description.is_empty() {
                ui.label(
                    egui::RichText::new(description)
                        .size(11.0)
                        .color(theme.muted),
                );
            }
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            result = Some(add_contents(ui));
        });
    });
    ui.add_space(8.0);
    result.unwrap()
}

/// Renders a sleek toggle switch property control.
pub fn render_toggle(
    ui: &mut Ui,
    painter: &egui::Painter,
    rect: Rect,
    value: &mut bool,
    theme: &Theme,
) -> bool {
    let response = ui.allocate_rect(rect, egui::Sense::click());
    if response.clicked() {
        *value = !*value;
    }

    let track_bg = if *value {
        theme.accent
    } else if theme.is_light() {
        Color32::from_gray(215)
    } else {
        Color32::from_gray(55)
    };

    let radius = rect.height() * 0.5;
    painter.rect_filled(rect, radius, track_bg);

    let thumb_radius = radius - 2.0;
    let thumb_x = if *value {
        rect.max.x - radius
    } else {
        rect.min.x + radius
    };
    let thumb_center = pos2(thumb_x, rect.center().y);
    painter.circle_filled(thumb_center, thumb_radius, Color32::WHITE);

    response.clicked()
}

/// Renders a card container frame with uniform padding and subtle border stroke.
pub fn render_card_frame(
    painter: &egui::Painter,
    rect: Rect,
    corner_radius: f32,
    theme: &Theme,
) {
    let card_bg = if theme.is_light() {
        Color32::from_white_alpha(180)
    } else {
        Color32::from_rgba_unmultiplied(
            theme.surface().r(),
            theme.surface().g(),
            theme.surface().b(),
            120,
        )
    };
    painter.rect(
        rect,
        corner_radius,
        card_bg,
        Stroke::new(1.0, theme.border()),
        egui::StrokeKind::Inside,
    );
}

/// Renders a labeled property slider.
pub fn render_property_slider(
    ui: &mut Ui,
    label: &str,
    value: &mut f32,
    min: f32,
    max: f32,
    theme: &Theme,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(label)
                .size(12.0)
                .color(theme.text),
        );
        let slider = egui::Slider::new(value, min..=max)
            .show_value(true)
            .trailing_fill(true);
        if ui.add(slider).changed() {
            changed = true;
        }
    });
    changed
}

/// Renders a selection chip/tag button.
pub fn render_choice_chip(
    ui: &mut Ui,
    label: &str,
    is_selected: bool,
    theme: &Theme,
) -> bool {
    let chip_bg = if is_selected {
        theme.accent
    } else if theme.is_light() {
        Color32::from_gray(235)
    } else {
        Color32::from_gray(40)
    };
    let text_color = if is_selected {
        Color32::WHITE
    } else {
        theme.text
    };

    let btn = egui::Button::new(
        egui::RichText::new(label)
            .size(12.0)
            .color(text_color),
    )
    .fill(chip_bg)
    .corner_radius(4.0);

    ui.add(btn).clicked()
}
