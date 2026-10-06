//! Caret styles and physics settings tab.

use crate::caret::{Caret, CaretKind};
use crate::ui::theme::Theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Pos2, Rect, Stroke};

/// Per-style accent colors shown as indicator dots on each caret chip.
/// Per-style accent colors shown as indicator dots on each caret chip.
pub fn caret_dot_color(kind: CaretKind, theme: &Theme) -> Color32 {
    let is_light = theme.is_light();
    match kind {
        CaretKind::Block => theme.accent,
        CaretKind::Beam => theme.accent,
        CaretKind::Underline => theme.accent,
        CaretKind::Candle => {
            if is_light {
                Color32::from_rgb(195, 120, 20)
            } else {
                Color32::from_rgb(255, 190, 70)
            }
        }
        CaretKind::Fire => Color32::from_rgb(235, 95, 20),
        CaretKind::Water => {
            if is_light {
                Color32::from_rgb(20, 130, 225)
            } else {
                Color32::from_rgb(65, 175, 255)
            }
        }
        CaretKind::Snow => {
            if is_light {
                Color32::from_rgb(35, 115, 185)
            } else {
                Color32::from_rgb(225, 245, 255)
            }
        }
        CaretKind::Electric => {
            if is_light {
                Color32::from_rgb(100, 70, 220)
            } else {
                Color32::from_rgb(190, 220, 255)
            }
        }
        CaretKind::Comet => {
            if is_light {
                Color32::from_rgb(120, 60, 200)
            } else {
                Color32::from_rgb(200, 200, 255)
            }
        }
        CaretKind::Rainbow => Color32::from_rgb(225, 60, 170),
        CaretKind::Matrix => {
            if is_light {
                Color32::from_rgb(25, 145, 55)
            } else {
                Color32::from_rgb(40, 255, 90)
            }
        }
        CaretKind::Ice => {
            if is_light {
                Color32::from_rgb(15, 140, 195)
            } else {
                Color32::from_rgb(130, 230, 255)
            }
        }
        CaretKind::Glitch => Color32::from_rgb(235, 45, 95),
        CaretKind::Neon => {
            if is_light {
                Color32::from_rgb(25, 150, 60)
            } else {
                Color32::from_rgb(180, 255, 180)
            }
        }
        CaretKind::Heartbeat => Color32::from_rgb(235, 50, 90),
    }
}

pub fn render_carets_tab(
    ui: &egui::Ui,
    painter: &egui::Painter,
    panel_rect: Rect,
    p_origin: Pos2,
    caret: &mut Caret,
    theme: &Theme,
    on_save_setting: &mut dyn FnMut(&str, &str),
) {
    let available_w = (panel_rect.width() - 56.0).max(300.0);

    painter.text(
        p_origin,
        Align2::LEFT_TOP,
        "CARET CUSTOMIZATION",
        FontId::proportional(14.0),
        theme.highlight,
    );
    painter.text(
        p_origin + vec2(0.0, 20.0),
        Align2::LEFT_TOP,
        format!(
            "Choose from {} animated styles & particle effects",
            CaretKind::ALL.len()
        ),
        FontId::proportional(11.5),
        theme.muted,
    );

    // ── Expanded Chip Grid ────────────────────────────────────────────
    let chip_start_y = p_origin.y + 46.0;
    let col_gap = 12.0;
    let row_gap = 10.0;
    let col_w = ((available_w - 3.0 * col_gap) / 4.0).floor().max(80.0);
    let row_h = 44.0;

    for (idx, &kind) in CaretKind::ALL.iter().enumerate() {
        let col = idx % 4;
        let row = idx / 4;
        let chip_rect = Rect::from_min_size(
            pos2(
                p_origin.x + col as f32 * (col_w + col_gap),
                chip_start_y + row as f32 * (row_h + row_gap),
            ),
            vec2(col_w, row_h),
        );
        let is_selected = caret.kind == kind;
        let hovered = ui.rect_contains_pointer(chip_rect);
        let dot_color = caret_dot_color(kind, theme);

        if hovered {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        let stroke = if is_selected {
            Stroke::new(1.0_f32, theme.accent)
        } else if hovered {
            Stroke::new(1.0_f32, theme.border().lerp_to_gamma(theme.accent, 0.4))
        } else {
            Stroke::new(1.0_f32, theme.border())
        };
        painter.rect(
            chip_rect,
            6.0,
            theme.surface(),
            stroke,
            egui::StrokeKind::Inside,
        );

        // Color indicator dot
        painter.circle_filled(
            pos2(chip_rect.max.x - 12.0, chip_rect.min.y + 12.0),
            4.0,
            dot_color,
        );

        // Label — highlighted in accent color only, never background
        painter.text(
            pos2(chip_rect.min.x + 14.0, chip_rect.center().y),
            Align2::LEFT_CENTER,
            kind.name(),
            FontId::proportional(12.5),
            if is_selected {
                theme.accent
            } else if hovered {
                theme.highlight
            } else {
                theme.text
            },
        );

        if hovered && ui.input(|i| i.pointer.primary_clicked()) {
            caret.kind = kind;
            on_save_setting("caret", kind.name());
        }
    }

    // ── Description card ─────────────────────────────────────────────
    let rows = (CaretKind::ALL.len() + 3) / 4;
    let desc_y = chip_start_y + rows as f32 * (row_h + row_gap) + 10.0;
    let desc_rect = Rect::from_min_size(pos2(p_origin.x, desc_y), vec2(available_w, 36.0));
    painter.rect_filled(desc_rect, 5.0, theme.surface());
    painter.text(
        pos2(desc_rect.min.x + 14.0, desc_rect.center().y),
        Align2::LEFT_CENTER,
        format!(
            "{}  —  {}",
            caret.kind.name().to_uppercase(),
            caret.kind.description()
        ),
        FontId::monospace(11.5),
        theme.text,
    );

    // ── Controls row 1: Width on Left, Blinking on Right ─────────────────
    let controls_y = desc_y + 46.0;
    let controls_h = 30.0;

    // LEFT: Width Slider with label
    painter.text(
        pos2(p_origin.x, controls_y + controls_h * 0.5),
        Align2::LEFT_CENTER,
        "Width",
        FontId::monospace(12.0),
        theme.muted,
    );

    let slider_x = p_origin.x + 56.0;
    let slider_w = 150.0;
    let slider_rect = Rect::from_min_size(
        pos2(slider_x, controls_y + (controls_h - 20.0) * 0.5),
        vec2(slider_w, 20.0),
    );

    let slider_hover = ui.rect_contains_pointer(slider_rect);
    let is_down = ui.input(|i| i.pointer.primary_down() || i.pointer.primary_clicked());
    if slider_hover && is_down {
        if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
            let t = ((pos.x - slider_rect.min.x) / slider_rect.width()).clamp(0.0, 1.0);
            let new_val = (1.0 + t * 9.0).round().clamp(1.0, 10.0);
            if (caret.width - new_val).abs() > 0.01 {
                caret.width = new_val;
                on_save_setting("caret_width", &format!("{:.0}", caret.width));
            }
        }
    }

    let track_y = slider_rect.center().y;
    let track_h = 5.0;
    let track_rect = Rect::from_min_size(
        pos2(slider_rect.min.x, track_y - track_h * 0.5),
        vec2(slider_w, track_h),
    );
    painter.rect_filled(track_rect, 3.0, theme.border());

    let t = ((caret.width - 1.0) / 9.0).clamp(0.0, 1.0);
    let active_w = slider_w * t;
    if active_w > 0.0 {
        let active_rect = Rect::from_min_size(
            pos2(slider_rect.min.x, track_y - track_h * 0.5),
            vec2(active_w, track_h),
        );
        painter.rect_filled(active_rect, 3.0, theme.accent);
    }

    let thumb_x = slider_rect.min.x + active_w;
    painter.circle_filled(
        pos2(thumb_x, track_y),
        if slider_hover { 8.5 } else { 7.5 },
        if slider_hover {
            Color32::WHITE
        } else {
            theme.accent
        },
    );
    painter.circle_stroke(
        pos2(thumb_x, track_y),
        if slider_hover { 8.5 } else { 7.5 },
        Stroke::new(1.5_f32, theme.border()),
    );

    painter.text(
        pos2(slider_rect.max.x + 12.0, track_y),
        Align2::LEFT_CENTER,
        format!("{:.0}px", caret.width),
        FontId::monospace(12.0),
        theme.accent,
    );

    // RIGHT: Blinking Toggle
    let pill_w = 40.0;
    let blink_pill_pos = pos2(
        p_origin.x + available_w - pill_w,
        controls_y + (controls_h - 22.0) * 0.5,
    );
    if crate::ui_components::toggle::render_toggle_with_label(
        ui,
        painter,
        blink_pill_pos,
        "Blinking Caret",
        &mut caret.blink_enabled,
        theme,
        "caret_blink_toggle",
    ) {
        on_save_setting(
            "caret_blinking",
            if caret.blink_enabled { "on" } else { "off" },
        );
    }

    // ── Controls row 2: Effects on Left, Living Animations on Right ───
    let row2_y = controls_y + 34.0;
    painter.text(
        pos2(p_origin.x, row2_y + controls_h * 0.5),
        Align2::LEFT_CENTER,
        "Physics & FX",
        FontId::monospace(12.0),
        theme.muted,
    );

    let anim_pill_pos = pos2(
        p_origin.x + available_w - pill_w,
        row2_y + (controls_h - 22.0) * 0.5,
    );
    if crate::ui_components::toggle::render_toggle_with_label(
        ui,
        painter,
        anim_pill_pos,
        "Living Animations",
        &mut caret.animations_enabled,
        theme,
        "caret_anim_toggle",
    ) {
        on_save_setting(
            "caret_animations",
            if caret.animations_enabled {
                "on"
            } else {
                "off"
            },
        );
    }
}
