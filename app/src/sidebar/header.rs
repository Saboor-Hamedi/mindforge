//! Sidebar header component: MindForge branding, Stats toggle, and Workspace controls.

use crate::sidebar::SidebarAction;
use crate::ui::theme::Theme;
use crate::workspace::{WorkspaceDialog, WorkspaceState};
use eframe::egui::{self, pos2, Rect, Stroke, Ui};

/// Renders the top header of the sidebar with exact bounding rect and clean 5px section boundaries.
pub fn render_sidebar_header(
    ui: &mut Ui,
    painter: &egui::Painter,
    header_rect: Rect,
    active_mode_idx: usize,
    theme: &Theme,
    any_modal_open: bool,
    workspace: &mut WorkspaceState,
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

            // Row 2: Workspace Project Name + Action Icons (ONLY shown when a workspace is open)
            if let Some(root) = workspace.root.clone() {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let project_name = root
                        .file_name()
                        .unwrap_or(root.as_os_str())
                        .to_string_lossy()
                        .to_string();

                    let root_btn = ui.add(
                        egui::Button::new(
                            egui::RichText::new(project_name.to_uppercase())
                                .size(11.0)
                                .strong()
                                .color(theme.highlight),
                        )
                        .frame(false)
                        .sense(egui::Sense::click_and_drag()),
                    );

                    if let Some(sources) = root_btn.dnd_release_payload::<Vec<std::path::PathBuf>>()
                    {
                        action = Some(SidebarAction::WorkspaceMove(
                            (*sources).clone(),
                            root.clone(),
                        ));
                    }
                    if root_btn
                        .dnd_hover_payload::<Vec<std::path::PathBuf>>()
                        .is_some()
                    {
                        ui.painter().rect_stroke(
                            root_btn.rect,
                            2.0,
                            Stroke::new(1.5, theme.accent),
                            egui::StrokeKind::Inside,
                        );
                    }
                    if root_btn.secondary_clicked() && !any_modal_open {
                        let pointer_pos = ui
                            .input(|i| i.pointer.interact_pos().or_else(|| i.pointer.hover_pos()))
                            .unwrap_or(root_btn.rect.left_bottom());
                        workspace.context_menu = Some(crate::ui::menu::MenuState::new(
                            pointer_pos,
                            crate::ui::menu::root_menu(&root),
                        ));
                    }
                    root_btn.on_hover_text(format!("Workspace root:\n{}", root.display()));

                    // Right-aligned icons for creating folder and file
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(egui::RichText::new("↗").size(12.0)).frame(false),
                            )
                            .on_hover_text("Open workspace folder...")
                            .clicked()
                        {
                            action = Some(SidebarAction::OpenWorkspace);
                        }
                        if ui
                            .add(
                                egui::Button::new(egui::RichText::new("↻").size(12.0)).frame(false),
                            )
                            .on_hover_text("Refresh explorer")
                            .clicked()
                        {
                            action = Some(SidebarAction::WorkspaceRefresh);
                        }
                        if ui
                            .add(
                                egui::Button::new(egui::RichText::new("📁+").size(11.5))
                                    .frame(false),
                            )
                            .on_hover_text("New Folder")
                            .clicked()
                        {
                            action = Some(SidebarAction::WorkspaceCreate(
                                root.clone(),
                                WorkspaceDialog::CreateFolder,
                            ));
                        }
                        if ui
                            .add(
                                egui::Button::new(egui::RichText::new("📄+").size(11.5))
                                    .frame(false),
                            )
                            .on_hover_text("New File")
                            .clicked()
                        {
                            action = Some(SidebarAction::WorkspaceCreate(
                                root,
                                WorkspaceDialog::CreateFile,
                            ));
                        }
                    });
                });
            }
        },
    );

    // Clean horizontal divider line dividing Header from the 5px gap below
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
