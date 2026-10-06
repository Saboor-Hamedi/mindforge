//! Markdown AST definition, parser, and inline text formatting for the preview pane.

use crate::ui::theme::Theme;
use eframe::egui::text::LayoutJob;
use eframe::egui::{Color32, Stroke, TextFormat};

#[derive(Debug, Clone)]
pub enum MdBlock {
    Heading1(String),
    Heading2(String),
    Heading3(String),
    Heading4(String),
    CodeBlock {
        lang: String,
        code: String,
    },
    Quote {
        depth: usize,
        text: String,
    },
    ListItem {
        bullet: String,
        text: String,
        checked: Option<bool>,
        indent_level: usize,
    },
    Table {
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    Paragraph(String),
    Rule,
}

/// Parses a markdown blockquote line, detecting nested depths (> or >> or > > > or >hello).
/// Returns `Some((depth, content))`.
pub fn parse_markdown_quote_line(s: &str) -> Option<(usize, &str)> {
    let trimmed = s.trim();
    if !trimmed.starts_with('>') {
        return None;
    }
    let chars: Vec<char> = trimmed.chars().collect();
    let mut depth = 0;
    let mut idx = 0;
    while idx < chars.len() {
        if chars[idx] == '>' {
            depth += 1;
            idx += 1;
            while idx < chars.len() && chars[idx] == ' ' {
                idx += 1;
            }
        } else {
            break;
        }
    }
    if depth > 0 {
        let byte_offset = chars[..idx].iter().map(|c| c.len_utf8()).sum();
        let content = trimmed[byte_offset..].trim();
        Some((depth, content))
    } else {
        None
    }
}

/// Returns true if a line represents a markdown table row (header, separator, or body).
#[inline]
pub fn is_markdown_table_row(s: &str) -> bool {
    let s_trim = s.trim();
    if !s_trim.contains('|') {
        return false;
    }
    s_trim.starts_with('|')
        || s_trim.ends_with('|')
        || s_trim.split('|').filter(|p| !p.trim().is_empty()).count() >= 2
}

pub fn parse_markdown(text: &str) -> Vec<MdBlock> {
    let mut blocks = Vec::new();
    let mut lines = text.lines().peekable();

    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Code block: ```
        if trimmed.starts_with("```") {
            let lang = trimmed.trim_start_matches('`').trim().to_string();
            let mut code_lines = Vec::new();
            while let Some(next_line) = lines.next() {
                if next_line.trim().starts_with("```") {
                    break;
                }
                code_lines.push(next_line);
            }
            blocks.push(MdBlock::CodeBlock {
                lang,
                code: code_lines.join("\n"),
            });
            continue;
        }

        // Horizontal Rule: --- or *** or ___
        if (trimmed.starts_with("---") || trimmed.starts_with("***") || trimmed.starts_with("___"))
            && trimmed
                .chars()
                .all(|c| c == '-' || c == '*' || c == '_' || c == ' ')
            && trimmed.len() >= 3
        {
            blocks.push(MdBlock::Rule);
            continue;
        }

        // Headings: #, ##, ###, ####
        if let Some(rest) = trimmed.strip_prefix("# ") {
            blocks.push(MdBlock::Heading1(rest.trim().to_string()));
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("## ") {
            blocks.push(MdBlock::Heading2(rest.trim().to_string()));
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("### ") {
            blocks.push(MdBlock::Heading3(rest.trim().to_string()));
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("#### ") {
            blocks.push(MdBlock::Heading4(rest.trim().to_string()));
            continue;
        }

        // Blockquote: > or >> or > > > or >hello
        if let Some((depth, content)) = parse_markdown_quote_line(trimmed) {
            let mut quote_text = content.to_string();
            while let Some(next_line) = lines.peek() {
                let next_trimmed = next_line.trim();
                if let Some((next_depth, next_content)) = parse_markdown_quote_line(next_trimmed) {
                    if next_depth == depth {
                        if !quote_text.is_empty() && !next_content.is_empty() {
                            quote_text.push('\n');
                        }
                        quote_text.push_str(next_content);
                        lines.next();
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
            blocks.push(MdBlock::Quote {
                depth,
                text: quote_text,
            });
            continue;
        }

        // Table: lines matching markdown table row format
        if is_markdown_table_row(trimmed) {
            let mut table_lines = vec![trimmed.to_string()];
            while let Some(next_l) = lines.peek() {
                let n_trim = next_l.trim();
                if is_markdown_table_row(n_trim) {
                    table_lines.push(n_trim.to_string());
                    lines.next();
                } else {
                    break;
                }
            }
            if !table_lines.is_empty() {
                let parse_row = |r: &str| -> Vec<String> {
                    r.trim()
                        .trim_matches('|')
                        .split('|')
                        .map(|c| c.trim().to_string())
                        .collect()
                };

                let sep_idx = table_lines.iter().position(|l| {
                    let t = l.trim();
                    t.contains('-')
                        && t.chars()
                            .all(|c| c == '|' || c == '-' || c == ':' || c == ' ')
                });

                let header_idx = if let Some(s_idx) = sep_idx {
                    if s_idx > 0 {
                        s_idx - 1
                    } else {
                        0
                    }
                } else {
                    0
                };

                let headers = parse_row(&table_lines[header_idx]);
                let col_count = headers.len().max(1);

                let mut rows = Vec::new();
                for (idx, row_line) in table_lines.iter().enumerate() {
                    if idx == header_idx || sep_idx == Some(idx) {
                        continue;
                    }
                    let mut row_cells = parse_row(row_line);
                    if row_cells.len() < col_count {
                        row_cells.resize(col_count, String::new());
                    } else if row_cells.len() > col_count {
                        row_cells.truncate(col_count);
                    }
                    rows.push(row_cells);
                }
                blocks.push(MdBlock::Table { headers, rows });
                continue;
            }
        }

        // Compute indentation level for nested list items
        let indent_spaces = line
            .chars()
            .take_while(|&c| c == ' ' || c == '\t')
            .fold(0, |acc, c| if c == '\t' { acc + 4 } else { acc + 1 });
        let indent_level = indent_spaces / 2;

        // List item with checkbox: - [ ] or - [x]
        if let Some(rest) = trimmed.strip_prefix("- [ ] ") {
            blocks.push(MdBlock::ListItem {
                bullet: "".to_string(),
                text: rest.trim().to_string(),
                checked: Some(false),
                indent_level,
            });
            continue;
        }
        if let Some(rest) = trimmed
            .strip_prefix("- [x] ")
            .or_else(|| trimmed.strip_prefix("- [X] "))
        {
            blocks.push(MdBlock::ListItem {
                bullet: "".to_string(),
                text: rest.trim().to_string(),
                checked: Some(true),
                indent_level,
            });
            continue;
        }

        // Unordered List item: - or * or +
        if let Some(rest) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
            .or_else(|| trimmed.strip_prefix("+ "))
        {
            blocks.push(MdBlock::ListItem {
                bullet: "•".to_string(),
                text: rest.trim().to_string(),
                checked: None,
                indent_level,
            });
            continue;
        }

        // Ordered List item: 1. or 2.
        if let Some(dot_idx) = trimmed.find(". ") {
            let prefix = &trimmed[..dot_idx];
            if prefix.chars().all(|c| c.is_ascii_digit()) && !prefix.is_empty() {
                blocks.push(MdBlock::ListItem {
                    bullet: format!("{}.", prefix),
                    text: trimmed[dot_idx + 2..].trim().to_string(),
                    checked: None,
                    indent_level,
                });
                continue;
            }
        }

        // Standard Paragraph (can span multiple lines)
        let mut p_text = line.to_string();
        while let Some(next_line) = lines.peek() {
            let next_trimmed = next_line.trim();
            let is_numbered = next_trimmed.find(". ").map_or(false, |dot_idx| {
                let prefix = &next_trimmed[..dot_idx];
                prefix.chars().all(|c| c.is_ascii_digit()) && !prefix.is_empty()
            });

            if next_trimmed.is_empty()
                || next_trimmed.starts_with('#')
                || next_trimmed.starts_with("```")
                || next_trimmed.starts_with('>')
                || next_trimmed.starts_with("- ")
                || next_trimmed.starts_with("* ")
                || next_trimmed.starts_with("+ ")
                || next_trimmed.starts_with("---")
                || is_numbered
                || is_markdown_table_row(next_trimmed)
            {
                break;
            }
            p_text.push(' ');
            p_text.push_str(next_trimmed);
            lines.next();
        }
        blocks.push(MdBlock::Paragraph(p_text));
    }

    blocks
}

/// Replaces multi-character coding symbols with Unicode ligature glyphs for preview rendering.
pub fn substitute_ligatures(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    let n = chars.len();

    while i < n {
        if i + 4 <= n {
            match (chars[i], chars[i + 1], chars[i + 2], chars[i + 3]) {
                ('=', '=', '=', '>') => {
                    result.push('⟹');
                    i += 4;
                    continue;
                }
                ('<', '=', '=', '=') => {
                    result.push('⟸');
                    i += 4;
                    continue;
                }
                ('-', '-', '-', '>') => {
                    result.push('⟶');
                    i += 4;
                    continue;
                }
                ('<', '-', '-', '-') => {
                    result.push('⟵');
                    i += 4;
                    continue;
                }
                _ => {}
            }
        }
        if i + 3 <= n {
            match (chars[i], chars[i + 1], chars[i + 2]) {
                ('=', '=', '>') => {
                    result.push('⟹');
                    i += 3;
                    continue;
                }
                ('<', '=', '=') => {
                    result.push('⟸');
                    i += 3;
                    continue;
                }
                ('-', '-', '>') => {
                    result.push('⟶');
                    i += 3;
                    continue;
                }
                ('<', '-', '-') => {
                    result.push('⟵');
                    i += 3;
                    continue;
                }
                ('=', '=', '=') => {
                    result.push('≡');
                    i += 3;
                    continue;
                }
                ('!', '=', '=') => {
                    result.push('≢');
                    i += 3;
                    continue;
                }
                _ => {}
            }
        }
        if i + 2 <= n {
            match (chars[i], chars[i + 1]) {
                ('-', '>') => {
                    result.push('→');
                    i += 2;
                    continue;
                }
                ('<', '-') => {
                    result.push('←');
                    i += 2;
                    continue;
                }
                ('=', '>') => {
                    result.push('⇒');
                    i += 2;
                    continue;
                }
                ('<', '=') => {
                    result.push('≤');
                    i += 2;
                    continue;
                }
                ('>', '=') => {
                    result.push('≥');
                    i += 2;
                    continue;
                }
                ('!', '=') => {
                    result.push('≠');
                    i += 2;
                    continue;
                }
                _ => {}
            }
        }

        result.push(chars[i]);
        i += 1;
    }

    result
}

/// Builds an egui LayoutJob supporting inline bold (**text**), italic (*text*),
/// inline code (`text`), strikethrough (~~text~~), and markdown links ([text](url)).
pub fn build_inline_job(
    text: &str,
    base_font_size: f32,
    default_color: Color32,
    theme: &Theme,
    wrap_width: f32,
) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.wrap.max_width = wrap_width;
    job.wrap.break_anywhere = false;

    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let mut i = 0;
    let mut plain_acc = String::new();

    let flush_plain = |acc: &mut String, job: &mut LayoutJob| {
        if !acc.is_empty() {
            let s = substitute_ligatures(acc);
            let fmt = TextFormat::simple(
                crate::services::font_manager::editor_font_id(base_font_size),
                default_color,
            );
            job.append(&s, 0.0, fmt);
            acc.clear();
        }
    };

    while i < n {
        // Check for **bold**
        if i + 1 < n && chars[i] == '*' && chars[i + 1] == '*' {
            if let Some(end_rel) = chars[i + 2..].windows(2).position(|w| w == ['*', '*']) {
                let bold_end = i + 2 + end_rel;
                flush_plain(&mut plain_acc, &mut job);
                let bold_raw: String = chars[i + 2..bold_end].iter().collect();
                let bold_text = substitute_ligatures(&bold_raw);
                let fmt = TextFormat::simple(
                    crate::services::font_manager::editor_font_id(base_font_size),
                    theme.highlight,
                );
                job.append(&bold_text, 0.0, fmt);
                i = bold_end + 2;
                continue;
            }
        }

        // Check for ~~strikethrough~~
        if i + 1 < n && chars[i] == '~' && chars[i + 1] == '~' {
            if let Some(end_rel) = chars[i + 2..].windows(2).position(|w| w == ['~', '~']) {
                let strike_end = i + 2 + end_rel;
                flush_plain(&mut plain_acc, &mut job);
                let strike_raw: String = chars[i + 2..strike_end].iter().collect();
                let strike_text = substitute_ligatures(&strike_raw);
                let mut fmt = TextFormat::simple(
                    crate::services::font_manager::editor_font_id(base_font_size),
                    theme.muted,
                );
                fmt.strikethrough = Stroke::new(1.0_f32, theme.muted);
                job.append(&strike_text, 0.0, fmt);
                i = strike_end + 2;
                continue;
            }
        }

        // Check for `inline code`
        if chars[i] == '`' {
            if let Some(end_rel) = chars[i + 1..].iter().position(|&c| c == '`') {
                let code_end = i + 1 + end_rel;
                flush_plain(&mut plain_acc, &mut job);
                let code_raw: String = chars[i + 1..code_end].iter().collect();
                let code_text = substitute_ligatures(&code_raw);
                let fmt = TextFormat::simple(
                    crate::services::font_manager::editor_font_id(base_font_size * 0.92),
                    theme.text,
                );
                job.append(&code_text, 0.0, fmt);
                i = code_end + 1;
                continue;
            }
        }

        // Check for [link](url)
        if chars[i] == '[' {
            if let Some(close_bracket) = chars[i + 1..].iter().position(|&c| c == ']') {
                let bracket_end = i + 1 + close_bracket;
                if bracket_end + 1 < n && chars[bracket_end + 1] == '(' {
                    if let Some(close_paren) =
                        chars[bracket_end + 2..].iter().position(|&c| c == ')')
                    {
                        let paren_end = bracket_end + 2 + close_paren;
                        flush_plain(&mut plain_acc, &mut job);
                        let link_raw: String = chars[i + 1..bracket_end].iter().collect();
                        let link_text = substitute_ligatures(&link_raw);
                        let mut fmt = TextFormat::simple(
                            crate::services::font_manager::editor_font_id(base_font_size),
                            theme.accent,
                        );
                        fmt.underline = Stroke::new(1.0_f32, theme.accent);
                        job.append(&link_text, 0.0, fmt);
                        i = paren_end + 1;
                        continue;
                    }
                }
            }
        }

        // Check for *italic*
        if chars[i] == '*' && (i + 1 >= n || chars[i + 1] != '*') {
            if let Some(end_rel) = chars[i + 1..].iter().position(|&c| c == '*') {
                let ital_end = i + 1 + end_rel;
                flush_plain(&mut plain_acc, &mut job);
                let ital_raw: String = chars[i + 1..ital_end].iter().collect();
                let ital_text = substitute_ligatures(&ital_raw);
                let mut fmt = TextFormat::simple(
                    crate::services::font_manager::editor_font_id(base_font_size),
                    theme.text,
                );
                fmt.italics = true;
                job.append(&ital_text, 0.0, fmt);
                i = ital_end + 1;
                continue;
            }
        }

        plain_acc.push(chars[i]);
        i += 1;
    }

    flush_plain(&mut plain_acc, &mut job);
    job
}
