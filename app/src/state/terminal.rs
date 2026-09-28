//! Embedded terminal pane state for MINDFORGE's `:term` command.
//!
//! Manages the docked terminal panel at the bottom of the editor:
//! - Visibility and focus state
//! - Split ratio for resizing the terminal pane
//! - The terminal pane itself (PTY sessions, shell detection)
//! - Mode to return to when the terminal closes

/// State for the embedded terminal pane.
pub struct TerminalState {
    /// Whether the terminal pane is currently visible
    pub open: bool,
    /// Height ratio of the terminal pane (0.0–1.0 of available space)
    pub split_ratio: f32,
    /// Whether the user is dragging the splitter to resize
    pub dragging_splitter: bool,
    /// Whether the terminal has keyboard focus
    pub focused: bool,
    /// The terminal pane with PTY sessions (None when closed)
    pub pane: Option<crate::ui::terminal_pane::TerminalPane>,
    /// Mode to restore when the terminal is closed
    pub prev_mode_before_term: crate::mode::Mode,
}

impl Default for TerminalState {
    fn default() -> Self {
        Self {
            open: false,
            split_ratio: 0.35,
            dragging_splitter: false,
            focused: false,
            pane: None,
            prev_mode_before_term: crate::mode::Mode::Normal,
        }
    }
}

impl TerminalState {
    /// Creates a new terminal state with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Opens the terminal pane and gives it focus.
    pub fn open(&mut self) {
        self.open = true;
        self.focused = true;
    }

    /// Closes the terminal pane and removes focus.
    pub fn close(&mut self) {
        self.open = false;
        self.focused = false;
    }

    /// Toggles terminal visibility.
    pub fn toggle(&mut self) {
        if self.open {
            self.close();
        } else {
            self.open();
        }
    }
}
