use super::{
    client::NeovimClient,
    state::{GridCell, GridState},
};
use crate::editor::{
    backend::{EditorBackend, EditorResult},
    events::{EditorKeyEvent, EditorMouseEvent},
    types::EditorMode,
};
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
    line_numbers_enabled: Option<bool>,
    cached_theme: Option<(Color32, Color32, Color32, Color32)>,
}

impl VimBackend {
    pub fn start(
        text: &str,
        width: usize,
        height: usize,
        row: usize,
        column: usize,
        ctx: Option<egui::Context>,
    ) -> Result<Self, String> {
        let mut client = NeovimClient::start(ctx)?;

        // Attach the embedded grid before installing the app's active note so
        // the first buffer update is rendered directly into our editor pane.
        client.attach_ui(width, height)?;
        // Keep shared editor navigation options consistent even when the
        // user's init.lua has absolute numbers or cursorline disabled.
        // Enable true colors, markdown syntax highlighting engine, and smooth visual line navigation.
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
        "#;
        let _ = client.request(
            "nvim_exec_lua",
            vec![Value::from(init_lua), Value::Array(vec![])],
        );

        // Give the buffer a markdown filename so Neovim filetype and syntax rules attach
        let _ = client.request("nvim_buf_set_name", vec![Value::from(0), Value::from("note.md")]);

        // Load text async — no blocking round-trip needed here.
        // nvim_buf_set_lines is a notification (fire and forget).
        client.set_buffer_text_async(text)?;
        let _ = client.notify("nvim_command", vec![Value::from("setlocal filetype=markdown syntax=markdown")]);

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
        } else if items.get(1).and_then(Value::as_str) == Some("nvim_buf_lines_event") {
            self.apply_lines_event(items.get(2).unwrap_or(&Value::Nil));
        }
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
                // grid_line payload: [grid, row, col_start, cells, wrap?]
                "grid_line" if args.len() >= 4 => self.apply_grid_line(args),
                "grid_scroll" if args.len() >= 7 => self.apply_grid_scroll(args),
                // grid_cursor_goto payload: [grid, row, column]
                "grid_cursor_goto" if args.len() >= 3 && num(&args[0]) == 1 => {
                    let target_row = num(&args[1]);
                    let target_col = num(&args[2]);
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
                    self.grid.mode = args[0].as_str().unwrap_or("normal").to_owned()
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
                "popupmenu_hide" => {
                    self.grid.popup_items.clear();
                    self.grid.popup_selected = None;
                    self.grid.popup_anchor = None;
                }
                "msg_show" if args.len() >= 2 => self.grid.message = cmdline_text(&args[1]),
                "msg_clear" => self.grid.message.clear(),
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
        self.client.input(input)
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
        let _ = self.client.request("nvim_buf_set_name", vec![Value::from(0), Value::from("note.md")]);
        self.client.set_buffer_text_async(text)?;
        let _ = self.client.notify("nvim_command", vec![Value::from("setlocal filetype=markdown syntax=markdown")]);
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

fn mouse_button(button: u8) -> &'static str {
    match button {
        1 => "middle",
        2 => "right",
        _ => "left",
    }
}

impl EditorBackend for VimBackend {
    fn handle_key(&mut self, event: EditorKeyEvent) -> EditorResult<()> {
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
        if !self.grid.popup_items.is_empty() {
            let popup_width = (rect.width() * 0.45).clamp(180.0, 360.0).min(rect.width());
            let popup_height = (self.grid.popup_items.len() as f32 * nvim_row_height).min(rect.height());
            let (popup_row, popup_col) = self
                .grid
                .popup_anchor
                .unwrap_or((self.grid.cursor.row, self.grid.cursor.column));
            let popup_x = (origin.x + popup_col as f32 * cell_width)
                .clamp(origin.x, (rect.max.x - popup_width).max(origin.x));
            let popup_y = (origin.y + (popup_row + 1) as f32 * nvim_row_height)
                .min((rect.max.y - popup_height).max(origin.y));
            let popup_rect = Rect::from_min_size(
                Pos2::new(popup_x, popup_y),
                egui::vec2(popup_width, popup_height),
            );
            painter.rect_filled(popup_rect, 2.0, theme.surface());
            painter.rect_stroke(popup_rect, 2.0, Stroke::new(1.0, theme.border()), egui::StrokeKind::Outside);
            for (index, item) in self.grid.popup_items.iter().enumerate() {
                let y = popup_y + index as f32 * nvim_row_height;
                if Some(index) == self.grid.popup_selected {
                    painter.rect_filled(
                        Rect::from_min_size(
                            Pos2::new(popup_x, y),
                            egui::vec2(popup_width, nvim_row_height),
                        ),
                        0.0,
                        theme.accent.linear_multiply(0.25),
                    );
                }
                painter.text(
                    Pos2::new(popup_x + 4.0, y),
                    Align2::LEFT_TOP,
                    item,
                    font.clone(),
                    theme.text,
                );
            }
        }

        if let Some(error) = &self.error {
            painter.text(origin, Align2::LEFT_TOP, error, font, theme.highlight);
        }

        // Mouse interactions
        let mut mouse_actions = Vec::new();
        if let Some(pos) = response.interact_pointer_pos() {
            let row = ((pos.y - origin.y) / nvim_row_height).floor().max(0.0) as usize;
            let column = ((pos.x - origin.x) / cell_width).floor().max(0.0) as usize;
            if response.dragged() {
                mouse_actions.push(EditorMouseEvent::Drag { row, column });
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
