//! Embedded DeepSeek AI Agent pane interface rendered as a tab in the right split-pane.

use super::{AgentState, MessageRole};
use crate::ui::theme::Theme;
use crate::view_editor::preview::MdBlock;
use core::Note;
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Key, Rect, Stroke};

pub fn render_ai_pane(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    rect: Rect,
    state: &mut AgentState,
    notes: &[Note],
    active_note: Option<(&str, &str)>,
    theme: &Theme,
    font_size: f32,
    request_focus: bool,
    _block_scroll: bool,
    opacity: f32,
) {
    // Fill pane background matching workspace surface with translucency
    let base_color = if theme.is_light() {
        theme.bg
    } else {
        theme.surface()
    };
    let bg_color = Color32::from_rgba_unmultiplied(
        base_color.r(),
        base_color.g(),
        base_color.b(),
        (opacity * 255.0) as u8,
    );
    painter.rect_filled(rect, 0.0, bg_color);

    // ── Action Bar (Context, Clear, Delete) ──────────────────────────────────
    let action_bar_h = 32.0;
    let action_bar_rect = Rect::from_min_size(rect.min, vec2(rect.width(), action_bar_h));

    // Context indicator on the left
    let (ctx_label, ctx_color) = if let Some((title, _)) = active_note {
        let clean_title = if title.len() > 28 {
            format!("{}...", &title[..25])
        } else {
            title.to_string()
        };
        (format!("Context: {}", clean_title), theme.accent)
    } else {
        ("Context: All Notes".to_string(), theme.muted)
    };
    painter.text(
        pos2(action_bar_rect.min.x + 12.0, action_bar_rect.center().y),
        Align2::LEFT_CENTER,
        ctx_label,
        FontId::proportional(11.5),
        ctx_color,
    );

    // Action buttons on right: Clear, Delete
    let btn_y = action_bar_rect.center().y - 10.0;
    let btn_h = 20.0;

    // 1. "Delete" button (delete conversation)
    let del_rect = Rect::from_min_size(pos2(action_bar_rect.max.x - 52.0, btn_y), vec2(44.0, btn_h));
    let del_hover = ui.rect_contains_pointer(del_rect);
    if del_hover {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        painter.rect_filled(del_rect, 4.0, Color32::from_rgba_unmultiplied(220, 60, 60, 26));
        if ui.input(|i| i.pointer.primary_clicked()) {
            state.chat_history.clear();
            state.input_text.clear();
            state.error_msg = None;
        }
    }
    painter.text(
        del_rect.center(),
        Align2::CENTER_CENTER,
        "Delete",
        FontId::proportional(11.0),
        if del_hover { Color32::from_rgb(230, 80, 80) } else { theme.muted },
    );

    // 2. "Clear" button
    let clear_rect = Rect::from_min_size(pos2(del_rect.min.x - 6.0 - 42.0, btn_y), vec2(42.0, btn_h));
    let clear_hover = ui.rect_contains_pointer(clear_rect);
    if clear_hover {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        painter.rect_filled(clear_rect, 4.0, theme.surface().lerp_to_gamma(theme.border(), 0.3));
        if ui.input(|i| i.pointer.primary_clicked()) {
            state.clear_chat();
        }
    }
    painter.text(
        clear_rect.center(),
        Align2::CENTER_CENTER,
        "Clear",
        FontId::proportional(11.0),
        if clear_hover { theme.text } else { theme.muted },
    );

    // Action bar divider line (subtle, non-harsh)
    let divider_color = Color32::from_rgba_unmultiplied(theme.muted.r(), theme.muted.g(), theme.muted.b(), 40);
    painter.line_segment(
        [pos2(rect.min.x, action_bar_rect.max.y), pos2(rect.max.x, action_bar_rect.max.y)],
        Stroke::new(1.0_f32, divider_color),
    );

    // ── Calculate Dynamic Scrollable Textarea Height ─────────────────────────
    // Generous base height (68px) with expansion up to 170px for large text
    let newline_count = state.input_text.chars().filter(|&c| c == '\n').count() + 1;
    let char_count = state.input_text.chars().count();
    let approx_wrap = (char_count / 38).max(1);
    let effective_lines = newline_count.max(approx_wrap);
    let dynamic_box_h = ((effective_lines as f32 * 20.0) + 26.0).clamp(60.0, 160.0);
    let hint_h = 16.0;
    let input_area_h = dynamic_box_h + hint_h + 20.0;

    // ── Chat Scroll Area (Balanced Symmetrical Margins) ──────────────────────
    let pad_left = 16.0f32;
    let pad_right = 6.0f32;
    let chat_rect = Rect::from_min_max(
        pos2(rect.min.x + pad_left, action_bar_rect.max.y + 6.0),
        pos2(rect.max.x - pad_right, rect.max.y - input_area_h - 4.0),
    );

    let avail_content_w = (chat_rect.width() - 10.0).max(60.0);

    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(chat_rect), |ui| {
        ui.style_mut().interaction.selectable_labels = true;
        ui.set_clip_rect(chat_rect.intersect(ui.clip_rect()));
        ui.set_max_width(avail_content_w);

        egui::ScrollArea::vertical()
            .id_salt("ai_agent_chat_scroll")
            .auto_shrink([false; 2])
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .show(ui, |ui| {
                ui.style_mut().interaction.selectable_labels = true;
                ui.set_max_width(avail_content_w);
                ui.add_space(4.0);

                if state.chat_history.is_empty() {
                    // Empty state with interactive starter prompts
                    ui.add_space(16.0);
                    ui.vertical_centered(|ui| {
                        ui.label(
                            egui::RichText::new("Ask anything about your notes")
                                .font(FontId::proportional(font_size * 1.05))
                                .color(theme.muted),
                        );
                        ui.add_space(10.0);

                        let starters = [
                            "When did I add my last note?",
                            "What topics are in my database?",
                            "Summarize what I have written so far",
                        ];

                        let btn_w = (avail_content_w - 16.0).max(80.0);
                        for prompt in &starters {
                            let (p_rect, p_resp) = ui.allocate_exact_size(
                                vec2(btn_w, 28.0),
                                egui::Sense::click(),
                            );
                            let h = p_resp.hovered();
                            if h {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            }
                            let bg = if h {
                                Color32::from_rgba_unmultiplied(
                                    theme.muted.r(),
                                    theme.muted.g(),
                                    theme.muted.b(),
                                    20,
                                )
                            } else {
                                Color32::TRANSPARENT
                            };
                            ui.painter().rect(
                                p_rect,
                                5.0,
                                bg,
                                Stroke::NONE,
                                egui::StrokeKind::Inside,
                            );
                            ui.painter().with_clip_rect(p_rect.intersect(ui.clip_rect())).text(
                                p_rect.center(),
                                Align2::CENTER_CENTER,
                                format!("\"{}\"", prompt),
                                FontId::proportional(font_size * 0.90),
                                if h { theme.text } else { theme.muted },
                            );

                            if p_resp.clicked() {
                                state.input_text = prompt.to_string();
                                state.send_message(notes, active_note);
                            }
                        }
                    });
                } else {
                    let mut feedback_action: Option<(usize, Option<bool>)> = None;
                    for (msg_idx, msg) in state.chat_history.iter().enumerate() {
                        let is_user = msg.role == MessageRole::User;

                        if is_user {
                            // User Message: aligned right with avatar on right side (bubble-then-avatar in reading order)
                            let user_max_bubble_w = (avail_content_w - 38.0).max(60.0);

                            ui.horizontal(|ui| {
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                                    // 1. User avatar on far right (~25px diameter circle, no border)
                                    let (avatar_rect, _) = ui.allocate_exact_size(vec2(25.0, 25.0), egui::Sense::hover());
                                    ui.painter().circle_filled(avatar_rect.center(), 12.5, theme.accent);
                                    ui.painter().text(
                                        avatar_rect.center(),
                                        Align2::CENTER_CENTER,
                                        "Y",
                                        FontId::proportional(11.0),
                                        if theme.is_light() { Color32::WHITE } else { theme.bg },
                                    );

                                    ui.add_space(8.0);

                                    // 2. User bubble content (clean right-aligned prompt, no background)
                                    egui::Frame::NONE
                                        .inner_margin(egui::Margin::symmetric(6, 4))
                                        .show(ui, |ui| {
                                            ui.set_max_width(user_max_bubble_w);
                                            for line in msg.content.lines() {
                                                if line.is_empty() {
                                                    ui.add_space(font_size * 0.4);
                                                    continue;
                                                }
                                                let job = crate::view_editor::preview::build_inline_job(
                                                    line,
                                                    font_size,
                                                    theme.text,
                                                    theme,
                                                    user_max_bubble_w - 20.0,
                                                );
                                                ui.add(egui::Label::new(job).selectable(true).wrap_mode(egui::TextWrapMode::Wrap));
                                            }
                                        });
                                });
                            });
                            ui.add_space(14.0);
                        } else {
                            // Assistant Message: aligned left with avatar on left side (avatar-then-bubble in reading order)
                            let resp_time_str = crate::agent::format_time_12h(&msg.timestamp);
                            let asst_max_bubble_w = (avail_content_w - 38.0).max(60.0);

                            ui.horizontal(|ui| {
                                ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                                    // 1. Assistant avatar on left (~25px diameter circle with crisp vector sparkle, no border)
                                    let (avatar_rect, _) = ui.allocate_exact_size(vec2(25.0, 25.0), egui::Sense::hover());
                                    let avatar_bg = if theme.is_light() {
                                        theme.surface().lerp_to_gamma(theme.accent, 0.16)
                                    } else {
                                        Color32::from_rgba_unmultiplied(theme.accent.r(), theme.accent.g(), theme.accent.b(), 36)
                                    };
                                    ui.painter().circle_filled(avatar_rect.center(), 12.5, avatar_bg);
                                    draw_ai_sparkle_icon(ui.painter(), avatar_rect.center(), 5.5, theme.accent);

                                    ui.add_space(8.0);

                                    // 2. Assistant bubble content to the right of avatar (no "Assistant 4:50 PM" header)
                                    ui.vertical(|ui| {
                                        ui.set_max_width(asst_max_bubble_w);

                                        render_chat_markdown(ui, &msg.content, theme, msg_idx, font_size, asst_max_bubble_w);

                                        ui.add_space(5.0);

                                        // Response action footer: Time on left, (Copy, Dislike, Like) on right, shown on hover
                                        let is_liked = msg.feedback == Some(true);
                                        let is_disliked = msg.feedback == Some(false);
                                        let has_feedback = is_liked || is_disliked;

                                        ui.horizontal(|ui| {
                                            ui.set_width(asst_max_bubble_w);

                                            // Time appears under the response on the left
                                            ui.label(
                                                egui::RichText::new(&resp_time_str)
                                                    .font(FontId::monospace(font_size * 0.74))
                                                    .color(Color32::from_rgba_unmultiplied(theme.muted.r(), theme.muted.g(), theme.muted.b(), 140)),
                                            );

                                            // Action buttons on the right, visible on hover or if feedback active
                                            let is_hovered = ui.rect_contains_pointer(ui.max_rect()) || ui.rect_contains_pointer(avatar_rect);
                                            if is_hovered || has_feedback {
                                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                    ui.spacing_mut().item_spacing = vec2(4.0, 0.0);

                                                    // 1. Copy response button (no border!)
                                                    let copy_id = egui::Id::new("ai_copy_msg").with(msg_idx);
                                                    let current_time = ui.input(|i| i.time);
                                                    let last_copied: Option<f64> = ui.data(|d| d.get_temp(copy_id));
                                                    let is_copied = last_copied.map_or(false, |t| current_time - t < 1.8);

                                                    let copy_text = if is_copied { "✓" } else { "Copy" };
                                                    let (btn_rect, btn_resp) = ui.allocate_exact_size(vec2(if is_copied { 24.0 } else { 44.0 }, 18.0), egui::Sense::click());
                                                    let copy_color = if is_copied {
                                                        Color32::from_rgb(60, 200, 110)
                                                    } else if btn_resp.hovered() {
                                                        theme.text
                                                    } else {
                                                        theme.muted
                                                    };
                                                    if btn_resp.hovered() {
                                                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                                    }
                                                    if btn_resp.clicked() {
                                                        crate::input::global::set_win32_clipboard(&msg.content);
                                                        ui.data_mut(|d| d.insert_temp(copy_id, current_time));
                                                        ui.ctx().request_repaint();
                                                    }
                                                    if is_copied {
                                                        let elapsed = current_time - last_copied.unwrap();
                                                        let rem = 1.8 - elapsed;
                                                        if rem > 0.0 {
                                                            ui.ctx().request_repaint_after(std::time::Duration::from_millis((rem * 1000.0) as u64 + 20));
                                                        }
                                                    }

                                                    if btn_resp.hovered() || is_copied {
                                                        let hover_bg = if is_copied {
                                                            Color32::from_rgba_unmultiplied(60, 200, 110, 25)
                                                        } else {
                                                            Color32::from_rgba_unmultiplied(theme.muted.r(), theme.muted.g(), theme.muted.b(), 25)
                                                        };
                                                        ui.painter().rect_filled(btn_rect, 3.0, hover_bg);
                                                    }
                                                    ui.painter().text(
                                                        btn_rect.center(),
                                                        Align2::CENTER_CENTER,
                                                        copy_text,
                                                        FontId::proportional(if is_copied { 12.0 } else { 9.5 }),
                                                        copy_color,
                                                    );

                                                    ui.add_space(4.0);

                                                    // 2. Thumbs down (Dislike)
                                                    let (dislike_rect, dislike_resp) = ui.allocate_exact_size(vec2(20.0, 18.0), egui::Sense::click());
                                                    if dislike_resp.hovered() {
                                                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                                    }
                                                    if dislike_resp.clicked() {
                                                        feedback_action = Some((msg_idx, if is_disliked { None } else { Some(false) }));
                                                    }
                                                    render_thumbs_down_icon(ui.painter(), dislike_rect, is_disliked, dislike_resp.hovered(), theme);

                                                    // 3. Thumbs up (Like)
                                                    let (like_rect, like_resp) = ui.allocate_exact_size(vec2(20.0, 18.0), egui::Sense::click());
                                                    if like_resp.hovered() {
                                                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                                    }
                                                    if like_resp.clicked() {
                                                        feedback_action = Some((msg_idx, if is_liked { None } else { Some(true) }));
                                                    }
                                                    render_thumbs_up_icon(ui.painter(), like_rect, is_liked, like_resp.hovered(), theme);
                                                });
                                            }
                                        });
                                    });
                                });
                            });
                            ui.add_space(16.0);
                        }
                    }

                    if let Some((idx, fb)) = feedback_action {
                        if let Some(m) = state.chat_history.get_mut(idx) {
                            m.feedback = fb;
                        }
                    }

                    if state.is_thinking {
                        ui.ctx().request_repaint_after(std::time::Duration::from_millis(120));
                        ui.horizontal(|ui| {
                            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                let (avatar_rect, _) = ui.allocate_exact_size(vec2(25.0, 25.0), egui::Sense::hover());
                                let avatar_bg = if theme.is_light() {
                                    theme.surface().lerp_to_gamma(theme.accent, 0.16)
                                } else {
                                    Color32::from_rgba_unmultiplied(theme.accent.r(), theme.accent.g(), theme.accent.b(), 36)
                                };
                                ui.painter().circle_filled(avatar_rect.center(), 12.5, avatar_bg);
                                draw_ai_sparkle_icon(ui.painter(), avatar_rect.center(), 5.5, theme.accent);
                                ui.add_space(8.0);
                                let t = ui.input(|i| i.time);
                                let dots = match ((t * 3.5) as u32) % 4 {
                                    1 => ".  ",
                                    2 => ".. ",
                                    3 => "...",
                                    _ => "   ",
                                };
                                ui.label(
                                    egui::RichText::new(format!("Thinking{}", dots))
                                        .font(FontId::monospace(11.5))
                                        .color(theme.accent),
                                );
                            });
                        });
                    }
                }

                if state.scroll_to_bottom {
                    ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                    state.scroll_to_bottom = false;
                }
            });
    });

    // Divider above input bar (subtle)
    let input_top_y = rect.max.y - input_area_h;
    painter.line_segment(
        [pos2(rect.min.x, input_top_y), pos2(rect.max.x, input_top_y)],
        Stroke::new(1.0_f32, divider_color),
    );

    // ── Bottom Input Row (Aligned with balanced margins) ──────────────────────
    let hint_h = 16.0;
    let send_btn_w = 36.0;
    let send_btn_h = 36.0;
    let send_btn_rect = Rect::from_min_size(
        pos2(rect.max.x - send_btn_w - 12.0, rect.max.y - hint_h - 8.0 - send_btn_h),
        vec2(send_btn_w, send_btn_h),
    );

    let input_box_rect = Rect::from_min_max(
        pos2(rect.min.x + pad_left, input_top_y + 8.0),
        pos2(send_btn_rect.min.x - 8.0, rect.max.y - hint_h - 8.0),
    );

    // Custom themed container for TextEdit
    let edit_id = egui::Id::new("deepseek_prompt_input");
    if request_focus {
        ui.memory_mut(|m| m.request_focus(edit_id));
    }
    let is_focused = ui.memory(|m| m.has_focus(edit_id));

    let input_bg = if theme.is_light() {
        theme.bg
    } else {
        theme.surface()
    };
    let border_stroke = if is_focused {
        Stroke::new(1.0_f32, theme.accent)
    } else {
        Stroke::new(1.0_f32, theme.border())
    };

    painter.rect(
        input_box_rect,
        8.0,
        input_bg,
        border_stroke,
        egui::StrokeKind::Inside,
    );

    // Surrender focus when user clicks outside the AI pane
    if ui.input(|i| i.pointer.primary_clicked()) {
        if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
            if !rect.contains(pos) {
                ui.memory_mut(|m| m.surrender_focus(edit_id));
            }
        }
    }

    let inner_rect = input_box_rect.shrink2(vec2(10.0, 8.0));
    let mut edit_resp = None;

    ui.allocate_new_ui(
        egui::UiBuilder::new().max_rect(inner_rect),
        |ui| {
            // Strictly enforce clipping to inner_rect so text and scrollbars never bleed out
            ui.set_clip_rect(inner_rect.intersect(ui.clip_rect()));

            egui::ScrollArea::vertical()
                .id_salt("ai_textarea_internal_scroll")
                .max_height(inner_rect.height())
                .auto_shrink([false, false])
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                .show(ui, |ui| {
                    let edit_w = (inner_rect.width() - 6.0).max(60.0);
                    let resp = ui.add(
                        egui::TextEdit::multiline(&mut state.input_text)
                            .id(edit_id)
                            .font(FontId::proportional(font_size * 0.96))
                            .text_color(theme.text)
                            .hint_text(
                                egui::RichText::new("Ask anything about your notes...")
                                    .font(FontId::proportional(font_size * 0.84))
                                    .color(theme.muted),
                            )
                            .desired_rows(3)
                            .desired_width(edit_w)
                            .frame(false),
                    );
                    edit_resp = Some(resp);
                });
        },
    );

    // Small persistent keyboard shortcut hint UNDER the textarea
    painter.text(
        pos2(input_box_rect.min.x + 2.0, rect.max.y - 4.0),
        Align2::LEFT_BOTTOM,
        "Enter to send  ·  Shift+Enter for newline",
        FontId::proportional(9.2),
        Color32::from_rgba_unmultiplied(theme.muted.r(), theme.muted.g(), theme.muted.b(), 160),
    );

    let has_text_focus = edit_resp.as_ref().map_or(false, |r| r.has_focus());
    state.is_input_focused = has_text_focus || is_focused;

    if request_focus {
        if let Some(ref r) = edit_resp {
            r.request_focus();
        }
    }

    // Send on Enter (without Shift)
    let enter_pressed = (is_focused || has_text_focus) && ui.input(|i| i.key_pressed(Key::Enter) && !i.modifiers.shift);
    let is_send_hover = ui.rect_contains_pointer(send_btn_rect);
    let send_clicked = is_send_hover && ui.input(|i| i.pointer.primary_clicked());

    if (enter_pressed || send_clicked) && !state.is_thinking {
        let clean = state.input_text.trim().to_string();
        if !clean.is_empty() {
            state.input_text = clean;
            state.send_message(notes, active_note);
        } else {
            state.input_text.clear();
        }
        ui.memory_mut(|m| m.request_focus(edit_id));
        if let Some(ref r) = edit_resp {
            r.request_focus();
        }
    }

    // ── Vector Send Button ────────────────────────────────────────────────────
    let has_content = !state.input_text.trim().is_empty();
    if is_send_hover && !state.is_thinking && has_content {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    let (send_bg, send_border, arrow_color) = if state.is_thinking {
        (theme.bg, Stroke::new(1.0_f32, theme.border()), theme.muted)
    } else if has_content {
        let bg = if is_send_hover {
            theme.accent.lerp_to_gamma(Color32::WHITE, 0.15)
        } else {
            theme.accent
        };
        let arrow = if theme.is_light() {
            Color32::WHITE
        } else {
            theme.bg
        };
        (bg, Stroke::NONE, arrow)
    } else {
        let bg = if is_send_hover {
            theme.surface().lerp_to_gamma(theme.border(), 0.35)
        } else {
            input_bg
        };
        let stroke = Stroke::new(1.0_f32, if is_send_hover { theme.accent } else { theme.border() });
        let arrow = if is_send_hover { theme.accent } else { theme.muted };
        (bg, stroke, arrow)
    };

    painter.rect(
        send_btn_rect,
        8.0,
        send_bg,
        send_border,
        egui::StrokeKind::Inside,
    );

    let c = send_btn_rect.center();
    if state.is_thinking {
        let t = ui.input(|i| i.time);
        let pulse_r = 3.5 + (t * 5.0).sin().abs() as f32 * 2.5;
        painter.circle_filled(c, pulse_r, theme.accent);
    } else {
        // Crisp upward vector arrow
        let arrow_stroke = Stroke::new(2.0_f32, arrow_color);
        painter.line_segment([pos2(c.x, c.y + 5.0), pos2(c.x, c.y - 5.0)], arrow_stroke);
        painter.line_segment([pos2(c.x - 4.5, c.y - 0.5), pos2(c.x, c.y - 5.0)], arrow_stroke);
        painter.line_segment([pos2(c.x + 4.5, c.y - 0.5), pos2(c.x, c.y - 5.0)], arrow_stroke);
    }
}

fn render_chat_markdown(ui: &mut egui::Ui, text: &str, theme: &Theme, block_seed: usize, font_size: f32, max_text_w: f32) {
    let blocks = crate::view_editor::preview::parse_markdown(text);

    for (b_idx, block) in blocks.into_iter().enumerate() {
        match block {
            MdBlock::Heading1(text) => {
                ui.add_space(8.0);
                let font = FontId::proportional(font_size * 1.52);
                let r_text = egui::RichText::new(text).font(font).color(theme.text).strong();
                ui.add(egui::Label::new(r_text).selectable(true).wrap_mode(egui::TextWrapMode::Wrap));
                ui.add_space(4.0);
            }
            MdBlock::Heading2(text) => {
                ui.add_space(6.0);
                let font = FontId::proportional(font_size * 1.28);
                let r_text = egui::RichText::new(text).font(font).color(theme.text).strong();
                ui.add(egui::Label::new(r_text).selectable(true).wrap_mode(egui::TextWrapMode::Wrap));
                ui.add_space(3.0);
            }
            MdBlock::Heading3(text) => {
                ui.add_space(4.0);
                let font = FontId::proportional(font_size * 1.12);
                let r_text = egui::RichText::new(text).font(font).color(theme.text).strong();
                ui.add(egui::Label::new(r_text).selectable(true).wrap_mode(egui::TextWrapMode::Wrap));
                ui.add_space(3.0);
            }
            MdBlock::Heading4(text) => {
                ui.add_space(3.0);
                let font = FontId::proportional(font_size * 1.00);
                let r_text = egui::RichText::new(text).font(font).color(theme.text).strong();
                ui.add(egui::Label::new(r_text).selectable(true).wrap_mode(egui::TextWrapMode::Wrap));
                ui.add_space(2.0);
            }
            MdBlock::Paragraph(text) => {
                for sub_line in text.lines() {
                    if sub_line.is_empty() {
                        ui.add_space(font_size * 0.4);
                        continue;
                    }
                    let job = crate::view_editor::preview::build_inline_job(
                        sub_line,
                        font_size,
                        theme.text,
                        theme,
                        max_text_w,
                    );
                    ui.add(egui::Label::new(job).selectable(true).wrap_mode(egui::TextWrapMode::Wrap));
                }
                ui.add_space(5.0);
            }
            MdBlock::Quote { depth: _, text } => {
                let inner_w = (max_text_w - 24.0).max(40.0);
                let job = crate::view_editor::preview::build_inline_job(
                    &text,
                    font_size * 0.96,
                    theme.muted,
                    theme,
                    inner_w,
                );
                let galley = ui.painter().layout_job(job.clone());
                let text_h = galley.size().y;
                let box_h = text_h + 12.0;
                let (q_rect, _) = ui.allocate_exact_size(vec2(max_text_w, box_h), egui::Sense::hover());

                // Left quote vertical accent border
                let bar_rect = Rect::from_min_size(q_rect.min, vec2(3.5, box_h));
                ui.painter().rect_filled(bar_rect, 1.5, theme.accent);

                // Subtle quote background box (no harsh outer border)
                let bg_rect = Rect::from_min_size(pos2(q_rect.min.x + 4.0, q_rect.min.y), vec2(max_text_w - 4.0, box_h));
                ui.painter().rect_filled(
                    bg_rect,
                    4.0,
                    Color32::from_rgba_unmultiplied(theme.accent.r(), theme.accent.g(), theme.accent.b(), 14),
                );
                let label_rect = Rect::from_min_size(pos2(q_rect.min.x + 14.0, q_rect.min.y + 6.0), vec2(inner_w, text_h));
                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(label_rect), |ui| {
                    ui.add(egui::Label::new(job).selectable(true).wrap_mode(egui::TextWrapMode::Wrap));
                });
                ui.add_space(6.0);
            }
            MdBlock::ListItem { bullet, text, checked, indent_level } => {
                let indent_offset = (indent_level as f32) * 16.0;
                let available_w = (max_text_w - indent_offset).max(40.0);

                let job = crate::view_editor::preview::build_inline_job(
                    &text,
                    font_size,
                    theme.text,
                    theme,
                    available_w - 24.0,
                );

                ui.horizontal(|ui| {
                    if indent_offset > 0.0 {
                        ui.add_space(indent_offset);
                    }
                    if let Some(is_checked) = checked {
                        let (cb_rect, _) = ui.allocate_exact_size(vec2(14.0, 14.0), egui::Sense::hover());
                        if is_checked {
                            ui.painter().rect_filled(cb_rect, 3.0, theme.accent);
                            let p1 = pos2(cb_rect.min.x + 3.0, cb_rect.min.y + 7.0);
                            let p2 = pos2(cb_rect.min.x + 5.5, cb_rect.min.y + 9.5);
                            let p3 = pos2(cb_rect.min.x + 10.5, cb_rect.min.y + 4.0);
                            ui.painter().line_segment([p1, p2], Stroke::new(1.6_f32, theme.bg));
                            ui.painter().line_segment([p2, p3], Stroke::new(1.6_f32, theme.bg));
                        } else {
                            ui.painter().rect_filled(
                                cb_rect,
                                3.0,
                                Color32::from_rgba_unmultiplied(theme.muted.r(), theme.muted.g(), theme.muted.b(), 18),
                            );
                            ui.painter().rect_stroke(
                                cb_rect,
                                3.0,
                                Stroke::new(1.4_f32, Color32::from_rgba_unmultiplied(theme.muted.r(), theme.muted.g(), theme.muted.b(), 140)),
                                egui::StrokeKind::Inside,
                            );
                        }
                        ui.add_space(4.0);
                    } else {
                        let bullet_font = FontId::monospace(font_size * 0.92);
                        ui.label(egui::RichText::new(&bullet).font(bullet_font).color(theme.text));
                    }
                    ui.add(egui::Label::new(job).selectable(true).wrap_mode(egui::TextWrapMode::Wrap));
                });
                ui.add_space(2.0);
            }
            MdBlock::CodeBlock { lang, code } => {
                let lines: Vec<&str> = code.lines().collect();
                let line_count = lines.len().max(1);
                let line_h = (font_size * 1.45).round();
                let pad_x = 12.0;
                let pad_top = 28.0; // Header area for language label and copy button
                let avail_w = (max_text_w - pad_x * 2.0).max(10.0);

                let line_galleys: Vec<std::sync::Arc<egui::Galley>> = lines
                    .iter()
                    .map(|line_str| ui.painter().layout_job(crate::view_editor::preview::highlight_code_line(line_str, &lang, font_size, theme)))
                    .collect();
                let max_line_w = line_galleys.iter().map(|g| g.size().x).fold(0.0f32, f32::max);
                let max_scroll_x = (max_line_w - avail_w).max(0.0);
                let needs_h_scroll = max_scroll_x > 0.0;
                let pad_bottom = if needs_h_scroll { 16.0 } else { 10.0 };
                let block_h = pad_top + (line_count as f32 * line_h) + pad_bottom;

                // Exactly sized container that NEVER overflows right
                let (code_rect, _) = ui.allocate_exact_size(vec2(max_text_w, block_h), egui::Sense::hover());

                // Single cohesive code wrapper card matching theme surface and border
                ui.painter().rect(
                    code_rect,
                    5.0,
                    theme.surface(),
                    Stroke::new(1.0_f32, theme.border()),
                    egui::StrokeKind::Inside,
                );

                // Horizontal scroll state & input
                let scroll_id = egui::Id::new("ai_code_block_scroll_x").with(block_seed).with(b_idx);
                let mut scroll_x: f32 = ui.data(|d| d.get_temp(scroll_id).unwrap_or(0.0));
                if ui.rect_contains_pointer(code_rect) && needs_h_scroll {
                    let h_delta = ui.input(|i| {
                        if i.modifiers.shift {
                            if i.smooth_scroll_delta.y.abs() > 0.001 {
                                i.smooth_scroll_delta.y
                            } else {
                                i.raw_scroll_delta.y * 0.5
                            }
                        } else if i.smooth_scroll_delta.x.abs() > 0.001 {
                            i.smooth_scroll_delta.x
                        } else {
                            i.raw_scroll_delta.x * 0.5
                        }
                    });
                    if h_delta != 0.0 {
                        scroll_x = (scroll_x - h_delta).clamp(0.0, max_scroll_x);
                        ui.data_mut(|d| d.insert_temp(scroll_id, scroll_x));
                        ui.ctx().request_repaint();
                    }
                }
                scroll_x = scroll_x.clamp(0.0, max_scroll_x);

                // Top-right copy button logic
                let copy_id = egui::Id::new("ai_code_block_copy").with(block_seed).with(b_idx);
                let current_time = ui.input(|i| i.time);
                let last_copied: Option<f64> = ui.data(|d| d.get_temp(copy_id));
                let is_copied = last_copied.map_or(false, |t| current_time - t < 1.8);

                let btn_w = 58.0;
                let btn_h = 18.0;
                let btn_rect = Rect::from_min_size(
                    pos2(code_rect.max.x - btn_w - 10.0, code_rect.min.y + 5.0),
                    vec2(btn_w, btn_h),
                );
                let is_btn_hovered = ui.rect_contains_pointer(btn_rect);
                if is_btn_hovered {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if is_btn_hovered && ui.input(|i| i.pointer.primary_clicked()) {
                    crate::input::global::set_win32_clipboard(&code);
                    ui.data_mut(|d| d.insert_temp(copy_id, current_time));
                    ui.ctx().request_repaint();
                }
                if is_copied {
                    let elapsed = current_time - last_copied.unwrap();
                    let remaining = 1.8 - elapsed;
                    if remaining > 0.0 {
                        ui.ctx().request_repaint_after(std::time::Duration::from_millis((remaining * 1000.0) as u64 + 20));
                    }
                }

                // Subtle language label on the left (integrated into card)
                let tag = if lang.is_empty() { "CODE" } else { &lang.to_uppercase() };
                ui.painter().text(
                    pos2(code_rect.min.x + pad_x, code_rect.min.y + 14.0),
                    Align2::LEFT_CENTER,
                    tag,
                    FontId::monospace(10.0),
                    theme.text,
                );

                // Render copy button in top-right
                let (btn_bg, btn_text, btn_color) = if is_copied {
                    (
                        Color32::from_rgba_unmultiplied(60, 200, 110, 30),
                        "✓",
                        Color32::from_rgb(60, 200, 110),
                    )
                } else if is_btn_hovered {
                    (
                        Color32::from_rgba_unmultiplied(theme.muted.r(), theme.muted.g(), theme.muted.b(), 45),
                        "Copy",
                        theme.text,
                    )
                } else {
                    (
                        Color32::TRANSPARENT,
                        "Copy",
                        theme.muted,
                    )
                };

                ui.painter().rect(
                    btn_rect,
                    3.0,
                    btn_bg,
                    Stroke::new(1.0_f32, if is_copied { Color32::from_rgb(60, 200, 110) } else if is_btn_hovered { theme.accent } else { theme.border() }),
                    egui::StrokeKind::Inside,
                );
                ui.painter().text(
                    btn_rect.center(),
                    Align2::CENTER_CENTER,
                    btn_text,
                    FontId::proportional(if is_copied { 12.0 } else { 10.5 }),
                    btn_color,
                );

                // Code lines clipped to padding area
                let code_clip_rect = Rect::from_min_max(
                    pos2(code_rect.min.x + pad_x, code_rect.min.y + pad_top),
                    pos2(code_rect.max.x - pad_x, code_rect.max.y - pad_bottom),
                );
                let line_painter = ui.painter().with_clip_rect(code_clip_rect.intersect(ui.clip_rect()));

                let mut line_y = code_rect.min.y + pad_top;
                for galley in line_galleys {
                    line_painter.galley(
                        pos2(code_rect.min.x + pad_x - scroll_x, line_y),
                        galley,
                        theme.text,
                    );
                    line_y += line_h;
                }

                // Horizontal scroll bar when needed
                if needs_h_scroll {
                    let track_y = code_rect.max.y - 7.0;
                    let track_rect = Rect::from_min_max(
                        pos2(code_rect.min.x + pad_x, track_y),
                        pos2(code_rect.max.x - pad_x, track_y + 3.5),
                    );
                    let ratio = scroll_x / max_scroll_x;
                    let thumb_w = (avail_w * (avail_w / max_line_w)).clamp(20.0, avail_w);
                    let thumb_x = code_rect.min.x + pad_x + ratio * (avail_w - thumb_w);
                    let thumb_rect = Rect::from_min_size(pos2(thumb_x, track_y), vec2(thumb_w, 3.5));

                    let hit_rect = Rect::from_min_max(
                        pos2(code_rect.min.x + pad_x, track_y - 3.0),
                        pos2(code_rect.max.x - pad_x, track_y + 6.5),
                    );
                    let is_hit = ui.rect_contains_pointer(hit_rect);
                    if is_hit && ui.input(|i| i.pointer.primary_down()) {
                        if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
                            let r = ((pos.x - (code_rect.min.x + pad_x) - thumb_w * 0.5) / (avail_w - thumb_w).max(1.0)).clamp(0.0, 1.0);
                            scroll_x = r * max_scroll_x;
                            ui.data_mut(|d| d.insert_temp(scroll_id, scroll_x));
                            ui.ctx().request_repaint();
                        }
                    }

                    ui.painter().rect_filled(
                        track_rect,
                        1.75,
                        Color32::from_rgba_unmultiplied(theme.muted.r(), theme.muted.g(), theme.muted.b(), 30),
                    );
                    let thumb_color = if is_hit {
                        Color32::from_rgba_unmultiplied(theme.accent.r(), theme.accent.g(), theme.accent.b(), 180)
                    } else {
                        Color32::from_rgba_unmultiplied(theme.muted.r(), theme.muted.g(), theme.muted.b(), 110)
                    };
                    ui.painter().rect_filled(thumb_rect, 1.75, thumb_color);
                }

                ui.add_space(8.0);
            }
            MdBlock::Table { headers, rows } => {
                ui.add_space(4.0);
                let col_count = headers.len().max(1);
                let cell_pad_x = 8.0f32;
                let cell_pad_y = 6.0f32;

                // Minimum readable column width so text is never crushed into narrow vertical slivers
                let min_col_w = (font_size * 8.0).max(100.0);
                let needed_w = col_count as f32 * min_col_w;
                let table_content_w = needed_w.max(max_text_w);
                let col_w = table_content_w / col_count as f32;
                let max_scroll_x = (table_content_w - max_text_w).max(0.0);
                let needs_h_scroll = max_scroll_x > 0.0;

                // 1. Layout header galleys & compute dynamic header height
                let header_galleys: Vec<std::sync::Arc<egui::Galley>> = headers
                    .iter()
                    .map(|h_text| {
                        let job = crate::view_editor::preview::build_inline_job(
                            h_text,
                            font_size * 0.95,
                            theme.highlight,
                            theme,
                            col_w - cell_pad_x * 2.0,
                        );
                        ui.painter().layout_job(job)
                    })
                    .collect();
                let header_h = header_galleys
                    .iter()
                    .map(|g| g.size().y)
                    .fold(font_size * 1.3, f32::max)
                    + cell_pad_y * 2.0;

                // 2. Layout row galleys & compute dynamic row heights based on multi-line text
                let mut row_galleys_list: Vec<Vec<std::sync::Arc<egui::Galley>>> = Vec::with_capacity(rows.len());
                let mut row_heights: Vec<f32> = Vec::with_capacity(rows.len());

                for row in &rows {
                    let mut row_galleys = Vec::with_capacity(col_count);
                    let mut max_cell_h = font_size * 1.3;
                    for c_idx in 0..col_count {
                        let cell_text = row.get(c_idx).map(|s| s.as_str()).unwrap_or("");
                        let job = crate::view_editor::preview::build_inline_job(
                            cell_text,
                            font_size * 0.92,
                            theme.text,
                            theme,
                            col_w - cell_pad_x * 2.0,
                        );
                        let galley = ui.painter().layout_job(job);
                        if galley.size().y > max_cell_h {
                            max_cell_h = galley.size().y;
                        }
                        row_galleys.push(galley);
                    }
                    row_heights.push(max_cell_h + cell_pad_y * 2.0);
                    row_galleys_list.push(row_galleys);
                }

                let total_rows_h: f32 = row_heights.iter().sum();
                let table_h = header_h + total_rows_h;

                let (table_rect, _) = ui.allocate_exact_size(vec2(max_text_w, table_h), egui::Sense::hover());

                // Horizontal scroll handling (invisible scrollbar, support shift+wheel and mouse drag)
                let scroll_id = egui::Id::new("ai_table_scroll_x").with(block_seed).with(b_idx);
                let mut scroll_x: f32 = ui.data(|d| d.get_temp(scroll_id).unwrap_or(0.0));
                if ui.rect_contains_pointer(table_rect) && needs_h_scroll {
                    let h_delta = ui.input(|i| {
                        if i.modifiers.shift {
                            if i.smooth_scroll_delta.y.abs() > 0.001 {
                                i.smooth_scroll_delta.y
                            } else {
                                i.raw_scroll_delta.y * 0.5
                            }
                        } else if i.smooth_scroll_delta.x.abs() > 0.001 {
                            i.smooth_scroll_delta.x
                        } else {
                            i.raw_scroll_delta.x * 0.5
                        }
                    });
                    if h_delta != 0.0 {
                        scroll_x = (scroll_x - h_delta).clamp(0.0, max_scroll_x);
                        ui.data_mut(|d| d.insert_temp(scroll_id, scroll_x));
                        ui.ctx().request_repaint();
                    }

                    if ui.input(|i| i.pointer.is_decidedly_dragging()) {
                        let drag_x = ui.input(|i| i.pointer.delta().x);
                        if drag_x != 0.0 {
                            scroll_x = (scroll_x - drag_x).clamp(0.0, max_scroll_x);
                            ui.data_mut(|d| d.insert_temp(scroll_id, scroll_x));
                            ui.ctx().request_repaint();
                        }
                    }
                }
                scroll_x = scroll_x.clamp(0.0, max_scroll_x);

                let content_clip = Rect::from_min_size(table_rect.min, vec2(max_text_w, table_h));

                // Table outer rounded container border
                ui.painter().rect_stroke(
                    content_clip,
                    4.0,
                    Stroke::new(1.0_f32, theme.border()),
                    egui::StrokeKind::Inside,
                );

                let t_painter = ui.painter().with_clip_rect(content_clip.intersect(ui.clip_rect()));

                // Header row background (no bottom border line)
                let header_bg_rect = Rect::from_min_size(
                    pos2(table_rect.min.x - scroll_x, table_rect.min.y),
                    vec2(table_content_w, header_h),
                );
                t_painter.rect_filled(
                    header_bg_rect,
                    egui::CornerRadius { nw: 4, ne: 4, sw: 0, se: 0 },
                    Color32::from_rgba_unmultiplied(theme.text.r(), theme.text.g(), theme.text.b(), 14),
                );

                // Header cells (text color)
                for (c_idx, galley) in header_galleys.into_iter().enumerate() {
                    let c_x = table_rect.min.x - scroll_x + c_idx as f32 * col_w + cell_pad_x;
                    t_painter.galley(pos2(c_x, table_rect.min.y + cell_pad_y), galley, theme.text);
                }

                // Data rows
                let mut cur_row_y = table_rect.min.y + header_h;
                for (r_idx, (row_galleys, &r_h)) in row_galleys_list.into_iter().zip(row_heights.iter()).enumerate() {
                    let r_rect = Rect::from_min_size(
                        pos2(table_rect.min.x - scroll_x, cur_row_y),
                        vec2(table_content_w, r_h),
                    );
                    if r_idx % 2 == 1 {
                        t_painter.rect_filled(
                            r_rect,
                            0.0,
                            Color32::from_rgba_unmultiplied(theme.muted.r(), theme.muted.g(), theme.muted.b(), 10),
                        );
                    }
                    if r_idx + 1 < rows.len() {
                        t_painter.line_segment(
                            [r_rect.left_bottom(), r_rect.right_bottom()],
                            Stroke::new(0.8_f32, Color32::from_rgba_unmultiplied(theme.border().r(), theme.border().g(), theme.border().b(), 80)),
                        );
                    }
                    for (c_idx, galley) in row_galleys.into_iter().enumerate() {
                        let c_x = table_rect.min.x - scroll_x + c_idx as f32 * col_w + cell_pad_x;
                        t_painter.galley(pos2(c_x, cur_row_y + cell_pad_y), galley, theme.text);
                    }
                    cur_row_y += r_h;
                }

                ui.add_space(8.0);
            }
            MdBlock::Rule => {
                ui.add_space(8.0);
            }
        }
    }
}

fn render_thumbs_up_icon(painter: &egui::Painter, rect: Rect, active: bool, hovered: bool, theme: &Theme) {
    let bg = if active {
        Color32::from_rgba_unmultiplied(theme.accent.r(), theme.accent.g(), theme.accent.b(), 45)
    } else if hovered {
        Color32::from_rgba_unmultiplied(theme.muted.r(), theme.muted.g(), theme.muted.b(), 30)
    } else {
        Color32::TRANSPARENT
    };
    painter.rect_filled(rect, 3.0, bg);

    let color = if active {
        theme.accent
    } else if hovered {
        theme.text
    } else {
        Color32::from_rgba_unmultiplied(theme.muted.r(), theme.muted.g(), theme.muted.b(), 160)
    };
    let stroke = Stroke::new(1.3_f32, color);

    let c = rect.center();
    // Cuff/wrist box on left
    let cuff = Rect::from_min_max(pos2(c.x - 6.0, c.y - 1.0), pos2(c.x - 3.5, c.y + 5.5));
    painter.rect_stroke(cuff, 1.0, stroke, egui::StrokeKind::Inside);
    // Hand contour & thumb pointing up
    let p_bottom_start = pos2(c.x - 3.5, c.y + 5.5);
    let p_bottom_end = pos2(c.x + 3.5, c.y + 5.5);
    let p_knuckle = pos2(c.x + 5.0, c.y + 2.0);
    let p_hand_top = pos2(c.x + 5.0, c.y - 1.0);
    let p_thumb_joint = pos2(c.x + 1.0, c.y - 3.5);
    let p_thumb_tip = pos2(c.x - 2.0, c.y - 5.5);
    let p_thumb_base = pos2(c.x - 2.0, c.y - 1.0);

    painter.line_segment([p_bottom_start, p_bottom_end], stroke);
    painter.line_segment([p_bottom_end, p_knuckle], stroke);
    painter.line_segment([p_knuckle, p_hand_top], stroke);
    painter.line_segment([p_hand_top, p_thumb_joint], stroke);
    painter.line_segment([p_thumb_joint, p_thumb_tip], stroke);
    painter.line_segment([p_thumb_tip, p_thumb_base], stroke);
}

fn render_thumbs_down_icon(painter: &egui::Painter, rect: Rect, active: bool, hovered: bool, theme: &Theme) {
    let bg = if active {
        Color32::from_rgba_unmultiplied(theme.accent.r(), theme.accent.g(), theme.accent.b(), 45)
    } else if hovered {
        Color32::from_rgba_unmultiplied(theme.muted.r(), theme.muted.g(), theme.muted.b(), 30)
    } else {
        Color32::TRANSPARENT
    };
    painter.rect_filled(rect, 3.0, bg);

    let color = if active {
        theme.accent
    } else if hovered {
        theme.text
    } else {
        Color32::from_rgba_unmultiplied(theme.muted.r(), theme.muted.g(), theme.muted.b(), 160)
    };
    let stroke = Stroke::new(1.3_f32, color);

    let c = rect.center();
    // Cuff/wrist box on left
    let cuff = Rect::from_min_max(pos2(c.x - 6.0, c.y - 5.5), pos2(c.x - 3.5, c.y + 1.0));
    painter.rect_stroke(cuff, 1.0, stroke, egui::StrokeKind::Inside);
    // Hand contour & thumb pointing down
    let p_top_start = pos2(c.x - 3.5, c.y - 5.5);
    let p_top_end = pos2(c.x + 3.5, c.y - 5.5);
    let p_knuckle = pos2(c.x + 5.0, c.y - 2.0);
    let p_hand_bottom = pos2(c.x + 5.0, c.y + 1.0);
    let p_thumb_joint = pos2(c.x + 1.0, c.y + 3.5);
    let p_thumb_tip = pos2(c.x - 2.0, c.y + 5.5);
    let p_thumb_base = pos2(c.x - 2.0, c.y + 1.0);

    painter.line_segment([p_top_start, p_top_end], stroke);
    painter.line_segment([p_top_end, p_knuckle], stroke);
    painter.line_segment([p_knuckle, p_hand_bottom], stroke);
    painter.line_segment([p_hand_bottom, p_thumb_joint], stroke);
    painter.line_segment([p_thumb_joint, p_thumb_tip], stroke);
    painter.line_segment([p_thumb_tip, p_thumb_base], stroke);
}

fn draw_ai_sparkle_icon(painter: &egui::Painter, center: egui::Pos2, r: f32, color: Color32) {
    let ir = r * 0.28;
    let points = vec![
        pos2(center.x, center.y - r),
        pos2(center.x + ir, center.y - ir),
        pos2(center.x + r, center.y),
        pos2(center.x + ir, center.y + ir),
        pos2(center.x, center.y + r),
        pos2(center.x - ir, center.y + ir),
        pos2(center.x - r, center.y),
        pos2(center.x - ir, center.y - ir),
    ];
    painter.add(egui::epaint::PathShape::convex_polygon(points, color, Stroke::NONE));
}
