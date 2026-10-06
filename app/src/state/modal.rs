//! Modal dialog state for MINDFORGE's overlay panels.
//!
//! Groups all modal-related fields: settings, fuzzy search, rename,
//! delete confirmation, and help panel. Each modal tracks its own
//! visibility, input text, selection index, and scroll position.

use crate::services::fuzzy::SearchItem;
use crate::setting::SettingTab;

/// State for all modal dialogs rendered as overlays on top of the editor.
#[derive(Clone)]
pub struct ModalState {
    // Settings modal (Ctrl+,)
    pub settings_open: bool,
    pub settings_just_opened: bool,
    pub settings_opened_at: f64,
    pub active_setting_tab: SettingTab,
    pub backup_dir: String,
    pub last_backup_status: Option<String>,
    pub keybind_capture: Option<crate::setting::keymap::KeybindCapture>,
    pub keymap: crate::setting::keymap::VimKeymap,

    // Fuzzy search modal (Ctrl+P)
    pub search_open: bool,
    pub search_query: String,
    pub search_results: Vec<SearchItem>,
    pub search_selected: usize,
    pub search_just_opened: bool,
    pub search_opened_at: f64,

    // Rename modal (Ctrl+R)
    pub rename_open: bool,
    pub rename_input: String,
    pub rename_just_opened: bool,

    // Delete confirmation modal (Ctrl+Shift+D or :d)
    pub delete_confirm_open: bool,
    pub delete_just_opened: bool,
    pub pending_delete_note_id: Option<i64>,
    pub pending_delete_path: Option<std::path::PathBuf>,

    // Help panel (:help or F1)
    pub help_open: bool,
    pub help_tab: usize,
    pub help_scroll_y: f32,
    pub help_tab_scroll_offset: f32,
}

impl Default for ModalState {
    fn default() -> Self {
        Self {
            settings_open: false,
            settings_just_opened: false,
            settings_opened_at: 0.0,
            active_setting_tab: SettingTab::Carets,
            backup_dir: String::new(),
            last_backup_status: None,
            keybind_capture: None,
            keymap: crate::setting::keymap::VimKeymap::load_or_init(),
            search_open: false,
            search_query: String::new(),
            search_results: Vec::new(),
            search_selected: 0,
            search_just_opened: false,
            search_opened_at: 0.0,
            rename_open: false,
            rename_input: String::new(),
            rename_just_opened: false,
            delete_confirm_open: false,
            delete_just_opened: false,
            pending_delete_note_id: None,
            pending_delete_path: None,
            help_open: false,
            help_tab: 0,
            help_scroll_y: 0.0,
            help_tab_scroll_offset: 0.0,
        }
    }
}

impl ModalState {
    /// Creates a new modal state with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true if any modal is currently open.
    pub fn any_open(&self) -> bool {
        self.settings_open
            || self.search_open
            || self.rename_open
            || self.delete_confirm_open
            || self.help_open
    }

    /// Closes all modals.
    pub fn close_all(&mut self) {
        self.settings_open = false;
        self.settings_just_opened = false;
        self.search_open = false;
        self.search_just_opened = false;
        self.rename_open = false;
        self.rename_just_opened = false;
        self.delete_confirm_open = false;
        self.delete_just_opened = false;
        self.pending_delete_note_id = None;
        self.pending_delete_path = None;
        self.help_open = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modal_any_open() {
        let mut modal = ModalState::new();
        assert!(!modal.any_open());
        modal.settings_open = true;
        assert!(modal.any_open());
    }

    #[test]
    fn test_modal_close_all() {
        let mut modal = ModalState::new();
        modal.settings_open = true;
        modal.search_open = true;
        modal.rename_open = true;
        modal.delete_confirm_open = true;
        modal.help_open = true;
        assert!(modal.any_open());
        modal.close_all();
        assert!(!modal.any_open());
    }
}
