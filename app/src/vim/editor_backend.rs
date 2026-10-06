//! MindForge EditorBackend Trait Implementation for Neovim.
//!
//! # Purpose
//! Adapts MindForge's generalized `EditorBackend` contract to the embedded
//! Neovim process and message stream.
//!
//! # Architecture & Responsibilities
//! - Translates egui `EditorKeyEvent` (Ctrl, Alt, Shift modifiers, navigation keys) into Neovim key notation (`<C-...>`, `<A-...>`, `<S-...>`).
//! - Dispatches mouse click, drag, and release events as Neovim mouse inputs.
//! - Integrates LSP command palettes when active.
//! - Drives event processing (`tick`) by draining RPC notification channels.
//! - Delegates grid surface painting to `render.rs`.
//!
//! # Invariants & Non-Goals
//! - Must NOT manage persistent file system or SQLite state directly.
//! - Must NOT duplicate editor state outside Neovim's authoritative process.

use super::backend::VimBackend;
use crate::editor::{
    backend::{EditorBackend, EditorResult},
    events::{EditorKeyEvent, EditorMouseEvent},
};
use eframe::egui::{self, Rect, Ui};

fn mouse_button(button: u8) -> &'static str {
    match button {
        1 => "middle",
        2 => "right",
        _ => "left",
    }
}

impl EditorBackend for VimBackend {
    fn handle_key(&mut self, event: EditorKeyEvent) -> EditorResult<()> {
        if self.lsp_panel.open {
            return match self.lsp_panel.handle_key(event.key, event.modifiers) {
                super::lsp_panel::PanelAction::Run(keys) => self.client.input(&keys),
                super::lsp_panel::PanelAction::None => Ok(()),
            };
        }
        if event.key == egui::Key::Escape && self.is_insert_mode() {
            self.grid.popup_items.clear();
            self.grid.popup_info.clear();
            self.grid.popup_selected = None;
            self.grid.popup_anchor = None;
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
            egui::Key::OpenBracket => "[".into(),
            egui::Key::CloseBracket => "]".into(),
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
        self.render_in_rect_impl(
            ui,
            rect,
            font_size,
            cell_width,
            row_height,
            theme,
            caret,
            dt,
            typed,
            show_line_numbers,
        );
    }

    fn tick(&mut self) {
        if let Err(error) = self.client.reload_config_if_changed() {
            self.error
                .get_or_insert_with(|| format!("Could not reload Neovim config: {error}"));
        }
        if let Err(error) = self.client.poll_exit() {
            self.error.get_or_insert(error);
        }
        for event in self.client.drain_events() {
            self.consume_notification(&event);
        }
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
