//! Chronological activity journal cards and per-day writing history.

use super::format::{format_duration, format_number};
use crate::ui::theme::Theme;
use core::DailyActivity;
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Sense, Stroke};

pub fn render_activity_journal(
    ui: &mut egui::Ui,
    avail_w: f32,
    history: &[DailyActivity],
    today_date: &str,
    yesterday_date: &str,
    theme: &Theme,
    scale: f32,
) {
    let journal_hdr_h = 24.0 * scale;
    let (story_hdr_rect, _) = ui.allocate_exact_size(vec2(avail_w, journal_hdr_h), Sense::hover());
    ui.painter().text(
        story_hdr_rect.min,
        Align2::LEFT_TOP,
        "ACTIVITY JOURNAL & STORY",
        FontId::monospace(12.5 * scale),
        theme.highlight,
    );

    ui.add_space(10.0 * scale);

    if history.is_empty() {
        let (empty_rect, _) = ui.allocate_exact_size(vec2(avail_w, 36.0 * scale), Sense::hover());
        ui.painter().text(
            empty_rect.min + vec2(10.0, 8.0),
            Align2::LEFT_TOP,
            "Start typing your notes to begin your activity story!",
            FontId::monospace(12.0 * scale),
            theme.muted,
        );
    } else {
        let row_h = 34.0 * scale;
        let left_pad = 16.0 * scale;
        let right_pad = 18.0 * scale;

        for act in history {
            let (row_rect, _) = ui.allocate_exact_size(vec2(avail_w, row_h), Sense::hover());
            let p = ui.painter();
            let is_today = act.date == today_date;

            p.rect(
                row_rect,
                4.0,
                if is_today {
                    Color32::from_rgba_unmultiplied(
                        theme.highlight.r(),
                        theme.highlight.g(),
                        theme.highlight.b(),
                        if theme.is_light() { 22 } else { 38 },
                    )
                } else {
                    theme.surface()
                },
                Stroke::new(
                    1.0_f32,
                    if is_today {
                        theme.highlight
                    } else {
                        theme.border()
                    },
                ),
                egui::StrokeKind::Inside,
            );

            let display_date = if is_today {
                format!("★ TODAY ({})", act.date)
            } else if act.date == yesterday_date {
                format!("• YESTERDAY ({})", act.date)
            } else {
                format!("• {}", act.date)
            };

            p.text(
                pos2(row_rect.min.x + left_pad, row_rect.center().y),
                Align2::LEFT_CENTER,
                display_date,
                FontId::monospace((11.0 * scale).max(11.5)),
                if is_today {
                    theme.highlight
                } else {
                    theme.text
                },
            );

            if avail_w >= 600.0 {
                let col2 = row_rect.min.x + avail_w * 0.28;
                let col3 = row_rect.min.x + avail_w * 0.44;
                let col4 = row_rect.min.x + avail_w * 0.62;

                p.text(
                    pos2(col2, row_rect.center().y),
                    Align2::LEFT_CENTER,
                    format!("⏱ {}", format_duration(act.active_seconds)),
                    FontId::monospace((11.0 * scale).max(11.5)),
                    theme.text,
                );

                p.text(
                    pos2(col3, row_rect.center().y),
                    Align2::LEFT_CENTER,
                    format!("✍ {} words", format_number(act.words_written as u64)),
                    FontId::monospace((11.0 * scale).max(11.5)),
                    theme.highlight,
                );

                p.text(
                    pos2(col4, row_rect.center().y),
                    Align2::LEFT_CENTER,
                    format!("⌨ {} keys", format_number(act.keystrokes as u64)),
                    FontId::monospace((11.0 * scale).max(11.5)),
                    theme.muted,
                );

                p.text(
                    pos2(row_rect.max.x - right_pad, row_rect.center().y),
                    Align2::RIGHT_CENTER,
                    format!("{} created · {} saved", act.notes_created, act.notes_edited),
                    FontId::monospace((10.5 * scale).max(11.0)),
                    theme.muted,
                );
            } else {
                let col2 = row_rect.min.x + avail_w * 0.36;
                let col3 = row_rect.min.x + avail_w * 0.62;

                p.text(
                    pos2(col2, row_rect.center().y),
                    Align2::LEFT_CENTER,
                    format!("⏱ {}", format_duration(act.active_seconds)),
                    FontId::monospace((10.5 * scale).max(11.0)),
                    theme.text,
                );

                p.text(
                    pos2(col3, row_rect.center().y),
                    Align2::LEFT_CENTER,
                    format!("✍ {}", format_number(act.words_written as u64)),
                    FontId::monospace((10.5 * scale).max(11.0)),
                    theme.highlight,
                );

                p.text(
                    pos2(row_rect.max.x - (right_pad - 4.0), row_rect.center().y),
                    Align2::RIGHT_CENTER,
                    format!("{} saved", act.notes_edited),
                    FontId::monospace((10.0 * scale).max(10.5)),
                    theme.muted,
                );
            }

            ui.add_space(6.0 * scale);
        }
    }
}
