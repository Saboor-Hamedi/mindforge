//! Document outline generator: scans headings H1-H6 and enables instant cursor jumping.

use crate::editor::Editor;
use crate::ui::theme::Theme;
use eframe::egui::{self, pos2, vec2, Align2, FontId, Rect, ScrollArea, Stroke, Ui};

/// A heading entry in the document outline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutlineHeading {
    pub level: u8,
    pub title: String,
    pub char_offset: usize,
    pub line_number: usize,
}

/// Extracts all H1-H6 headings from editor buffer, strictly ignoring code fences and paragraphs.
pub fn extract_outline_headings(ed: &Editor) -> Vec<OutlineHeading> {
    let mut headings = Vec::new();
    let text = ed.text();
    let mut char_count = 0;
    let mut in_code_block = false;

    for (line_idx, line) in text.lines().enumerate() {
        let trimmed_all = line.trim();
        if trimmed_all.starts_with("```") || trimmed_all.starts_with("~~~") {
            in_code_block = !in_code_block;
            char_count += line.chars().count() + 1;
            continue;
        }

        if in_code_block {
            char_count += line.chars().count() + 1;
            continue;
        }

        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            let hash_count = trimmed.chars().take_while(|&c| c == '#').count();
            if (1..=6).contains(&hash_count) {
                let rest = &trimmed[hash_count..];
                // Heading MUST have at least one space or tab immediately after the hash symbols
                if rest.starts_with(' ') || rest.starts_with('\t') {
                    let title = rest.trim();
                    if !title.is_empty() {
                        let leading_spaces = line.chars().count() - trimmed.chars().count();
                        headings.push(OutlineHeading {
                            level: hash_count as u8,
                            title: title.to_string(),
                            char_offset: char_count + leading_spaces,
                            line_number: line_idx + 1,
                        });
                    }
                }
            }
        }
        char_count += line.chars().count() + 1; // +1 for newline
    }

    headings
}

pub enum OutlineAction {
    JumpToChar(usize),
}

/// Renders the sleek Outline panel.
pub fn render_outline_panel(
    ui: &mut Ui,
    rect: Rect,
    headings: &[OutlineHeading],
    selected_idx: &mut usize,
    theme: &Theme,
    ed_cur: usize,
) -> Option<OutlineAction> {
    if headings.is_empty() {
        let painter = ui.painter();
        let center = rect.center();

        // Vector outline / list icon
        let icon_rect = Rect::from_center_size(pos2(center.x, center.y - 32.0), vec2(28.0, 28.0));
        let stroke = Stroke::new(
            1.8_f32,
            eframe::egui::Color32::from_rgba_unmultiplied(
                theme.accent.r(),
                theme.accent.g(),
                theme.accent.b(),
                130,
            ),
        );
        let ic = icon_rect.center();
        painter.line_segment(
            [pos2(ic.x - 9.0, ic.y - 6.0), pos2(ic.x + 9.0, ic.y - 6.0)],
            stroke,
        );
        painter.line_segment([pos2(ic.x - 9.0, ic.y), pos2(ic.x + 6.0, ic.y)], stroke);
        painter.line_segment(
            [pos2(ic.x - 9.0, ic.y + 6.0), pos2(ic.x + 3.0, ic.y + 6.0)],
            stroke,
        );

        // Centered Title
        painter.text(
            pos2(center.x, center.y + 2.0),
            Align2::CENTER_CENTER,
            "No Outline Headings",
            FontId::proportional(13.0),
            theme.text,
        );

        // Subtext / guidance
        painter.text(
            pos2(center.x, center.y + 24.0),
            Align2::CENTER_CENTER,
            "Structure your note using # Heading 1\nup to ###### Heading 6 to view outline.",
            FontId::proportional(11.0),
            theme.muted,
        );
        return None;
    }

    // Auto-sync selected heading to closest heading before editor cursor
    if *selected_idx >= headings.len() {
        *selected_idx = 0;
    }

    let mut action = None;

    // Handle keyboard navigation: ArrowUp/Down and Ctrl+K/J
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
            action = Some(OutlineAction::JumpToChar(
                headings[*selected_idx].char_offset,
            ));
        }
        if down && *selected_idx + 1 < headings.len() {
            *selected_idx += 1;
            action = Some(OutlineAction::JumpToChar(
                headings[*selected_idx].char_offset,
            ));
        }
        if enter {
            if let Some(h) = headings.get(*selected_idx) {
                action = Some(OutlineAction::JumpToChar(h.char_offset));
            }
        }
    }

    // Determine currently active heading based on editor caret
    let mut active_heading_idx = 0;
    for (i, h) in headings.iter().enumerate() {
        if ed_cur >= h.char_offset {
            active_heading_idx = i;
        } else {
            break;
        }
    }

    let item_h = 24.0;

    ui.allocate_new_ui(eframe::egui::UiBuilder::new().max_rect(rect), |ui| {
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (idx, h) in headings.iter().enumerate() {
                    let is_active = idx == active_heading_idx;
                    let is_selected = idx == *selected_idx;

                    // Indent based on heading level: H1 = 0px, H2 = 10px, H3 = 20px, etc.
                    let indent = (h.level.saturating_sub(1) as f32) * 11.0;

                    let (item_rect, resp) = ui.allocate_exact_size(
                        vec2(rect.width() - 8.0, item_h),
                        egui::Sense::click(),
                    );
                    let is_hovered = resp.hovered();

                    if resp.clicked() {
                        *selected_idx = idx;
                        action = Some(OutlineAction::JumpToChar(h.char_offset));
                    }

                    let painter = ui.painter();

                    // Pure borderless typography: NO background highlight!
                    let text_color = if is_selected || is_active || is_hovered {
                        theme.accent
                    } else {
                        theme.text
                    };

                    // Level badge, e.g. "H1", "H2"
                    let tag = format!("H{}", h.level);
                    painter.text(
                        pos2(item_rect.min.x + 6.0 + indent, item_rect.center().y),
                        Align2::LEFT_CENTER,
                        tag,
                        FontId::monospace(9.5),
                        if is_selected || is_active {
                            theme.accent
                        } else {
                            theme.muted
                        },
                    );

                    // Heading title
                    let title_x = item_rect.min.x + 28.0 + indent;
                    let max_chars = (((item_rect.max.x - title_x - 30.0).max(20.0)) / 7.2) as usize;
                    let display_title = crate::ui::truncate_with_ellipsis(&h.title, max_chars);

                    painter.text(
                        pos2(title_x, item_rect.center().y),
                        Align2::LEFT_CENTER,
                        display_title,
                        FontId::monospace(11.0),
                        text_color,
                    );

                    // Line number on far right
                    painter.text(
                        pos2(item_rect.max.x - 4.0, item_rect.center().y),
                        Align2::RIGHT_CENTER,
                        format!(":{}", h.line_number),
                        FontId::monospace(9.5),
                        theme.muted,
                    );
                }
            });
    });

    action
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_outline_headings_levels() {
        let mut ed = Editor::new();
        ed.set_text("# Main Heading\nParagraph text\n## Sub Section\n### Deep Subsection\n###### Level 6 Detail");

        let headings = extract_outline_headings(&ed);
        assert_eq!(headings.len(), 4);

        assert_eq!(headings[0].level, 1);
        assert_eq!(headings[0].title, "Main Heading");
        assert_eq!(headings[0].line_number, 1);

        assert_eq!(headings[1].level, 2);
        assert_eq!(headings[1].title, "Sub Section");
        assert_eq!(headings[1].line_number, 3);

        assert_eq!(headings[2].level, 3);
        assert_eq!(headings[2].title, "Deep Subsection");
        assert_eq!(headings[2].line_number, 4);

        assert_eq!(headings[3].level, 6);
        assert_eq!(headings[3].title, "Level 6 Detail");
        assert_eq!(headings[3].line_number, 5);
    }

    #[test]
    fn test_extract_outline_ignores_code_blocks_and_paragraphs() {
        let mut ed = Editor::new();
        let markdown = r#"# Real Heading
This is regular paragraph text with #hashtag and no heading.

```rust
// Code block comment
# [derive(Debug)]
pub struct Example;
```

## Subheading Outside Code
Normal text
"#;
        ed.set_text(markdown);
        let headings = extract_outline_headings(&ed);
        assert_eq!(headings.len(), 2);
        assert_eq!(headings[0].title, "Real Heading");
        assert_eq!(headings[1].title, "Subheading Outside Code");
    }
}
