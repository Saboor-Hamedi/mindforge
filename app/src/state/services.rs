//! External service and engine state for MINDFORGE's background infrastructure.
//!
//! Groups all fields that represent external systems or background workers:
//! - Filesystem storage worker channel
//! - Editor controller (Vim/Hybrid mode switching)
//! - Vim runtime and hybrid engine
//! - AI agent state
//! - LunaLine statusline config
//! - Wikilink hover and autocomplete state
//!
//! These fields are typically not `Clone` and are owned exclusively by the `App`.

use crate::editor::controller::EditorController;
use crate::hybrid::HybridEngine;
use crate::vim::VimRuntime;

/// State for all external services and background engines.
pub struct ServicesState {
    /// Channel sender for the background storage worker thread
    pub db_tx: std::sync::mpsc::Sender<crate::services::db_worker::DbMsg>,
    /// Editor controller for switching between Vim and Hybrid modes
    pub editor_controller: EditorController,
    /// Vim modal editing runtime
    pub vim_runtime: VimRuntime,
    /// Hybrid editing engine (non-modal)
    pub hybrid: HybridEngine,
    /// In-app GitHub updater
    pub updater: crate::services::updater::UpdateManager,
    /// DeepSeek Pro AI agent state
    pub agent_state: crate::agent::AgentState,
    /// LunaLine statusline configuration
    pub lunaline_config: crate::lunaline::LunaLineConfig,
    /// Wikilink hover preview state
    pub hover_wikilink: crate::wikilink::hover_wikilink::HoverWikiLinkState,
    /// Wikilink autocomplete state
    pub wikilink_autocomplete: crate::wikilink::wikilink_autocompletion::WikiLinkAutocompleteState,
}

impl ServicesState {
    /// Creates a new services state with the given storage worker sender.
    pub fn new(db_tx: std::sync::mpsc::Sender<crate::services::db_worker::DbMsg>) -> Self {
        Self {
            db_tx,
            editor_controller: EditorController::new(crate::app::EditorInputMode::Hybrid),
            vim_runtime: VimRuntime::default(),
            hybrid: HybridEngine::new(),
            updater: crate::services::updater::UpdateManager::new(),
            agent_state: crate::agent::AgentState::new(),
            lunaline_config: crate::lunaline::LunaLineConfig::default(),
            hover_wikilink: crate::wikilink::hover_wikilink::HoverWikiLinkState::default(),
            wikilink_autocomplete:
                crate::wikilink::wikilink_autocompletion::WikiLinkAutocompleteState::default(),
        }
    }

    /// Sends a message to the background storage worker.
    pub fn send_db(&self, msg: crate::services::db_worker::DbMsg) {
        let _ = self.db_tx.send(msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::db_worker::DbMsg;

    #[test]
    fn test_services_send_db() {
        let (tx, rx) = std::sync::mpsc::channel();
        let services = ServicesState::new(tx);
        services.send_db(DbMsg::SaveSetting {
            key: "test".to_string(),
            val: "value".to_string(),
        });
        assert!(rx.try_recv().is_ok());
    }
}
