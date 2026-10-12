//! Terminal UI rendering: the embedded pane surface and the docked drawer splitter.
//!
//! The terminal is rendered directly through egui (never through a terminal
//! emulator widget besides `egui_term`'s own `TerminalView`), with a
//! theme-aware color palette rebuilt only when the application theme changes.

use crate::ui::theme::Theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Painter, Rect, Stroke, Ui};
use egui_term::{ColorPalette, TerminalTheme, TerminalView};

use super::state::TerminalState;
use super::terminal::{TerminalAction, TerminalPane};

fn to_hex(c: Color32) -> String {
    format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b())
}

pub(crate) fn build_terminal_palette(theme: &Theme) -> ColorPalette {
    let mut palette = ColorPalette::default();
    let bg_hex = to_hex(theme.bg);
    let fg_hex = to_hex(theme.text);
    let accent_hex = to_hex(theme.accent);
    let muted_hex = to_hex(theme.muted);
    let highlight_hex = to_hex(theme.highlight);

    palette.background = bg_hex.clone();
    palette.foreground = fg_hex.clone();
    palette.black = bg_hex.clone();
    palette.dim_black = bg_hex;
    palette.dim_foreground = muted_hex;
    palette.bright_white = highlight_hex.clone();
    palette.white = fg_hex;
    palette.bright_cyan = accent_hex.clone();
    palette.cyan = accent_hex.clone();
    palette.bright_blue = accent_hex.clone();
    palette.blue = accent_hex;
    palette.bright_foreground = Some(highlight_hex);

    palette
}

impl TerminalPane {
    /// Renders the docked terminal pane with:
    /// 1. Cyber-styled header showing status pill, active session badge, and close button.
    /// 2. Active `TerminalView` on the left.
    /// 3. Dedicated sessions sidebar on the right with a `+` button to open new concurrent sessions.
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        theme: &Theme,
        font_size: f32,
        is_focused: bool,
        opacity: f32,
    ) -> TerminalAction {
        let mut action = TerminalAction::None;
        let painter = ui.painter().with_clip_rect(rect);

        // Detect mouse click inside dock to request focus
        if ui.rect_contains_pointer(rect) && ui.input(|i| i.pointer.primary_clicked()) {
            action = TerminalAction::RequestFocus;
        }

        // Sessions right sidebar layout with resizable width (collapsed when 1 or fewer sessions)
        let show_sessions_sidebar = self.sessions.len() > 1;
        let min_sidebar_w = 110.0f32;
        let max_sidebar_w = (rect.width() * 0.50).max(min_sidebar_w);
        let sidebar_w = self.sessions_sidebar_w.clamp(min_sidebar_w, max_sidebar_w);
        let splitter_w = 6.0f32;

        let (term_rect, splitter_rect_opt, sessions_rect_opt) = if show_sessions_sidebar {
            let term_rect = Rect::from_min_max(
                rect.min,
                pos2(rect.max.x - sidebar_w - splitter_w, rect.max.y),
            );
            let splitter_rect = Rect::from_min_max(
                pos2(rect.max.x - sidebar_w - splitter_w, rect.min.y),
                pos2(rect.max.x - sidebar_w, rect.max.y),
            );
            let sessions_rect =
                Rect::from_min_max(pos2(rect.max.x - sidebar_w, rect.min.y), rect.max);
            (term_rect, Some(splitter_rect), Some(sessions_rect))
        } else {
            (rect, None, None)
        };

        let header_h = 28.0;

        // --- 1. LEFT TERMINAL HEADER & BODY ---
        let term_header_rect = Rect::from_min_max(
            term_rect.min,
            pos2(term_rect.max.x, term_rect.min.y + header_h),
        );
        let term_body_rect = Rect::from_min_max(
            pos2(term_rect.min.x, term_rect.min.y + header_h),
            term_rect.max,
        );

        // Term header background & bottom divider line
        let term_h_bg = Color32::from_rgba_unmultiplied(
            theme.surface().r(),
            theme.surface().g(),
            theme.surface().b(),
            (opacity * 255.0) as u8,
        );
        painter.rect_filled(term_header_rect, 0.0, term_h_bg);

        // Title on left: Cyberpunk/Hacker aesthetic
        let title_color = if is_focused {
            theme.accent
        } else {
            theme.muted
        };
        painter.text(
            pos2(term_header_rect.min.x + 12.0, term_header_rect.center().y),
            Align2::LEFT_CENTER,
            "TERMINAL",
            FontId::monospace(font_size * 0.75),
            title_color,
        );

        // Current active session badge
        if let Some(active) = self.sessions.get(self.active_session_idx) {
            let tag_text = format!("[PTY-0{}: {}]", self.active_session_idx + 1, active.title);
            painter.text(
                pos2(term_header_rect.min.x + 115.0, term_header_rect.center().y),
                Align2::LEFT_CENTER,
                tag_text,
                FontId::monospace(font_size * 0.70),
                theme.muted,
            );
        }

        // Focus status pill
        let status_text = if is_focused {
            "● ACTIVE (Ctrl+J toggle • Esc editor)"
        } else {
            "○ READY (Click to focus • Ctrl+J)"
        };
        painter.text(
            pos2(term_header_rect.max.x - 62.0, term_header_rect.center().y),
            Align2::RIGHT_CENTER,
            status_text,
            FontId::proportional(font_size * 0.70),
            if is_focused {
                theme.highlight
            } else {
                theme.muted
            },
        );

        // New session (+) button in header (easy access when collapsed)
        let add_btn_center = pos2(term_header_rect.max.x - 40.0, term_header_rect.center().y);
        let add_btn_rect = Rect::from_center_size(add_btn_center, vec2(18.0, 18.0));
        let is_add_hover = ui.rect_contains_pointer(add_btn_rect);
        if is_add_hover {
            painter.rect_filled(
                add_btn_rect,
                3.0,
                theme.surface().lerp_to_gamma(theme.accent, 0.15),
            );
            if ui.input(|i| i.pointer.primary_clicked()) {
                self.new_session(ui.ctx());
                action = TerminalAction::RequestFocus;
            }
        }
        painter.text(
            add_btn_center,
            Align2::CENTER_CENTER,
            "+",
            FontId::proportional(14.0),
            if is_add_hover {
                theme.accent
            } else {
                theme.muted
            },
        );

        // Unified sleek close button on far right of terminal header
        let btn_center = pos2(term_header_rect.max.x - 16.0, term_header_rect.center().y);
        if crate::ui_components::render_close_button(
            ui,
            &painter,
            btn_center,
            20.0,
            theme,
            "term_header_close",
        ) {
            action = TerminalAction::Close;
        }

        // Ensure theme matches current Mindforge theme
        if self.last_theme_kind != Some(theme.kind) || self.cached_theme.is_none() {
            let palette = build_terminal_palette(theme);
            self.cached_theme = Some(TerminalTheme::new(Box::new(palette)));
            self.last_theme_kind = Some(theme.kind);
        }

        // Render active session embedded terminal widget with breathing room
        if let Some(ref term_theme) = self.cached_theme {
            if let Some(active_session) = self.sessions.get_mut(self.active_session_idx) {
                // egui_term only performs mouse selection while the widget holds
                // focus AND the pointer is over it. Keyboard input, however, is
                // routed through `feed_event` so it works even when the pointer is
                // elsewhere. Granting egui_term focus permanently would make both
                // paths write to the PTY (duplicate characters), so we grant focus
                // only for the duration of a mouse drag — enough to select text.
                let selecting = is_focused
                    && ui.input(|i| i.pointer.primary_down() || i.pointer.primary_released());
                // Uniform padding inside terminal body: 8px horizontal, 6px vertical
                let inner_rect = term_body_rect.shrink2(vec2(8.0, 6.0));
                let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(inner_rect));
                let term_view = TerminalView::new(&mut child_ui, &mut active_session.backend)
                    .set_size(inner_rect.size())
                    .set_focus(selecting)
                    .set_theme(term_theme.clone());
                child_ui.add(term_view);
            }
        }

        if let (Some(splitter_rect), Some(sessions_rect)) = (splitter_rect_opt, sessions_rect_opt) {
            // --- 2. SESSIONS RESIZE SPLITTER (Between Terminal & Sessions Sidebar) ---
            let mid_x = splitter_rect.center().x;
            let knob_mid = pos2(mid_x, splitter_rect.center().y);
            let knob_h = 32.0;
            let is_dragging = self.is_dragging_sessions_splitter;
            let knob_w = if is_dragging { 6.0 } else { 4.0 };
            let knob_rect = Rect::from_center_size(knob_mid, vec2(knob_w, knob_h));
            let knob_hit_rect = Rect::from_center_size(knob_mid, vec2(16.0, 40.0));

            let is_splitter_hovered = ui.rect_contains_pointer(knob_hit_rect);
            let primary_down = ui.input(|i| i.pointer.primary_down());
            let primary_pressed = ui.input(|i| {
                i.pointer.primary_clicked()
                    || i.pointer.button_pressed(egui::PointerButton::Primary)
            });

            if is_splitter_hovered && primary_pressed {
                self.is_dragging_sessions_splitter = true;
            }

            if self.is_dragging_sessions_splitter {
                if primary_down {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeColumn);
                    if let Some(pos) =
                        ui.input(|i| i.pointer.interact_pos().or_else(|| i.pointer.hover_pos()))
                    {
                        let desired_w = rect.max.x - pos.x;
                        self.sessions_sidebar_w = desired_w.clamp(min_sidebar_w, max_sidebar_w);
                        ui.ctx().request_repaint();
                    }
                } else {
                    self.is_dragging_sessions_splitter = false;
                }
            } else if is_splitter_hovered {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeColumn);
            }

            let is_splitter_active = is_splitter_hovered || self.is_dragging_sessions_splitter;

            // Tactile knob on the sessions splitter — no harsh full-height line
            let active_knob_rect = if is_splitter_active {
                Rect::from_center_size(knob_mid, vec2(6.0, 34.0))
            } else {
                knob_rect
            };
            painter.rect_filled(
                active_knob_rect,
                2.0,
                if is_splitter_active {
                    theme.accent
                } else {
                    Color32::from_rgba_unmultiplied(
                        theme.muted.r(),
                        theme.muted.g(),
                        theme.muted.b(),
                        110,
                    )
                },
            );
            for dy in [-5.0, 0.0, 5.0] {
                painter.line_segment(
                    [
                        pos2(knob_mid.x - 1.0, knob_mid.y + dy),
                        pos2(knob_mid.x + 1.0, knob_mid.y + dy),
                    ],
                    Stroke::new(1.0_f32, theme.bg),
                );
            }

            // --- 3. RIGHT SIDEBAR: SESSIONS LIST ---
            let sessions_bg = Color32::from_rgba_unmultiplied(
                theme.surface().r(),
                theme.surface().g(),
                theme.surface().b(),
                (opacity * 255.0) as u8,
            );
            painter.rect_filled(sessions_rect, 0.0, sessions_bg);

            // Sidebar Header
            let sessions_header_rect = Rect::from_min_max(
                sessions_rect.min,
                pos2(sessions_rect.max.x, sessions_rect.min.y + header_h),
            );
            painter.rect_filled(sessions_header_rect, 0.0, sessions_bg);
            painter.line_segment(
                [
                    pos2(sessions_header_rect.min.x, sessions_header_rect.max.y),
                    pos2(sessions_header_rect.max.x, sessions_header_rect.max.y),
                ],
                Stroke::new(1.0_f32, theme.border()),
            );

            // Sessions Header Label
            let count_str = format!("SESSIONS ({})", self.sessions.len());
            painter.text(
                pos2(
                    sessions_header_rect.min.x + 10.0,
                    sessions_header_rect.center().y,
                ),
                Align2::LEFT_CENTER,
                count_str,
                FontId::monospace(font_size * 0.68),
                theme.muted,
            );

            // [+] Sleek New Session Button (matching sidebar [+] button: transparent inactive, borderless soft hover)
            let btn_center_x = sessions_rect.max.x - 18.0;
            let new_btn_rect = Rect::from_center_size(
                pos2(btn_center_x, sessions_header_rect.center().y),
                vec2(20.0, 20.0),
            );
            let is_new_btn_hovered = ui.rect_contains_pointer(new_btn_rect);
            if is_new_btn_hovered {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                painter.rect_filled(
                    new_btn_rect,
                    4.0,
                    theme.surface().lerp_to_gamma(theme.accent, 0.15),
                );
                if ui.input(|i| i.pointer.primary_clicked()) {
                    self.new_session(ui.ctx());
                    action = TerminalAction::RequestFocus;
                }
            }
            let plus_color = if is_new_btn_hovered {
                theme.accent
            } else {
                theme.muted
            };
            painter.text(
                new_btn_rect.center(),
                Align2::CENTER_CENTER,
                "+",
                FontId::proportional(14.0),
                plus_color,
            );

            // Scrollable Sessions List Items
            let list_rect = Rect::from_min_max(
                pos2(sessions_rect.min.x, sessions_header_rect.max.y),
                sessions_rect.max,
            );
            let list_painter = painter.with_clip_rect(list_rect);

            let row_height = 28.0f32;
            let row_gap = 4.0f32;
            let total_content_h = (self.sessions.len() as f32) * (row_height + row_gap) + 8.0;
            let visible_list_h = list_rect.height().max(1.0);
            let max_scroll = (total_content_h - visible_list_h).max(0.0);

            if ui.rect_contains_pointer(list_rect) {
                let scroll_delta = ui.input(|i| {
                    if i.smooth_scroll_delta.y.abs() > 0.001 {
                        i.smooth_scroll_delta.y
                    } else {
                        i.raw_scroll_delta.y * 0.5
                    }
                });
                if scroll_delta.abs() > 0.001 {
                    self.sessions_scroll_y =
                        (self.sessions_scroll_y - scroll_delta).clamp(0.0, max_scroll);
                }
            }
            self.sessions_scroll_y = self.sessions_scroll_y.clamp(0.0, max_scroll);

            let list_y_start = list_rect.min.y + 5.0 - self.sessions_scroll_y;
            let mut session_to_close: Option<usize> = None;
            let mut session_to_select: Option<usize> = None;

            for (idx, session) in self.sessions.iter().enumerate() {
                let row_y = list_y_start + (idx as f32) * (row_height + row_gap);
                if row_y + row_height < list_rect.min.y || row_y > list_rect.max.y {
                    continue;
                }

                let row_rect = Rect::from_min_max(
                    pos2(list_rect.min.x + 6.0, row_y),
                    pos2(list_rect.max.x - 6.0, row_y + row_height),
                );

                let is_active = idx == self.active_session_idx;
                let is_hovered = ui.rect_contains_pointer(row_rect);

                // Pill surface: match sidebar notes exactly (no stroke, transparent when inactive)
                if is_active || is_hovered {
                    let bg_color = if is_active {
                        Color32::from_rgba_unmultiplied(
                            theme.accent.r(),
                            theme.accent.g(),
                            theme.accent.b(),
                            24,
                        )
                    } else if theme.is_light() {
                        Color32::from_rgba_unmultiplied(0, 0, 0, 13)
                    } else {
                        Color32::from_rgba_unmultiplied(255, 255, 255, 14)
                    };
                    list_painter.rect_filled(row_rect, 6.0, bg_color);

                    // Left vertical accent bar on active session (matching sidebar notes)
                    if is_active {
                        let bar = Rect::from_min_size(
                            pos2(row_rect.min.x + 1.0, row_rect.min.y + 5.0),
                            vec2(3.0, row_rect.height() - 10.0),
                        );
                        list_painter.rect_filled(bar, 1.5, theme.accent);
                    }
                }

                // Session title (matching sidebar notes typography, color, and clipping)
                let title_x = row_rect.min.x + 12.0;
                let text_color = if is_active {
                    theme.accent
                } else if is_hovered {
                    theme.text
                } else {
                    theme.text.lerp_to_gamma(theme.muted, 0.35)
                };

                let title_display = format!("{}. {}", idx + 1, session.title);
                let text_right_limit = btn_center_x - 12.0;
                let row_clip = Rect::from_min_max(
                    pos2(row_rect.min.x, row_rect.min.y),
                    pos2(text_right_limit, row_rect.max.y),
                )
                .intersect(list_rect);
                let row_painter = list_painter.with_clip_rect(row_clip);

                row_painter.text(
                    pos2(title_x, row_rect.center().y),
                    Align2::LEFT_CENTER,
                    title_display,
                    FontId::proportional(12.5),
                    text_color,
                );

                // Close session button on the right of the row - vertically aligned with header [+] button
                let close_center = pos2(btn_center_x, row_rect.center().y);
                let is_close_hover = ui
                    .rect_contains_pointer(Rect::from_center_size(close_center, vec2(18.0, 18.0)));

                if (is_hovered || is_active)
                    && crate::ui_components::render_close_button(
                        ui,
                        &list_painter,
                        close_center,
                        18.0,
                        theme,
                        ("term_session_close", idx),
                    )
                {
                    session_to_close = Some(idx);
                }

                // Handle clicking the row to select session
                if is_hovered && !is_close_hover && ui.input(|i| i.pointer.primary_clicked()) {
                    session_to_select = Some(idx);
                }
            }

            // Mini vertical scrollbar indicator if overflowing
            if max_scroll > 0.0 && visible_list_h > 30.0 {
                let thumb_h = ((visible_list_h / total_content_h) * visible_list_h)
                    .clamp(18.0, visible_list_h);
                let scroll_ratio = (self.sessions_scroll_y / max_scroll).clamp(0.0, 1.0);
                let thumb_y = list_rect.min.y + scroll_ratio * (visible_list_h - thumb_h);
                let thumb_rect =
                    Rect::from_min_size(pos2(list_rect.max.x - 4.0, thumb_y), vec2(2.5, thumb_h));
                list_painter.rect_filled(
                    thumb_rect,
                    1.25,
                    Color32::from_rgba_unmultiplied(
                        theme.muted.r(),
                        theme.muted.g(),
                        theme.muted.b(),
                        100,
                    ),
                );
            }

            if let Some(idx) = session_to_select {
                self.active_session_idx = idx;
                action = TerminalAction::RequestFocus;
            }

            if let Some(idx) = session_to_close {
                if self.close_session(idx) {
                    action = TerminalAction::Close;
                }
            }
        }

        action
    }
}

/// Result of rendering the docked terminal drawer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalDrawerAction {
    /// Nothing happened; keep the terminal as-is.
    None,
    /// The terminal was closed by the user.
    Closed,
}

/// Renders the bottom-docked terminal drawer, handling interactive splitter resizing.
///
/// This is a free function (rather than an `App` method) so the whole terminal
/// subsystem stays inside this module. The caller applies any resulting status
/// message once the borrow of `terminal` has ended.
#[allow(clippy::too_many_arguments)]
pub fn render_terminal_drawer(
    terminal: &mut TerminalState,
    ui: &mut Ui,
    painter: &Painter,
    editor_panel_rect: Rect,
    bottom_terminal_rect: Option<Rect>,
    term_splitter_rect_opt: Option<Rect>,
    theme: &Theme,
    font_size: f32,
    opacity: f32,
    working_dir: Option<std::path::PathBuf>,
) -> TerminalDrawerAction {
    let mut action = TerminalDrawerAction::None;
    if let (Some(term_rect), Some(divider_rect)) = (bottom_terminal_rect, term_splitter_rect_opt) {
        let divider_h = 10.0;
        let available_h = (editor_panel_rect.height() - divider_h).max(140.0);
        let mid_y = divider_rect.center().y;
        let panel_left = divider_rect.min.x;
        let panel_right = divider_rect.max.x;
        let knob_mid = pos2((panel_left + panel_right) * 0.5, mid_y);
        let is_dragging = terminal.dragging_splitter;
        let knob_w = 36.0;
        let knob_h = if is_dragging { 6.0 } else { 4.0 };
        let knob_rect = Rect::from_center_size(knob_mid, vec2(knob_w, knob_h));
        let knob_hit_rect = Rect::from_center_size(knob_mid, vec2(44.0, 16.0));

        let is_knob_hovered = ui.rect_contains_pointer(knob_hit_rect);
        let primary_down = ui.input(|i| i.pointer.primary_down());
        let primary_pressed = ui.input(|i| {
            i.pointer.primary_clicked() || i.pointer.button_pressed(egui::PointerButton::Primary)
        });

        if is_knob_hovered && primary_pressed {
            terminal.dragging_splitter = true;
        }

        if terminal.dragging_splitter {
            if primary_down {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeRow);
                if let Some(pos) =
                    ui.input(|i| i.pointer.interact_pos().or_else(|| i.pointer.hover_pos()))
                {
                    let term_pixel_h = editor_panel_rect.max.y - pos.y;
                    let raw_ratio = term_pixel_h / available_h;
                    terminal.split_ratio = raw_ratio.clamp(0.12, 0.85);
                    ui.ctx().request_repaint();
                }
            } else {
                terminal.dragging_splitter = false;
            }
        } else if is_knob_hovered {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeRow);
        }

        let is_active = is_knob_hovered || terminal.dragging_splitter;

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
                theme.accent
            } else {
                Color32::from_rgba_unmultiplied(
                    theme.muted.r(),
                    theme.muted.g(),
                    theme.muted.b(),
                    100,
                )
            },
        );

        let grip_color = theme.bg;
        for dx in [-5.0, 0.0, 5.0] {
            painter.line_segment(
                [
                    pos2(knob_mid.x + dx, knob_mid.y - 1.2),
                    pos2(knob_mid.x + dx, knob_mid.y + 1.2),
                ],
                Stroke::new(1.0_f32, grip_color),
            );
        }

        // Respawn when absent OR when every session has exited (so Ctrl+J after
        // `exit` opens a fresh shell instead of rendering an empty pane).
        let needs_spawn = terminal
            .pane
            .as_ref()
            .is_none_or(|pane| pane.sessions.is_empty());
        if needs_spawn {
            terminal.pane = TerminalPane::spawn(ui.ctx(), theme, working_dir.clone()).ok();
        }
        if let Some(ref mut pane) = terminal.pane {
            let pane_action = pane.ui(
                ui,
                term_rect,
                theme,
                font_size,
                terminal.focused,
                opacity,
            );
            match pane_action {
                TerminalAction::Close => {
                    terminal.open = false;
                    terminal.focused = false;
                    action = TerminalDrawerAction::Closed;
                    ui.ctx().request_repaint();
                }
                TerminalAction::RequestFocus => {
                    terminal.focused = true;
                }
                TerminalAction::None => {}
            }
        }
    } else {
        terminal.dragging_splitter = false;
    }
    action
}
