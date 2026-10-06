//! Sidebar body component: High-performance documents explorer and notes library.

use crate::sidebar::SidebarAction;
use crate::workspace::{WorkspaceDialog, WorkspaceState};
use core::Note;
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Rect};

use crate::ui::theme::Theme;

/// File type icon mapper for IDE-style explorer
fn get_entry_icon(path: &std::path::Path, is_dir: bool, expanded: bool) -> &'static str {
    if is_dir {
        if expanded {
            "▾ 📂"
        } else {
            "▸ 📁"
        }
    } else {
        match path.extension().and_then(|s| s.to_str()).unwrap_or("") {
            "rs" => "    🦀",
            "md" | "markdown" => "    📝",
            "py" => "    🐍",
            "js" | "jsx" | "ts" | "tsx" => "    ⚡",
            "json" | "toml" | "yaml" | "yml" => "    ⚙",
            "html" | "css" | "scss" => "    🌐",
            "txt" | "log" => "    📄",
            _ => "    📄",
        }
    }
}

/// Renders the inline file or folder creation row in the tree (VS Code style).
fn render_inline_creation(
    ui: &mut egui::Ui,
    workspace: &mut WorkspaceState,
    depth: usize,
    is_dir: bool,
    _theme: &Theme,
    action: &mut Option<SidebarAction>,
) {
    let indent = depth as f32 * 14.0;
    let icon = if is_dir { "▸ 📁" } else { "    📄" };
    ui.horizontal(|ui| {
        ui.add_space(indent + 4.0);
        ui.label(egui::RichText::new(icon).size(13.0));
        let field_w = (ui.available_width() - 8.0).max(60.0);
        let field = ui.add(
            egui::TextEdit::singleline(&mut workspace.dialog_name)
                .desired_width(field_w)
                .margin(egui::Margin::symmetric(4, 2))
                .font(FontId::proportional(12.5))
                .hint_text(if is_dir { "new_folder" } else { "new_file.ext" }),
        );
        if workspace.dialog_focus_requested {
            field.request_focus();
            if field.has_focus() {
                workspace.dialog_focus_requested = false;
            }
        }

        let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter))
            || (field.lost_focus() && ui.input(|i| i.key_down(egui::Key::Enter)));
        let escape_pressed = ui.input(|i| i.key_pressed(egui::Key::Escape));

        if escape_pressed {
            *action = Some(SidebarAction::WorkspaceCancel);
        } else if enter_pressed {
            if !workspace.dialog_name.trim().is_empty() {
                *action = Some(SidebarAction::WorkspaceCommit);
            } else {
                *action = Some(SidebarAction::WorkspaceCancel);
            }
        } else if field.lost_focus() && !workspace.dialog_focus_requested {
            if !workspace.dialog_name.trim().is_empty() {
                *action = Some(SidebarAction::WorkspaceCommit);
            } else {
                *action = Some(SidebarAction::WorkspaceCancel);
            }
        }
    });

    if let Some(err) = workspace.dialog_error.as_deref() {
        ui.horizontal(|ui| {
            ui.add_space(indent + 26.0);
            ui.label(
                egui::RichText::new(format!("⚠ {}", err))
                    .color(Color32::from_rgb(248, 113, 113))
                    .size(11.0),
            );
        });
    }
}

/// Renders the inline rename row replacing the item in the tree (VS Code style).
fn render_inline_rename(
    ui: &mut egui::Ui,
    workspace: &mut WorkspaceState,
    depth: usize,
    is_dir: bool,
    _theme: &Theme,
    action: &mut Option<SidebarAction>,
) {
    let indent = depth as f32 * 14.0;
    let icon = if is_dir { "▸ 📁" } else { "    📄" };
    ui.horizontal(|ui| {
        ui.add_space(indent + 4.0);
        ui.label(egui::RichText::new(icon).size(13.0));
        let field_w = (ui.available_width() - 8.0).max(60.0);
        let field = ui.add(
            egui::TextEdit::singleline(&mut workspace.dialog_name)
                .desired_width(field_w)
                .margin(egui::Margin::symmetric(4, 2))
                .font(FontId::proportional(12.5)),
        );
        if workspace.dialog_focus_requested {
            field.request_focus();
            if field.has_focus() {
                workspace.dialog_focus_requested = false;
            }
        }

        let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter))
            || (field.lost_focus() && ui.input(|i| i.key_down(egui::Key::Enter)));
        let escape_pressed = ui.input(|i| i.key_pressed(egui::Key::Escape));

        if escape_pressed {
            *action = Some(SidebarAction::WorkspaceCancel);
        } else if enter_pressed {
            if !workspace.dialog_name.trim().is_empty() {
                *action = Some(SidebarAction::WorkspaceCommit);
            } else {
                *action = Some(SidebarAction::WorkspaceCancel);
            }
        } else if field.lost_focus() && !workspace.dialog_focus_requested {
            if !workspace.dialog_name.trim().is_empty() {
                *action = Some(SidebarAction::WorkspaceCommit);
            } else {
                *action = Some(SidebarAction::WorkspaceCancel);
            }
        }
    });

    if let Some(err) = workspace.dialog_error.as_deref() {
        ui.horizontal(|ui| {
            ui.add_space(indent + 26.0);
            ui.label(
                egui::RichText::new(format!("⚠ {}", err))
                    .color(Color32::from_rgb(248, 113, 113))
                    .size(11.0),
            );
        });
    }
}

/// Renders a single filesystem tree entry with strict left alignment, clean indentation, and IDE-grade hover/selection.
fn render_tree_entry(
    ui: &mut egui::Ui,
    entry_path: &std::path::Path,
    entry_depth: usize,
    entry_is_directory: bool,
    workspace: &mut WorkspaceState,
    active_file: Option<&std::path::Path>,
    any_modal_open: bool,
    theme: &Theme,
    action: &mut Option<SidebarAction>,
) {
    let indent = entry_depth as f32 * 14.0;
    let selected = workspace.selected_items.contains(entry_path);
    let active = active_file
        .is_some_and(|path| crate::workspace::same_path(path, entry_path));
    let expanded = workspace.expanded.contains(entry_path);
    let icon = get_entry_icon(entry_path, entry_is_directory, expanded);
    let file_name = entry_path.file_name().unwrap_or_default().to_string_lossy();

    let row_w = ui.available_width();
    let row_h = 24.0;
    let (row_rect, response) = ui.allocate_exact_size(
        vec2(row_w, row_h),
        if any_modal_open {
            egui::Sense::hover()
        } else {
            egui::Sense::click_and_drag()
        },
    );
    let hovered = !any_modal_open && response.hovered();

    // 1. Background highlighting
    if active || selected || hovered {
        if active {
            ui.painter().rect_filled(row_rect, 4.0, theme.accent.gamma_multiply(0.18));
            // Thin left-edge accent indicator
            let bar = Rect::from_min_size(
                pos2(row_rect.min.x + 1.0, row_rect.min.y + 3.0),
                vec2(2.5, row_h - 6.0),
            );
            ui.painter().rect_filled(bar, 1.25, theme.accent);
        } else if selected {
            ui.painter().rect_filled(row_rect, 4.0, theme.accent.gamma_multiply(0.10));
        } else if hovered {
            let hover_bg = if theme.is_light() {
                Color32::from_rgba_unmultiplied(0, 0, 0, 10)
            } else {
                Color32::from_rgba_unmultiplied(255, 255, 255, 12)
            };
            ui.painter().rect_filled(row_rect, 4.0, hover_bg);
        }
    }

    // 2. Strict left-aligned text and icon
    let text_x = row_rect.min.x + indent + 6.0;
    let text_clip = Rect::from_min_max(
        pos2(text_x, row_rect.min.y),
        pos2(row_rect.max.x - 4.0, row_rect.max.y),
    );
    let text_painter = ui.painter().with_clip_rect(text_clip);

    let text_color = if active {
        theme.highlight
    } else if selected {
        theme.highlight
    } else if hovered {
        theme.text
    } else {
        theme.text.lerp_to_gamma(theme.muted, 0.20)
    };

    let display_str = format!("{icon}  {file_name}");
    text_painter.text(
        pos2(text_x, row_rect.center().y),
        Align2::LEFT_CENTER,
        display_str,
        FontId::proportional(13.0),
        text_color,
    );

    let response = response.on_hover_text(entry_path.to_string_lossy());

    // Single click: select and open file immediately
    if response.clicked() {
        let (ctrl, shift) = ui.input(|i| {
            (i.modifiers.ctrl || i.modifiers.command, i.modifiers.shift)
        });
        if shift {
            let anchor = workspace
                .selection_anchor
                .as_ref()
                .and_then(|anchor| {
                    workspace.entries.iter().position(|e| &e.path == anchor)
                })
                .unwrap_or_else(|| {
                    workspace
                        .entries
                        .iter()
                        .position(|e| e.path == entry_path)
                        .unwrap_or(0)
                });
            let target = workspace
                .entries
                .iter()
                .position(|e| e.path == entry_path)
                .unwrap_or(anchor);
            if !ctrl {
                workspace.selected_items.clear();
            }
            for item in
                workspace.entries[anchor.min(target)..=anchor.max(target)].iter()
            {
                workspace.selected_items.insert(item.path.clone());
            }
        } else if ctrl {
            if !workspace.selected_items.insert(entry_path.to_path_buf()) {
                workspace.selected_items.remove(entry_path);
            }
            workspace.selection_anchor = Some(entry_path.to_path_buf());
        } else {
            workspace.selected_items.clear();
            workspace.selected_items.insert(entry_path.to_path_buf());
            workspace.selection_anchor = Some(entry_path.to_path_buf());
            if entry_is_directory {
                *action = Some(SidebarAction::ToggleWorkspaceFolder(entry_path.to_path_buf()));
            } else {
                *action = Some(SidebarAction::OpenWorkspaceFile(entry_path.to_path_buf()));
            }
        }
    } else if response.double_clicked() {
        if entry_is_directory {
            *action = Some(SidebarAction::ToggleWorkspaceFolder(entry_path.to_path_buf()));
        } else {
            *action = Some(SidebarAction::OpenWorkspaceFile(entry_path.to_path_buf()));
        }
    }

    // Only set drag payload when actually dragged!
    if response.dragged() {
        let payload = if workspace.selected_items.contains(entry_path) {
            workspace.selected_items.iter().cloned().collect::<Vec<_>>()
        } else {
            vec![entry_path.to_path_buf()]
        };
        response.dnd_set_drag_payload(payload);
    }

    // Drop target on folders
    if entry_is_directory {
        if response
            .dnd_hover_payload::<Vec<std::path::PathBuf>>()
            .is_some()
        {
            ui.painter().rect_stroke(
                response.rect.shrink(1.0),
                3.0,
                egui::Stroke::new(1.5, theme.accent),
                egui::StrokeKind::Inside,
            );
        }
        if let Some(sources) =
            response.dnd_release_payload::<Vec<std::path::PathBuf>>()
        {
            *action = Some(SidebarAction::WorkspaceMove(
                (*sources).clone(),
                entry_path.to_path_buf(),
            ));
        }
    }

    // Context menu (Right-click)
    response.context_menu(|ui| {
        if entry_is_directory {
            if ui
                .button(if workspace.expanded.contains(entry_path) {
                    "Collapse"
                } else {
                    "Expand"
                })
                .clicked()
            {
                *action = Some(SidebarAction::ToggleWorkspaceFolder(entry_path.to_path_buf()));
                ui.close_menu();
            }
            if ui.button("📄 New File").clicked() {
                *action = Some(SidebarAction::WorkspaceCreate(
                    entry_path.to_path_buf(),
                    WorkspaceDialog::CreateFile,
                ));
                ui.close_menu();
            }
            if ui.button("📁 New Folder").clicked() {
                *action = Some(SidebarAction::WorkspaceCreate(
                    entry_path.to_path_buf(),
                    WorkspaceDialog::CreateFolder,
                ));
                ui.close_menu();
            }
            ui.separator();
        } else {
            if ui.button("Open").clicked() {
                *action = Some(SidebarAction::OpenWorkspaceFile(entry_path.to_path_buf()));
                ui.close_menu();
            }
            ui.separator();
        }
        if ui.button("Rename").clicked() {
            *action = Some(SidebarAction::WorkspaceRename(entry_path.to_path_buf()));
            ui.close_menu();
        }
        if ui
            .button(egui::RichText::new("Delete").color(Color32::from_rgb(248, 113, 113)))
            .clicked()
        {
            *action = Some(SidebarAction::WorkspaceDelete(entry_path.to_path_buf()));
            ui.close_menu();
        }
        ui.separator();
        if ui.button("Copy Path").clicked() {
            ui.ctx().copy_text(entry_path.to_string_lossy().into_owned());
            ui.close_menu();
        }
        if ui.button("Reveal in File Manager").clicked() {
            *action = Some(SidebarAction::WorkspaceReveal(entry_path.to_path_buf()));
            ui.close_menu();
        }
    });
}

/// Renders the scrollable documents list, active selection indicators, and note management controls.
pub fn render_sidebar_body(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    sb_rect: Rect,
    sb_origin: egui::Pos2,
    active_note_id: Option<i64>,
    notes: &[Note],
    notes_limit: usize,
    total_notes_count: usize,
    is_dirty: bool,
    theme: &Theme,
    sidebar_selected_idx: usize,
    sidebar_focused: bool,
    sidebar_needs_scroll: bool,
    any_modal_open: bool,
    workspace: &mut WorkspaceState,
    active_file: Option<&std::path::Path>,
) -> Option<SidebarAction> {
    let mut action = None;

    // Start cleanly below the Stats header and divider
    let workspace_y = sb_origin.y + 64.0;

    // Track whether documents section is expanded (defaults to true if no workspace open)
    let docs_expanded_id = egui::Id::new("sidebar_docs_expanded");
    let mut docs_expanded: bool = ui.data_mut(|d| {
        *d.get_temp_mut_or_insert_with(docs_expanded_id, || workspace.root.is_none())
    });

    let tree_max_y = if workspace.root.is_some() && !notes.is_empty() {
        if docs_expanded {
            sb_origin.y + (sb_rect.height() * 0.55).max(180.0)
        } else {
            sb_rect.max.y - 66.0
        }
    } else {
        sb_rect.max.y - 42.0
    };

    // 1. WORKSPACE FILE EXPLORER
    let tree_rect = Rect::from_min_max(
        pos2(sb_origin.x, workspace_y),
        pos2(sb_rect.max.x - 10.0, tree_max_y),
    );

    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(tree_rect), |ui| {
        // Workspace Header (clean project name + action buttons)
        ui.horizontal(|ui| {
            let project_name = if let Some(root) = workspace.root.as_deref() {
                root.file_name()
                    .unwrap_or(root.as_os_str())
                    .to_string_lossy()
                    .to_string()
            } else {
                "NO WORKSPACE".to_string()
            };

            let title_text = egui::RichText::new(project_name.to_uppercase())
                .size(11.0)
                .strong()
                .color(theme.highlight);

            if let Some(root) = workspace.root.as_deref() {
                let root_button = ui.add(
                    egui::Button::new(title_text)
                        .frame(false)
                        .sense(egui::Sense::click_and_drag()),
                );
                if let Some(sources) = root_button.dnd_release_payload::<Vec<std::path::PathBuf>>() {
                    action = Some(SidebarAction::WorkspaceMove(
                        (*sources).clone(),
                        root.to_path_buf(),
                    ));
                }
                if root_button.dnd_hover_payload::<Vec<std::path::PathBuf>>().is_some() {
                    ui.painter().rect_stroke(
                        root_button.rect,
                        3.0,
                        egui::Stroke::new(1.5, theme.accent),
                        egui::StrokeKind::Inside,
                    );
                }
                root_button.on_hover_text(format!("Workspace root:\n{}", root.display()));
            } else {
                ui.label(title_text);
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(egui::Button::new(egui::RichText::new("↗").size(12.0)).frame(false))
                    .on_hover_text("Open workspace folder...")
                    .clicked()
                {
                    action = Some(SidebarAction::OpenWorkspace);
                }
                if workspace.root.is_some() {
                    if ui
                        .add(egui::Button::new(egui::RichText::new("↻").size(12.0)).frame(false))
                        .on_hover_text("Refresh explorer")
                        .clicked()
                    {
                        action = Some(SidebarAction::WorkspaceRefresh);
                    }
                    if ui
                        .add(egui::Button::new(egui::RichText::new("📁+").size(11.5)).frame(false))
                        .on_hover_text("New Folder")
                        .clicked()
                    {
                        if let Some(root) = workspace.root.clone() {
                            action = Some(SidebarAction::WorkspaceCreate(
                                root,
                                WorkspaceDialog::CreateFolder,
                            ));
                        }
                    }
                    if ui
                        .add(egui::Button::new(egui::RichText::new("📄+").size(11.5)).frame(false))
                        .on_hover_text("New File")
                        .clicked()
                    {
                        if let Some(root) = workspace.root.clone() {
                            action = Some(SidebarAction::WorkspaceCreate(
                                root,
                                WorkspaceDialog::CreateFile,
                            ));
                        }
                    }
                }
            });
        });

        ui.add_space(4.0);

        egui::ScrollArea::vertical()
            .id_salt("workspace_tree")
            .show(ui, |ui| {
                if workspace.root.is_none() {
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("Open a folder to browse and edit project files")
                            .color(theme.muted)
                            .size(11.5),
                    );
                    if ui.button("↗ Open Folder").clicked() {
                        action = Some(SidebarAction::OpenWorkspace);
                    }
                    return;
                }

                let is_creating = matches!(
                    workspace.dialog,
                    Some(WorkspaceDialog::CreateFile | WorkspaceDialog::CreateFolder)
                );
                let is_creating_dir = matches!(workspace.dialog, Some(WorkspaceDialog::CreateFolder));
                let creating_at_root = workspace
                    .root
                    .as_ref()
                    .map_or(false, |r| r == &workspace.dialog_parent);
                let mut inline_created_rendered = false;

                // Inline creation row at root level
                if is_creating && creating_at_root {
                    render_inline_creation(ui, workspace, 0, is_creating_dir, theme, &mut action);
                    inline_created_rendered = true;
                }

                let total_entries = workspace.entries.len();
                if total_entries == 0 && !is_creating {
                    ui.label(
                        egui::RichText::new("This folder is empty")
                            .color(theme.muted)
                            .size(12.0),
                    );
                }

                // Iterate through entries without full vector cloning every frame
                for i in 0..total_entries {
                    let (entry_path, entry_depth, entry_directory) = {
                        let e = &workspace.entries[i];
                        (e.path.clone(), e.depth, e.directory)
                    };

                    let is_renaming_this = workspace.dialog == Some(WorkspaceDialog::Rename)
                        && workspace.selected_path.as_ref() == Some(&entry_path);

                    if is_renaming_this {
                        render_inline_rename(
                            ui,
                            workspace,
                            entry_depth,
                            entry_directory,
                            theme,
                            &mut action,
                        );
                    } else {
                        render_tree_entry(
                            ui,
                            &entry_path,
                            entry_depth,
                            entry_directory,
                            workspace,
                            active_file,
                            any_modal_open,
                            theme,
                            &mut action,
                        );
                    }

                    // Inline creation row under this folder
                    if is_creating
                        && !inline_created_rendered
                        && entry_directory
                        && entry_path == workspace.dialog_parent
                    {
                        render_inline_creation(
                            ui,
                            workspace,
                            entry_depth + 1,
                            is_creating_dir,
                            theme,
                            &mut action,
                        );
                        inline_created_rendered = true;
                    }
                }

                // Fallback for inline creation if parent was not encountered
                if is_creating && !inline_created_rendered {
                    render_inline_creation(ui, workspace, 0, is_creating_dir, theme, &mut action);
                }
            });
    });

    // 2. DOCUMENTS (Knowledge notes) - Collapsible or bottom area
    if workspace.root.is_some() && !notes.is_empty() {
        let docs_bar_y = tree_max_y + 6.0;
        let docs_header_rect = Rect::from_min_size(
            pos2(sb_origin.x, docs_bar_y),
            vec2(sb_rect.width() - 24.0, 22.0),
        );

        let toggle_label = if docs_expanded {
            format!("▾ DOCUMENTS ({})", notes.len())
        } else {
            format!("▸ DOCUMENTS ({})", notes.len())
        };

        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(docs_header_rect), |ui| {
            ui.horizontal(|ui| {
                if ui
                    .add(egui::Button::new(
                        egui::RichText::new(toggle_label)
                            .size(11.0)
                            .color(theme.muted),
                    ).frame(false))
                    .clicked()
                {
                    docs_expanded = !docs_expanded;
                    ui.data_mut(|d| d.insert_temp(docs_expanded_id, docs_expanded));
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(egui::Button::new(egui::RichText::new("+").size(13.0).color(theme.accent)).frame(false))
                        .on_hover_text("New Note")
                        .clicked()
                    {
                        action = Some(SidebarAction::NewNote);
                    }
                });
            });
        });

        if docs_expanded {
            let docs_list_rect = Rect::from_min_max(
                pos2(sb_origin.x, docs_bar_y + 24.0),
                pos2(sb_rect.max.x - 12.0, sb_rect.max.y - 48.0),
            );
            render_documents_list(
                ui,
                docs_list_rect,
                sb_rect,
                active_note_id,
                notes,
                notes_limit,
                total_notes_count,
                is_dirty,
                theme,
                sidebar_selected_idx,
                sidebar_focused,
                sidebar_needs_scroll,
                any_modal_open,
                &mut action,
            );
        }
    } else if workspace.root.is_none() {
        let docs_y = workspace_y + 70.0;
        let header_label = if total_notes_count > notes.len() {
            format!("DOCUMENTS ({}/{})", notes.len(), total_notes_count)
        } else {
            format!("DOCUMENTS ({})", notes.len())
        };

        painter.text(
            pos2(sb_origin.x, docs_y),
            Align2::LEFT_TOP,
            header_label,
            FontId::proportional(11.5),
            theme.muted,
        );

        let new_btn = Rect::from_min_size(pos2(sb_rect.max.x - 38.0, docs_y - 2.0), vec2(22.0, 20.0));
        if ui.rect_contains_pointer(new_btn) {
            painter.rect_filled(
                new_btn,
                4.0,
                theme.surface().lerp_to_gamma(theme.accent, 0.15),
            );
            if ui.input(|inp| inp.pointer.primary_clicked()) {
                action = Some(SidebarAction::NewNote);
            }
        }
        painter.text(
            new_btn.center(),
            Align2::CENTER_CENTER,
            "+",
            FontId::proportional(14.0),
            theme.accent,
        );

        let docs_list_rect = Rect::from_min_max(
            pos2(sb_origin.x, docs_y + 24.0),
            pos2(sb_rect.max.x - 12.0, sb_rect.max.y - 48.0),
        );
        render_documents_list(
            ui,
            docs_list_rect,
            sb_rect,
            active_note_id,
            notes,
            notes_limit,
            total_notes_count,
            is_dirty,
            theme,
            sidebar_selected_idx,
            sidebar_focused,
            sidebar_needs_scroll,
            any_modal_open,
            &mut action,
        );
    }

    action
}

fn render_documents_list(
    ui: &mut egui::Ui,
    docs_list_rect: Rect,
    sb_rect: Rect,
    active_note_id: Option<i64>,
    notes: &[Note],
    notes_limit: usize,
    total_notes_count: usize,
    is_dirty: bool,
    theme: &Theme,
    sidebar_selected_idx: usize,
    sidebar_focused: bool,
    sidebar_needs_scroll: bool,
    any_modal_open: bool,
    action: &mut Option<SidebarAction>,
) {
    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(docs_list_rect), |ui| {
        egui::ScrollArea::vertical()
            .id_salt("sidebar_docs_scroll")
            .auto_shrink([false; 2])
            .enable_scrolling(!any_modal_open)
            .show(ui, |ui| {
                for (idx, note) in notes.iter().enumerate() {
                    let is_active = active_note_id == Some(note.id);
                    let is_selected = idx == sidebar_selected_idx;
                    let row_w = ui.available_width();
                    let row_h = 30.0;
                    let sense = if any_modal_open {
                        egui::Sense::hover()
                    } else {
                        egui::Sense::click()
                    };
                    let (rect, resp) = ui.allocate_exact_size(vec2(row_w, row_h), sense);
                    let hovered = !any_modal_open && resp.hovered();

                    if is_selected && sidebar_focused && sidebar_needs_scroll {
                        resp.scroll_to_me(Some(egui::Align::Center));
                    }

                    let del_w = 20.0;
                    let del_rect = Rect::from_center_size(
                        pos2(rect.max.x - 14.0, rect.center().y),
                        vec2(del_w, 20.0),
                    );
                    let del_hover = !any_modal_open
                        && del_rect
                            .contains(ui.input(|i| i.pointer.hover_pos().unwrap_or_default()));

                    let pill_rect = Rect::from_min_max(
                        pos2(rect.min.x + 4.0, rect.min.y + 1.5),
                        pos2(rect.max.x - 4.0, rect.max.y - 1.5),
                    );

                    if is_active || (is_selected && sidebar_focused) || hovered {
                        let is_active_or_selected = is_active || (is_selected && sidebar_focused);
                        let bg_color = if is_active_or_selected {
                            if theme.is_light() {
                                Color32::from_rgba_unmultiplied(
                                    theme.accent.r(),
                                    theme.accent.g(),
                                    theme.accent.b(),
                                    16,
                                )
                            } else {
                                Color32::from_rgba_unmultiplied(
                                    theme.accent.r(),
                                    theme.accent.g(),
                                    theme.accent.b(),
                                    20,
                                )
                            }
                        } else {
                            if theme.is_light() {
                                Color32::from_rgba_unmultiplied(0, 0, 0, 10)
                            } else {
                                Color32::from_rgba_unmultiplied(255, 255, 255, 12)
                            }
                        };
                        ui.painter().rect_filled(pill_rect, 5.0, bg_color);

                        if is_active_or_selected {
                            let bar = Rect::from_min_size(
                                pos2(pill_rect.min.x + 1.5, pill_rect.min.y + 4.0),
                                vec2(2.5, pill_rect.height() - 8.0),
                            );
                            ui.painter().rect_filled(bar, 1.25, theme.accent);
                        }
                    }

                    if !any_modal_open && resp.clicked() && !del_hover {
                        *action = Some(SidebarAction::LoadNote {
                            id: note.id,
                            topic: note.topic.clone(),
                            body: note.body.clone(),
                            index: idx,
                        });
                    }

                    let text_x = pill_rect.min.x + 12.0;
                    let text_right_limit = (rect.max.x - del_w - 6.0).min(sb_rect.max.x - 20.0);
                    let avail_w = (text_right_limit - text_x).max(10.0);

                    let font_id = egui::FontId::proportional(12.5);
                    let mut display_title = note.topic.clone();
                    let full_w = ui.fonts(|f| {
                        f.layout_no_wrap(display_title.clone(), font_id.clone(), Color32::WHITE)
                            .size()
                            .x
                    });
                    if full_w > avail_w {
                        let mut truncated = String::new();
                        for ch in note.topic.chars() {
                            let candidate = format!("{}...", truncated);
                            let w = ui.fonts(|f| {
                                f.layout_no_wrap(candidate.clone(), font_id.clone(), Color32::WHITE)
                                    .size()
                                    .x
                            });
                            if w > avail_w {
                                break;
                            }
                            truncated.push(ch);
                        }
                        display_title = if truncated.is_empty() {
                            "…".to_string()
                        } else {
                            format!("{}…", truncated)
                        };
                    }

                    let title_color = if is_active {
                        theme.accent
                    } else if is_selected && sidebar_focused {
                        theme.highlight
                    } else if hovered {
                        theme.text
                    } else {
                        theme.text.lerp_to_gamma(theme.muted, 0.35)
                    };

                    let row_clip = Rect::from_min_max(
                        pos2(rect.min.x, rect.min.y),
                        pos2(text_right_limit, rect.max.y),
                    )
                    .intersect(sb_rect);
                    let row_painter = ui.painter().with_clip_rect(row_clip);

                    row_painter.text(
                        pos2(text_x, pill_rect.center().y),
                        Align2::LEFT_CENTER,
                        display_title,
                        font_id,
                        title_color,
                    );

                    if is_active && is_dirty {
                        let dot_color = Color32::from_rgba_unmultiplied(
                            theme.accent.r(),
                            theme.accent.g(),
                            theme.accent.b(),
                            180,
                        );
                        let dot_x = if hovered {
                            rect.max.x - del_w - 12.0
                        } else {
                            rect.max.x - 14.0
                        };
                        ui.painter().text(
                            pos2(dot_x, rect.center().y),
                            Align2::CENTER_CENTER,
                            "●",
                            FontId::proportional(11.0),
                            dot_color,
                        );
                    }

                    if hovered && !any_modal_open {
                        if crate::ui_components::render_close_button_rect(
                            ui,
                            ui.painter(),
                            del_rect,
                            theme,
                            ("del_note", note.id),
                        ) {
                            *action = Some(SidebarAction::DeleteNote(note.id));
                        }
                    }
                }

                if notes_limit < 100 && total_notes_count > notes.len() {
                    ui.add_space(6.0);
                    let row_w = ui.available_width();
                    let (btn_rect, btn_resp) = ui.allocate_exact_size(
                        vec2(row_w, 28.0),
                        if any_modal_open {
                            egui::Sense::hover()
                        } else {
                            egui::Sense::click()
                        },
                    );
                    let b_hover = !any_modal_open && btn_resp.hovered();
                    let b_bg = if b_hover {
                        theme.surface().lerp_to_gamma(theme.accent, 0.12)
                    } else {
                        theme.surface()
                    };
                    ui.painter().rect(
                        btn_rect,
                        5.0,
                        b_bg,
                        egui::Stroke::new(1.0_f32, theme.border()),
                        egui::StrokeKind::Inside,
                    );
                    ui.painter().text(
                        btn_rect.center(),
                        Align2::CENTER_CENTER,
                        "▼ See more (100 max)",
                        FontId::proportional(11.5),
                        theme.accent,
                    );
                    if !any_modal_open && btn_resp.clicked() {
                        *action = Some(SidebarAction::ToggleNotesLimit);
                    }
                } else if notes_limit >= 100 {
                    ui.add_space(6.0);
                    let row_w = ui.available_width();
                    let (btn_rect, btn_resp) = ui.allocate_exact_size(
                        vec2(row_w, 28.0),
                        if any_modal_open {
                            egui::Sense::hover()
                        } else {
                            egui::Sense::click()
                        },
                    );
                    let b_hover = !any_modal_open && btn_resp.hovered();
                    let b_bg = if b_hover {
                        theme.surface().lerp_to_gamma(theme.accent, 0.12)
                    } else {
                        theme.surface()
                    };
                    ui.painter().rect(
                        btn_rect,
                        5.0,
                        b_bg,
                        egui::Stroke::new(1.0_f32, theme.border()),
                        egui::StrokeKind::Inside,
                    );
                    ui.painter().text(
                        btn_rect.center(),
                        Align2::CENTER_CENTER,
                        "▲ Show less (50)",
                        FontId::proportional(11.5),
                        theme.muted,
                    );
                    if !any_modal_open && btn_resp.clicked() {
                        *action = Some(SidebarAction::ToggleNotesLimit);
                    }
                }
            });
    });
}
