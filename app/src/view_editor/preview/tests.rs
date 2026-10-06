#[cfg(test)]
mod tests {
    use super::super::parser::{parse_markdown, MdBlock};

    #[test]
    fn test_parse_markdown_blocks() {
        let md = r#"# Main Title
## Sub Title
### Section
#### Minor
- [ ] Unchecked task
- [x] Checked task
- Normal bullet
1. First step
```rust
fn main() {
    println!("hello");
}
```
> A famous quote

| Header A | Header B |
| -------- | -------- |
| Val 1    | Val 2    |

---
Paragraph text
"#;
        let blocks = parse_markdown(md);
        assert!(matches!(blocks[0], MdBlock::Heading1(ref t) if t == "Main Title"));
        assert!(matches!(blocks[1], MdBlock::Heading2(ref t) if t == "Sub Title"));
        assert!(matches!(blocks[2], MdBlock::Heading3(ref t) if t == "Section"));
        assert!(matches!(blocks[3], MdBlock::Heading4(ref t) if t == "Minor"));

        // Checkboxes
        assert!(
            matches!(blocks[4], MdBlock::ListItem { checked: Some(false), ref text, .. } if text == "Unchecked task")
        );
        assert!(
            matches!(blocks[5], MdBlock::ListItem { checked: Some(true), ref text, .. } if text == "Checked task")
        );
        assert!(
            matches!(blocks[6], MdBlock::ListItem { checked: None, ref text, .. } if text == "Normal bullet")
        );
        assert!(
            matches!(blocks[7], MdBlock::ListItem { checked: None, ref text, .. } if text == "First step")
        );

        // Code block
        assert!(
            matches!(blocks[8], MdBlock::CodeBlock { ref lang, ref code } if lang == "rust" && code.contains("println!"))
        );

        // Quote
        assert!(matches!(blocks[9], MdBlock::Quote { ref text, .. } if text == "A famous quote"));

        // Table
        if let MdBlock::Table {
            ref headers,
            ref rows,
        } = blocks[10]
        {
            assert_eq!(headers, &["Header A", "Header B"]);
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0], &["Val 1", "Val 2"]);
        } else {
            panic!("Expected Table block");
        }

        // Rule & Paragraph
        assert!(matches!(blocks[11], MdBlock::Rule));
        assert!(matches!(blocks[12], MdBlock::Paragraph(ref p) if p == "Paragraph text"));
    }

    #[test]
    fn test_nested_list_indent_levels() {
        let md = "- Level 0\n  - Level 1\n    - Level 2\n  1. Nested numbered\n";
        let blocks = parse_markdown(md);
        assert_eq!(blocks.len(), 4);
        assert!(
            matches!(blocks[0], MdBlock::ListItem { indent_level: 0, ref text, .. } if text == "Level 0")
        );
        assert!(
            matches!(blocks[1], MdBlock::ListItem { indent_level: 1, ref text, .. } if text == "Level 1")
        );
        assert!(
            matches!(blocks[2], MdBlock::ListItem { indent_level: 2, ref text, .. } if text == "Level 2")
        );
        assert!(
            matches!(blocks[3], MdBlock::ListItem { indent_level: 1, ref text, .. } if text == "Nested numbered")
        );
    }

    #[test]
    fn test_preview_first_block_suppresses_top_margin() {
        let md = "# Heading One\nParagraph";
        let blocks = parse_markdown(md);
        assert_eq!(blocks.len(), 2);
        assert!(matches!(blocks[0], MdBlock::Heading1(ref h) if h == "Heading One"));
        assert!(matches!(blocks[1], MdBlock::Paragraph(ref p) if p == "Paragraph"));
    }

    #[test]
    fn test_table_with_incomplete_rows_and_no_separator() {
        // Missing trailing pipe on row 2, and short row 3
        let md = "| name | age | country |\n| saboor | 34 | afghanistan\n| akjdf |";
        let blocks = parse_markdown(md);
        assert_eq!(blocks.len(), 1);
        if let MdBlock::Table {
            ref headers,
            ref rows,
        } = blocks[0]
        {
            assert_eq!(headers, &["name", "age", "country"]);
            assert_eq!(rows.len(), 2);
            assert_eq!(rows[0], &["saboor", "34", "afghanistan"]);
            // Short row normalized to 3 columns
            assert_eq!(rows[1], &["akjdf", "", ""]);
        } else {
            panic!("Expected Table block, got {:?}", blocks[0]);
        }
    }

    #[test]
    fn test_nested_quotes_without_double_enter() {
        let md = ">hello\n>> how are you?\n> > > deep quote";
        let blocks = parse_markdown(md);
        assert_eq!(blocks.len(), 3);
        assert!(matches!(&blocks[0], MdBlock::Quote { depth: 1, text } if text == "hello"));
        assert!(matches!(&blocks[1], MdBlock::Quote { depth: 2, text } if text == "how are you?"));
        assert!(matches!(&blocks[2], MdBlock::Quote { depth: 3, text } if text == "deep quote"));
    }

    #[test]
    fn test_render_preview_heading_metrics() {
        use crate::ui::theme::Theme;
        use eframe::egui::{pos2, Context, Rect};

        let ctx = Context::default();
        crate::services::font_manager::ensure_editor_font(&ctx);
        let _ = ctx.run(Default::default(), |ctx| {
            let painter = ctx.layer_painter(eframe::egui::LayerId::background());
            let theme = Theme::default();
            let viewport = Rect::from_min_max(pos2(0.0, 0.0), pos2(800.0, 600.0));

            // Level 1 heading at b_idx = 0 should have no top margin
            let h1_first = super::super::heading::render_preview_heading(
                &painter, &painter, 10.0, 10.0, 500.0, 14.0, 1, "Title 1", &theme, 0, viewport,
            );

            // Level 1 heading at b_idx > 0 should include top margin (12.0)
            let h1_later = super::super::heading::render_preview_heading(
                &painter, &painter, 10.0, 10.0, 500.0, 14.0, 1, "Title 1", &theme, 1, viewport,
            );
            assert_eq!(h1_later - h1_first, 12.0);

            // Level 4 heading should be shorter than Level 1 heading
            let h4 = super::super::heading::render_preview_heading(
                &painter, &painter, 10.0, 10.0, 500.0, 14.0, 4, "Title 4", &theme, 0, viewport,
            );
            assert!(h1_first > h4);
        });
    }

    #[test]
    fn test_render_preview_blockquote_metrics() {
        use crate::ui::theme::Theme;
        use eframe::egui::{pos2, Context, Rect};

        let ctx = Context::default();
        crate::services::font_manager::ensure_editor_font(&ctx);
        let _ = ctx.run(Default::default(), |ctx| {
            let painter = ctx.layer_painter(eframe::egui::LayerId::background());
            let theme = Theme::default();
            let viewport = Rect::from_min_max(pos2(0.0, 0.0), pos2(800.0, 600.0));

            let h_depth1 = super::super::blockquote::render_preview_blockquote(
                &painter,
                &painter,
                10.0,
                10.0,
                500.0,
                14.0,
                1,
                "Simple quote",
                &theme,
                viewport,
            );
            assert!(h_depth1 > 0.0);

            // Large depth clamp check
            let h_deep = super::super::blockquote::render_preview_blockquote(
                &painter,
                &painter,
                10.0,
                10.0,
                500.0,
                14.0,
                99,
                "Deep quote",
                &theme,
                viewport,
            );
            assert!(h_deep > 0.0);
        });
    }

    #[test]
    fn test_render_preview_table_metrics() {
        use crate::ui::theme::Theme;
        use eframe::egui::{pos2, Context, Rect};

        let ctx = Context::default();
        crate::services::font_manager::ensure_editor_font(&ctx);
        let _ = ctx.run(Default::default(), |ctx| {
            eframe::egui::CentralPanel::default().show(ctx, |ui| {
                let painter = ui.painter().clone();
                let theme = Theme::default();
                let viewport = Rect::from_min_max(pos2(0.0, 0.0), pos2(800.0, 600.0));

                let headers = vec!["Col A".to_string(), "Col B".to_string()];
                let rows = vec![
                    vec!["Val 1".to_string(), "Val 2".to_string()],
                    vec!["Val 3".to_string(), "Val 4".to_string()],
                ];

                let table_h = super::super::table::render_preview_table(
                    ui, &painter, &painter, 10.0, 10.0, 500.0, 14.0, &headers, &rows, &theme, 0,
                    viewport,
                );
                assert!(table_h > 50.0);
            });
        });
    }

    #[test]
    fn test_render_preview_code_block_metrics() {
        use crate::ui::theme::Theme;
        use eframe::egui::{pos2, CentralPanel, Context, Rect};

        let ctx = Context::default();
        crate::services::font_manager::ensure_editor_font(&ctx);
        let _ = ctx.run(Default::default(), |ctx| {
            CentralPanel::default().show(ctx, |ui| {
                let painter = ui.painter().clone();
                let theme = Theme::default();
                let viewport = Rect::from_min_max(pos2(0.0, 0.0), pos2(800.0, 600.0));

                let code_h = super::super::code_block::render_preview_code_block(
                    ui,
                    &painter,
                    &painter,
                    10.0,
                    10.0,
                    500.0,
                    14.0,
                    "rust",
                    "fn main() {\n    println!(\"hello\");\n}\n",
                    &theme,
                    0,
                    viewport,
                );
                assert!(code_h > 50.0);
            });
        });
    }

    #[test]
    fn test_preview_empty_placeholder_zoom_safe() {
        use crate::ui::theme::Theme;
        use eframe::egui::{pos2, Context, Rect};

        let ctx = Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            let painter = ctx.layer_painter(eframe::egui::LayerId::background());
            let theme = Theme::default();
            let viewport = Rect::from_min_max(pos2(0.0, 0.0), pos2(600.0, 400.0));

            for font_size in [12.0, 16.0, 24.0, 36.0, 48.0, 64.0] {
                let avail_w = (viewport.width() - 32.0).max(80.0);
                let title_font = eframe::egui::FontId::proportional(font_size * 1.05);
                let sub_font = eframe::egui::FontId::proportional(font_size * 0.85);

                let title_galley = painter.layout(
                    "Nothing to preview yet".to_string(),
                    title_font,
                    theme.muted,
                    avail_w,
                );
                let sub_galley = painter.layout(
                    "Type markdown in the editor to see live rendering".to_string(),
                    sub_font,
                    theme.muted,
                    avail_w,
                );

                let gap = (font_size * 0.55).round().max(8.0);
                let total_h = title_galley.size().y + gap + sub_galley.size().y;
                assert!(gap >= 8.0);
                assert!(total_h > title_galley.size().y + sub_galley.size().y);
                // Verify strictly non-overlapping placement
                let title_bottom = title_galley.size().y;
                let sub_top = title_bottom + gap;
                assert!(sub_top > title_bottom);
            }
        });
    }
}
