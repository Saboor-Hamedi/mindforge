//! Multi-line document layout calculation and Galley compilation using CharMapBuilder.

use super::charmap::CharMapBuilder;
use super::classify::classify_lines;
use super::render::active::render_active_line;
use super::render::inactive::render_inactive_line;
use super::types::{InlineEditorLayout, InlineLine, InlineLineKind};
use crate::editor::Editor;
use crate::ui::theme::Theme;
use eframe::egui::text::LayoutJob;
use eframe::egui::{pos2, vec2, Rect};

/// Builds the `LayoutJob` and corresponding `char_map` for a single visual line,
/// strictly maintaining the invariant that `char_map.len() == total_glyphs_in_job + 1`.
pub fn build_line_layout(
    chars: &[char],
    char_start: usize,
    is_active: bool,
    base_font_size: f32,
    theme: &Theme,
    kind: &InlineLineKind,
    prefix_len: usize,
    in_code_block: bool,
    code_lang: Option<&str>,
    table_width: Option<f32>,
) -> (LayoutJob, Vec<usize>, f32) {
    let mut job = LayoutJob::default();
    let mut charmap = CharMapBuilder::new(char_start, chars.len());

    let line_h = if is_active {
        render_active_line(
            chars,
            char_start,
            base_font_size,
            theme,
            kind,
            prefix_len,
            in_code_block,
            code_lang,
            &mut job,
            &mut charmap,
        )
    } else {
        render_inactive_line(
            chars,
            char_start,
            base_font_size,
            theme,
            kind,
            prefix_len,
            in_code_block,
            code_lang,
            table_width,
            &mut job,
            &mut charmap,
        )
    };

    let map = charmap.finish(chars.len());
    (job, map, line_h)
}

/// Computes the complete layout of all lines in the editor buffer using `egui::Ui`.
pub fn compute_inline_layout(
    ui: &eframe::egui::Ui,
    ed: &Editor,
    wrap_width: f32,
    base_font_size: f32,
    theme: &Theme,
    ed_origin_x: f32,
) -> InlineEditorLayout {
    compute_inline_layout_ctx(ui.ctx(), ed, wrap_width, base_font_size, theme, ed_origin_x)
}

/// Computes the complete layout of all lines in the editor buffer using `egui::Context`.
pub fn compute_inline_layout_ctx(
    ctx: &eframe::egui::Context,
    ed: &Editor,
    wrap_width: f32,
    base_font_size: f32,
    theme: &Theme,
    _ed_origin_x: f32,
) -> InlineEditorLayout {
    crate::services::font_manager::ensure_editor_font(ctx);
    let mut layout = InlineEditorLayout::new();
    let buf = &ed.buf;
    let n = buf.len();

    // 1. Gather all line slices and ranges
    struct RawRange {
        start: usize,
        end: usize,
    }
    let mut ranges = Vec::new();
    let mut line_chars_vec: Vec<Vec<char>> = Vec::new();
    let mut line_start = 0;

    while line_start <= n {
        let mut line_end = line_start;
        while line_end < n && buf[line_end] != '\n' {
            line_end += 1;
        }

        ranges.push(RawRange { start: line_start, end: line_end });
        line_chars_vec.push(buf[line_start..line_end].to_vec());

        if line_end >= n {
            break;
        }
        line_start = line_end + 1;
    }

    // 2. Classify lines with multi-line context (Setext headings and Table headers)
    let classified = classify_lines(&line_chars_vec);

    // 3. Layout calculation pass
    let mut cumulative_y = 0.0;
    let mut in_code_block = false;
    let mut current_code_lang: Option<String> = None;

    for (idx, raw) in ranges.into_iter().enumerate() {
        let line_chars = &line_chars_vec[idx];
        let (mut kind, prefix_len) = classified[idx].clone();

        if let InlineLineKind::CodeFence(ref l) = &kind {
            if !in_code_block {
                in_code_block = true;
                current_code_lang = if l.is_empty() { None } else { Some(l.clone()) };
            } else {
                in_code_block = false;
                current_code_lang = None;
            }
        } else if in_code_block {
            kind = InlineLineKind::CodeLine;
        }

        let is_active = ed.cur >= raw.start && (ed.cur <= raw.end || raw.end == n);

        let (mut job, char_map, min_line_h) = build_line_layout(
            line_chars,
            raw.start,
            is_active,
            base_font_size,
            theme,
            &kind,
            prefix_len,
            in_code_block && !matches!(kind, InlineLineKind::CodeFence(_)),
            current_code_lang.as_deref(),
            Some((wrap_width - 8.0).max(120.0).min((base_font_size * 25.0).max(650.0))),
        );

        job.wrap.max_width = wrap_width.max(100.0);
        let galley = ctx.fonts(|f| f.layout_job(job));

        let galley_h = galley.size().y;
        let line_h = galley_h.max(min_line_h);

        // Calculate interactive checkbox rect in document-relative coordinates (offset by ed_origin when drawn/hit-tested)
        let checkbox_rect = if let InlineLineKind::TaskItem { check_char_idx, .. } = kind {
            let box_size = 14.0;
            let indent_spaces = check_char_idx.saturating_sub(3);
            let indent_px = indent_spaces as f32 * (base_font_size * 0.55);
            let box_x = 2.0 + indent_px;
            let box_y = cumulative_y + (line_h - box_size) * 0.5;
            Some(Rect::from_min_size(pos2(box_x, box_y), vec2(box_size, box_size)))
        } else {
            None
        };

        layout.lines.push(InlineLine {
            char_start: raw.start,
            char_end: raw.end,
            y_offset: cumulative_y,
            height: line_h,
            base_font_size,
            kind,
            galley,
            char_map,
            checkbox_rect,
        });

        cumulative_y += line_h;
    }

    layout.total_height = cumulative_y;
    layout
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_char_map_invariant_all_lines() {
        let theme = Theme::default();
        let test_cases = vec![
            "# Heading 1",
            "## Heading 2",
            "- [ ] Unchecked task",
            "- [x] Checked task",
            "> Quote level 1",
            ">> Nested quote",
            "- Bullet list item",
            "1. Numbered item",
            "| Header 1 | Header 2 |",
            "| :--- | ---: |",
            "| Cell 1 | Cell 2 |",
            "---",
            "```rust",
            "let x = 42;",
            "Normal text with **bold**, *italic*, and `code`",
            "Escaped \\*asterisk\\* and \\[brackets\\]",
            "[link text](https://example.com)",
            "![alt text](https://example.com/img.png)",
            "<https://example.com>",
            "",
            "   ",
        ];

        for text in test_cases {
            let chars: Vec<char> = text.chars().collect();
            let (kind, p_len) = super::super::classify::classify_line(&chars);

            // Test active line
            let (job_act, map_act, _) = build_line_layout(
                &chars, 0, true, 14.0, &theme, &kind, p_len, false, None, None,
            );
            let glyph_count_act = job_act.text.chars().count();
            // Verify invariant: char_map length must equal total glyph count + 1
            assert_eq!(
                map_act.len(),
                glyph_count_act + 1,
                "Active invariant failed on {:?}: map.len()={}, glyphs={}",
                text, map_act.len(), glyph_count_act
            );

            // Test inactive line
            let (job_inact, map_inact, _) = build_line_layout(
                &chars, 0, false, 14.0, &theme, &kind, p_len, false, None, None,
            );
            let glyph_count_inact = job_inact.text.chars().count();
            assert_eq!(
                map_inact.len(),
                glyph_count_inact + 1,
                "Inactive invariant failed on {:?}: map.len()={}, glyphs={}",
                text, map_inact.len(), glyph_count_inact
            );
        }
    }

    #[test]
    fn test_blockquote_no_phantom_gap() {
        let theme = Theme::default();
        let chars: Vec<char> = ">>>> Nested quote".chars().collect();
        let (kind, p_len) = super::super::classify::classify_line(&chars);

        let (job_inact, map_inact, _) = build_line_layout(
            &chars, 0, false, 14.0, &theme, &kind, p_len, false, None, None,
        );

        let glyph_count = job_inact.text.chars().count();
        assert_eq!(map_inact.len(), glyph_count + 1);
        // The first 4 glyphs are spaces mapping to 0
        assert_eq!(map_inact[0], 0);
        assert_eq!(map_inact[1], 0);
        assert_eq!(map_inact[2], 0);
        assert_eq!(map_inact[3], 0);
    }
}
