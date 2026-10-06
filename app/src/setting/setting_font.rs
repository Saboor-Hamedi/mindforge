//! Font customization and typography preferences panel.

use crate::services::font_manager::SUPPORTED_FONTS;
use crate::ui::theme::Theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Pos2, Rect, Stroke};

/// Renders the sleek typography and font family selection panel.
pub fn render_font_settings(
    ui: &egui::Ui,
    painter: &egui::Painter,
    panel_rect: Rect,
    origin: Pos2,
    selected_font: &mut String,
    font_size: &mut f32,
    theme: &Theme,
    on_save_setting: &mut dyn FnMut(&str, &str),
) {
    let available_w = panel_rect.width() - 56.0;

    // ── Header ──────────────────────────────────────────────────────────
    painter.text(
        origin,
        Align2::LEFT_TOP,
        "EDITOR TYPOGRAPHY & FONTS",
        FontId::proportional(15.0),
        theme.highlight,
    );
    painter.text(
        origin + vec2(0.0, 22.0),
        Align2::LEFT_TOP,
        "Select your preferred coding typeface • Safe zero-crash font engine",
        FontId::proportional(12.0),
        theme.muted,
    );

    // ── Font Cards Stack (matching Editor Mode cards) ───────────────────
    let mut cur_y = origin.y + 50.0;
    let card_h = 48.0;
    let card_gap = 7.0;

    for font in SUPPORTED_FONTS.iter() {
        let card_rect = Rect::from_min_size(pos2(origin.x, cur_y), vec2(available_w, card_h));

        let is_selected = selected_font.eq_ignore_ascii_case(font.display_name)
            || (selected_font.is_empty() && font.id == "jetbrains_mono");
        let is_installed = crate::services::font_manager::is_font_available(font.display_name);
        let hovered = ui.rect_contains_pointer(card_rect);

        if hovered {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        let bg = if hovered {
            theme.surface().lerp_to_gamma(theme.accent, 0.04)
        } else {
            theme.surface()
        };

        // Beautiful card border matching Editor Mode cards
        painter.rect(
            card_rect,
            6.0,
            bg,
            Stroke::new(
                1.0_f32,
                if is_selected {
                    theme.accent
                } else if hovered {
                    theme.border().lerp_to_gamma(theme.accent, 0.4)
                } else {
                    theme.border()
                },
            ),
            egui::StrokeKind::Inside,
        );

        // Title on row 1
        painter.text(
            card_rect.min + vec2(14.0, 8.5),
            Align2::LEFT_TOP,
            font.display_name,
            FontId::proportional(13.5),
            if is_selected {
                theme.accent
            } else {
                theme.text
            },
        );

        // Status pill / badge on right of row 1
        if is_selected {
            painter.text(
                pos2(card_rect.max.x - 14.0, card_rect.min.y + 9.5),
                Align2::RIGHT_TOP,
                "● ACTIVE",
                FontId::proportional(11.0),
                theme.accent,
            );
        } else if font.is_embedded {
            painter.text(
                pos2(card_rect.max.x - 14.0, card_rect.min.y + 9.5),
                Align2::RIGHT_TOP,
                "BUILT-IN",
                FontId::proportional(10.5),
                theme.accent.lerp_to_gamma(theme.muted, 0.3),
            );
        } else if is_installed {
            painter.text(
                pos2(card_rect.max.x - 14.0, card_rect.min.y + 9.5),
                Align2::RIGHT_TOP,
                "INSTALLED",
                FontId::proportional(10.5),
                theme.accent.lerp_to_gamma(theme.muted, 0.4),
            );
        } else {
            painter.text(
                pos2(card_rect.max.x - 14.0, card_rect.min.y + 9.5),
                Align2::RIGHT_TOP,
                "NOT INSTALLED",
                FontId::proportional(10.5),
                Color32::from_rgb(180, 110, 100),
            );
        }

        // Subtitle / description on row 2
        let desc = if is_installed || font.is_embedded {
            font.description.to_string()
        } else {
            format!(
                "{} • Install in Windows or drop .ttf into fonts/",
                font.description
            )
        };
        painter.text(
            card_rect.min + vec2(14.0, 27.5),
            Align2::LEFT_TOP,
            desc,
            FontId::proportional(10.5),
            theme.muted,
        );

        if hovered && ui.input(|i| i.pointer.primary_clicked()) {
            *selected_font = font.display_name.to_string();
            on_save_setting("selected_font", font.display_name);
            crate::services::font_manager::apply_font(ui.ctx(), font.display_name);
        }

        cur_y += card_h + card_gap;
    }

    // ── Font Size Slider ────────────────────────────────────────────────
    let slider_row_y = cur_y + 14.0;
    let slider_h = 24.0;

    painter.text(
        pos2(origin.x, slider_row_y + slider_h * 0.5),
        Align2::LEFT_CENTER,
        "Font Size",
        FontId::monospace(12.0),
        theme.muted,
    );

    let slider_x = origin.x + 76.0;
    let slider_w = 160.0;
    let slider_rect = Rect::from_min_size(
        pos2(slider_x, slider_row_y + (slider_h - 20.0) * 0.5),
        vec2(slider_w, 20.0),
    );

    let slider_hover = ui.rect_contains_pointer(slider_rect);
    let is_down = ui.input(|i| i.pointer.primary_down() || i.pointer.primary_clicked());
    if slider_hover && is_down {
        if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
            let t = ((pos.x - slider_rect.min.x) / slider_rect.width()).clamp(0.0, 1.0);
            let new_size = (12.0 + t * 16.0).round().clamp(12.0, 28.0);
            if (*font_size - new_size).abs() > 0.01 {
                *font_size = new_size;
                on_save_setting("font", &format!("{:.0}", *font_size));
            }
        }
    }

    let track_y = slider_rect.center().y;
    let track_h = 4.0;
    let track_rect = Rect::from_min_size(
        pos2(slider_rect.min.x, track_y - track_h * 0.5),
        vec2(slider_w, track_h),
    );
    painter.rect_filled(track_rect, 2.0, theme.border());

    let t = ((*font_size - 12.0) / 16.0).clamp(0.0, 1.0);
    let active_w = slider_w * t;
    if active_w > 0.0 {
        let active_rect = Rect::from_min_size(
            pos2(slider_rect.min.x, track_y - track_h * 0.5),
            vec2(active_w, track_h),
        );
        painter.rect_filled(active_rect, 2.0, theme.accent);
    }

    let thumb_x = slider_rect.min.x + active_w;
    painter.circle_filled(
        pos2(thumb_x, track_y),
        if slider_hover { 7.5 } else { 6.5 },
        if slider_hover {
            Color32::WHITE
        } else {
            theme.accent
        },
    );

    painter.text(
        pos2(slider_rect.max.x + 12.0, track_y),
        Align2::LEFT_CENTER,
        format!("{:.0}px", *font_size),
        FontId::monospace(12.0),
        theme.accent,
    );
}
