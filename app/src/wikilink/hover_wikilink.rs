//! Sleek read-only Wikilink hover popup with full markdown preview and direct open action.

use crate::ui::theme::Theme;
use crate::wikilink::resolve_wikilink;
use core::Note;
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Pos2, Rect, Stroke};

/// State for the active hover wikilink popup.
#[derive(Debug, Clone, Default)]
pub struct HoverWikiLinkState {
    /// Active wikilink target under mouse
    pub target: Option<String>,
    /// Resolved target note ID if present in notes_list
    pub target_note_id: Option<i64>,
    /// Note title or path
    pub target_title: String,
    /// Note body markdown content
    pub target_body: String,
    /// Anchor position on screen where link is located
    pub anchor_pos: Pos2,
    /// Vertical scroll offset for preview
    pub scroll_y: f32,
    /// Time when hover started
    pub hover_start: f64,
    /// Bounding rectangle of the visible floating popup
    pub popup_rect: Option<Rect>,
    /// True if mouse is inside the popup itself
    pub is_mouse_inside_popup: bool,
    /// Candidate target being hovered during dwell delay
    pub pending_target: Option<String>,
    /// Anchor for candidate target
    pub pending_anchor: Pos2,
    /// Timestamp when hover on pending target began
    pub pending_since: f64,
    /// Suppress re-opening until mouse leaves the current link
    pub dismissed_target: Option<String>,
    /// Last recorded pointer position to detect actual mouse motion
    pub last_pointer_pos: Option<Pos2>,
    /// Last timestamp when hover or bridge was active (for graceful bridge transition)
    pub last_hover_time: f64,
}

impl HoverWikiLinkState {
    pub fn is_active(&self) -> bool {
        self.target.is_some()
    }

    pub fn clear(&mut self) {
        self.target = None;
        self.target_note_id = None;
        self.target_title.clear();
        self.target_body.clear();
        self.popup_rect = None;
        self.is_mouse_inside_popup = false;
        self.scroll_y = 0.0;
        self.pending_target = None;
    }

    pub fn dismiss(&mut self) {
        self.dismissed_target = self.target.clone().or_else(|| self.pending_target.clone());
        self.clear();
    }

    pub fn update_hover(&mut self, target: &str, anchor: Pos2, notes: &[Note], now: f64) {
        // If user explicitly dismissed this target, stay dismissed until mouse leaves it
        if self.dismissed_target.as_deref() == Some(target) {
            return;
        }

        self.last_hover_time = now;

        // If already active on this target, update anchor pos
        if self.target.as_deref() == Some(target) {
            self.anchor_pos = anchor;
            return;
        }

        // If target changed, reset dwell timer
        if self.pending_target.as_deref() != Some(target) {
            self.pending_target = Some(target.to_string());
            self.pending_anchor = anchor;
            self.pending_since = now;
            return;
        }

        // Dwell check: Snappy 150ms hover required before showing popup
        if now - self.pending_since < 0.15 {
            return;
        }

        // Promote candidate to active popup
        self.target = Some(target.to_string());
        self.anchor_pos = anchor;
        self.hover_start = now;
        self.scroll_y = 0.0;
        self.pending_target = None;

        if let Some(note) = resolve_wikilink(target, notes) {
            self.target_note_id = Some(note.id);
            self.target_title = note.topic.clone();
            self.target_body = note.body.clone();
        } else {
            self.target_note_id = None;
            self.target_title = target.to_string();
            self.target_body = "*Note does not exist yet. Click the link icon above to create it.*".to_string();
        }
    }
}

pub enum HoverWikiLinkAction {
    OpenNote { id: Option<i64>, title: String },
}

/// Renders sleek floating Wikilink preview card.
pub fn render_hover_wikilink_popup(
    ui: &egui::Ui,
    painter: &egui::Painter,
    state: &mut HoverWikiLinkState,
    theme: &Theme,
    font_size: f32,
    window_bounds: Rect,
) -> Option<HoverWikiLinkAction> {
    if !state.is_active() {
        return None;
    }

    // Escape key closes hover preview immediately
    let esc = ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
    if esc {
        state.dismiss();
        return None;
    }

    let pointer_pos = ui.input(|i| i.pointer.hover_pos().or_else(|| i.pointer.interact_pos()));
    let primary_clicked = ui.input(|i| i.pointer.primary_clicked());

    let popup_w: f32 = 540.0f32.min(window_bounds.width() - 40.0);
    let popup_h: f32 = 360.0f32.min(window_bounds.height() - 60.0);

    // Position popup right below or above the anchor with zero gap
    let mut top_left = pos2(
        (state.anchor_pos.x - 12.0).clamp(window_bounds.min.x + 10.0, (window_bounds.max.x - popup_w - 10.0).max(window_bounds.min.x + 10.0)),
        state.anchor_pos.y + 2.0,
    );
    if top_left.y + popup_h > window_bounds.max.y - 30.0 {
        top_left.y = (state.anchor_pos.y - popup_h - 22.0).max(window_bounds.min.y + 35.0);
    }

    let popup_rect = Rect::from_min_size(top_left, vec2(popup_w, popup_h));
    state.popup_rect = Some(popup_rect);

    // Mouse inside popup check
    state.is_mouse_inside_popup = pointer_pos.map_or(false, |p| popup_rect.contains(p));

    // Clicking outside dismisses popup immediately
    if primary_clicked && !state.is_mouse_inside_popup {
        state.dismiss();
        return None;
    }

    // Outer shadow / glass surface
    let shadow_color = Color32::from_black_alpha(65);
    painter.rect_filled(popup_rect.translate(vec2(0.0, 4.0)), 8.0, shadow_color);
    painter.rect(
        popup_rect,
        8.0,
        theme.surface(),
        Stroke::new(1.0_f32, theme.border()),
        egui::StrokeKind::Inside,
    );

    let header_h = 34.0;
    let header_rect = Rect::from_min_max(popup_rect.min, pos2(popup_rect.max.x, popup_rect.min.y + header_h));

    // Header subtle divider
    painter.line_segment(
        [pos2(header_rect.min.x, header_rect.max.y), pos2(header_rect.max.x, header_rect.max.y)],
        Stroke::new(1.0_f32, theme.border()),
    );

    // Left: Document / Folder icon
    let is_nested = state.target_title.contains('/') || state.target_title.contains('\\');
    let icon_name = if is_nested { "folder" } else { "doc" };
    let icon_rect = Rect::from_center_size(pos2(header_rect.min.x + 18.0, header_rect.center().y), vec2(14.0, 14.0));
    crate::ui_components::render_vector_icon(painter, icon_name, icon_rect, theme.accent);

    // Handle large file names safely without panicking on UTF-8 boundaries
    let avail_title_w = header_rect.width() - 95.0;
    let max_title_chars = ((avail_title_w / 7.2).max(12.0)) as usize;
    let title_chars: Vec<char> = state.target_title.chars().collect();
    let title_display = if title_chars.len() > max_title_chars {
        let take = max_title_chars.saturating_sub(3);
        format!("{}...", title_chars[..take].iter().collect::<String>())
    } else {
        state.target_title.clone()
    };

    painter.text(
        pos2(header_rect.min.x + 34.0, header_rect.center().y),
        Align2::LEFT_CENTER,
        title_display,
        FontId::monospace(12.0),
        theme.text,
    );

    // Right action buttons: Link / Open Icon and Close "x" Icon
    let btn_size = 22.0;

    // 1. Open Note button
    let open_btn_rect = Rect::from_center_size(
        pos2(header_rect.max.x - 42.0, header_rect.center().y),
        vec2(btn_size, btn_size),
    );
    let is_open_hovered = pointer_pos.map_or(false, |p| open_btn_rect.contains(p));
    let link_color = if is_open_hovered { theme.accent } else { theme.muted };
    let link_c = open_btn_rect.center();
    let stroke = Stroke::new(1.3_f32, link_color);
    painter.line_segment([pos2(link_c.x - 4.0, link_c.y + 4.0), pos2(link_c.x + 4.0, link_c.y - 4.0)], stroke);
    painter.circle_stroke(pos2(link_c.x - 2.2, link_c.y + 2.2), 2.2, stroke);
    painter.circle_stroke(pos2(link_c.x + 2.2, link_c.y - 2.2), 2.2, stroke);

    // 2. Close "x" button
    let close_btn_rect = Rect::from_center_size(
        pos2(header_rect.max.x - 18.0, header_rect.center().y),
        vec2(btn_size, btn_size),
    );
    let is_close_hovered = pointer_pos.map_or(false, |p| close_btn_rect.contains(p));
    let close_color = if is_close_hovered { theme.accent } else { theme.muted };
    let close_c = close_btn_rect.center();
    let x_stroke = Stroke::new(1.3_f32, close_color);
    painter.line_segment([pos2(close_c.x - 3.5, close_c.y - 3.5), pos2(close_c.x + 3.5, close_c.y + 3.5)], x_stroke);
    painter.line_segment([pos2(close_c.x + 3.5, close_c.y - 3.5), pos2(close_c.x - 3.5, close_c.y + 3.5)], x_stroke);

    if is_close_hovered && primary_clicked {
        state.dismiss();
        return None;
    }

    let mut action = None;
    if is_open_hovered && primary_clicked {
        action = Some(HoverWikiLinkAction::OpenNote {
            id: state.target_note_id,
            title: state.target_title.clone(),
        });
        state.clear();
        return action;
    }

    // Scrollable Preview Body: handles tables, code blocks, typography
    let body_rect = Rect::from_min_max(
        pos2(popup_rect.min.x + 10.0, header_rect.max.y + 6.0),
        pos2(popup_rect.max.x - 10.0, popup_rect.max.y - 6.0),
    );

    // Preview scroll handling (mouse wheel and keyboard arrow/ctrl+j down)
    // Consumes scroll deltas so background editor NEVER scrolls!
    if state.is_mouse_inside_popup {
        let scroll_delta = ui.input_mut(|i| {
            let delta = if i.raw_scroll_delta.y.abs() > 0.0 {
                i.raw_scroll_delta.y
            } else {
                i.smooth_scroll_delta.y
            };
            i.raw_scroll_delta = egui::Vec2::ZERO;
            i.smooth_scroll_delta = egui::Vec2::ZERO;
            delta
        });
        if scroll_delta.abs() > 0.0 {
            state.scroll_y = (state.scroll_y - scroll_delta).max(0.0);
        }

        let (down, up) = ui.input_mut(|i| {
            let d = i.key_pressed(egui::Key::ArrowDown) || i.key_pressed(egui::Key::PageDown) || (i.modifiers.ctrl && i.key_pressed(egui::Key::J));
            let u = i.key_pressed(egui::Key::ArrowUp) || i.key_pressed(egui::Key::PageUp) || (i.modifiers.ctrl && i.key_pressed(egui::Key::K));
            (d, u)
        });
        if down {
            state.scroll_y += 35.0;
        }
        if up {
            state.scroll_y = (state.scroll_y - 35.0).max(0.0);
        }
    }

    let clip_painter = painter.with_clip_rect(body_rect);
    let _ = crate::view_editor::preview::render::render_markdown_view_inner(
        ui,
        &clip_painter,
        body_rect,
        &state.target_body,
        &mut state.scroll_y,
        theme,
        font_size, // Full comfortable font size matching editor typography
        false,     // no extra header bar
        false,     // allow scrolling
    );

    action
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Local;

    #[test]
    fn test_hover_wikilink_dwell_delay_and_dismissal() {
        let now = Local::now().naive_local();
        let notes = vec![
            Note { id: 10, topic: "Architecture".into(), body: "System design specs".into(), struggled_with: None, created_at: now },
        ];

        let mut state = HoverWikiLinkState::default();

        // 1. Mouse enters the link
        state.update_hover("Architecture", pos2(100.0, 100.0), &notes, 1.0);
        assert!(!state.is_active(), "Should not be active before dwell delay of 150ms");
        assert_eq!(state.pending_target.as_deref(), Some("Architecture"));

        // 2. Before 150ms passes (e.g. at 1.10s -> 100ms)
        state.update_hover("Architecture", pos2(100.0, 100.0), &notes, 1.10);
        assert!(!state.is_active());

        // 3. After 150ms passes (at 1.20s -> 200ms)
        state.update_hover("Architecture", pos2(100.0, 100.0), &notes, 1.20);
        assert!(state.is_active(), "Should become active after 150ms dwell");
        assert_eq!(state.target.as_deref(), Some("Architecture"));
        assert_eq!(state.target_note_id, Some(10));
        assert_eq!(state.target_body, "System design specs");

        // 4. User dismisses popup (e.g. presses Escape or clicks close)
        state.dismiss();
        assert!(!state.is_active());
        assert_eq!(state.dismissed_target.as_deref(), Some("Architecture"));

        // 5. As long as mouse is still on this link, it remains dismissed
        state.update_hover("Architecture", pos2(100.0, 100.0), &notes, 1.30);
        assert!(!state.is_active(), "Must stay dismissed until mouse leaves link");

        // 6. Mouse moves away from link
        state.dismissed_target = None;
        state.clear();

        // 7. Now mouse moves to a link
        state.update_hover("Architecture", pos2(100.0, 100.0), &notes, 2.0);
        state.update_hover("Architecture", pos2(100.0, 100.0), &notes, 2.20);
        assert!(state.is_active(), "Can hover again after moving away");
    }
}

