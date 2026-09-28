//! Learning statistics, writing analytics, and daily activity story view.
//!
//! Displays a comprehensive dashboard of the user's writing activity:
//! - Hero metric cards (time in editor, words written, keystrokes, notes)
//! - 14-day writing rhythm bar chart
//! - Chronological activity journal with per-day breakdowns
//!
//! The view is fully responsive — it scales typography, spacing, and layout
//! based on available width. All rendering is done with direct painter calls
//! (no egui widgets) for maximum performance and pixel-perfect control.

use crate::ui::theme::Theme;
use core::DailyActivity;
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Rect, Sense, Stroke};

fn format_duration(seconds: u32) -> String {
    if seconds < 60 {
        format!("{}s", seconds)
    } else if seconds < 3600 {
        format!("{}m", seconds / 60)
    } else {
        let hrs = seconds / 3600;
        let mins = (seconds % 3600) / 60;
        if mins > 0 {
            format!("{}h {}m", hrs, mins)
        } else {
            format!("{}h", hrs)
        }
    }
}

fn format_duration_u64(seconds: u64) -> String {
    if seconds < 60 {
        format!("{}s", seconds)
    } else if seconds < 3600 {
        format!("{}m", seconds / 60)
    } else {
        let hrs = seconds as f64 / 3600.0;
        format!("{:.1}h", hrs)
    }
}

fn format_number(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

/// Renders the full statistics view inside the given rectangle.
///
/// # Arguments
/// * `ui` — egui UI context
/// * `editor_rect` — Screen area allocated to the stats panel
/// * `today_activity` — Today's activity record (time, keystrokes, words, notes)
/// * `history` — Historical daily activity records (most recent first)
/// * `lifetime` — Tuple of (total_seconds, total_keystrokes, total_words, active_days)
/// * `total_notes` — Total number of notes in the vault
/// * `theme` — Active color theme
/// * `today_date` — Today's date string (YYYY-MM-DD) for highlighting
/// * `yesterday_date` — Yesterday's date string for labeling
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
    // ── Task 2: Generous padding inset from the panel edges ─────────────────
    let pad_left = 28.0;
    let pad_right = 28.0;
    let pad_top = 26.0;

    let content_rect = Rect::from_min_max(
        pos2(editor_rect.min.x + pad_left, editor_rect.min.y + pad_top),
        pos2(editor_rect.max.x - pad_right, editor_rect.max.y),
    );

    // Task 1: No outer border or background is drawn around the Stats view.
    // It shares the editor panel's surface.
    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(content_rect), |ui| {
        egui::ScrollArea::vertical()
            .id_salt("stats_scroll_view")
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                let avail_w = ui.available_width().max(360.0);

                // ── Task 3: Scale typography and dimensions with available width ──
                let scale = (avail_w / 900.0).clamp(1.0, 1.35);

                // ── 1. Header Section ─────────────────────────────────────────
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

                // Task 4: Breathing room between header and metric cards
                ui.add_space(16.0 * scale);

                // ── 2. Responsive Hero Summary Cards ─────────────────────────
                let cards_data = [
                    (
                        "⏱ TIME IN EDITOR",
                        format_duration(today_activity.active_seconds),
                        format!("{} all-time", format_duration_u64(lifetime.0)),
                    ),
                    (
                        "✍ WORDS WRITTEN",
                        format_number(today_activity.words_written as u64),
                        format!("{} total words", format_number(lifetime.2)),
                    ),
                    (
                        "⌨ KEYSTROKES",
                        format_number(today_activity.keystrokes as u64),
                        format!("{} total keys", format_number(lifetime.1)),
                    ),
                    (
                        "📚 NOTES & STREAK",
                        format!("{} notes", total_notes),
                        format!("{} active days", lifetime.3),
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

                // Task 4: Breathing room between metric cards and chart
                ui.add_space(28.0 * scale);

                // ── 3. Writing Rhythm (14-Day Activity Bar Chart) ────────────
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
                        theme.accent
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
                        if is_today { theme.accent } else { Color32::from_gray(120) },
                    );
                }

                // Task 4: Breathing room between chart and activity journal
                ui.add_space(28.0 * scale);

                // ── 4. Chronological Activity Journal Cards ──────────────────
                let journal_hdr_h = 24.0 * scale;
                let (story_hdr_rect, _) = ui.allocate_exact_size(vec2(avail_w, journal_hdr_h), Sense::hover());
                ui.painter().text(
                    story_hdr_rect.min,
                    Align2::LEFT_TOP,
                    "ACTIVITY JOURNAL & STORY",
                    FontId::monospace(12.5 * scale),
                    theme.highlight,
                );

                // Task 4: Breathing room between journal heading and first row
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

                        // Task 5: Row corner radius stays at 4.0, background and border stay the same
                        p.rect(
                            row_rect,
                            4.0,
                            if is_today {
                                Color32::from_rgba_unmultiplied(theme.accent.r(), theme.accent.g(), theme.accent.b(), if theme.is_light() { 22 } else { 38 })
                            } else {
                                theme.surface()
                            },
                            Stroke::new(1.0_f32, if is_today { theme.accent } else { theme.border() }),
                            egui::StrokeKind::Inside,
                        );

                        // Date tag with increased left padding
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
                            if is_today { theme.accent } else { theme.text },
                        );

                        // Flexible positioning depending on available width
                        if avail_w >= 600.0 {
                            // Full layout with 4 metrics
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
                            // Compact layout for narrow windows
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

                        // Task 4: Vertical space between journal rows
                        ui.add_space(6.0 * scale);
                    }
                }

                // Task 4: Final bottom space (at least 32px of scrollable space below last row)
                ui.add_space(36.0 * scale);
            });
    });
}

// ── Task 6: Metric card layout & padding ──────────────────────────────────
fn draw_metric_card(
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
    p.rect_filled(stripe, egui::CornerRadius { nw: 5, sw: 5, ne: 0, se: 0 }, theme.accent);

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
