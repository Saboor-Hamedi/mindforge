//! Blockquote styling and card wrapper rendering.
//!
//! Renders a minimal, elegant callout wrapper:
//! 1. A soft background surface slightly elevated from the editor canvas.
//! 2. Sleek vertical accent pill(s) on the left edge with rounded caps supporting nested depths.
//! 3. Strictly pinned left margin — nested levels indent INWARD (to the right), never bleeding left.
//! 4. Seamless continuous multi-line callout card without broken line scallops.
//! 5. Seamlessly integrates with active theme and user accent color.

use crate::ui::theme::Theme;
use eframe::egui::{pos2, Color32, CornerRadius, Painter, Rect};

/// Returns the quote text color.
pub fn quote_color(theme: &Theme) -> Color32 {
    if theme.is_light() {
        Color32::from_rgb(
            ((theme.text.r() as u16 * 7 + theme.muted.r() as u16 * 3) / 10) as u8,
            ((theme.text.g() as u16 * 7 + theme.muted.g() as u16 * 3) / 10) as u8,
            ((theme.text.b() as u16 * 7 + theme.muted.b() as u16 * 3) / 10) as u8,
        )
    } else {
        Color32::from_rgb(
            ((theme.text.r() as u16 * 8 + theme.muted.r() as u16 * 2) / 10) as u8,
            ((theme.text.g() as u16 * 8 + theme.muted.g() as u16 * 2) / 10) as u8,
            ((theme.text.b() as u16 * 8 + theme.muted.b() as u16 * 2) / 10) as u8,
        )
    }
}

/// Returns soft indent spaces proportional to nesting depth inside the quote card.
pub fn quote_indent(depth: usize) -> String {
    " ".repeat(depth.saturating_sub(1).min(5) * 2 + 1)
}

/// Renders the blockquote card wrapper (background + left accent bar(s)) for one inline line.
pub fn render_block_quote_wrapper(
    painter: &Painter,
    line_y: f32,
    line_h: f32,
    text_left: f32,
    content_right: f32,
    theme: &Theme,
    is_active: bool,
    depth: usize,
    is_first: bool,
    is_last: bool,
) {
    let effective_depth = depth.clamp(1, 5);
    let bar_w = 2.5;
    let bar_pitch = 7.0; // pitch between multiple nested bars stepping inward to the right (4.5px gap between > > >)
    let bar_gap = 2.0;

    // The left edge of the card is strictly pinned relative to text_left.
    // It NEVER shifts to the left when nesting increases, preventing any bleed into the gutter.
    let card_left = text_left - 10.0;

    // Card background — subtle luminance step from the editor bg
    let bg = theme.bg;
    let step_base: i16 = if theme.is_light() { -8 } else { 12 };
    let step: i16 = step_base + if theme.is_light() { -(effective_depth as i16 * 2) } else { effective_depth as i16 * 2 };
    let bg_color = Color32::from_rgb(
        (bg.r() as i16 + step).clamp(0, 255) as u8,
        (bg.g() as i16 + step).clamp(0, 255) as u8,
        (bg.b() as i16 + step).clamp(0, 255) as u8,
    );

    let card_rect = Rect::from_min_max(
        pos2(card_left, line_y),
        pos2(content_right, line_y + line_h),
    );
    let card_rounding = CornerRadius {
        nw: if is_first { 5 } else { 0 },
        ne: if is_first { 5 } else { 0 },
        sw: if is_last { 5 } else { 0 },
        se: if is_last { 5 } else { 0 },
    };
    painter.rect_filled(card_rect, card_rounding, bg_color);

    // Accent bars — step INWARD to the right for each nesting depth level
    let bar_inset_top = if is_first { 3.0 } else { 0.0 };
    let bar_inset_bottom = if is_last { 3.0 } else { 0.0 };
    let bar_rounding = CornerRadius {
        nw: if is_first { 2 } else { 0 },
        ne: if is_first { 2 } else { 0 },
        sw: if is_last { 2 } else { 0 },
        se: if is_last { 2 } else { 0 },
    };

    for lvl in 0..effective_depth {
        let bx = card_left + bar_gap + (lvl as f32 * bar_pitch);
        let bar_rect = Rect::from_min_max(
            pos2(bx, line_y + bar_inset_top),
            pos2(bx + bar_w, line_y + line_h - bar_inset_bottom),
        );

        let is_innermost = lvl + 1 == effective_depth;
        let alpha: u8 = if is_active {
            if is_innermost { 255 } else { 185 }
        } else if is_innermost {
            165
        } else {
            100
        };

        let bar_color = Color32::from_rgba_unmultiplied(
            theme.accent.r(),
            theme.accent.g(),
            theme.accent.b(),
            alpha,
        );
        painter.rect_filled(bar_rect, bar_rounding, bar_color);
    }
}
