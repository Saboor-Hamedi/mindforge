//! Active line renderer: renders markdown syntax tokens explicitly with syntax styling
//! so the user can see and edit markdown markup with exact 1:1 character mapping.

use super::super::charmap::{append_and_map, append_run_and_map, CharMapBuilder};
use super::super::elements::{
    code_metrics, heading_color, heading_metrics, highlight_code_chars, pipe_color, table_metrics,
};
use super::super::spans::parse_inline_spans;
use super::super::types::{InlineLineKind, InlineSpanKind};
use crate::ui::theme::Theme;
use eframe::egui::text::LayoutJob;
use eframe::egui::{Color32, FontId, Stroke, TextFormat};

/// Emits the active visual line into `LayoutJob` and `CharMapBuilder`.
pub fn render_active_line(
    chars: &[char],
    char_start: usize,
    base_font_size: f32,
    theme: &Theme,
    kind: &InlineLineKind,
    prefix_len: usize,
    in_code_block: bool,
    code_lang: Option<&str>,
    job: &mut LayoutJob,
    charmap: &mut CharMapBuilder,
) -> f32 {
    let n = chars.len();

    // 1. Inside multi-line code block (multi-color syntax highlighting with 14px code inset)
    if in_code_block && !matches!(kind, InlineLineKind::CodeFence(_)) {
        let font = FontId::monospace(base_font_size * 0.95);
        let line_h = (base_font_size * 1.55).round();
        let pad_fmt = TextFormat::simple(font, Color32::TRANSPARENT);
        append_and_map(job, charmap, "  ", char_start, pad_fmt);
        highlight_code_chars(chars, code_lang, base_font_size * 0.95, theme, job, charmap);
        return line_h;
    }

    // 2. Metrics and base text colors
    let (font_size, line_h) = match kind {
        InlineLineKind::Heading(lvl) => {
            let (font_id, lh) = heading_metrics(*lvl, base_font_size);
            (font_id.size, lh)
        }
        InlineLineKind::SetextHeading(lvl) => {
            let (font_id, lh) = heading_metrics(*lvl, base_font_size);
            (font_id.size, lh)
        }
        InlineLineKind::SetextUnderline(_) => (base_font_size * 0.85, (base_font_size * 1.3).round()),
        InlineLineKind::CodeFence(_) => {
            let is_closing = in_code_block;
            let (_, lh) = code_metrics(base_font_size, is_closing, true);
            (base_font_size * 0.95, lh)
        }
        InlineLineKind::Rule => (base_font_size * 0.8, 18.0),
        InlineLineKind::TableRow(info) => {
            let (font_id, lh) = table_metrics(base_font_size, info.is_header, info.is_separator, true);
            (font_id.size, lh)
        }
        _ => (base_font_size, (base_font_size * 1.55).round()),
    };

    let default_font = crate::services::font_manager::editor_font_id(font_size);
    let syntax_font = crate::services::font_manager::editor_font_id(font_size);

    let syntax_color = Color32::from_rgba_unmultiplied(
        theme.muted.r(),
        theme.muted.g(),
        theme.muted.b(),
        160,
    );

    let base_text_color = match kind {
        InlineLineKind::Heading(lvl) | InlineLineKind::SetextHeading(lvl) => heading_color(*lvl, theme),
        InlineLineKind::TaskItem { checked: true, .. } => {
            Color32::from_rgba_unmultiplied(theme.text.r(), theme.text.g(), theme.text.b(), 135)
        }
        _ => theme.text,
    };

    // 3. Render line-level prefix (if any)
    let content_start = match kind {
        InlineLineKind::CodeFence(_) => {
            let fmt = TextFormat::simple(syntax_font.clone(), theme.text);
            append_run_and_map(job, charmap, chars, 0..n, fmt);
            return line_h;
        }
        InlineLineKind::SetextUnderline(_) => {
            let fmt = TextFormat::simple(syntax_font.clone(), theme.text);
            append_run_and_map(job, charmap, chars, 0..n, fmt);
            return line_h;
        }
        InlineLineKind::Rule => {
            let fmt = TextFormat::simple(syntax_font.clone(), theme.text);
            append_run_and_map(job, charmap, chars, 0..n, fmt);
            return line_h;
        }
        InlineLineKind::TableRow(info) if info.is_separator => {
            let fmt = TextFormat::simple(syntax_font.clone(), theme.text);
            append_run_and_map(job, charmap, chars, 0..n, fmt);
            return line_h;
        }
        InlineLineKind::Heading(_)
        | InlineLineKind::Quote(_)
        | InlineLineKind::BulletItem
        | InlineLineKind::NumberedItem(_) => {
            if prefix_len > 0 && prefix_len <= n {
                let fmt = TextFormat::simple(syntax_font.clone(), theme.text);
                append_run_and_map(job, charmap, chars, 0..prefix_len, fmt);
                prefix_len
            } else {
                0
            }
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
        _ => 0,
    };

    // 4. Render Table Row cells with visible syntax on active line
    if let InlineLineKind::TableRow(_) = kind {
        let font = FontId::monospace(font_size);
        for i in content_start..n {
            let ch = chars[i];
            let color = if ch == '|' {
                pipe_color(theme, true)
            } else {
                base_text_color
            };
            let fmt = TextFormat::simple(font.clone(), color);
            append_run_and_map(job, charmap, chars, i..i + 1, fmt);
        }
        return line_h;
    }

    // 5. Parse and render inline spans for the remainder of the line
    let content_chars = &chars[content_start..n];
    let spans = parse_inline_spans(content_chars);

    for span in spans {
        let abs_start = content_start + span.start;
        let abs_end = content_start + span.end;

        match span.kind {
            InlineSpanKind::Text => {
                let fmt = TextFormat::simple(default_font.clone(), base_text_color);
                append_run_and_map(job, charmap, chars, abs_start..abs_end, fmt);
            }
            InlineSpanKind::Escape { ch: _ } => {
                // Render the '\' in syntax color, and escaped char in base_text_color
                let fmt_slash = TextFormat::simple(syntax_font.clone(), syntax_color);
                append_run_and_map(job, charmap, chars, abs_start..abs_start + 1, fmt_slash);
                let fmt_ch = TextFormat::simple(default_font.clone(), base_text_color);
                append_run_and_map(job, charmap, chars, abs_start + 1..abs_end, fmt_ch);
            }
            InlineSpanKind::Code => {
                let m = span.marker_len;
                let fmt_marker = TextFormat::simple(syntax_font.clone(), syntax_color);
                let fmt_body = TextFormat::simple(FontId::monospace(font_size * 0.95), theme.text);

                append_run_and_map(job, charmap, chars, abs_start..abs_start + m, fmt_marker.clone());
                if abs_end >= abs_start + 2 * m {
                    append_run_and_map(job, charmap, chars, abs_start + m..abs_end - m, fmt_body);
                    append_run_and_map(job, charmap, chars, abs_end - m..abs_end, fmt_marker);
                }
            }
            InlineSpanKind::Bold => {
                let m = span.marker_len;
                let fmt_marker = TextFormat::simple(syntax_font.clone(), syntax_color);
                let fmt_body = TextFormat::simple(default_font.clone(), theme.highlight);

                append_run_and_map(job, charmap, chars, abs_start..abs_start + m, fmt_marker.clone());
                if abs_end >= abs_start + 2 * m {
                    append_run_and_map(job, charmap, chars, abs_start + m..abs_end - m, fmt_body);
                    append_run_and_map(job, charmap, chars, abs_end - m..abs_end, fmt_marker);
                }
            }
            InlineSpanKind::Italic => {
                let m = span.marker_len;
                let fmt_marker = TextFormat::simple(syntax_font.clone(), syntax_color);
                let mut fmt_body = TextFormat::simple(default_font.clone(), base_text_color);
                fmt_body.italics = true;

                append_run_and_map(job, charmap, chars, abs_start..abs_start + m, fmt_marker.clone());
                if abs_end >= abs_start + 2 * m {
                    append_run_and_map(job, charmap, chars, abs_start + m..abs_end - m, fmt_body);
                    append_run_and_map(job, charmap, chars, abs_end - m..abs_end, fmt_marker);
                }
            }
            InlineSpanKind::BoldItalic => {
                let m = span.marker_len;
                let fmt_marker = TextFormat::simple(syntax_font.clone(), syntax_color);
                let mut fmt_body = TextFormat::simple(default_font.clone(), theme.highlight);
                fmt_body.italics = true;

                append_run_and_map(job, charmap, chars, abs_start..abs_start + m, fmt_marker.clone());
                if abs_end >= abs_start + 2 * m {
                    append_run_and_map(job, charmap, chars, abs_start + m..abs_end - m, fmt_body);
                    append_run_and_map(job, charmap, chars, abs_end - m..abs_end, fmt_marker);
                }
            }
            InlineSpanKind::Strike => {
                let m = span.marker_len;
                let fmt_marker = TextFormat::simple(syntax_font.clone(), syntax_color);
                let mut fmt_body = TextFormat::simple(default_font.clone(), theme.muted);
                fmt_body.strikethrough = Stroke::new(1.0_f32, theme.muted);

                append_run_and_map(job, charmap, chars, abs_start..abs_start + m, fmt_marker.clone());
                if abs_end >= abs_start + 2 * m {
                    append_run_and_map(job, charmap, chars, abs_start + m..abs_end - m, fmt_body);
                    append_run_and_map(job, charmap, chars, abs_end - m..abs_end, fmt_marker);
                }
            }
            InlineSpanKind::Link { .. } | InlineSpanKind::Image { .. } => {
                let mut fmt_text = TextFormat::simple(default_font.clone(), theme.accent);
                fmt_text.underline = Stroke::new(1.0_f32, theme.accent);
                append_run_and_map(job, charmap, chars, abs_start..abs_end, fmt_text);
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
            InlineSpanKind::Autolink { .. } => {
                let mut fmt = TextFormat::simple(default_font.clone(), theme.accent);
                fmt.underline = Stroke::new(1.0_f32, theme.accent);
                append_run_and_map(job, charmap, chars, abs_start..abs_end, fmt);
            }
            InlineSpanKind::FootnoteRef { .. } => {
                let fmt = TextFormat::simple(crate::services::font_manager::editor_font_id(font_size * 0.85), theme.text);
                append_run_and_map(job, charmap, chars, abs_start..abs_end, fmt);
            }
            InlineSpanKind::Html { .. } => {
                let fmt = TextFormat::simple(crate::services::font_manager::editor_font_id(font_size * 0.9), theme.muted);
                append_run_and_map(job, charmap, chars, abs_start..abs_end, fmt);
            }
            InlineSpanKind::HardBreak => {
                let fmt = TextFormat::simple(syntax_font.clone(), syntax_color);
                append_run_and_map(job, charmap, chars, abs_start..abs_end, fmt);
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
