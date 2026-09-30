//! Hero summary cards displaying editor time, words, keystrokes, and note totals.

use crate::ui::theme::Theme;
use eframe::egui::{self, pos2, vec2, Align2, FontId, Rect, Sense, Stroke};

pub fn render_hero_cards(
    ui: &mut egui::Ui,
    avail_w: f32,
    today_time: &str,
    lifetime_time: &str,
    today_words: &str,
    lifetime_words: &str,
    today_keys: &str,
    lifetime_keys: &str,
    total_notes: usize,
    active_days: usize,
    theme: &Theme,
    scale: f32,
) {
    let cards_data = [
        ("⏱ TIME IN EDITOR", today_time, lifetime_time),
        ("✍ WORDS WRITTEN", today_words, lifetime_words),
        ("⌨ KEYSTROKES", today_keys, lifetime_keys),
        (
            "📚 NOTES & STREAK",
            &format!("{} notes", total_notes),
            &format!("{} active days", active_days),
        ),
    ];

    let gap = 12.0 * scale;
    let is_4_cols = avail_w >= 640.0;
    let card_h = 76.0 * scale;

    let section_h = if is_4_cols {
        card_h
    } else {
        card_h * 2.0 + gap
    };

    let (cards_rect, _) = ui.allocate_exact_size(vec2(avail_w, section_h), Sense::hover());
    let p = ui.painter();

    if is_4_cols {
        let card_w = ((avail_w - 3.0 * gap) / 4.0).max(100.0);
        for (i, (label, val, sub)) in cards_data.iter().enumerate() {
            let c_rect = Rect::from_min_size(
                pos2(cards_rect.min.x + i as f32 * (card_w + gap), cards_rect.min.y),
                vec2(card_w, card_h),
            );
            draw_metric_card(p, c_rect, label, val, sub, theme, scale);
        }
    } else {
        let card_w = ((avail_w - gap) / 2.0).max(100.0);
        for (i, (label, val, sub)) in cards_data.iter().enumerate() {
            let col = i % 2;
            let row = i / 2;
            let c_rect = Rect::from_min_size(
                pos2(
                    cards_rect.min.x + col as f32 * (card_w + gap),
                    cards_rect.min.y + row as f32 * (card_h + gap),
                ),
                vec2(card_w, card_h),
            );
            draw_metric_card(p, c_rect, label, val, sub, theme, scale);
        }
    }
}

pub fn draw_metric_card(
    p: &eframe::egui::Painter,
    rect: Rect,
    label: &str,
    val: &str,
    sub: &str,
    theme: &Theme,
    scale: f32,
) {
    p.rect(
        rect,
        5.0,
        theme.surface(),
        Stroke::new(1.0_f32, theme.border()),
        egui::StrokeKind::Inside,
    );

    // Accent line on left edge: width stays at 3.0
    let stripe = Rect::from_min_size(rect.min, vec2(3.0, rect.height()));
    p.rect_filled(stripe, egui::CornerRadius { nw: 5, sw: 5, ne: 0, se: 0 }, theme.highlight);

    let left_content_pad = 16.0 * scale;

    // Header label
    p.text(
        pos2(rect.min.x + left_content_pad, rect.min.y + 14.0 * scale),
        Align2::LEFT_TOP,
        label,
        FontId::monospace((9.5 * scale).max(10.0)),
        theme.muted,
    );

    // Main value
    p.text(
        pos2(rect.min.x + left_content_pad, rect.min.y + 32.0 * scale),
        Align2::LEFT_TOP,
        val,
        FontId::monospace((17.0 * scale).max(18.0)),
        theme.highlight,
    );

    // Subtitle
    p.text(
        pos2(rect.min.x + left_content_pad, rect.min.y + 58.0 * scale),
        Align2::LEFT_TOP,
        sub,
        FontId::monospace((10.0 * scale).max(10.5)),
        theme.muted,
    );
}
