//! Sound profile settings tab.

use crate::services::sound::{SoundEngine, SoundProfile};
use crate::ui::theme::Theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Pos2, Rect, Stroke};

/// Waveform bar heights representing audio character of each profile.
pub fn sound_wave_heights(profile: SoundProfile) -> [f32; 7] {
    match profile {
        SoundProfile::Off    => [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0],
        SoundProfile::Thocky => [3.0, 6.0, 9.0, 11.0, 8.0, 5.0, 2.0],
        SoundProfile::Clacky => [4.0, 9.0, 12.0, 10.0, 7.0, 4.0, 2.0],
        SoundProfile::Creamy => [2.0, 5.0, 8.0, 10.0, 8.0, 5.0, 2.0],
        SoundProfile::Marbly => [5.0, 10.0, 13.0, 10.0, 6.0, 3.0, 1.0],
        SoundProfile::Poppy  => [6.0, 11.0, 13.0, 9.0, 5.0, 2.0, 1.0],
        SoundProfile::Clicky => [4.0, 8.0, 13.0, 12.0, 8.0, 4.0, 2.0],
    }
}

pub fn render_sounds_tab(
    ui: &egui::Ui,
    painter: &egui::Painter,
    panel_rect: Rect,
    p_origin: Pos2,
    sound: &mut SoundEngine,
    theme: &Theme,
    on_save_setting: &mut dyn FnMut(&str, &str),
) {
    painter.text(
        p_origin,
        Align2::LEFT_TOP,
        "MECHANICAL KEYBOARD SOUNDS",
        FontId::proportional(15.0),
        theme.highlight,
    );
    painter.text(
        p_origin + vec2(0.0, 22.0),
        Align2::LEFT_TOP,
        "Acoustic profiles synthesized with 0ms latency",
        FontId::proportional(12.0),
        theme.muted,
    );

    // Mute All toggle in top-right of section
    let mute_btn_w = 96.0;
    let mute_btn = Rect::from_min_size(
        pos2(panel_rect.max.x - mute_btn_w - 28.0, p_origin.y),
        vec2(mute_btn_w, 24.0),
    );
    let mute_hover = ui.rect_contains_pointer(mute_btn);
    let is_muted = sound.profile == SoundProfile::Off;

    if mute_hover && ui.input(|i| i.pointer.primary_clicked()) {
        if is_muted {
            sound.profile = SoundProfile::Thocky;
            on_save_setting("sound", "thocky");
        } else {
            sound.profile = SoundProfile::Off;
            on_save_setting("sound", "off");
        }
    }

    let mute_bg = if is_muted {
        Color32::from_rgba_unmultiplied(220, 60, 60, 28)
    } else if mute_hover {
        theme.surface().lerp_to_gamma(theme.accent, 0.12)
    } else {
        theme.surface()
    };
    let mute_stroke = if is_muted {
        Stroke::new(1.0, Color32::from_rgb(220, 80, 80))
    } else if mute_hover {
        Stroke::new(1.0, theme.accent)
    } else {
        Stroke::new(1.0, theme.border())
    };
    painter.rect(
        mute_btn,
        4.0,
        mute_bg,
        mute_stroke,
        egui::StrokeKind::Inside,
    );
    painter.text(
        mute_btn.center(),
        Align2::CENTER_CENTER,
        if is_muted { "🔇 Muted" } else { "🔊 Sound: On" },
        FontId::proportional(11.0),
        if is_muted { Color32::from_rgb(220, 80, 80) } else { theme.text },
    );

    // ── Sound Chips (4 responsive columns) ───────────────────────────
    let sound_start_y = p_origin.y + 56.0;
    let available_w = panel_rect.width() - 56.0;
    let cols = 4;
    let col_gap = 10.0;
    let row_gap = 10.0;
    let s_col_w = (available_w - (cols - 1) as f32 * col_gap) / cols as f32;
    let s_row_h = 52.0;

    for (i, &profile) in SoundProfile::ALL.iter().enumerate() {
        let col = i % cols;
        let row = i / cols;
        let s_rect = Rect::from_min_size(
            pos2(
                p_origin.x + col as f32 * (s_col_w + col_gap),
                sound_start_y + row as f32 * (s_row_h + row_gap),
            ),
            vec2(s_col_w, s_row_h),
        );
        let is_sel = sound.profile == profile;
        let s_hovered = ui.rect_contains_pointer(s_rect);

        let bg = if s_hovered {
            theme.surface().lerp_to_gamma(theme.accent, 0.05)
        } else {
            theme.surface()
        };
        let border_stroke = if is_sel {
            Stroke::new(1.0, theme.accent)
        } else if s_hovered {
            Stroke::new(1.0, theme.border().lerp_to_gamma(theme.accent, 0.4))
        } else {
            Stroke::new(1.0, theme.border())
        };

        painter.rect(s_rect, 6.0, bg, border_stroke, egui::StrokeKind::Inside);

        // Mini waveform bars
        let bars = sound_wave_heights(profile);
        let bar_area_w = 52.0;
        let bar_w = 5.0;
        let bar_gap = 2.5;
        let bar_base_y = s_rect.center().y + 8.0;
        let bar_x0 = s_rect.min.x + (s_col_w - bar_area_w) * 0.5;
        let wave_color = if is_sel { theme.accent } else { theme.muted };

        for (bi, &h) in bars.iter().enumerate() {
            let bx = bar_x0 + bi as f32 * (bar_w + bar_gap);
            painter.rect_filled(
                Rect::from_min_size(pos2(bx, bar_base_y - h), vec2(bar_w, h)),
                1.5,
                wave_color,
            );
        }

        // Profile name above bars
        painter.text(
            pos2(s_rect.center().x, s_rect.min.y + 13.0),
            Align2::CENTER_CENTER,
            profile.name(),
            FontId::proportional(12.5),
            if is_sel {
                if theme.is_light() {
                    Color32::from_rgb(
                        (theme.accent.r() as f32 * 0.75) as u8,
                        (theme.accent.g() as f32 * 0.75) as u8,
                        (theme.accent.b() as f32 * 0.75) as u8,
                    )
                } else {
                    theme.accent
                }
            } else {
                theme.text
            },
        );

        if s_hovered && ui.input(|inp| inp.pointer.primary_clicked()) {
            sound.profile = profile;
            on_save_setting("sound", profile.name().to_lowercase().as_str());
        }
    }

    // ── Description card ─────────────────────────────────────────────
    let rows_used = SoundProfile::ALL.len().div_ceil(cols);
    let desc_y = sound_start_y + rows_used as f32 * (s_row_h + row_gap) + 8.0;
    let desc_rect = Rect::from_min_size(
        pos2(p_origin.x, desc_y),
        vec2(available_w, 32.0),
    );
    painter.rect(
        desc_rect,
        5.0,
        theme.surface(),
        Stroke::new(1.0, theme.border()),
        egui::StrokeKind::Inside,
    );
    painter.text(
        pos2(desc_rect.min.x + 12.0, desc_rect.center().y),
        Align2::LEFT_CENTER,
        format!("{}  —  {}", sound.profile.name(), sound.profile.description()),
        FontId::proportional(12.0),
        theme.muted,
    );
}
