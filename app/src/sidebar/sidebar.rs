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

pub enum SidebarAction {
    SwitchMode(usize),
    LoadNote { id: i64, topic: String, body: String, index: usize },
    DeleteNote(i64),
    NewNote,
    OpenSettings,
    ToggleNotesLimit,
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
) -> Option<SidebarAction> {
    let sidebar_w = sb_rect.width();

    // Translucent sidebar background matching desktop backdrop blur
    let sb_bg = Color32::from_rgba_unmultiplied(
        theme.sidebar_bg().r(),
        theme.sidebar_bg().g(),
        theme.sidebar_bg().b(),
        (opacity * 255.0) as u8,
    );
    painter.rect_filled(sb_rect, 0.0, sb_bg);

    // Subtle focus divider indicator: glows with accent when sidebar is focused
    let (divider_w, divider_color) = if sidebar_focused {
        (1.5, theme.accent.gamma_multiply(0.75))
    } else {
        (1.0, theme.border())
    };
    painter.line_segment(
        [sb_rect.right_top(), sb_rect.right_bottom()],
        Stroke::new(divider_w, divider_color),
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

    // 2. Sidebar Body (File explorer / SQLite documents)
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
