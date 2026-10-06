//! Filesystem drop handling retained under its historic module name.
//! Dropped directories are opened as workspaces directly on the filesystem.

pub mod drag_drop;
pub mod scanner;

pub use drag_drop::{handle_drag_and_drop, render_hover_indicator};
