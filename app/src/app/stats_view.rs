//! Daily writing statistics, lifetime metrics, and activity history heatmap view.

use super::App;
use crate::views::stats::render_stats;
use chrono::Local;
use eframe::egui::{Rect, Ui};

impl App {
    /// Renders the statistics dashboard with productivity charts, word counts, and streak metrics.
    pub fn render_stats_pane(&mut self, ui: &mut Ui, editor_panel_rect: Rect) {
        let today_str = Local::now().date_naive().format("%Y-%m-%d").to_string();
        let yest_str = (Local::now().date_naive() - chrono::Duration::days(1))
            .format("%Y-%m-%d")
            .to_string();
        render_stats(
            ui,
            editor_panel_rect,
            &self.today_activity,
            &self.activity_history,
            self.lifetime_activity,
            self.total_notes_count,
            &self.theme,
            &today_str,
            &yest_str,
        );
    }
}
