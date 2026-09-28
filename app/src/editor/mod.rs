mod editing;
pub mod backend;
pub mod controller;
pub mod events;
pub mod types;

mod lines;
mod movement;
mod selection;
mod undo;
mod visual;

#[cfg(test)]
mod tests;

pub use crate::types::{EditorSnapshot, VisualLine};
pub use visual::caret_cell;

#[derive(Default, Clone)]
pub struct Editor {
    pub buf: Vec<char>,
    pub cur: usize,
    pub selection: Option<usize>,
    pub selection_inclusive: bool,
    pub undo_stack: Vec<EditorSnapshot>,
    pub redo_stack: Vec<EditorSnapshot>,
    pub desired_col: Option<usize>,
}

impl Editor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    pub fn clear(&mut self) {
        if !self.buf.is_empty() {
            self.save_undo_snapshot();
        }
        self.buf.clear();
        self.cur = 0;
        self.selection = None;
        self.selection_inclusive = false;
        self.desired_col = None;
    }

    pub fn clear_history(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    pub fn set_text(&mut self, s: &str) {
        self.buf = s.chars().collect();
        self.cur = 0;
        self.selection = None;
        self.selection_inclusive = false;
        self.desired_col = None;
    }

    pub fn text(&self) -> String {
        self.buf.iter().collect()
    }

    pub fn row_col(&self) -> (usize, usize) {
        self.row_col_of(self.cur)
    }

    pub fn row_col_of(&self, idx: usize) -> (usize, usize) {
        let (mut row, mut col) = (0, 0);
        let end = idx.min(self.buf.len());
        for &c in &self.buf[..end] {
            if c == '\n' {
                row += 1;
                col = 0;
            } else {
                col += 1;
            }
        }
        (row, col)
    }

    /// Returns the (start, end) char indices of the line at 0-based `row`, excluding trailing newline.
    pub fn line_char_range(&self, row: usize) -> (usize, usize) {
        let mut cur_row = 0;
        let mut start = 0;
        for (i, &c) in self.buf.iter().enumerate() {
            if cur_row == row {
                let mut end = i;
                while end < self.buf.len() && self.buf[end] != '\n' {
                    end += 1;
                }
                return (i, end);
            }
            if c == '\n' {
                cur_row += 1;
                start = i + 1;
            }
        }
        if cur_row == row {
            return (start, self.buf.len());
        }
        (self.buf.len(), self.buf.len())
    }

    /// Returns the text content of the line at 0-based `row`, excluding trailing newline.
    #[allow(dead_code)]
    pub fn line_text(&self, row: usize) -> String {
        let (s, e) = self.line_char_range(row);
        self.buf[s..e].iter().collect()
    }
}
