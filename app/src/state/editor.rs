//! Editor buffer and rendering state for MINDFORGE's text editing engine.
//!
//! Groups all fields related to the editor's visual presentation:
//! - The main and documentation editor buffers
//! - Visual line layout (computed each frame for wrapping)
//! - Scroll positions for editor, docs, and preview
//! - Inline vs raw mode, line numbers, split preview
//! - Font metrics cache and dirty flags

use crate::editor::{Editor, VisualLine};
use eframe::egui::{Pos2, Rect};

/// State for the editor buffers and their visual rendering.
#[derive(Clone)]
pub struct EditorState {
    /// Main editor buffer (the active note)
    pub ed: Editor,
    /// Documentation reader buffer
    pub doc_ed: Editor,
    /// Command-line input buffer (`:` prompt)
    pub cmd_ed: Editor,
    /// Precomputed visual lines for the current frame (word wrapping, inline layout)
    pub visual_lines: Vec<VisualLine>,
    /// Vertical scroll position of the main editor
    pub scroll_y: f32,
    /// Vertical scroll position of the documentation reader
    pub doc_scroll_y: f32,
    /// Vertical scroll position of the markdown preview pane
    pub preview_scroll_y: f32,
    /// Whether the split preview pane is visible
    pub preview_open: bool,
    /// Whether inline (WYSIWYG) mode is active vs raw monospace
    pub inline_mode: bool,
    /// Width ratio of the editor when split with preview (0.0–1.0)
    pub split_ratio: f32,
    /// Whether the user is dragging the preview splitter
    pub dragging_splitter: bool,
    /// Whether line numbers are shown in the gutter
    pub show_line_numbers: bool,
    /// Whether the buffer has unsaved changes
    pub is_dirty: bool,
    /// Whether the font has changed and needs re-application
    pub font_dirty: bool,
    /// Timestamp of the last save operation
    pub last_saved_time: f64,
    /// Cached character cell width and line height (computed once per font size)
    pub cell: Option<(f32, f32)>,
    /// Last known editor rectangle (for layout persistence)
    pub last_editor_rect: Option<Rect>,
    /// Last known editor origin position
    pub last_ed_origin: Option<Pos2>,
    /// Last known editor font size
    pub last_ed_font_size: Option<f32>,
    pub language_selector: crate::language::LanguageSelectorState,
    pub cached_visual_cols: Option<usize>,
    pub cached_buf_len: usize,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            ed: Editor::new(),
            doc_ed: Editor::new(),
            cmd_ed: Editor::new(),
            visual_lines: vec![VisualLine { char_start: 0, char_end: 0 }],
            scroll_y: 0.0,
            doc_scroll_y: 0.0,
            preview_scroll_y: 0.0,
            preview_open: false,
            inline_mode: false,
            split_ratio: 0.5,
            dragging_splitter: false,
            show_line_numbers: true,
            is_dirty: false,
            font_dirty: false,
            last_saved_time: 0.0,
            cell: None,
            last_editor_rect: None,
            last_ed_origin: None,
            last_ed_font_size: None,
            language_selector: crate::language::LanguageSelectorState::default(),
            cached_visual_cols: None,
            cached_buf_len: 0,
        }
    }
}

impl EditorState {
    /// Creates a new editor state with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a reference to the currently active editor buffer.
    pub fn active(&self) -> &Editor {
        &self.ed
    }

    /// Returns a mutable reference to the currently active editor buffer.
    pub fn active_mut(&mut self) -> &mut Editor {
        &mut self.ed
    }

    /// Toggles between inline (WYSIWYG) and raw monospace editing modes.
    pub fn toggle_inline_mode(&mut self) {
        self.inline_mode = !self.inline_mode;
    }

    /// Toggles the split preview pane visibility.
    pub fn toggle_preview(&mut self) {
        self.preview_open = !self.preview_open;
    }

    /// Toggles line number visibility in the gutter.
    pub fn toggle_line_numbers(&mut self) {
        self.show_line_numbers = !self.show_line_numbers;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_toggles() {
        let mut ed = EditorState::new();
        assert!(!ed.inline_mode);
        ed.toggle_inline_mode();
        assert!(ed.inline_mode);
        assert!(!ed.preview_open);
        ed.toggle_preview();
        assert!(ed.preview_open);
        assert!(ed.show_line_numbers);
        ed.toggle_line_numbers();
        assert!(!ed.show_line_numbers);
    }
}
