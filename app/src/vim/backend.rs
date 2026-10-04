use super::{
    client::NeovimClient,
    state::{GridCell, GridState},
};
use crate::editor::{
    backend::{EditorBackend, EditorResult},
    events::{EditorKeyEvent, EditorMouseEvent},
    types::EditorMode,
};
use crate::language::FileLanguage;
use eframe::egui::{self, Align2, Color32, Id, Pos2, Rect, Sense, Stroke, Ui};
use rmpv::Value;
use std::{sync::Arc, time::{Duration, Instant}};

pub struct VimBackend {
    client: NeovimClient,
    pub grid: GridState,
    lines: Vec<String>,
    lines_revision: u64,
    document_words: usize,
    document_lines: usize,
    document_bytes: usize,
    cursor_char_cache: Option<(crate::editor::types::CursorPosition, u64, usize)>,
    pending_insert_cursor: Option<crate::editor::types::CursorPosition>,
    dirty: bool,
    text_updated: bool,
    last_buffer_change: Option<Instant>,
    error: Option<String>,
    last_size: (usize, usize),
    pending_size: Option<(usize, usize)>,
    resize_pending_since: Option<f64>,
    row_layouts: Vec<Option<(u64, Arc<egui::Galley>)>>,
    layout_font_size: u32,
    cursor_render_initialized: bool,
    busy: Option<String>,
    lsp_status: (String, String),
    lsp_panel: super::lsp_panel::LspPanel,
    line_numbers_enabled: Option<bool>,
    cached_theme: Option<(Color32, Color32, Color32, Color32)>,
}

/// Lua sources of the embedded LSP integration, preloaded into Neovim.
const LSP_MODULES: &[(&str, &str)] = &[
    ("mindforge.lsp", include_str!("../../assets/nvim/mindforge/lsp/init.lua")),
    ("mindforge.lsp.paths", include_str!("../../assets/nvim/mindforge/lsp/paths.lua")),
    ("mindforge.lsp.notify", include_str!("../../assets/nvim/mindforge/lsp/notify.lua")),
    ("mindforge.lsp.progress", include_str!("../../assets/nvim/mindforge/lsp/progress.lua")),
    ("mindforge.lsp.config", include_str!("../../assets/nvim/mindforge/lsp/config.lua")),
    ("mindforge.lsp.registry", include_str!("../../assets/nvim/mindforge/lsp/registry.lua")),
    ("mindforge.lsp.install", include_str!("../../assets/nvim/mindforge/lsp/install.lua")),
    ("mindforge.lsp.emmet", include_str!("../../assets/nvim/mindforge/lsp/emmet.lua")),
    ("mindforge.lsp.snippets", include_str!("../../assets/nvim/mindforge/lsp/snippets.lua")),
    ("mindforge.lsp.completion", include_str!("../../assets/nvim/mindforge/lsp/completion.lua")),
    ("mindforge.lsp.status", include_str!("../../assets/nvim/mindforge/lsp/status.lua")),
    ("mindforge.lsp.commands", include_str!("../../assets/nvim/mindforge/lsp/commands.lua")),
];
impl VimBackend {
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

        // Attach the embedded grid before installing the app's active note so
        // the first buffer update is rendered directly into our editor pane.
        client.attach_ui(width, height)?;
        // Keep shared editor navigation options consistent even when the
        // user's init.lua has absolute numbers or cursorline disabled.
        // Enable true colors, syntax highlighting, and smooth visual line navigation.
        let init_lua = r#"
            vim.cmd([[
                set number relativenumber cursorline numberwidth=4 signcolumn=no laststatus=0 noruler noshowmode virtualedit=onemore guicursor=a:ver1-Cursor/lCursor fillchars+=eob:\ 
                syntax on
                syntax enable
                filetype plugin indent on
                set termguicolors
                hi Cursor NONE
                hi TermCursor NONE
            ]])
            local opts = { noremap = true, silent = true, expr = true }
            vim.keymap.set("n", "j", "v:count == 0 ? 'gj' : 'j'", opts)
            vim.keymap.set("n", "k", "v:count == 0 ? 'gk' : 'k'", opts)
            vim.keymap.set("n", "<Down>", "v:count == 0 ? 'gj' : '<Down>'", opts)
            vim.keymap.set("n", "<Up>", "v:count == 0 ? 'gk' : '<Up>'", opts)

            -- The user's embedded init maps `jk` to Escape in Insert mode.
            -- That prefix mapping delays every literal `j`; remove it only in
            -- this editor instance so ordinary typing is immediate.
            pcall(vim.keymap.del, 'i', 'jk')

            -- Auto-closing pairs so the Neovim surface matches Hybrid typing.
            if vim.g.mindforge_autopair ~= false then
                local function next_char()
                    local col = vim.fn.col('.')
                    return vim.fn.getline('.'):sub(col, col)
                end
                local function prev_char()
                    local col = vim.fn.col('.') - 1
                    if col < 1 then return '' end
                    return vim.fn.getline('.'):sub(col, col)
                end
                local imap = function(lhs, fn)
                    vim.keymap.set('i', lhs, fn, { expr = true, noremap = true, silent = true })
                end
                for open, close in pairs({ ['('] = ')', ['['] = ']', ['{'] = '}' }) do
                    imap(open, function() return open .. close .. '<Left>' end)
                    imap(close, function()
                        if next_char() == close then return '<Right>' end
                        return close
                    end)
                end
                for _, quote in ipairs({ '"', "'", '`' }) do
                    imap(quote, function()
                        if next_char() == quote then return '<Right>' end
                        if prev_char():match('[%w_]') or next_char():match('[%w_]') then return quote end
                        return quote .. quote .. '<Left>'
                    end)
                end
                local pairs_map = { ['('] = ')', ['['] = ']', ['{'] = '}', ['"'] = '"', ["'"] = "'", ['`'] = '`' }
                imap('<BS>', function()
                    local p, n = prev_char(), next_char()
                    if p ~= '' and pairs_map[p] == n then return '<Right><BS><BS>' end
                    return '<BS>'
                end)
            end
        "#;
        let _ = client.request(
            "nvim_exec_lua",
            vec![Value::from(init_lua), Value::Array(vec![])],
        );
        let lsp_boot = r#"
            local mods = ...
            for name, src in pairs(mods) do
                package.preload[name] = assert(load(src, '=' .. name))
            end
            local ok, err = pcall(function() require('mindforge.lsp').setup() end)
            if not ok then vim.g.mindforge_lsp_error = tostring(err) end
        "#;
        let modules = Value::Map(
            LSP_MODULES
                .iter()
                .map(|(name, source)| (Value::from(*name), Value::from(*source)))
                .collect(),
        );        let _ = client.request(
            "nvim_exec_lua",
            vec![Value::from(lsp_boot), Value::Array(vec![modules])],
        );

        let _ = client.request(
            "nvim_buf_set_name",
            vec![Value::from(0), Value::from(file_name.to_owned())],
        );

        // Load text async — no blocking round-trip needed here.
        // nvim_buf_set_lines is a notification (fire and forget).
        client.set_buffer_text_async(text)?;
        let filetype = language.nvim_filetype();
        let _ = client.notify(
            "nvim_command",
            vec![Value::from(format!("setlocal filetype={filetype} syntax={filetype}"))],
        );

        // Attach only for future changes. The local mirror already came from
        // `text`, and the preceding set-lines notification is ordered on the
        // same RPC writer. Requesting the initial buffer dump here would send
        // the entire document back to egui during startup for no reason.
        client.notify(
            "nvim_buf_attach",
            vec![Value::from(0), Value::from(false), Value::Map(vec![])],
        )?;

        // Set initial cursor position async.
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
            pending_insert_cursor: None,
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
        })
    }

    pub fn set_repaint_context(&self, ctx: egui::Context) {
        self.client.set_repaint_context(ctx);
    }

    fn consume_notification(&mut self, packet: &Value) {
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
                items.get(2).and_then(Value::as_array).and_then(|args| args.get(i)).and_then(Value::as_str).unwrap_or("").to_owned()
            };
            self.lsp_status = (arg(0), arg(1));
        } else if items.get(1).and_then(Value::as_str) == Some("mindforge_lsp_list") {
            self.apply_lsp_list(items.get(2));
        } else if items.get(1).and_then(Value::as_str) == Some("nvim_buf_lines_event") {
            self.apply_lines_event(items.get(2).unwrap_or(&Value::Nil));
        }
    }

    fn apply_lsp_list(&mut self, args: Option<&Value>) {
        let Some(args) = args.and_then(Value::as_array) else { return };
        let text = |value: &Value, i: usize| value.as_array().and_then(|r| r.get(i)).and_then(Value::as_str).unwrap_or("").to_owned();
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
        let raw_start = args.get(2).and_then(Value::as_i64).or_else(|| args.get(2).and_then(Value::as_u64).map(|v| v as i64)).unwrap_or(0);
        let start = if raw_start < 0 {
            (self.lines.len() as i64 + 1 + raw_start).max(0) as usize
        } else {
            raw_start as usize
        };
        let raw_end = args.get(3).and_then(Value::as_i64).or_else(|| args.get(3).and_then(Value::as_u64).map(|v| v as i64)).unwrap_or(start as i64);
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
            self.document_words = self.document_words.saturating_sub(old_words).saturating_add(new_words);
            self.document_bytes = self.document_bytes.saturating_sub(old_bytes).saturating_add(new_bytes);
            if added >= removed {
                self.document_bytes = self.document_bytes.saturating_add(added - removed);
            } else {
                self.document_bytes = self.document_bytes.saturating_sub(removed - added);
            }
            self.predict_insert_caret(start, end, &replacement);
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

    /// Neovim sends buffer edits immediately but the matching cursor position
    /// only with the next redraw, which can be delayed by completion work. Move
    /// the caret along with single-line insert-mode edits so it never trails
    /// the typed text; the authoritative `grid_cursor_goto` corrects it later.
    fn predict_insert_caret(&mut self, start: usize, end: usize, replacement: &[String]) {
        if !self.is_insert_mode() || end != start + 1 || replacement.len() != 1 || start != self.grid.cursor.row {
            return;
        }
        // Grid columns are display cells, while `chars().count()` is Unicode
        // scalar count. Predict only for ASCII edits; Unicode uses Neovim's
        // authoritative cursor redraw instead of an invalid local estimate.
        if !self.lines[start].is_ascii() || !replacement[0].is_ascii() {
            self.pending_insert_cursor = None;
            return;
        }
        let old_len = self.lines[start].chars().count();
        let new_len = replacement[0].chars().count();
        let column = self.grid.cursor.column;
        if new_len > old_len && column <= old_len {
            self.grid.cursor.column = (column + new_len - old_len).min(new_len);
        } else if new_len < old_len && column <= old_len {
            self.grid.cursor.column = column.saturating_sub(old_len - new_len).min(new_len);
        }
        if new_len != old_len {
            self.pending_insert_cursor = Some(self.grid.cursor);
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
                // grid_line payload: [grid, row, col_start, cells, wrap?]
                "grid_line" if args.len() >= 4 => self.apply_grid_line(args),
                "grid_scroll" if args.len() >= 7 => self.apply_grid_scroll(args),
                // grid_cursor_goto payload: [grid, row, column]
                "grid_cursor_goto" if args.len() >= 3 && num(&args[0]) == 1 => {
                    let target_row = num(&args[1]);
                    let target_col = num(&args[2]);
                    // A buffer update can arrive before a delayed cursor redraw.
                    // Keep the predicted insertion caret when that redraw points
                    // behind it; navigation keys clear the prediction explicitly.
                    if self.is_insert_mode() {
                        if let Some(predicted) = self.pending_insert_cursor {
                            if cursor_goto_is_stale(self.grid.mode.as_str(), Some(predicted), target_row, target_col) {
                                continue;
                            }
                        }
                    }
                    self.pending_insert_cursor = None;
                    let is_bottom_cmd_row = self.grid.height > 0 && target_row >= self.grid.height.saturating_sub(1);
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
                    // Event payload is [mode_name, mode_index]. The previous
                    // code read mode_index as a string, leaving the UI stuck
                    // in NORMAL even after Neovim entered INSERT or VISUAL.
                    self.grid.mode = args[0].as_str().unwrap_or("normal").to_owned();
                    if !self.is_insert_mode() {
                        self.pending_insert_cursor = None;
                    }
                }
                // hl_attr_define payload: [id, rgb_attrs, cterm_attrs, info]
                "hl_attr_define" if args.len() >= 2 => self.apply_highlight(args),
                "cmdline_show" if !args.is_empty() => {
                    let prefix = args.get(2).and_then(Value::as_str).unwrap_or("");
                    self.grid.command_line = format!("{prefix}{}", cmdline_text(&args[0]));
                }
                "cmdline_hide" => self.grid.command_line.clear(),
                "popupmenu_show" if !args.is_empty() => {
                    self.grid.popup_items = popupmenu_items(&args[0]);
                    self.grid.popup_meta = popupmenu_meta(&args[0]);
                    self.grid.popup_info = popupmenu_info(&args[0]);
                    self.grid.popup_selected = args
                        .get(1)
                        .and_then(Value::as_i64)
                        .filter(|selected| *selected >= 0)
                        .map(|selected| selected as usize);
                    if self.grid.popup_selected.is_none() {
                        // Keep Neovim's actual selection in sync with the first
                        // (best-ranked) visible match. `insert=false` highlights
                        // it without changing the buffer; the redraw event is
                        // the sole source for the UI's selected-row state.
                        let _ = self.client.notify(
                            "nvim_exec_lua",
                            vec![Value::from(concat!(
                                "if vim.fn.mode():sub(1, 1) ~= 'i' or vim.fn.pumvisible() ~= 1 then return end; ",
                                "if vim.fn.complete_info({ 'selected' }).selected == -1 then ",
                                "local ok = pcall(vim.api.nvim_select_popupmenu_item, 0, false, false); ",
                                "if not ok then vim.api.nvim_feedkeys(vim.api.nvim_replace_termcodes('<C-n>', true, false, true), 'n', false) end end"
                            ))],
                        );
                    }
                    self.grid.popup_anchor = Some((
                        num(args.get(2).unwrap_or(&Value::Nil)),
                        num(args.get(3).unwrap_or(&Value::Nil)),
                    ));
                }
                // Arrow / Ctrl+J/K navigation only emits a select event.
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
                    self.grid.message_is_error =
                        matches!(args[0].as_str(), Some("emsg" | "echoerr" | "lua_error" | "rpc_error"));
                    self.grid.message = compact_message(&cmdline_text(&args[1]));
                    self.grid.message_at = Some(std::time::Instant::now());
                }
                "msg_clear" => {
                    let sticky = self.grid.message_is_error
                        && self.grid.message_at.is_some_and(|at| at.elapsed() < Duration::from_secs(8));
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
            let text = cell.first().and_then(super::client::rmpv_value_to_str).unwrap_or_else(|| " ".to_string());
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
                let source_col = col as isize - col_delta - left as isize;
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
        let (mut foreground, mut background, mut reverse) = (None, None, false);
        let mut is_visual = false;
        if let Some(attrs) = args[1].as_map() {
            for (key, value) in attrs {
                let color = value.as_u64().map(|n| n as u32);
                match key.as_str() {
                    Some("foreground") => foreground = color,
                    Some("background") => background = color,
                    Some("reverse") => reverse = value.as_bool().unwrap_or(false),
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
        self.grid.highlights.insert(id, super::state::HighlightStyle { foreground, background, reverse, is_visual });
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
        self.pending_insert_cursor = None;
        self.client.input(input)
    }

    /// (state, detail) of the language servers for the current buffer. State is one of
    /// connected, starting, unavailable or none.
    pub fn lsp_status(&self) -> (&str, &str) {
        (&self.lsp_status.0, &self.lsp_status.1)
    }

    /// Label of long-running background work (installs, LSP indexing), if any.
    pub fn busy_label(&self) -> Option<&str> {
        self.busy.as_deref()
    }

    /// Selects completion item index and accepts it (mouse click on the popup).
    fn accept_popup_item(&mut self, index: usize) {
        let total = self.grid.popup_items.len() as i64;
        if index as i64 >= total {
            return;
        }
        self.pending_insert_cursor = None;
        // Select and finish by exact popup index without blocking the UI. If
        // the bundled Neovim lacks this API, fall back inside Neovim to key
        // navigation while the same popup state is still current.
        let current = self.grid.popup_selected.map_or(-1, |selected| selected as i64);
        let _ = self.client.notify(
            "nvim_exec_lua",
            vec![
                Value::from(concat!(
                    "local index, current = ...; ",
                    "local select_item = vim.api.nvim_select_popupmenu_item; ",
                    "if select_item then local ok = pcall(select_item, index, true, true); if ok then return end end; ",
                    "local steps = current >= 0 and (index - current) or (index + 1); ",
                    "local key = steps >= 0 and '<C-n>' or '<C-p>'; ",
                    "local keys = string.rep(key, math.abs(steps)) .. '<C-y>'; ",
                    "vim.api.nvim_feedkeys(vim.api.nvim_replace_termcodes(keys, true, false, true), 'n', false)"
                )),
                Value::Array(vec![Value::from(index as i64), Value::from(current)]),
            ],
        );
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
            vec![Value::from(
                "local id = vim.w._mindforge_live_search_id; if id then pcall(vim.fn.matchdelete, id); vim.w._mindforge_live_search_id = nil end",
            )],
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
        let _ = self.client.notify(
            "nvim_exec_lua",
            vec![
                Value::from("local ok, m = pcall(require, 'mindforge.lsp'); if ok then m.reset_buffer() end"),
                Value::Array(vec![]),
            ],
        );
        let _ = self.client.request(
            "nvim_buf_set_name",
            vec![Value::from(0), Value::from(file_name.to_owned())],
        );
        self.client.set_buffer_text_async(text)?;
        let filetype = language.nvim_filetype();
        let _ = self.client.notify(
            "nvim_command",
            vec![Value::from(format!("setlocal filetype={filetype} syntax={filetype}"))],
        );
        let _ = self.client.notify("nvim_command", vec![Value::from("redraw!")]);
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
            // Keep the legacy app buffer and its derived features out of the
            // per-keystroke path. A short idle debounce coalesces the many
            // line events Neovim emits during a typing burst into one snapshot.
            .is_some_and(|last| last.elapsed() >= Duration::from_millis(300));
        if self.text_updated && settled {
            self.text_updated = false;
            self.last_buffer_change = None;
            Some(self.lines.join("\n"))
        } else {
            None
        }
    }

    /// Immediately queries the current lines from Neovim and synchronizes the local cache,
    /// bypassing the 300ms idle debounce. Used right after Ex-commands (:sort, :%s, :g/.../d).
    pub fn force_sync_text(&mut self) -> Option<String> {
        self.tick();
        if let Ok(text) = self.client.get_buffer_text() {
            // Guard: If Neovim returned empty string but our cached buffer has content,
            // don't wipe out the buffer if get_buffer_text had a transient timing issue.
            if text.is_empty() && !self.lines.is_empty() && (self.lines.len() > 1 || !self.lines[0].is_empty()) {
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
            self.dirty = true;
            self.row_layouts.clear();
            Some(text)
        } else if self.text_updated || self.dirty {
            self.text_updated = false;
            self.last_buffer_change = None;
            Some(self.lines.join("\n"))
        } else {
            None
        }
    }

    /// Synchronizes MindForge's theme colors and accent into Neovim's highlight definitions
    /// and resets cached text galleys for instant, zero-delay color updates.
    pub fn sync_theme(&mut self, theme: &crate::ui::theme::Theme) {
        let accent_hex = crate::accent::hex_from_color(theme.accent);
        let text_hex = crate::accent::hex_from_color(theme.text);
        let bg_hex = crate::accent::hex_from_color(theme.bg);
        let muted_hex = crate::accent::hex_from_color(theme.muted);
        let surface_hex = crate::accent::hex_from_color(theme.surface());
        let hl_hex = crate::accent::hex_from_color(theme.highlight);
        let is_light = theme.is_light();

        let h1_fg = accent_hex.clone();
        let h2_fg = if is_light { "#d97706" } else { "#e5c07b" };
        let h3_fg = if is_light { "#0284c7" } else { "#61afef" };
        let h4_fg = if is_light { "#16a34a" } else { "#98c379" };
        let h5_fg = if is_light { "#9333ea" } else { "#c678dd" };
        let h6_fg = if is_light { "#0d9488" } else { "#56b6c2" };

        let bold_fg = if is_light { "#b91c1c" } else { "#e06c75" };
        let italic_fg = if is_light { "#0284c7" } else { "#61afef" };
        let code_fg = if is_light { "#059669" } else { "#98c379" };
        let stmt_fg = if is_light { "#9333ea" } else { "#c678dd" };
        let ident_fg = if is_light { "#0284c7" } else { "#61afef" };
        let str_fg = if is_light { "#16a34a" } else { "#98c379" };
        let url_fg = if is_light { "#0284c7" } else { "#56b6c2" };
        let type_fg = if is_light { "#ea580c" } else { "#e5c07b" };

        let lua_code = format!(
            r##"
            vim.opt.termguicolors = true
            vim.opt.background = "{bg_mode}"
            local hls = {{
                Normal = {{ fg = "{text}", bg = "{bg}" }},
                NormalNC = {{ fg = "{text}", bg = "{bg}" }},
                CursorLine = {{ bg = "{surface}" }},
                Visual = {{ bg = "{hl}" }},
                Search = {{ fg = "{bg}", bg = "{accent}" }},
                CurSearch = {{ fg = "{bg}", bg = "{accent}", bold = true }},
                Title = {{ fg = "{h1_fg}", bold = true }},
                markdownH1 = {{ fg = "{h1_fg}", bold = true }},
                markdownH2 = {{ fg = "{h2_fg}", bold = true }},
                markdownH3 = {{ fg = "{h3_fg}", bold = true }},
                markdownH4 = {{ fg = "{h4_fg}", bold = true }},
                markdownH5 = {{ fg = "{h5_fg}", bold = true }},
                markdownH6 = {{ fg = "{h6_fg}", bold = true }},
                htmlH1 = {{ fg = "{h1_fg}", bold = true }},
                htmlH2 = {{ fg = "{h2_fg}", bold = true }},
                htmlH3 = {{ fg = "{h3_fg}", bold = true }},
                htmlH4 = {{ fg = "{h4_fg}", bold = true }},
                htmlH5 = {{ fg = "{h5_fg}", bold = true }},
                htmlH6 = {{ fg = "{h6_fg}", bold = true }},
                markdownHeadingDelimiter = {{ fg = "{accent}", bold = true }},
                markdownH1Delimiter = {{ fg = "{h1_fg}", bold = true }},
                markdownH2Delimiter = {{ fg = "{h2_fg}", bold = true }},
                markdownH3Delimiter = {{ fg = "{h3_fg}", bold = true }},
                markdownH4Delimiter = {{ fg = "{h4_fg}", bold = true }},
                markdownH5Delimiter = {{ fg = "{h5_fg}", bold = true }},
                markdownH6Delimiter = {{ fg = "{h6_fg}", bold = true }},
                markdownHeadingRule = {{ fg = "{muted}" }},
                markdownBold = {{ fg = "{bold_fg}", bold = true }},
                htmlBold = {{ fg = "{bold_fg}", bold = true }},
                markdownItalic = {{ fg = "{italic_fg}", italic = true }},
                htmlItalic = {{ fg = "{italic_fg}", italic = true }},
                markdownBoldItalic = {{ fg = "{bold_fg}", bold = true, italic = true }},
                htmlBoldItalic = {{ fg = "{bold_fg}", bold = true, italic = true }},
                markdownCode = {{ fg = "{code_fg}", bg = "{surface}" }},
                markdownCodeBlock = {{ fg = "{code_fg}", bg = "{surface}" }},
                markdownCodeDelimiter = {{ fg = "{muted}" }},
                markdownBlockquote = {{ fg = "{muted}", italic = true }},
                markdownListMarker = {{ fg = "{accent}", bold = true }},
                markdownOrderedListMarker = {{ fg = "{accent}" }},
                markdownRule = {{ fg = "{muted}" }},
                markdownUrl = {{ fg = "{url_fg}", underline = true }},
                markdownLinkText = {{ fg = "{accent}", underline = true }},
                markdownLink = {{ fg = "{muted}" }},
                markdownId = {{ fg = "{accent}" }},
                markdownIdDeclaration = {{ fg = "{accent}" }},
                markdownAutomaticLink = {{ fg = "{url_fg}", underline = true }},
                ["@markup.heading"] = {{ fg = "{h1_fg}", bold = true }},
                ["@markup.heading.1"] = {{ fg = "{h1_fg}", bold = true }},
                ["@markup.heading.2"] = {{ fg = "{h2_fg}", bold = true }},
                ["@markup.heading.3"] = {{ fg = "{h3_fg}", bold = true }},
                ["@markup.heading.4"] = {{ fg = "{h4_fg}", bold = true }},
                ["@markup.heading.5"] = {{ fg = "{h5_fg}", bold = true }},
                ["@markup.heading.6"] = {{ fg = "{h6_fg}", bold = true }},
                ["@markup.heading.1.markdown"] = {{ fg = "{h1_fg}", bold = true }},
                ["@markup.heading.2.markdown"] = {{ fg = "{h2_fg}", bold = true }},
                ["@markup.heading.3.markdown"] = {{ fg = "{h3_fg}", bold = true }},
                ["@markup.heading.4.markdown"] = {{ fg = "{h4_fg}", bold = true }},
                ["@markup.heading.5.markdown"] = {{ fg = "{h5_fg}", bold = true }},
                ["@markup.heading.6.markdown"] = {{ fg = "{h6_fg}", bold = true }},
                ["@markup.strong"] = {{ fg = "{bold_fg}", bold = true }},
                ["@markup.italic"] = {{ fg = "{italic_fg}", italic = true }},
                ["@markup.raw"] = {{ fg = "{code_fg}", bg = "{surface}" }},
                ["@markup.raw.block"] = {{ fg = "{code_fg}", bg = "{surface}" }},
                ["@markup.quote"] = {{ fg = "{muted}", italic = true }},
                ["@markup.list"] = {{ fg = "{accent}", bold = true }},
                ["@markup.list.checked"] = {{ fg = "{muted}" }},
                ["@markup.list.unchecked"] = {{ fg = "{accent}", bold = true }},
                ["@markup.link.url"] = {{ fg = "{url_fg}", underline = true }},
                ["@markup.link.label"] = {{ fg = "{accent}", underline = true }},
                Comment = {{ fg = "{muted}", italic = true }},
                Statement = {{ fg = "{stmt_fg}" }},
                Identifier = {{ fg = "{ident_fg}" }},
                Type = {{ fg = "{type_fg}" }},
                Special = {{ fg = "{accent}" }},
                String = {{ fg = "{str_fg}" }},
            }}
            for name, opts in pairs(hls) do
                pcall(vim.api.nvim_set_hl, 0, name, opts)
            end
            pcall(vim.cmd, "redraw!")
            "##,
            bg_mode = if is_light { "light" } else { "dark" },
            text = text_hex,
            bg = bg_hex,
            surface = surface_hex,
            hl = hl_hex,
            accent = accent_hex,
            muted = muted_hex,
            h1_fg = h1_fg,
            h2_fg = h2_fg,
            h3_fg = h3_fg,
            h4_fg = h4_fg,
            h5_fg = h5_fg,
            h6_fg = h6_fg,
            bold_fg = bold_fg,
            italic_fg = italic_fg,
            code_fg = code_fg,
            stmt_fg = stmt_fg,
            ident_fg = ident_fg,
            str_fg = str_fg,
            url_fg = url_fg,
            type_fg = type_fg,
        );
        let _ = self.client.notify("nvim_exec_lua", vec![Value::from(lua_code), Value::Array(vec![])]);
        self.row_layouts.clear();
        for revision in &mut self.grid.row_revision {
            *revision = revision.wrapping_add(1);
        }
    }

    /// Cached document statistics updated from Neovim's incremental line events.
    /// This keeps the per-frame status bar independent of full-buffer copies.
    pub fn document_stats(&self) -> (usize, usize, usize) {
        (self.document_lines, self.document_words, self.document_bytes)
    }

    /// Converts Neovim row/column coordinates into the document character offset.
    /// In ext_linegrid, cursor.column is a screen column index (0-based characters),
    /// NOT a byte offset.
    pub fn cursor_char_index(&mut self) -> usize {
        let cursor = self.grid.cursor;
        if let Some((cached_cursor, cached_revision, char_index)) = self.cursor_char_cache {
            if cached_cursor == cursor && cached_revision == self.lines_revision {
                return char_index;
            }
        }
        let row = self.grid.cursor.row.min(self.lines.len().saturating_sub(1));
        let prev_chars: usize = self.lines.iter().take(row).map(|line| line.chars().count() + 1).sum();
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

    fn cursor_column_bytes(&self, row: usize, character_column: usize) -> usize {
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
        if let Ok(Value::Array(pos)) = self.client.request("nvim_win_get_cursor", vec![Value::from(0)]) {
            if pos.len() >= 2 {
                let win_row = pos[0].as_u64().unwrap_or(1).saturating_sub(1) as usize;
                let win_col = pos[1].as_u64().unwrap_or(0) as usize;
                if win_row < self.lines.len() {
                    let char_col = self.lines.get(win_row)
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
        if let Ok(Value::Array(pos)) = self.client.request("nvim_win_get_cursor", vec![Value::from(0)]) {
            if pos.len() >= 2 {
                let win_row = pos[0].as_u64().unwrap_or(1).saturating_sub(1) as usize;
                let win_col = pos[1].as_u64().unwrap_or(0) as usize;
                if win_row < self.lines.len() {
                    let char_col = self.lines.get(win_row)
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

fn num(value: &Value) -> usize {
    value.as_u64().unwrap_or(0) as usize
}

fn rgb(value: u32) -> Color32 {
    Color32::from_rgb((value >> 16) as u8, (value >> 8) as u8, value as u8)
}

fn cmdline_text(value: &Value) -> String {
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

/// Plain-text form of LSP documentation: drops Markdown code fences and
/// inline backticks, caps the length, and trims blank edges.
fn clean_documentation(body: &str) -> String {
    let text: Vec<&str> = body
        .lines()
        .filter(|line| !line.trim_start().starts_with("```"))
        .collect();
    let joined = text.join("\n").replace('`', "").replace('\t', "  ");
    let trimmed = joined.trim();
    if trimmed.chars().count() > 1200 {
        trimmed.chars().take(1199).chain(std::iter::once('…')).collect()
    } else {
        trimmed.to_string()
    }
}

fn popupmenu_items(value: &Value) -> Vec<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_array)
        .filter_map(|item| item.first().and_then(Value::as_str))
        .map(str::to_owned)
        .collect()
}

fn popupmenu_info(value: &Value) -> Vec<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_array)
        .map(|item| item.get(3).and_then(Value::as_str).unwrap_or("").to_owned())
        .collect()
}

/// Maps the LSP completion kind label to a short glyph column.
fn popupmenu_meta(value: &Value) -> Vec<(String, String)> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_array)
        .map(|item| {
            let text = |i: usize| item.get(i).and_then(Value::as_str).unwrap_or("");
            let kind = match text(1) {
                "Text" => "t",
                "Method" | "Function" | "Constructor" => "ƒ",
                "Field" | "Property" => "p",
                "Variable" | "Value" | "Unit" => "x",
                "Class" | "TypeParameter" => "C",
                "Interface" => "I",
                "Module" | "Folder" => "M",
                "Struct" => "S",
                "Enum" | "EnumMember" => "E",
                "Constant" => "c",
                "Keyword" => "k",
                "Snippet" => "▣",
                "Event" => "e",
                "Operator" => "±",
                "Reference" | "Color" => "&",
                "File" => "f",
                "" => "",
                _ => "·",
            };            (kind.to_owned(), text(2).to_owned())
        })
        .collect()
}

fn mouse_button(button: u8) -> &'static str {
    match button {
        1 => "middle",
        2 => "right",
        _ => "left",
    }
}

impl EditorBackend for VimBackend {
    fn handle_key(&mut self, event: EditorKeyEvent) -> EditorResult<()> {
        // Any explicit key command (navigation, deletion, completion accept,
        // etc.) supersedes the optimistic cursor position from text input.
        self.pending_insert_cursor = None;
        if self.lsp_panel.open {
            return match self.lsp_panel.handle_key(event.key, event.modifiers) {
                super::lsp_panel::PanelAction::Run(keys) => self.client.input(&keys),
                super::lsp_panel::PanelAction::None => Ok(()),
            };
        }
        let mut key = match event.key {
            egui::Key::Escape => "<Esc>".to_owned(),
            egui::Key::Enter => "<CR>".into(),
            egui::Key::Tab => "<Tab>".into(),
            egui::Key::Backspace => "<BS>".into(),
            egui::Key::Delete => "<Del>".into(),
            egui::Key::ArrowLeft => "<Left>".into(),
            egui::Key::ArrowRight => "<Right>".into(),
            egui::Key::ArrowUp => "<Up>".into(),
            egui::Key::ArrowDown => "<Down>".into(),
            egui::Key::Home => "<Home>".into(),
            egui::Key::End => "<End>".into(),
            egui::Key::PageUp => "<PageUp>".into(),
            egui::Key::PageDown => "<PageDown>".into(),
            other => {
                let name = format!("{other:?}");
                if name.starts_with('F') && name[1..].parse::<u8>().is_ok() {
                    format!("<{name}>")
                } else {
                    name.to_lowercase()
                }
            }
        };
        if event.modifiers.ctrl || event.modifiers.command {
            let key_name = key.trim_matches(['<', '>']);
            key = format!("<C-{key_name}>");
        } else if event.modifiers.alt {
            let key_name = key.trim_matches(['<', '>']);
            key = format!("<A-{key_name}>");
        } else if event.modifiers.shift
            && matches!(
                key.as_str(),
                "<Tab>"
                    | "<Left>"
                    | "<Right>"
                    | "<Up>"
                    | "<Down>"
                    | "<Home>"
                    | "<End>"
                    | "<PageUp>"
                    | "<PageDown>"
            )
        {
            let key_name = key.trim_matches(['<', '>']);
            key = format!("<S-{key_name}>");
        }
        self.client.input(&key)
    }

    fn handle_text(&mut self, text: &str) -> EditorResult<()> {
        if self.lsp_panel.open {
            self.lsp_panel.handle_text(text);
            return Ok(());
        }
        self.client.input_text(text)
    }

    fn handle_mouse(&mut self, event: EditorMouseEvent) -> EditorResult<()> {
        match event {
            EditorMouseEvent::Press {
                row,
                column,
                button,
            } => self
                .client
                .input_mouse(mouse_button(button), "press", "", 0, row, column),
            EditorMouseEvent::Drag { row, column } => {
                self.client.input_mouse("left", "drag", "", 0, row, column)
            }
            EditorMouseEvent::Release {
                row,
                column,
                button,
            } => self
                .client
                .input_mouse(mouse_button(button), "release", "", 0, row, column),
        }
    }

    /// Paints Neovim's grid strictly inside the application's existing editor content rectangle.
    fn render_in_rect(
        &mut self,
        ui: &mut Ui,
        rect: Rect,
        font_size: f32,
        cell_width: f32,
        row_height: f32,
        theme: &crate::ui::theme::Theme,
        caret: &mut crate::caret::Caret,
        dt: f32,
        typed: bool,
        show_line_numbers: bool,
    ) {
        self.tick();
        let font = crate::services::font_manager::editor_font_id(font_size);
        if self.layout_font_size != font_size.to_bits() {
            self.row_layouts.clear();
            self.layout_font_size = font_size.to_bits();
        }

        // Use the same line metric as Hybrid so row positions and overlays do
        // not jump when switching editor backends.
        let nvim_row_height = row_height.max(1.0);
        // Match the Hybrid editor's content inset so Neovim's grid occupies
        // the same text surface instead of starting at the card's top-left.
        let total_lines = self.lines.len().max(1);
        let digits = total_lines.to_string().len().max(2);
        let gutter_w = if show_line_numbers {
            (digits as f32 * (font_size * 0.55) + 14.0).max(28.0)
        } else {
            0.0
        };
        let effective_gutter_w = if rect.width() > gutter_w + 40.0 { gutter_w } else { 0.0 };
        let pad_x = if show_line_numbers { 16.0 } else { 24.0 };
        let text_left = rect.min.x + effective_gutter_w + pad_x;
        // Reserve enough native grid cells for relative numbers on large files
        // (250 lines can require three digits), then draw the visible label in
        // Hybrid's gutter.
        let number_columns = if show_line_numbers && effective_gutter_w > 0.0 { 4 } else { 0 };
        if self.line_numbers_enabled != Some(show_line_numbers) {
            let options = if show_line_numbers {
                "set number relativenumber cursorline numberwidth=4 signcolumn=no laststatus=0 noruler noshowmode virtualedit=onemore guicursor=a:ver1-Cursor/lCursor fillchars+=eob:\\ \nhi Cursor NONE\nhi TermCursor NONE"
            } else {
                "set nonumber norelativenumber cursorline signcolumn=no laststatus=0 noruler noshowmode virtualedit=onemore guicursor=a:ver1-Cursor/lCursor fillchars+=eob:\\ \nhi Cursor NONE\nhi TermCursor NONE"
            };
            let _ = self.client.notify("nvim_command", vec![Value::from(options)]);
            self.line_numbers_enabled = Some(show_line_numbers);
        }
        // Keep Neovim's number cells in its grid, but align document text
        // with Hybrid and paint the gutter using the app's typography/colors.
        let origin = Pos2::new(text_left - number_columns as f32 * cell_width, rect.min.y + 10.0);
        let content_rect = Rect::from_min_max(origin, rect.max);
        let width = ((rect.max.x - text_left) / cell_width.max(1.0)).floor().max(1.0) as usize
            + number_columns;
        let height = (content_rect.height() / nvim_row_height).floor().max(1.0) as usize;
        let now = ui.input(|i| i.time);

        if (width, height) != self.last_size {
            if self.pending_size != Some((width, height)) {
                self.pending_size = Some((width, height));
                self.resize_pending_since = Some(now);
            }
        } else {
            self.pending_size = None;
            self.resize_pending_since = None;
        }

        if let (Some(size), Some(since)) = (self.pending_size, self.resize_pending_since) {
            if now - since >= 0.05 {
                if let Err(error) = self.client.resize(size.0, size.1) {
                    self.error = Some(error);
                } else {
                    self.last_size = size;
                    self.pending_size = None;
                    self.resize_pending_since = None;
                }
            }
        }

        let response = ui.interact(rect, Id::new("neovim_editor_surface"), Sense::click_and_drag());
        if response.clicked() {
            response.request_focus();
        }

        // Clip all painter output strictly to the editor rectangle
        let painter = ui.painter().with_clip_rect(rect);

        // Keep Neovim syntax highlighting and accent color in sync with MindForge theme
        let current_theme_colors = (theme.accent, theme.text, theme.bg, theme.highlight);
        if self.cached_theme != Some(current_theme_colors) {
            self.cached_theme = Some(current_theme_colors);
            self.sync_theme(theme);
        }

        if self.grid.width == 0 || self.grid.height == 0 {
            painter.text(
                    Pos2::new(origin.x + number_columns as f32 * cell_width, origin.y),
                Align2::LEFT_TOP,
                self.error.as_deref().unwrap_or("Starting Neovim…"),
                font.clone(),
                theme.text,
            );
        }

        let rows = self.grid.cells.len().min(height);
        self.row_layouts.resize_with(rows, || None);

        for row_idx in 0..rows {
            let row = &self.grid.cells[row_idx];
            let revision = self.grid.row_revision.get(row_idx).copied().unwrap_or(0);
            let is_last_row = row_idx + 1 >= rows;
            let is_mode_msg = is_last_row && row.iter().skip(number_columns).any(|c| c.text.contains("--"));
            let row_font = if is_mode_msg {
                crate::services::font_manager::editor_font_id(13.0)
            } else {
                font.clone()
            };
            if self.row_layouts[row_idx]
                .as_ref()
                .is_none_or(|(cached_revision, _)| *cached_revision != revision)
            {
                let mut job = egui::text::LayoutJob::default();
                for (col_offset, cell) in row.iter().skip(number_columns).take(width.saturating_sub(number_columns)).enumerate() {
                    let col_idx = number_columns + col_offset;
                    let is_cursor_cell = row_idx == self.grid.cursor.row && col_idx == self.grid.cursor.column;
                    let mut foreground = theme.text;
                    let mut format = egui::TextFormat {
                        font_id: row_font.clone(),
                        color: foreground,
                        background: Color32::TRANSPARENT,
                        ..Default::default()
                    };
                    if let Some(style) = self.grid.highlights.get(&cell.highlight) {
                        if style.reverse {
                            if !is_cursor_cell || caret.kind == crate::caret::CaretKind::Block {
                                foreground = style.background.map(rgb).unwrap_or(theme.bg);
                            }
                        } else if let Some(color) = style.foreground {
                            foreground = rgb(color);
                        }
                    }
                    format.color = foreground;
                    job.append(&cell.text, 0.0, format);
                }
                self.row_layouts[row_idx] = Some((revision, painter.layout_job(job)));
            }
            let font_row_height = self.row_layouts.iter()
                .filter_map(|opt| opt.as_ref().map(|(_, g)| g.size().y))
                .next()
                .unwrap_or(font_size * 1.25);
            let y_offset = ((nvim_row_height - font_row_height) * 0.5).max(0.0).round();

            let row_top = if is_mode_msg {
                rect.max.y - 20.0
            } else {
                origin.y + row_idx as f32 * nvim_row_height
            };
            let row_h = if is_mode_msg { 20.0 } else { nvim_row_height + 0.5 };

            // Render contiguous cell highlight backgrounds (e.g. visual selection) with full row height
            // to completely eliminate gaps between consecutive lines and paragraphs.
            let mut span_start: Option<(usize, Color32)> = None;
            for (col_offset, cell) in row.iter().skip(number_columns).take(width.saturating_sub(number_columns)).enumerate() {
                let col_idx = number_columns + col_offset;
                let is_cursor_cell = row_idx == self.grid.cursor.row && col_idx == self.grid.cursor.column;
                let mut bg_color = None;
                if let Some(style) = self.grid.highlights.get(&cell.highlight) {
                    if style.reverse {
                        if !is_cursor_cell || caret.kind == crate::caret::CaretKind::Block {
                            bg_color = Some(style.foreground.map(rgb).unwrap_or(theme.text));
                        }
                    } else if let Some(color) = style.background {
                        let c = rgb(color);
                        let is_sel = style.is_visual
                            || (c.r() == theme.highlight.r() && c.g() == theme.highlight.g() && c.b() == theme.highlight.b());
                        if is_sel {
                            let alpha = if theme.is_light() { 55 } else { 40 };
                            bg_color = Some(Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha));
                        } else {
                            bg_color = Some(c);
                        }
                    }
                }
                match (span_start, bg_color) {
                    (Some((_start_col, cur_bg)), Some(new_bg)) if cur_bg == new_bg => {
                        // continue contiguous span
                    }
                    (Some((start_col, cur_bg)), _) => {
                        let x1 = text_left + start_col as f32 * cell_width;
                        let x2 = text_left + col_offset as f32 * cell_width;
                        painter.rect_filled(
                            Rect::from_min_max(Pos2::new(x1, row_top), Pos2::new(x2, row_top + row_h)),
                            0.0,
                            cur_bg,
                        );
                        span_start = bg_color.map(|c| (col_offset, c));
                    }
                    (None, Some(new_bg)) => {
                        span_start = Some((col_offset, new_bg));
                    }
                    (None, None) => {}
                }
            }
            if let Some((start_col, cur_bg)) = span_start {
                let max_col = width.saturating_sub(number_columns);
                let x1 = text_left + start_col as f32 * cell_width;
                let x2 = text_left + max_col as f32 * cell_width;
                painter.rect_filled(
                    Rect::from_min_max(Pos2::new(x1, row_top), Pos2::new(x2, row_top + row_h)),
                    0.0,
                    cur_bg,
                );
            }

            if let Some((_, galley)) = &self.row_layouts[row_idx] {
                let y_pos = if is_mode_msg {
                    rect.max.y - 20.0
                } else {
                    origin.y + row_idx as f32 * nvim_row_height + y_offset
                };
                painter.galley(
                    Pos2::new(text_left, y_pos),
                    Arc::clone(galley),
                    theme.text,
                );
            }
            if number_columns > 0 && !is_mode_msg {
                let number = row.iter().take(number_columns).map(|cell| cell.text.as_str()).collect::<String>();
                let number = number.trim();
                let current = row_idx == self.grid.cursor.row;
                let number_color = if current {
                    theme.accent
                } else if theme.is_light() {
                    theme.muted
                } else {
                    Color32::from_rgba_unmultiplied(theme.muted.r(), theme.muted.g(), theme.muted.b(), 100)
                };
                let gutter_painter = painter.with_clip_rect(Rect::from_min_max(
                    rect.min,
                    Pos2::new(rect.min.x + gutter_w, rect.max.y),
                ));
                let label = if number.is_empty() {
                    let has_text = row.iter().skip(number_columns).any(|cell| !cell.text.trim().is_empty());
                    if has_text { "·" } else { "" }
                } else {
                    number
                };
                if !label.is_empty() {
                    gutter_painter.text(
                        Pos2::new(rect.min.x + gutter_w - 4.0, origin.y + row_idx as f32 * nvim_row_height + y_offset),
                        Align2::RIGHT_TOP,
                        label,
                        crate::services::font_manager::editor_font_id(font_size * 0.82),
                        number_color,
                    );
                }
            }
        }

        // Render the configured application caret at Neovim's grid cursor.
        // The caret height and vertical position strictly match the select background line height.
        let font_row_height = self.row_layouts.iter()
            .filter_map(|opt| opt.as_ref().map(|(_, g)| g.size().y))
            .next()
            .unwrap_or(font_size * 1.25);
        let y_offset = ((nvim_row_height - font_row_height) * 0.5).max(0.0).round();

        let cursor_row = self.grid.cursor.row;
        let cursor_column = self.grid.cursor.column;
        if cursor_row < rows && cursor_column < width {
            let is_visual = self.grid.mode == "visual" || self.grid.mode.starts_with('v') || self.grid.mode.starts_with('V') || self.grid.mode == "\x16";
            let visual_offset_x = if is_visual && caret.kind != crate::caret::CaretKind::Block {
                let next_col = cursor_column + 1;
                let current_highlight = self.grid.cells.get(cursor_row).and_then(|r| r.get(cursor_column)).map(|c| c.highlight).unwrap_or(0);
                let next_is_same_highlight = if next_col < width {
                    self.grid.cells.get(cursor_row).and_then(|r| r.get(next_col)).map(|c| c.highlight) == Some(current_highlight)
                } else {
                    false
                };
                if next_is_same_highlight {
                    0.0
                } else {
                    cell_width
                }
            } else {
                0.0
            };
            let cursor_x = origin.x + cursor_column as f32 * cell_width + visual_offset_x;
            let cursor_y = origin.y + cursor_row as f32 * nvim_row_height;
            let target = Pos2::new(cursor_x, cursor_y);
            if !self.cursor_render_initialized {
                caret.pos = target;
                self.cursor_render_initialized = true;
                response.request_focus();
            }
            if self.is_insert_mode() || self.grid.mode.starts_with('r') {
                caret.pos = target;
                caret.gliding = false;
            }
            caret.update(dt, target, typed && (self.is_insert_mode() || self.grid.mode.starts_with('r')), now, cell_width, nvim_row_height);
            caret.paint(&painter, cell_width, nvim_row_height, now, theme.accent, theme.is_light());

            // A block caret is translucent; repaint its character in the app
            // font so Neovim's Cursor highlight cannot change the glyph size.
            if caret.kind == crate::caret::CaretKind::Block {
                if let Some(cell) = self.grid.cells.get(cursor_row).and_then(|row| row.get(cursor_column)) {
                    if !cell.text.trim().is_empty() {
                        painter.text(
                            Pos2::new(caret.pos.x, caret.pos.y + y_offset),
                            Align2::LEFT_TOP,
                            &cell.text,
                            font.clone(),
                            theme.text,
                        );
                    }
                }
            }
        }

        // The app's LunaLine owns command entry; do not paint Neovim's
        // captured command line beneath the editor as a second prompt.
        // If a message is displayed, use a fixed font size so it never zooms in or out.
        if !self.grid.message.is_empty() {
            let fixed_font = crate::services::font_manager::editor_font_id(13.0);
            painter.text(
                Pos2::new(origin.x, rect.max.y - 20.0),
                Align2::LEFT_TOP,
                &self.grid.message,
                fixed_font,
                theme.highlight,
            );
        }

        // Popup completion menu
        let mut popup_hit: Option<(Rect, usize)> = None;
        let hover_pos = response.hover_pos();
        if !self.grid.popup_items.is_empty() {
            const MAX_ROWS: usize = 12;
            let total = self.grid.popup_items.len();
            let visible = total.min(MAX_ROWS);
            let selected = self.grid.popup_selected;
            let start = selected.map_or(0, |s| (s + 1).saturating_sub(visible)).min(total - visible);
            const DETAIL_CHAR: f32 = 6.4;
            let label_chars = self.grid.popup_items.iter().map(|s| s.chars().count()).max().unwrap_or(0);
            let detail_chars = self.grid.popup_meta.iter().map(|(_, e)| e.chars().count()).max().unwrap_or(0).min(40);
            let wanted = 30.0 + label_chars as f32 * cell_width + 28.0 + detail_chars as f32 * DETAIL_CHAR;
            let popup_width = wanted.clamp(200.0, 520.0).min(rect.width());
            let popup_height = (visible as f32 * nvim_row_height).min(rect.height());
            let (popup_row, popup_col) = self
                .grid
                .popup_anchor
                .unwrap_or((self.grid.cursor.row, self.grid.cursor.column));
            let popup_x = (origin.x + popup_col as f32 * cell_width)
                .clamp(origin.x, (rect.max.x - popup_width).max(origin.x));
            let below_y = origin.y + (popup_row + 1) as f32 * nvim_row_height;
            let popup_y = if below_y + popup_height <= rect.max.y {
                below_y
            } else {
                (origin.y + popup_row as f32 * nvim_row_height - popup_height).max(origin.y)
            };
            let popup_rect = Rect::from_min_size(
                Pos2::new(popup_x, popup_y),
                egui::vec2(popup_width, popup_height),
            );
            popup_hit = Some((popup_rect, start));
            let small = crate::services::font_manager::editor_font_id(11.0);
            painter.rect_filled(popup_rect.translate(egui::vec2(0.0, 4.0)), 8.0, Color32::from_black_alpha(65));
            painter.rect(popup_rect, 8.0, theme.surface(), Stroke::new(1.0, theme.border()), egui::StrokeKind::Inside);
            for (row, index) in (start..start + visible).enumerate() {
                let item = &self.grid.popup_items[index];
                let (kind, extra) = self
                    .grid
                    .popup_meta
                    .get(index)
                    .map(|(k, e)| (k.as_str(), e.as_str()))
                    .unwrap_or(("", ""));
                let y = popup_y + row as f32 * nvim_row_height;
                let hovered = hover_pos.is_some_and(|p| {
                    p.x >= popup_x && p.x <= popup_x + popup_width && p.y >= y && p.y < y + nvim_row_height
                });
                if hovered && Some(index) != selected {
                    painter.rect_filled(
                        Rect::from_min_size(Pos2::new(popup_x, y), egui::vec2(popup_width, nvim_row_height)),
                        0.0,
                        theme.text.linear_multiply(0.05),
                    );
                }
                if Some(index) == selected {
                    painter.rect_filled(
                        Rect::from_min_size(
                            Pos2::new(popup_x, y),
                            egui::vec2(popup_width, nvim_row_height),
                        ),
                        3.0,
                        theme.text.linear_multiply(0.09),
                    );
                    painter.rect_filled(
                        Rect::from_min_size(Pos2::new(popup_x + 2.0, y + 3.0), egui::vec2(2.0, nvim_row_height - 6.0)),
                        1.0,
                        theme.accent,
                    );
                }
                let mid_y = y + nvim_row_height * 0.5;
                let mut text_x = popup_x + 8.0;
                if !kind.is_empty() {
                    painter.text(Pos2::new(text_x + 6.0, mid_y), Align2::CENTER_CENTER, kind, small.clone(), theme.accent);
                    text_x += 18.0;
                }
                let label_color = if Some(index) == selected { theme.accent } else { theme.text };
                painter.text(Pos2::new(text_x, mid_y), Align2::LEFT_CENTER, item, font.clone(), label_color);
                let room = popup_rect.max.x - 12.0 - (text_x + item.chars().count() as f32 * cell_width + 16.0);
                let fit = (room / DETAIL_CHAR).floor().max(0.0) as usize;
                let shown: String = if extra.chars().count() > fit && fit > 1 {
                    extra.chars().take(fit - 1).chain(std::iter::once('…')).collect()
                } else if fit == 0 {
                    String::new()
                } else {
                    extra.to_string()
                };
                if !shown.is_empty() {
                    painter.text(
                        Pos2::new(popup_rect.max.x - 8.0, mid_y),
                        Align2::RIGHT_CENTER,
                        shown,
                        small.clone(),
                        theme.text.linear_multiply(0.5),
                    );
                }
            }
            // Detail pane for the highlighted item, like the hover preview.
            let focus = selected.or_else(|| hover_pos.and_then(|p| {
                (popup_rect.contains(p)).then(|| start + ((p.y - popup_rect.min.y) / nvim_row_height) as usize)
            })).filter(|index| *index < total).unwrap_or(0);
            let body = self.grid.popup_info.get(focus).map(String::as_str).unwrap_or("");
            let doc = clean_documentation(body);
            if !doc.is_empty() {
                let space_right = rect.max.x - popup_rect.max.x - 6.0;
                let space_left = popup_rect.min.x - rect.min.x - 6.0;
                let on_right = space_right >= 200.0 || space_right >= space_left;
                let pane_width = (if on_right { space_right } else { space_left }).clamp(120.0, 360.0);
                if pane_width >= 120.0 {
                    let galley = painter.layout(doc, small.clone(), theme.text.linear_multiply(0.85), pane_width - 20.0);
                    let max_height = (nvim_row_height * 14.0).min(rect.height());
                    let pane_height = (galley.size().y + 16.0).min(max_height);
                    let pane_x = if on_right { popup_rect.max.x + 6.0 } else { popup_rect.min.x - pane_width - 6.0 };
                    let pane_y = popup_y.min((rect.max.y - pane_height).max(rect.min.y));
                    let pane = Rect::from_min_size(Pos2::new(pane_x, pane_y), egui::vec2(pane_width, pane_height));
                    painter.rect_filled(pane.translate(egui::vec2(0.0, 4.0)), 8.0, Color32::from_black_alpha(65));
                    painter.rect(pane, 8.0, theme.surface(), Stroke::new(1.0, theme.border()), egui::StrokeKind::Inside);
                    painter
                        .with_clip_rect(pane.shrink(1.0))
                        .galley(Pos2::new(pane.min.x + 10.0, pane.min.y + 8.0), galley, theme.text);
                }
            }
            if total > visible {
                let track = Rect::from_min_max(
                    Pos2::new(popup_rect.max.x - 4.0, popup_rect.min.y + 4.0),
                    Pos2::new(popup_rect.max.x - 2.0, popup_rect.max.y - 4.0),
                );
                let thumb_h = (track.height() * visible as f32 / total as f32).max(12.0);
                let top = track.min.y + (track.height() - thumb_h) * start as f32 / (total - visible).max(1) as f32;
                painter.rect_filled(
                    Rect::from_min_size(Pos2::new(track.min.x, top), egui::vec2(2.0, thumb_h)),
                    1.0,
                    theme.text.linear_multiply(0.25),
                );
            }
            if popup_rect.contains(hover_pos.unwrap_or(Pos2::ZERO)) {
                let delta = ui.input(|i| i.raw_scroll_delta.y);
                if delta != 0.0 {
                    let _ = self.client.input(if delta > 0.0 { "<C-p>" } else { "<C-n>" });
                }
            }
        }
        let mut panel_clicked = false;
        if self.lsp_panel.open {
            let click = response.clicked().then(|| response.interact_pointer_pos()).flatten();
            let small = crate::services::font_manager::editor_font_id(11.0);
            if let Some(index) = self.lsp_panel.paint(&painter, rect, nvim_row_height, &font, &small, theme, click) {
                if let super::lsp_panel::PanelAction::Run(keys) = self.lsp_panel.activate_row(index) {
                    let _ = self.client.input(&keys);
                }
            }
            panel_clicked = click.is_some_and(|p| self.lsp_panel.contains(rect, nvim_row_height, p));
            if response.hovered() {
                let delta = ui.input(|i| i.raw_scroll_delta.y);
                if delta != 0.0 {
                    self.lsp_panel.scroll(if delta > 0.0 { -1 } else { 1 });
                }
            }
        }
        if let Some(error) = &self.error {
            painter.text(origin, Align2::LEFT_TOP, error, font, theme.highlight);
        }

        // Mouse interactions
        let mut mouse_actions = Vec::new();
        if let Some(pos) = response.interact_pointer_pos().filter(|_| !self.lsp_panel.open && !panel_clicked) {
            let row = ((pos.y - origin.y) / nvim_row_height).floor().max(0.0) as usize;
            let column = ((pos.x - origin.x) / cell_width).floor().max(0.0) as usize;
            if response.dragged() {
                mouse_actions.push(EditorMouseEvent::Drag { row, column });
            } else if let Some(index) = popup_hit.filter(|_| response.clicked()).and_then(|(popup_rect, start)| {
                popup_rect.contains(pos).then(|| start + ((pos.y - popup_rect.min.y) / nvim_row_height) as usize)
            }) {
                self.accept_popup_item(index);
            } else if response.clicked() {
                mouse_actions.push(EditorMouseEvent::Press {
                    row,
                    column,
                    button: 0,
                });
                mouse_actions.push(EditorMouseEvent::Release {
                    row,
                    column,
                    button: 0,
                });
            }
        }
        if response.hovered() {
            let delta = ui.input(|i| i.raw_scroll_delta.y);
            if delta != 0.0 {
                let steps = (delta / nvim_row_height).round() as i32;
                let action = if steps > 0 { "up" } else { "down" };
                for _ in 0..steps.unsigned_abs().min(12) {
                    let _ = self.client.input_mouse("wheel", action, "", 0, 0, 0);
                }
            }
        }
        for event in mouse_actions {
            let _ = self.handle_mouse(event);
        }
    }

    fn tick(&mut self) {
        if let Err(error) = self.client.reload_config_if_changed() {
            self.error.get_or_insert_with(|| format!("Could not reload Neovim config: {error}"));
        }
        if let Err(error) = self.client.poll_exit() {
            self.error.get_or_insert(error);
        }
        for event in self.client.drain_events() {
            self.consume_notification(&event);
        }
    }

    fn mode(&self) -> EditorMode {
        EditorMode::Vim
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }

    fn text(&self) -> Option<String> {
        Some(self.lines.join("\n"))
    }

    fn save(&mut self) -> EditorResult<()> {
        Ok(())
    }

    fn shutdown(&mut self) {
        self.client.shutdown();
    }
}

/// Collapses a possibly multi-line message into one short status line.
fn compact_message(text: &str) -> String {
    const MAX: usize = 120;
    let line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if line.chars().count() <= MAX {
        return line;
    }
    let mut out: String = line.chars().take(MAX).collect();
    out.push('…');
    out
}

fn cursor_goto_is_stale(
    mode: &str,
    predicted: Option<crate::editor::types::CursorPosition>,
    target_row: usize,
    target_column: usize,
) -> bool {
    mode.starts_with('i')
        && predicted.is_some_and(|position| {
            position.row == target_row && position.column != target_column
        })
}

#[cfg(test)]
mod cursor_sync_tests {
    use super::cursor_goto_is_stale;
    use crate::editor::types::CursorPosition;

    #[test]
    fn stale_cursor_after_backspace_is_rejected() {
        let predicted = CursorPosition { row: 0, column: 1 };
        assert!(cursor_goto_is_stale("insert", Some(predicted), 0, 2));
    }

    #[test]
    fn stale_cursor_before_insert_position_is_rejected() {
        let predicted = CursorPosition { row: 0, column: 1 };
        assert!(cursor_goto_is_stale("insert", Some(predicted), 0, 0));
    }

    #[test]
    fn matching_cursor_and_non_insert_navigation_are_accepted() {
        let predicted = CursorPosition { row: 0, column: 1 };
        assert!(!cursor_goto_is_stale("insert", Some(predicted), 0, 1));
        assert!(!cursor_goto_is_stale("normal", Some(predicted), 0, 2));
        assert!(!cursor_goto_is_stale("insert", Some(predicted), 1, 2));
    }
}
