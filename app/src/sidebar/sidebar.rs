//! Sleek floating sidebar with navigation and user notes library.

#[path = "body.rs"]
pub mod body;
#[path = "footer.rs"]
pub mod footer;
#[path = "header.rs"]
pub mod header;

#[allow(unused_imports)]
pub use body::render_sidebar_body;
#[allow(unused_imports)]
pub use footer::render_sidebar_footer;
#[allow(unused_imports)]
pub use header::render_sidebar_header;

use core::Note;
use eframe::egui::{self, vec2, Color32, Rect, Stroke};
use crate::workspace::{WorkspaceDialog, WorkspaceState};

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
    let sidebar_w = sb_rect.width();

    // Sidebar card styling matches the text area / editor card:
    // 5px corner radius, identical background opacity/blur handling, and focus outline
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

    let sb_origin = sb_rect.min + vec2(16.0, 18.0);

    // 1. Sidebar Header (Branding & Stats view)
    let header_action = header::render_sidebar_header(
        ui,
        painter,
        sb_origin,
        sidebar_w,
        active_mode_idx,
        theme,
        any_modal_open,
    );

    // 2. Sidebar Body (File explorer / documents)
    let body_action = body::render_sidebar_body(
        ui,
        painter,
        sb_rect,
        sb_origin,
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
        sb_rect,
        theme,
        any_modal_open,
    );

    if any_modal_open {
        None
    } else {
        header_action.or(body_action).or(footer_action)
    }
}
