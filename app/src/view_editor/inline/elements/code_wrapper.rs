//! Unified code block card container rendering and metrics.
//!
//! Renders a single cohesive elevated surface card around the entire code block
//! (from opening fence to closing fence) with rounded corners and a language badge pill,
//! perfectly matching the look of the Markdown preview and eliminating broken line strips.

use crate::ui::theme::Theme;
use eframe::egui::{
    pos2, vec2, Align2, Color32, CornerRadius, FontId, Painter, Rect, Stroke, StrokeKind,
};

/// Renders a single cohesive elevated container card around an entire code block
/// with a distinct, sleek header bar, subtle divider hairline, and language badge pill.
pub fn render_code_block_card(
    painter: &Painter,
    top_y: f32,
    bottom_y: f32,
    text_left: f32,
    content_right: f32,
    theme: &Theme,
    fence_lang: Option<&str>,
) {
    let card_left = text_left;
    let card_right = content_right;
    let card_rect = Rect::from_min_max(pos2(card_left, top_y), pos2(card_right, bottom_y));

    // 1. Single cohesive elevated container surface with subtle rounded border
    painter.rect(
        card_rect,
        5.0,
        theme.surface(),
        Stroke::new(1.0_f32, theme.border()),
        StrokeKind::Inside,
    );

    // 2. Distinct, sleek header bar at the top of the code card (28px height)
    let header_h = 28.0;
    if bottom_y >= top_y + header_h {
        let header_rect =
            Rect::from_min_max(pos2(card_left, top_y), pos2(card_right, top_y + header_h));
        let header_bg = Color32::from_rgba_unmultiplied(
            theme.border().r(),
            theme.border().g(),
            theme.border().b(),
            if theme.is_light() { 22 } else { 35 },
        );
        painter.rect_filled(
            header_rect,
            CornerRadius {
                nw: 5,
                ne: 5,
                sw: 0,
                se: 0,
            },
            header_bg,
        );

        // Subtle hairline divider separating header bar from code area
        let divider_color = Color32::from_rgba_unmultiplied(
            theme.border().r(),
            theme.border().g(),
            theme.border().b(),
            if theme.is_light() { 65 } else { 75 },
        );
        painter.line_segment(
            [
                pos2(card_left, top_y + header_h),
                pos2(card_right, top_y + header_h),
            ],
            Stroke::new(0.8_f32, divider_color),
        );
    }

    // 3. Language badge on the left of the header bar
    if let Some(lang) = fence_lang {
        if !lang.is_empty() {
            let label_font = FontId::monospace(10.0);
            let label_pos = pos2(card_rect.min.x + 14.0, top_y + 14.0);
            painter.text(
                label_pos,
                Align2::LEFT_CENTER,
                lang.to_uppercase(),
                label_font,
                theme.text,
            );
        }
    }
}

/// Returns the hit-test and render rectangle for a code block copy button.
pub fn code_block_copy_button_rect(card_right: f32, top_y: f32) -> Rect {
    let btn_w = 60.0;
    let btn_h = 20.0;
    Rect::from_min_size(
        pos2(card_right - btn_w - 12.0, top_y + 4.0),
        vec2(btn_w, btn_h),
    )
}
