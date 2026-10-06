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
) -> Option<DocSidebarAction> {
    let mut action = None;

    let sb_stroke = if is_focused {
        Stroke::new(1.0, theme.accent.gamma_multiply(0.40))
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

    if any_modal_open {
        painter.rect(rect, 5.0, sb_bg, sb_stroke, egui::StrokeKind::Inside);
        return None;
    }

    painter.rect(rect, 5.0, sb_bg, sb_stroke, egui::StrokeKind::Inside);

    let origin = rect.min + vec2(16.0, 18.0);

    // Header branding
    painter.text(
        origin,
        Align2::LEFT_TOP,
        "MINDFORGE",
        FontId::monospace(15.0),
        theme.accent,
    );

    // Subtitle badge
    painter.text(
        origin + vec2(0.0, 20.0),
        Align2::LEFT_TOP,
        format!("📖 User Guides ({}) · Try examples", BRAIN_DOCS.len()),
        FontId::monospace(10.5),
        Color32::from_gray(140),
    );

    // Sidebar collapse icon on top right [◀]
    let collapse_rect = Rect::from_min_size(pos2(rect.max.x - 34.0, origin.y), vec2(22.0, 22.0));
    let collapse_resp = ui
        .allocate_rect(collapse_rect, egui::Sense::click())
        .on_hover_text("Collapse sidebar (Ctrl+B)");
    let collapse_hover = collapse_resp.hovered() || ui.rect_contains_pointer(collapse_rect);
    if collapse_hover {
        painter.rect_filled(collapse_rect, 4.0, Color32::from_rgb(26, 28, 36));
        if collapse_resp.clicked() || ui.input(|i| i.pointer.primary_clicked()) {
            action = Some(DocSidebarAction::ToggleSidebar);
        }
    }
    painter.text(
        collapse_rect.center(),
        Align2::CENTER_CENTER,
        "◀",
        FontId::monospace(11.0),
        if collapse_hover {
            theme.accent
        } else {
            Color32::from_gray(130)
        },
    );

    // Divider under header
    let div_y = origin.y + 44.0;
    painter.line_segment(
        [
            pos2(rect.min.x + 16.0, div_y),
            pos2(rect.max.x - 16.0, div_y),
        ],
        Stroke::new(1.0_f32, theme.border()),
    );

    // Navigation items (Body matching sidebar/body.rs)
    let doc_icons = ["📖", "⚡", "⌨", "⚔", "✨", "🚀", "🌐", "🧠", "🎨", "🖥", "🛡"];
    let start_y = div_y + 10.0;
    let item_w = rect.width() - 32.0;
    // Fit every guide between the header divider and the footer buttons.
    let footer_top = rect.max.y - 30.0 - 10.0 - 8.0;
    let doc_count = BRAIN_DOCS.len().max(1) as f32;
    let pitch = ((footer_top - start_y) / doc_count).clamp(20.0, 34.0);
    let item_h = (pitch - 4.0).max(18.0);
    let list_clip = Rect::from_min_max(pos2(rect.min.x, start_y), pos2(rect.max.x, footer_top));
    let list_painter = painter.with_clip_rect(list_clip);

    // Only the list uses the clipped painter; the footer below keeps the full one.
    {
        let painter = &list_painter;
        for (idx, doc) in BRAIN_DOCS.iter().enumerate() {
            let item_rect = Rect::from_min_size(
                pos2(origin.x, start_y + idx as f32 * pitch),
                vec2(item_w, item_h),
            );
            let is_active = idx == active_idx;
            let is_selected = idx == selected_idx;
            let is_hovered = ui.rect_contains_pointer(item_rect.intersect(list_clip));

            if is_active || (is_selected && is_focused) || is_hovered {
                let bg = if is_selected && is_focused && is_active {
                    Color32::from_rgba_unmultiplied(
                        theme.accent.r(),
                        theme.accent.g(),
                        theme.accent.b(),
                        24,
                    )
                } else if is_selected && is_focused {
                    Color32::from_rgba_unmultiplied(
                        theme.accent.r(),
                        theme.accent.g(),
                        theme.accent.b(),
                        18,
                    )
                } else if is_active {
                    Color32::from_rgba_unmultiplied(
                        theme.accent.r(),
                        theme.accent.g(),
                        theme.accent.b(),
                        16,
                    )
                } else {
                    Color32::from_rgba_unmultiplied(255, 255, 255, 8)
                };
                painter.rect_filled(item_rect, 4.0, bg);

                // Left accent bar on active or focused/selected item
                if is_active || (is_selected && is_focused) {
                    let bar_w = if is_selected && is_focused { 3.0 } else { 2.5 };
                    let bar = Rect::from_min_size(
                        pos2(item_rect.min.x, item_rect.min.y + 3.0),
                        vec2(bar_w, item_rect.height() - 6.0),
                    );
                    painter.rect_filled(bar, 1.5, theme.accent);
                }

                if is_hovered && ui.input(|i| i.pointer.primary_clicked()) {
                    action = Some(DocSidebarAction::SelectDoc(idx));
                }
            }

            let icon = doc_icons.get(idx).copied().unwrap_or("📄");
            let label = format!("{} {}", icon, doc.title);
            let label_color = if is_active {
                theme.accent
            } else if is_selected && is_focused {
                Color32::WHITE
            } else {
                theme.text
            };

            painter.text(
                pos2(item_rect.min.x + 10.0, item_rect.center().y),
                Align2::LEFT_CENTER,
                label,
                FontId::monospace(12.0),
                label_color,
            );
        }
    }

    // ── Footer Bar: Exactly matching sidebar/footer.rs ──────────────────────
    let btn_size = 30.0;
    let bottom_y = rect.max.y - btn_size - 10.0;

    // 1. Settings icon button on bottom left (identical geometry & styling to footer.rs)
    let settings_rect =
        Rect::from_min_size(pos2(rect.min.x + 14.0, bottom_y), vec2(btn_size, btn_size));
    let settings_resp = ui.allocate_rect(settings_rect, egui::Sense::click());
    let settings_hovered = settings_resp.hovered() || ui.rect_contains_pointer(settings_rect);

    let settings_bg = if settings_hovered {
        if theme.is_light() {
            Color32::from_rgba_unmultiplied(0, 0, 0, 14)
        } else {
            Color32::from_rgba_unmultiplied(255, 255, 255, 16)
        }
    } else {
        Color32::TRANSPARENT
    };
    let settings_stroke = if settings_hovered {
        Stroke::new(1.0_f32, theme.border())
    } else {
        Stroke::NONE
    };
    painter.rect(
        settings_rect,
        6.0,
        settings_bg,
        settings_stroke,
        egui::StrokeKind::Inside,
    );

    painter.text(
        settings_rect.center(),
        Align2::CENTER_CENTER,
        "⚙",
        FontId::proportional(15.5),
        if settings_hovered {
            theme.text
        } else {
            theme.muted
        },
    );
    let settings_resp = settings_resp.on_hover_text("Settings (Ctrl+,)");
    if settings_resp.clicked() || (settings_hovered && ui.input(|i| i.pointer.primary_clicked())) {
        action = Some(DocSidebarAction::OpenSettings);
    }

    // 2. Return to Notes Editor button on bottom right (sleek, compact, borderless idle)
    let back_btn_w = 76.0;
    let back_btn_h = 30.0;
    let back_rect = Rect::from_min_size(
        pos2(rect.max.x - back_btn_w - 14.0, bottom_y),
        vec2(back_btn_w, back_btn_h),
    );
    let back_resp = ui
        .allocate_rect(back_rect, egui::Sense::click())
        .on_hover_text("Return to Notes (:editor)");
    let back_hovered = back_resp.hovered() || ui.rect_contains_pointer(back_rect);

    let back_bg = if back_hovered {
        if theme.is_light() {
            Color32::from_rgba_unmultiplied(0, 0, 0, 14)
        } else {
            Color32::from_rgba_unmultiplied(255, 255, 255, 16)
        }
    } else {
        Color32::TRANSPARENT
    };
    let back_stroke = if back_hovered {
        Stroke::new(1.0_f32, theme.border())
    } else {
        Stroke::NONE
    };
    painter.rect(
        back_rect,
        6.0,
        back_bg,
        back_stroke,
        egui::StrokeKind::Inside,
    );

    painter.text(
        back_rect.center(),
        Align2::CENTER_CENTER,
        "← Notes",
        FontId::monospace(11.5),
        if back_hovered {
            theme.accent
        } else {
            theme.muted
        },
    );

    if back_resp.clicked() || (back_hovered && ui.input(|i| i.pointer.primary_clicked())) {
        action = Some(DocSidebarAction::BackToEditor);
    }

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
