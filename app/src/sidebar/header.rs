//! Sidebar header component: MindForge branding and Stats toggle.

use crate::sidebar::SidebarAction;
use crate::ui::theme::Theme;
use eframe::egui::{self, pos2, Rect, Stroke, Ui};

/// Renders the top header of the sidebar with exact bounding rect and clean section boundary.
pub fn render_sidebar_header(
    ui: &mut Ui,
    painter: &egui::Painter,
    header_rect: Rect,
    active_mode_idx: usize,
    theme: &Theme,
    any_modal_open: bool,
) -> Option<SidebarAction> {
    let mut action = None;

    ui.allocate_new_ui(
        egui::UiBuilder::new()
            .max_rect(header_rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
        |ui| {
            // Row 1: MindForge Branding + Stats toggle button
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("MINDFORGE")
                        .size(13.0)
                        .strong()
                        .color(theme.accent),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let is_stats_active = active_mode_idx == 1;
                    let stats_btn = ui.add(
                        egui::Button::new(egui::RichText::new("📊 Stats").size(11.0).color(
                            if is_stats_active {
                                theme.accent
                            } else {
                                theme.muted
                            },
                        ))
                        .frame(false),
                    );

                    if stats_btn.clicked() && !any_modal_open {
                        action = Some(SidebarAction::SwitchMode(if is_stats_active {
                            0
                        } else {
                            1
                        }));
                    }
                });
            });
        },
    );

    // Clean horizontal divider line dividing Header from the body below
    let sep_y = header_rect.max.y;
    painter.line_segment(
        [
            pos2(header_rect.min.x, sep_y),
            pos2(header_rect.max.x, sep_y),
        ],
        Stroke::new(1.0, theme.border().gamma_multiply(0.35)),
    );

    action
}
