//! Notes list and active note state for MINDFORGE's document management.
//!
//! Tracks the currently active note, the full notes list loaded from SQLite,
//! and sidebar display limits. The notes list is reloaded from the database
//! whenever changes occur (create, delete, rename, import).

use core::Note;

/// State for the active note and notes list.
#[derive(Clone)]
pub struct NotesState {
    /// ID of the currently active note (None if no note is open)
    pub active_note_id: Option<i64>,
    /// Title of the currently active note
    pub active_note_title: String,
    /// Full list of notes loaded from SQLite
    pub notes_list: Vec<Note>,
    /// Maximum number of notes to display in the sidebar
    pub sidebar_notes_limit: usize,
    /// Total number of notes in the database
    pub total_notes_count: usize,
}

impl Default for NotesState {
    fn default() -> Self {
        Self {
            active_note_id: None,
            active_note_title: "Untitled Note".to_string(),
            notes_list: Vec::new(),
            sidebar_notes_limit: 50,
            total_notes_count: 0,
        }
    }
}

impl NotesState {
    /// Creates a new notes state with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the active note by ID and title.
    pub fn set_active(&mut self, id: i64, title: &str) {
        self.active_note_id = Some(id);
        self.active_note_title = title.to_string();
    }

    /// Clears the active note.
    pub fn clear_active(&mut self) {
        self.active_note_id = None;
        self.active_note_title = "Untitled Note".to_string();
    }

    /// Toggles the sidebar notes limit between 50 and 100.
    pub fn toggle_notes_limit(&mut self) {
        self.sidebar_notes_limit = if self.sidebar_notes_limit >= 100 { 50 } else { 100 };
    }
}
