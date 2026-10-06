//! Syntax highlighting tokenizer for preview code blocks.

use crate::ui::theme::Theme;
use eframe::egui::text::LayoutJob;
use eframe::egui::{Color32, TextFormat};

/// Tokenizes a code line into rich multi-color syntax highlighting.
pub fn highlight_code_line(line: &str, lang: &str, font_size: f32, theme: &Theme) -> LayoutJob {
    highlight_code_line_impl(line, lang, font_size, theme, true)
}

fn highlight_code_line_impl(
    line: &str,
    lang: &str,
    font_size: f32,
    theme: &Theme,
    use_ligatures: bool,
) -> LayoutJob {
    let mut job = LayoutJob::default();
    let mono_font = crate::services::font_manager::editor_font_id(font_size);
    let chars: Vec<char> = line.chars().collect();
    let n = chars.len();
    let mut i = 0;

    let language = lang.to_ascii_lowercase();
    if matches!(language.as_str(), "markdown" | "md") {
        return highlight_markdown_line(line, font_size, theme);
    }
    let is_python = matches!(language.as_str(), "python" | "py");
    let is_bash = lang.eq_ignore_ascii_case("bash")
        || lang.eq_ignore_ascii_case("sh")
        || lang.eq_ignore_ascii_case("shell");
    let is_yaml = matches!(language.as_str(), "yaml" | "yml");
    let is_sql = language == "sql";
    let is_html = matches!(language.as_str(), "html" | "htm");
    let is_php = language == "php";
    let is_c_style = matches!(
        language.as_str(),
        "javascript" | "js" | "jsx" | "typescript" | "ts" | "tsx" | "rust" | "rs" | "php" | "css"
    );

    while i < n {
        // 1. Comments
        let is_html_comment = is_html && i + 3 < n && chars[i..i + 4] == ['<', '!', '-', '-'];
        let is_sql_comment = is_sql && i + 1 < n && chars[i] == '-' && chars[i + 1] == '-';
        if is_html_comment
            || is_sql_comment
            || (is_c_style && chars[i] == '/' && i + 1 < n && chars[i + 1] == '/')
            || ((is_python || is_bash || is_yaml || is_php) && chars[i] == '#')
        {
            let comment_text: String = chars[i..].iter().collect();
            let mut fmt = TextFormat::simple(
                mono_font.clone(),
                Color32::from_rgba_unmultiplied(
                    theme.muted.r(),
                    theme.muted.g(),
                    theme.muted.b(),
                    170,
                ),
            );
            fmt.italics = true;
            job.append(&comment_text, 0.0, fmt);
            break;
        }

        // 2. Strings ("..." or '...')
        if chars[i] == '"' || chars[i] == '\'' || (chars[i] == '`' && !is_yaml && !is_sql) {
            let quote = chars[i];
            let start = i;
            i += 1;
            while i < n {
                if chars[i] == '\\' && i + 1 < n {
                    i += 2;
                } else if chars[i] == quote {
                    i += 1;
                    break;
                } else {
                    i += 1;
                }
            }
            let str_val: String = chars[start..i].iter().collect();
            // Strings: emerald / lime green
            let str_color = Color32::from_rgb(152, 195, 121);
            job.append(
                &str_val,
                0.0,
                TextFormat::simple(mono_font.clone(), str_color),
            );
            continue;
        }

        // 3. Numbers (integers, floats, hex)
        if chars[i].is_ascii_digit()
            && (i == 0 || (!chars[i - 1].is_alphanumeric() && chars[i - 1] != '_'))
        {
            let start = i;
            while i < n && (chars[i].is_ascii_alphanumeric() || chars[i] == '.' || chars[i] == '_')
            {
                i += 1;
            }
            let num_val: String = chars[start..i].iter().collect();
            // Numbers: amber / orange
            let num_color = Color32::from_rgb(209, 154, 102);
            job.append(
                &num_val,
                0.0,
                TextFormat::simple(mono_font.clone(), num_color),
            );
            continue;
        }

        // 4. Words (Keywords, Types, Identifiers, Macros)
        if chars[i].is_alphabetic() || chars[i] == '_' {
            let start = i;
            while i < n && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            // Macro indicator `!` (e.g. `println!`)
            if i < n && chars[i] == '!' {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();

            let is_sql_keyword = is_sql
                && matches!(
                    word.to_ascii_lowercase().as_str(),
                    "select"
                        | "from"
                        | "where"
                        | "join"
                        | "left"
                        | "right"
                        | "inner"
                        | "outer"
                        | "on"
                        | "as"
                        | "insert"
                        | "into"
                        | "values"
                        | "update"
                        | "set"
                        | "delete"
                        | "create"
                        | "table"
                        | "index"
                        | "drop"
                        | "alter"
                        | "group"
                        | "by"
                        | "order"
                        | "having"
                        | "limit"
                        | "offset"
                        | "and"
                        | "or"
                        | "not"
                        | "null"
                        | "primary"
                        | "key"
                        | "foreign"
                        | "references"
                        | "distinct"
                        | "union"
                        | "all"
                        | "case"
                        | "when"
                        | "then"
                        | "else"
                        | "end"
                );
            let is_keyword = is_sql_keyword
                || match word.as_str() {
                // Rust
                "fn" | "let" | "mut" | "pub" | "struct" | "enum" | "impl" | "match" | "if" | "else"
                | "return" | "use" | "mod" | "trait" | "type" | "where" | "async" | "await" | "for"
                | "in" | "while" | "loop" | "const" | "static" | "crate" | "super" | "self"
                // Python / JS / General
                | "def" | "class" | "import" | "from" | "as" | "with" | "yield" | "pass" | "lambda"
                | "function" | "var" | "export" | "try" | "catch" | "finally" | "new" | "this"
                | "typeof" | "instanceof" | "echo" | "sudo" | "public" | "private"
                | "protected" | "extends" | "implements" | "interface" | "default"
                | "switch" | "case" | "break" | "continue" | "throw" | "void" | "final"
                | "abstract" | "namespace" => true,
                _ => false,
            };

            let is_bool_or_none = match word.as_str() {
                "true" | "false" | "True" | "False" | "None" | "Some" | "Ok" | "Err" | "null"
                | "undefined" | "NULL" | "Null" => true,
                _ => false,
            };

            let is_type = !is_keyword
                && (word.starts_with(|c: char| c.is_ascii_uppercase())
                    || matches!(
                        word.as_str(),
                        "bool"
                            | "u8"
                            | "u16"
                            | "u32"
                            | "u64"
                            | "u128"
                            | "usize"
                            | "i8"
                            | "i16"
                            | "i32"
                            | "i64"
                            | "i128"
                            | "isize"
                            | "f32"
                            | "f64"
                            | "char"
                            | "str"
                            | "int"
                            | "float"
                            | "dict"
                            | "list"
                            | "number"
                            | "string"
                    ));

            let color = if is_keyword {
                theme.accent
            } else if is_bool_or_none {
                Color32::from_rgb(209, 154, 102) // amber
            } else if is_type {
                Color32::from_rgb(229, 192, 123) // warm gold
            } else if word.ends_with('!') {
                Color32::from_rgb(97, 175, 239) // cyan/blue for macros
            } else {
                theme.text
            };

            job.append(&word, 0.0, TextFormat::simple(mono_font.clone(), color));
            continue;
        }

        // 5. Coding Ligatures and Operators
        let ligature = if use_ligatures {
            crate::view_editor::ligatures::detect_ligature(&chars, i)
        } else {
            None
        };
        if let Some(lig) = ligature {
            let lig_len = lig.char_len();
            let raw: String = chars[i..i + lig_len].iter().collect();
            let sub = crate::view_editor::preview::parser::substitute_ligatures(&raw);
            let lig_color = Color32::from_rgb(97, 175, 239); // vibrant operator
            job.append(&sub, 0.0, TextFormat::simple(mono_font.clone(), lig_color));
            i += lig_len;
            continue;
        }

        // 6. Punctuation & Single Operators
        let punc_char = chars[i];
        let punc_color = match punc_char {
            '=' | '+' | '-' | '*' | '/' | '%' | '&' | '|' | '^' | '!' | '<' | '>' | '~' | '?'
            | ':' => {
                Color32::from_rgb(97, 175, 239) // vibrant operator
            }
            '{' | '}' | '(' | ')' | '[' | ']' => {
                Color32::from_rgb(224, 108, 117) // coral bracket
            }
            _ => Color32::from_rgba_unmultiplied(
                theme.muted.r(),
                theme.muted.g(),
                theme.muted.b(),
                180,
            ),
        };
        job.append(
            &punc_char.to_string(),
            0.0,
            TextFormat::simple(mono_font.clone(), punc_color),
        );
        i += 1;
    }

    job
}

/// Tokenizes a single markdown line with heading tiers, wikilinks, code blocks, links, and text formatting.
pub fn highlight_markdown_line(line: &str, font_size: f32, theme: &Theme) -> LayoutJob {
    let mut job = LayoutJob::default();
    let mono_font = crate::services::font_manager::editor_font_id(font_size);
    let trimmed = line.trim_start();
    let indent_len = line.len() - trimmed.len();

    // 1. Heading line (#, ##, ###, ...)
    if trimmed.starts_with('#') {
        let hash_count = trimmed.chars().take_while(|&c| c == '#').count();
        if hash_count <= 6
            && trimmed
                .chars()
                .nth(hash_count)
                .is_some_and(|c| c.is_whitespace())
        {
            let heading_color = match hash_count {
                1 => theme.accent,
                2 => Color32::from_rgb(97, 175, 239), // Light blue
                3 => Color32::from_rgb(152, 195, 121), // Sage green
                4 => Color32::from_rgb(224, 108, 117), // Coral
                _ => Color32::from_rgb(209, 154, 102), // Orange
            };
            if indent_len > 0 {
                job.append(
                    &line[..indent_len],
                    0.0,
                    TextFormat::simple(mono_font.clone(), theme.text),
                );
            }
            job.append(trimmed, 0.0, TextFormat::simple(mono_font, heading_color));
            return job;
        }
    }

    // 2. Blockquote (> ...)
    if trimmed.starts_with('>') {
        if indent_len > 0 {
            job.append(
                &line[..indent_len],
                0.0,
                TextFormat::simple(mono_font.clone(), theme.text),
            );
        }
        let quote_color =
            Color32::from_rgba_unmultiplied(theme.muted.r(), theme.muted.g(), theme.muted.b(), 180);
        let mut fmt = TextFormat::simple(mono_font, quote_color);
        fmt.italics = true;
        job.append(trimmed, 0.0, fmt);
        return job;
    }

    // 3. Inline tokens: [[Wikilinks]], `code`, [link](url), **bold**
    let chars: Vec<char> = line.chars().collect();
    let n = chars.len();
    let mut i = 0;

    while i < n {
        // Wikilink: [[Target]]
        if chars[i] == '[' && i + 1 < n && chars[i + 1] == '[' {
            let start = i;
            i += 2;
            let mut found_end = false;
            while i + 1 < n {
                if chars[i] == ']' && chars[i + 1] == ']' {
                    i += 2;
                    found_end = true;
                    break;
                }
                i += 1;
            }
            if found_end {
                let link_text: String = chars[start..i].iter().collect();
                let link_color = Color32::from_rgb(198, 120, 221); // purple
                job.append(
                    &link_text,
                    0.0,
                    TextFormat::simple(mono_font.clone(), link_color),
                );
                continue;
            } else {
                i = start;
            }
        }

        // Inline code: `code`
        if chars[i] == '`' {
            let start = i;
            i += 1;
            while i < n && chars[i] != '`' {
                i += 1;
            }
            if i < n && chars[i] == '`' {
                i += 1;
                let code_text: String = chars[start..i].iter().collect();
                let code_color = Color32::from_rgb(209, 154, 102); // amber
                job.append(
                    &code_text,
                    0.0,
                    TextFormat::simple(mono_font.clone(), code_color),
                );
                continue;
            } else {
                i = start;
            }
        }

        // Markdown link: [label](url)
        if chars[i] == '[' {
            let start = i;
            let mut end_bracket = None;
            let mut j = i + 1;
            while j < n {
                if chars[j] == ']' {
                    end_bracket = Some(j);
                    break;
                }
                j += 1;
            }
            if let Some(brk) = end_bracket {
                if brk + 1 < n && chars[brk + 1] == '(' {
                    let mut end_paren = None;
                    let mut k = brk + 2;
                    while k < n {
                        if chars[k] == ')' {
                            end_paren = Some(k);
                            break;
                        }
                        k += 1;
                    }
                    if let Some(par) = end_paren {
                        let link_text: String = chars[start..=par].iter().collect();
                        let link_color = Color32::from_rgb(97, 175, 239);
                        job.append(
                            &link_text,
                            0.0,
                            TextFormat::simple(mono_font.clone(), link_color),
                        );
                        i = par + 1;
                        continue;
                    }
                }
            }
        }

        // Bold: **text**
        if chars[i] == '*' && i + 1 < n && chars[i + 1] == '*' {
            let start = i;
            i += 2;
            let mut found_bold = false;
            while i + 1 < n {
                if chars[i] == '*' && chars[i + 1] == '*' {
                    i += 2;
                    found_bold = true;
                    break;
                }
                i += 1;
            }
            if found_bold {
                let bold_text: String = chars[start..i].iter().collect();
                job.append(
                    &bold_text,
                    0.0,
                    TextFormat::simple(mono_font.clone(), theme.highlight),
                );
                continue;
            } else {
                i = start;
            }
        }

        // Plain char
        job.append(
            &chars[i].to_string(),
            0.0,
            TextFormat::simple(mono_font.clone(), theme.text),
        );
        i += 1;
    }

    job
}
