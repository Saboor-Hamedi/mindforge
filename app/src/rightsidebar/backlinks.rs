//! Incoming backlinks inspector for the active document.

use crate::ui::theme::Theme;
use crate::wikilink::BacklinkItem;
use eframe::egui::{self, pos2, vec2, Align2, FontId, Rect, ScrollArea, Stroke, Ui};

pub enum BacklinkAction {
    OpenNote { id: i64, title: String },
}

/// Renders the sleek Backlinks panel.
pub fn render_backlinks_panel(
    ui: &mut Ui,
    rect: Rect,
    backlinks: &[BacklinkItem],
    selected_idx: &mut usize,
    theme: &Theme,
) -> Option<BacklinkAction> {
    if backlinks.is_empty() {
        let painter = ui.painter();
        let center = rect.center();

        // Vector chain link icon
        let icon_rect = Rect::from_center_size(pos2(center.x, center.y - 32.0), vec2(28.0, 28.0));
        let stroke = Stroke::new(
            1.8_f32,
            eframe::egui::Color32::from_rgba_unmultiplied(theme.accent.r(), theme.accent.g(), theme.accent.b(), 130),
        );
        let ic = icon_rect.center();
        painter.line_segment([pos2(ic.x - 5.5, ic.y + 5.5), pos2(ic.x + 5.5, ic.y - 5.5)], stroke);
        painter.circle_stroke(pos2(ic.x - 3.5, ic.y + 3.5), 3.5, stroke);
        painter.circle_stroke(pos2(ic.x + 3.5, ic.y - 3.5), 3.5, stroke);

        // Centered Title
        painter.text(
            pos2(center.x, center.y + 2.0),
            Align2::CENTER_CENTER,
            "No Linked References",
            FontId::proportional(13.0),
            theme.text,
        );

        // Subtext / guidance
        painter.text(
            pos2(center.x, center.y + 24.0),
            Align2::CENTER_CENTER,
            "Notes that link to this document via\n[[title]] will appear here automatically.",
            FontId::proportional(11.0),
            theme.muted,
        );
        return None;
    }

    if *selected_idx >= backlinks.len() {
        *selected_idx = 0;
    }

    let mut action = None;

    // Keyboard navigation: ArrowUp/Down and Ctrl+K/J
    if ui.rect_contains_pointer(rect) {
        let (up, down, enter) = ui.input_mut(|i| {
            let ctrl_k = i.modifiers.ctrl && i.key_pressed(egui::Key::K);
            let ctrl_j = i.modifiers.ctrl && i.key_pressed(egui::Key::J);
            let up = i.key_pressed(egui::Key::ArrowUp) || ctrl_k;
            let down = i.key_pressed(egui::Key::ArrowDown) || ctrl_j;
            let enter = i.key_pressed(egui::Key::Enter);

            if up {
                i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp);
                i.consume_key(egui::Modifiers::CTRL, egui::Key::K);
            }
            if down {
                i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown);
                i.consume_key(egui::Modifiers::CTRL, egui::Key::J);
            }

            (up, down, enter)
        });

        if up && *selected_idx > 0 {
            *selected_idx -= 1;
        }
        if down && *selected_idx + 1 < backlinks.len() {
            *selected_idx += 1;
        }
        if enter {
            if let Some(bl) = backlinks.get(*selected_idx) {
                action = Some(BacklinkAction::OpenNote {
                    id: bl.source_note_id,
                    title: bl.source_note_title.clone(),
                });
            }
        }
    }

    let item_h = 42.0;

    ui.allocate_new_ui(eframe::egui::UiBuilder::new().max_rect(rect), |ui| {
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (idx, bl) in backlinks.iter().enumerate() {
                    let is_selected = idx == *selected_idx;
                    let (item_rect, resp) = ui.allocate_exact_size(vec2(rect.width() - 8.0, item_h), egui::Sense::click());
                    let is_hovered = resp.hovered();

                    if resp.clicked() {
                        *selected_idx = idx;
                        action = Some(BacklinkAction::OpenNote {
                            id: bl.source_note_id,
                            title: bl.source_note_title.clone(),
                        });
                    }

                    let painter = ui.painter();

                    // Pure borderless typography: NO background highlight box!
                    let title_color = if is_selected || is_hovered {
                        theme.accent
                    } else {
                        theme.text
                    };

                    // Left note / folder icon
                    let is_nested = bl.source_note_title.contains('/') || bl.source_note_title.contains('\\');
                    let icon_name = if is_nested { "folder" } else { "doc" };
                    let icon_rect = Rect::from_center_size(pos2(item_rect.min.x + 12.0, item_rect.min.y + 12.0), vec2(12.0, 12.0));
                    crate::ui_components::render_vector_icon(
                        painter,
                        icon_name,
                        icon_rect,
                        if is_selected { theme.accent } else { theme.muted },
                    );

                    // Note title
                    let max_chars = (((item_rect.max.x - item_rect.min.x - 30.0).max(20.0)) / 7.2) as usize;
                    let display_title = if bl.source_note_title.len() > max_chars && max_chars > 3 {
                        format!("{}...", &bl.source_note_title[..max_chars.saturating_sub(3)])
                    } else {
                        bl.source_note_title.clone()
                    };

                    painter.text(
                        pos2(item_rect.min.x + 24.0, item_rect.min.y + 12.0),
                        Align2::LEFT_CENTER,
                        display_title,
                        FontId::monospace(11.0),
                        title_color,
                    );

                    // Line snippet below title
                    let snip_chars = (((item_rect.max.x - item_rect.min.x - 24.0).max(20.0)) / 6.5) as usize;
                    let display_snip = if bl.snippet.len() > snip_chars && snip_chars > 3 {
                        format!("{}...", &bl.snippet[..snip_chars.saturating_sub(3)])
                    } else {
                        bl.snippet.clone()
                    };

                    painter.text(
                        pos2(item_rect.min.x + 24.0, item_rect.min.y + 28.0),
                        Align2::LEFT_CENTER,
                        display_snip,
                        FontId::monospace(9.5),
                        theme.muted,
                    );
                }
            });
    });

    action
}
