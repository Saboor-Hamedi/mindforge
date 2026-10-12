//! Terminal session representation and multi-session management.
//!
//! Each session owns its own PTY process, title, and event channel. The
//! [`TerminalPane`] keeps an ordered list of sessions plus the index of the
//! currently active one.

use eframe::egui;
use egui_term::{PtyEvent, TerminalBackend};
use std::sync::mpsc;

use super::process::spawn_backend_session;
use super::terminal::TerminalPane;

/// An individual terminal session with its own PTY process, title, and event channel.
pub struct TerminalSession {
    pub id: u64,
    pub title: String,
    pub shell_type: String,
    pub backend: TerminalBackend,
    pub event_rx: mpsc::Receiver<(u64, PtyEvent)>,
}

impl TerminalPane {
    /// Spawns a new session and sets it as the active session.
    pub fn new_session(&mut self, ctx: &egui::Context) -> bool {
        let session_num = self.sessions.len() + 1;
        let id = self.next_session_id;
        self.next_session_id += 1;

        match spawn_backend_session(id, ctx, session_num, self.working_dir.clone()) {
            Ok(session) => {
                self.sessions.push(session);
                self.active_session_idx = self.sessions.len() - 1;
                true
            }
            Err(_) => false,
        }
    }

    /// Closes the session at the specified index.
    /// Returns true if all sessions were closed and the terminal dock should collapse.
    pub fn close_session(&mut self, idx: usize) -> bool {
        if idx < self.sessions.len() {
            self.sessions.remove(idx);
            if self.sessions.is_empty() {
                return true;
            }
            if self.active_session_idx >= self.sessions.len() {
                self.active_session_idx = self.sessions.len() - 1;
            }
        }
        false
    }
}
