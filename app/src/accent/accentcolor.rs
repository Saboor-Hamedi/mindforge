//! Accent color customization, palette overrides, and titlebar dropdown.
//!
//! v2: replaced the native egui color-picker popup with a custom hex input
//! (matches the app's "type it" visual language instead of a foreign OS-style
//! widget), switched all hit-testing to `ui.interact()` so hover/click/tooltip
//! come for free instead of manual pointer-position checks, and added
//! animated hover states via `ctx.animate_bool` (cheap: egui only requests a
//! repaint while a value is actually transitioning, then goes idle).

use crate::ui::theme::Theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Id, Rect, Sense, Stroke};

/// Persistent custom color overrides for theme accent, text, and highlight/selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AccentOverrides {
    pub accent: Option<Color32>,
    pub text: Option<Color32>,
    pub highlight: Option<Color32>,
}

impl AccentOverrides {
    pub fn apply(&self, theme: &mut Theme) {
        if let Some(c) = self.accent {
            theme.accent = c;
        }
        if let Some(c) = self.text {
            theme.text = c;
        }
        if let Some(c) = self.highlight {
            theme.highlight = c;
        }
    }

    pub fn is_empty(&self) -> bool {
        self.accent.is_none() && self.text.is_none() && self.highlight.is_none()
    }

    pub fn clear(&mut self) {
        self.accent = None;
        self.text = None;
        self.highlight = None;
    }

    pub fn load_from_db(db: &core::Database) -> Self {
        let get = |key: &str| db.get_setting(key).ok().flatten().and_then(|s| color_from_hex(&s));
        Self {
            accent: get("override_accent"),
            text: get("override_text"),
            highlight: get("override_highlight"),
        }
    }

    pub fn save_to_db(&self, db: &core::Database) {
        let put = |key: &str, c: Option<Color32>| {
            let _ = db.set_setting(key, &c.map(hex_from_color).unwrap_or_default());
        };
        put("override_accent", self.accent);
        put("override_text", self.text);
        put("override_highlight", self.highlight);
    }
}

pub enum AccentAction {
    Changed,
    ResetAll,
    Close,
}

/// Curated modern palette swatches, each with a short name for the hover tooltip.
const PALETTE_SWATCHES: &[(Color32, &str)] = &[
    (Color32::from_rgb(42, 161, 152), "Solarized Cyan"),
    (Color32::from_rgb(16, 185, 129), "Emerald Green"),
    (Color32::from_rgb(245, 158, 11), "Warm Amber"),
    (Color32::from_rgb(249, 115, 22), "Sunset Coral"),
    (Color32::from_rgb(244, 63, 94), "Crimson Rose"),
    (Color32::from_rgb(168, 85, 247), "Neon Violet"),
    (Color32::from_rgb(56, 189, 248), "Sky Blue"),
    (Color32::from_rgb(59, 130, 246), "Royal Blue"),
    (Color32::from_rgb(255, 255, 255), "Pure White"),
    (Color32::from_rgb(203, 213, 225), "Soft Silver"),
];

pub fn hex_from_color(c: Color32) -> String {
    format!("#{:02X}{:02X}{:02X}", c.r(), c.g(), c.b())
}

pub fn color_from_hex(s: &str) -> Option<Color32> {
    let s = s.trim().trim_start_matches('#');
    if s.len() == 6 {
        let r = u8::from_str_radix(&s[0..2], 16).ok()?;
        let g = u8::from_str_radix(&s[2..4], 16).ok()?;
        let b = u8::from_str_radix(&s[4..6], 16).ok()?;
        Some(Color32::from_rgb(r, g, b))
    } else {
        None
    }
}

/// Renders the accent color customizer dropdown beneath the titlebar palette button.
pub fn render_accent_dropdown(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    anchor_rect: Rect,
    overrides: &mut AccentOverrides,
    theme: &Theme,
    default_theme: &Theme,
    opacity: &mut f32,
    blur_effect: &mut crate::services::blur::BlurEffect,
    on_save_setting: &mut dyn FnMut(&str, &str),
) -> Option<AccentAction> {
    let mut action = None;

    let dropdown_w = 320.0;
    let dropdown_h = 394.0;
    let min_x = (anchor_rect.max.x - dropdown_w).max(8.0);
    let min_y = anchor_rect.max.y + 4.0;
    let dropdown_rect = Rect::from_min_size(pos2(min_x, min_y), vec2(dropdown_w, dropdown_h));

    // One-shot entrance animation: eases from 0->1 over ~120ms each time the
    // dropdown starts being drawn again. Cheap — egui only keeps repainting
    // while the value is actually moving, and clears stale animation state
    // automatically after the id goes unused for a while.
    let open_t = ui
        .ctx()
        .animate_bool_with_time(Id::new("accent_dropdown_open"), true, 0.12);
    let dropdown_rect = Rect::from_min_size(
        dropdown_rect.min + vec2(0.0, (1.0 - open_t) * -6.0),
        dropdown_rect.size(),
    );

    if ui.input(|i| i.pointer.primary_clicked()) {
        if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
            if !dropdown_rect.contains(pos) && !anchor_rect.contains(pos) {
                return Some(AccentAction::Close);
            }
        }
    }

    let alpha = (open_t * 255.0) as u8;

    // Soft ambient shadow (never a harsh black ring)
    let shadow_alpha = if theme.is_light() {
        (18.0 * open_t) as u8
    } else {
        (55.0 * open_t) as u8
    };
    painter.rect(
        dropdown_rect.expand(4.0),
        10.0,
        Color32::from_black_alpha(shadow_alpha),
        Stroke::NONE,
        egui::StrokeKind::Outside,
    );

    // Card body: sleek surface and border matching modal styling
    painter.rect(
        dropdown_rect,
        8.0,
        theme.surface().gamma_multiply(open_t),
        Stroke::new(1.0_f32, theme.border().gamma_multiply(open_t)),
        egui::StrokeKind::Inside,
    );

    // --- Header ---
    let header_h = 44.0;
    let header_rect =
        Rect::from_min_max(dropdown_rect.min, pos2(dropdown_rect.max.x, dropdown_rect.min.y + header_h));
    painter.line_segment(
        [header_rect.left_bottom(), header_rect.right_bottom()],
        Stroke::new(1.0_f32, theme.border().gamma_multiply(open_t)),
    );
    painter.text(
        pos2(header_rect.min.x + 14.0, header_rect.min.y + 13.0),
        Align2::LEFT_CENTER,
        "Color & Backdrop Customizer",
        FontId::monospace(13.0),
        theme.highlight.gamma_multiply(open_t),
    );
    painter.text(
        pos2(header_rect.min.x + 14.0, header_rect.min.y + 30.0),
        Align2::LEFT_CENTER,
        "accent, text, opacity & desktop blur",
        FontId::monospace(10.0),
        theme.muted.gamma_multiply(open_t),
    );

    let close_btn_rect =
        Rect::from_center_size(pos2(header_rect.max.x - 18.0, header_rect.center().y), vec2(22.0, 22.0));
    let close_hovered = ui.rect_contains_pointer(close_btn_rect);
    let close_t = ui.ctx().animate_bool(ui.id().with("close_btn_hover"), close_hovered);
    if close_t > 0.0 {
        let hover_bg = if theme.is_light() {
            Color32::from_black_alpha((18.0 * close_t) as u8)
        } else {
            Color32::from_white_alpha((25.0 * close_t) as u8)
        };
        painter.rect_filled(close_btn_rect, 4.0, hover_bg);
    }
    painter.text(
        close_btn_rect.center(),
        Align2::CENTER_CENTER,
        "\u{2715}",
        FontId::monospace(12.0),
        lerp_color(theme.muted, theme.text, close_t),
    );
    if close_hovered && ui.input(|i| i.pointer.primary_clicked()) {
        return Some(AccentAction::Close);
    }

    // --- Content ---
    let inner_rect = Rect::from_min_max(
        pos2(dropdown_rect.min.x + 10.0, header_rect.max.y + 8.0),
        pos2(dropdown_rect.max.x - 10.0, dropdown_rect.max.y - 44.0),
    );
    let mut changed = false;
    let section_h = 96.0;
    let section_gap = 6.0;
    let mut cur_y = inner_rect.min.y;

    for (label, slot, default_color) in [
        ("Accent Color", &mut overrides.accent, default_theme.accent),
        ("Text Color", &mut overrides.text, default_theme.text),
    ] {
        let rect = Rect::from_min_size(pos2(inner_rect.min.x, cur_y), vec2(inner_rect.width(), section_h));
        render_color_section(ui, painter, rect, label, slot, default_color, theme, &mut changed);
        cur_y += section_h + section_gap;
    }

    // --- Window Blur & Opacity Section ---
    let backdrop_card_rect = Rect::from_min_size(pos2(inner_rect.min.x, cur_y), vec2(inner_rect.width(), 84.0));
    // Clean transparent section with subtle divider — ZERO background fill
    painter.line_segment(
        [backdrop_card_rect.left_top(), backdrop_card_rect.right_top()],
        Stroke::new(1.0_f32, theme.border().gamma_multiply(open_t)),
    );

    // Row 1: Blur Effect Pills (ZERO background, ZERO border)
    let row1_y = backdrop_card_rect.min.y + 18.0;
    painter.text(
        pos2(backdrop_card_rect.min.x + 4.0, row1_y),
        Align2::LEFT_CENTER,
        "Blur Effect",
        FontId::monospace(11.5),
        theme.muted.gamma_multiply(open_t),
    );

    let effect_pills = [
        ("None", crate::services::blur::BlurEffect::None),
        ("Mica", crate::services::blur::BlurEffect::Mica),
        ("Acrylic", crate::services::blur::BlurEffect::Acrylic),
    ];
    let pill_w = 58.0;
    let pill_h = 22.0;
    let pill_gap = 6.0;
    let pills_total_w = 3.0 * pill_w + 2.0 * pill_gap;
    let pills_start_x = backdrop_card_rect.max.x - 4.0 - pills_total_w;

    for (p_idx, &(p_label, p_eff)) in effect_pills.iter().enumerate() {
        let p_rect = Rect::from_min_size(
            pos2(pills_start_x + p_idx as f32 * (pill_w + pill_gap), row1_y - 11.0),
            vec2(pill_w, pill_h),
        );
        let is_sel = *blur_effect == p_eff;
        let p_resp = ui.interact(p_rect, ui.id().with(("blur_pill", p_idx)), Sense::click());
        let p_hover = p_resp.hovered();

        if p_hover {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        // Clean solid text: ZERO background, ZERO border
        painter.text(
            p_rect.center(),
            Align2::CENTER_CENTER,
            p_label,
            FontId::monospace(12.0),
            if is_sel {
                theme.accent
            } else if p_hover {
                theme.text
            } else {
                theme.muted
            },
        );

        // Minimalist active indicator: 2px bottom accent bar (NO background fill)
        if is_sel {
            let active_bar = Rect::from_center_size(
                pos2(p_rect.center().x, p_rect.max.y - 1.0),
                vec2(pill_w - 14.0, 2.0),
            );
            painter.rect_filled(active_bar, 1.0, theme.accent);
        }

        if p_resp.clicked() {
            *blur_effect = p_eff;
            if p_eff == crate::services::blur::BlurEffect::None {
                *opacity = 1.0;
                on_save_setting("opacity", "1.00");
            } else if *opacity > 0.95 {
                *opacity = 0.88;
                on_save_setting("opacity", "0.88");
            }
            crate::services::blur::apply_window_blur(p_eff);
            on_save_setting("blur", p_eff.name().to_lowercase().as_str());
            ui.ctx().request_repaint();
        }
    }

    // Row 2: Opacity Slider
    let row2_y = backdrop_card_rect.min.y + 54.0;
    painter.text(
        pos2(backdrop_card_rect.min.x + 4.0, row2_y),
        Align2::LEFT_CENTER,
        "Opacity",
        FontId::monospace(11.5),
        theme.muted.gamma_multiply(open_t),
    );

    let slider_w = 120.0;
    let slider_x = backdrop_card_rect.max.x - 4.0 - slider_w - 42.0;
    let slider_rect = Rect::from_min_size(pos2(slider_x, row2_y - 9.0), vec2(slider_w, 18.0));
    let slider_resp = ui.interact(slider_rect, ui.id().with("accent_opacity_slider"), Sense::click_and_drag());
    let slider_hover = slider_resp.hovered() || slider_resp.dragged();

    if slider_resp.dragged() || slider_resp.clicked() {
        if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
            let t = ((pos.x - slider_rect.min.x) / slider_rect.width()).clamp(0.0, 1.0);
            let new_op = (0.4 + t * 0.6).clamp(0.4, 1.0);
            let rounded_op = (new_op * 100.0).round() / 100.0;
            if (*opacity - rounded_op).abs() > 0.005 {
                *opacity = rounded_op;
                on_save_setting("opacity", &format!("{:.2}", *opacity));
                ui.ctx().request_repaint();
            }
        }
    }

    let track_y = slider_rect.center().y;
    painter.rect_filled(Rect::from_min_size(pos2(slider_x, track_y - 2.0), vec2(slider_w, 4.0)), 2.0, theme.border());
    let active_w = slider_w * ((*opacity - 0.4) / 0.6).clamp(0.0, 1.0);
    if active_w > 0.0 {
        painter.rect_filled(Rect::from_min_size(pos2(slider_x, track_y - 2.0), vec2(active_w, 4.0)), 2.0, theme.accent);
    }
    let thumb_x = slider_x + active_w;
    painter.circle_filled(pos2(thumb_x, track_y), if slider_hover { 6.0 } else { 5.0 }, theme.accent);
    painter.text(
        pos2(slider_rect.max.x + 8.0, track_y),
        Align2::LEFT_CENTER,
        format!("{:.0}%", *opacity * 100.0),
        FontId::monospace(10.5),
        theme.accent,
    );

    // --- Footer: reset ---
    let footer_rect = Rect::from_min_max(
        pos2(dropdown_rect.min.x + 10.0, dropdown_rect.max.y - 36.0),
        pos2(dropdown_rect.max.x - 10.0, dropdown_rect.max.y - 8.0),
    );
    let reset_resp = ui.interact(footer_rect, ui.id().with("reset_all"), Sense::click());
    let reset_t = ui.ctx().animate_bool(reset_resp.id.with("hover"), reset_resp.hovered());
    let reset_bg = lerp_color(
        Color32::from_rgba_unmultiplied(theme.bg.r(), theme.bg.g(), theme.bg.b(), 180),
        Color32::from_rgba_unmultiplied(theme.accent.r(), theme.accent.g(), theme.accent.b(), 45),
        reset_t,
    );
    painter.rect(
        footer_rect,
        5.0,
        reset_bg,
        Stroke::new(1.0_f32, lerp_color(theme.border(), theme.accent, reset_t)),
        egui::StrokeKind::Inside,
    );
    painter.text(
        footer_rect.center(),
        Align2::CENTER_CENTER,
        "Reset All to Defaults",
        FontId::monospace(11.5),
        lerp_color(theme.muted, theme.accent, reset_t),
    );
    if reset_resp.clicked() {
        overrides.clear();
        *opacity = 1.0;
        *blur_effect = crate::services::blur::BlurEffect::None;
        crate::services::blur::apply_window_blur(crate::services::blur::BlurEffect::None);
        on_save_setting("opacity", "1.00");
        on_save_setting("blur", "none");
        action = Some(AccentAction::ResetAll);
    }

    let _ = alpha; // reserved if you want to fade content alpha too, not just the chrome
    if changed && action.is_none() {
        action = Some(AccentAction::Changed);
    }
    action
}

/// Renders a single color category sub-card: label, live hex badge (click to
/// copy), palette swatches with hover-scale + tooltip, and a custom hex
/// input field (replaces the native color picker so the whole panel stays
/// in the app's own visual language).
fn render_color_section(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    rect: Rect,
    label: &str,
    current_override: &mut Option<Color32>,
    default_color: Color32,
    theme: &Theme,
    changed: &mut bool,
) {
    let id = ui.id().with(label);
    let active_color = current_override.unwrap_or(default_color);
    let is_overridden = current_override.is_some();

    painter.rect(
        rect,
        5.0,
        Color32::from_rgba_unmultiplied(theme.bg.r(), theme.bg.g(), theme.bg.b(), 130),
        Stroke::new(1.0_f32, theme.border()),
        egui::StrokeKind::Inside,
    );

    let pad = 8.0;

    painter.text(
        pos2(rect.min.x + pad, rect.min.y + pad + 6.0),
        Align2::LEFT_CENTER,
        label,
        FontId::monospace(11.5),
        theme.text,
    );

    // Hex badge — click to copy.
    let hex_text = hex_from_color(active_color);
    let badge_rect = Rect::from_min_max(
        pos2(rect.max.x - pad - 82.0, rect.min.y + pad),
        pos2(rect.max.x - pad, rect.min.y + pad + 18.0),
    );
    let badge_resp = ui.interact(badge_rect, id.with("copy"), Sense::click());
    let badge_t = ui.ctx().animate_bool(id.with("copy_hover"), badge_resp.hovered());
    painter.rect(
        badge_rect,
        3.0,
        Color32::from_rgba_unmultiplied(
            theme.surface().r(),
            theme.surface().g(),
            theme.surface().b(),
            (200.0 + 40.0 * badge_t).min(255.0) as u8,
        ),
        Stroke::new(1.0_f32, lerp_color(theme.border(), theme.accent, badge_t)),
        egui::StrokeKind::Inside,
    );
    let chip_rect = Rect::from_min_size(pos2(badge_rect.min.x + 3.0, badge_rect.min.y + 2.0), vec2(14.0, 14.0));
    painter.rect_filled(chip_rect, 2.0, active_color);
    painter.text(
        pos2(chip_rect.max.x + 5.0, badge_rect.center().y),
        Align2::LEFT_CENTER,
        &hex_text,
        FontId::monospace(9.5),
        if is_overridden { theme.accent } else { theme.muted },
    );
    if badge_resp.clicked() {
        ui.ctx().copy_text(hex_text.clone());
    }
    badge_resp.on_hover_text(if is_overridden { "Click to copy" } else { "Default — click to copy" });

    // Reset chip, only when overridden.
    if is_overridden {
        let reset_chip = Rect::from_center_size(pos2(badge_rect.min.x - 10.0, badge_rect.center().y), vec2(14.0, 14.0));
        let reset_resp = ui.interact(reset_chip, id.with("reset"), Sense::click());
        let reset_t = ui.ctx().animate_bool(id.with("reset_hover"), reset_resp.hovered());
        if reset_t > 0.0 {
            painter.rect_filled(reset_chip, 3.0, Color32::from_white_alpha((25.0 * reset_t) as u8));
        }
        painter.text(
            reset_chip.center(),
            Align2::CENTER_CENTER,
            "\u{d7}",
            FontId::monospace(11.0),
            lerp_color(theme.muted, theme.accent, reset_t),
        );
        if reset_resp.clicked() {
            *current_override = None;
            *changed = true;
        }
        reset_resp.on_hover_text("Reset to default");
    }

    // Palette swatches: interact() gives hover/click/tooltip for free, and a
    // subtle scale-up on hover via animate_bool makes the row feel alive
    // without any continuous per-frame animation once settled.
    let swatch_y = rect.min.y + 32.0;
    let base_size = 20.0;
    let gap = 5.0;

    for (i, &(swatch, name)) in PALETTE_SWATCHES.iter().enumerate() {
        let sx = rect.min.x + pad + (i as f32 * (base_size + gap));
        let base_rect = Rect::from_min_size(pos2(sx, swatch_y), vec2(base_size, base_size));
        let resp = ui.interact(base_rect, id.with(("swatch", i)), Sense::click());
        let hover_t = ui.ctx().animate_bool(resp.id.with("hover"), resp.hovered());
        let is_selected = active_color == swatch;

        // Grow slightly on hover, centered on the same spot.
        let grown = base_rect.expand(hover_t * 2.0);

        let stroke = if is_selected {
            Stroke::new(1.8_f32, theme.accent)
        } else if hover_t > 0.0 {
            Stroke::new(1.0 + 0.5 * hover_t, theme.accent)
        } else {
            Stroke::new(0.8_f32, theme.border())
        };

        painter.rect(grown, 3.5, swatch, stroke, egui::StrokeKind::Inside);
        if is_selected {
            let dot_col = if crate::ui::theme::relative_luminance(swatch) > 0.5 {
                Color32::BLACK
            } else {
                Color32::WHITE
            };
            painter.circle_filled(grown.center(), 2.2, dot_col);
        }
        if resp.clicked() {
            *current_override = Some(swatch);
            *changed = true;
        }
        resp.on_hover_text(name);
    }

    // Custom hex input — replaces the native egui color-picker popup so this
    // panel never breaks out of the app's own black/monospace look.
    let field_y = swatch_y + base_size + 8.0;
    let field_rect = Rect::from_min_size(pos2(rect.min.x + pad, field_y), vec2(rect.width() - pad * 2.0, 20.0));
    if let Some(new_color) = hex_input(ui, painter, field_rect, id.with("hex_field"), active_color, theme) {
        *current_override = Some(new_color);
        *changed = true;
    }
}

/// A small custom hex text field (`#RRGGBB`), styled to match the rest of
/// the app instead of using egui's default TextEdit chrome. Commits on
/// Enter or when it loses focus; shows a red border while the text is not
/// valid hex so you get feedback without a popup or error dialog.
fn hex_input(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    rect: Rect,
    id: Id,
    active_color: Color32,
    theme: &Theme,
) -> Option<Color32> {
    let buf_id = id.with("buf");
    let mut buf = ui
        .ctx()
        .data_mut(|d| d.get_temp::<String>(buf_id))
        .unwrap_or_else(|| hex_from_color(active_color));

    // Interactive Color Picker button (opens egui's full palette/wheel picker popup) + hex field
    let preview_rect = Rect::from_min_size(rect.min, vec2(22.0, rect.height()));
    let mut srgba = [active_color.r(), active_color.g(), active_color.b(), 255];
    let picker_resp = ui
        .allocate_new_ui(egui::UiBuilder::new().max_rect(preview_rect), |ui| {
            ui.spacing_mut().interact_size = vec2(22.0, rect.height());
            ui.color_edit_button_srgba_unmultiplied(&mut srgba)
        })
        .inner;

    let mut committed = None;
    if picker_resp.changed() {
        let picked = Color32::from_rgb(srgba[0], srgba[1], srgba[2]);
        committed = Some(picked);
        buf = hex_from_color(picked);
    }

    let text_rect = Rect::from_min_max(pos2(preview_rect.max.x + 6.0, rect.min.y), rect.max);
    let valid = color_from_hex(&buf).is_some();
    let border = if !valid && !buf.is_empty() {
        Color32::from_rgb(220, 70, 70)
    } else {
        theme.border()
    };
    painter.rect(
        text_rect,
        3.0,
        Color32::from_rgba_unmultiplied(theme.bg.r(), theme.bg.g(), theme.bg.b(), 200),
        Stroke::new(1.0_f32, border),
        egui::StrokeKind::Inside,
    );

    let resp = ui
        .allocate_new_ui(egui::UiBuilder::new().max_rect(text_rect.shrink2(vec2(6.0, 2.0))), |ui| {
            ui.style_mut().visuals.extreme_bg_color = Color32::TRANSPARENT;
            ui.add(
                egui::TextEdit::singleline(&mut buf)
                    .font(FontId::monospace(11.0))
                    .text_color(theme.text)
                    .frame(false)
                    .desired_width(text_rect.width() - 12.0)
                    .hint_text("#RRGGBB"),
            )
        })
        .inner;

    if (resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) || resp.lost_focus() {
        if let Some(c) = color_from_hex(&buf) {
            committed = Some(c);
        }
        buf = hex_from_color(committed.unwrap_or(active_color));
    }

    ui.ctx().data_mut(|d| d.insert_temp(buf_id, buf));
    committed
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t) as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::theme::ThemeKind;

    #[test]
    fn test_hex_conversion() {
        let col = Color32::from_rgb(1, 43, 54);
        let hex = hex_from_color(col);
        assert_eq!(hex, "#012B36");
        let parsed = color_from_hex(&hex).unwrap();
        assert_eq!(parsed, col);
    }

    #[test]
    fn test_accent_overrides_apply_and_clear() {
        let mut theme = Theme::from_kind(ThemeKind::Shell);
        let custom_accent = Color32::from_rgb(255, 0, 128);
        let custom_text = Color32::from_rgb(200, 220, 240);

        let mut overrides = AccentOverrides {
            accent: Some(custom_accent),
            text: Some(custom_text),
            highlight: None,
        };

        overrides.apply(&mut theme);
        assert_eq!(theme.accent, custom_accent);
        assert_eq!(theme.text, custom_text);

        overrides.clear();
        assert!(overrides.is_empty());
    }

    #[test]
    fn test_invalid_hex_rejected() {
        assert_eq!(color_from_hex("not-a-color"), None);
        assert_eq!(color_from_hex("#zzzzzz"), None);
        assert!(color_from_hex("#a1b2c3").is_some());
    }
}
