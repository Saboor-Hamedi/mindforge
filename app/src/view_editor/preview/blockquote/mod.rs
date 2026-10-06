//! Preview blockquote rendering module providing 1:1 visual parity with Editor blockquotes.
//!
//! Reuses the exact geometry, accent pill metrics, background luminance steps,
//! and typography colors established by the inline editor.

use crate::ui::theme::Theme;
use crate::view_editor::inline::elements::quote_color;
use crate::view_editor::preview::build_inline_job;
use eframe::egui::{pos2, Color32, CornerRadius, Painter, Rect};

/// Renders a markdown blockquote with exact 1:1 parity to the editor's blockquote card wrapper.
/// Supports multi-level nested quotes with inward-stepping accent bars and subtle surface elevation.
///
/// Returns the vertical height consumed by the rendered blockquote.
pub fn render_preview_blockquote(
    painter: &Painter,
    content_painter: &Painter,
    start_x: f32,
    current_y: f32,
    max_text_w: f32,
    font_size: f32,
    depth: usize,
    text: &str,
    theme: &Theme,
    viewport_rect: Rect,
) -> f32 {
    let effective_depth = depth.clamp(1, 5);
    let bar_w = 2.5;
    let bar_pitch = 7.0; // pitch between multiple nested bars (4.5px gap between > > >)
    let bar_gap = 2.0;
    let bar_total_w = (effective_depth as f32) * bar_pitch;

    // Pin left edge to match editor card_left exactly (10px padding outward to hug the gutter)
    let card_left = start_x - 10.0;
    let content_right = start_x + max_text_w;
    let text_x = card_left + bar_gap + bar_total_w + 8.0;
    let available_text_w = (content_right - text_x - 12.0).max(40.0);

    // Text color matches editor's quote_color exactly across all themes
    let text_color = quote_color(theme);
    let job = build_inline_job(text, font_size, text_color, theme, available_text_w);
    let galley = painter.layout_job(job);
    let text_h = galley.size().y;
    let box_h = text_h.max(font_size * 1.55) + 8.0;

    // Viewport frustum culling
    if current_y + box_h >= viewport_rect.min.y && current_y <= viewport_rect.max.y {
        // 1. Elevated callout card surface matching editor's luminance step
        let bg = theme.bg;
        let step_base: i16 = if theme.is_light() { -8 } else { 12 };
        let step: i16 = step_base
            + if theme.is_light() {
                -(effective_depth as i16 * 2)
            } else {
                effective_depth as i16 * 2
            };
        let bg_color = Color32::from_rgb(
            (bg.r() as i16 + step).clamp(0, 255) as u8,
            (bg.g() as i16 + step).clamp(0, 255) as u8,
            (bg.b() as i16 + step).clamp(0, 255) as u8,
        );
        let card_rect = Rect::from_min_max(
            pos2(card_left, current_y),
            pos2(content_right, current_y + box_h),
        );
        content_painter.rect_filled(card_rect, CornerRadius::same(5), bg_color);

        // 2. Sleek vertical accent pill(s) stepping inward to the right
        let bar_pad_y = 3.0;
        for lvl in 0..effective_depth {
            let bx = card_left + bar_gap + (lvl as f32 * bar_pitch);
            let bar_rect = Rect::from_min_max(
                pos2(bx, current_y + bar_pad_y),
                pos2(bx + bar_w, current_y + box_h - bar_pad_y),
            );
            let is_innermost = lvl + 1 == effective_depth;
            let alpha: u8 = if is_innermost { 165 } else { 120 };
            let bar_color = Color32::from_rgba_unmultiplied(
                theme.accent.r(),
                theme.accent.g(),
                theme.accent.b(),
                alpha,
            );
            content_painter.rect_filled(bar_rect, CornerRadius::same(2), bar_color);
        }

        // 3. Render quote text galley
        content_painter.galley(pos2(text_x, current_y + 4.0), galley, text_color);
    }

    box_h + 6.0
}
