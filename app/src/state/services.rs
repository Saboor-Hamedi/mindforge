//! External service and engine state for MINDFORGE's background infrastructure.
//!
//! Groups all fields that represent external systems or background workers:
//! - Database connection and async worker channel
//! - Editor controller (Vim/Hybrid mode switching)
//! - Vim runtime and hybrid engine
//! - AI agent state
//! - Workspace importer
//! - LunaLine statusline config
//! - Wikilink hover and autocomplete state
//!
//! These fields are typically not `Clone` and are owned exclusively by the `App`.

use crate::editor::controller::EditorController;
use crate::hybrid::HybridEngine;
use crate::vim::VimRuntime;

/// State for all external services and background engines.
pub struct ServicesState {
    /// Direct SQLite database connection (for reads; writes go through db_tx)
    pub db: Option<core::Database>,
    /// Channel sender for the background DB worker thread
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
    /// Workspace/Obsidian vault importer
    pub workspace_importer: crate::workspace_import::WorkspaceImporter,
    /// LunaLine statusline configuration
    pub lunaline_config: crate::lunaline::LunaLineConfig,
    /// Wikilink hover preview state
    pub hover_wikilink: crate::wikilink::hover_wikilink::HoverWikiLinkState,
    /// Wikilink autocomplete state
    pub wikilink_autocomplete: crate::wikilink::wikilink_autocompletion::WikiLinkAutocompleteState,
}

impl ServicesState {
    /// Creates a new services state with the given DB worker sender.
    ///
    /// # Arguments
    /// * `db_tx` — Channel sender for the background DB worker thread
    pub fn new(db_tx: std::sync::mpsc::Sender<crate::services::db_worker::DbMsg>) -> Self {
        Self {
            db: None,
            db_tx,
            editor_controller: EditorController::new(crate::app::EditorInputMode::Hybrid),
            vim_runtime: VimRuntime::default(),
            hybrid: HybridEngine::new(),
            updater: crate::services::updater::UpdateManager::new(),
            agent_state: crate::agent::AgentState::new(),
            workspace_importer: crate::workspace_import::WorkspaceImporter::new(),
            lunaline_config: crate::lunaline::LunaLineConfig::default(),
            hover_wikilink: crate::wikilink::hover_wikilink::HoverWikiLinkState::default(),
            wikilink_autocomplete: crate::wikilink::wikilink_autocompletion::WikiLinkAutocompleteState::default(),
        }
    }

    /// Opens the database connection and stores it in the state.
    pub fn init_db(&mut self) {
        if self.db.is_none() {
            self.db = core::Database::open_default().ok();
        }
    }

    /// Sends a message to the background DB worker.
    pub fn send_db(&self, msg: crate::services::db_worker::DbMsg) {
        let _ = self.db_tx.send(msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::db_worker::DbMsg;

    #[test]
    fn test_services_db() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let mut services = ServicesState::new(tx);
        assert!(services.db.is_none());
        services.init_db();
        assert!(services.db.is_some());
    }

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
