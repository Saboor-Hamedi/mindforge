//! Neovim process, startup, and pending-input state owned by the Vim mode.

use crate::app::App;
use crate::editor::backend::EditorBackend;
use crate::editor::events::EditorKeyEvent;
use crate::mode::Mode;
use eframe::egui;
use std::collections::VecDeque;

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
    pub buffer_name: Option<String>,
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
            buffer_name: None,
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
pub fn update(app: &mut App, ctx: &egui::Context, now: f64) {
    let startup_result = app
        .services
        .vim_runtime
        .start_rx
        .as_ref()
        .map(|rx| rx.try_recv());
    match startup_result {
        Some(Ok(Ok(mut backend))) => {
            app.services.vim_runtime.start_rx = None;
            let pending = std::mem::take(&mut app.services.vim_runtime.pending_input);
            for event in pending {
                match event {
                    PendingVimInput::Text(text) => {
                        let _ = backend.handle_text(&text);
                    }
                    PendingVimInput::Paste(text) => {
                        let _ = backend.paste(&text);
                    }
                    PendingVimInput::Key(event) => {
                        let _ = backend.handle_key(event);
                    }
                }
            }
            app.services.vim_runtime.backend = Some(backend);
            if let (Some(backend), Some(root)) = (
                app.services.vim_runtime.backend.as_mut(),
                app.workspace.root.as_deref(),
            ) {
                let _ = backend.set_workspace_root(root);
            }
            app.services.vim_runtime.note_id = app.services.vim_runtime.start_note_id;
            app.services.vim_runtime.tab_index = app.services.vim_runtime.start_tab_index;
            app.services.vim_runtime.language = app.services.vim_runtime.start_language;
            app.services.vim_runtime.buffer_name = Some(app.active_buffer_name());
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

    let vim_active = matches!(app.misc.mode, Mode::Normal | Mode::Doc)
        && (!app.misc.show_welcome || app.misc.mode == Mode::Doc)
        && (!app.open_notes.is_empty() || app.misc.mode == Mode::Doc);

    if !vim_active {
        return;
    }

    let (active_doc_id, active_tab_idx, file_name, language) = if app.misc.mode == Mode::Doc {
        (
            Some(-(app.tabs.active_doc_idx as i64 + 1)),
            Some(app.tabs.active_doc_tab),
            "note.md".to_string(),
            crate::language::FileLanguage::Markdown,
        )
    } else {
        let file_name = app.active_buffer_name();
        (
            app.notes.active_note_id,
            Some(app.tabs.active_tab),
            file_name,
            app.active_language(),
        )
    };

    if vim_active
        && app.services.vim_runtime.backend.is_none()
        && app.services.vim_runtime.start_rx.is_none()
        && app.services.vim_runtime.start_error.is_none()
    {
        let (active_text, active_row, active_col) = if app.misc.mode == Mode::Doc {
            let (r, c) = app.editor.doc_ed.row_col();
            (app.editor.doc_ed.text(), r, c)
        } else {
            let (r, c) = app.editor.ed.row_col();
            (app.editor.ed.text(), r, c)
        };

        let (tx, rx) = std::sync::mpsc::channel();
        app.services.vim_runtime.start_rx = Some(rx);
        app.services.vim_runtime.start_note_id = active_doc_id;
        app.services.vim_runtime.start_tab_index = active_tab_idx;
        app.services.vim_runtime.start_language = Some(language);
        let start_context = ctx.clone();
        let repaint_context = ctx.clone();
        let text_clone = active_text.clone();
        let file_name_clone = file_name.clone();
        // Match the grid sizing used by `vim::render` exactly, so Neovim is
        // attached at the correct dimensions on the first try. A mismatch forced
        // a reflow resize shortly after opening, which is what made the editor
        // (and its line-number gutter) visibly settle a moment after opening.
        let (init_cols, init_rows) = if let (Some(rect), Some(font_size)) =
            (app.editor.last_editor_rect, app.editor.last_ed_font_size)
        {
            let (_, cw, lh) = app.misc.zoom.editor_metrics(font_size, ctx);
            const NUMBER_WIDTH: usize = 4;
            let show_numbers = app.editor.show_line_numbers;
            let gutter_w = if show_numbers {
                (NUMBER_WIDTH as f32 * (font_size * 0.55) + 14.0).max(28.0)
            } else {
                0.0
            };
            let effective_gutter_w = if rect.width() > gutter_w + 40.0 {
                gutter_w
            } else {
                0.0
            };
            let pad_x = if show_numbers { 16.0 } else { 24.0 };
            let text_left = rect.min.x + effective_gutter_w + pad_x;
            let number_columns = if show_numbers && effective_gutter_w > 0.0 {
                NUMBER_WIDTH
            } else {
                0
            };
            let cols = ((rect.max.x - text_left) / cw.max(1.0)).floor().max(1.0) as usize
                + number_columns;
            let rows = ((rect.max.y - (rect.min.y + 10.0)) / lh.max(1.0))
                .floor()
                .max(1.0) as usize;
            (cols.max(20), rows.max(5))
        } else {
            (80, 24)
        };
        let spawn = std::thread::Builder::new()
            .name("neovim-startup".into())
            .spawn(move || {
                let result = super::VimBackend::start(
                    &text_clone,
                    init_cols,
                    init_rows,
                    active_row,
                    active_col,
                    Some(start_context),
                    &file_name_clone,
                    language,
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
            || app.services.vim_runtime.language != Some(language)
            || app.services.vim_runtime.buffer_name.as_deref() != Some(file_name.as_str());
        if switched_document {
            let (active_text, active_row, active_col) = if app.misc.mode == Mode::Doc {
                let (r, c) = app.editor.doc_ed.row_col();
                (app.editor.doc_ed.text(), r, c)
            } else {
                let (r, c) = app.editor.ed.row_col();
                (app.editor.ed.text(), r, c)
            };

            sync_neovim_changes(app, now);
            if let Some(backend) = app.services.vim_runtime.backend.as_mut() {
                let _ = backend.set_document_for_language(
                    &active_text,
                    active_row,
                    active_col,
                    &file_name,
                    language,
                );
            }
            app.services.vim_runtime.note_id = active_doc_id;
            app.services.vim_runtime.tab_index = active_tab_idx;
            app.services.vim_runtime.language = Some(language);
            app.services.vim_runtime.buffer_name = Some(file_name.clone());
        }
    }
    sync_neovim_changes(app, now);
}

pub(crate) fn sync_neovim_changes(app: &mut App, now: f64) {
    let (
        update,
        cursor,
        tab_index,
        note_id,
        backend_error,
        line_numbers_enabled,
        rpc_showcmd,
        rpc_cmdline,
        rpc_mode,
    ) = {
        let Some(backend) = app.services.vim_runtime.backend.as_mut() else {
            return;
        };
        backend.tick();
        let update = backend.take_text_update();
        let cursor = Some(backend.cursor_char_index());
        let backend_error = backend.take_error();
        let line_numbers_enabled = backend.line_numbers_enabled;
        let rpc_showcmd = backend.grid.showcmd.clone();
        let rpc_cmdline = backend.grid.command_line.clone();
        let rpc_mode = backend.grid.mode.clone();
        (
            update,
            cursor,
            app.services.vim_runtime.tab_index,
            app.services.vim_runtime.note_id,
            backend_error,
            line_numbers_enabled,
            rpc_showcmd,
            rpc_cmdline,
            rpc_mode,
        )
    };
    app.misc
        .showcmd
        .sync_from_rpc(&rpc_showcmd, &rpc_cmdline, &rpc_mode, now);
    if let Some(show_numbers) = line_numbers_enabled {
        app.editor.show_line_numbers = show_numbers;
    }
    if let Some(error) = backend_error {
        app.set_status(format!("Neovim error: {error}"), now);
    }
    let changed = update.is_some();
    if app.misc.mode == Mode::Doc {
        if let Some(text) = update {
            app.editor.doc_ed.set_text(&text);
        }
        if let Some(cursor) = cursor {
            app.editor.doc_ed.cur = cursor.min(app.editor.doc_ed.buf.len());
        }
    } else if tab_index == Some(app.tabs.active_tab) && note_id == app.notes.active_note_id {
        if let Some(text) = update {
            app.editor.ed.set_text(&text);
        }
        if let Some(cursor) = cursor {
            app.editor.ed.cur = cursor.min(app.editor.ed.buf.len());
        }
        if changed {
            app.editor.is_dirty = true;
            app.misc.last_char_time = now;
        }
    } else if let Some(tab) = tab_index.and_then(|idx| app.open_notes.get_mut(idx)) {
        if note_id.is_none() || tab.id == note_id.unwrap_or_default() {
            if let Some(text) = update {
                tab.editor.set_text(&text);
            }
            if let Some(cursor) = cursor {
                tab.editor.cur = cursor.min(tab.editor.buf.len());
            }
            if changed {
                tab.is_dirty = true;
            }
        }
    }
}
