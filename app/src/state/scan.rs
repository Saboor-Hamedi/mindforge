//! Web security scan state for MINDFORGE's `:scan` and `:scans` commands.
//!
//! Manages the full scan lifecycle:
//! - Initiating a scan (spawns background thread)
//! - Polling for results via channel
//! - Storing the active scan result/error
//! - Browsing scan history from disk
//! - Scroll positions for both report and history views

/// State for web security scanning and scan history.
pub struct ScanState {
    /// Mode to return to when exiting the scan report view
    pub prev_mode_before_scan: crate::mode::Mode,
    /// URL currently being scanned (None when idle)
    pub scan_in_progress: Option<String>,
    /// Channel receiver for scan results from the background thread
    pub scan_rx: Option<std::sync::mpsc::Receiver<Result<webscan::ScanResult, String>>>,
    /// The most recent successful scan result
    pub active_scan_result: Option<webscan::ScanResult>,
    /// The most recent scan error as (url, error_message)
    pub active_scan_error: Option<(String, String)>,
    /// Scroll position for the scan report view
    pub scan_report_scroll_y: f32,
    /// Past scans loaded from storage
    pub past_scans: Vec<core::ScanRecord>,
    /// Selected index in the scan history list
    pub scan_history_selected: usize,
    /// Scroll position for the scan history view
    pub scan_history_scroll_y: f32,
}

impl Default for ScanState {
    fn default() -> Self {
        Self {
            prev_mode_before_scan: crate::mode::Mode::Normal,
            scan_in_progress: None,
            scan_rx: None,
            active_scan_result: None,
            active_scan_error: None,
            scan_report_scroll_y: 0.0,
            past_scans: Vec::new(),
            scan_history_selected: 0,
            scan_history_scroll_y: 0.0,
        }
    }
}

impl ScanState {
    /// Creates a new scan state with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true if a scan is currently in progress.
    pub fn is_scanning(&self) -> bool {
        self.scan_in_progress.is_some()
    }

    /// Clears the current scan result and error.
    pub fn clear_result(&mut self) {
        self.active_scan_result = None;
        self.active_scan_error = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_state() {
        let mut scan = ScanState::new();
        assert!(!scan.is_scanning());
        scan.scan_in_progress = Some("https://example.com".to_string());
        assert!(scan.is_scanning());
        scan.clear_result();
        assert!(scan.active_scan_result.is_none());
    }
}
