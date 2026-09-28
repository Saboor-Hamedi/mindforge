//! Application controller, state management, and egui frame loop.

pub mod init;
pub mod modals;
pub mod notes;
pub mod panes;
pub mod right_pane;
pub mod scan_view;
pub mod shell;
pub mod stats_view;
pub mod terminal_drawer;

#[allow(unused_imports)]
pub use init::default_backup_dir;

use crate::caret::Caret;
use crate::services::db_worker::DbMsg;
use crate::editor::{Editor, VisualLine};
use crate::services::fuzzy::SearchItem;
use crate::input::{handle_input, window_shortcuts};
use crate::hybrid::HybridEngine;
use crate::mode::Mode;
use crate::settings::SettingTab;
use crate::services::sound::SoundEngine;
use crate::ui::theme::Theme;
use eframe::egui::{Pos2, Rect};
use crate::services::updater::UpdateManager;

use core::{DailyActivity, Database, Note};
use eframe::egui::{self, Color32, FontId};
use std::sync::mpsc::Sender;
use std::time::Duration;

pub use crate::editor::types::EditorMode as EditorInputMode;

#[derive(Clone)]
pub struct OpenNote {
    pub id: i64,
    pub title: String,
    pub editor: Editor,
    pub scroll_y: f32,
    pub is_dirty: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RightPaneTab {
    Preview,
    AiAgent,
    Backlinks,
    Outline,
}

pub struct App {
    pub ed: Editor,
    pub doc_ed: Editor,
    pub cmd_ed: Editor,
    pub in_command: bool,
    pub cmd_prefix: char,
    pub cmd_selected_idx: usize,
    pub cmd_navigated: bool,
    pub command_history: Vec<String>,
    pub caret: Caret,
    pub theme: Theme,
    pub sound: SoundEngine,
    pub font_size: f32,
    pub selected_font: String,
    pub blur_effect: crate::services::blur::BlurEffect,
    pub show_welcome: bool,
    pub opacity: f32,
    pub last_char_time: f64,
    pub cell: Option<(f32, f32)>,
    pub visual_lines: Vec<VisualLine>,
    pub mode: Mode,
    pub status_msg: String,
    pub status_time: f64,

    pub first_frame: bool,

    // Sleek floating sidebar (Ctrl+B)
    pub sidebar_open: bool,
    pub sidebar_focused: bool,
    pub sidebar_selected_idx: usize,
    pub sidebar_width: f32,
    pub is_dragging_sidebar_splitter: bool,
    pub sidebar_needs_scroll: bool,

    // Two-column Preferences modal (Ctrl+,)
    pub settings_open: bool,
    pub settings_just_opened: bool,
    pub active_setting_tab: SettingTab,
    pub backup_dir: String,
    pub last_backup_status: Option<String>,
    pub keybind_capture: Option<crate::settings::keymap::KeybindCapture>,
    pub keymap: crate::settings::keymap::VimKeymap,

    // Fuzzy search modal (Ctrl+P)
    pub search_open: bool,
    pub search_query: String,
    pub search_results: Vec<SearchItem>,
    pub search_selected: usize,
    pub search_just_opened: bool,

    // Rename modal (Ctrl+R)
    pub rename_open: bool,
    pub rename_input: String,
    pub rename_just_opened: bool,

    // Delete confirmation modal (Ctrl+Shift+D or :d)
    pub delete_confirm_open: bool,
    pub delete_just_opened: bool,
    pub pending_delete_note_id: Option<i64>,

    // Guidance & Help Tab (:help or F1)
    pub help_open: bool,
    pub help_tab: usize,
    pub help_scroll_y: f32,
    pub help_tab_scroll_offset: f32,

    // Active document & sidebar limits
    pub active_note_id: Option<i64>,
    pub active_note_title: String,
    pub notes_list: Vec<Note>,
    pub sidebar_notes_limit: usize,
    pub total_notes_count: usize,

    // Multi-note & Documentation Tabs
    pub open_notes: Vec<OpenNote>,
    pub active_tab: usize,
    pub last_active_tab: usize,
    pub open_doc_tabs: Vec<usize>,
    pub active_doc_tab: usize,
    pub last_active_doc_tab: usize,
    pub tab_scroll_offset: f32,
    pub doc_tab_scroll_offset: f32,

    // Scrolling & auto-save
    pub scroll_y: f32,
    pub doc_scroll_y: f32,
    pub preview_scroll_y: f32,
    pub preview_open: bool,
    pub inline_mode: bool,
    pub split_ratio: f32,
    pub is_dragging_splitter: bool,
    pub show_line_numbers: bool,
    pub is_dirty: bool,
    pub font_dirty: bool,
    pub last_saved_time: f64,
    pub last_editor_rect: Option<Rect>,
    pub last_ed_origin: Option<Pos2>,
    pub last_ed_font_size: Option<f32>,

    // Database & worker channel
    pub db: Option<Database>,
    pub db_tx: Sender<DbMsg>,

    // Active editing mode (Hybrid vs Vim)
    pub editor_controller: crate::editor::controller::EditorController,
    pub vim_runtime: crate::vim::VimRuntime,
    pub hybrid: HybridEngine,
    pub showcmd: crate::ui::showcmd::ShowCmdState,

    // Daily Activity & Writing Story tracking
    pub pending_secs: f32,
    pub pending_keys: u32,
    pub pending_words: u32,
    pub pending_created: u32,
    pub pending_edited: u32,
    pub last_flush_time: f64,
    pub today_activity: DailyActivity,
    pub activity_history: Vec<DailyActivity>,
    pub lifetime_activity: (u64, u64, u64, usize),

    // In-app auto-updater
    pub updater: UpdateManager,
    pub active_doc_idx: usize,
    pub doc_sidebar_focused: bool,
    pub doc_selected_idx: usize,

    // Webscan (:scan & :scans) state
    pub prev_mode_before_scan: Mode,
    pub scan_in_progress: Option<String>,
    pub scan_rx: Option<std::sync::mpsc::Receiver<Result<webscan::ScanResult, String>>>,
    pub active_scan_result: Option<webscan::ScanResult>,
    pub active_scan_error: Option<(String, String)>,
    pub scan_report_scroll_y: f32,
    pub past_scans: Vec<core::ScanRecord>,
    pub scan_history_selected: usize,
    pub scan_history_scroll_y: f32,
    pub clipboard_text: Option<String>,
    pub accent_overrides: crate::accent::AccentOverrides,
    pub accent_dropdown_open: bool,

    // Embedded Terminal (:term) docked session state
    pub terminal_open: bool,
    pub terminal_split_ratio: f32,
    pub is_dragging_terminal_splitter: bool,
    pub terminal_focused: bool,
    pub term_pane: Option<crate::ui::terminal_pane::TerminalPane>,
    pub prev_mode_before_term: Mode,

    // Editor-only smooth zoom & HUD state
    pub zoom: crate::ui::zoom::ZoomState,

    // DeepSeek Pro AI Agent state
    pub agent_state: crate::agent::AgentState,

    // Right side pane (Preview and AI Agent tabs)
    pub right_pane_tab: RightPaneTab,
    pub ai_focus_requested: bool,

    // Zen Mode (Ctrl+.)
    pub zen_mode: bool,

    // Granular UI Surface Visibility (toggleable via commands & shortcuts)
    pub show_titlebar: bool,
    pub show_tabs: bool,

    // Workspace & Obsidian Vault Importer
    pub workspace_importer: crate::workspace_import::WorkspaceImporter,

    // LunaLine Statusline Configuration
    pub lunaline_config: crate::lunaline::LunaLineConfig,

    // Right Sidebar (Outline & Backlinks)
    pub right_sidebar_open: bool,
    pub right_sidebar_width: f32,
    pub right_sidebar_state: crate::rightsidebar::RightSidebarState,

    // Wikilink hover preview & autocomplete state
    pub hover_wikilink: crate::wikilink::hover_wikilink::HoverWikiLinkState,
    pub wikilink_autocomplete: crate::wikilink::wikilink_autocompletion::WikiLinkAutocompleteState,
}

impl App {
    /// Computes the exact monospace character advance width and line height.
    /// Measuring 100 characters eliminates single-glyph bounding box ink discrepancies,
    /// guaranteeing that caret placement at `col * cw` aligns with rendered text at any line length.
    pub fn cell_size(&mut self, ctx: &egui::Context) -> (f32, f32) {
        *self.cell.get_or_insert_with(|| {
            let font = FontId::monospace(self.font_size);
            let sample_100 = "MMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMM";
            let g100 = ctx.fonts(|f| f.layout_no_wrap(sample_100.to_owned(), font.clone(), Color32::WHITE));
            let g1 = ctx.fonts(|f| f.layout_no_wrap("M".to_owned(), font, Color32::WHITE));
            let cw = (g100.size().x - g1.size().x) / 99.0;
            let lh = (g1.size().y * 1.30).round();
            (cw, lh)
        })
    }
}

impl eframe::App for App {
    fn clear_color(&self, _v: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, _f: &mut eframe::Frame) {
        if self.first_frame {
            self.first_frame = false;
            crate::services::blur::apply_window_blur(self.blur_effect);
            if let Some(cmd) = egui::ViewportCommand::center_on_screen(ctx) {
                ctx.send_viewport_cmd(cmd);
            }
        }

        let now = ctx.input(|i| i.time);
        let dt = ctx.input(|i| i.unstable_dt).clamp(0.0, 0.05);
        let mode_at_frame_start = self.editor_controller.mode;
        crate::vim::runtime::update(self, ctx, now, mode_at_frame_start);

        // Precompute visual lines so keyboard navigation (ArrowUp, ArrowDown, PageUp, PageDown) uses accurate visual layout
        let (cw, _) = self.cell_size(ctx);
        let screen_w = ctx.screen_rect().width();
        let left_margin = if self.mode == Mode::Doc || self.sidebar_open {
            crate::layout::GAP + self.sidebar_width + crate::layout::SPLITTER_BAR_W + crate::layout::GAP
        } else {
            crate::layout::GAP
        };
        let editor_w = (screen_w - left_margin - crate::layout::GAP).max(100.0);
        let is_preview_active = self.preview_open && self.mode == Mode::Normal;
        let effective_editor_w = if is_preview_active {
            let divider_w = 12.0;
            let available_w = (editor_w - divider_w).max(200.0);
            let min_w = 120.0f32;
            let max_w = (available_w - 120.0f32).max(min_w);
            (available_w * self.split_ratio).clamp(min_w, max_w)
        } else {
            editor_w
        };
        let (ed_font_size, _, _) = self.zoom.editor_metrics(self.font_size, ctx);
        let active_ed = if self.mode == Mode::Doc { &self.doc_ed } else { &self.ed };
        let in_vim = self.editor_controller.mode == EditorInputMode::Vim && self.mode == Mode::Normal;
        self.visual_lines = if in_vim {
            vec![crate::types::VisualLine { char_start: 0, char_end: self.ed.buf.len() }]
        } else if self.inline_mode {
            let gutter_w = if self.show_line_numbers {
                let total_lines = (active_ed.buf.iter().filter(|&&c| c == '\n').count() + 1).max(1);
                let digits = total_lines.to_string().len().max(2);
                (digits as f32 * (ed_font_size * 0.55) + 14.0).max(28.0)
            } else {
                0.0
            };
            let pad_x = if self.show_line_numbers { 16.0 } else { 24.0 };
            let safe_w = effective_editor_w;
            let effective_gutter_w = if safe_w > gutter_w + 40.0 { gutter_w } else { 0.0 };
            let wrap_w = (effective_editor_w - effective_gutter_w - pad_x - 24.0).max(120.0);
            let inline_layout = crate::view_editor::inline::compute_inline_layout_ctx(
                ctx,
                active_ed,
                wrap_w,
                ed_font_size,
                &self.theme,
                0.0,
            );
            inline_layout.compute_visual_lines()
        } else {
            let gutter_space = if self.show_line_numbers { 42.0 } else { 0.0 };
            let text_area_w = (effective_editor_w - gutter_space - 8.0).max(100.0);
            let max_cols = (text_area_w / cw).floor().max(15.0) as usize;
            active_ed.compute_visual_lines(max_cols)
        };

        if self.terminal_open || self.mode == Mode::Terminal {
            if let Some(ref mut pane) = self.term_pane {
                if pane.pump() {
                    self.terminal_open = false;
                    self.terminal_focused = false;
                    if self.mode == Mode::Terminal {
                        self.mode = self.prev_mode_before_term;
                    }
                    self.set_status("Terminal session ended", now);
                }
            }
        }

        if self.font_dirty {
            self.font_dirty = false;
            crate::services::font_manager::apply_font(ctx, &self.selected_font);
        }

        let typed = handle_input(self, ctx, now);

        // Activity tracking: if user interacted in the last 60s, count dt towards active editor time
        if (now - self.last_char_time) < 60.0 {
            self.pending_secs += dt;
        }
        if typed {
            self.pending_keys += 1;
            // Track completed word if space or newline typed
            if let Some(&last_ch) = self.ed.buf.get(self.ed.cur.saturating_sub(1)) {
                if last_ch.is_whitespace() {
                    self.pending_words += 1;
                }
            }
        }

        // Periodic auto-flush of activity stats to database every 10 seconds
        if (now - self.last_flush_time) > 10.0 {
            self.flush_activity(now);
            self.save_caret_position();
        }

        // Auto-save when idle for 1.2s in Normal mode
        if self.is_dirty && (now - self.last_char_time) > 1.2 && self.mode == Mode::Normal {
            self.quick_save_active_note(now);
            self.save_caret_position();
        }

        // Save session state immediately if window close is requested
        if ctx.input(|i| i.viewport().close_requested()) {
            self.sync_save_session();
        }

        // Check Webscan background worker channel
        if let Some(ref rx) = self.scan_rx {
            if let Ok(msg) = rx.try_recv() {
                let target_url = self.scan_in_progress.take().unwrap_or_default();
                self.scan_rx = None;
                match msg {
                    Ok(scan_res) => {
                        // Persist scan result to SQLite
                        if let Some(ref db) = self.db {
                            if let Ok(json_str) = serde_json::to_string(&scan_res.findings) {
                                let _ = db.save_scan(&scan_res.url, scan_res.note.as_deref(), &json_str);
                            }
                        }
                        self.active_scan_result = Some(scan_res);
                        self.active_scan_error = None;
                        self.scan_report_scroll_y = 0.0;
                        self.mode = Mode::ScanReport;
                        self.set_status("Scan completed successfully", now);
                    }
                    Err(err_msg) => {
                        self.active_scan_result = None;
                        self.active_scan_error = Some((target_url, err_msg));
                        self.scan_report_scroll_y = 0.0;
                        self.mode = Mode::ScanReport;
                        self.set_status("Scan failed (see report for details)", now);
                    }
                }
            }
        }

        // Detect dragged & dropped Obsidian vaults, folders, and markdown files
        if crate::workspace_import::handle_drag_and_drop(ctx, &mut self.workspace_importer) {
            self.set_status("Started vault import in background...", now);
        }

        // Poll whether background workspace import completed
        if let Some(count) = self.workspace_importer.poll_completion() {
            self.reload_db_state();
            self.set_status(&format!("Successfully imported {} notes into MindForge", count), now);
        }

        window_shortcuts(ctx);

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                self.draw(ui, dt, now, typed);
            });
        // Defer recording transitions performed during this frame until the
        // next update, where the newly selected backend can reconcile state.
        self.editor_controller.last_observed_mode = mode_at_frame_start;

        // Repaint gating: animate caret at high rate; command/modal at ~60fps; idle at 100ms.
        // Do NOT call request_repaint() (unbounded) for command mode — it starves the Windows
        // message pump and causes "Not Responding" when `:` is typed.
        let focused = ctx.input(|i| i.focused);
        let caret_animating = self.caret.is_animating(now);
        if focused && caret_animating {
            ctx.request_repaint_after(Duration::from_millis(8));
        } else if focused && typed {
            // Give queued Neovim input/redraws a quick follow-up frame without
            // running the egui loop continuously while Vim mode is idle.
            ctx.request_repaint_after(Duration::from_millis(8));
        } else if focused
            && (self.in_command
                || !self.showcmd.text.is_empty()
                || self.search_open
                || self.settings_open
                || self.help_open
                || self.rename_open
                || self.delete_confirm_open
                || self.accent_dropdown_open
                || self.workspace_importer.is_modal_open
                || self.workspace_importer.is_active())
        {
            ctx.request_repaint_after(Duration::from_millis(16));
        } else {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.sync_save_session();
    }
}
