//! Sleek floating sidebar with navigation and filesystem workspace explorer.

#[path = "body.rs"]
pub mod body;
#[path = "footer.rs"]
pub mod footer;
#[path = "header.rs"]
pub mod header;
#[path = "icons.rs"]
pub mod icons;

#[allow(unused_imports)]
pub use body::render_sidebar_body;
#[allow(unused_imports)]
pub use footer::render_sidebar_footer;
#[allow(unused_imports)]
pub use header::render_sidebar_header;

use core::Note;
use eframe::egui::{self, pos2, Color32, Rect, Stroke};
use crate::workspace::{WorkspaceDialog, WorkspaceState};

pub const SECTION_GAP: f32 = 5.0;

#[allow(dead_code)]
pub enum SidebarAction {
    SwitchMode(usize),
    LoadNote { id: i64, topic: String, body: String, index: usize },
    DeleteNote(i64),
    NewNote,
    OpenSettings,
    ToggleNotesLimit,
    OpenWorkspace,
    OpenWorkspaceFile(std::path::PathBuf),
    ToggleWorkspaceFolder(std::path::PathBuf),
    WorkspaceCreate(std::path::PathBuf, WorkspaceDialog),
    WorkspaceRename(std::path::PathBuf),
    WorkspaceDelete(std::path::PathBuf),
    WorkspaceMove(Vec<std::path::PathBuf>, std::path::PathBuf),
    WorkspaceReveal(std::path::PathBuf),
    WorkspaceCommit,
    WorkspaceCancel,
    WorkspaceRefresh,
    CloseWorkspace,
}

pub fn render_sidebar(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    sb_rect: Rect,
    active_mode_idx: usize,
    active_note_id: Option<i64>,
    notes: &[Note],
    notes_limit: usize,
    total_notes_count: usize,
    is_dirty: bool,
    theme: &crate::ui::theme::Theme,
    sidebar_selected_idx: usize,
    sidebar_focused: bool,
    opacity: f32,
    sidebar_needs_scroll: bool,
    any_modal_open: bool,
    workspace: &mut WorkspaceState,
    active_file: Option<&std::path::Path>,
) -> Option<SidebarAction> {
    // 1. Sidebar card background and border
    let sb_stroke = if sidebar_focused {
        Stroke::new(1.0, theme.accent.gamma_multiply(0.40))
    } else {
        Stroke::new(1.0, theme.border().gamma_multiply(0.60))
    };
    let sb_bg = if opacity >= 0.99 {
        theme.sidebar_bg()
    } else {
        let alpha = ((opacity * 255.0) as u8).max(225);
        Color32::from_rgba_unmultiplied(
            theme.sidebar_bg().r(),
            theme.sidebar_bg().g(),
            theme.sidebar_bg().b(),
            alpha,
        )
    };
    painter.rect(
        sb_rect,
        5.0,
        sb_bg,
        sb_stroke,
        egui::StrokeKind::Inside,
    );

    // 2. Compute inner boundary and exact 5px section gaps
    let pad_x = 12.0;
    let pad_y = 12.0;
    let inner_rect = Rect::from_min_max(
        pos2(sb_rect.min.x + pad_x, sb_rect.min.y + pad_y),
        pos2(sb_rect.max.x - pad_x, sb_rect.max.y - pad_y),
    );

    // Header height: compact when empty, taller when workspace root has controls
    let header_h = if workspace.root.is_some() { 56.0 } else { 32.0 };
    let header_rect = Rect::from_min_max(
        inner_rect.min,
        pos2(inner_rect.max.x, inner_rect.min.y + header_h),
    );

    let footer_h = 28.0;
    let footer_rect = Rect::from_min_max(
        pos2(inner_rect.min.x, inner_rect.max.y - footer_h),
        inner_rect.max,
    );

    // Body sits strictly between header and footer with a 5px gap on both sides
    let body_top = header_rect.max.y + SECTION_GAP;
    let body_bottom = (footer_rect.min.y - SECTION_GAP).max(body_top);
    let body_rect = Rect::from_min_max(
        pos2(inner_rect.min.x, body_top),
        pos2(inner_rect.max.x, body_bottom),
    );

    // 1. Sidebar Header (Branding, Stats toggle, and Workspace controls)
    let header_action = header::render_sidebar_header(
        ui,
        painter,
        header_rect,
        active_mode_idx,
        theme,
        any_modal_open,
        workspace,
    );

    // 2. Sidebar Body (File explorer tree or clickable empty state)
    let body_action = body::render_sidebar_body(
        ui,
        painter,
        body_rect,
        active_note_id,
        notes,
        notes_limit,
        total_notes_count,
        is_dirty,
        theme,
        sidebar_selected_idx,
        sidebar_focused,
        sidebar_needs_scroll,
        any_modal_open,
        workspace,
        active_file,
    );

    // 3. Sidebar Footer (Round settings button with tooltip)
    let footer_action = footer::render_sidebar_footer(
        ui,
        painter,
        footer_rect,
        theme,
        any_modal_open,
    );

    if any_modal_open {
        None
    } else {
        header_action.or(body_action).or(footer_action)
    }
}
