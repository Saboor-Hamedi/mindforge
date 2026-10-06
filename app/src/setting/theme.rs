//! Appearance & theme settings tab.
//!
//! v2: the grid now scrolls. At 11 themes, 6 rows mostly fit in a normal
//! panel; at 17 themes (9 rows) that stopped being true, and since the old
//! code drew every card unconditionally with no clipping or scroll offset,
//! cards rendered below the visible panel were still clickable — an actual
//! bug, not just a cosmetic one. Also added a visible contrast badge per
//! card (AA/AAA) using the `contrast_ratio` check from theme.rs, so the
//! readability guarantee is something the user can actually see.

use crate::ui::theme::{contrast_ratio, Theme, ThemeKind};
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Id, Pos2, Rect, Stroke};

pub fn render_theme_tab(
    ui: &egui::Ui,
    painter: &egui::Painter,
    panel_rect: Rect,
    p_origin: Pos2,
    theme: &mut Theme,
    _opacity: &mut f32,
    _blur_effect: &mut crate::services::blur::BlurEffect,
    on_save_setting: &mut dyn FnMut(&str, &str),
) {
    painter.text(
        p_origin,
        Align2::LEFT_TOP,
        "APPEARANCE & THEME",
        FontId::proportional(15.0),
        theme.highlight,
    );
    painter.text(
        p_origin + vec2(0.0, 22.0),
        Align2::LEFT_TOP,
        "Curated developer palettes and desktop frosted blur controls",
        FontId::proportional(12.0),
        theme.muted,
    );
    let pad_x = 20.0;
    let grid_left = panel_rect.min.x + pad_x;
    let grid_right = panel_rect.max.x - pad_x;
    let grid_w = grid_right - grid_left;

    let col_gap = 14.0;
    let row_gap = 12.0;
    let card_w = (grid_w - col_gap) * 0.5;
    let card_h = 80.0;
    let grid_top = p_origin.y + 48.0;

    let rows = ThemeKind::ALL.len().div_ceil(2);
    let content_h = rows as f32 * (card_h + row_gap) + 64.0;
    let visible_h = (panel_rect.max.y - grid_top).max(0.0);
    let max_scroll = (content_h - visible_h).max(0.0);

    // Persistent scroll offset for this tab
    let scroll_id = Id::new("theme_tab_scroll");
    let mut scroll = ui
        .ctx()
        .data_mut(|d| d.get_temp::<f32>(scroll_id))
        .unwrap_or(0.0);

    let pointer_over_grid = ui.rect_contains_pointer(Rect::from_min_max(
        pos2(panel_rect.min.x, grid_top),
        panel_rect.max,
    ));
    if pointer_over_grid && max_scroll > 0.0 {
        let wheel = ui.input(|i| i.smooth_scroll_delta.y);
        scroll = (scroll - wheel).clamp(0.0, max_scroll);
    }
    ui.ctx().data_mut(|d| d.insert_temp(scroll_id, scroll));

    // Everything below grid_top is clipped to the panel
    let clip_rect = Rect::from_min_max(pos2(panel_rect.min.x, grid_top), panel_rect.max);
    let grid_painter = painter.with_clip_rect(clip_rect);

    let mut selected: Option<ThemeKind> = None;

    for (i, &t_kind) in ThemeKind::ALL.iter().enumerate() {
        let col = i % 2;
        let row = i / 2;
        let t_preset = Theme::from_kind(t_kind);

        let unscrolled = Rect::from_min_size(
            pos2(
                grid_left + col as f32 * (card_w + col_gap),
                grid_top + row as f32 * (card_h + row_gap),
            ),
            vec2(card_w, card_h),
        );
        let t_rect = unscrolled.translate(vec2(0.0, -scroll));

        if !clip_rect.intersects(t_rect) {
            continue;
        }

        let is_sel = theme.kind == t_kind;
        let hovered = ui.rect_contains_pointer(t_rect);
        let hover_t = ui
            .ctx()
            .animate_bool(Id::new(("theme_card_hover", t_kind)), hovered);

        if hovered {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        let card_bg = {
            let base = theme.surface();
            let hover_bg = lerp_color(base, t_preset.accent, 0.05);
            lerp_color(base, hover_bg, hover_t)
        };

        let card_stroke = if is_sel {
            Stroke::new(1.0, t_preset.accent)
        } else {
            Stroke::new(
                1.0,
                lerp_color(theme.border(), t_preset.accent, hover_t * 0.7),
            )
        };

        grid_painter.rect(t_rect, 7.0, card_bg, card_stroke, egui::StrokeKind::Inside);

        // --- Left: Miniature Live UI Canvas ---
        let preview_w = 90.0;
        let preview_h = 60.0;
        let preview_rect = Rect::from_min_size(
            pos2(t_rect.min.x + 10.0, t_rect.min.y + 10.0),
            vec2(preview_w, preview_h),
        );

        // Preview card surface
        grid_painter.rect(
            preview_rect,
            5.0,
            t_preset.bg,
            Stroke::new(1.0, t_preset.border()),
            egui::StrokeKind::Inside,
        );

        // Three window control dots
        let dot_y = preview_rect.min.y + 6.0;
        for (d_idx, dot_color) in [
            Color32::from_rgb(255, 95, 86),
            Color32::from_rgb(255, 189, 46),
            Color32::from_rgb(39, 201, 63),
        ]
        .iter()
        .enumerate()
        {
            let dot_center = pos2(preview_rect.min.x + 8.0 + d_idx as f32 * 6.5, dot_y);
            grid_painter.circle_filled(dot_center, 2.0, *dot_color);
        }

        // Live typography mockup lines inside preview canvas
        let line_start_x = preview_rect.min.x + 8.0;
        let line1_y = preview_rect.min.y + 16.0;
        grid_painter.text(
            pos2(line_start_x, line1_y),
            Align2::LEFT_TOP,
            "fn",
            FontId::monospace(8.0),
            t_preset.accent,
        );
        grid_painter.text(
            pos2(line_start_x + 14.0, line1_y),
            Align2::LEFT_TOP,
            "note()",
            FontId::monospace(8.0),
            t_preset.text,
        );

        let line2_y = line1_y + 11.5;
        grid_painter.text(
            pos2(line_start_x + 6.0, line2_y),
            Align2::LEFT_TOP,
            "// live",
            FontId::monospace(7.5),
            t_preset.muted,
        );

        let line3_y = line2_y + 11.5;
        grid_painter.text(
            pos2(line_start_x + 6.0, line3_y),
            Align2::LEFT_TOP,
            "\"text\"",
            FontId::monospace(7.5),
            t_preset.highlight,
        );

        // --- Right: Details, Badges & Color Swatches ---
        let detail_left = preview_rect.max.x + 14.0;
        let detail_right = t_rect.max.x - 12.0;

        // Title
        grid_painter.text(
            pos2(detail_left, t_rect.min.y + 11.0),
            Align2::LEFT_TOP,
            t_kind.display_name(),
            FontId::proportional(13.2),
            if is_sel { t_preset.accent } else { theme.text },
        );

        // Top Right: ACTIVE pill or LIGHT/DARK mode badge
        if is_sel {
            let active_pill = Rect::from_min_size(
                pos2(detail_right - 58.0, t_rect.min.y + 10.0),
                vec2(58.0, 18.0),
            );
            grid_painter.rect_filled(
                active_pill,
                4.0,
                Color32::from_rgba_unmultiplied(
                    t_preset.accent.r(),
                    t_preset.accent.g(),
                    t_preset.accent.b(),
                    38,
                ),
            );
            grid_painter.text(
                active_pill.center(),
                Align2::CENTER_CENTER,
                "✓ ACTIVE",
                FontId::monospace(9.5),
                t_preset.accent,
            );
        } else {
            let is_light = t_kind.is_light();
            let mode_label = if is_light { "LIGHT" } else { "DARK" };
            let (mode_bg, mode_fg) = if is_light {
                (
                    Color32::from_rgba_unmultiplied(225, 150, 40, 32),
                    Color32::from_rgb(215, 140, 25),
                )
            } else {
                (
                    Color32::from_rgba_unmultiplied(
                        theme.muted.r(),
                        theme.muted.g(),
                        theme.muted.b(),
                        26,
                    ),
                    theme.muted,
                )
            };
            let mode_pill = Rect::from_min_size(
                pos2(detail_right - 44.0, t_rect.min.y + 10.0),
                vec2(44.0, 17.0),
            );
            grid_painter.rect_filled(mode_pill, 3.5, mode_bg);
            grid_painter.text(
                mode_pill.center(),
                Align2::CENTER_CENTER,
                mode_label,
                FontId::monospace(9.0),
                mode_fg,
            );
        }

        // Subtitle: Readability ratio
        let ratio = contrast_ratio(t_preset.text, t_preset.bg);
        let label = if ratio >= 7.0 { "AAA" } else { "AA" };
        grid_painter.text(
            pos2(detail_left, t_rect.min.y + 33.0),
            Align2::LEFT_TOP,
            format!("{label} · {ratio:.1}:1 contrast"),
            FontId::monospace(10.0),
            theme.muted,
        );

        // Circular Palette Dots
        let dot_y = t_rect.min.y + 57.0;
        let swatches = [
            t_preset.bg,
            t_preset.text,
            t_preset.accent,
            t_preset.highlight,
        ];
        for (s_idx, &color) in swatches.iter().enumerate() {
            let dot_center = pos2(detail_left + 6.0 + s_idx as f32 * 17.0, dot_y);
            grid_painter.circle_filled(dot_center, 5.5, color);
            grid_painter.circle_stroke(
                dot_center,
                5.5,
                Stroke::new(1.0, Color32::from_rgba_unmultiplied(128, 128, 128, 60)),
            );
        }

        if hovered && ui.input(|inp| inp.pointer.primary_clicked()) {
            selected = Some(t_kind);
        }
    }

    // Thin custom scrollbar so it's obvious there's more content below —
    // visual only, no drag-to-scroll (mouse wheel covers the common case).
    if max_scroll > 0.0 {
        let track_x = panel_rect.max.x - 6.0;
        let track = Rect::from_min_max(
            pos2(track_x, grid_top),
            pos2(track_x + 3.0, panel_rect.max.y),
        );
        painter.rect_filled(
            track,
            1.5,
            Color32::from_rgba_unmultiplied(255, 255, 255, 15),
        );

        let thumb_h = (visible_h * visible_h / content_h).max(24.0);
        let thumb_y = grid_top + (scroll / max_scroll) * (visible_h - thumb_h);
        let thumb = Rect::from_min_size(pos2(track_x, thumb_y), vec2(3.0, thumb_h));
        painter.rect_filled(
            thumb,
            1.5,
            Color32::from_rgba_unmultiplied(255, 255, 255, 60),
        );
    }

    if let Some(t_kind) = selected {
        *theme = Theme::from_kind(t_kind);
        on_save_setting("theme", t_kind.name());
    }
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t) as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}
