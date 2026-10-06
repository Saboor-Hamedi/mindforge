//! Sidebar body component: High-performance documents explorer and workspace tree.

use crate::sidebar::SidebarAction;
use crate::workspace::{WorkspaceDialog, WorkspaceState};
use core::Note;
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Rect, Stroke};

use crate::ui::theme::Theme;

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
    let (caret, glyph) = super::icons::get_explorer_icon(entry_path, entry_is_directory, expanded);
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
            ui.painter().rect_filled(row_rect, 3.0, theme.accent.gamma_multiply(0.18));
            // Thin left-edge accent indicator
            let bar = Rect::from_min_size(
                pos2(row_rect.min.x + 1.0, row_rect.min.y + 3.0),
                vec2(2.5, row_h - 6.0),
            );
            ui.painter().rect_filled(bar, 1.25, theme.accent);
        } else if selected {
            ui.painter().rect_filled(row_rect, 3.0, theme.accent.gamma_multiply(0.10));
        } else if hovered {
            let hover_bg = if theme.is_light() {
                Color32::from_rgba_unmultiplied(0, 0, 0, 10)
            } else {
                Color32::from_rgba_unmultiplied(255, 255, 255, 12)
            };
            ui.painter().rect_filled(row_rect, 3.0, hover_bg);
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

    let display_str = if entry_is_directory {
        format!("{caret} {glyph}  {file_name}")
    } else {
        format!("   {glyph}  {file_name}")
    };
    text_painter.text(
        pos2(text_x, row_rect.center().y),
        Align2::LEFT_CENTER,
        display_str,
        FontId::proportional(12.5),
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

    // Only set drag payload when actually dragged with left button!
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
                Stroke::new(1.5, theme.accent),
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

    // Professional Reusable Context Menu (Right-click)
    if response.secondary_clicked() && !any_modal_open {
        let pointer_pos = ui.input(|i| i.pointer.interact_pos().or_else(|| i.pointer.hover_pos()))
            .unwrap_or(row_rect.left_bottom());
        let items = if entry_is_directory {
            crate::ui::menu::folder_menu(entry_path, expanded)
        } else {
            crate::ui::menu::file_menu(entry_path)
        };
        workspace.context_menu = Some(crate::ui::menu::MenuState::new(pointer_pos, items));
    }
}

/// Renders the sidebar body: workspace file tree when a project is open, or clean clickable 'nothing available' message when empty.
pub fn render_sidebar_body(
    ui: &mut egui::Ui,
    _painter: &egui::Painter,
    body_rect: Rect,
    _active_note_id: Option<i64>,
    _notes: &[Note],
    _notes_limit: usize,
    _total_notes_count: usize,
    _is_dirty: bool,
    theme: &Theme,
    _sidebar_selected_idx: usize,
    _sidebar_focused: bool,
    _sidebar_needs_scroll: bool,
    any_modal_open: bool,
    workspace: &mut WorkspaceState,
    active_file: Option<&std::path::Path>,
) -> Option<SidebarAction> {
    let mut action = None;

    // 1. EMPTY STATE: When no project is open, show ONLY the clickable 'nothing available' message with inner padding
    if workspace.root.is_none() {
        ui.allocate_new_ui(
            egui::UiBuilder::new()
                .max_rect(body_rect)
                .layout(egui::Layout::top_down(egui::Align::Center)),
            |ui| {
                ui.add_space(24.0);

                let card_padding = egui::Margin {
                    left: 14,
                    right: 14,
                    top: 10,
                    bottom: 10,
                };

                let card_frame = egui::Frame::NONE
                    .inner_margin(card_padding)
                    .fill(if ui.rect_contains_pointer(body_rect) {
                        theme.surface()
                    } else {
                        Color32::from_rgba_unmultiplied(theme.surface().r(), theme.surface().g(), theme.surface().b(), 80)
                    })
                    .stroke(Stroke::new(1.0, theme.border().gamma_multiply(0.50)));

                let resp = card_frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("📂").size(15.0));
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new("nothing available")
                                .size(12.5)
                                .color(theme.muted),
                        );
                    });
                }).response;

                let btn_interact = ui.interact(resp.rect, ui.id().with("empty_workspace_btn"), egui::Sense::click())
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .on_hover_text("Click to open a workspace folder");

                if btn_interact.clicked() {
                    action = Some(SidebarAction::OpenWorkspace);
                }
            },
        );

        return action;
    }

    // 2. ACTIVE WORKSPACE: Render filesystem tree within body_rect
    ui.allocate_new_ui(
        egui::UiBuilder::new()
            .max_rect(body_rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
        |ui| {
            egui::ScrollArea::vertical()
                .id_salt("workspace_tree_scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
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
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new("Folder is empty")
                                .color(theme.muted)
                                .size(12.0),
                        );
                    }

                    // Iterate through tree entries
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
        },
    );

    // 3. Render Reusable MindForge Context Menu (if open)
    if let Some(mut menu_state) = workspace.context_menu.take() {
        let (menu_action, should_close) = crate::ui::menu::render_menu_container(
            ui.ctx(),
            &mut menu_state,
            theme,
            1.0,
        );

        if let Some(m_act) = menu_action {
            match m_act {
                crate::ui::menu::MenuAction::OpenFile(p) => {
                    action = Some(SidebarAction::OpenWorkspaceFile(p));
                }
                crate::ui::menu::MenuAction::ToggleFolder(p) => {
                    action = Some(SidebarAction::ToggleWorkspaceFolder(p));
                }
                crate::ui::menu::MenuAction::NewFile(p) => {
                    action = Some(SidebarAction::WorkspaceCreate(p, WorkspaceDialog::CreateFile));
                }
                crate::ui::menu::MenuAction::NewFolder(p) => {
                    action = Some(SidebarAction::WorkspaceCreate(p, WorkspaceDialog::CreateFolder));
                }
                crate::ui::menu::MenuAction::Rename(p) => {
                    action = Some(SidebarAction::WorkspaceRename(p));
                }
                crate::ui::menu::MenuAction::Delete(p) => {
                    action = Some(SidebarAction::WorkspaceDelete(p));
                }
                crate::ui::menu::MenuAction::CopyPath(p) => {
                    ui.ctx().copy_text(p.to_string_lossy().into_owned());
                }
                crate::ui::menu::MenuAction::Reveal(p) => {
                    action = Some(SidebarAction::WorkspaceReveal(p));
                }
                crate::ui::menu::MenuAction::OpenWorkspace => {
                    action = Some(SidebarAction::OpenWorkspace);
                }
                crate::ui::menu::MenuAction::RefreshWorkspace => {
                    action = Some(SidebarAction::WorkspaceRefresh);
                }
                crate::ui::menu::MenuAction::CloseWorkspace => {
                    action = Some(SidebarAction::CloseWorkspace);
                }
            }
        }

        if !should_close {
            workspace.context_menu = Some(menu_state);
        }
    }

    action
}
