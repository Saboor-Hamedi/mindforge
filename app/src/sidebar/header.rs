//! Sidebar header component: Search input on top, and aligned Stats + file/folder action buttons below.

use crate::sidebar::SidebarAction;
use crate::ui::theme::Theme;
use crate::workspace::{WorkspaceDialog, WorkspaceState};
use eframe::egui::{self, pos2, vec2, Color32, FontId, Rect, Stroke, Ui};

/// Renders the top header of the sidebar with search input, flex-aligned stats & action buttons.
pub fn render_sidebar_header(
    ui: &mut Ui,
    painter: &egui::Painter,
    header_rect: Rect,
    active_mode_idx: usize,
    theme: &Theme,
    any_modal_open: bool,
    filter_query: &mut String,
    focus_search: &mut bool,
    workspace: &mut WorkspaceState,
) -> Option<SidebarAction> {
    let mut action = None;

    ui.allocate_new_ui(
        egui::UiBuilder::new()
            .max_rect(header_rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
        |ui| {
            // Row 1: Search input (borderless, sleek, VS Code style)
            let search_row_h = 26.0;
            let (search_rect, _) = ui.allocate_exact_size(
                vec2(ui.available_width(), search_row_h),
                egui::Sense::hover(),
            );

            // Subtle dark background for input container
            let input_bg = if theme.is_light() {
                Color32::from_rgba_unmultiplied(0, 0, 0, 12)
            } else {
                Color32::from_rgba_unmultiplied(255, 255, 255, 10)
            };
            ui.painter().rect_filled(search_rect, 4.0, input_bg);

            ui.allocate_new_ui(
                egui::UiBuilder::new()
                    .max_rect(search_rect.shrink2(vec2(6.0, 2.0)))
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
                |ui| {
                    ui.label(egui::RichText::new("🔍").size(11.0).color(theme.muted));

                    let clear_btn_w = if !filter_query.is_empty() { 18.0 } else { 0.0 };
                    let available_input_w = (ui.available_width() - clear_btn_w - 4.0).max(40.0);

                    let edit = egui::TextEdit::singleline(filter_query)
                        .desired_width(available_input_w)
                        .margin(egui::Margin::symmetric(2, 2))
                        .font(FontId::proportional(12.0))
                        .text_color(theme.text)
                        .frame(false)
                        .hint_text(
                            egui::RichText::new("Search files (Ctrl+F)...")
                                .color(theme.muted)
                                .size(11.5),
                        );

                    let edit_resp = ui.add(edit);

                    if *focus_search {
                        edit_resp.request_focus();
                        *focus_search = false;
                    }

                    if edit_resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        if !filter_query.is_empty() {
                            filter_query.clear();
                        } else {
                            edit_resp.surrender_focus();
                        }
                    }

                    if !filter_query.is_empty() {
                        let clear_btn = ui.add(
                            egui::Button::new(
                                egui::RichText::new("✕").size(10.5).color(theme.muted),
                            )
                            .frame(false),
                        );
                        if clear_btn.clicked() {
                            filter_query.clear();
                        }
                    }
                },
            );

            ui.add_space(5.0);

            // Row 2: Flex row with Stats on the LEFT, and action buttons (↗, ↻, 📁+, 📄+) on the RIGHT
            ui.horizontal(|ui| {
                // LEFT: 📊 Stats toggle
                let is_stats_active = active_mode_idx == 1;
                let stats_btn = ui.add(
                    egui::Button::new(egui::RichText::new("📊 Stats").size(11.5).color(
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

                // RIGHT: Flex action buttons for workspace files and folders
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing = vec2(3.0, 0.0);

                    let root_opt = workspace.root.clone();

                    // Open external workspace folder
                    if ui
                        .add(egui::Button::new(egui::RichText::new("↗").size(12.0)).frame(false))
                        .on_hover_text("Open folder...")
                        .clicked()
                        && !any_modal_open
                    {
                        action = Some(SidebarAction::OpenWorkspace);
                    }

                    // Refresh workspace
                    if ui
                        .add(egui::Button::new(egui::RichText::new("↻").size(12.0)).frame(false))
                        .on_hover_text("Refresh explorer")
                        .clicked()
                        && !any_modal_open
                    {
                        action = Some(SidebarAction::WorkspaceRefresh);
                    }

                    // New Folder button
                    if ui
                        .add(egui::Button::new(egui::RichText::new("📁+").size(12.0)).frame(false))
                        .on_hover_text("New Folder")
                        .clicked()
                        && !any_modal_open
                    {
                        let default_dir = root_opt
                            .clone()
                            .unwrap_or_else(crate::workspace::default_workspace_dir);
                        let parent = if let Some(selected) = &workspace.selected_path {
                            if selected.is_dir() {
                                selected.clone()
                            } else {
                                selected
                                    .parent()
                                    .map(std::path::Path::to_path_buf)
                                    .unwrap_or(default_dir)
                            }
                        } else {
                            default_dir
                        };
                        action = Some(SidebarAction::WorkspaceCreate(
                            parent,
                            WorkspaceDialog::CreateFolder,
                        ));
                    }

                    // New File button
                    if ui
                        .add(egui::Button::new(egui::RichText::new("📄+").size(12.0)).frame(false))
                        .on_hover_text("New File")
                        .clicked()
                        && !any_modal_open
                    {
                        let default_dir =
                            root_opt.unwrap_or_else(crate::workspace::default_workspace_dir);
                        let parent = if let Some(selected) = &workspace.selected_path {
                            if selected.is_dir() {
                                selected.clone()
                            } else {
                                selected
                                    .parent()
                                    .map(std::path::Path::to_path_buf)
                                    .unwrap_or_else(|| default_dir.clone())
                            }
                        } else {
                            default_dir
                        };
                        action = Some(SidebarAction::WorkspaceCreate(
                            parent,
                            WorkspaceDialog::CreateFile,
                        ));
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
