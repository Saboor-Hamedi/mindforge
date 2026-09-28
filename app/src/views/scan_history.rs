//! History view for past webscans stored in SQLite (`:scans` command).
//!
//! Displays a scrollable list of previous security scans with:
//! - URL and timestamp for each scan
//! - Optional user notes
//! - Keyboard navigation (↑/↓) and mouse click selection
//! - Enter to open the full report, Esc to close
//!
//! Returns the index of the selected scan when the user presses Enter
/// or clicks, allowing the caller to load and display the full report.

use crate::ui::theme::Theme;
use core::ScanRecord;
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Rect, Stroke};

/// Renders the scan history list and handles selection input.
///
/// # Arguments
/// * `ui` — egui UI context for input detection
/// * `painter` — egui painter for drawing
/// * `rect` — Screen area to render within
/// * `scans` — Slice of stored scan records (most recent first)
/// * `selected_idx` — Currently selected index (auto-clamped if out of bounds)
/// * `scroll_y` — Mutable scroll offset (auto-clamped to content bounds)
/// * `theme` — Active color theme
/// * `font_size` — Base font size
///
/// # Returns
/// `Some(index)` if the user selected a scan (Enter/click), `None` otherwise.
pub fn render_scan_history(
    ui: &egui::Ui,
    painter: &egui::Painter,
    rect: Rect,
    scans: &[ScanRecord],
    selected_idx: &mut usize,
    scroll_y: &mut f32,
    theme: &Theme,
    font_size: f32,
) -> Option<usize> {
    let p = painter.with_clip_rect(rect);
    p.rect_filled(rect, 0.0, theme.bg);

    let pad_x = 24.0;
    let pad_y = 20.0;
    let start_x = rect.min.x + pad_x;
    let mut current_y = rect.min.y + pad_y - *scroll_y;
    let max_w = (rect.width() - pad_x * 2.0).max(100.0);

    // Mouse scroll handling
    if ui.rect_contains_pointer(rect) {
        let delta = ui.input(|i| {
            if i.smooth_scroll_delta.y.abs() > 0.001 {
                i.smooth_scroll_delta.y
            } else {
                i.raw_scroll_delta.y * 0.5
            }
        });
        if delta != 0.0 {
            *scroll_y = (*scroll_y - delta).max(0.0);
        }
    }

    // Title
    p.text(
        pos2(start_x, current_y),
        Align2::LEFT_TOP,
        "📜 WEBSCAN HISTORY",
        FontId::monospace(font_size * 1.35),
        theme.accent,
    );
    current_y += (font_size * 1.8).round();

    p.text(
        pos2(start_x, current_y),
        Align2::LEFT_TOP,
        "Select a past scan to view full findings report without rescanning. (↑/↓ or click, Enter to open, Esc to close)",
        FontId::monospace(font_size * 0.90),
        theme.muted,
    );
    current_y += (font_size * 1.5).round();

    // Divider
    p.line_segment(
        [pos2(start_x, current_y), pos2(start_x + max_w, current_y)],
        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(theme.muted.r(), theme.muted.g(), theme.muted.b(), 40)),
    );
    current_y += 16.0;

    if scans.is_empty() {
        p.text(
            pos2(start_x, current_y),
            Align2::LEFT_TOP,
            "No past scans recorded yet. Run `:scan <url>` to perform a scan.",
            FontId::monospace(font_size),
            theme.muted,
        );
        return None;
    }

    let mut open_idx = None;

    if *selected_idx >= scans.len() {
        *selected_idx = scans.len().saturating_sub(1);
    }

    let row_h = (font_size * 2.4).round();

    for (i, scan) in scans.iter().enumerate() {
        let item_rect = Rect::from_min_size(pos2(start_x, current_y), vec2(max_w, row_h));
        let is_selected = i == *selected_idx;
        let is_hovered = ui.rect_contains_pointer(item_rect);

        if is_selected {
            p.rect_filled(
                item_rect,
                4.0,
                Color32::from_rgba_unmultiplied(theme.accent.r(), theme.accent.g(), theme.accent.b(), 28),
            );
            p.rect_stroke(
                item_rect,
                4.0,
                Stroke::new(1.0_f32, theme.accent),
                egui::StrokeKind::Inside,
            );
        } else if is_hovered {
            p.rect_filled(
                item_rect,
                4.0,
                Color32::from_rgba_unmultiplied(theme.muted.r(), theme.muted.g(), theme.muted.b(), 22),
            );
        }

        if is_hovered && ui.input(|inp| inp.pointer.primary_clicked()) {
            *selected_idx = i;
            open_idx = Some(i);
        }

        // Top line: URL + timestamp
        p.text(
            pos2(item_rect.min.x + 12.0, item_rect.min.y + 6.0),
            Align2::LEFT_TOP,
            &scan.url,
            FontId::monospace(font_size * 1.05),
            if is_selected { theme.highlight } else { theme.text },
        );

        p.text(
            pos2(item_rect.max.x - 12.0, item_rect.min.y + 7.0),
            Align2::RIGHT_TOP,
            &scan.scanned_at,
            FontId::monospace(font_size * 0.82),
            theme.muted,
        );

        // Bottom line: Note
        let note_text = scan.note.as_deref().unwrap_or("No notes");
        p.text(
            pos2(item_rect.min.x + 12.0, item_rect.min.y + font_size * 1.3),
            Align2::LEFT_TOP,
            format!("Note: {}", note_text),
            FontId::monospace(font_size * 0.85),
            theme.muted,
        );

        current_y += row_h + 8.0;
    }

    // Clamp scroll
    let total_h = (current_y + *scroll_y - (rect.min.y + pad_y)).max(0.0);
    let max_scroll = (total_h - rect.height() + pad_y * 2.0).max(0.0);
    *scroll_y = scroll_y.clamp(0.0, max_scroll);

    open_idx
}
