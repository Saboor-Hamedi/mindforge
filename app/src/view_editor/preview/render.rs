//! Rich markdown live preview renderer with full multi-language syntax highlighting,
//! custom vector checkboxes, formatted tables, blockquotes, and smooth typography.

use crate::ui::theme::Theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Rect, Stroke};

use super::parser::{build_inline_job, parse_markdown, MdBlock};

/// Renders parsed markdown blocks inside `rect` with mouse-wheel scrolling, top header with close button, and polished styling.
/// Returns `true` if the user clicked the close button.
pub fn render_markdown_preview(
    ui: &egui::Ui,
    painter: &egui::Painter,
    rect: Rect,
    content: &str,
    scroll_y: &mut f32,
    theme: &Theme,
    font_size: f32,
    block_scroll: bool,
) -> bool {
    render_markdown_view_inner(
        ui,
        painter,
        rect,
        content,
        scroll_y,
        theme,
        font_size,
        false,
        block_scroll,
    )
}

/// Renders parsed markdown blocks inside `rect` as a full-height document without any nested header bar.
/// Ideal for tabs such as the Quick Start Help guide.
pub fn render_markdown_document(
    ui: &egui::Ui,
    painter: &egui::Painter,
    rect: Rect,
    content: &str,
    scroll_y: &mut f32,
    theme: &Theme,
    font_size: f32,
) {
    let _ = render_markdown_view_inner(
        ui, painter, rect, content, scroll_y, theme, font_size, false, false,
    );
}

pub fn render_markdown_view_inner(
    ui: &egui::Ui,
    painter: &egui::Painter,
    rect: Rect,
    content: &str,
    scroll_y: &mut f32,
    theme: &Theme,
    font_size: f32,
    show_header: bool,
    block_scroll: bool,
) -> bool {
    crate::services::font_manager::ensure_editor_font(ui.ctx());
    let mut close_clicked = false;

    let content_rect = if show_header {
        // Header bar (28px height)
        let header_h = 28.0;
        let header_rect = Rect::from_min_max(rect.min, pos2(rect.max.x, rect.min.y + header_h));

        // Header subtle translucent background & bottom border
        let header_bg = Color32::from_rgba_unmultiplied(
            theme.surface().r(),
            theme.surface().g(),
            theme.surface().b(),
            160,
        );
        painter.rect_filled(header_rect, 0.0, header_bg);
        painter.line_segment(
            [
                pos2(header_rect.min.x, header_rect.max.y),
                pos2(header_rect.max.x, header_rect.max.y),
            ],
            Stroke::new(1.0_f32, theme.border()),
        );

        // Header title
        painter.text(
            pos2(header_rect.min.x + 12.0, header_rect.center().y),
            Align2::LEFT_CENTER,
            "PREVIEW",
            FontId::proportional(font_size * 0.75),
            theme.muted,
        );

        // Unified sleek close button on top-right
        let btn_center = pos2(header_rect.max.x - 16.0, header_rect.center().y);
        if crate::ui_components::render_close_button(
            ui,
            painter,
            btn_center,
            20.0,
            theme,
            "preview_header_close",
        ) {
            close_clicked = true;
        }

        Rect::from_min_max(pos2(rect.min.x, rect.min.y + header_h), rect.max)
    } else {
        rect
    };

    let blocks = parse_markdown(content);

    // Generous, comfortable reading margins (target width ~680px for 60-80 character line length)
    let max_reading_w = 680.0f32;
    let pad_x = if content_rect.width() > max_reading_w + 64.0 {
        ((content_rect.width() - max_reading_w) * 0.5).round()
    } else {
        32.0f32.min(content_rect.width() * 0.08).max(18.0)
    };
    let pad_y = if show_header { 16.0 } else { 14.0 };
    let content_painter = painter.with_clip_rect(content_rect);
    let max_text_w = (content_rect.width() - pad_x * 2.0).max(60.0);

    // Mouse scroll handling inside preview pane (blocked when modals or floating AI assistant window are on top)
    if !block_scroll && ui.rect_contains_pointer(content_rect) {
        let delta = ui.input(|i| {
            if i.smooth_scroll_delta.y.abs() > 0.001 {
                i.smooth_scroll_delta.y
            } else {
                i.raw_scroll_delta.y * 0.5
            }
        });
        if delta != 0.0 {
            *scroll_y = (*scroll_y - delta).max(0.0);
        }
    }

    // Empty state placeholder (zoom-safe dynamic layout that never entangles on zoom)
    if blocks.is_empty() {
        let avail_w = (content_rect.width() - 32.0).max(80.0);

        let title_font = FontId::proportional(font_size * 1.05);
        let title_color =
            Color32::from_rgba_unmultiplied(theme.muted.r(), theme.muted.g(), theme.muted.b(), 130);
        let title_galley = painter.layout(
            "Nothing to preview yet".to_string(),
            title_font,
            title_color,
            avail_w,
        );

        let sub_font = FontId::proportional(font_size * 0.85);
        let sub_color =
            Color32::from_rgba_unmultiplied(theme.muted.r(), theme.muted.g(), theme.muted.b(), 90);
        let sub_galley = painter.layout(
            "Type markdown in the editor to see live rendering".to_string(),
            sub_font,
            sub_color,
            avail_w,
        );

        let title_h = title_galley.size().y;
        let sub_h = sub_galley.size().y;
        let gap = (font_size * 0.55).round().max(8.0);
        let total_h = title_h + gap + sub_h;

        let start_y = (content_rect.center().y - total_h * 0.5)
            .round()
            .max(content_rect.min.y + 10.0);

        let title_pos = pos2(
            (content_rect.center().x - title_galley.size().x * 0.5).round(),
            start_y,
        );
        content_painter.galley(title_pos, title_galley, title_color);

        let sub_pos = pos2(
            (content_rect.center().x - sub_galley.size().x * 0.5).round(),
            start_y + title_h + gap,
        );
        content_painter.galley(sub_pos, sub_galley, sub_color);

        return close_clicked;
    }

    let start_x = content_rect.min.x + pad_x;
    let mut current_y = content_rect.min.y + pad_y - *scroll_y;
    let mut code_block_idx: usize = 0;
    let mut table_idx: usize = 0;

    for (b_idx, block) in blocks.iter().enumerate() {
        match block {
            MdBlock::Heading1(text) => {
                current_y += super::heading::render_preview_heading(
                    painter,
                    &content_painter,
                    start_x,
                    current_y,
                    max_text_w,
                    font_size,
                    1,
                    text,
                    theme,
                    b_idx,
                    rect,
                );
            }
            MdBlock::Heading2(text) => {
                current_y += super::heading::render_preview_heading(
                    painter,
                    &content_painter,
                    start_x,
                    current_y,
                    max_text_w,
                    font_size,
                    2,
                    text,
                    theme,
                    b_idx,
                    rect,
                );
            }
            MdBlock::Heading3(text) => {
                current_y += super::heading::render_preview_heading(
                    painter,
                    &content_painter,
                    start_x,
                    current_y,
                    max_text_w,
                    font_size,
                    3,
                    text,
                    theme,
                    b_idx,
                    rect,
                );
            }
            MdBlock::Heading4(text) => {
                current_y += super::heading::render_preview_heading(
                    painter,
                    &content_painter,
                    start_x,
                    current_y,
                    max_text_w,
                    font_size,
                    4,
                    text,
                    theme,
                    b_idx,
                    rect,
                );
            }
            MdBlock::Paragraph(text) => {
                let job = build_inline_job(text, font_size, theme.text, theme, max_text_w);
                let galley = painter.layout_job(job);
                let text_h = galley.size().y;
                if current_y + text_h >= rect.min.y && current_y <= rect.max.y {
                    content_painter.galley(pos2(start_x, current_y), galley, theme.text);
                }
                current_y += text_h + 14.0;
            }
            MdBlock::Quote { depth, text } => {
                if b_idx > 0 {
                    current_y += 4.0;
                }
                current_y += super::blockquote::render_preview_blockquote(
                    painter,
                    &content_painter,
                    start_x,
                    current_y,
                    max_text_w,
                    font_size,
                    *depth,
                    text,
                    theme,
                    rect,
                );
            }
            MdBlock::ListItem {
                bullet,
                text,
                checked,
                indent_level,
            } => {
                let indent_offset = *indent_level as f32 * 18.0;
                let item_start_x = start_x + indent_offset;
                let available_w = (max_text_w - indent_offset).max(40.0);

                let job = build_inline_job(text, font_size, theme.text, theme, available_w - 28.0);
                let galley = painter.layout_job(job);
                let text_h = galley.size().y;
                let row_h = text_h.max(font_size * 1.35);

                if current_y + row_h >= rect.min.y && current_y <= rect.max.y {
                    if let Some(is_checked) = checked {
                        // Custom vector checkbox widget matching inline editor
                        let cb_size = 15.0;
                        let cb_y = current_y + 1.5;
                        let cb_rect =
                            Rect::from_min_size(pos2(item_start_x, cb_y), vec2(cb_size, cb_size));

                        // Render unified vector checkbox
                        crate::view_editor::inline::elements::render_task_checkbox(
                            &content_painter,
                            cb_rect,
                            *is_checked,
                            None,
                            theme,
                        );

                        if *is_checked {
                            // Text: dimmed with strike-through
                            let text_start = pos2(item_start_x + 24.0, current_y);
                            let text_dimmed = Color32::from_rgba_unmultiplied(
                                theme.text.r(),
                                theme.text.g(),
                                theme.text.b(),
                                140,
                            );
                            content_painter.galley(text_start, galley, text_dimmed);
                            let strike_y = current_y + text_h * 0.52;
                            content_painter.line_segment(
                                [
                                    pos2(text_start.x, strike_y),
                                    pos2(text_start.x + available_w - 28.0, strike_y),
                                ],
                                Stroke::new(
                                    1.0_f32,
                                    Color32::from_rgba_unmultiplied(
                                        theme.muted.r(),
                                        theme.muted.g(),
                                        theme.muted.b(),
                                        120,
                                    ),
                                ),
                            );
                        } else {
                            content_painter.galley(
                                pos2(item_start_x + 24.0, current_y),
                                galley,
                                theme.text,
                            );
                        }
                    } else {
                        // Standard bullet or numbered list
                        let bullet_font = crate::services::font_manager::editor_font_id(font_size);
                        let b_color = theme.text;
                        let bullet_w = if bullet.ends_with('.') {
                            (bullet.len() as f32 * font_size * 0.58).max(18.0)
                        } else {
                            16.0
                        };
                        content_painter.text(
                            pos2(item_start_x, current_y),
                            Align2::LEFT_TOP,
                            bullet,
                            bullet_font,
                            b_color,
                        );
                        content_painter.galley(
                            pos2(item_start_x + bullet_w + 6.0, current_y),
                            galley,
                            theme.text,
                        );
                    }
                }
                current_y += row_h + 6.0;
            }
            MdBlock::CodeBlock { lang, code } => {
                code_block_idx += 1;
                if b_idx > 0 {
                    current_y += 6.0;
                }
                current_y += super::code_block::render_preview_code_block(
                    ui,
                    painter,
                    &content_painter,
                    start_x,
                    current_y,
                    max_text_w,
                    font_size,
                    lang,
                    code,
                    theme,
                    code_block_idx,
                    rect,
                );
            }
            MdBlock::Table { headers, rows } => {
                table_idx += 1;
                if b_idx > 0 {
                    current_y += 8.0;
                }
                current_y += super::table::render_preview_table(
                    ui,
                    painter,
                    &content_painter,
                    start_x,
                    current_y,
                    max_text_w,
                    font_size,
                    headers,
                    rows,
                    theme,
                    table_idx,
                    rect,
                );
            }
            MdBlock::Rule => {
                if b_idx > 0 {
                    current_y += 6.0;
                }
                if current_y >= rect.min.y && current_y <= rect.max.y {
                    content_painter.line_segment(
                        [
                            pos2(start_x, current_y),
                            pos2(start_x + max_text_w, current_y),
                        ],
                        Stroke::new(1.0_f32, theme.border()),
                    );
                }
                current_y += 10.0;
            }
        }
    }

    // Clamp scroll with bottom breathing room aligned with editor body
    let total_h = (current_y + *scroll_y - (rect.min.y + pad_y)).max(0.0);
    let bottom_pad = 28.0;
    let max_scroll = (total_h + bottom_pad - rect.height()).max(0.0);
    *scroll_y = scroll_y.clamp(0.0, max_scroll);

    close_clicked
}
