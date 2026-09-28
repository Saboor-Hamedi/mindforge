use super::{
    events::{EditorKeyEvent, EditorMouseEvent},
    types::EditorMode,
};
use eframe::egui::{Rect, Ui};
use crate::ui::theme::Theme;

pub type EditorResult<T> = Result<T, String>;

/// Application-facing contract for an editing engine. Backends receive events
/// and expose state; app-level document features remain owned by the app.
#[allow(dead_code)]
pub trait EditorBackend {
    fn handle_key(&mut self, event: EditorKeyEvent) -> EditorResult<()>;
    fn handle_text(&mut self, text: &str) -> EditorResult<()>;
    fn handle_mouse(&mut self, event: EditorMouseEvent) -> EditorResult<()>;
    fn render_in_rect(
        &mut self,
        ui: &mut Ui,
        rect: Rect,
        font_size: f32,
        cell_width: f32,
        row_height: f32,
        theme: &Theme,
        caret: &mut crate::caret::Caret,
        dt: f32,
        typed: bool,
        show_line_numbers: bool,
    );
    fn tick(&mut self);
    fn mode(&self) -> EditorMode;
    fn is_dirty(&self) -> bool;
    fn text(&self) -> Option<String>;
    fn save(&mut self) -> EditorResult<()>;
    fn shutdown(&mut self);
}
