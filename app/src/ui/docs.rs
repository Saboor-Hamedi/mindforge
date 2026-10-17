//! Built-in Brain Documentation & Interactive Tutorials.
//!
//! Embeds user-friendly MindForge guides as `include_str!` constants for
//! offline, instant access in both development and production release
//! packages. No network requests needed — all docs ship with the binary.
//!
//! The documentation viewer renders Markdown content through the same
//! preview engine used for note live-preview, ensuring consistent styling.

use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Rect, Stroke};
use std::sync::OnceLock;

/// A single documentation page with embedded Markdown content.
pub struct DocItem {
    pub id: &'static str,
    pub title: &'static str,
    pub filename: &'static str,
    pub content: &'static str,
}

pub const BRAIN_DOCS: &[DocItem] = &[
    DocItem {
        id: "introduction",
        title: "Introduction",
        filename: "introduction.txt",
        content: include_str!("../../../brain/introduction.txt"),
    },
    DocItem {
        id: "architecture",
        title: "Architecture",
        filename: "architecture.txt",
        content: include_str!("../../../brain/architecture.txt"),
    },
    DocItem {
        id: "notes",
        title: "Notes & Checklists",
        filename: "notes.txt",
        content: include_str!("../../../brain/notes.txt"),
    },
    DocItem {
        id: "shortcuts",
        title: "Shortcuts",
        filename: "shortcuts.txt",
        content: include_str!("../../../brain/shortcuts.txt"),
    },
    DocItem {
        id: "vim",
        title: "Vim Editor",
        filename: "vim.txt",
        content: include_str!("../../../brain/vim.txt"),
    },
    DocItem {
        id: "motions",
        title: "Vim Motions & Objects",
        filename: "motions.txt",
        content: include_str!("../../../brain/motions.txt"),
    },
    DocItem {
        id: "neovim",
        title: "Native Neovim RPC & Lua",
        filename: "neovim.txt",
        content: include_str!("../../../brain/neovim.txt"),
    },
    DocItem {
        id: "lsp",
        title: "Language Servers & Completion",
        filename: "lsp.txt",
        content: include_str!("../../../brain/lsp.txt"),
    },
    DocItem {
        id: "customization",
        title: "Customization & AI",
        filename: "customization.txt",
        content: include_str!("../../../brain/customization.txt"),
    },
    DocItem {
        id: "terminal",
        title: "Embedded Terminal",
        filename: "terminal.txt",
        content: include_str!("../../../brain/terminal.txt"),
    },
    DocItem {
        id: "security",
        title: "Security Scanner",
        filename: "security.txt",
        content: include_str!("../../../brain/security.txt"),
    },
];

/// Returns all available documentation documents.
/// If a local `brain/` folder exists on disk, reads the latest version from disk;
/// otherwise, falls back to the embedded compile-time copy.
pub fn get_docs() -> &'static [DocItem] {
    static DOCS: OnceLock<Vec<DocItem>> = OnceLock::new();
    DOCS.get_or_init(|| {
        BRAIN_DOCS
            .iter()
            .map(|doc| {
                let disk_paths = [
                    format!("brain/{}", doc.filename),
                    format!("../brain/{}", doc.filename),
                ];
                let disk_content = disk_paths
                    .iter()
                    .find_map(|p| std::fs::read_to_string(p).ok());

                if let Some(content) = disk_content {
                    // Disk overrides are loaded once and kept for the app lifetime.
                    let content = Box::leak(content.into_boxed_str());
                    DocItem {
                        id: doc.id,
                        title: doc.title,
                        filename: doc.filename,
                        content,
                    }
                } else {
                    DocItem {
                        id: doc.id,
                        title: doc.title,
                        filename: doc.filename,
                        content: doc.content,
                    }
                }
            })
            .collect()
    })
}

/// Formats a raw markdown string into a clean, beautifully formatted reader text.
/// Strips raw symbols (#, ##, ###, ---, *, `, etc.) and replaces them with
/// elegant typography, section dividers, and bullet points so it wraps perfectly like the editor.
pub fn format_doc_for_reader(raw_md: &str) -> String {
    let mut out = String::new();
    let clean = raw_md.replace("\r\n", "\n").replace('\r', "\n");
    let mut in_code_block = false;

    for line in clean.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            if in_code_block {
                out.push_str("┌─────────────────────────────────────────────────────────────\n");
            } else {
                out.push_str("└─────────────────────────────────────────────────────────────\n");
            }
            continue;
        }

        if in_code_block {
            out.push_str("│  ");
            out.push_str(line);
            out.push('\n');
            continue;
        }

        if trimmed.is_empty() {
            out.push('\n');
            continue;
        }

        if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            out.push_str("───────────────────────────────────────────────────────────────\n");
            continue;
        }

        if let Some(title) = trimmed.strip_prefix("# ") {
            let clean_title = clean_inline_formatting(title.trim());
            let upper = clean_title.to_uppercase();
            out.push_str(&upper);
            out.push('\n');
            out.push_str("═══════════════════════════════════════════════════════════════\n");
            continue;
        }

        if let Some(title) = trimmed.strip_prefix("## ") {
            let clean_title = clean_inline_formatting(title.trim());
            let upper = clean_title.to_uppercase();
            out.push_str("■ ");
            out.push_str(&upper);
            out.push('\n');
            out.push_str("───────────────────────────────────────────────────────────────\n");
            continue;
        }

        if let Some(title) = trimmed.strip_prefix("### ") {
            let clean_title = clean_inline_formatting(title.trim());
            out.push_str("▶ ");
            out.push_str(&clean_title);
            out.push('\n');
            continue;
        }

        if let Some(quote) = trimmed.strip_prefix("> ") {
            let clean_quote = clean_inline_formatting(quote.trim());
            out.push_str("  │ ");
            out.push_str(&clean_quote);
            out.push('\n');
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("- [ ] ") {
            let clean_text = clean_inline_formatting(rest.trim());
            out.push_str("  ☐ ");
            out.push_str(&clean_text);
            out.push('\n');
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("- [x] ") {
            let clean_text = clean_inline_formatting(rest.trim());
            out.push_str("  ☑ ");
            out.push_str(&clean_text);
            out.push('\n');
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("- ") {
            let clean_text = clean_inline_formatting(rest.trim());
            out.push_str("  • ");
            out.push_str(&clean_text);
            out.push('\n');
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("* ") {
            let clean_text = clean_inline_formatting(rest.trim());
            out.push_str("  • ");
            out.push_str(&clean_text);
            out.push('\n');
            continue;
        }

        // Markdown table support: format tables into cozy, cleanly aligned text
        if trimmed.starts_with('|') && trimmed.ends_with('|') {
            let inner = &trimmed[1..trimmed.len() - 1];
            let is_separator = inner
                .chars()
                .all(|c| c == '-' || c == '|' || c == ':' || c == ' ');
            if is_separator {
                out.push_str("  ├────────────────────────────────────────────────────────────\n");
                continue;
            }
            let cells: Vec<String> = inner
                .split('|')
                .map(|cell| clean_inline_formatting(cell.trim()))
                .collect();

            if cells.len() == 2 {
                let col1 = &cells[0];
                let col2 = &cells[1];
                let pad = 16usize.saturating_sub(col1.chars().count());
                out.push_str("  ");
                out.push_str(col1);
                for _ in 0..pad {
                    out.push(' ');
                }
                out.push_str("│ ");
                out.push_str(col2);
                out.push('\n');
                continue;
            } else if cells.len() >= 3 {
                let col1 = &cells[0];
                let col2 = &cells[1];
                let col3 = &cells[2];
                let pad1 = 12usize.saturating_sub(col1.chars().count());
                let pad2 = 24usize.saturating_sub(col2.chars().count());
                out.push_str("  ");
                out.push_str(col1);
                for _ in 0..pad1 {
                    out.push(' ');
                }
                out.push_str("│ ");
                out.push_str(col2);
                for _ in 0..pad2 {
                    out.push(' ');
                }
                out.push_str("│ ");
                out.push_str(col3);
                out.push('\n');
                continue;
            }
        }

        // Standard text line: clean inline markdown symbols
        let clean_line = clean_inline_formatting(line);
        out.push_str(&clean_line);
        out.push('\n');
    }

    out
}

/// Strips inline formatting such as **bold**, *italic*, `code`, and [links](url).
pub fn clean_inline_formatting(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '*' => {
                if chars.peek() == Some(&'*') {
                    chars.next();
                }
            }
            '`' => {}
            '[' => {
                let mut label = String::new();
                let mut found_close = false;
                while let Some(&nc) = chars.peek() {
                    chars.next();
                    if nc == ']' {
                        found_close = true;
                        break;
                    }
                    label.push(nc);
                }
                if found_close && chars.peek() == Some(&'(') {
                    chars.next();
                    while let Some(&nc) = chars.peek() {
                        chars.next();
                        if nc == ')' {
                            break;
                        }
                    }
                }
                out.push_str(&label);
            }
            _ => {
                out.push(c);
            }
        }
    }

    out
}

pub enum DocSidebarAction {
    SelectDoc(usize),
    ToggleSidebar,
    BackToEditor,
    OpenSettings,
}

/// Renders the dedicated documentation navigation sidebar on the left side of the doc view.
/// Exactly mirrors the architecture and aesthetic quality of the main notes sidebar.
pub fn render_doc_sidebar(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    rect: Rect,
    active_idx: usize,
    selected_idx: usize,
    is_focused: bool,
    theme: &crate::ui::theme::Theme,
    opacity: f32,
    any_modal_open: bool,
    filter_query: &mut String,
    focus_search: &mut bool,
) -> Option<DocSidebarAction> {
    let mut action = None;

    // 1. Sidebar card background and border (matching sidebar.rs)
    let sb_stroke = if is_focused {
        Stroke::new(1.5, theme.accent.gamma_multiply(0.80))
    } else {
        Stroke::new(1.0, theme.border().gamma_multiply(0.60))
    };
    let sb_bg = if opacity >= 0.99 {
        theme.sidebar_bg()
    } else {
        let alpha = ((opacity * 255.0) as u8).max(225);
        Color32::from_rgba_unmultiplied(
            theme.sidebar_bg().r(),
            theme.sidebar_bg().g(),
            theme.sidebar_bg().b(),
            alpha,
        )
    };
    painter.rect(rect, 5.0, sb_bg, sb_stroke, egui::StrokeKind::Inside);

    if any_modal_open {
        return None;
    }

    // 2. Compute inner boundary and exact 5px section gaps (identical to sidebar.rs)
    let pad_x = 12.0;
    let pad_y = 12.0;
    let inner_rect = Rect::from_min_max(
        pos2(rect.min.x + pad_x, rect.min.y + pad_y),
        pos2(rect.max.x - pad_x, rect.max.y - pad_y),
    );

    let header_h = 58.0;
    let header_rect = Rect::from_min_max(
        inner_rect.min,
        pos2(inner_rect.max.x, inner_rect.min.y + header_h),
    );

    let footer_h = 28.0;
    let footer_rect = Rect::from_min_max(
        pos2(inner_rect.min.x, inner_rect.max.y - footer_h),
        inner_rect.max,
    );

    const SECTION_GAP: f32 = 5.0;
    let body_top = header_rect.max.y + SECTION_GAP;
    let body_bottom = (footer_rect.min.y - SECTION_GAP).max(body_top);
    let body_rect = Rect::from_min_max(
        pos2(inner_rect.min.x, body_top),
        pos2(inner_rect.max.x, body_bottom),
    );

    // ── Header Component: Search input on Row 1, Stats + Collapse on Row 2 ──
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
                            egui::RichText::new("Search tutor guides (Ctrl+F)...")
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

            // Row 2: Flex row with Stats/Title on the LEFT, and Collapse icon on the RIGHT
            let controls_row_h = 22.0;
            let (controls_rect, _) = ui.allocate_exact_size(
                vec2(ui.available_width(), controls_row_h),
                egui::Sense::hover(),
            );

            ui.allocate_new_ui(
                egui::UiBuilder::new()
                    .max_rect(controls_rect)
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
                |ui| {
                    // Left: Section Title / Badge
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = vec2(4.0, 0.0);
                        ui.label(
                            egui::RichText::new("📖")
                                .size(11.5)
                                .color(theme.accent),
                        );
                        ui.label(
                            egui::RichText::new("TUTOR GUIDES")
                                .size(11.0)
                                .strong()
                                .color(theme.text),
                        );
                        let count_str = format!("({})", BRAIN_DOCS.len());
                        ui.label(
                            egui::RichText::new(count_str)
                                .size(10.5)
                                .color(theme.muted),
                        );
                    });

                    // Right: Collapse sidebar button
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let btn_size = 20.0;
                        let (btn_rect, btn_resp) = ui.allocate_exact_size(
                            vec2(btn_size, btn_size),
                            egui::Sense::click(),
                        );
                        let btn_resp = btn_resp.on_hover_text("Collapse sidebar (Ctrl+B)");
                        let is_hovered = btn_resp.hovered() || ui.rect_contains_pointer(btn_rect);
                        if is_hovered {
                            let hover_bg = if theme.is_light() {
                                Color32::from_rgba_unmultiplied(0, 0, 0, 14)
                            } else {
                                Color32::from_rgba_unmultiplied(255, 255, 255, 14)
                            };
                            ui.painter().rect_filled(btn_rect, 3.0, hover_bg);
                        }
                        ui.painter().text(
                            btn_rect.center(),
                            Align2::CENTER_CENTER,
                            "◀",
                            FontId::proportional(11.0),
                            if is_hovered { theme.text } else { theme.muted },
                        );
                        if btn_resp.clicked() {
                            action = Some(DocSidebarAction::ToggleSidebar);
                        }
                    });
                },
            );
        },
    );

    // ── Body Component: ScrollArea matching sidebar/body.rs ─────────────────
    ui.allocate_new_ui(
        egui::UiBuilder::new()
            .max_rect(body_rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
        |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let q = filter_query.trim().to_lowercase();
                    let mut visible_count = 0;

                    for (idx, doc) in BRAIN_DOCS.iter().enumerate() {
                        if !q.is_empty()
                            && !doc.title.to_lowercase().contains(&q)
                            && !doc.filename.to_lowercase().contains(&q)
                        {
                            continue;
                        }
                        visible_count += 1;

                        let row_w = ui.available_width();
                        let row_h = 24.0;
                        let (row_rect, response) = ui.allocate_exact_size(
                            vec2(row_w, row_h),
                            egui::Sense::click(),
                        );
                        let hovered = response.hovered();
                        let active = idx == active_idx;
                        let selected = idx == selected_idx;

                        // Background & accent bar matching body.rs
                        if active || selected || hovered {
                            if selected && active {
                                let bg_alpha = if is_focused { 0.25 } else { 0.16 };
                                ui.painter().rect_filled(
                                    row_rect,
                                    3.0,
                                    theme.accent.gamma_multiply(bg_alpha),
                                );
                                let bar = Rect::from_min_size(
                                    pos2(row_rect.min.x + 1.0, row_rect.min.y + 3.0),
                                    vec2(3.0, row_h - 6.0),
                                );
                                ui.painter().rect_filled(bar, 1.5, theme.accent);
                            } else if active {
                                ui.painter().rect_filled(
                                    row_rect,
                                    3.0,
                                    theme.accent.gamma_multiply(0.18),
                                );
                                let bar = Rect::from_min_size(
                                    pos2(row_rect.min.x + 1.0, row_rect.min.y + 3.0),
                                    vec2(2.5, row_h - 6.0),
                                );
                                ui.painter().rect_filled(bar, 1.25, theme.accent);
                            } else if selected {
                                let bg_alpha = if is_focused { 0.18 } else { 0.10 };
                                let bar_color = if is_focused {
                                    theme.accent
                                } else {
                                    theme.muted
                                };
                                ui.painter().rect_filled(
                                    row_rect,
                                    3.0,
                                    theme.accent.gamma_multiply(bg_alpha),
                                );
                                let bar = Rect::from_min_size(
                                    pos2(row_rect.min.x + 1.0, row_rect.min.y + 3.0),
                                    vec2(2.5, row_h - 6.0),
                                );
                                ui.painter().rect_filled(bar, 1.25, bar_color);
                            } else if hovered {
                                let hover_bg = if theme.is_light() {
                                    Color32::from_rgba_unmultiplied(0, 0, 0, 10)
                                } else {
                                    Color32::from_rgba_unmultiplied(255, 255, 255, 12)
                                };
                                ui.painter().rect_filled(row_rect, 3.0, hover_bg);
                            }
                        }

                        // Document icon matching sidebar document styling
                        let icon_rect = Rect::from_min_size(
                            pos2(row_rect.min.x + 6.0, row_rect.center().y - 7.0),
                            vec2(14.0, 14.0),
                        );
                        let doc_icon_color = if active {
                            theme.accent
                        } else if selected && is_focused {
                            theme.accent.lerp_to_gamma(theme.text, 0.40)
                        } else {
                            theme.text.lerp_to_gamma(theme.muted, 0.30)
                        };
                        crate::ui_components::icons::render_document_icon(
                            ui.painter(),
                            icon_rect,
                            doc_icon_color,
                        );

                        // Document title text
                        let text_rect = Rect::from_min_max(
                            pos2(row_rect.min.x + 26.0, row_rect.min.y),
                            pos2(row_rect.max.x - 4.0, row_rect.max.y),
                        );
                        let text_painter = ui.painter().with_clip_rect(text_rect);
                        let text_color = if active || (selected && is_focused) {
                            theme.accent
                        } else if hovered {
                            theme.text
                        } else {
                            theme.text.lerp_to_gamma(theme.muted, 0.20)
                        };
                        text_painter.text(
                            pos2(text_rect.min.x, row_rect.center().y),
                            Align2::LEFT_CENTER,
                            doc.title,
                            FontId::proportional(12.5),
                            text_color,
                        );

                        if response.clicked() {
                            action = Some(DocSidebarAction::SelectDoc(idx));
                        }
                    }

                    if visible_count == 0 {
                        ui.add_space(20.0);
                        ui.vertical_centered(|ui| {
                            ui.label(
                                egui::RichText::new("No matching guides")
                                    .size(11.5)
                                    .color(theme.muted),
                            );
                        });
                    }
                });
        },
    );

    // ── Footer Component: Exactly matching sidebar/footer.rs ────────────────
    ui.allocate_new_ui(
        egui::UiBuilder::new()
            .max_rect(footer_rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
        |ui| {
            // Left: Settings button (28x28px, identical to footer.rs)
            let btn_size = 28.0;
            let (btn_rect, btn_resp) = ui.allocate_exact_size(
                vec2(btn_size, btn_size),
                egui::Sense::click(),
            );
            let btn_resp = btn_resp.on_hover_text("Settings & Preferences (Ctrl+,)");
            let is_hovered = btn_resp.hovered() || ui.rect_contains_pointer(btn_rect);
            let bg_color = if is_hovered {
                if theme.is_light() {
                    Color32::from_rgba_unmultiplied(0, 0, 0, 14)
                } else {
                    Color32::from_rgba_unmultiplied(255, 255, 255, 16)
                }
            } else {
                Color32::TRANSPARENT
            };
            let stroke = if is_hovered {
                Stroke::new(1.0_f32, theme.border())
            } else {
                Stroke::NONE
            };
            ui.painter().rect(btn_rect, 4.0, bg_color, stroke, egui::StrokeKind::Inside);
            ui.painter().text(
                btn_rect.center(),
                Align2::CENTER_CENTER,
                "⚙",
                FontId::proportional(15.0),
                if is_hovered { theme.text } else { theme.muted },
            );
            if btn_resp.clicked() {
                action = Some(DocSidebarAction::OpenSettings);
            }

            // Right: Return to Editor / Exit Tutor button
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let back_btn_w = 84.0;
                let back_btn_h = 26.0;
                let (back_rect, back_resp) = ui.allocate_exact_size(
                    vec2(back_btn_w, back_btn_h),
                    egui::Sense::click(),
                );
                let back_resp = back_resp
                    .on_hover_text("Exit Tutor mode and return to Notes (:tutor or :edit)");
                let is_hovered = back_resp.hovered() || ui.rect_contains_pointer(back_rect);
                let bg_color = if is_hovered {
                    if theme.is_light() {
                        Color32::from_rgba_unmultiplied(0, 0, 0, 14)
                    } else {
                        Color32::from_rgba_unmultiplied(255, 255, 255, 16)
                    }
                } else {
                    Color32::TRANSPARENT
                };
                let stroke = if is_hovered {
                    Stroke::new(1.0_f32, theme.border())
                } else {
                    Stroke::NONE
                };
                ui.painter().rect(back_rect, 4.0, bg_color, stroke, egui::StrokeKind::Inside);
                ui.painter().text(
                    back_rect.center(),
                    Align2::CENTER_CENTER,
                    "← Editor",
                    FontId::proportional(12.0),
                    if is_hovered { theme.accent } else { theme.muted },
                );
                if back_resp.clicked() {
                    action = Some(DocSidebarAction::BackToEditor);
                }
            });
        },
    );

    action
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_brain_docs_embedded_and_valid() {
        assert_eq!(BRAIN_DOCS.len(), 11);
        for doc in BRAIN_DOCS {
            assert!(!doc.id.is_empty());
            assert!(!doc.title.is_empty());
            assert!(!doc.filename.is_empty());
            assert!(!doc.content.is_empty());
            assert!(doc.content.len() > 100);
            assert!(!doc.filename.contains('-'));
            assert!(!doc.filename.contains('_'));
        }
    }

    #[test]
    fn test_format_doc_for_reader() {
        let raw = "# Hello World\n\n## Subheading\n\n- Point 1\n- Point 2\n\nText with **bold**.";
        let formatted = format_doc_for_reader(raw);
        assert!(!formatted.contains("# Hello"));
        assert!(formatted.contains("HELLO WORLD"));
        assert!(formatted.contains("═════"));
        assert!(formatted.contains("• Point 1"));
        assert!(!formatted.contains("**bold**"));
        assert!(formatted.contains("bold"));
    }

    #[test]
    fn test_doc_selection_bounds() {
        let total = BRAIN_DOCS.len();
        assert_eq!(total, 11);
        let mut selected = 0usize;
        // simulate Down / j moves
        for _ in 0..10 {
            if selected + 1 < total {
                selected += 1;
            }
        }
        assert_eq!(selected, total - 1);
        // simulate Up / k moves
        for _ in 0..10 {
            selected = selected.saturating_sub(1);
        }
        assert_eq!(selected, 0);
    }

    #[test]
    fn test_format_doc_markdown_table() {
        let raw = "| Key | Action |\n|---|---|\n| `h` | Move left |\n| `l` | Move right |";
        let formatted = format_doc_for_reader(raw);
        assert!(formatted.contains("Key"));
        assert!(formatted.contains("Action"));
        assert!(formatted.contains("├────────────────────────"));
        assert!(formatted.contains("Move left"));
        assert!(formatted.contains("Move right"));
    }
}
