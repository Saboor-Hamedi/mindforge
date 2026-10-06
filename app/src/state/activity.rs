//! Daily activity and writing analytics tracking state.
//!
//! Accumulates keystrokes, words, and active time in memory, then flushes
//! to disk every 10 seconds via the background storage worker. Also holds
//! the loaded history and lifetime totals for the stats view.

use core::DailyActivity;

/// State for activity tracking and writing analytics.
#[derive(Clone)]
pub struct ActivityState {
    /// Accumulated active seconds since last flush
    pub pending_secs: f32,
    /// Accumulated keystrokes since last flush
    pub pending_keys: u32,
    /// Accumulated words written since last flush
    pub pending_words: u32,
    /// Accumulated notes created since last flush
    pub pending_created: u32,
    /// Accumulated notes edited since last flush
    pub pending_edited: u32,
    /// Timestamp of the last storage flush
    pub last_flush_time: f64,
    /// Today's activity record (loaded from storage)
    pub today_activity: DailyActivity,
    /// Historical daily activity records (most recent first)
    pub activity_history: Vec<DailyActivity>,
    /// Lifetime totals: (total_seconds, total_keystrokes, total_words, active_days)
    pub lifetime_activity: (u64, u64, u64, usize),
}

impl Default for ActivityState {
    fn default() -> Self {
        Self {
            pending_secs: 0.0,
            pending_keys: 0,
            pending_words: 0,
            pending_created: 0,
            pending_edited: 0,
            last_flush_time: 0.0,
            today_activity: DailyActivity {
                date: String::new(),
                active_seconds: 0,
                keystrokes: 0,
                words_written: 0,
                notes_created: 0,
                notes_edited: 0,
            },
            activity_history: Vec::new(),
            lifetime_activity: (0, 0, 0, 0),
        }
    }
}

impl ActivityState {
    /// Creates a new activity state with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a keystroke and optional word completion.
    pub fn record_keystroke(&mut self, word_completed: bool) {
        self.pending_keys += 1;
        if word_completed {
            self.pending_words += 1;
        }
    }

    /// Records accumulated active time.
    pub fn record_time(&mut self, seconds: f32) {
        self.pending_secs += seconds;
    }

    /// Records a note creation event.
    pub fn record_note_created(&mut self) {
        self.pending_created += 1;
    }

    /// Records a note edit event.
    pub fn record_note_edited(&mut self) {
        self.pending_edited += 1;
    }

    /// Returns true if there are pending changes to flush.
    pub fn has_pending(&self) -> bool {
        self.pending_secs > 0.0
            || self.pending_keys > 0
            || self.pending_words > 0
            || self.pending_created > 0
            || self.pending_edited > 0
    }

    /// Resets all pending counters (called after successful flush).
    pub fn clear_pending(&mut self) {
        self.pending_secs = 0.0;
        self.pending_keys = 0;
        self.pending_words = 0;
        self.pending_created = 0;
        self.pending_edited = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activity_tracking() {
        let mut act = ActivityState::new();
        assert!(!act.has_pending());
        act.record_keystroke(true);
        assert!(act.has_pending());
        assert_eq!(act.pending_keys, 1);
        assert_eq!(act.pending_words, 1);
    }

    #[test]
    fn test_activity_clear_pending() {
        let mut act = ActivityState::new();
        act.record_keystroke(false);
        act.record_time(5.0);
        act.record_note_created();
        act.record_note_edited();
        assert!(act.has_pending());
        act.clear_pending();
        assert!(!act.has_pending());
    }
}
