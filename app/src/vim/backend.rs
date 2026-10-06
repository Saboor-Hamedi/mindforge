//! Neovim Embedded Process Backend Controller.
//!
//! # Purpose
//! Manages the lifecycle, MessagePack-RPC communication, line updates, and grid
//! state for the embedded Neovim engine instance in MindForge.
//!
//! # Architecture & Responsibilities
//! - Spawns and supervises the child `nvim` process via `NeovimClient`.
//! - Receives incremental line events (`nvim_buf_lines_event`) and updates document stats.
//! - Dispatches redraw batches (`grid_line`, `grid_scroll`, `grid_resize`, `hl_attr_define`).
//! - Manages document synchronization between MindForge filesystem tabs and Neovim buffers.
//! - Executes synchronous Ex-commands (`:w`, `:sort`, `:%s/...`) and Lua snippets.
//!
//! # Subsystem Delegations
//! - Embedded assets & init scripts: [`super::bootstrap`]
//! - Theme palette synchronization: [`super::theme`]
//! - Editor surface rasterization & popup menus: [`super::render`]
//! - MindForge EditorBackend trait mapping: [`super::editor_backend`]
//!
//! # Invariants & Non-Goals
//! - Must NOT perform blocking UI rendering or layout jobs on the main thread.
//! - Must NOT bypass Neovim as the authority for Vim modes and cursor movements.

use super::{
    bootstrap::{build_lsp_modules_value, INIT_LUA, LSP_BOOT},
    client::NeovimClient,
    state::{GridCell, GridState},
};
use crate::editor::backend::EditorResult;
use crate::language::FileLanguage;
use eframe::egui::{self, Color32};
use rmpv::Value;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

pub struct VimBackend {
    pub(crate) client: NeovimClient,
    pub grid: GridState,
    pub(crate) lines: Vec<String>,
    pub(crate) lines_revision: u64,
    pub(crate) document_words: usize,
    pub(crate) document_lines: usize,
    pub(crate) document_bytes: usize,
    pub(crate) cursor_char_cache: Option<(crate::editor::types::CursorPosition, u64, usize)>,
    pub(crate) dirty: bool,
    pub(crate) text_updated: bool,
    pub(crate) last_buffer_change: Option<Instant>,
    pub(crate) error: Option<String>,
    pub(crate) last_size: (usize, usize),
    pub(crate) pending_size: Option<(usize, usize)>,
    pub(crate) resize_pending_since: Option<f64>,
    pub(crate) row_layouts: Vec<Option<(u64, Arc<egui::Galley>)>>,
    pub(crate) layout_font_size: u32,
    pub(crate) cursor_render_initialized: bool,
    pub(crate) busy: Option<String>,
    pub(crate) lsp_status: (String, String),
    pub(crate) lsp_panel: super::lsp_panel::LspPanel,
    pub(crate) line_numbers_enabled: Option<bool>,
    pub(crate) cached_theme: Option<(Color32, Color32, Color32, Color32)>,
    pub(crate) suppress_lines_events: usize,
}

impl VimBackend {
    /// Sets the active workspace root folder inside Neovim and refreshes LSP.
    pub fn set_workspace_root(&mut self, root: &std::path::Path) -> Result<(), String> {
        let root = root.to_string_lossy().replace('\\', "/");
        self.client.notify(
            "nvim_exec_lua",
            vec![
                Value::from("vim.api.nvim_set_current_dir((...))"),
                Value::Array(vec![Value::from(root)]),
            ],
        )?;
        self.client.notify(
            "nvim_exec_lua",
            vec![
                Value::from(
                    "local ok,m=pcall(require,'mindforge.lsp'); if ok then m.refresh() end",
                ),
                Value::Array(vec![]),
            ],
        )?;
        Ok(())
    }

    /// Spawns an embedded Neovim child instance and bootstraps MindForge's Lua LSP environment.
    pub fn start(
        text: &str,
        width: usize,
        height: usize,
        row: usize,
        column: usize,
        ctx: Option<egui::Context>,
        file_name: &str,
        language: FileLanguage,
    ) -> Result<Self, String> {
        let mut client = NeovimClient::start(ctx)?;

        // Attach embedded grid before installing active note
        client.attach_ui(width, height)?;

        // Execute init options and key bindings
        let _ = client.request(
            "nvim_exec_lua",
            vec![Value::from(INIT_LUA), Value::Array(vec![])],
        );

        // Preload MindForge LSP modules
        let modules = build_lsp_modules_value();
        let _ = client.request(
            "nvim_exec_lua",
            vec![Value::from(LSP_BOOT), Value::Array(vec![modules])],
        );

        let _ = client.request(
            "nvim_buf_set_name",
            vec![Value::from(0), Value::from(file_name.to_owned())],
        );

        // Load text asynchronously via notifications
        client.set_buffer_text_async(text)?;
        let filetype = language.nvim_filetype();
        let _ = client.notify(
            "nvim_command",
            vec![Value::from(format!(
                "setlocal filetype={filetype} syntax={filetype} numberwidth=4 signcolumn=no foldcolumn=0"
            ))],
        );
        let _ = client.notify(
            "nvim_exec_lua",
            vec![
                Value::from(
                    "local ok, m = pcall(require, 'mindforge.lsp'); if ok then m.refresh() end",
                ),
                Value::Array(vec![]),
            ],
        );

        client.notify(
            "nvim_buf_attach",
            vec![Value::from(0), Value::from(false), Value::Map(vec![])],
        )?;

        let byte_column = text
            .split('\n')
            .nth(row)
            .map(|line| line.chars().take(column).map(char::len_utf8).sum())
            .unwrap_or(0);
        let _ = client.set_cursor_async(row, byte_column);

        Ok(Self {
            client,
            grid: GridState::default(),
            lines: text.split('\n').map(str::to_owned).collect(),
            lines_revision: 0,
            document_words: text.split_whitespace().count(),
            document_lines: text.lines().count(),
            document_bytes: text.len(),
            cursor_char_cache: None,
            dirty: false,
            text_updated: false,
            last_buffer_change: None,
            error: None,
            last_size: (width, height),
            pending_size: None,
            resize_pending_since: None,
            row_layouts: Vec::new(),
            layout_font_size: 0,
            cursor_render_initialized: false,
            busy: None,
            lsp_status: Default::default(),
            lsp_panel: Default::default(),
            line_numbers_enabled: None,
            cached_theme: None,
            suppress_lines_events: 0,
        })
    }

    pub fn set_repaint_context(&self, ctx: egui::Context) {
        self.client.set_repaint_context(ctx);
    }

    pub(crate) fn consume_notification(&mut self, packet: &Value) {
        let Some(items) = packet.as_array() else {
            return;
        };
        if items.get(1).and_then(Value::as_str) == Some("redraw") {
            if let Some(groups) = items.get(2).and_then(Value::as_array) {
                for group in groups {
                    self.apply_redraw_group(group);
                }
            }
        } else if items.get(1).and_then(Value::as_str) == Some("mindforge_progress") {
            let label = items
                .get(2)
                .and_then(Value::as_array)
                .and_then(|args| args.first())
                .and_then(Value::as_str)
                .unwrap_or("");
            self.busy = (!label.is_empty()).then(|| label.to_owned());
        } else if items.get(1).and_then(Value::as_str) == Some("mindforge_lsp_status") {
            let arg = |i: usize| {
                items
                    .get(2)
                    .and_then(Value::as_array)
                    .and_then(|args| args.get(i))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned()
            };
            self.lsp_status = (arg(0), arg(1));
        } else if items.get(1).and_then(Value::as_str) == Some("mindforge_lsp_list") {
            self.apply_lsp_list(items.get(2));
        } else if items.get(1).and_then(Value::as_str) == Some("nvim_buf_lines_event") {
            self.apply_lines_event(items.get(2).unwrap_or(&Value::Nil));
        }
    }

    fn apply_lsp_list(&mut self, args: Option<&Value>) {
        let Some(args) = args.and_then(Value::as_array) else {
            return;
        };
        let text = |value: &Value, i: usize| {
            value
                .as_array()
                .and_then(|r| r.get(i))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned()
        };
        let rows = args
            .first()
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter()
                    .map(|row| super::lsp_panel::LspRow {
                        name: text(row, 0),
                        status: text(row, 1),
                        desc: text(row, 2),
                        filetypes: text(row, 3),
                    })
                    .collect()
            })
            .unwrap_or_default();
        let open = args.get(1).and_then(Value::as_bool).unwrap_or(false);
        let filter = args.get(2).and_then(Value::as_str).unwrap_or("");
        self.lsp_panel.update(rows, open, filter);
    }

    fn apply_lines_event(&mut self, args: &Value) {
        let Some(args) = args.as_array() else { return };
        if self.suppress_lines_events > 0 {
            self.suppress_lines_events -= 1;
            return;
        }

        let raw_start = args
            .get(2)
            .and_then(Value::as_i64)
            .or_else(|| args.get(2).and_then(Value::as_u64).map(|v| v as i64))
            .unwrap_or(0);
        let start = if raw_start < 0 {
            (self.lines.len() as i64 + 1 + raw_start).max(0) as usize
        } else {
            raw_start as usize
        };
        let raw_end = args
            .get(3)
            .and_then(Value::as_i64)
            .or_else(|| args.get(3).and_then(Value::as_u64).map(|v| v as i64))
            .unwrap_or(start as i64);
        let end = if raw_end < 0 {
            self.lines.len()
        } else {
            raw_end as usize
        };
        let start = start.min(self.lines.len());
        let end = end.min(self.lines.len()).max(start);
        let mut replacement: Vec<String> = args
            .get(4)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|v| super::client::rmpv_value_to_str(v))
            .collect();

        // Exact match with existing lines — echo of document load, do not duplicate or dirty
        if start == 0 && replacement == self.lines {
            return;
        }

        // Full-buffer replacement (e.g. from document reset or :%s)
        if start == 0 && (raw_end < 0 || raw_end as usize >= self.lines.len() || replacement.len() == self.lines.len()) {
            if replacement.is_empty() {
                replacement.push(String::new());
            }
            self.document_words = replacement.iter().map(|line| line.split_whitespace().count()).sum();
            self.document_bytes = replacement.iter().map(String::len).sum::<usize>() + replacement.len().saturating_sub(1);
            self.lines = replacement;
            self.document_lines = if self.lines.len() == 1 && self.lines[0].is_empty() {
                0
            } else if self.lines.last().is_some_and(String::is_empty) {
                self.lines.len() - 1
            } else {
                self.lines.len()
            };
            self.lines_revision = self.lines_revision.wrapping_add(1);
            self.cursor_char_cache = None;
            self.dirty = true;
            self.text_updated = true;
            self.last_buffer_change = Some(Instant::now());
            return;
        }

        if start <= self.lines.len() && end <= self.lines.len() && start <= end {
            let removed = end - start;
            let old_words: usize = self.lines[start..end]
                .iter()
                .map(|line| line.split_whitespace().count())
                .sum();
            let old_bytes: usize = self.lines[start..end].iter().map(String::len).sum();
            let new_words: usize = replacement
                .iter()
                .map(|line| line.split_whitespace().count())
                .sum();
            let new_bytes: usize = replacement.iter().map(String::len).sum();
            if start == 0 && end == self.lines.len() && replacement.is_empty() {
                replacement.push(String::new());
            }
            let added = replacement.len();
            self.document_words = self
                .document_words
                .saturating_sub(old_words)
                .saturating_add(new_words);
            self.document_bytes = self
                .document_bytes
                .saturating_sub(old_bytes)
                .saturating_add(new_bytes);
            if added >= removed {
                self.document_bytes = self.document_bytes.saturating_add(added - removed);
            } else {
                self.document_bytes = self.document_bytes.saturating_sub(removed - added);
            }
            self.lines.splice(start..end, replacement);
            if self.lines.is_empty() {
                self.lines.push(String::new());
            }
            self.document_lines = if self.lines.len() == 1 && self.lines[0].is_empty() {
                0
            } else if self.lines.last().is_some_and(String::is_empty) {
                self.lines.len() - 1
            } else {
                self.lines.len()
            };
            self.lines_revision = self.lines_revision.wrapping_add(1);
            self.cursor_char_cache = None;
            self.dirty = true;
            self.text_updated = true;
            self.last_buffer_change = Some(Instant::now());
        }
    }

    fn apply_redraw_group(&mut self, group: &Value) {
        let Some(entries) = group.as_array() else {
            return;
        };
        let Some(name) = entries.first().and_then(Value::as_str) else {
            return;
        };
        for update in entries.iter().skip(1) {
            let Some(args) = update.as_array() else {
                continue;
            };
            match name {
                "grid_resize" if args.len() >= 3 && num(&args[0]) == 1 => {
                    self.grid.resize(num(&args[1]), num(&args[2]));
                }
                "grid_clear" if !args.is_empty() => {
                    let grid = num(&args[0]);
                    if grid == 1 {
                        for (index, row) in self.grid.cells.iter_mut().enumerate() {
                            for cell in row {
                                *cell = GridCell {
                                    text: " ".into(),
                                    highlight: 0,
                                };
                            }
                            if let Some(revision) = self.grid.row_revision.get_mut(index) {
                                *revision = revision.wrapping_add(1);
                            }
                        }
                    }
                }
                "grid_line" if args.len() >= 4 => self.apply_grid_line(args),
                "grid_scroll" if args.len() >= 7 => self.apply_grid_scroll(args),
                "grid_cursor_goto" if args.len() >= 3 && num(&args[0]) == 1 => {
                    let target_row = num(&args[1]);
                    let target_col = num(&args[2]);
                    let is_bottom_cmd_row =
                        self.grid.height > 0 && target_row >= self.grid.height.saturating_sub(1);
                    let in_cmd_or_msg = self.grid.mode == "c"
                        || self.grid.mode.starts_with("cmdline")
                        || !self.grid.command_line.is_empty()
                        || !self.grid.message.is_empty();

                    if !(is_bottom_cmd_row && in_cmd_or_msg) {
                        self.grid.cursor = crate::editor::types::CursorPosition {
                            row: target_row,
                            column: target_col,
                        };
                    }
                }
                "mode_change" if !args.is_empty() => {
                    self.grid.mode = args[0].as_str().unwrap_or("normal").to_owned();
                }
                "hl_attr_define" if args.len() >= 2 => self.apply_highlight(args),
                "cmdline_show" if !args.is_empty() => {
                    let prefix = args.get(2).and_then(Value::as_str).unwrap_or("");
                    self.grid.command_line = format!("{prefix}{}", cmdline_text(&args[0]));
                }
                "cmdline_hide" => self.grid.command_line.clear(),
                "popupmenu_show" if !args.is_empty() => {
                    self.grid.popup_items = super::render::popupmenu_items(&args[0]);
                    self.grid.popup_meta = super::render::popupmenu_meta(&args[0]);
                    self.grid.popup_info = super::render::popupmenu_info(&args[0]);
                    self.grid.popup_selected = args
                        .get(1)
                        .and_then(Value::as_i64)
                        .filter(|selected| *selected >= 0)
                        .map(|selected| selected as usize);
                    self.grid.popup_anchor = Some((
                        num(args.get(2).unwrap_or(&Value::Nil)),
                        num(args.get(3).unwrap_or(&Value::Nil)),
                    ));
                }
                "popupmenu_select" if !args.is_empty() => {
                    self.grid.popup_selected = args[0]
                        .as_i64()
                        .filter(|selected| *selected >= 0)
                        .map(|selected| selected as usize);
                }
                "popupmenu_hide" => {
                    self.grid.popup_items.clear();
                    self.grid.popup_meta.clear();
                    self.grid.popup_info.clear();
                    self.grid.popup_selected = None;
                    self.grid.popup_anchor = None;
                }
                "msg_show" if args.len() >= 2 => {
                    if matches!(args[0].as_str(), Some("return_prompt" | "confirm")) {
                        let _ = self.client.input("<Esc>");
                    }
                    self.grid.message_is_error = matches!(
                        args[0].as_str(),
                        Some("emsg" | "echoerr" | "lua_error" | "rpc_error")
                    );
                    self.grid.message = compact_message(&cmdline_text(&args[1]));
                    self.grid.message_at = Some(std::time::Instant::now());
                }
                "msg_clear" => {
                    let sticky = self.grid.message_is_error
                        && self
                            .grid
                            .message_at
                            .is_some_and(|at| at.elapsed() < Duration::from_secs(8));
                    if sticky {
                        continue;
                    }
                    self.grid.message.clear();
                    self.grid.message_at = None;
                }
                _ => {}
            }
        }
    }

    fn apply_grid_line(&mut self, args: &[Value]) {
        if num(&args[0]) != 1 {
            return;
        }
        let row = num(&args[1]);
        let mut col = num(&args[2]);
        let Some(cells) = args[3].as_array() else {
            return;
        };
        if row >= self.grid.height {
            return;
        }

        let mut last_highlight = 0u64;
        for cell in cells {
            let Some(cell) = cell.as_array() else {
                continue;
            };
            if cell.is_empty() {
                continue;
            }
            let text = cell
                .first()
                .and_then(super::client::rmpv_value_to_str)
                .unwrap_or_else(|| " ".to_string());
            if let Some(h) = cell.get(1).and_then(Value::as_u64) {
                last_highlight = h;
            }
            let repeat = cell.get(2).and_then(Value::as_u64).unwrap_or(1).min(4096) as usize;
            for _ in 0..repeat {
                if col >= self.grid.width {
                    break;
                }
                self.grid.cells[row][col] = GridCell {
                    text: text.clone(),
                    highlight: last_highlight,
                };
                col += 1;
            }
        }
        if row < self.grid.row_revision.len() {
            self.grid.row_revision[row] = self.grid.row_revision[row].wrapping_add(1);
        }
    }

    fn apply_grid_scroll(&mut self, args: &[Value]) {
        if num(&args[0]) != 1 || self.grid.height == 0 || self.grid.width == 0 {
            return;
        }
        let top = num(&args[1]).min(self.grid.height);
        let bottom = num(&args[2]).min(self.grid.height);
        let left = num(&args[3]).min(self.grid.width);
        let right = num(&args[4]).min(self.grid.width);
        let row_delta = args[5].as_i64().unwrap_or(0) as isize;
        let col_delta = args[6].as_i64().unwrap_or(0) as isize;
        if top >= bottom || left >= right {
            return;
        }

        let previous: Vec<Vec<GridCell>> = self.grid.cells[top..bottom]
            .iter()
            .map(|row| row[left..right].to_vec())
            .collect();
        for row in top..bottom {
            for col in left..right {
                let source_row = row as isize + row_delta - top as isize;
                let source_col = col as isize + col_delta - left as isize;
                self.grid.cells[row][col] = if source_row >= 0
                    && source_row < previous.len() as isize
                    && source_col >= 0
                    && source_col < previous[0].len() as isize
                {
                    previous[source_row as usize][source_col as usize].clone()
                } else {
                    GridCell {
                        text: " ".into(),
                        highlight: 0,
                    }
                };
            }
        }
        for revision in &mut self.grid.row_revision[top..bottom] {
            *revision = revision.wrapping_add(1);
        }
    }

    fn apply_highlight(&mut self, args: &[Value]) {
        let id = num(&args[0]) as u64;
        let (mut foreground, mut background, mut special, mut reverse, mut underline) =
            (None, None, None, false, false);
        let mut is_visual = false;
        if let Some(attrs) = args[1].as_map() {
            for (key, value) in attrs {
                let color = value.as_u64().map(|n| n as u32);
                match key.as_str() {
                    Some("foreground") => foreground = color,
                    Some("background") => background = color,
                    Some("special") => special = color,
                    Some("reverse") => reverse = value.as_bool().unwrap_or(false),
                    Some(
                        "underline" | "undercurl" | "underdouble" | "underdotted" | "underdashed",
                    ) => underline = value.as_bool().unwrap_or(false),
                    _ => {}
                }
            }
        }
        if let Some(info) = args.get(3).and_then(Value::as_array) {
            for map in info {
                if let Some(map) = map.as_map() {
                    for (k, v) in map {
                        if k.as_str() == Some("ui_name") || k.as_str() == Some("hi_name") {
                            if let Some(name) = v.as_str() {
                                if name == "Visual" || name == "VisualNOS" {
                                    is_visual = true;
                                }
                                if name == "Cursor" || name == "TermCursor" {
                                    reverse = false;
                                    background = None;
                                }
                            }
                        }
                    }
                }
            }
        }
        self.grid.highlights.insert(
            id,
            super::state::HighlightStyle {
                foreground,
                background,
                special,
                reverse,
                underline,
                is_visual,
            },
        );
        for revision in &mut self.grid.row_revision {
            *revision = revision.wrapping_add(1);
        }
    }

    pub fn take_error(&mut self) -> Option<String> {
        self.error.take()
    }

    pub fn paste(&mut self, text: &str) -> EditorResult<()> {
        self.client.paste(text)
    }

    pub fn send_input(&mut self, input: &str) -> EditorResult<()> {
        self.client.input(input)
    }

    /// (state, detail) of the language servers for the current buffer.
    pub fn lsp_status(&self) -> (&str, &str) {
        (&self.lsp_status.0, &self.lsp_status.1)
    }

    /// Label of long-running background work (installs, LSP indexing), if any.
    pub fn busy_label(&self) -> Option<&str> {
        self.busy.as_deref()
    }

    pub fn popup_visible(&self) -> bool {
        !self.grid.popup_items.is_empty()
    }

    pub fn search(&mut self, query: &str, backwards: bool) -> EditorResult<()> {
        if query.is_empty() {
            return Ok(());
        }
        let direction = if backwards { "b" } else { "" };
        self.client.notify(
            "nvim_exec_lua",
            vec![
                Value::from(concat!(
                    "local query, flags = ...; ",
                    "local id = vim.w._mindforge_live_search_id; ",
                    "if id then pcall(vim.fn.matchdelete, id); vim.w._mindforge_live_search_id = nil end; ",
                    "vim.fn.setreg('/', query); vim.o.hlsearch = true; ",
                    "vim.fn.search(query, flags)"
                )),
                Value::Array(vec![Value::from(query), Value::from(direction)]),
            ],
        )
    }

    pub fn preview_search(&mut self, query: &str) -> EditorResult<()> {
        self.client.notify(
            "nvim_exec_lua",
            vec![
                Value::from(concat!(
                    "local query = ...; ",
                    "local id = vim.w._mindforge_live_search_id; ",
                    "if id then pcall(vim.fn.matchdelete, id); vim.w._mindforge_live_search_id = nil end; ",
                    "if query ~= '' then ",
                    "local ok, match_id = pcall(vim.fn.matchadd, 'IncSearch', query, 10); ",
                    "if ok then vim.w._mindforge_live_search_id = match_id end; end"
                )),
                Value::Array(vec![Value::from(query)]),
            ],
        )
    }

    pub fn clear_preview_search(&mut self) -> EditorResult<()> {
        self.client.notify(
            "nvim_exec_lua",
            vec![
                Value::from(
                    "local id = vim.w._mindforge_live_search_id; if id then pcall(vim.fn.matchdelete, id); vim.w._mindforge_live_search_id = nil end",
                ),
                Value::Array(vec![]),
            ],
        )
    }

    pub fn set_document(&mut self, text: &str, row: usize, column: usize) -> EditorResult<()> {
        self.set_document_for_language(text, row, column, "note.md", FileLanguage::Markdown)
    }

    pub fn set_document_for_language(
        &mut self,
        text: &str,
        row: usize,
        column: usize,
        file_name: &str,
        language: FileLanguage,
    ) -> EditorResult<()> {
        self.grid.popup_items.clear();
        self.grid.popup_meta.clear();
        self.grid.popup_info.clear();
        self.grid.popup_selected = None;
        self.grid.popup_anchor = None;

        let _ = self.client.notify(
            "nvim_exec_lua",
            vec![
                Value::from("local ok, m = pcall(require, 'mindforge.lsp'); if ok then m.reset_buffer() end"),
                Value::Array(vec![]),
            ],
        );
        let set_name_lua = r#"
            local name = ...
            pcall(function()
                local existing = vim.fn.bufnr(name)
                if existing ~= -1 and existing ~= vim.api.nvim_get_current_buf() then
                    pcall(vim.api.nvim_buf_delete, existing, { force = true })
                end
                vim.api.nvim_buf_set_name(0, name)
            end)
        "#;
        let _ = self.client.request(
            "nvim_exec_lua",
            vec![
                Value::from(set_name_lua),
                Value::Array(vec![Value::from(file_name.to_owned())]),
            ],
        );
        self.suppress_lines_events = self.suppress_lines_events.saturating_add(1);
        self.client.set_buffer_text_async(text)?;
        let filetype = language.nvim_filetype();
        let _ = self.client.notify(
            "nvim_command",
            vec![Value::from(format!("setlocal filetype={filetype} syntax={filetype} numberwidth=4 signcolumn=no foldcolumn=0"))],
        );
        let _ = self.client.notify(
            "nvim_exec_lua",
            vec![
                Value::from(
                    "local ok, m = pcall(require, 'mindforge.lsp'); if ok then m.refresh() end",
                ),
                Value::Array(vec![]),
            ],
        );
        let _ = self
            .client
            .notify("nvim_command", vec![Value::from("redraw!")]);
        self.lines = text.split('\n').map(str::to_owned).collect();
        self.document_words = text.split_whitespace().count();
        self.document_lines = text.lines().count();
        self.document_bytes = text.len();
        self.lines_revision = self.lines_revision.wrapping_add(1);
        self.cursor_char_cache = None;
        self.client
            .set_cursor_async(row, self.cursor_column_bytes(row, column))?;
        self.text_updated = false;
        self.last_buffer_change = None;
        self.dirty = false;
        self.row_layouts.clear();
        Ok(())
    }

    pub fn set_cursor(&mut self, row: usize, column: usize) -> EditorResult<()> {
        let byte_column = self.cursor_column_bytes(row, column);
        self.client.set_cursor_async(row, byte_column)
    }

    pub fn take_text_update(&mut self) -> Option<String> {
        let settled = self
            .last_buffer_change
            .is_some_and(|last| last.elapsed() >= Duration::from_millis(300));
        if self.text_updated && settled {
            self.text_updated = false;
            self.last_buffer_change = None;
            Some(self.lines.join("\n"))
        } else {
            None
        }
    }

    /// Synchronizes lines from Neovim, bypassing idle debounce (used after Ex-commands).
    pub fn force_sync_text(&mut self) -> Option<String> {
        if let Ok(text) = self.client.get_buffer_text() {
            if text.is_empty()
                && !self.lines.is_empty()
                && (self.lines.len() > 1 || !self.lines[0].is_empty())
            {
                return Some(self.lines.join("\n"));
            }
            self.lines = text.split('\n').map(str::to_owned).collect();
            self.document_words = text.split_whitespace().count();
            self.document_lines = if self.lines.len() == 1 && self.lines[0].is_empty() {
                0
            } else if self.lines.last().is_some_and(String::is_empty) {
                self.lines.len() - 1
            } else {
                self.lines.len()
            };
            self.document_bytes = text.len();
            self.lines_revision = self.lines_revision.wrapping_add(1);
            self.cursor_char_cache = None;
            self.text_updated = false;
            self.last_buffer_change = None;
            Some(self.lines.join("\n"))
        } else {
            None
        }
    }

    /// Cached document statistics updated from Neovim's incremental line events.
    pub fn document_stats(&self) -> (usize, usize, usize) {
        (
            self.document_lines,
            self.document_words,
            self.document_bytes,
        )
    }

    /// Converts Neovim row/column coordinates into document character offset.
    pub fn cursor_char_index(&mut self) -> usize {
        let cursor = self.grid.cursor;
        if let Some((cached_cursor, cached_revision, char_index)) = self.cursor_char_cache {
            if cached_cursor == cursor && cached_revision == self.lines_revision {
                return char_index;
            }
        }
        let row = self.grid.cursor.row.min(self.lines.len().saturating_sub(1));
        let prev_chars: usize = self
            .lines
            .iter()
            .take(row)
            .map(|line| line.chars().count() + 1)
            .sum();
        let col = self.grid.cursor.column;
        let line_char_offset = self
            .lines
            .get(row)
            .map(|line| line.chars().take(col).count())
            .unwrap_or(0);
        let char_index = prev_chars + line_char_offset;
        self.cursor_char_cache = Some((cursor, self.lines_revision, char_index));
        char_index
    }

    pub(crate) fn cursor_column_bytes(&self, row: usize, character_column: usize) -> usize {
        self.lines
            .get(row)
            .map(|line| {
                line.chars()
                    .take(character_column)
                    .map(char::len_utf8)
                    .sum()
            })
            .unwrap_or(0)
    }

    pub fn is_insert_mode(&self) -> bool {
        self.grid.mode.starts_with('i')
    }

    pub fn mark_saved(&mut self) {
        self.dirty = false;
    }

    /// Execute an arbitrary Neovim Ex-command synchronously via MessagePack-RPC.
    pub fn execute_command(&mut self, command: &str) -> Result<String, String> {
        let res = self.client.execute_command(command);
        if let Ok(Value::Array(pos)) = self
            .client
            .request("nvim_win_get_cursor", vec![Value::from(0)])
        {
            if pos.len() >= 2 {
                let win_row = pos[0].as_u64().unwrap_or(1).saturating_sub(1) as usize;
                let win_col = pos[1].as_u64().unwrap_or(0) as usize;
                if win_row < self.lines.len() {
                    let char_col = self
                        .lines
                        .get(win_row)
                        .map(|l| l.char_indices().take_while(|(b, _)| *b < win_col).count())
                        .unwrap_or(0);
                    self.grid.cursor = crate::editor::types::CursorPosition {
                        row: win_row,
                        column: char_col,
                    };
                }
            }
        }
        res
    }

    /// Execute arbitrary Lua code synchronously in Neovim via MessagePack-RPC.
    pub fn execute_lua(&mut self, code: &str) -> Result<String, String> {
        let res = self.client.execute_lua(code, vec![]);
        if let Ok(Value::Array(pos)) = self
            .client
            .request("nvim_win_get_cursor", vec![Value::from(0)])
        {
            if pos.len() >= 2 {
                let win_row = pos[0].as_u64().unwrap_or(1).saturating_sub(1) as usize;
                let win_col = pos[1].as_u64().unwrap_or(0) as usize;
                if win_row < self.lines.len() {
                    let char_col = self
                        .lines
                        .get(win_row)
                        .map(|l| l.char_indices().take_while(|(b, _)| *b < win_col).count())
                        .unwrap_or(0);
                    self.grid.cursor = crate::editor::types::CursorPosition {
                        row: win_row,
                        column: char_col,
                    };
                }
            }
        }
        res
    }
}

pub(crate) fn num(value: &Value) -> usize {
    value.as_u64().unwrap_or(0) as usize
}

pub(crate) fn cmdline_text(value: &Value) -> String {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_array)
        .filter_map(|chunk| {
            chunk
                .get(1)
                .and_then(Value::as_str)
                .or_else(|| chunk.first().and_then(Value::as_str))
        })
        .collect()
}

pub(crate) fn compact_message(text: &str) -> String {
    const MAX: usize = 120;
    let line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if line.chars().count() <= MAX {
        return line;
    }
    let mut out: String = line.chars().take(MAX).collect();
    out.push('…');
    out
}
