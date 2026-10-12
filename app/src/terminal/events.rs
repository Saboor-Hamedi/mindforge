//! PTY output processing and terminal exit handling.

use egui_term::PtyEvent;

use super::terminal::TerminalPane;

impl TerminalPane {
    /// Process pending PTY events across all sessions.
    /// If an individual session exits (e.g. user typed 'exit'), remove only that session.
    /// Returns true only if all sessions have exited.
    pub fn pump(&mut self) -> bool {
        let mut exited_ids = Vec::new();
        for session in &mut self.sessions {
            while let Ok((_id, event)) = session.event_rx.try_recv() {
                if let PtyEvent::Exit = event {
                    exited_ids.push(session.id);
                }
            }
        }
        for id in exited_ids {
            if let Some(pos) = self.sessions.iter().position(|s| s.id == id) {
                self.sessions.remove(pos);
            }
        }
        if self.sessions.is_empty() {
            return true;
        }
        if self.active_session_idx >= self.sessions.len() {
            self.active_session_idx = self.sessions.len() - 1;
        }
        false
    }
}
