//! AI Engine preferences and DeepSeek Pro API settings tab.

use crate::agent::client::{deobfuscate_key, obfuscate_key};
use crate::ui::theme::Theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Pos2, Rect, Stroke};

pub fn render_ai_tab(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    panel_rect: Rect,
    p_origin: Pos2,
    api_key_enc: &mut String,
    model: &mut String,
    theme: &Theme,
    on_save_setting: &mut dyn FnMut(&str, &str),
) {
    painter.text(
        p_origin,
        Align2::LEFT_TOP,
        "AI ASSISTANT",
        FontId::proportional(15.0),
        theme.highlight,
    );
    painter.text(
        p_origin + vec2(0.0, 22.0),
        Align2::LEFT_TOP,
        "Configure API key and reasoning model for your knowledge vault",
        FontId::proportional(12.0),
        theme.muted,
    );

    let content_w = (panel_rect.width() - 56.0).max(320.0);
    let mut cur_y = p_origin.y + 58.0;

    // ── 1. API Key Card ───────────────────────────────────────────────────────
    painter.text(
        pos2(p_origin.x, cur_y),
        Align2::LEFT_TOP,
        "API Key (Encrypted Storage)",
        FontId::proportional(12.5),
        theme.text,
    );
    cur_y += 22.0;

    let key_card = Rect::from_min_size(pos2(p_origin.x, cur_y), vec2(content_w, 42.0));
    painter.rect(
        key_card,
        6.0,
        theme.surface(),
        Stroke::new(1.0_f32, theme.border()),
        egui::StrokeKind::Inside,
    );

    // Persistent show/hide toggle and edit buffer
    let reveal_id = egui::Id::new("deepseek_reveal_key");
    let mut reveal = ui.ctx().data_mut(|d| d.get_temp::<bool>(reveal_id)).unwrap_or(false);

    let mut current_key = deobfuscate_key(api_key_enc);
    let mut key_changed = false;

    let btn_y = key_card.center().y - 11.0;
    let btn_h = 22.0;

    // Show/Hide toggle button
    let show_btn = Rect::from_min_size(pos2(key_card.max.x - 46.0, btn_y), vec2(40.0, btn_h));
    let show_resp = ui.allocate_rect(show_btn, egui::Sense::click());
    let show_hover = show_resp.hovered() || ui.rect_contains_pointer(show_btn);
    if show_hover {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if show_resp.clicked() || (show_hover && ui.input(|i| i.pointer.primary_clicked())) {
        reveal = !reveal;
        ui.ctx().data_mut(|d| d.insert_temp(reveal_id, reveal));
    }
    painter.rect(
        show_btn,
        4.0,
        if show_hover { theme.surface().lerp_to_gamma(theme.accent, 0.15) } else { theme.bg },
        Stroke::new(1.0_f32, if show_hover { theme.accent } else { theme.border() }),
        egui::StrokeKind::Inside,
    );
    painter.text(
        show_btn.center(),
        Align2::CENTER_CENTER,
        if reveal { "Hide" } else { "Show" },
        FontId::proportional(11.0),
        if show_hover { theme.accent } else { theme.muted },
    );

    // One-click Paste button
    let paste_btn = Rect::from_min_size(pos2(show_btn.min.x - 48.0, btn_y), vec2(44.0, btn_h));
    let paste_resp = ui.allocate_rect(paste_btn, egui::Sense::click());
    let paste_hover = paste_resp.hovered() || ui.rect_contains_pointer(paste_btn);
    if paste_hover {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if paste_resp.clicked() || (paste_hover && ui.input(|i| i.pointer.primary_clicked())) {
        if let Some(clip) = crate::input::global::get_win32_clipboard() {
            let trimmed = clip.trim().to_string();
            if !trimmed.is_empty() {
                current_key = trimmed;
                key_changed = true;
            }
        }
    }
    painter.rect(
        paste_btn,
        4.0,
        if paste_hover { theme.surface().lerp_to_gamma(theme.accent, 0.15) } else { theme.bg },
        Stroke::new(1.0_f32, if paste_hover { theme.accent } else { theme.border() }),
        egui::StrokeKind::Inside,
    );
    painter.text(
        paste_btn.center(),
        Align2::CENTER_CENTER,
        "Paste",
        FontId::proportional(11.0),
        if paste_hover { theme.accent } else { theme.muted },
    );

    // Optional Clear button when key is present
    let clear_btn_w = if !current_key.is_empty() { 42.0 } else { 0.0 };
    let clear_btn = Rect::from_min_size(pos2(paste_btn.min.x - clear_btn_w - 4.0, btn_y), vec2(clear_btn_w, btn_h));
    if !current_key.is_empty() {
        let clear_resp = ui.allocate_rect(clear_btn, egui::Sense::click());
        let clear_hover = clear_resp.hovered() || ui.rect_contains_pointer(clear_btn);
        if clear_hover {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if clear_resp.clicked() || (clear_hover && ui.input(|i| i.pointer.primary_clicked())) {
            current_key.clear();
            key_changed = true;
        }
        painter.rect(
            clear_btn,
            4.0,
            if clear_hover { Color32::from_rgba_unmultiplied(220, 60, 60, 24) } else { theme.bg },
            Stroke::new(1.0_f32, if clear_hover { Color32::from_rgb(220, 60, 60) } else { theme.border() }),
            egui::StrokeKind::Inside,
        );
        painter.text(
            clear_btn.center(),
            Align2::CENTER_CENTER,
            "Clear",
            FontId::proportional(11.0),
            if clear_hover { Color32::from_rgb(230, 80, 80) } else { theme.muted },
        );
    }

    // Text Edit inside key_card using native egui password masking
    let input_right = if !current_key.is_empty() { clear_btn.min.x - 8.0 } else { paste_btn.min.x - 8.0 };
    let input_rect = Rect::from_min_max(
        pos2(key_card.min.x + 12.0, key_card.min.y + 8.0),
        pos2(input_right, key_card.max.y - 8.0),
    );

    let resp = ui.put(
        input_rect,
        egui::TextEdit::singleline(&mut current_key)
            .password(!reveal)
            .font(FontId::monospace(12.5))
            .text_color(theme.text)
            .hint_text("Paste API key (sk-...)")
            .frame(false),
    );

    if resp.changed() {
        key_changed = true;
    }

    if key_changed {
        let clean = current_key.trim().to_string();
        *api_key_enc = obfuscate_key(&clean);
        on_save_setting("deepseek_api_key_enc", api_key_enc);
    }

    cur_y += 48.0;

    // Status line below key card
    if current_key.trim().is_empty() {
        painter.text(
            pos2(p_origin.x, cur_y),
            Align2::LEFT_TOP,
            "⚠️ API key required for assistant. Get yours at platform.deepseek.com",
            FontId::proportional(11.0),
            Color32::from_rgb(235, 160, 50),
        );
    } else {
        let clean = current_key.trim();
        let masked = if clean.len() > 8 {
            format!("✓ Key configured ({}...{})", &clean[..4], &clean[clean.len() - 4..])
        } else {
            "✓ Key configured".to_string()
        };
        painter.text(
            pos2(p_origin.x, cur_y),
            Align2::LEFT_TOP,
            masked,
            FontId::monospace(11.0),
            Color32::from_rgb(46, 204, 113),
        );
    }

    cur_y += 26.0;

    // ── 2. Model Selection ────────────────────────────────────────────────────
    painter.text(
        pos2(p_origin.x, cur_y),
        Align2::LEFT_TOP,
        "Default Model",
        FontId::proportional(12.5),
        theme.text,
    );
    cur_y += 22.0;

    let models = [
        ("deepseek-chat", "General & Fast Reasoning"),
        ("deepseek-reasoner", "Deep Chain-of-Thought (Reasoning)"),
    ];

    let opt_w = (content_w - 12.0) * 0.5;
    for (idx, (m_id, m_desc)) in models.iter().enumerate() {
        let opt_rect = Rect::from_min_size(
            pos2(p_origin.x + idx as f32 * (opt_w + 12.0), cur_y),
            vec2(opt_w, 54.0),
        );
        let opt_resp = ui.allocate_rect(opt_rect, egui::Sense::click());
        let is_sel = model == m_id;
        let opt_hover = opt_resp.hovered() || ui.rect_contains_pointer(opt_rect);
        if opt_hover {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        let bg = if opt_hover {
            theme.surface().lerp_to_gamma(theme.accent, 0.05)
        } else {
            theme.surface()
        };

        painter.rect(
            opt_rect,
            6.0,
            bg,
            Stroke::new(1.0_f32, if is_sel { theme.accent } else if opt_hover { theme.border().lerp_to_gamma(theme.accent, 0.4) } else { theme.border() }),
            egui::StrokeKind::Inside,
        );

        painter.text(
            pos2(opt_rect.min.x + 12.0, opt_rect.min.y + 12.0),
            Align2::LEFT_TOP,
            *m_id,
            FontId::monospace(12.5),
            if is_sel { theme.accent } else { theme.text },
        );
        painter.text(
            pos2(opt_rect.min.x + 12.0, opt_rect.min.y + 32.0),
            Align2::LEFT_TOP,
            *m_desc,
            FontId::proportional(10.5),
            theme.muted,
        );

        if opt_resp.clicked() || (opt_hover && ui.input(|i| i.pointer.primary_clicked())) {
            *model = m_id.to_string();
            on_save_setting("deepseek_model", m_id);
        }
    }

    cur_y += 74.0;

    // ── 3. Knowledge Base Capabilities & Security Info ────────────────────────
    let info_rect = Rect::from_min_size(pos2(p_origin.x, cur_y), vec2(content_w, 110.0));
    painter.rect(
        info_rect,
        6.0,
        theme.surface(),
        Stroke::new(1.0_f32, theme.border()),
        egui::StrokeKind::Inside,
    );

    painter.text(
        pos2(info_rect.min.x + 14.0, info_rect.min.y + 14.0),
        Align2::LEFT_TOP,
        "🔒 Local Database Privacy & Security",
        FontId::proportional(12.5),
        theme.highlight,
    );

    let info_text = "• API keys are hashed and encrypted before storing in your local settings.\n\
                     • DeepSeek AI queries your SQLite notes dynamically via prompt context.\n\
                     • Use shortcut Ctrl+Shift+I or the status bar button to summon the agent anytime.\n\
                     • Chat history can be cleared or deleted at any time with zero trace.";
    painter.text(
        pos2(info_rect.min.x + 14.0, info_rect.min.y + 38.0),
        Align2::LEFT_TOP,
        info_text,
        FontId::proportional(11.5),
        theme.muted,
    );
}
