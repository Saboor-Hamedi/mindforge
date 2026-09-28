//! Settings modal tab navigation.

use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Pos2, Rect, Stroke};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingTab {
    Carets,
    Fonts,
    EditorMode,
    Sounds,
    Theme,
    Shortcuts,
    Keybindings,
    Backup,
    Updates,
    Ai,
    LunaLine,
}

impl SettingTab {
    pub const ALL: [SettingTab; 11] = [
        SettingTab::Carets,
        SettingTab::Fonts,
        SettingTab::EditorMode,
        SettingTab::Sounds,
        SettingTab::Theme,
        SettingTab::Shortcuts,
        SettingTab::Keybindings,
        SettingTab::LunaLine,
        SettingTab::Backup,
        SettingTab::Updates,
        SettingTab::Ai,
    ];

    pub fn title(&self) -> &'static str {
        match self {
            SettingTab::Carets => "Carets",
            SettingTab::Fonts => "Fonts & Type",
            SettingTab::EditorMode => "Editor Mode",
            SettingTab::Sounds => "Sounds",
            SettingTab::Theme => "Theme",
            SettingTab::Shortcuts => "Shortcuts",
            SettingTab::Keybindings => "Keybindings",
            SettingTab::LunaLine => "LunaLine",
            SettingTab::Backup => "Backup",
            SettingTab::Updates => "Updates",
            SettingTab::Ai => "AI Agent",
        }
    }

    pub fn draw_icon(&self, painter: &egui::Painter, center: Pos2, color: Color32) {
        let stroke = Stroke::new(1.3, color);
        match self {
            SettingTab::Carets => {
                // Sleek I-beam caret cursor
                painter.line_segment([pos2(center.x - 3.5, center.y - 6.0), pos2(center.x + 3.5, center.y - 6.0)], stroke);
                painter.line_segment([pos2(center.x, center.y - 6.0), pos2(center.x, center.y + 6.0)], stroke);
                painter.line_segment([pos2(center.x - 3.5, center.y + 6.0), pos2(center.x + 3.5, center.y + 6.0)], stroke);
            }
            SettingTab::Fonts => {
                // Typographic "Aa" glyph
                painter.text(center, Align2::CENTER_CENTER, "Aa", FontId::monospace(13.0), color);
            }
            SettingTab::EditorMode => {
                // Lightning bolt
                let p1 = pos2(center.x + 1.0, center.y - 6.5);
                let p2 = pos2(center.x - 4.0, center.y + 0.5);
                let p3 = pos2(center.x + 0.0, center.y + 0.5);
                let p4 = pos2(center.x - 1.0, center.y + 6.5);
                let p5 = pos2(center.x + 4.0, center.y - 0.5);
                let p6 = pos2(center.x - 0.0, center.y - 0.5);
                painter.line_segment([p1, p2], stroke);
                painter.line_segment([p2, p3], stroke);
                painter.line_segment([p3, p4], stroke);
                painter.line_segment([p4, p5], stroke);
                painter.line_segment([p5, p6], stroke);
                painter.line_segment([p6, p1], stroke);
            }
            SettingTab::Sounds => {
                // Speaker icon
                let spk = Rect::from_center_size(pos2(center.x - 3.0, center.y), vec2(4.0, 6.0));
                painter.rect_stroke(spk, 1.0, stroke, egui::StrokeKind::Inside);
                painter.line_segment([pos2(center.x - 1.0, center.y - 3.0), pos2(center.x + 3.0, center.y - 6.0)], stroke);
                painter.line_segment([pos2(center.x + 3.0, center.y - 6.0), pos2(center.x + 3.0, center.y + 6.0)], stroke);
                painter.line_segment([pos2(center.x + 3.0, center.y + 6.0), pos2(center.x - 1.0, center.y + 3.0)], stroke);
            }
            SettingTab::Theme => {
                // Palette circle
                painter.circle_stroke(center, 6.0, stroke);
                painter.circle_filled(pos2(center.x - 2.5, center.y - 2.0), 1.3, color);
                painter.circle_filled(pos2(center.x + 2.5, center.y - 2.0), 1.3, color);
                painter.circle_filled(pos2(center.x, center.y + 2.5), 1.3, color);
            }
            SettingTab::Shortcuts => {
                // Keyboard frame
                let kb = Rect::from_center_size(center, vec2(15.0, 10.0));
                painter.rect_stroke(kb, 2.0, stroke, egui::StrokeKind::Inside);
                painter.circle_filled(pos2(center.x - 3.5, center.y - 1.5), 0.9, color);
                painter.circle_filled(pos2(center.x, center.y - 1.5), 0.9, color);
                painter.circle_filled(pos2(center.x + 3.5, center.y - 1.5), 0.9, color);
                painter.line_segment([pos2(center.x - 3.5, center.y + 2.0), pos2(center.x + 3.5, center.y + 2.0)], stroke);
            }
            SettingTab::Keybindings => {
                // Sliders icon
                painter.line_segment([pos2(center.x - 6.0, center.y - 3.0), pos2(center.x + 6.0, center.y - 3.0)], stroke);
                painter.circle_filled(pos2(center.x - 2.0, center.y - 3.0), 1.8, color);
                painter.line_segment([pos2(center.x - 6.0, center.y + 3.0), pos2(center.x + 6.0, center.y + 3.0)], stroke);
                painter.circle_filled(pos2(center.x + 2.0, center.y + 3.0), 1.8, color);
            }
            SettingTab::Backup => {
                // Floppy disk icon
                let dsk = Rect::from_center_size(center, vec2(12.0, 12.0));
                painter.rect_stroke(dsk, 2.0, stroke, egui::StrokeKind::Inside);
                let inner = Rect::from_min_size(pos2(center.x - 3.0, center.y - 5.0), vec2(6.0, 4.0));
                painter.rect_filled(inner, 1.0, color);
            }
            SettingTab::Updates => {
                // Sync arrows
                painter.circle_stroke(center, 5.0, stroke);
                painter.circle_filled(pos2(center.x + 4.0, center.y - 2.0), 1.4, color);
            }
            SettingTab::Ai => {
                // Diamond spark ◈
                painter.line_segment([pos2(center.x, center.y - 6.0), pos2(center.x + 5.0, center.y)], stroke);
                painter.line_segment([pos2(center.x + 5.0, center.y), pos2(center.x, center.y + 6.0)], stroke);
                painter.line_segment([pos2(center.x, center.y + 6.0), pos2(center.x - 5.0, center.y)], stroke);
                painter.line_segment([pos2(center.x - 5.0, center.y), pos2(center.x, center.y - 6.0)], stroke);
            }
            SettingTab::LunaLine => {
                // Sleek statusline bar with left & right capsule segments
                let bar = Rect::from_center_size(center, vec2(14.0, 8.0));
                painter.rect_stroke(bar, 2.0, stroke, egui::StrokeKind::Inside);
                painter.rect_filled(Rect::from_min_size(pos2(bar.min.x + 1.5, bar.min.y + 1.5), vec2(3.5, 5.0)), 1.0, color);
                painter.rect_filled(Rect::from_min_size(pos2(bar.max.x - 5.0, bar.min.y + 1.5), vec2(3.5, 5.0)), 1.0, color);
            }
        }
    }
}

pub fn render_setting_tabs(
    ui: &egui::Ui,
    painter: &egui::Painter,
    tabs_rect: Rect,
    active_tab: &mut SettingTab,
    theme: &crate::ui::theme::Theme,
) {
    // Left tab sidebar background with 8px rounded left corners
    painter.rect(
        tabs_rect,
        egui::CornerRadius {
            nw: 8,
            sw: 8,
            ne: 0,
            se: 0,
        },
        theme.sidebar_bg(),
        Stroke::NONE,
        egui::StrokeKind::Inside,
    );
    painter.line_segment(
        [tabs_rect.right_top(), tabs_rect.right_bottom()],
        Stroke::new(1.0, theme.border()),
    );

    let start_y = tabs_rect.min.y + 24.0;
    let tab_w = tabs_rect.width() - 16.0;
    let tab_h = 36.0;

    // "PREFERENCES" header
    painter.text(
        pos2(tabs_rect.min.x + 16.0, start_y),
        Align2::LEFT_TOP,
        "PREFERENCES",
        FontId::proportional(11.0),
        theme.muted,
    );

    for (i, &tab) in SettingTab::ALL.iter().enumerate() {
        let item_rect = Rect::from_min_size(
            pos2(tabs_rect.min.x + 8.0, start_y + 26.0 + i as f32 * (tab_h + 3.0)),
            vec2(tab_w, tab_h),
        );
        let is_sel = *active_tab == tab;
        let hovered = ui.rect_contains_pointer(item_rect);

        // Clean selection: left accent bar only, no filled background card
        if is_sel {
            let bar = Rect::from_min_size(
                pos2(item_rect.min.x + 2.0, item_rect.min.y + 7.0),
                vec2(3.5, item_rect.height() - 14.0),
            );
            painter.rect_filled(bar, 2.0, theme.accent);
        } else if hovered {
            painter.rect_filled(
                item_rect,
                6.0,
                if theme.is_light() {
                    Color32::from_rgba_unmultiplied(0, 0, 0, 8)
                } else {
                    Color32::from_rgba_unmultiplied(255, 255, 255, 8)
                },
            );
        }

        if hovered && ui.input(|inp| inp.pointer.primary_clicked()) {
            *active_tab = tab;
        }

        let (icon_color, label_color) = if is_sel {
            (theme.accent, theme.text)
        } else if hovered {
            (theme.accent, theme.text)
        } else {
            (theme.muted, theme.muted)
        };

        // Draw crisp vector icon (no emoji)
        let icon_center = pos2(item_rect.min.x + 20.0, item_rect.center().y);
        tab.draw_icon(painter, icon_center, icon_color);

        // Label
        painter.text(
            pos2(item_rect.min.x + 38.0, item_rect.center().y),
            Align2::LEFT_CENTER,
            tab.title(),
            FontId::proportional(13.0),
            label_color,
        );
    }
}
