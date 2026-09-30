//! Writing Rhythm 14-day activity bar chart.

use crate::ui::theme::Theme;
use core::DailyActivity;
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Rect, Sense, Stroke};

pub fn render_rhythm_chart(
    ui: &mut egui::Ui,
    avail_w: f32,
    history: &[DailyActivity],
    today_date: &str,
    yesterday_date: &str,
    theme: &Theme,
    scale: f32,
) {
    let chart_box_h = 136.0 * scale;
    let (chart_container, _) = ui.allocate_exact_size(vec2(avail_w, chart_box_h), Sense::hover());
    let p = ui.painter();

    // Chart outer container card
    p.rect(
        chart_container,
        5.0,
        theme.surface(),
        Stroke::new(1.0_f32, theme.border()),
        egui::StrokeKind::Inside,
    );

    p.text(
        pos2(chart_container.min.x + 16.0 * scale, chart_container.min.y + 14.0 * scale),
        Align2::LEFT_TOP,
        "WRITING RHYTHM (RECENT DAYS)",
        FontId::monospace(11.5 * scale),
        theme.highlight,
    );

    // Take up to 14 days
    let display_days: Vec<&DailyActivity> = history.iter().take(14).collect();
    let num_bars = display_days.len().max(1);
    let bar_area_w = chart_container.width() - 32.0 * scale;
    let bar_gap = 8.0 * scale;
    let bar_w = ((bar_area_w - (num_bars as f32 - 1.0) * bar_gap) / num_bars as f32)
        .clamp(14.0 * scale, 36.0 * scale);

    let max_chart_h = 60.0 * scale;
    let chart_base_y = chart_container.min.y + 104.0 * scale;

    let max_secs = display_days
        .iter()
        .map(|a| a.active_seconds)
        .max()
        .unwrap_or(60)
        .max(60) as f32;

    for (i, act) in display_days.iter().enumerate() {
        let bx = chart_container.min.x + 16.0 * scale + i as f32 * (bar_w + bar_gap);
        let h = ((act.active_seconds as f32 / max_secs) * max_chart_h).max(3.0);
        let bar_rect = Rect::from_min_max(pos2(bx, chart_base_y - h), pos2(bx + bar_w, chart_base_y));

        let is_today = act.date == today_date;
        let is_active = act.active_seconds > 0;

        let fill = if is_today {
            theme.highlight
        } else if is_active {
            theme.highlight
        } else if theme.is_light() {
            Color32::from_rgba_unmultiplied(theme.muted.r(), theme.muted.g(), theme.muted.b(), 35)
        } else {
            Color32::from_rgb(24, 25, 30)
        };

        p.rect_filled(bar_rect, 2.5, fill);

        // Minutes badge on top of bar
        if act.active_seconds >= 60 {
            p.text(
                pos2(bx + bar_w * 0.5, chart_base_y - h - 12.0 * scale),
                Align2::CENTER_TOP,
                format!("{}m", act.active_seconds / 60),
                FontId::monospace((9.0 * scale).max(9.5)),
                if is_today { theme.highlight } else { Color32::from_gray(160) },
            );
        }

        // Date label beneath bar
        let date_label = if is_today {
            "Today"
        } else if act.date == yesterday_date {
            "Yest"
        } else {
            act.date.split('-').last().unwrap_or(&act.date)
        };

        p.text(
            pos2(bx + bar_w * 0.5, chart_base_y + 4.0 * scale),
            Align2::CENTER_TOP,
            date_label,
            FontId::monospace((9.0 * scale).max(9.5)),
            if is_today { theme.highlight } else { Color32::from_gray(120) },
        );
    }
}
