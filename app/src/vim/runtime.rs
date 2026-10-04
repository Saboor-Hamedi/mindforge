//! Neovim process, startup, and pending-input state owned by the Vim mode.

use crate::editor::backend::EditorBackend;
use crate::editor::events::EditorKeyEvent;
use std::collections::VecDeque;
use crate::app::{App, EditorInputMode};
use crate::mode::Mode;
use eframe::egui;

pub enum PendingVimInput {
    Text(String),
    Paste(String),
    Key(EditorKeyEvent),
}

pub struct VimRuntime {
    pub backend: Option<super::VimBackend>,
    pub start_rx: Option<std::sync::mpsc::Receiver<Result<super::VimBackend, String>>>,
    pub start_note_id: Option<i64>,
    pub start_tab_index: Option<usize>,
    pub start_language: Option<crate::language::FileLanguage>,
    pub note_id: Option<i64>,
    pub tab_index: Option<usize>,
    pub language: Option<crate::language::FileLanguage>,
    pub pending_input: VecDeque<PendingVimInput>,
    pub start_error: Option<String>,
}

impl Default for VimRuntime {
    fn default() -> Self {
        Self {
            backend: None,
            start_rx: None,
            start_note_id: None,
            start_tab_index: None,
            start_language: None,
            note_id: None,
            tab_index: None,
            language: None,
            pending_input: VecDeque::new(),
            start_error: None,
        }
    }
}

impl VimRuntime {
    pub fn queue_input(&mut self, input: PendingVimInput) -> bool {
        const MAX_PENDING_INPUTS: usize = 512;
        if self.pending_input.len() >= MAX_PENDING_INPUTS {
            return false;
        }
        self.pending_input.push_back(input);
        true
    }
}

/// Run Vim startup, RPC synchronization, and shutdown at the app frame boundary.
pub fn update(app: &mut App, ctx: &egui::Context, now: f64, mode_at_frame_start: EditorInputMode) {
    let entering_vim = mode_at_frame_start == EditorInputMode::Vim
        && app.services.editor_controller.last_observed_mode != EditorInputMode::Vim;
    if entering_vim {
        app.misc.caret.clear_transient_effects();
        app.services.vim_runtime.start_error = None;
    }

    let mut vim_started_this_frame = false;
    let startup_result = app.services.vim_runtime.start_rx.as_ref().map(|rx| rx.try_recv());
    match startup_result {
        Some(Ok(Ok(mut backend))) => {
            vim_started_this_frame = true;
            app.services.vim_runtime.start_rx = None;
            if app.services.editor_controller.mode == EditorInputMode::Vim {
                let pending = std::mem::take(&mut app.services.vim_runtime.pending_input);
                for event in pending {
                    match event {
                        PendingVimInput::Text(text) => { let _ = backend.handle_text(&text); }
                        PendingVimInput::Paste(text) => { let _ = backend.paste(&text); }
                        PendingVimInput::Key(event) => { let _ = backend.handle_key(event); }
                    }
                }
                app.services.vim_runtime.backend = Some(backend);
                app.services.vim_runtime.note_id = app.services.vim_runtime.start_note_id;
                app.services.vim_runtime.tab_index = app.services.vim_runtime.start_tab_index;
                app.services.vim_runtime.language = app.services.vim_runtime.start_language;
            } else {
                backend.shutdown();
            }
        }
        Some(Ok(Err(error))) => {
            app.services.vim_runtime.start_rx = None;
            app.services.vim_runtime.start_error = Some(error.clone());
            app.set_status(&format!("Neovim could not start: {error}"), now);
        }
        Some(Err(std::sync::mpsc::TryRecvError::Disconnected)) => {
            app.services.vim_runtime.start_rx = None;
            let error = "Neovim startup worker stopped unexpectedly".to_string();
            app.services.vim_runtime.start_error = Some(error.clone());
            app.set_status(&error, now);
        }
        _ => {}
    }

    if app.services.editor_controller.mode == EditorInputMode::Hybrid {
        replay_pending_hybrid_input(app, now);
        sync_neovim_changes(app, now);
    }

    let (active_text, active_row, active_col, active_doc_id, active_tab_idx, file_name, language) = if app.misc.mode == Mode::Doc {
        let text = app.editor.doc_ed.text();
        let (r, c) = app.editor.doc_ed.row_col();
        (text, r, c, Some(-(app.tabs.active_doc_idx as i64 + 1)), Some(app.tabs.active_doc_tab), "note.md".to_string(), crate::language::FileLanguage::Markdown)
    } else {
        let text = app.editor.ed.text();
        let (r, c) = app.editor.ed.row_col();
        let file_name = app.active_buffer_name();
        (text, r, c, app.notes.active_note_id, Some(app.tabs.active_tab), file_name, app.active_language())
    };

    let vim_active = matches!(app.misc.mode, Mode::Normal | Mode::Doc)
        && app.services.editor_controller.mode == EditorInputMode::Vim
        && (!app.misc.show_welcome || app.misc.mode == Mode::Doc)
        && (!app.open_notes.is_empty() || app.misc.mode == Mode::Doc);
    if vim_active
        && app.services.vim_runtime.backend.is_none()
        && app.services.vim_runtime.start_rx.is_none()
        && app.services.vim_runtime.start_error.is_none()
    {
        let (tx, rx) = std::sync::mpsc::channel();
        app.services.vim_runtime.start_rx = Some(rx);
        app.services.vim_runtime.start_note_id = active_doc_id;
        app.services.vim_runtime.start_tab_index = active_tab_idx;
        app.services.vim_runtime.start_language = Some(language);
        let start_context = ctx.clone();
        let repaint_context = ctx.clone();
        let text_clone = active_text.clone();
        let file_name_clone = file_name.clone();
        let (init_cols, init_rows) = if let (Some(rect), Some(font_size)) = (app.editor.last_editor_rect, app.editor.last_ed_font_size) {
            let (_, cw, lh) = app.misc.zoom.editor_metrics(font_size, ctx);
            let cols = (rect.width() / cw.max(1.0)).floor().max(20.0) as usize;
            let rows = (rect.height() / lh.max(1.0)).floor().max(5.0) as usize;
            (cols, rows)
        } else {
            (80, 24)
        };
        let spawn = std::thread::Builder::new()
            .name("neovim-startup".into())
            .spawn(move || {
                let result = super::VimBackend::start(
                    &text_clone, init_cols, init_rows, active_row, active_col,
                    Some(start_context), &file_name_clone, language,
                );
                let _ = tx.send(result);
                repaint_context.request_repaint();
            });
        if let Err(error) = spawn {
            app.services.vim_runtime.start_rx = None;
            app.services.vim_runtime.start_error = Some(error.to_string());
            app.set_status(&format!("Neovim worker could not start: {error}"), now);
        }
    } else if vim_active && app.services.vim_runtime.backend.is_some() {
        let switched_document = app.services.vim_runtime.note_id != active_doc_id
            || app.services.vim_runtime.tab_index != active_tab_idx
            || app.services.vim_runtime.language != Some(language);
        if switched_document || (entering_vim && !vim_started_this_frame) {
            if entering_vim {
                if let Some(backend) = app.services.vim_runtime.backend.as_mut() {
                    backend.tick();
                    let _ = backend.take_text_update();
                    let _ = backend.set_document_for_language(&active_text, active_row, active_col, &file_name, language);
                    backend.sync_theme(&app.misc.theme);
                }
            } else {
                sync_neovim_changes(app, now);
                if let Some(backend) = app.services.vim_runtime.backend.as_mut() {
                    let _ = backend.set_document_for_language(&active_text, active_row, active_col, &file_name, language);
                }
            }
            app.services.vim_runtime.note_id = active_doc_id;
            app.services.vim_runtime.tab_index = active_tab_idx;
            app.services.vim_runtime.language = Some(language);
        }
    }
    sync_neovim_changes(app, now);
}

fn replay_pending_hybrid_input(app: &mut App, now: f64) {
    let pending = std::mem::take(&mut app.services.vim_runtime.pending_input);
    for event in pending {
        match event {
            PendingVimInput::Text(text) => crate::input::editor::handle_editor_text(app, &text, now),
            PendingVimInput::Paste(text) => crate::input::editor::handle_editor_paste(app, &text, now),
            PendingVimInput::Key(event) => crate::input::editor::handle_editor_key(app, event.key, event.modifiers, now),
        };
    }
}

fn sync_neovim_changes(app: &mut App, now: f64) {
    if app.services.editor_controller.mode != EditorInputMode::Vim {
        return;
    }
    let (update, cursor, tab_index, note_id) = {
        let Some(backend) = app.services.vim_runtime.backend.as_mut() else { return };
        backend.tick();
        let update = backend.take_text_update();
        let cursor = update.as_ref().map(|_| backend.cursor_char_index());
        (update, cursor, app.services.vim_runtime.tab_index, app.services.vim_runtime.note_id)
    };
    let changed = update.is_some();
    if app.misc.mode == Mode::Doc {
        if let Some(text) = update {
            app.editor.doc_ed.set_text(&text);
        }
        if let Some(cursor) = cursor {
            app.editor.doc_ed.cur = cursor.min(app.editor.doc_ed.buf.len());
        }
    } else if tab_index == Some(app.tabs.active_tab) && note_id == app.notes.active_note_id {
        if let Some(text) = update { app.editor.ed.set_text(&text); }
        if let Some(cursor) = cursor { app.editor.ed.cur = cursor.min(app.editor.ed.buf.len()); }
        if changed {
            app.editor.is_dirty = true;
            app.misc.last_char_time = now;
        }
    } else if let Some(tab) = tab_index.and_then(|idx| app.open_notes.get_mut(idx)) {
        if note_id.is_none() || tab.id == note_id.unwrap_or_default() {
            if let Some(text) = update { tab.editor.set_text(&text); }
            if let Some(cursor) = cursor { tab.editor.cur = cursor.min(tab.editor.buf.len()); }
            if changed { tab.is_dirty = true; }
        }
    }
}
