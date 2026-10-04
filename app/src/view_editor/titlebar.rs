//! Document title bar, color customizer button, and borderless window drag gripper.

use crate::ui::theme::Theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Rect, Stroke};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TitlebarAction {
    ToggleAccentDropdown,
}

/// Renders the full-width modern titlebar across the entire window top.
/// Displays Codex icon, MindForge branding, active document title, drag gripper, and window controls.
pub fn render_full_titlebar(
    ui: &egui::Ui,
    painter: &egui::Painter,
    titlebar_rect: Rect,
    active_title: &str,
    is_dirty: bool,
    theme: &Theme,
    accent_dropdown_open: bool,
    opacity: f32,
) -> (Option<TitlebarAction>, Rect) {
    // 1. Sleek translucent card surface matching desktop backdrop blur
    let title_bg = Color32::from_rgba_unmultiplied(
        theme.surface().r(),
        theme.surface().g(),
        theme.surface().b(),
        (opacity * 255.0) as u8,
    );
    painter.rect(
        titlebar_rect,
        5.0,
        title_bg,
        Stroke::NONE,
        egui::StrokeKind::Inside,
    );

    let center_y = titlebar_rect.center().y;

    // 2. Modern vector Codex / Book icon
    crate::ui_components::render_vector_icon(
        painter,
        "book",
        Rect::from_center_size(pos2(titlebar_rect.min.x + 20.0, center_y), vec2(15.0, 15.0)),
        theme.accent,
    );

    // 3. MINDFORGE branding
    painter.text(
        pos2(titlebar_rect.min.x + 32.0, center_y),
        Align2::LEFT_CENTER,
        "MINDFORGE",
        FontId::monospace(12.5),
        theme.accent,
    );

    // 4. Subtle separator derived from theme
    painter.text(
        pos2(titlebar_rect.min.x + 122.0, center_y),
        Align2::LEFT_CENTER,
        "│",
        FontId::monospace(12.0),
        theme.border(),
    );

    // Total width consumed by the 5 right buttons (36px * 5 = 180px)
    let btn_w = 36.0;
    let right_controls_start = titlebar_rect.max.x - btn_w * 5.0;
    let gap_from_controls = 48.0; // Clean, generous breathing room between title and right icons
    let title_start_x = titlebar_rect.min.x + 136.0;

    // 5. Active Document / Section Title — fully styled using theme.text
    let max_avail_w = (right_controls_start - gap_from_controls - title_start_x).max(40.0);
    let max_chars = (max_avail_w / 7.5) as usize;
    let base_title = crate::ui::truncate_with_ellipsis(active_title, max_chars);
    let display_title = if is_dirty {
        format!("{}  ●", base_title)
    } else {
        base_title
    };
    painter.text(
        pos2(title_start_x, center_y),
        Align2::LEFT_CENTER,
        display_title,
        FontId::monospace(11.5),
        theme.text,
    );

    // 6. Right side: Accent button, Drag gripper, and Window Controls
    render_window_controls(ui, painter, titlebar_rect, theme, accent_dropdown_open, opacity)
}

/// Renders modern window controls (Accent Picker, Drag Button, Minimize, Maximize/Restore, Close).
/// All buttons match the exact height of the titlebar, are vertically centered, and use theme colors.
pub fn render_window_controls(
    ui: &egui::Ui,
    painter: &egui::Painter,
    bounds: Rect,
    theme: &Theme,
    accent_dropdown_open: bool,
    opacity: f32,
) -> (Option<TitlebarAction>, Rect) {
    let btn_w = 36.0;
    let _btn_h = bounds.height();
    let top_y = bounds.min.y;
    let bottom_y = bounds.max.y;
    let right_x = bounds.max.x;

    let mut action = None;

    // Match the same translucent bg used by render_full_titlebar so the
    // maximize restore-icon mask doesn't appear as an opaque ghost.
    let title_bg = Color32::from_rgba_unmultiplied(
        theme.surface().r(),
        theme.surface().g(),
        theme.surface().b(),
        (opacity * 255.0) as u8,
    );

    // 1. Accent Color Customizer Button (🎨) - directly beside the Drag button
    let accent_rect = Rect::from_min_max(
        pos2(right_x - btn_w * 5.0, top_y),
        pos2(right_x - btn_w * 4.0, bottom_y),
    );
    let is_accent_hovered = ui.rect_contains_pointer(accent_rect);
    let accent_bg = if accent_dropdown_open {
        Color32::from_rgba_unmultiplied(theme.accent.r(), theme.accent.g(), theme.accent.b(), 50)
    } else if is_accent_hovered {
        Color32::from_rgba_unmultiplied(255, 255, 255, 20)
    } else {
        Color32::TRANSPARENT
    };
    if accent_bg != Color32::TRANSPARENT {
        painter.rect_filled(accent_rect, 0.0, accent_bg);
    }
    if is_accent_hovered && ui.input(|i| i.pointer.primary_clicked()) {
        action = Some(TitlebarAction::ToggleAccentDropdown);
    }

    // Palette icon with active accent color dot
    let c = accent_rect.center();
    let palette_rect = Rect::from_center_size(c - vec2(1.0, 0.0), vec2(15.0, 15.0));
    crate::ui_components::render_vector_icon(
        painter,
        "palette",
        palette_rect,
        if is_accent_hovered || accent_dropdown_open { theme.accent } else { theme.text },
    );
    // Indicator dot showing active accent color
    painter.circle_filled(
        pos2(accent_rect.max.x - 7.0, accent_rect.max.y - 7.0),
        2.5,
        theme.accent,
    );

    // 2. Drag Button (Gripper) - spans full titlebar height, centered on right side
    let drag_rect = Rect::from_min_max(
        pos2(right_x - btn_w * 4.0, top_y),
        pos2(right_x - btn_w * 3.0, bottom_y),
    );
    let is_drag_hovered = ui.rect_contains_pointer(drag_rect);
    if is_drag_hovered && ui.input(|i| i.pointer.primary_down()) {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }
    if is_drag_hovered {
        painter.rect_filled(drag_rect, 0.0, Color32::from_rgba_unmultiplied(255, 255, 255, 16));
    }
    let grip_color = if is_drag_hovered { theme.accent } else { theme.muted };
    let drag_c = drag_rect.center();
    for col in 0..2 {
        for row in 0..3 {
            let cx = drag_c.x - 3.5 + col as f32 * 7.0;
            let cy = drag_c.y - 6.0 + row as f32 * 6.0;
            painter.circle_filled(pos2(cx, cy), 1.3, grip_color);
        }
    }

    // 3. Minimize Button (—)
    let min_rect = Rect::from_min_max(
        pos2(right_x - btn_w * 3.0, top_y),
        pos2(right_x - btn_w * 2.0, bottom_y),
    );
    let min_hovered = ui.rect_contains_pointer(min_rect);
    if min_hovered {
        painter.rect_filled(min_rect, 0.0, Color32::from_rgba_unmultiplied(255, 255, 255, 22));
        if ui.input(|i| i.pointer.primary_clicked()) {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        }
    }
    let min_stroke_color = if min_hovered { theme.text } else { theme.muted };
    let min_c = min_rect.center();
    painter.line_segment(
        [
            pos2(min_c.x - 5.5, min_c.y + 1.0),
            pos2(min_c.x + 5.5, min_c.y + 1.0),
        ],
        Stroke::new(1.5_f32, min_stroke_color),
    );

    // 4. Maximize / Restore Button (□)
    let max_rect = Rect::from_min_max(
        pos2(right_x - btn_w * 2.0, top_y),
        pos2(right_x - btn_w * 1.0, bottom_y),
    );
    let max_hovered = ui.rect_contains_pointer(max_rect);
    let is_maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
    if max_hovered {
        painter.rect_filled(max_rect, 0.0, Color32::from_rgba_unmultiplied(255, 255, 255, 22));
        if ui.input(|i| i.pointer.primary_clicked()) {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
        }
    }
    let max_stroke_color = if max_hovered { theme.text } else { theme.muted };
    let max_c = max_rect.center();
    if is_maximized {
        let s = 4.2;
        // Background window (offset up-right)
        painter.rect_stroke(
            Rect::from_center_size(pos2(max_c.x + 2.0, max_c.y - 2.0), vec2(s * 2.0 - 1.0, s * 2.0 - 1.0)),
            1.0,
            Stroke::new(1.2_f32, max_stroke_color),
            egui::StrokeKind::Inside,
        );
        // Foreground window mask + stroke — use translucent title_bg so the
        // mask doesn't appear as an opaque ghost through the transparent titlebar.
        painter.rect_filled(
            Rect::from_center_size(pos2(max_c.x - 1.8, max_c.y + 1.8), vec2(s * 2.0, s * 2.0)),
            1.0,
            title_bg,
        );
        painter.rect_stroke(
            Rect::from_center_size(pos2(max_c.x - 1.8, max_c.y + 1.8), vec2(s * 2.0, s * 2.0)),
            1.0,
            Stroke::new(1.2_f32, max_stroke_color),
            egui::StrokeKind::Inside,
        );
    } else {
        let s = 4.8;
        painter.rect_stroke(
            Rect::from_center_size(max_c, vec2(s * 2.0, s * 2.0)),
            1.0,
            Stroke::new(1.3_f32, max_stroke_color),
            egui::StrokeKind::Inside,
        );
    }

    // 5. Close Button (✕) - outer right corner matches 5.0 titlebar radius
    let close_rect = Rect::from_min_max(
        pos2(right_x - btn_w, top_y),
        pos2(right_x, bottom_y),
    );
    let close_hovered = ui.rect_contains_pointer(close_rect);
    if close_hovered {
        painter.rect(
            close_rect,
            egui::CornerRadius { nw: 0, ne: 5, sw: 0, se: 5 },
            Color32::from_rgb(225, 45, 57),
            Stroke::NONE,
            egui::StrokeKind::Inside,
        );
        if ui.input(|i| i.pointer.primary_clicked()) {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
    let close_stroke_color = if close_hovered { Color32::WHITE } else { theme.muted };
    let close_c = close_rect.center();
    let d = 4.4;
    painter.line_segment(
        [pos2(close_c.x - d, close_c.y - d), pos2(close_c.x + d, close_c.y + d)],
        Stroke::new(1.5_f32, close_stroke_color),
    );
    painter.line_segment(
        [pos2(close_c.x + d, close_c.y - d), pos2(close_c.x - d, close_c.y + d)],
        Stroke::new(1.5_f32, close_stroke_color),
    );

    (action, accent_rect)
}
