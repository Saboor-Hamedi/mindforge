//! Right-side pane state for MINDFORGE's preview, AI agent, and sidebar panels.
//!
//! Manages the right-hand panel that can display:
//! - Markdown live preview
//! - AI assistant chat
//! - Document outline
//! - Backlinks
//!
//! Also tracks the right sidebar (outline/backlinks) visibility and width.

/// State for the right-side pane and right sidebar.
#[derive(Clone)]
pub struct RightPaneState {
    /// Which tab is active in the right pane
    pub tab: crate::app::RightPaneTab,
    /// Whether the AI assistant has requested keyboard focus
    pub ai_focus_requested: bool,
    /// Whether the right sidebar (outline/backlinks) is visible
    pub sidebar_open: bool,
    /// Width of the right sidebar in pixels
    pub sidebar_width: f32,
    /// Internal state for the right sidebar (outline tree, backlinks list)
    pub sidebar_state: crate::rightsidebar::RightSidebarState,
}

impl Default for RightPaneState {
    fn default() -> Self {
        Self {
            tab: crate::app::RightPaneTab::Preview,
            ai_focus_requested: false,
            sidebar_open: false,
            sidebar_width: 280.0,
            sidebar_state: crate::rightsidebar::RightSidebarState::default(),
        }
    }
}

impl RightPaneState {
    /// Creates a new right pane state with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Toggles the right sidebar visibility.
    pub fn toggle_sidebar(&mut self) {
        self.sidebar_open = !self.sidebar_open;
    }

    /// Switches to a specific right pane tab.
    pub fn switch_tab(&mut self, tab: crate::app::RightPaneTab) {
        self.tab = tab;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_right_pane_toggle() {
        let mut rp = RightPaneState::new();
        assert!(!rp.sidebar_open);
        rp.toggle_sidebar();
        assert!(rp.sidebar_open);
    }

    #[test]
    fn test_right_pane_switch_tab() {
        let mut rp = RightPaneState::new();
        assert_eq!(rp.tab, crate::app::RightPaneTab::Preview);
        rp.switch_tab(crate::app::RightPaneTab::AiAgent);
        assert_eq!(rp.tab, crate::app::RightPaneTab::AiAgent);
    }
}
