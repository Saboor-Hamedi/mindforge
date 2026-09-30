//! Command-line bar state for MINDFORGE's `:` prompt.
//!
//! Manages the command dock at the bottom of the editor:
//! - Visibility and input buffer
//! - Command history (persistent, up to 200 entries)
//! - Autocomplete selection and navigation
//! - Prefix character (`:` or `/` for search)

/// State for the command-line bar (`:` prompt).
#[derive(Clone)]
pub struct CommandBarState {
    /// Whether the command bar is currently visible
    pub in_command: bool,
    /// The prefix character that opened the command (`:` or `/`)
    pub prefix: char,
    /// Index of the currently highlighted autocomplete suggestion
    pub selected_idx: usize,
    /// Whether the user has navigated the autocomplete list
    pub navigated: bool,
    /// Command history (most recent last, up to 200 entries)
    pub history: Vec<String>,
}

impl Default for CommandBarState {
    fn default() -> Self {
        Self {
            in_command: false,
            prefix: ':',
            selected_idx: 0,
            navigated: false,
            history: Vec::new(),
        }
    }
}

impl CommandBarState {
    /// Creates a new command bar state with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Opens the command bar with the given prefix character.
    pub fn open(&mut self, prefix: char) {
        self.in_command = true;
        self.prefix = prefix;
        self.selected_idx = 0;
        self.navigated = false;
    }

    /// Closes the command bar.
    pub fn close(&mut self) {
        self.in_command = false;
        self.navigated = false;
    }

    /// Adds a command to history (deduplicated, max 200 entries).
    pub fn push_history(&mut self, cmd: &str) {
        let cmd = cmd.trim();
        if cmd.is_empty() {
            return;
        }
        // Remove duplicate if already present
        self.history.retain(|c| c != cmd);
        self.history.push(cmd.to_string());
        // Keep only the last 200 entries
        if self.history.len() > 200 {
            self.history.remove(0);
        }
    }

    /// Navigates to the next autocomplete suggestion.
    pub fn next_suggestion(&mut self, count: usize) {
        if count == 0 {
            return;
        }
        self.selected_idx = (self.selected_idx + 1) % count;
        self.navigated = true;
    }

    /// Navigates to the previous autocomplete suggestion.
    pub fn prev_suggestion(&mut self, count: usize) {
        if count == 0 {
            return;
        }
        self.selected_idx = if self.selected_idx == 0 {
            count - 1
        } else {
            self.selected_idx - 1
        };
        self.navigated = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_bar_history() {
        let mut cb = CommandBarState::new();
        cb.push_history(":w");
        cb.push_history(":q");
        cb.push_history(":w");
        assert_eq!(cb.history.len(), 2);
        assert_eq!(cb.history[0], ":q");
        assert_eq!(cb.history[1], ":w");
    }

    #[test]
    fn test_command_bar_navigation() {
        let mut cb = CommandBarState::new();
        cb.next_suggestion(3);
        assert_eq!(cb.selected_idx, 1);
        cb.prev_suggestion(3);
        assert_eq!(cb.selected_idx, 0);
    }
}
