//! Bottom-docked embedded terminal drawer and resize splitter.

use super::App;
use eframe::egui::{self, pos2, vec2, Color32, Painter, Rect, Stroke, Ui};

impl App {
    /// Renders the bottom-docked terminal drawer, handling interactive splitter resizing.
    pub fn render_terminal_drawer(
        &mut self,
        ui: &mut Ui,
        painter: &Painter,
        editor_panel_rect: Rect,
        bottom_terminal_rect: Option<Rect>,
        term_splitter_rect_opt: Option<Rect>,
        now: f64,
    ) {
        if let (Some(term_rect), Some(divider_rect)) = (bottom_terminal_rect, term_splitter_rect_opt) {
            let divider_h = 10.0;
            let available_h = (editor_panel_rect.height() - divider_h).max(140.0);
            let mid_y = divider_rect.center().y;
            let panel_left = divider_rect.min.x;
            let panel_right = divider_rect.max.x;
            let knob_mid = pos2((panel_left + panel_right) * 0.5, mid_y);
            let is_dragging = self.terminal.dragging_splitter;
            let knob_w = 36.0;
            let knob_h = if is_dragging { 6.0 } else { 4.0 };
            let knob_rect = Rect::from_center_size(knob_mid, vec2(knob_w, knob_h));
            let knob_hit_rect = Rect::from_center_size(knob_mid, vec2(44.0, 16.0));

            let is_knob_hovered = ui.rect_contains_pointer(knob_hit_rect);
            let primary_down = ui.input(|i| i.pointer.primary_down());
            let primary_pressed = ui.input(|i| i.pointer.primary_clicked() || i.pointer.button_pressed(egui::PointerButton::Primary));

            if is_knob_hovered && primary_pressed {
                self.terminal.dragging_splitter = true;
            }

            if self.terminal.dragging_splitter {
                if primary_down {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeRow);
                    if let Some(pos) = ui.input(|i| i.pointer.interact_pos().or_else(|| i.pointer.hover_pos())) {
                        let term_pixel_h = editor_panel_rect.max.y - pos.y;
                        let raw_ratio = term_pixel_h / available_h;
                        self.terminal.split_ratio = raw_ratio.clamp(0.12, 0.85);
                        ui.ctx().request_repaint();
                    }
                } else {
                    self.terminal.dragging_splitter = false;
                }
            } else if is_knob_hovered {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeRow);
            }

            let is_active = is_knob_hovered || self.terminal.dragging_splitter;

            // Render tactile knob only — no harsh full-width line
            let active_knob_rect = if is_active {
                Rect::from_center_size(knob_mid, vec2(38.0, 6.0))
            } else {
                knob_rect
            };
            painter.rect_filled(
                active_knob_rect,
                2.5,
                if is_active {
                    self.misc.theme.accent
                } else {
                    Color32::from_rgba_unmultiplied(self.misc.theme.muted.r(), self.misc.theme.muted.g(), self.misc.theme.muted.b(), 100)
                },
            );

            let grip_color = self.misc.theme.bg;
            for dx in [-5.0, 0.0, 5.0] {
                painter.line_segment(
                    [pos2(knob_mid.x + dx, knob_mid.y - 1.2), pos2(knob_mid.x + dx, knob_mid.y + 1.2)],
                    Stroke::new(1.0_f32, grip_color),
                );
            }

            if self.terminal.pane.is_none() {
                self.terminal.pane = crate::ui::terminal_pane::TerminalPane::spawn(ui.ctx(), &self.misc.theme).ok();
            }
            if let Some(ref mut pane) = self.terminal.pane {
                let action = pane.ui(ui, term_rect, &self.misc.theme, self.misc.font_size, self.terminal.focused, self.misc.opacity);
                match action {
                    crate::ui::terminal_pane::TerminalAction::Close => {
                        self.terminal.open = false;
                        self.terminal.focused = false;
                        self.set_status("Terminal closed", now);
                        ui.ctx().request_repaint();
                    }
                    crate::ui::terminal_pane::TerminalAction::RequestFocus => {
                        self.terminal.focused = true;
                    }
                    crate::ui::terminal_pane::TerminalAction::None => {}
                }
            }
        } else {
            self.terminal.dragging_splitter = false;
        }
    }
}
