//! Inactive line renderer: renders pure styled typography, hiding markdown syntax tokens
//! while strictly preserving 1:1 glyph mapping invariants with CharMapBuilder.

use super::super::charmap::{
    append_and_map, append_run_and_map, append_run_with_leading_space_and_map,
    append_with_leading_space_and_map, CharMapBuilder,
};
use super::super::elements::{
    bullet_glyph, cell_color, code_metrics, heading_color, heading_metrics,
    highlight_code_chars, number_glyph, quote_color, quote_indent, split_table_cells, table_metrics,
};
use super::super::spans::parse_inline_spans;
use super::super::types::{InlineLineKind, InlineSpanKind};
use crate::ui::theme::Theme;
use eframe::egui::text::LayoutJob;
use eframe::egui::{Color32, FontId, Stroke, TextFormat};

/// Emits the inactive visual line into `LayoutJob` and `CharMapBuilder`.
pub fn render_inactive_line(
    chars: &[char],
    char_start: usize,
    base_font_size: f32,
    theme: &Theme,
    kind: &InlineLineKind,
    prefix_len: usize,
    in_code_block: bool,
    code_lang: Option<&str>,
    table_width: Option<f32>,
    job: &mut LayoutJob,
    charmap: &mut CharMapBuilder,
) -> f32 {
    let n = chars.len();

    // 1. Inside multi-line code block (tokenized syntax highlighting with 14px code inset)
    if in_code_block && !matches!(kind, InlineLineKind::CodeFence(_)) {
        let font = FontId::monospace(base_font_size * 0.95);
        let line_h = (base_font_size * 1.55).round();
        let pad_fmt = TextFormat::simple(font, Color32::TRANSPARENT);
        append_and_map(job, charmap, "  ", char_start, pad_fmt);
        highlight_code_chars(chars, code_lang, base_font_size * 0.95, theme, job, charmap);
        return line_h;
    }

    // 2. Metrics and font sizing
    let (font_size, line_h) = match kind {
        InlineLineKind::Heading(lvl) | InlineLineKind::SetextHeading(lvl) => {
            let (font_id, lh) = heading_metrics(*lvl, base_font_size);
            (font_id.size, lh)
        }
        InlineLineKind::SetextUnderline(_) => (base_font_size * 0.5, 4.0),
        InlineLineKind::CodeFence(_) => {
            let is_closing = in_code_block;
            let (_, lh) = code_metrics(base_font_size, is_closing, false);
            (base_font_size * 0.7, lh)
        }
        InlineLineKind::Rule => (base_font_size * 0.8, 18.0),
        InlineLineKind::TableRow(info) => {
            let (font_id, lh) = table_metrics(base_font_size, info.is_header, info.is_separator, false);
            (font_id.size, lh)
        }
        _ => (base_font_size, (base_font_size * 1.55).round()),
    };

    let default_font = crate::services::font_manager::editor_font_id(font_size);

    let base_text_color = match kind {
        InlineLineKind::Heading(lvl) | InlineLineKind::SetextHeading(lvl) => heading_color(*lvl, theme),
        InlineLineKind::Quote(_) => quote_color(theme),
        InlineLineKind::TaskItem { checked: true, .. } => {
            Color32::from_rgba_unmultiplied(theme.text.r(), theme.text.g(), theme.text.b(), 135)
        }
        _ => theme.text,
    };

    // 3. Line-level element handling
    let content_start = match kind {
        InlineLineKind::CodeFence(_) => {
            // Opening fence provides 28px card header clearance; closing fence provides 12px bottom padding
            let fmt = TextFormat::simple(default_font.clone(), Color32::TRANSPARENT);
            append_and_map(job, charmap, " ", char_start, fmt);
            return line_h;
        }
        InlineLineKind::SetextUnderline(_) => {
            let fmt = TextFormat::simple(default_font.clone(), Color32::TRANSPARENT);
            append_and_map(job, charmap, " ", char_start, fmt);
            return line_h;
        }
        InlineLineKind::Rule => {
            let fmt = TextFormat::simple(default_font.clone(), Color32::TRANSPARENT);
            append_and_map(job, charmap, " ", char_start, fmt);
            return line_h;
        }
        InlineLineKind::TableRow(info) if info.is_separator => {
            // Inactive table separator line is invisible (header accent border acts as divider)
            let fmt = TextFormat::simple(default_font.clone(), Color32::TRANSPARENT);
            append_and_map(job, charmap, " ", char_start, fmt);
            return line_h;
        }
        InlineLineKind::Heading(_) => {
            // Skip ATX heading prefix (# ... ) on inactive line
            prefix_len
        }
        InlineLineKind::Quote(depth) => {
            // Append soft indent spaces proportional to depth inside card
            let indent_spaces = quote_indent(*depth);
            let fmt = TextFormat::simple(default_font.clone(), base_text_color);
            append_and_map(job, charmap, &indent_spaces, char_start, fmt);
            prefix_len
        }
        InlineLineKind::TaskItem { check_char_idx, .. } => {
            let indent_count = check_char_idx.saturating_sub(3);
            if indent_count > 0 {
                let spaces: String = " ".repeat(indent_count);
                append_and_map(
                    job,
                    charmap,
                    &spaces,
                    char_start,
                    TextFormat::simple(default_font.clone(), Color32::TRANSPARENT),
                );
            }
            // Reserve clean transparent space for the vector checkbox widget (22px)
            let fmt = TextFormat::simple(default_font.clone(), Color32::TRANSPARENT);
            append_and_map(job, charmap, "   ", char_start + check_char_idx, fmt);
            prefix_len
        }
        InlineLineKind::BulletItem => {
            // Render Bullet glyph
            let glyph = bullet_glyph();
            let fmt = TextFormat::simple(default_font.clone(), theme.text);
            append_and_map(job, charmap, glyph, char_start, fmt);
            prefix_len
        }
        InlineLineKind::NumberedItem(num) => {
            // Render Numbered item prefix
            let glyph = number_glyph(num);
            let fmt = TextFormat::simple(default_font.clone(), theme.text);
            append_and_map(job, charmap, &glyph, char_start, fmt);
            prefix_len
        }
        _ => 0,
    };

    // 4. Render Table Row (Inactive: column-aligned cells without pipe clutter)
    if let InlineLineKind::TableRow(info) = kind {
        let cell_font = if info.is_header {
            FontId::proportional(font_size * 0.95)
        } else {
            FontId::proportional(font_size * 0.90)
        };
        let c_color = cell_color(theme, info.is_header);
        let cells = split_table_cells(chars);

        if cells.is_empty() {
            let fmt = TextFormat::simple(cell_font, c_color);
            append_and_map(job, charmap, " ", char_start, fmt);
            return line_h;
        }

        let col_count = if info.col_count > 0 {
            info.col_count
        } else {
            info.aligns.len().max(cells.len()).max(1)
        };
        let min_table_w = (col_count as f32 * font_size * 5.0).max(300.0);
        let table_w = table_width.unwrap_or(600.0).max(min_table_w);
        let cell_pad_x = 14.0;
        let col_w = ((table_w - cell_pad_x * 2.0) / col_count as f32).max(font_size * 4.0);

        let mut cursor_x = 0.0f32;
        for (c_idx, cell) in cells.iter().enumerate() {
            let target_x = c_idx as f32 * col_w + cell_pad_x;
            let leading_space = (target_x - cursor_x).max(8.0);

            let fmt = TextFormat::simple(cell_font.clone(), c_color);

            if cell.content_end > cell.content_start {
                let char_count = cell.content_end - cell.content_start;
                let approx_text_w = char_count as f32 * (font_size * 0.52);
                append_run_with_leading_space_and_map(
                    job,
                    charmap,
                    chars,
                    cell.content_start..cell.content_end,
                    leading_space,
                    fmt,
                );
                cursor_x = target_x + approx_text_w;
            } else {
                // Empty cell: push single invisible space to keep column slot
                let empty_fmt = TextFormat::simple(cell_font.clone(), Color32::TRANSPARENT);
                append_with_leading_space_and_map(
                    job,
                    charmap,
                    " ",
                    char_start + cell.start.min(n),
                    leading_space,
                    empty_fmt,
                );
                cursor_x = target_x + font_size * 0.5;
            }
        }
        return line_h;
    }

    // 5. Parse and render inline spans for the remainder of the line
    let content_chars = if content_start <= n { &chars[content_start..n] } else { &[] };
    let spans = parse_inline_spans(content_chars);

    for span in spans {
        let abs_start = content_start + span.start;
        let abs_end = content_start + span.end;
        let m = span.marker_len;

        match span.kind {
            InlineSpanKind::Text => {
                let fmt = TextFormat::simple(default_font.clone(), base_text_color);
                append_run_and_map(job, charmap, chars, abs_start..abs_end, fmt);
            }
            InlineSpanKind::Escape { ch: _ } => {
                // Render only the escaped character (skip backslash)
                let fmt = TextFormat::simple(default_font.clone(), base_text_color);
                append_run_and_map(job, charmap, chars, abs_start + 1..abs_end, fmt);
            }
            InlineSpanKind::Code => {
                let fmt = TextFormat::simple(crate::services::font_manager::editor_font_id(font_size * 0.95), theme.text);
                if abs_end >= abs_start + 2 * m {
                    append_run_and_map(job, charmap, chars, abs_start + m..abs_end - m, fmt);
                }
            }
            InlineSpanKind::Bold => {
                let fmt = TextFormat::simple(default_font.clone(), theme.highlight);
                if abs_end >= abs_start + 2 * m {
                    append_run_and_map(job, charmap, chars, abs_start + m..abs_end - m, fmt);
                }
            }
            InlineSpanKind::Italic => {
                let mut fmt = TextFormat::simple(default_font.clone(), base_text_color);
                fmt.italics = true;
                if abs_end >= abs_start + 2 * m {
                    append_run_and_map(job, charmap, chars, abs_start + m..abs_end - m, fmt);
                }
            }
            InlineSpanKind::BoldItalic => {
                let mut fmt = TextFormat::simple(default_font.clone(), theme.highlight);
                fmt.italics = true;
                if abs_end >= abs_start + 2 * m {
                    append_run_and_map(job, charmap, chars, abs_start + m..abs_end - m, fmt);
                }
            }
            InlineSpanKind::Strike => {
                let mut fmt = TextFormat::simple(default_font.clone(), theme.muted);
                fmt.strikethrough = Stroke::new(1.0_f32, theme.muted);
                if abs_end >= abs_start + 2 * m {
                    append_run_and_map(job, charmap, chars, abs_start + m..abs_end - m, fmt);
                }
            }
            InlineSpanKind::Link { .. } => {
                // Render only [text], hide (url)
                let mut fmt = TextFormat::simple(default_font.clone(), theme.accent);
                fmt.underline = Stroke::new(1.0_f32, theme.accent);
                if let Some(close_bracket) = chars[abs_start..abs_end].iter().position(|&c| c == ']') {
                    append_run_and_map(job, charmap, chars, abs_start + 1..abs_start + close_bracket, fmt);
                } else {
                    append_run_and_map(job, charmap, chars, abs_start..abs_end, fmt);
                }
            }
            InlineSpanKind::WikiLink { .. } => {
                let fmt_marker = TextFormat::simple(default_font.clone(), theme.muted);
                let mut fmt_body = TextFormat::simple(default_font.clone(), theme.accent);
                fmt_body.underline = Stroke::new(1.0_f32, theme.accent);

                append_run_and_map(job, charmap, chars, abs_start..(abs_start + 2).min(abs_end), fmt_marker.clone());
                if abs_end >= abs_start + 4 {
                    append_run_and_map(job, charmap, chars, abs_start + 2..abs_end - 2, fmt_body);
                    append_run_and_map(job, charmap, chars, abs_end - 2..abs_end, fmt_marker);
                } else if abs_end > abs_start + 2 {
                    append_run_and_map(job, charmap, chars, abs_start + 2..abs_end, fmt_body);
                }
            }
            InlineSpanKind::Image { ref alt, .. } => {
                // Render image icon + alt text
                let fmt_icon = TextFormat::simple(default_font.clone(), theme.text);
                append_and_map(job, charmap, "🖼 ", abs_start, fmt_icon);
                let fmt_alt = TextFormat::simple(default_font.clone(), theme.muted);
                append_and_map(job, charmap, alt, abs_start, fmt_alt);
            }
            InlineSpanKind::Autolink { ref url } => {
                let mut fmt = TextFormat::simple(default_font.clone(), theme.accent);
                fmt.underline = Stroke::new(1.0_f32, theme.accent);
                append_and_map(job, charmap, url, abs_start + 1, fmt);
            }
            InlineSpanKind::FootnoteRef { ref id } => {
                let fmt = TextFormat::simple(crate::services::font_manager::editor_font_id(font_size * 0.85), theme.text);
                let s = format!("[^{id}]");
                append_and_map(job, charmap, &s, abs_start, fmt);
            }
            InlineSpanKind::Html { ref tag } => {
                let fmt = TextFormat::simple(crate::services::font_manager::editor_font_id(font_size * 0.9), theme.muted);
                let s = format!("<{tag}>");
                append_and_map(job, charmap, &s, abs_start, fmt);
            }
            InlineSpanKind::HardBreak => {
                // No extra visual glyph needed on inactive line
            }
        }
    }

    // Ensure empty lines have at least one transparent space so Galley has valid height
    if job.is_empty() {
        let fmt = TextFormat::simple(default_font, Color32::TRANSPARENT);
        append_and_map(job, charmap, " ", char_start, fmt);
    }

    line_h
}
