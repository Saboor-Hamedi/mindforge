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

use crate::editor::Editor;
use crate::input::{handle_input, window_shortcuts};
use crate::mode::Mode;
use eframe::egui::{Pos2, Rect};

use eframe::egui::{self, Color32, FontId};
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
    pub editor: crate::state::editor::EditorState,
    pub command_bar: crate::state::command_bar::CommandBarState,
    pub misc: crate::state::misc::MiscState,

    // Sleek floating sidebar (Ctrl+B)
    pub sidebar: crate::state::sidebar::SidebarState,

    // All modal dialogs (settings, search, rename, delete, help)
    pub modal: crate::state::modal::ModalState,

    // Active document & sidebar limits
    pub notes: crate::state::notes::NotesState,

    // Multi-note & Documentation Tabs
    pub open_notes: Vec<OpenNote>,
    pub tabs: crate::state::tabs::TabsState,

    // Daily Activity & Writing Story tracking
    pub activity: crate::state::activity::ActivityState,

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

    // External services and background engines
    pub services: crate::state::services::ServicesState,

    // Active editing mode (Hybrid vs Vim)
    pub showcmd: crate::ui::showcmd::ShowCmdState,
    pub active_doc_idx: usize,
    pub doc_sidebar_focused: bool,
    pub doc_selected_idx: usize,

    // Webscan (:scan & :scans) state
    pub scan: crate::state::scan::ScanState,


    // Embedded Terminal (:term) docked session state
    pub terminal: crate::state::terminal::TerminalState,



    // Right side pane (Preview and AI Agent tabs)
    pub right_pane: crate::state::right_pane::RightPaneState,
}

impl App {
    /// Computes the exact monospace character advance width and line height.
    /// Measuring 100 characters eliminates single-glyph bounding box ink discrepancies,
    /// guaranteeing that caret placement at `col * cw` aligns with rendered text at any line length.
    pub fn cell_size(&mut self, ctx: &egui::Context) -> (f32, f32) {
        *self.editor.cell.get_or_insert_with(|| {
            let font = FontId::monospace(self.misc.font_size);
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
        if self.misc.first_frame {
            self.misc.first_frame = false;
            crate::services::blur::apply_window_blur(self.misc.blur_effect);
            if let Some(cmd) = egui::ViewportCommand::center_on_screen(ctx) {
                ctx.send_viewport_cmd(cmd);
            }
        }

        let now = ctx.input(|i| i.time);
        let dt = ctx.input(|i| i.unstable_dt).clamp(0.0, 0.05);
        let mode_at_frame_start = self.services.editor_controller.mode;
        crate::vim::runtime::update(self, ctx, now, mode_at_frame_start);

        // Precompute visual lines so keyboard navigation (ArrowUp, ArrowDown, PageUp, PageDown) uses accurate visual layout
        let (cw, _) = self.cell_size(ctx);
        let screen_w = ctx.screen_rect().width();
        let left_margin = if self.sidebar.open {
            crate::layout::GAP + self.sidebar.width + crate::layout::SPLITTER_BAR_W + crate::layout::GAP
        } else {
            crate::layout::GAP
        };
        let editor_w = (screen_w - left_margin - crate::layout::GAP).max(100.0);
        let is_preview_active = self.editor.preview_open && self.misc.mode == Mode::Normal;
        let effective_editor_w = if is_preview_active {
            let divider_w = 12.0;
            let available_w = (editor_w - divider_w).max(200.0);
            let min_w = 120.0f32;
            let max_w = (available_w - 120.0f32).max(min_w);
            (available_w * self.editor.split_ratio).clamp(min_w, max_w)
        } else {
            editor_w
        };
        let (ed_font_size, _, _) = self.misc.zoom.editor_metrics(self.misc.font_size, ctx);
        let active_ed = if self.misc.mode == Mode::Doc { &self.editor.doc_ed } else { &self.editor.ed };
        let in_vim = self.services.editor_controller.mode == EditorInputMode::Vim && matches!(self.misc.mode, Mode::Normal | Mode::Doc);
        self.editor.visual_lines = if in_vim {
            vec![crate::types::VisualLine { char_start: 0, char_end: active_ed.buf.len() }]
        } else if self.editor.inline_mode {
            let gutter_w = if self.editor.show_line_numbers {
                let total_lines = (active_ed.buf.iter().filter(|&&c| c == '\n').count() + 1).max(1);
                let digits = total_lines.to_string().len().max(2);
                (digits as f32 * (ed_font_size * 0.55) + 14.0).max(28.0)
            } else {
                0.0
            };
            let pad_x = if self.editor.show_line_numbers { 16.0 } else { 24.0 };
            let safe_w = effective_editor_w;
            let effective_gutter_w = if safe_w > gutter_w + 40.0 { gutter_w } else { 0.0 };
            let wrap_w = (effective_editor_w - effective_gutter_w - pad_x - 24.0).max(120.0);
            let inline_layout = crate::view_editor::inline::compute_inline_layout_ctx(
                ctx,
                active_ed,
                wrap_w,
                ed_font_size,
                &self.misc.theme,
                0.0,
            );
            inline_layout.compute_visual_lines()
        } else {
            let gutter_space = if self.editor.show_line_numbers { 42.0 } else { 0.0 };
            let text_area_w = (effective_editor_w - gutter_space - 8.0).max(100.0);
            let max_cols = (text_area_w / cw).floor().max(15.0) as usize;
            active_ed.compute_visual_lines(max_cols)
        };

        if self.terminal.open || self.misc.mode == Mode::Terminal {
            if let Some(ref mut pane) = self.terminal.pane {
                if pane.pump() {
                    self.terminal.open = false;
                    self.terminal.focused = false;
                    if self.misc.mode == Mode::Terminal {
                        self.misc.mode = self.terminal.prev_mode_before_term;
                    }
                    self.set_status("Terminal session ended", now);
                }
            }
        }

        if self.editor.font_dirty {
            self.editor.font_dirty = false;
            crate::services::font_manager::apply_font(ctx, &self.misc.selected_font);
        }

        let typed = handle_input(self, ctx, now);

        // Activity tracking: if user interacted in the last 60s, count dt towards active editor time
        if (now - self.misc.last_char_time) < 60.0 {
            self.activity.pending_secs += dt;
        }
        if typed {
            self.activity.pending_keys += 1;
            // Track completed word if space or newline typed
            if let Some(&last_ch) = self.editor.ed.buf.get(self.editor.ed.cur.saturating_sub(1)) {
                if last_ch.is_whitespace() {
                    self.activity.pending_words += 1;
                }
            }
        }

        // Periodic auto-flush of activity stats to database every 10 seconds
        if (now - self.activity.last_flush_time) > 10.0 {
            self.flush_activity(now);
            self.save_caret_position();
        }

        // Auto-save when idle for 1.2s in Normal mode
        if self.editor.is_dirty && (now - self.misc.last_char_time) > 1.2 && self.misc.mode == Mode::Normal {
            self.quick_save_active_note(now);
            self.save_caret_position();
        }

        // Save session state immediately if window close is requested
        if ctx.input(|i| i.viewport().close_requested()) {
            self.sync_save_session();
        }

        // Check Webscan background worker channel
        if let Some(ref rx) = self.scan.scan_rx {
            if let Ok(msg) = rx.try_recv() {
                let target_url = self.scan.scan_in_progress.take().unwrap_or_default();
                self.scan.scan_rx = None;
                match msg {
                    Ok(scan_res) => {
                        // Persist scan result to SQLite
                        if let Some(ref db) = self.services.db {
                            if let Ok(json_str) = serde_json::to_string(&scan_res.findings) {
                                let _ = db.save_scan(&scan_res.url, scan_res.note.as_deref(), &json_str);
                            }
                        }
                        self.scan.active_scan_result = Some(scan_res);
                        self.scan.active_scan_error = None;
                        self.scan.scan_report_scroll_y = 0.0;
                        self.misc.mode = Mode::ScanReport;
                        self.set_status("Scan completed successfully", now);
                    }
                    Err(err_msg) => {
                        self.scan.active_scan_result = None;
                        self.scan.active_scan_error = Some((target_url, err_msg));
                        self.scan.scan_report_scroll_y = 0.0;
                        self.misc.mode = Mode::ScanReport;
                        self.set_status("Scan failed (see report for details)", now);
                    }
                }
            }
        }

        // Detect dragged & dropped Obsidian vaults, folders, and markdown files
        if crate::workspace_import::handle_drag_and_drop(ctx, &mut self.services.workspace_importer) {
            self.set_status("Started vault import in background...", now);
        }

        // Poll whether background workspace import completed
        if let Some(count) = self.services.workspace_importer.poll_completion() {
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
        self.services.editor_controller.last_observed_mode = mode_at_frame_start;

        // Repaint gating: animate caret at high rate; command/modal at ~60fps; idle at 100ms.
        // Do NOT call request_repaint() (unbounded) for command mode — it starves the Windows
        // message pump and causes "Not Responding" when `:` is typed.
        let focused = ctx.input(|i| i.focused);
        let caret_animating = self.misc.caret.is_animating(now);
        if focused && caret_animating {
            ctx.request_repaint_after(Duration::from_millis(8));
        } else if focused && typed {
            // Give queued Neovim input/redraws a quick follow-up frame without
            // running the egui loop continuously while Vim mode is idle.
            ctx.request_repaint_after(Duration::from_millis(8));
        } else if focused
            && (self.command_bar.in_command
                || !self.misc.showcmd.text.is_empty()
                || self.modal.search_open
                || self.modal.settings_open
                || self.modal.help_open
                || self.modal.rename_open
                || self.modal.delete_confirm_open
                || self.misc.accent_dropdown_open
                || self.services.workspace_importer.is_modal_open
                || self.services.workspace_importer.is_active())
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
