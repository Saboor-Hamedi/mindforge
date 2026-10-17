//! Language detection dropdown popup for LunaLine.
//!
//! Provides a search-filtered dropdown allowing the user to override
//! file syntax or switch back to automatic extension/content detection.

use crate::language::{FileLanguage, LanguageSelection, LanguageSelectorState};
use crate::ui::theme::Theme;
use eframe::egui::{self, pos2, vec2, Align2, FontId, Rect, Stroke};

/// Renders the floating language selector dropdown card above LunaLine.
pub fn render_language_detection_dropdown(
    ui: &egui::Ui,
    selector: &mut LanguageSelectorState,
    language_rect: Rect,
    language_override: Option<FileLanguage>,
    detected_language: FileLanguage,
    theme: &Theme,
) -> Option<LanguageSelection> {
    if !selector.open {
        return None;
    }

    let mut result_selection = None;
    let mut choices = vec![(None, "Auto Detect")];
    choices.extend(
        FileLanguage::ALL
            .into_iter()
            .map(|language| (Some(language), language.label())),
    );

    let query = selector.query.trim().to_ascii_lowercase();
    let filtered: Vec<_> = choices
        .into_iter()
        .filter(|(_, label)| query.is_empty() || label.to_ascii_lowercase().contains(&query))
        .collect();

    selector.selected_index = selector
        .selected_index
        .min(filtered.len().saturating_sub(1));

    let is_navigating_arrows = ui.input(|i| {
        i.key_pressed(egui::Key::ArrowDown) || i.key_pressed(egui::Key::ArrowUp)
    });

    if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
        selector.open = false;
        return None;
    } else if !filtered.is_empty() {
        if ui.input(|input| input.key_pressed(egui::Key::ArrowDown)) {
            selector.selected_index = (selector.selected_index + 1).min(filtered.len() - 1);
        }
        if ui.input(|input| input.key_pressed(egui::Key::ArrowUp)) {
            selector.selected_index = selector.selected_index.saturating_sub(1);
        }
        if ui.input(|input| input.key_pressed(egui::Key::Enter)) {
            let (language, _) = filtered[selector.selected_index];
            result_selection = Some(match language {
                Some(language) => LanguageSelection::Language(language),
                None => LanguageSelection::AutoDetect,
            });
            selector.open = false;
            return result_selection;
        }
    }

    let popup_width = 260.0;
    let popup_height = 320.0;
    let screen = ui.ctx().screen_rect();
    let max_x = (screen.max.x - popup_width - 8.0).max(screen.min.x + 8.0);
    let popup_pos = pos2(
        language_rect.min.x.clamp(screen.min.x + 8.0, max_x),
        (language_rect.min.y - popup_height - 6.0).max(screen.min.y + 8.0),
    );
    let popup_rect = Rect::from_min_size(popup_pos, vec2(popup_width, popup_height));

    // Outside click dismiss
    if ui.input(|i| i.pointer.primary_clicked() || i.pointer.secondary_clicked()) {
        if let Some(pos) = ui.input(|i| i.pointer.interact_pos().or_else(|| i.pointer.hover_pos())) {
            if !popup_rect.contains(pos) && !language_rect.contains(pos) {
                selector.open = false;
                return None;
            }
        }
    }

    egui::Area::new(egui::Id::new("mindforge-language-selector"))
        .order(egui::Order::Foreground)
        .fixed_pos(popup_pos)
        .show(ui.ctx(), |ui| {
            // Frame: surface background and crisp theme border
            let painter = ui.painter();
            painter.rect(
                popup_rect,
                4.0,
                theme.surface(),
                Stroke::new(1.0, theme.border()),
                egui::StrokeKind::Inside,
            );

            ui.allocate_new_ui(
                egui::UiBuilder::new()
                    .max_rect(popup_rect.shrink(1.0))
                    .layout(egui::Layout::top_down(egui::Align::Min)),
                |ui| {
                    ui.add_space(5.0);

                    // Symmetrical 5px gap on left AND right
                    let search_bar_width = popup_width - 10.0;
                    ui.horizontal(|ui| {
                        ui.add_space(5.0);
                        egui::Frame::NONE
                            .fill(theme.bg)
                            .stroke(Stroke::new(1.0, theme.border()))
                            .corner_radius(4.0)
                            .inner_margin(egui::Margin::symmetric(8, 5))
                            .show(ui, |ui| {
                                let inner_width = (search_bar_width - 18.0).max(40.0);
                                let search = ui.add(
                                    egui::TextEdit::singleline(&mut selector.query)
                                        .id_salt("mindforge-language-search")
                                        .hint_text(
                                            egui::RichText::new("Select language mode...")
                                                .size(12.0)
                                                .color(theme.muted),
                                        )
                                        .desired_width(inner_width)
                                        .font(FontId::proportional(12.0))
                                        .text_color(theme.text)
                                        .frame(false),
                                );
                                if selector.focus_search {
                                    search.request_focus();
                                    selector.focus_search = false;
                                }
                            });
                    });

                    ui.add_space(5.0);
                    let sep_y = ui.cursor().min.y;
                    ui.painter().line_segment(
                        [pos2(popup_rect.min.x, sep_y), pos2(popup_rect.max.x, sep_y)],
                        Stroke::new(1.0, theme.border().gamma_multiply(0.4)),
                    );
                    ui.add_space(3.0);

                    egui::ScrollArea::vertical()
                        .id_salt("mindforge-language-selector-scroll")
                        .max_height(popup_height - 52.0)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for (index, (language, label)) in filtered.iter().enumerate() {
                                let is_selected = match language {
                                    Some(language) => language_override == Some(*language),
                                    None => language_override.is_none(),
                                };
                                let is_highlighted = index == selector.selected_index;
                                let row_w = popup_width - 2.0;
                                let row_h = 25.0;
                                let (row_rect, resp) = ui.allocate_exact_size(
                                    vec2(row_w, row_h),
                                    egui::Sense::click(),
                                );

                                // ONLY scroll to element when actively navigating with arrow keys!
                                // Crucial fix: never call scroll_to_me during free mouse scrolling!
                                if is_navigating_arrows && is_highlighted {
                                    resp.scroll_to_me(Some(egui::Align::Center));
                                }

                                if is_highlighted || resp.hovered() {
                                    ui.painter().rect_filled(
                                        row_rect,
                                        2.0,
                                        theme.border().gamma_multiply(0.25),
                                    );
                                }

                                let text_x = row_rect.min.x + 10.0;
                                let text_y = row_rect.center().y;

                                // Checkmark indicator for currently active language
                                if is_selected {
                                    ui.painter().text(
                                        pos2(text_x, text_y),
                                        Align2::LEFT_CENTER,
                                        "✓",
                                        FontId::proportional(12.0),
                                        theme.accent,
                                    );
                                }

                                let label_x = text_x + 16.0;
                                let display_name = if language.is_none() {
                                    format!("{label} ({})", detected_language.label())
                                } else {
                                    (*label).to_string()
                                };

                                ui.painter().text(
                                    pos2(label_x, text_y),
                                    Align2::LEFT_CENTER,
                                    &display_name,
                                    FontId::proportional(12.5),
                                    if is_selected {
                                        theme.highlight
                                    } else if is_highlighted || resp.hovered() {
                                        theme.text
                                    } else {
                                        theme.muted
                                    },
                                );

                                if resp.clicked() {
                                    result_selection = Some(match language {
                                        Some(lang) => LanguageSelection::Language(*lang),
                                        None => LanguageSelection::AutoDetect,
                                    });
                                    selector.open = false;
                                }
                            }
                        });
                },
            );
        });

    result_selection
}
