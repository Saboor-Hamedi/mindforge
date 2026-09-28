//! Sidebar panel state for MINDFORGE's floating notes sidebar.
//!
//! Tracks visibility, focus, selection, width, and drag state for the
//! left-hand navigation sidebar (Ctrl+B). All fields are grouped here
//! to keep the main `App` struct manageable.

/// State for the floating left sidebar (notes navigator).
#[derive(Clone)]
pub struct SidebarState {
    /// Whether the sidebar panel is currently visible
    pub open: bool,
    /// Whether the sidebar has keyboard focus (for j/k navigation)
    pub focused: bool,
    /// Index of the currently highlighted note in the list
    pub selected_idx: usize,
    /// Current width in pixels (clamped to MIN/MAX_SIDEBAR_W)
    pub width: f32,
    /// Whether the user is dragging the splitter to resize
    pub dragging_splitter: bool,
    /// Flag set when the sidebar needs to scroll the selected item into view
    pub needs_scroll: bool,
}

impl Default for SidebarState {
    fn default() -> Self {
        Self {
            open: false,
            focused: false,
            selected_idx: 0,
            width: 260.0,
            dragging_splitter: false,
            needs_scroll: false,
        }
    }
}

impl SidebarState {
    /// Creates a new sidebar state with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Toggles sidebar visibility.
    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    /// Opens the sidebar and gives it focus.
    pub fn open(&mut self) {
        self.open = true;
        self.focused = true;
    }

    /// Closes the sidebar and removes focus.
    pub fn close(&mut self) {
        self.open = false;
        self.focused = false;
    }
}
