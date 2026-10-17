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

    /// Appends a keystroke to the active sequence or starts a new one.
    /// Navigation keys never repeat or chain unless a count/operator is pending in Neovim.
    pub fn push_key(&mut self, key: &str, now: f64) {
        if !self.enabled || key.is_empty() {
            if !self.enabled {
                self.text.clear();
            }
            return;
        }

        if key == "<Esc>" {
            self.text = "<Esc>".to_string();
            self.kind = ShowCmdKind::Keystroke;
            self.last_change_time = now;
            self.is_pending = false;
            return;
        }

        let elapsed = now - self.last_change_time;
        let is_seq_kind = matches!(self.kind, ShowCmdKind::Keystroke | ShowCmdKind::Visual);
        let is_nav = is_navigation_key(key);

        // Navigation keys (`j`, `k`, `h`, `l`, `<Up>`, `<Down>`, etc.) only append
        // when Neovim has a pending operator/count (e.g. `20j`, `dj`); otherwise they
        // replace the display so holding/repeating `j` or `<Down>` never stacks.
        let can_append = !self.text.is_empty()
            && self.text != "<Esc>"
            && is_seq_kind
            && !is_navigation_key(&self.text)
            && ((self.is_pending && elapsed <= 2.0)
                || (!is_nav
                    && elapsed <= 0.9
                    && !key.starts_with('<')
                    && self.text.chars().count() < 6));

        let next_text = if can_append {
            format!("{}{}", self.text, key)
        } else {
            key.to_string()
        };

        self.text = Self::truncate(&next_text);
        self.kind = infer_kind(&self.text);
        self.last_change_time = now;
        if is_nav {
            self.is_pending = false;
        }
    }

    /// Synchronizes the HUD directly with Neovim's MessagePack-RPC state
    /// (`msg_showcmd`, `cmdline_show`, and `mode_change`).
    pub fn sync_from_rpc(&mut self, rpc_showcmd: &str, rpc_cmdline: &str, rpc_mode: &str, now: f64) {
        if !self.enabled {
            self.text.clear();
            self.is_pending = false;
            return;
        }

        // 1. Live command-line or search input from Neovim `cmdline_show`
        if !rpc_cmdline.is_empty() {
            let next_kind = if rpc_cmdline.starts_with('/') || rpc_cmdline.starts_with('?') {
                ShowCmdKind::Search
            } else {
                ShowCmdKind::Command
            };
            let truncated = Self::truncate(rpc_cmdline);
            if self.text != truncated || self.kind != next_kind || !self.is_pending {
                self.text = truncated;
                self.kind = next_kind;
                self.last_change_time = now;
                self.is_pending = true;
            }
            return;
        } else if matches!(self.kind, ShowCmdKind::Command | ShowCmdKind::Search) && self.is_pending {
            self.is_pending = false;
            self.last_change_time = now;
        }

        // 2. Insert mode clears keystroke HUD
        if rpc_mode.starts_with("insert") {
            self.clear();
            return;
        }

        // 3. Pending operator / count / prefix from Neovim `msg_showcmd`
        let trimmed_showcmd = rpc_showcmd.trim();
        let is_visual_mode = rpc_mode.starts_with("visual");
        // In Visual mode without a pending operator, Neovim's `showcmd` reports
        // selection dimensions (digits or `RxC`), which shouldn't overwrite typed keys.
        let is_visual_size_only = is_visual_mode
            && !trimmed_showcmd.is_empty()
            && trimmed_showcmd
                .chars()
                .all(|c| c.is_ascii_digit() || c == 'x');

        if !trimmed_showcmd.is_empty() && !is_visual_size_only {
            let truncated = Self::truncate(trimmed_showcmd);
            let next_kind = if is_visual_mode {
                ShowCmdKind::Visual
            } else {
                infer_kind(&truncated)
            };
            if self.text != truncated || !self.is_pending || self.kind != next_kind {
                self.text = truncated;
                self.kind = next_kind;
                self.last_change_time = now;
                self.is_pending = true;
            }
        } else {
            if self.is_pending && matches!(self.kind, ShowCmdKind::Keystroke | ShowCmdKind::Visual) {
                self.is_pending = false;
                self.last_change_time = now;
            }
            if is_visual_mode && !self.text.is_empty() && self.kind == ShowCmdKind::Keystroke {
                self.kind = ShowCmdKind::Visual;
            }
        }
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
    /// Sleek, compact HUD capsule with tactile keycaps and mode badge.
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

        let font_badge = FontId::monospace(10.0);
        let font_text = FontId::monospace(13.0);

        let badge_galley = painter.layout_no_wrap(badge.to_string(), font_badge, badge_a);

        let use_keycaps = matches!(self.kind, ShowCmdKind::Keystroke | ShowCmdKind::Visual);
        let keycap_tokens = if use_keycaps {
            tokenize_keycaps(&self.text)
        } else {
            Vec::new()
        };
        let keycap_galleys: Vec<_> = keycap_tokens
            .iter()
            .map(|tok| painter.layout_no_wrap(tok.clone(), font_text.clone(), text_color))
            .collect();
        let plain_galley = if !use_keycaps {
            Some(painter.layout_no_wrap(self.text.clone(), font_text, text_color))
        } else {
            None
        };

        let keys_w: f32 = if use_keycaps {
            let caps_sum: f32 = keycap_galleys
                .iter()
                .map(|g| (g.size().x + 10.0).max(20.0))
                .sum();
            let gaps = keycap_galleys.len().saturating_sub(1) as f32 * 3.5;
            caps_sum + gaps
        } else {
            plain_galley.as_ref().map_or(0.0, |g| g.size().x)
        };

        let card_h = 34.0f32;
        let pad_x = 10.0f32;
        let badge_w = badge_galley.size().x + 12.0;
        let badge_h = 20.0f32;
        let gap = 8.0f32;
        let pulse_dot_w = if self.is_pending { 10.0 } else { 0.0 };

        let content_w = pad_x + badge_w + gap + keys_w + pulse_dot_w + pad_x;
        let card_w = content_w.max(72.0);

        let card_rect = Rect::from_min_max(
            Pos2::new(
                anchor_bottom_right.x - card_w,
                anchor_bottom_right.y - card_h,
            ),
            anchor_bottom_right,
        );

        // 1. Soft ambient drop-shadow
        let shadow_alpha = if is_light {
            (alpha as f32 * 0.10) as u8
        } else {
            (alpha as f32 * 0.28) as u8
        };
        painter.rect(
            card_rect.expand(2.5),
            10.0,
            Color32::from_rgba_unmultiplied(0, 0, 0, shadow_alpha),
            Stroke::NONE,
            egui::StrokeKind::Outside,
        );

        // 2. Frosted capsule body with subtle border
        let body_bg = if is_light {
            Color32::from_rgba_unmultiplied(255, 255, 255, (alpha as f32 * 0.94) as u8)
        } else {
            Color32::from_rgba_unmultiplied(20, 22, 28, (alpha as f32 * 0.92) as u8)
        };
        let body_border = Color32::from_rgba_unmultiplied(
            badge_color.r(),
            badge_color.g(),
            badge_color.b(),
            (alpha as f32 * 0.22) as u8,
        );
        painter.rect(
            card_rect,
            8.0,
            body_bg,
            Stroke::new(1.0, body_border),
            egui::StrokeKind::Inside,
        );

        // 3. Render Badge Micro-Pill
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

        // 4. Render Keycaps or Command/Search Text
        let mut cursor_x = badge_rect.max.x + gap;
        let center_y = card_rect.center().y;

        if use_keycaps {
            let cap_bg = if is_light {
                Color32::from_rgba_unmultiplied(236, 240, 246, (alpha as f32 * 0.95) as u8)
            } else {
                Color32::from_rgba_unmultiplied(36, 40, 50, (alpha as f32 * 0.95) as u8)
            };
            let cap_border = if is_light {
                Color32::from_rgba_unmultiplied(195, 202, 214, (alpha as f32 * 0.85) as u8)
            } else {
                Color32::from_rgba_unmultiplied(70, 76, 92, (alpha as f32 * 0.80) as u8)
            };
            let cap_h = 21.0f32;

            for galley in keycap_galleys {
                let cap_w = (galley.size().x + 10.0).max(20.0);
                let cap_rect = Rect::from_min_size(
                    eframe::egui::pos2(cursor_x, center_y - cap_h * 0.5),
                    eframe::egui::vec2(cap_w, cap_h),
                );
                painter.rect(
                    cap_rect,
                    4.5,
                    cap_bg,
                    Stroke::new(0.9, cap_border),
                    egui::StrokeKind::Inside,
                );
                let g_pos = eframe::egui::pos2(
                    cap_rect.center().x - galley.size().x * 0.5,
                    cap_rect.center().y - galley.size().y * 0.5,
                );
                painter.galley(g_pos, galley, text_color);
                cursor_x = cap_rect.max.x + 3.5;
            }
            if !keycap_tokens.is_empty() {
                cursor_x -= 3.5;
            }
        } else if let Some(galley) = plain_galley {
            let text_w = galley.size().x;
            let text_y = center_y - galley.size().y * 0.5;
            painter.galley(eframe::egui::pos2(cursor_x, text_y), galley, text_color);
            cursor_x += text_w;
        }

        // 5. Breathing pending live dot indicator
        if self.is_pending {
            let dot_x = cursor_x + 6.0;
            let dot_pulse = ((now * 8.0).sin() as f32 * 0.5 + 0.5) * 0.8 + 0.2;
            let dot_color = Color32::from_rgba_unmultiplied(
                badge_color.r(),
                badge_color.g(),
                badge_color.b(),
                (alpha as f32 * dot_pulse) as u8,
            );
            painter.circle_filled(
                eframe::egui::pos2(dot_x, center_y),
                2.5,
                dot_color,
            );
        }

        Some(card_rect)
    }
}

// ── Kind inference & keycap tokenization ──────────────────────────────────────

/// Infers ShowCmdKind from the raw text string.
fn infer_kind(text: &str) -> ShowCmdKind {
    if text.starts_with(':') {
        ShowCmdKind::Command
    } else if text.starts_with('/') || text.starts_with('?') {
        ShowCmdKind::Search
    } else if text.starts_with('v')
        || text.starts_with('V')
        || text.starts_with("<C-v>")
        || text.starts_with("<C-V>")
    {
        ShowCmdKind::Visual
    } else {
        ShowCmdKind::Keystroke
    }
}

/// Returns true if `key` is a cursor navigation or single-step editing key that
/// should never stack with itself or append to completed keys.
fn is_navigation_key(key: &str) -> bool {
    matches!(
        key,
        "h" | "j"
            | "k"
            | "l"
            | "<Up>"
            | "<Down>"
            | "<Left>"
            | "<Right>"
            | "<BS>"
            | "<Del>"
            | "<PageUp>"
            | "<PageDown>"
            | "<Home>"
            | "<End>"
    )
}

/// Splits a raw keystroke string like `"<C-w>v"`, `"20dd"`, or `"vpw"` into keycap tokens.
fn tokenize_keycaps(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '<' {
            if let Some(rel_end) = chars[i..].iter().position(|&c| c == '>') {
                let token: String = chars[i..=i + rel_end].iter().collect();
                tokens.push(token);
                i += rel_end + 1;
                continue;
            }
        }
        if chars[i].is_ascii_digit() && (chars[i] != '0' || !tokens.is_empty()) {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            tokens.push(chars[start..i].iter().collect());
            continue;
        }
        tokens.push(chars[i].to_string());
        i += 1;
    }
    tokens
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
    fn test_showcmd_multi_key_and_rpc_sync() {
        let mut hud = ShowCmdState::new(true);

        // Pressing `v`, `p`, `w` accumulates all 3 letters into `"vpw"`
        hud.push_key("v", 1.0);
        assert_eq!(hud.text, "v");
        assert_eq!(hud.kind, ShowCmdKind::Visual);

        hud.push_key("p", 1.2);
        assert_eq!(hud.text, "vp");
        assert_eq!(hud.kind, ShowCmdKind::Visual);

        hud.push_key("w", 1.4);
        assert_eq!(hud.text, "vpw");
        assert_eq!(hud.kind, ShowCmdKind::Visual);

        // Repeated navigation keys (`j`, `<Down>`) never stack
        hud.push_key("j", 2.0);
        assert_eq!(hud.text, "j");
        hud.push_key("j", 2.1);
        assert_eq!(hud.text, "j");
        hud.push_key("<Down>", 2.2);
        assert_eq!(hud.text, "<Down>");
        hud.push_key("<Down>", 2.3);
        assert_eq!(hud.text, "<Down>");

        // Neovim MessagePack-RPC `msg_showcmd` drives pending operator/count state
        hud.sync_from_rpc("20", "", "normal", 5.0);
        assert_eq!(hud.text, "20");
        assert!(hud.is_pending);
        hud.push_key("j", 5.1);
        hud.sync_from_rpc("", "", "normal", 5.1);
        assert_eq!(hud.text, "20j");
        assert!(!hud.is_pending);
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
