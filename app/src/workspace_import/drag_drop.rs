//! Drag-and-drop event listener and visual drop-target overlay.

use super::state::WorkspaceImporter;
use super::worker::spawn_import_worker;
use crate::ui::theme::Theme;
use eframe::egui::{self, pos2, Align2, Color32, FontId, Rect, Stroke};
use std::path::PathBuf;

/// Inspects egui raw input for dropped files and folders.
/// When files are dropped, starts the background import and displays the progress modal.
pub fn handle_drag_and_drop(ctx: &egui::Context, importer: &mut WorkspaceImporter) -> bool {
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

    if !valid_paths.is_empty() {
        start_workspace_import(valid_paths, importer, ctx.clone());
        true
    } else {
        false
    }
}

/// Initiates background import and opens the modal.
pub fn start_workspace_import(
    paths: Vec<PathBuf>,
    importer: &mut WorkspaceImporter,
    ctx: egui::Context,
) {
    importer.is_modal_open = true;
    spawn_import_worker(
        paths,
        importer.stats.clone(),
        importer.status.clone(),
        importer.cancel_token.clone(),
        importer.is_running.clone(),
        ctx,
    );
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
        "📥 DROP TO IMPORT VAULT",
        FontId::proportional(20.0),
        theme.accent,
    );

    painter.text(
        pos2(center.x, center.y + 16.0),
        Align2::CENTER_CENTER,
        "Folders & notes will be embedded smoothly into MindForge (.md, .txt)",
        FontId::monospace(12.0),
        theme.muted,
    );
}
