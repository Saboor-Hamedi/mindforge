//! Tab bar and document tab state for MINDFORGE's multi-note interface.
//!
//! Manages two parallel tab systems:
//! - **Note tabs**: Open editor buffers with their own scroll position
//! - **Doc tabs**: Documentation reader tabs (built-in guides)
//!
//! Also tracks the doc sidebar (right-hand panel for doc navigation)
//! and scroll offsets for both tab strips.

/// State for all tab bars and tab-related UI.
#[derive(Clone)]
pub struct TabsState {
    // Note tabs
    /// Currently active note tab index
    pub active_tab: usize,
    /// Previously active note tab (for Ctrl+Tab cycling)
    pub last_active_tab: usize,
    /// Horizontal scroll offset for the note tab strip
    pub tab_scroll_offset: f32,

    // Doc tabs (documentation reader)
    /// Indices of open documentation tabs
    pub open_doc_tabs: Vec<usize>,
    /// Currently active doc tab index
    pub active_doc_tab: usize,
    /// Previously active doc tab
    pub last_active_doc_tab: usize,
    /// Horizontal scroll offset for the doc tab strip
    pub doc_tab_scroll_offset: f32,

    // Doc sidebar (right-hand panel)
    /// Index of the currently selected doc in the sidebar
    pub active_doc_idx: usize,
    /// Whether the doc sidebar has keyboard focus
    pub doc_sidebar_focused: bool,
    /// Selected item index within the doc sidebar
    pub doc_selected_idx: usize,
}

impl Default for TabsState {
    fn default() -> Self {
        Self {
            active_tab: 0,
            last_active_tab: 0,
            tab_scroll_offset: 0.0,
            open_doc_tabs: vec![0],
            active_doc_tab: 0,
            last_active_doc_tab: 0,
            doc_tab_scroll_offset: 0.0,
            active_doc_idx: 0,
            doc_sidebar_focused: false,
            doc_selected_idx: 0,
        }
    }
}

impl TabsState {
    /// Creates a new tabs state with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Switches to the next note tab (wraps around).
    pub fn next_tab(&mut self, count: usize) {
        if count == 0 {
            return;
        }
        self.last_active_tab = self.active_tab;
        self.active_tab = (self.active_tab + 1) % count;
    }

    /// Switches to the previous note tab (wraps around).
    pub fn prev_tab(&mut self, count: usize) {
        if count == 0 {
            return;
        }
        self.last_active_tab = self.active_tab;
        self.active_tab = if self.active_tab == 0 {
            count - 1
        } else {
            self.active_tab - 1
        };
    }

    /// Switches to a specific note tab by index.
    pub fn goto_tab(&mut self, idx: usize, count: usize) {
        if idx < count {
            self.last_active_tab = self.active_tab;
            self.active_tab = idx;
        }
    }
}
