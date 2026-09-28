//! Progress modal dialog displaying live import metrics, gauge, and background/cancel controls.

use super::state::{ImportStats, ImportStatus, WorkspaceImporter};
use crate::ui::theme::Theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Rect, Stroke, Ui};

pub enum ImportModalAction {
    None,
    RefreshNotes,
}

/// Renders the floating Workspace Import progress modal dialog.
/// Note: Closing the modal does NOT cancel the import. It continues running in the background.
pub fn render_import_modal(
    ui: &mut Ui,
    painter: &egui::Painter,
    bounds: Rect,
    importer: &mut WorkspaceImporter,
    theme: &Theme,
) -> ImportModalAction {
    if !importer.is_modal_open {
        return ImportModalAction::None;
    }

    let mut action = ImportModalAction::None;

    // Dark backdrop overlay
    let backdrop_alpha = if theme.is_light() { 110 } else { 180 };
    painter.rect_filled(bounds, 0.0, Color32::from_black_alpha(backdrop_alpha));

    // Centered modal window
    let modal_w = 540.0f32.min(bounds.width() - 40.0);
    let modal_h = 350.0f32.min(bounds.height() - 40.0);
    let modal_rect = Rect::from_center_size(bounds.center(), vec2(modal_w, modal_h));

    // Modal container surface
    painter.rect(
        modal_rect,
        6.0,
        theme.surface(),
        Stroke::new(1.0_f32, theme.border()),
        egui::StrokeKind::Inside,
    );

    let stats = importer.get_stats();
    let status = importer.get_status();
    let is_active = importer.is_active();

    // ── Header Bar ───────────────────────────────────────────────────────────
    let header_h = 44.0;
    let header_rect = Rect::from_min_size(modal_rect.min, vec2(modal_w, header_h));

    // Header divider
    painter.line_segment(
        [
            pos2(header_rect.min.x, header_rect.max.y),
            pos2(header_rect.max.x, header_rect.max.y),
        ],
        Stroke::new(1.0_f32, theme.border()),
    );

    // Title
    painter.text(
        pos2(header_rect.min.x + 18.0, header_rect.center().y),
        Align2::LEFT_CENTER,
        "VAULT IMPORT",
        FontId::monospace(13.5),
        theme.text,
    );

    // Background running badge
    if is_active {
        let badge_pos = pos2(header_rect.min.x + 130.0, header_rect.center().y);
        painter.text(
            badge_pos,
            Align2::LEFT_CENTER,
            "● RUNNING IN BACKGROUND",
            FontId::monospace(10.0),
            theme.accent,
        );
    }

    // Close button (✕) on top-right: simply closes the modal window without aborting!
    let close_btn_size = vec2(32.0, 32.0);
    let close_btn_rect = Rect::from_center_size(
        pos2(header_rect.max.x - 22.0, header_rect.center().y),
        close_btn_size,
    );
    let close_hovered = ui.rect_contains_pointer(close_btn_rect);
    let close_clicked = close_hovered && ui.input(|i| i.pointer.primary_clicked());

    let close_color = if close_hovered {
        theme.accent
    } else {
        theme.muted
    };
    painter.text(
        close_btn_rect.center(),
        Align2::CENTER_CENTER,
        "✕",
        FontId::monospace(14.0),
        close_color,
    );

    if close_clicked {
        importer.close_modal();
    }

    // ── Main Content Area ───────────────────────────────────────────────────
    let content_rect = Rect::from_min_max(
        pos2(modal_rect.min.x + 20.0, header_rect.max.y + 16.0),
        pos2(modal_rect.max.x - 20.0, modal_rect.max.y - 60.0),
    );

    // Status description line
    let (status_text, status_color): (String, Color32) = match &status {
        ImportStatus::Idle => ("Ready to import".to_string(), theme.muted),
        ImportStatus::Scanning => (
            "Scanning directory tree for .md & .txt files...".to_string(),
            theme.accent,
        ),
        ImportStatus::Importing { current_file } => {
            let label = if current_file.is_empty() {
                "Importing notes into database...".to_string()
            } else {
                format!("Importing: {}", truncate_str(current_file, 38))
            };
            (label, theme.text)
        }
        ImportStatus::Completed { count, total_bytes } => {
            let label = format!(
                "✓ Successfully imported {} notes ({})",
                count,
                ImportStats::format_bytes(*total_bytes)
            );
            (label, theme.accent)
        }
        ImportStatus::Cancelled { count } => {
            let label = format!("Import cancelled (preserved {} inserted notes)", count);
            (label, Color32::from_rgb(239, 68, 68))
        }
        ImportStatus::Error(err) => {
            let label = format!("Error: {}", truncate_str(err, 44));
            (label, Color32::from_rgb(239, 68, 68))
        }
    };

    painter.text(
        pos2(content_rect.min.x, content_rect.min.y + 8.0),
        Align2::LEFT_CENTER,
        &status_text,
        FontId::monospace(12.5),
        status_color,
    );

    // ── Progress Bar Gauge ──────────────────────────────────────────────────
    let bar_y = content_rect.min.y + 36.0;
    let bar_h = 14.0;
    let bar_rect = Rect::from_min_size(pos2(content_rect.min.x, bar_y), vec2(content_rect.width(), bar_h));

    // Bar background
    painter.rect(
        bar_rect,
        3.0,
        theme.bg,
        Stroke::new(1.0_f32, theme.border()),
        egui::StrokeKind::Inside,
    );

    let progress_ratio = if stats.total_files > 0 {
        (stats.inserted_files as f32 / stats.total_files as f32).clamp(0.0, 1.0)
    } else if matches!(status, ImportStatus::Completed { .. }) {
        1.0
    } else {
        0.0
    };

    if progress_ratio > 0.0 {
        let fill_w = bar_rect.width() * progress_ratio;
        let fill_rect = Rect::from_min_size(bar_rect.min, vec2(fill_w, bar_h));
        painter.rect_filled(fill_rect, 3.0, theme.accent);
    }

    // Percentage on right of bar
    let pct_text = format!("{:.0}%", progress_ratio * 100.0);
    painter.text(
        pos2(bar_rect.max.x, bar_y - 12.0),
        Align2::RIGHT_BOTTOM,
        pct_text,
        FontId::monospace(11.0),
        theme.muted,
    );

    // ── Metrics Stat Cards ──────────────────────────────────────────────────
    let stat_top = bar_y + 32.0;
    let col_w = (content_rect.width() - 24.0) / 4.0;

    let stat_items = [
        ("TOTAL FOUND", stats.total_files.to_string()),
        ("INSERTED", stats.inserted_files.to_string()),
        ("REMAINING", stats.remaining_files.to_string()),
        (
            "SIZE",
            format!(
                "{}",
                ImportStats::format_bytes(stats.inserted_bytes)
            ),
        ),
    ];

    for (i, (label, val)) in stat_items.iter().enumerate() {
        let x = content_rect.min.x + i as f32 * (col_w + 8.0);
        let box_rect = Rect::from_min_size(pos2(x, stat_top), vec2(col_w, 58.0));

        // Subdued border box
        painter.rect(
            box_rect,
            4.0,
            theme.bg,
            Stroke::new(1.0_f32, theme.border()),
            egui::StrokeKind::Inside,
        );

        painter.text(
            pos2(box_rect.center().x, box_rect.min.y + 16.0),
            Align2::CENTER_CENTER,
            *label,
            FontId::monospace(9.0),
            theme.muted,
        );

        let val_color = if *label == "INSERTED" && stats.inserted_files > 0 {
            theme.accent
        } else {
            theme.text
        };

        painter.text(
            pos2(box_rect.center().x, box_rect.min.y + 38.0),
            Align2::CENTER_CENTER,
            val,
            FontId::monospace(14.0),
            val_color,
        );
    }

    // ── Footer Controls ─────────────────────────────────────────────────────
    let footer_y = modal_rect.max.y - 48.0;

    if is_active {
        // Cancel button (explicitly aborts worker)
        let cancel_rect = Rect::from_min_size(
            pos2(modal_rect.min.x + 20.0, footer_y),
            vec2(100.0, 32.0),
        );
        let cancel_hov = ui.rect_contains_pointer(cancel_rect);
        if cancel_hov && ui.input(|i| i.pointer.primary_clicked()) {
            importer.cancel();
        }

        let cancel_color = if cancel_hov {
            Color32::from_rgb(239, 68, 68)
        } else {
            theme.muted
        };
        painter.text(
            cancel_rect.center(),
            Align2::CENTER_CENTER,
            "CANCEL IMPORT",
            FontId::monospace(11.0),
            cancel_color,
        );

        // Run in Background button (closes modal only)
        let hide_rect = Rect::from_min_size(
            pos2(modal_rect.max.x - 180.0, footer_y),
            vec2(160.0, 32.0),
        );
        let hide_hov = ui.rect_contains_pointer(hide_rect);
        if hide_hov && ui.input(|i| i.pointer.primary_clicked()) {
            importer.close_modal();
        }

        let hide_color = if hide_hov { theme.accent } else { theme.text };
        painter.text(
            hide_rect.center(),
            Align2::CENTER_CENTER,
            "HIDE (RUNS IN BG) ➔",
            FontId::monospace(11.0),
            hide_color,
        );
    } else {
        // Finished or Cancelled: Done button
        let done_rect = Rect::from_min_size(
            pos2(modal_rect.max.x - 120.0, footer_y),
            vec2(100.0, 32.0),
        );
        let done_hov = ui.rect_contains_pointer(done_rect);
        if done_hov && ui.input(|i| i.pointer.primary_clicked()) {
            importer.close_modal();
            action = ImportModalAction::RefreshNotes;
        }

        let done_color = if done_hov { theme.accent } else { theme.text };
        painter.text(
            done_rect.center(),
            Align2::CENTER_CENTER,
            "DONE",
            FontId::monospace(12.0),
            done_color,
        );
    }

    action
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.chars().count() > max_len {
        let truncated: String = s.chars().take(max_len.saturating_sub(3)).collect();
        format!("{}...", truncated)
    } else {
        s.to_string()
    }
}
