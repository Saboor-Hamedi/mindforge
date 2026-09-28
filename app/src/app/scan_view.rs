//! Security and vulnerability scan views: active scan report and scan history.

use super::App;
use crate::mode::Mode;
use eframe::egui::{Painter, Rect, Ui};

impl App {
    /// Renders either the active vulnerability scan report or past scans history log.
    pub fn render_scan_panes(
        &mut self,
        ui: &mut Ui,
        painter: &Painter,
        editor_panel_rect: Rect,
    ) {
        match self.misc.mode {
            Mode::ScanReport => {
                crate::views::scan::render_scan_view(
                    ui,
                    painter,
                    editor_panel_rect,
                    self.scan.active_scan_result.as_ref(),
                    self.scan.active_scan_error.as_ref().map(|(u, e)| (u.as_str(), e.as_str())),
                    &mut self.scan.scan_report_scroll_y,
                    &self.misc.theme,
                    self.misc.font_size,
                );
            }
            Mode::ScanHistory => {
                let opened_idx = crate::views::scan_history::render_scan_history(
                    ui,
                    painter,
                    editor_panel_rect,
                    &self.scan.past_scans,
                    &mut self.scan.scan_history_selected,
                    &mut self.scan.scan_history_scroll_y,
                    &self.misc.theme,
                    self.misc.font_size,
                );
                if let Some(idx) = opened_idx {
                    if let Some(record) = self.scan.past_scans.get(idx) {
                        let findings: Vec<webscan::Finding> = serde_json::from_str(&record.findings_json).unwrap_or_default();
                        let result = webscan::ScanResult {
                            url: record.url.clone(),
                            status_code: 200,
                            response_time_ms: 0,
                            tls: None,
                            server_header: None,
                            page_size_bytes: 0,
                            note: record.note.clone(),
                            findings,
                        };
                        self.scan.active_scan_result = Some(result);
                        self.scan.active_scan_error = None;
                        self.scan.scan_report_scroll_y = 0.0;
                        self.misc.mode = Mode::ScanReport;
                    }
                }
            }
            _ => {}
        }
    }
}
