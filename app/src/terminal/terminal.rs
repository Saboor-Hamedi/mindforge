//! Top-level terminal coordinator and shared types.
//!
//! [`TerminalPane`] owns the collection of [`TerminalSession`]s and the
//! active-session bookkeeping. Rendering, input, events, and session
//! management are implemented in the sibling modules as `impl TerminalPane`
//! blocks so each concern stays in its own file.

use eframe::egui;
use egui_term::TerminalTheme;

use crate::ui::theme::{Theme, ThemeKind};

use super::process::spawn_backend_session;
use super::render::build_terminal_palette;
use super::session::TerminalSession;

/// Actions that can be triggered from the terminal pane UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalAction {
    None,
    Close,
    RequestFocus,
}

/// The embedded terminal pane: a set of PTY sessions plus the active one.
pub struct TerminalPane {
    pub sessions: Vec<TerminalSession>,
    pub active_session_idx: usize,
    pub next_session_id: u64,
    pub sessions_sidebar_w: f32,
    pub is_dragging_sessions_splitter: bool,
    pub sessions_scroll_y: f32,
    /// Directory new sessions should start in (the workspace root when open).
    pub working_dir: Option<std::path::PathBuf>,
    pub(crate) last_theme_kind: Option<ThemeKind>,
    pub(crate) cached_theme: Option<TerminalTheme>,
}

impl TerminalPane {
    /// Spawns the pane with a single initial session using the given theme.
    /// `working_dir` is the directory every session starts in — typically the
    /// active workspace root so shells open inside the dropped/opened project.
    pub fn spawn(
        ctx: &egui::Context,
        theme: &Theme,
        working_dir: Option<std::path::PathBuf>,
    ) -> anyhow::Result<Self> {
        let first_session = spawn_backend_session(1, ctx, 1, working_dir.clone())?;
        let palette = build_terminal_palette(theme);
        let term_theme = TerminalTheme::new(Box::new(palette));

        Ok(Self {
            sessions: vec![first_session],
            active_session_idx: 0,
            next_session_id: 2,
            sessions_sidebar_w: 160.0,
            is_dragging_sessions_splitter: false,
            sessions_scroll_y: 0.0,
            working_dir,
            last_theme_kind: Some(theme.kind),
            cached_theme: Some(term_theme),
        })
    }
}
