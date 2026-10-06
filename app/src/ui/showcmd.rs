//! Standalone ShowCmd Keystroke & Command HUD Component.
//!
//! Tracks pending/executed Vim commands, search queries, and command-line
//! inputs, rendering them as a clean, borderless floating capsule card at
//! the bottom-center of the editor.
//!
//! Features:
//! - Auto-hides after 2 seconds of inactivity
//! - Truncates long text with ellipsis (max 32 chars)
//! - Color-coded by kind: Keystroke (muted), Command (accent), Search (highlight), Visual (green)
//! - Zero-allocation rendering when hidden

use eframe::egui::{self, Color32, FontId, Pos2, Rect, Stroke};

/// Maximum characters to display — beyond this the text is truncated with "…".
const MAX_DISPLAY_CHARS: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShowCmdKind {
    Keystroke,
    Command,
    Search,
    Visual,
}

#[derive(Clone, Debug)]
pub struct ShowCmdState {
    pub enabled: bool,
    pub text: String,
    pub kind: ShowCmdKind,
    pub last_change_time: f64,
    pub is_pending: bool,
}

impl Default for ShowCmdState {
    fn default() -> Self {
        Self::new(true)
    }
}

impl ShowCmdState {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            text: String::new(),
            kind: ShowCmdKind::Keystroke,
            last_change_time: 0.0,
            is_pending: false,
        }
    }

    // ── Internal ──────────────────────────────────────────────────────────────

    /// Truncates text to MAX_DISPLAY_CHARS chars, appending "…" if needed.
    fn truncate(text: &str) -> String {
        let char_count = text.chars().count();
        if char_count <= MAX_DISPLAY_CHARS {
            text.to_string()
        } else {
            let truncated: String = text.chars().take(MAX_DISPLAY_CHARS).collect();
            format!("{}…", truncated)
        }
    }

    // ── Public API ────────────────────────────────────────────────────────────

    /// Sets a pending partial command (e.g. "d", "40j", "ci", "\"a").
    /// Pass `is_visual = true` when called from Visual mode (sets VIS badge).
    pub fn set_pending(&mut self, text: &str, now: f64) {
        if !self.enabled {
            self.text.clear();
            return;
        }
        self.text = Self::truncate(text);
        // Infer kind from the text itself: visual pending starts with 'v'/'V'/'<'
        self.kind = infer_kind(text);
        self.last_change_time = now;
        self.is_pending = true;
    }

    /// Live-tracks command mode input (e.g. ":w", ":delete", ":set showcmd").
    pub fn set_command(&mut self, cmd: &str, now: f64) {
        if !self.enabled {
            self.text.clear();
            return;
        }
        // Truncate just the cmd part so the leading ':' is always visible
        let display_cmd = if cmd.chars().count() > MAX_DISPLAY_CHARS - 1 {
            let t: String = cmd.chars().take(MAX_DISPLAY_CHARS - 1).collect();
            format!("{}…", t)
        } else {
            cmd.to_string()
        };
        self.text = format!(":{}", display_cmd);
        self.kind = ShowCmdKind::Command;
        self.last_change_time = now;
        self.is_pending = true;
    }

    /// Live-tracks search query input (e.g. "/pattern", "?pattern").
    pub fn set_search(&mut self, symbol: &str, query: &str, now: f64) {
        if !self.enabled {
            self.text.clear();
            return;
        }
        let display_query = if query.chars().count() > MAX_DISPLAY_CHARS - 1 {
            let t: String = query.chars().take(MAX_DISPLAY_CHARS - 1).collect();
            format!("{}…", t)
        } else {
            query.to_string()
        };
        self.text = format!("{}{}", symbol, display_query);
        self.kind = ShowCmdKind::Search;
        self.last_change_time = now;
        self.is_pending = true;
    }

    /// Flashes a completed action (e.g. "ci\"", "vi\"", "40j", "dw", "dd", "x", "p").
    pub fn record_action(&mut self, text: &str, now: f64) {
        if !self.enabled {
            self.text.clear();
            return;
        }
        self.text = Self::truncate(text);
        self.kind = infer_kind(text);
        self.last_change_time = now;
        self.is_pending = false;
    }

    /// Clears the card display immediately (e.g. on Escape).
    pub fn clear(&mut self) {
        self.text.clear();
        self.is_pending = false;
    }

    /// Ticks timeouts: 2.0s for pending keys/search/command, 1.2s for completed actions.
    pub fn update(&mut self, now: f64) {
        if self.text.is_empty() {
            return;
        }
        let timeout = if self.is_pending { 2.0 } else { 1.2 };
        if (now - self.last_change_time) > timeout {
            self.text.clear();
            self.is_pending = false;
        }
    }

    /// Returns whether the card is currently visible within its timeout window.
    #[allow(dead_code)]
    pub fn is_visible(&self, now: f64) -> bool {
        if !self.enabled || self.text.is_empty() {
            return false;
        }
        let timeout = if self.is_pending { 2.0 } else { 1.2 };
        (now - self.last_change_time) <= timeout
    }

    /// Renders the card anchored at `anchor_bottom_right` (bottom-right corner of card).
    /// Sleek, borderless glass micro-capsule HUD with crisp typography and subtle mode badge.
    pub fn render_card(
        &self,
        painter: &egui::Painter,
        anchor_bottom_right: Pos2,
        theme: &crate::ui::theme::Theme,
        now: f64,
    ) -> Option<Rect> {
        if !self.enabled || self.text.is_empty() {
            return None;
        }

        let timeout = if self.is_pending { 2.0 } else { 1.2 };
        let elapsed = now - self.last_change_time;
        if elapsed > timeout {
            return None;
        }

        // Smooth fade-out over the last 0.35 s
        let alpha = {
            let fade = 0.35f64;
            if elapsed > timeout - fade {
                let frac = (timeout - elapsed) / fade;
                (frac.clamp(0.0, 1.0) * 255.0) as u8
            } else {
                255u8
            }
        };

        // ── Per-mode palette ─────────────────────────────────────────────────
        let is_light = theme.is_light();
        let (badge, badge_color) = match self.kind {
            ShowCmdKind::Command => (
                "CMD",
                if is_light {
                    Color32::from_rgb(0, 120, 215)
                } else {
                    Color32::from_rgb(100, 210, 255)
                },
            ),
            ShowCmdKind::Search => (
                "FIND",
                if is_light {
                    Color32::from_rgb(180, 110, 0)
                } else {
                    Color32::from_rgb(255, 210, 70)
                },
            ),
            ShowCmdKind::Visual => (
                "VIS",
                if is_light {
                    Color32::from_rgb(130, 50, 200)
                } else {
                    Color32::from_rgb(200, 140, 255)
                },
            ),
            ShowCmdKind::Keystroke => ("VIM", theme.accent),
        };

        let badge_a = Color32::from_rgba_unmultiplied(
            badge_color.r(),
            badge_color.g(),
            badge_color.b(),
            alpha,
        );
        let badge_bg = Color32::from_rgba_unmultiplied(
            badge_color.r(),
            badge_color.g(),
            badge_color.b(),
            if is_light {
                (alpha as f32 * 0.18) as u8
            } else {
                (alpha as f32 * 0.16) as u8
            },
        );
        let text_color = if is_light {
            Color32::from_rgba_unmultiplied(theme.text.r(), theme.text.g(), theme.text.b(), alpha)
        } else {
            Color32::from_rgba_unmultiplied(240, 244, 252, alpha)
        };

        // Pre-layout galleys for crisp rendering
        let font_badge = FontId::monospace(10.5);
        let font_text = FontId::monospace(14.0);

        let badge_galley = painter.layout_no_wrap(badge.to_string(), font_badge, badge_a);
        let text_galley = painter.layout_no_wrap(self.text.clone(), font_text, text_color);

        // Generous, comfortable dimensions with 5px corner radius
        let card_h = 34.0f32;
        let pad_x = 10.0f32;
        let badge_w = badge_galley.size().x + 12.0;
        let badge_h = 20.0f32;
        let gap = 8.0f32;
        let pulse_dot_w = if self.is_pending { 10.0 } else { 0.0 };

        let content_w = pad_x + badge_w + gap + text_galley.size().x + pulse_dot_w + pad_x;
        let card_w = content_w.max(80.0);

        let card_rect = Rect::from_min_max(
            Pos2::new(
                anchor_bottom_right.x - card_w,
                anchor_bottom_right.y - card_h,
            ),
            anchor_bottom_right,
        );

        // 1. Soft ambient drop-shadow (matching zoom HUD)
        let shadow_alpha = if is_light {
            (alpha as f32 * 0.11) as u8
        } else {
            (alpha as f32 * 0.28) as u8
        };
        painter.rect(
            card_rect.expand(3.0),
            10.0,
            Color32::from_rgba_unmultiplied(0, 0, 0, shadow_alpha),
            Stroke::NONE,
            egui::StrokeKind::Outside,
        );

        // 2. Borderless frosted capsule body — exactly matching zoom HUD
        let body_bg = if is_light {
            Color32::from_rgba_unmultiplied(255, 255, 255, (alpha as f32 * 0.90) as u8)
        } else {
            Color32::from_rgba_unmultiplied(20, 22, 28, (alpha as f32 * 0.90) as u8)
        };
        painter.rect(
            card_rect,
            8.0,
            body_bg,
            Stroke::NONE,
            egui::StrokeKind::Inside,
        );

        // 3. Render Badge Micro-Pill (borderless tinted badge, 4px round)
        let badge_x = card_rect.min.x + pad_x;
        let badge_y = card_rect.center().y - badge_h * 0.5;
        let badge_rect = Rect::from_min_size(
            eframe::egui::pos2(badge_x, badge_y),
            eframe::egui::vec2(badge_w, badge_h),
        );
        painter.rect(
            badge_rect,
            4.0,
            badge_bg,
            Stroke::NONE,
            egui::StrokeKind::Inside,
        );
        let badge_text_pos = eframe::egui::pos2(
            badge_rect.center().x - badge_galley.size().x * 0.5,
            badge_rect.center().y - badge_galley.size().y * 0.5,
        );
        painter.galley(badge_text_pos, badge_galley, badge_color);

        // 4. Render Command Text
        let text_w = text_galley.size().x;
        let text_x = badge_rect.max.x + gap;
        let text_y = card_rect.center().y - text_galley.size().y * 0.5;
        painter.galley(eframe::egui::pos2(text_x, text_y), text_galley, text_color);

        // 5. Breathing pending live dot indicator
        if self.is_pending {
            let dot_x = text_x + text_w + 6.0;
            let dot_pulse = ((now * 8.0).sin() as f32 * 0.5 + 0.5) * 0.8 + 0.2;
            let dot_color = Color32::from_rgba_unmultiplied(
                badge_color.r(),
                badge_color.g(),
                badge_color.b(),
                (alpha as f32 * dot_pulse) as u8,
            );
            painter.circle_filled(
                eframe::egui::pos2(dot_x, card_rect.center().y),
                2.5,
                dot_color,
            );
        }

        Some(card_rect)
    }
}

// ── Kind inference ────────────────────────────────────────────────────────────

/// Infers ShowCmdKind from the raw text string.
fn infer_kind(text: &str) -> ShowCmdKind {
    if text.starts_with(':') {
        ShowCmdKind::Command
    } else if text.starts_with('/') || text.starts_with('?') {
        ShowCmdKind::Search
    } else if text.starts_with('v') || text.starts_with('V') || text.starts_with('<') {
        ShowCmdKind::Visual
    } else {
        ShowCmdKind::Keystroke
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_showcmd_state_transitions() {
        let mut hud = ShowCmdState::new(true);
        assert!(hud.enabled);
        assert_eq!(hud.text, "");

        // Pending multi-key text object
        hud.set_pending("ci\"", 10.0);
        assert_eq!(hud.text, "ci\"");
        assert!(hud.is_pending);
        assert!(hud.is_visible(10.0));
        assert!(hud.is_visible(11.5)); // still within 2.0s

        // Still visible before timeout
        hud.update(11.0);
        assert_eq!(hud.text, "ci\"");

        // Expires after 2.0s
        hud.update(12.1);
        assert_eq!(hud.text, "");
        assert!(!hud.is_visible(12.1));

        // Completed action — 1.2s timeout
        hud.record_action("40j", 20.0);
        assert_eq!(hud.text, "40j");
        assert!(!hud.is_pending);
        assert!(hud.is_visible(20.5));
        hud.update(21.0);
        assert_eq!(hud.text, "40j");
        hud.update(21.3);
        assert_eq!(hud.text, "");

        // Command + search
        hud.set_command("w", 30.0);
        assert_eq!(hud.text, ":w");
        assert_eq!(hud.kind, ShowCmdKind::Command);

        hud.set_search("/", "needle", 35.0);
        assert_eq!(hud.text, "/needle");
        assert_eq!(hud.kind, ShowCmdKind::Search);

        // Visual from record_action
        hud.record_action("vi\"", 40.0);
        assert_eq!(hud.kind, ShowCmdKind::Visual);

        // Clear works immediately
        hud.set_pending("dw", 50.0);
        hud.clear();
        assert_eq!(hud.text, "");
        assert!(!hud.is_visible(50.0));

        // Disabled state never stores text
        let mut off = ShowCmdState::new(false);
        off.set_pending("ci\"", 1.0);
        assert_eq!(off.text, "");
        off.set_command("d", 1.0);
        assert_eq!(off.text, "");
        off.set_search("/", "pat", 1.0);
        assert_eq!(off.text, "");
        off.record_action("dw", 1.0);
        assert_eq!(off.text, "");
    }

    #[test]
    fn test_showcmd_truncation() {
        let mut hud = ShowCmdState::new(true);

        // Exactly at limit — no truncation
        let exact: String = "x".repeat(MAX_DISPLAY_CHARS);
        hud.record_action(&exact, 1.0);
        assert_eq!(hud.text.chars().count(), MAX_DISPLAY_CHARS);
        assert!(!hud.text.ends_with('…'));

        // One over limit — gets truncated
        let over: String = "x".repeat(MAX_DISPLAY_CHARS + 5);
        hud.record_action(&over, 2.0);
        assert!(hud.text.ends_with('…'));
        assert!(hud.text.chars().count() <= MAX_DISPLAY_CHARS + 1); // +1 for the ellipsis char

        // Command truncation keeps leading ':'
        let long_cmd: String = "w".repeat(MAX_DISPLAY_CHARS + 10);
        hud.set_command(&long_cmd, 3.0);
        assert!(hud.text.starts_with(':'));
        assert!(hud.text.ends_with('…'));

        // Search truncation keeps leading symbol
        let long_query: String = "a".repeat(MAX_DISPLAY_CHARS + 10);
        hud.set_search("/", &long_query, 4.0);
        assert!(hud.text.starts_with('/'));
        assert!(hud.text.ends_with('…'));
    }

    #[test]
    fn test_showcmd_kind_classification() {
        let mut hud = ShowCmdState::new(true);

        hud.record_action(":delete", 1.0);
        assert_eq!(hud.kind, ShowCmdKind::Command);

        hud.record_action("/pattern", 2.0);
        assert_eq!(hud.kind, ShowCmdKind::Search);

        hud.record_action("?back", 3.0);
        assert_eq!(hud.kind, ShowCmdKind::Search);

        hud.record_action("vi\"", 4.0);
        assert_eq!(hud.kind, ShowCmdKind::Visual);

        hud.record_action("VA", 5.0);
        assert_eq!(hud.kind, ShowCmdKind::Visual);

        hud.record_action("40j", 6.0);
        assert_eq!(hud.kind, ShowCmdKind::Keystroke);

        hud.record_action("ci\"", 7.0);
        assert_eq!(hud.kind, ShowCmdKind::Keystroke);

        hud.record_action("dw", 8.0);
        assert_eq!(hud.kind, ShowCmdKind::Keystroke);

        // set_pending also infers kind correctly
        hud.set_pending("vi", 9.0);
        assert_eq!(hud.kind, ShowCmdKind::Visual);

        hud.set_pending("d", 10.0);
        assert_eq!(hud.kind, ShowCmdKind::Keystroke);
    }

    #[test]
    fn test_showcmd_timeout_values() {
        let mut hud = ShowCmdState::new(true);

        // Pending: 2.0s timeout
        hud.set_pending("d", 100.0);
        assert!(hud.is_visible(101.9));
        assert!(!hud.is_visible(102.1));

        // Completed action: 1.2s timeout
        hud.record_action("dd", 200.0);
        assert!(hud.is_visible(201.1));
        assert!(!hud.is_visible(201.3));
    }
}
