use crate::editor::types::CursorPosition;
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct GridCell {
    pub text: String,
    pub highlight: u64,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct HighlightStyle {
    pub foreground: Option<u32>,
    pub background: Option<u32>,
    pub special: Option<u32>,
    pub reverse: bool,
    pub underline: bool,
    pub is_visual: bool,
}

#[derive(Debug, Clone, Default)]
pub struct GridState {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<Vec<GridCell>>,
    /// Incremented only for rows changed by Neovim redraw events.
    /// The egui renderer uses this to cache row layout jobs.
    pub row_revision: Vec<u64>,
    pub cursor: CursorPosition,
    pub mode: String,
    pub highlights: HashMap<u64, HighlightStyle>,
    pub command_line: String,
    pub popup_items: Vec<String>,
    pub popup_meta: Vec<(String, String)>,
    /// Documentation / body preview of each completion item.
    pub popup_info: Vec<String>,
    pub message_at: Option<std::time::Instant>,
    pub popup_selected: Option<usize>,
    pub popup_anchor: Option<(usize, usize)>,
    pub message: String,
    /// Errors stay visible for a few seconds instead of vanishing on the next redraw.
    pub message_is_error: bool,
}

impl GridState {
    pub fn resize(&mut self, width: usize, height: usize) {
        let dimensions_changed = self.width != width || self.height != height;
        self.width = width;
        self.height = height;
        self.cells.resize_with(height, Vec::new);
        self.row_revision.resize(height, 1);
        for row in &mut self.cells {
            row.resize_with(width, || GridCell {
                text: " ".into(),
                highlight: 0,
            });
            row.truncate(width);
        }
        self.cells.truncate(height);
        if dimensions_changed {
            for revision in &mut self.row_revision {
                *revision = revision.wrapping_add(1);
            }
        }
    }
}
