//! Unit tests for inline markdown classification, tokenization, and layout geometry.

use super::classify::classify_line;
use super::layout::build_line_layout;
use super::types::{InlineLineKind, TableAlign, TableRowInfo};
use crate::ui::theme::Theme;

#[test]
fn test_classify_headings() {
    let h1: Vec<char> = "# Hello World".chars().collect();
    let (kind, prefix_len) = classify_line(&h1);
    assert_eq!(kind, InlineLineKind::Heading(1));
    assert_eq!(prefix_len, 2);

    let h2: Vec<char> = "## Subtitle".chars().collect();
    let (kind, prefix_len) = classify_line(&h2);
    assert_eq!(kind, InlineLineKind::Heading(2));
    assert_eq!(prefix_len, 3);

    let h3: Vec<char> = "### Deep Topic".chars().collect();
    let (kind, prefix_len) = classify_line(&h3);
    assert_eq!(kind, InlineLineKind::Heading(3));
    assert_eq!(prefix_len, 4);

    let h4: Vec<char> = "#### Micro Topic".chars().collect();
    let (kind, prefix_len) = classify_line(&h4);
    assert_eq!(kind, InlineLineKind::Heading(4));
    assert_eq!(prefix_len, 5);
}

#[test]
fn test_classify_tasks() {
    let uncheck: Vec<char> = "- [ ] Buy groceries".chars().collect();
    let (kind, prefix_len) = classify_line(&uncheck);
    assert_eq!(
        kind,
        InlineLineKind::TaskItem {
            checked: false,
            check_char_idx: 3,
        }
    );
    assert_eq!(prefix_len, 6);

    let check: Vec<char> = "- [x] Fix compiler errors".chars().collect();
    let (kind, prefix_len) = classify_line(&check);
    assert_eq!(
        kind,
        InlineLineKind::TaskItem {
            checked: true,
            check_char_idx: 3,
        }
    );
    assert_eq!(prefix_len, 6);
}

#[test]
fn test_classify_bullets_and_quotes() {
    let bullet: Vec<char> = "- List item".chars().collect();
    let (kind, prefix_len) = classify_line(&bullet);
    assert_eq!(kind, InlineLineKind::BulletItem);
    assert_eq!(prefix_len, 2);

    let quote: Vec<char> = "> Inspirational quote".chars().collect();
    let (kind, prefix_len) = classify_line(&quote);
    assert_eq!(kind, InlineLineKind::Quote(1));
    assert_eq!(prefix_len, 2);

    let nested_quote: Vec<char> = ">> Level 2 quote".chars().collect();
    let (nested_kind, nested_prefix_len) = classify_line(&nested_quote);
    assert_eq!(nested_kind, InlineLineKind::Quote(2));
    assert_eq!(nested_prefix_len, 3);

    let deep_quote: Vec<char> = "> > > Level 3 quote".chars().collect();
    let (deep_kind, deep_prefix_len) = classify_line(&deep_quote);
    assert_eq!(deep_kind, InlineLineKind::Quote(3));
    assert_eq!(deep_prefix_len, 6);
}

#[test]
fn test_classify_rules_and_code() {
    let rule: Vec<char> = "---".chars().collect();
    let (kind, prefix_len) = classify_line(&rule);
    assert_eq!(kind, InlineLineKind::Rule);
    assert_eq!(prefix_len, 3);

    let code_fence: Vec<char> = "```rust".chars().collect();
    let (kind, prefix_len) = classify_line(&code_fence);
    assert_eq!(kind, InlineLineKind::CodeFence("rust".to_string()));
    assert_eq!(prefix_len, 7);
}

#[test]
fn test_classify_tables() {
    let header: Vec<char> = "| Title | Status | Date |".chars().collect();
    let (kind, prefix_len) = classify_line(&header);
    assert_eq!(
        kind,
        InlineLineKind::TableRow(TableRowInfo {
            is_header: false,
            is_separator: false,
            aligns: Vec::new(),
            col_count: 0,
        })
    );
    assert_eq!(prefix_len, 0);

    // Trailing spaces
    let header_trailing: Vec<char> = "| Title | Status | Date |   ".chars().collect();
    let (kind_trail, _) = classify_line(&header_trailing);
    assert_eq!(
        kind_trail,
        InlineLineKind::TableRow(TableRowInfo {
            is_header: false,
            is_separator: false,
            aligns: Vec::new(),
            col_count: 0,
        })
    );

    // Multi-pipe without leading pipe
    let no_lead: Vec<char> = "Title | Status | Date".chars().collect();
    let (kind_no_lead, _) = classify_line(&no_lead);
    assert_eq!(
        kind_no_lead,
        InlineLineKind::TableRow(TableRowInfo {
            is_header: false,
            is_separator: false,
            aligns: Vec::new(),
            col_count: 0,
        })
    );

    // Alignment colons
    let sep: Vec<char> = "|:---|:---:|---:|".chars().collect();
    let (kind, prefix_len) = classify_line(&sep);
    assert_eq!(
        kind,
        InlineLineKind::TableRow(TableRowInfo {
            is_header: false,
            is_separator: true,
            aligns: vec![TableAlign::Left, TableAlign::Center, TableAlign::Right],
            col_count: 0,
        })
    );
    assert_eq!(prefix_len, 0);

    // Separator with spaces
    let sep_spaces: Vec<char> = "| --- | --- | --- |".chars().collect();
    let (kind_spaces, _) = classify_line(&sep_spaces);
    assert_eq!(
        kind_spaces,
        InlineLineKind::TableRow(TableRowInfo {
            is_header: false,
            is_separator: true,
            aligns: vec![TableAlign::None, TableAlign::None, TableAlign::None],
            col_count: 0,
        })
    );
}

#[test]
fn test_unmatched_syntax_termination() {
    let theme = Theme::default();
    let edge_cases = [
        "* isolated asterisk",
        "** unclosed bold",
        "` unclosed code",
        "~~ unclosed strike",
        "> empty quote prefix ",
        "> > nested without text",
        "normal text with * and ` and ~ scattered",
    ];

    for line_text in edge_cases {
        let chars: Vec<char> = line_text.chars().collect();
        let (kind, p_len) = classify_line(&chars);

        // Test both active and inactive rendering
        let (job_active, map_active, _) =
            build_line_layout(&chars, 0, true, 14.0, &theme, &kind, p_len, false, None, None);
        assert!(!job_active.is_empty());
        assert!(!map_active.is_empty());

        let (job_inactive, map_inactive, _) =
            build_line_layout(&chars, 0, false, 14.0, &theme, &kind, p_len, false, None, None);
        assert!(!job_inactive.is_empty());
        assert!(!map_inactive.is_empty());
    }
}

#[test]
fn test_nested_quote_depth_and_inward_geometry() {
    let q1: Vec<char> = "> Level 1".chars().collect();
    let (kind1, len1) = classify_line(&q1);
    assert_eq!(kind1, InlineLineKind::Quote(1));
    assert_eq!(len1, 2);

    let q2: Vec<char> = ">> Level 2 compact".chars().collect();
    let (kind2, len2) = classify_line(&q2);
    assert_eq!(kind2, InlineLineKind::Quote(2));
    assert_eq!(len2, 3);

    let q3: Vec<char> = "> > > Level 3 spaced".chars().collect();
    let (kind3, len3) = classify_line(&q3);
    assert_eq!(kind3, InlineLineKind::Quote(3));
    assert_eq!(len3, 6);

    let q4: Vec<char> = ">>>> Level 4 compact".chars().collect();
    let (kind4, len4) = classify_line(&q4);
    assert_eq!(kind4, InlineLineKind::Quote(4));
    assert_eq!(len4, 5);
}

#[test]
fn test_task_item_variations() {
    let dash_unchecked: Vec<char> = "- [ ] Task 1".chars().collect();
    let (kind, len) = classify_line(&dash_unchecked);
    assert_eq!(kind, InlineLineKind::TaskItem { checked: false, check_char_idx: 3 });
    assert_eq!(len, 6);

    let star_unchecked: Vec<char> = "* [ ] Task 2".chars().collect();
    let (kind, len) = classify_line(&star_unchecked);
    assert_eq!(kind, InlineLineKind::TaskItem { checked: false, check_char_idx: 3 });
    assert_eq!(len, 6);

    let dash_checked_lower: Vec<char> = "- [x] Task 3".chars().collect();
    let (kind, len) = classify_line(&dash_checked_lower);
    assert_eq!(kind, InlineLineKind::TaskItem { checked: true, check_char_idx: 3 });
    assert_eq!(len, 6);

    let dash_checked_upper: Vec<char> = "- [X] Task 4".chars().collect();
    let (kind, len) = classify_line(&dash_checked_upper);
    assert_eq!(kind, InlineLineKind::TaskItem { checked: true, check_char_idx: 3 });
    assert_eq!(len, 6);

    let star_checked_upper: Vec<char> = "* [X] Task 5".chars().collect();
    let (kind, len) = classify_line(&star_checked_upper);
    assert_eq!(kind, InlineLineKind::TaskItem { checked: true, check_char_idx: 3 });
    assert_eq!(len, 6);
}

#[test]
fn test_code_fence_preserves_backticks() {
    let theme = Theme::default();

    // Opening code fence with language
    let open_fence: Vec<char> = "```rust".chars().collect();
    let (open_kind, p_len) = classify_line(&open_fence);
    assert_eq!(open_kind, InlineLineKind::CodeFence("rust".to_string()));

    // Verify active line retains all 3 backticks and language
    let (job_active, map_active, _) =
        build_line_layout(&open_fence, 0, true, 14.0, &theme, &open_kind, p_len, false, None, None);
    assert_eq!(&job_active.text, "```rust");
    assert_eq!(map_active.len(), open_fence.len() + 1);

    // Closing fence without language
    let close_fence: Vec<char> = "```".chars().collect();
    let (close_kind, close_p_len) = classify_line(&close_fence);
    assert_eq!(close_kind, InlineLineKind::CodeFence("".to_string()));

    let (close_active, _, _) =
        build_line_layout(&close_fence, 0, true, 14.0, &theme, &close_kind, close_p_len, false, None, None);
    assert_eq!(&close_active.text, "```");
}

#[test]
fn test_multiline_setext_and_table_header_promotion() {
    use super::classify::classify_lines;

    let lines: Vec<Vec<char>> = vec![
        "Setext Title".chars().collect(),
        "===".chars().collect(),
        "Setext Subtitle".chars().collect(),
        "---".chars().collect(),
        "| Header A | Header B |".chars().collect(),
        "| :--- | ---: |".chars().collect(),
        "| Cell 1 | Cell 2 |".chars().collect(),
    ];

    let classified = classify_lines(&lines);

    // 0: Setext Title -> Heading(1)
    assert_eq!(classified[0].0, InlineLineKind::SetextHeading(1));
    // 1: === -> SetextUnderline(1)
    assert_eq!(classified[1].0, InlineLineKind::SetextUnderline(1));
    // 2: Setext Subtitle -> Heading(2)
    assert_eq!(classified[2].0, InlineLineKind::SetextHeading(2));
    // 3: --- -> SetextUnderline(2)
    assert_eq!(classified[3].0, InlineLineKind::SetextUnderline(2));

    // 4: Header row promoted to is_header = true with aligns
    if let InlineLineKind::TableRow(ref info) = classified[4].0 {
        assert!(info.is_header);
        assert!(!info.is_separator);
        assert_eq!(info.aligns, vec![TableAlign::Left, TableAlign::Right]);
    } else {
        panic!("Expected TableRow header");
    }

    // 5: Separator row
    if let InlineLineKind::TableRow(ref info) = classified[5].0 {
        assert!(!info.is_header);
        assert!(info.is_separator);
    } else {
        panic!("Expected TableRow separator");
    }

    // 6: Data row
    if let InlineLineKind::TableRow(ref info) = classified[6].0 {
        assert!(!info.is_header);
        assert!(!info.is_separator);
    } else {
        panic!("Expected TableRow data");
    }
}

#[test]
fn test_inline_spans_parsing() {
    use super::spans::parse_inline_spans;
    use super::types::InlineSpanKind;

    let chars: Vec<char> = "Text with [Link](https://example.com) and ![Image](https://example.com/pic.png) and <https://auto.com> and \\*escaped\\*".chars().collect();
    let spans = parse_inline_spans(&chars);

    assert!(spans.iter().any(|s| matches!(s.kind, InlineSpanKind::Link { ref url, .. } if url == "https://example.com")));
    assert!(spans.iter().any(|s| matches!(s.kind, InlineSpanKind::Image { ref url, ref alt } if url == "https://example.com/pic.png" && alt == "Image")));
    assert!(spans.iter().any(|s| matches!(s.kind, InlineSpanKind::Autolink { ref url } if url == "https://auto.com")));
    assert!(spans.iter().any(|s| matches!(s.kind, InlineSpanKind::Escape { ch: '*' })));
}

#[test]
fn test_caret_pos_resets_cleanly_after_quote() {
    use super::types::{InlineEditorLayout, InlineLine};
    use eframe::egui::pos2;

    let theme = Theme::default();
    let origin = pos2(50.0, 100.0);

    // Line 0: >>>> Quote line (char 0..10)
    let q_chars: Vec<char> = ">>>> Quote".chars().collect();
    let (q_kind, q_len) = classify_line(&q_chars);
    let (job0, map0, h0) = build_line_layout(&q_chars, 0, false, 14.0, &theme, &q_kind, q_len, false, None, None);

    // Line 1: Normal line (char 11..16)
    let norm_chars: Vec<char> = "Hello".chars().collect();
    let (norm_kind, norm_len) = classify_line(&norm_chars);
    let (job1, map1, h1) = build_line_layout(&norm_chars, 11, true, 14.0, &theme, &norm_kind, norm_len, false, None, None);

    let ctx = eframe::egui::Context::default();
    crate::services::font_manager::ensure_editor_font(&ctx);
    let _ = ctx.run(Default::default(), |_| {});
    let galley0 = ctx.fonts(|f| f.layout_job(job0));
    let galley1 = ctx.fonts(|f| f.layout_job(job1));

    let layout = InlineEditorLayout {
        lines: vec![
            InlineLine {
                char_start: 0,
                char_end: 10,
                y_offset: 0.0,
                height: h0,
                base_font_size: 14.0,
                kind: q_kind,
                galley: galley0,
                char_map: map0,
                checkbox_rect: None,
            },
            InlineLine {
                char_start: 11,
                char_end: 16,
                y_offset: h0,
                height: h1,
                base_font_size: 14.0,
                kind: norm_kind,
                galley: galley1,
                char_map: map1,
                checkbox_rect: None,
            },
        ],
        total_height: h0 + h1,
    };

    // When cursor moves to line 1 start (char index 11), line_for_char must resolve line 1
    assert_eq!(layout.line_for_char(11), 1);
    let (pos, _) = layout.pos_for_char(11, origin);
    // X position at start of line 1 must snap cleanly to origin.x with zero quote offset drag
    assert_eq!(pos.x, origin.x);
}

#[test]
fn test_split_table_cells_and_column_alignment() {
    use super::elements::split_table_cells;

    let row: Vec<char> = "| Header A | Col B | Long Header C |".chars().collect();
    let cells = split_table_cells(&row);
    assert_eq!(cells.len(), 3);

    let cell0: String = row[cells[0].content_start..cells[0].content_end].iter().collect();
    let cell1: String = row[cells[1].content_start..cells[1].content_end].iter().collect();
    let cell2: String = row[cells[2].content_start..cells[2].content_end].iter().collect();
    assert_eq!(cell0, "Header A");
    assert_eq!(cell1, "Col B");
    assert_eq!(cell2, "Long Header C");

    // Inactive table row layout retains strictly invariant char_map
    let theme = Theme::default();
    let (kind, p_len) = classify_line(&row);
    let (job, map, _) = build_line_layout(&row, 0, false, 14.0, &theme, &kind, p_len, false, None, Some(600.0));
    let glyphs = job.text.chars().count();
    assert_eq!(map.len(), glyphs + 1);
}

#[test]
fn test_code_block_syntax_highlighting() {
    let theme = Theme::default();
    let code_line: Vec<char> = "    let mut x: u32 = 42; // comment".chars().collect();
    let kind = InlineLineKind::CodeLine;

    // Both active and inactive code lines preserve char_map invariant
    let (job_act, map_act, _) =
        build_line_layout(&code_line, 0, true, 14.0, &theme, &kind, 0, true, Some("rust"), None);
    let glyphs_act = job_act.text.chars().count();
    assert_eq!(map_act.len(), glyphs_act + 1);

    let (job_inact, map_inact, _) =
        build_line_layout(&code_line, 0, false, 14.0, &theme, &kind, 0, true, Some("rust"), None);
    let glyphs_inact = job_inact.text.chars().count();
    assert_eq!(map_inact.len(), glyphs_inact + 1);
}

#[test]
fn test_table_canonical_column_propagation() {
    use super::classify::classify_lines;

    let lines = vec![
        "| name | age | country |".chars().collect::<Vec<char>>(),
        "| --- | --- | --- |".chars().collect::<Vec<char>>(),
        "| saboor | 34 | afghanistan |".chars().collect::<Vec<char>>(),
        "| incomplete row |".chars().collect::<Vec<char>>(),
    ];

    let classified = classify_lines(&lines);
    assert_eq!(classified.len(), 4);

    // All rows in this table must receive col_count == 3
    for (i, (kind, _)) in classified.iter().enumerate() {
        if let InlineLineKind::TableRow(ref info) = kind {
            assert_eq!(
                info.col_count, 3,
                "Row {} has col_count {} instead of canonical 3",
                i, info.col_count
            );
        } else {
            panic!("Expected TableRow for line {}", i);
        }
    }
}

#[test]
fn test_paragraph_selection_height_matches_row_not_full_paragraph() {
    use crate::editor::Editor;
    use crate::ui::theme::Theme;
    use crate::view_editor::inline::layout::compute_inline_layout;
    use eframe::egui::{pos2, CentralPanel, Context};

    let mut ed = Editor::new();
    ed.buf = "This is a long paragraph designed to wrap across multiple lines in the layout engine for testing visual selection height.".chars().collect();
    ed.cur = 50;
    ed.selection = Some(50);
    ed.selection_inclusive = true; // normal 'v' selection

    let ctx = Context::default();
    crate::services::font_manager::ensure_editor_font(&ctx);
    let _ = ctx.run(Default::default(), |ctx| {
        CentralPanel::default().show(ctx, |ui| {
            let (sel_start, sel_end) = ed.selected_range().unwrap();
            assert_eq!(sel_start, 50);
            assert_eq!(sel_end, 51);

            let theme = Theme::default();
            // Wrap width of 120.0 forces this 123-char line to wrap into multiple rows
            let layout = compute_inline_layout(ui, &ed, 120.0, 14.0, &theme, 10.0);
            assert_eq!(layout.lines.len(), 1);
            let line = &layout.lines[0];
            assert!(line.galley.rows.len() > 1, "Expected paragraph to wrap into multiple rows");

            // Verify that pos_for_char gives the row's height, not the entire paragraph's height
            let (_pos, caret_h) = layout.pos_for_char(50, pos2(10.0, 10.0));
            assert!(caret_h < line.height, "Caret height ({}) must be less than full paragraph height ({})", caret_h, line.height);
        });
    });
}

#[test]
fn test_multiline_selection_geometric_continuity_and_text_boundary_precision() {
    use crate::editor::Editor;
    use crate::ui::theme::Theme;
    use crate::view_editor::inline::elements::selection::render_document_selection;
    use crate::view_editor::inline::layout::compute_inline_layout;
    use eframe::egui::{pos2, CentralPanel, Color32, Context};

    let mut ed = Editor::new();
    ed.buf = "First line\nSecond line is much longer and detailed\nThird line".chars().collect();
    // Select from character 3 ('s' of First) down to character 45 ('r' of Third)
    ed.cur = 45;
    ed.selection = Some(3);
    ed.selection_inclusive = true;

    let ctx = Context::default();
    crate::services::font_manager::ensure_editor_font(&ctx);
    let _ = ctx.run(Default::default(), |ctx| {
        CentralPanel::default().show(ctx, |ui| {
            let (sel_start, sel_end) = ed.selected_range().unwrap();
            assert_eq!(sel_start, 3);
            assert_eq!(sel_end, 46);

            let theme = Theme::default();
            let layout = compute_inline_layout(ui, &ed, 600.0, 14.0, &theme, 10.0);
            assert_eq!(layout.lines.len(), 3);

            let painter = ui.painter();
            let text_left = 10.0;
            let sel_color = Color32::from_rgba_unmultiplied(80, 120, 240, 70);

            // Execute selection rendering - renders via single mesh without panic or NaN
            render_document_selection(
                painter,
                &layout,
                pos2(10.0, 10.0),
                text_left,
                sel_start,
                sel_end,
                sel_color,
            );

            // Verify intermediate line text-boundary precision (line 1 is intermediate)
            let mid_line = &layout.lines[1];
            let row = &mid_line.galley.rows[0];
            // Row text width should be strictly bounded by text, not window width (600.0)
            assert!(row.rect.max.x < 500.0, "Intermediate row max.x ({}) must stop at text end", row.rect.max.x);
        });
    });
}




