//! Unified code block card container rendering and metrics.
//!
//! Renders a single cohesive elevated surface card around the entire code block
//! (from opening fence to closing fence) with rounded corners and a language badge pill,
//! perfectly matching the look of the Markdown preview and eliminating broken line strips.

use super::super::charmap::{append_run_and_map, CharMapBuilder};
use crate::ui::theme::Theme;
use eframe::egui::text::LayoutJob;
use eframe::egui::{pos2, vec2, Align2, Color32, CornerRadius, FontId, Painter, Rect, Stroke, StrokeKind, TextFormat};

/// Returns font and line height metrics for code fence lines.
pub fn code_metrics(base_font_size: f32, is_closing: bool, is_active: bool) -> (FontId, f32) {
    if !is_active {
        if is_closing {
            (crate::services::font_manager::editor_font_id(base_font_size * 0.7), 12.0)
        } else {
            (crate::services::font_manager::editor_font_id(base_font_size * 0.7), 28.0)
        }
    } else {
        (crate::services::font_manager::editor_font_id(base_font_size * 0.95), (base_font_size * 1.55).round())
    }
}

/// Tokenizes and highlights code characters into `job` and `charmap`,
/// mirroring the multi-color syntax highlighting of `preview.rs::highlight_code_line`
/// while strictly preserving 1:1 character mapping invariants.
pub fn highlight_code_chars(
    chars: &[char],
    lang: Option<&str>,
    font_size: f32,
    theme: &Theme,
    job: &mut LayoutJob,
    charmap: &mut CharMapBuilder,
) {
    let n = chars.len();
    let mono_font = crate::services::font_manager::editor_font_id(font_size);
    let mut i = 0;

    let lang_str = lang.unwrap_or("");
    let is_python = lang_str.eq_ignore_ascii_case("python") || lang_str.eq_ignore_ascii_case("py");
    let is_bash = lang_str.eq_ignore_ascii_case("bash") || lang_str.eq_ignore_ascii_case("sh") || lang_str.eq_ignore_ascii_case("shell");

    while i < n {
        // 1. Comments
        if (chars[i] == '/' && i + 1 < n && chars[i + 1] == '/')
            || ((is_python || is_bash) && chars[i] == '#')
        {
            let mut fmt = TextFormat::simple(
                mono_font.clone(),
                Color32::from_rgba_unmultiplied(theme.muted.r(), theme.muted.g(), theme.muted.b(), 170),
            );
            fmt.italics = true;
            append_run_and_map(job, charmap, chars, i..n, fmt);
            break;
        }

        // 2. Strings ("..." or '...')
        if chars[i] == '"' || chars[i] == '\'' {
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
            let str_color = Color32::from_rgb(152, 195, 121); // emerald / lime green
            let fmt = TextFormat::simple(mono_font.clone(), str_color);
            append_run_and_map(job, charmap, chars, start..i, fmt);
            continue;
        }

        // 3. Numbers (integers, floats, hex)
        if chars[i].is_ascii_digit() && (i == 0 || (!chars[i - 1].is_alphanumeric() && chars[i - 1] != '_')) {
            let start = i;
            while i < n && (chars[i].is_ascii_alphanumeric() || chars[i] == '.' || chars[i] == '_') {
                i += 1;
            }
            let num_color = Color32::from_rgb(209, 154, 102); // amber / orange
            let fmt = TextFormat::simple(mono_font.clone(), num_color);
            append_run_and_map(job, charmap, chars, start..i, fmt);
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

            let is_keyword = match word.as_str() {
                // Rust
                "fn" | "let" | "mut" | "pub" | "struct" | "enum" | "impl" | "match" | "if" | "else"
                | "return" | "use" | "mod" | "trait" | "type" | "where" | "async" | "await" | "for"
                | "in" | "while" | "loop" | "const" | "static" | "crate" | "super" | "self"
                // Python / JS / General
                | "def" | "class" | "import" | "from" | "as" | "with" | "yield" | "pass" | "lambda"
                | "function" | "var" | "export" | "try" | "catch" | "finally" | "new" | "this"
                | "typeof" | "instanceof" | "echo" | "sudo" => true,
                _ => false,
            };

            let is_bool_or_none = match word.as_str() {
                "true" | "false" | "None" | "Some" | "Ok" | "Err" | "null" | "undefined" => true,
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

            let fmt = TextFormat::simple(mono_font.clone(), color);
            append_run_and_map(job, charmap, chars, start..i, fmt);
            continue;
        }

        // 5. Punctuation & Operators
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
        let fmt = TextFormat::simple(mono_font.clone(), punc_color);
        append_run_and_map(job, charmap, chars, i..i + 1, fmt);
        i += 1;
    }
}

/// Renders a single cohesive elevated container card around an entire code block
/// with a distinct, sleek header bar, subtle divider hairline, and language badge pill.
pub fn render_code_block_card(
    painter: &Painter,
    top_y: f32,
    bottom_y: f32,
    text_left: f32,
    content_right: f32,
    theme: &Theme,
    fence_lang: Option<&str>,
) {
    let card_left = text_left;
    let card_right = content_right;
    let card_rect = Rect::from_min_max(
        pos2(card_left, top_y),
        pos2(card_right, bottom_y),
    );

    // 1. Single cohesive elevated container surface with subtle rounded border
    painter.rect(
        card_rect,
        5.0,
        theme.surface(),
        Stroke::new(1.0_f32, theme.border()),
        StrokeKind::Inside,
    );

    // 2. Distinct, sleek header bar at the top of the code card (28px height)
    let header_h = 28.0;
    if bottom_y >= top_y + header_h {
        let header_rect = Rect::from_min_max(
            pos2(card_left, top_y),
            pos2(card_right, top_y + header_h),
        );
        let header_bg = Color32::from_rgba_unmultiplied(
            theme.border().r(),
            theme.border().g(),
            theme.border().b(),
            if theme.is_light() { 22 } else { 35 },
        );
        painter.rect_filled(
            header_rect,
            CornerRadius { nw: 5, ne: 5, sw: 0, se: 0 },
            header_bg,
        );

        // Subtle hairline divider separating header bar from code area
        let divider_color = Color32::from_rgba_unmultiplied(
            theme.border().r(),
            theme.border().g(),
            theme.border().b(),
            if theme.is_light() { 65 } else { 75 },
        );
        painter.line_segment(
            [pos2(card_left, top_y + header_h), pos2(card_right, top_y + header_h)],
            Stroke::new(0.8_f32, divider_color),
        );
    }

    // 3. Language badge on the left of the header bar
    if let Some(lang) = fence_lang {
        if !lang.is_empty() {
            let label_font = FontId::monospace(10.0);
            let label_pos = pos2(card_rect.min.x + 14.0, top_y + 14.0);
            painter.text(
                label_pos,
                Align2::LEFT_CENTER,
                lang.to_uppercase(),
                label_font,
                theme.text,
            );
        }
    }
}

/// Returns the hit-test and render rectangle for a code block copy button.
pub fn code_block_copy_button_rect(card_right: f32, top_y: f32) -> Rect {
    let btn_w = 60.0;
    let btn_h = 20.0;
    Rect::from_min_size(
        pos2(card_right - btn_w - 12.0, top_y + 4.0),
        vec2(btn_w, btn_h),
    )
}
