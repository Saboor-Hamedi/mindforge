//! Drag-and-drop event listener and visual drop-target overlay.

use crate::ui::theme::Theme;
use eframe::egui::{self, pos2, Align2, Color32, FontId, Rect, Stroke};

/// Opens the dropped directory, or the containing directory of dropped files, as a workspace.
pub fn handle_drag_and_drop(
    ctx: &egui::Context,
    workspace: &mut crate::workspace::WorkspaceState,
) -> bool {
    let dropped = ctx.input(|i| i.raw.dropped_files.clone());
    if dropped.is_empty() {
        return false;
    }

    let mut valid_paths = Vec::new();
    for item in dropped {
        if let Some(path) = item.path {
            if path.exists() {
                valid_paths.push(path);
            }
        }
    }

    if valid_paths.is_empty() {
        return false;
    }
    let candidate = if valid_paths.len() == 1 && valid_paths[0].is_dir() {
        valid_paths[0].clone()
    } else {
        valid_paths[0]
            .parent()
            .unwrap_or(&valid_paths[0])
            .to_path_buf()
    };
    workspace.open(candidate).is_ok()
}

/// Renders a sleek floating drop indicator when files are being hovered over the application window.
pub fn render_hover_indicator(
    ctx: &egui::Context,
    painter: &egui::Painter,
    bounds: Rect,
    theme: &Theme,
) {
    let has_hovered = ctx.input(|i| !i.raw.hovered_files.is_empty());
    if !has_hovered {
        return;
    }

    // Semi-transparent backdrop overlay
    let overlay_bg = Color32::from_black_alpha(140);
    painter.rect_filled(bounds, 5.0, overlay_bg);

    // Accent dashed/subtle border around inner area
    let inner_rect = bounds.shrink(28.0);
    painter.rect_stroke(
        inner_rect,
        8.0,
        Stroke::new(2.0_f32, theme.accent),
        egui::StrokeKind::Inside,
    );

    // Centered label & icon
    let center = bounds.center();
    painter.text(
        pos2(center.x, center.y - 18.0),
        Align2::CENTER_CENTER,
        "📂 DROP TO OPEN WORKSPACE",
        FontId::proportional(20.0),
        theme.accent,
    );

    painter.text(
        pos2(center.x, center.y + 16.0),
        Align2::CENTER_CENTER,
        "MindForge will read and edit the files directly on disk",
        FontId::monospace(12.0),
        theme.muted,
    );
}
