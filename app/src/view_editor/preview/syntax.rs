//! Syntax highlighting tokenizer for preview code blocks.

use crate::ui::theme::Theme;
use eframe::egui::text::LayoutJob;
use eframe::egui::{Color32, TextFormat};

/// Tokenizes a code line into rich multi-color syntax highlighting.
pub fn highlight_code_line(
    line: &str,
    lang: &str,
    font_size: f32,
    theme: &Theme,
) -> LayoutJob {
    highlight_code_line_impl(line, lang, font_size, theme, true)
}

pub fn highlight_editor_code_line(
    line: &str,
    lang: &str,
    font_size: f32,
    theme: &Theme,
) -> LayoutJob {
    highlight_code_line_impl(line, lang, font_size, theme, false)
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
    let is_python = matches!(language.as_str(), "python" | "py");
    let is_bash = lang.eq_ignore_ascii_case("bash") || lang.eq_ignore_ascii_case("sh") || lang.eq_ignore_ascii_case("shell");
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
                Color32::from_rgba_unmultiplied(theme.muted.r(), theme.muted.g(), theme.muted.b(), 170),
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
            job.append(&str_val, 0.0, TextFormat::simple(mono_font.clone(), str_color));
            continue;
        }

        // 3. Numbers (integers, floats, hex)
        if chars[i].is_ascii_digit() && (i == 0 || (!chars[i - 1].is_alphanumeric() && chars[i - 1] != '_')) {
            let start = i;
            while i < n && (chars[i].is_ascii_alphanumeric() || chars[i] == '.' || chars[i] == '_') {
                i += 1;
            }
            let num_val: String = chars[start..i].iter().collect();
            // Numbers: amber / orange
            let num_color = Color32::from_rgb(209, 154, 102);
            job.append(&num_val, 0.0, TextFormat::simple(mono_font.clone(), num_color));
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

            let is_sql_keyword = is_sql && matches!(
                word.to_ascii_lowercase().as_str(),
                "select" | "from" | "where" | "join" | "left" | "right" | "inner" | "outer"
                    | "on" | "as" | "insert" | "into" | "values" | "update" | "set"
                    | "delete" | "create" | "table" | "index" | "drop" | "alter" | "group"
                    | "by" | "order" | "having" | "limit" | "offset" | "and" | "or" | "not"
                    | "null" | "primary" | "key" | "foreign" | "references" | "distinct"
                    | "union" | "all" | "case" | "when" | "then" | "else" | "end"
            );
            let is_keyword = is_sql_keyword || match word.as_str() {
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
                "true" | "false" | "True" | "False" | "None" | "Some" | "Ok" | "Err"
                | "null" | "undefined" | "NULL" | "Null" => true,
                _ => false,
            };

            let is_type = !is_keyword && (
                word.starts_with(|c: char| c.is_ascii_uppercase())
                || matches!(word.as_str(), "bool" | "u8" | "u16" | "u32" | "u64" | "u128" | "usize"
                    | "i8" | "i16" | "i32" | "i64" | "i128" | "isize"
                    | "f32" | "f64" | "char" | "str" | "int" | "float" | "dict" | "list" | "number" | "string")
            );

            let color = if is_keyword {
                theme.accent
            } else if is_bool_or_none {
                Color32::from_rgb(209, 154, 102) // amber
            } else if is_type {
                Color32::from_rgb(229, 192, 123) // warm gold
            } else if word.ends_with('!') {
                Color32::from_rgb(97, 175, 239)  // cyan/blue for macros
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
            '=' | '+' | '-' | '*' | '/' | '%' | '&' | '|' | '^' | '!' | '<' | '>' | '~' | '?' | ':' => {
                Color32::from_rgb(97, 175, 239) // vibrant operator
            }
            '{' | '}' | '(' | ')' | '[' | ']' => {
                Color32::from_rgb(224, 108, 117) // coral bracket
            }
            _ => Color32::from_rgba_unmultiplied(theme.muted.r(), theme.muted.g(), theme.muted.b(), 180),
        };
        job.append(&punc_char.to_string(), 0.0, TextFormat::simple(mono_font.clone(), punc_color));
        i += 1;
    }

    job
}
