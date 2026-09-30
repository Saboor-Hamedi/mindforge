//! Learning statistics, writing analytics, and daily activity story view.
//!
//! Submodules:
//! - `format`: Duration and number formatting helpers
//! - `cards`: Hero summary cards for editor time, words, keystrokes, and note totals
//! - `chart`: 14-day writing rhythm bar chart
//! - `journal`: Chronological activity journal and per-day writing history

pub mod cards;
pub mod chart;
pub mod format;
pub mod journal;

pub use cards::render_hero_cards;
pub use chart::render_rhythm_chart;
pub use format::{format_duration, format_duration_u64, format_number};
pub use journal::render_activity_journal;

use crate::ui::theme::Theme;
use core::DailyActivity;
use eframe::egui::{self, pos2, vec2, Align2, FontId, Rect, Sense};

/// Renders the full statistics view inside the given rectangle.
pub fn render_stats(
    ui: &mut egui::Ui,
    editor_rect: Rect,
    today_activity: &DailyActivity,
    history: &[DailyActivity],
    lifetime: (u64, u64, u64, usize), // (total_seconds, total_keystrokes, total_words, active_days)
    total_notes: usize,
    theme: &Theme,
    today_date: &str,
    yesterday_date: &str,
) {
    let pad_left = 28.0;
    let pad_right = 28.0;
    let pad_top = 26.0;

    let content_rect = Rect::from_min_max(
        pos2(editor_rect.min.x + pad_left, editor_rect.min.y + pad_top),
        pos2(editor_rect.max.x - pad_right, editor_rect.max.y),
    );

    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(content_rect), |ui| {
        egui::ScrollArea::vertical()
            .id_salt("stats_scroll_view")
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                let avail_w = ui.available_width().max(360.0);
                let scale = (avail_w / 900.0).clamp(1.0, 1.35);

                // 1. Header Section
                let header_h = 56.0 * scale;
                let (header_rect, _) = ui.allocate_exact_size(vec2(avail_w, header_h), Sense::hover());
                let p = ui.painter();

                p.text(
                    header_rect.min,
                    Align2::LEFT_TOP,
                    "WRITING STORY & ACTIVITY",
                    FontId::monospace(19.0 * scale),
                    theme.highlight,
                );
                p.text(
                    header_rect.min + vec2(0.0, 26.0 * scale),
                    Align2::LEFT_TOP,
                    "A living chronicle of your thoughts, focus, and time in MindForge",
                    FontId::monospace(11.5 * scale),
                    theme.muted,
                );

                ui.add_space(16.0 * scale);

                // 2. Responsive Hero Summary Cards
                render_hero_cards(
                    ui,
                    avail_w,
                    &format_duration(today_activity.active_seconds),
                    &format!("{} all-time", format_duration_u64(lifetime.0)),
                    &format_number(today_activity.words_written as u64),
                    &format!("{} total words", format_number(lifetime.2)),
                    &format_number(today_activity.keystrokes as u64),
                    &format!("{} total keys", format_number(lifetime.1)),
                    total_notes,
                    lifetime.3,
                    theme,
                    scale,
                );

                ui.add_space(28.0 * scale);

                // 3. Writing Rhythm (14-Day Activity Bar Chart)
                render_rhythm_chart(
                    ui,
                    avail_w,
                    history,
                    today_date,
                    yesterday_date,
                    theme,
                    scale,
                );

                ui.add_space(28.0 * scale);

                // 4. Chronological Activity Journal Cards
                render_activity_journal(
                    ui,
                    avail_w,
                    history,
                    today_date,
                    yesterday_date,
                    theme,
                    scale,
                );

                ui.add_space(36.0 * scale);
            });
    });
}
