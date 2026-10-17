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
    /// Accumulates multi-key sequences (`vi(`, `vpw`, `gd`, `ciw`, `20j`, `"ayy`) while the HUD is visible,
    /// and never stacks repeated navigation keys (`j`, `k`, `<Down>`, `<Up>`).
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

        let timeout = if self.is_pending { 4.5 } else { 3.0 };
        let elapsed = now - self.last_change_time;
        let card_visible = !self.text.is_empty() && elapsed <= timeout;
        let is_seq_kind = matches!(self.kind, ShowCmdKind::Keystroke | ShowCmdKind::Visual);
        let is_nav = is_navigation_key(key);
        let token_count = tokenize_keycaps(&self.text).len();
        let awaiting_motion = is_incomplete_sequence(&self.text);

        // A key can ONLY append to an existing sequence if:
        // 1. The HUD card is currently visible
        // 2. The previous sequence is INCOMPLETE / PENDING (awaiting_motion)
        // 3. The previous sequence is not <Esc> or a finished action
        // 4. Token limit hasn't been exceeded
        let can_append = card_visible
            && self.text != "<Esc>"
            && is_seq_kind
            && awaiting_motion
            && token_count < 8
            && if is_nav {
                awaiting_motion
            } else {
                !ends_with_navigation_key(&self.text)
            };

        let next_text = if can_append {
            format!("{}{}", self.text, key)
        } else {
            key.to_string()
        };

        self.text = Self::truncate(&next_text);
        if !can_append || self.kind != ShowCmdKind::Visual {
            self.kind = infer_kind(&self.text);
        }
        self.last_change_time = now;
        self.is_pending = if is_nav {
            false
        } else {
            is_incomplete_sequence(&self.text)
        };
    }

    /// Synchronizes the HUD directly with Neovim's MessagePack-RPC state
    /// (`msg_showcmd` event, `cmdline_show`, and `mode_change`).
    pub fn sync_from_rpc(
        &mut self,
        rpc_showcmd_update: Option<&str>,
        rpc_cmdline: &str,
        rpc_mode: &str,
        now: f64,
    ) {
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

        // 2. When entering Insert mode (e.g. after `i`, `a`, `ciw`, `ci(`), mark the
        //    sequence as completed (`is_pending = false`) instead of wiping `self.text`
        //    so the command that entered Insert mode stays visible for its full duration.
        if rpc_mode.starts_with("insert") {
            if self.is_pending {
                self.is_pending = false;
                self.last_change_time = now;
            }
            return;
        }

        let is_visual_mode = rpc_mode.starts_with("visual");
        if is_visual_mode && !self.text.is_empty() && self.kind == ShowCmdKind::Keystroke {
            self.kind = ShowCmdKind::Visual;
        }

        // 3. Process event-driven `msg_showcmd` update from Neovim when one arrives.
        //    Never overwrite a non-empty `self.text` with `trimmed_showcmd` because Neovim's
        //    internal `showcmd_buf` omits mode-entering prefixes (such as `v` in `vi(`),
        //    which would clobber `"vi"` into `"i"`.
        if let Some(raw_showcmd) = rpc_showcmd_update {
            let trimmed_showcmd = raw_showcmd.trim();
            let is_visual_size_only = is_visual_mode
                && !trimmed_showcmd.is_empty()
                && trimmed_showcmd
                    .chars()
                    .all(|c| c.is_ascii_digit() || c == 'x');

            if is_visual_size_only {
                // In visual mode, Neovim sends selection dimensions (e.g. "1" or "3x5").
                // Visual mode is actively running; do not clear pending state.
            } else if !trimmed_showcmd.is_empty() {
                let truncated = Self::truncate(trimmed_showcmd);
                let next_kind = if is_visual_mode || self.kind == ShowCmdKind::Visual {
                    ShowCmdKind::Visual
                } else {
                    infer_kind(&truncated)
                };
                if self.text.is_empty() {
                    self.text = truncated;
                }
                self.kind = next_kind;
                self.last_change_time = now;
                self.is_pending = true;
            } else if self.is_pending
                && matches!(self.kind, ShowCmdKind::Keystroke | ShowCmdKind::Visual)
            {
                self.is_pending = false;
                self.last_change_time = now;
            }
        }
    }

    /// Clears the card display immediately (e.g. on Escape in command bar).
    pub fn clear(&mut self) {
        self.text.clear();
        self.is_pending = false;
    }

    /// Ticks timeouts: 4.5s for pending keys/search/command, 3.0s for completed actions.
    pub fn update(&mut self, now: f64) {
        if self.text.is_empty() {
            return;
        }
        let timeout = if self.is_pending { 4.5 } else { 3.0 };
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
        let timeout = if self.is_pending { 4.5 } else { 3.0 };
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

        let timeout = if self.is_pending { 4.5 } else { 3.0 };
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

/// Returns true if the current sequence ends with a navigation key (e.g. `"20j"` or `"dj"`).
fn ends_with_navigation_key(text: &str) -> bool {
    tokenize_keycaps(text)
        .last()
        .is_some_and(|tok| is_navigation_key(tok))
}

/// Returns true if `text` is an incomplete key sequence awaiting further keystrokes
/// (e.g. operators `"d"`, `"c"`, `"y"`, counts `"20"`, text-object prefixes `"ci"`, `"vi"`, `"da"`,
/// or character-taking commands `"f"`, `"r"`, `"g"`).
pub fn is_incomplete_sequence(text: &str) -> bool {
    if text.is_empty() {
        return false;
    }
    // Completed doubled operators: "dd", "cc", "yy", ">>", "<<", "=="
    if matches!(text, "dd" | "cc" | "yy" | ">>" | "<<" | "==") {
        return false;
    }
    let mut s = text;
    if let Some(after_quote) = s.strip_prefix('"') {
        let mut chars = after_quote.chars();
        if chars.next().is_none() {
            return true;
        }
        s = chars.as_str();
    }
    // Only digits -> incomplete count (e.g. "2", "20")
    let after_digits = s.trim_start_matches(|c: char| c.is_ascii_digit());
    if after_digits.is_empty() {
        return true;
    }
    // Check text object prefixes: [dcyv][ia] or with count (e.g. "di", "ci", "vi", "da", "va", "ya")
    // If it's just "di" / "vi" / "ci", it's awaiting target object (e.g. '(', '"', 'w').
    // If it already has the target object (3 chars or more like "vi(", "ciw", "da]"), it is COMPLETE!
    let after_op_digits = after_digits.trim_start_matches(|c: char| c.is_ascii_digit());
    if let Some(prefix) = after_op_digits.strip_prefix(['d', 'c', 'y', 'v']) {
        if prefix == "i" || prefix == "a" {
            return true;
        }
        if (prefix.starts_with('i') || prefix.starts_with('a')) && prefix.len() >= 2 {
            return false;
        }
    }
    // Standalone visual mode starter awaiting motion / target, or single visual motion (e.g. "vp")
    if after_op_digits == "v" || after_op_digits == "V" || after_op_digits == "<C-v>" {
        return true;
    }
    if let Some(rest) = after_op_digits.strip_prefix('v') {
        if rest.chars().count() == 1 && rest != "i" && rest != "a" {
            return true;
        }
    }
    // Operators waiting for a motion
    matches!(
        after_op_digits,
        "d" | "c"
            | "y"
            | "g"
            | "z"
            | ">"
            | "<"
            | "="
            | "!"
            | "m"
            | "r"
            | "f"
            | "F"
            | "t"
            | "T"
            | "<C-w>"
            | "gu"
            | "gU"
            | "g~"
            | "gq"
            | "gw"
            | "z="
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
        assert!(hud.is_visible(14.0)); // still within 4.5s

        // Still visible before timeout
        hud.update(13.5);
        assert_eq!(hud.text, "ci\"");

        // Expires after 4.5s
        hud.update(14.6);
        assert_eq!(hud.text, "");
        assert!(!hud.is_visible(14.6));

        // Completed action — 3.0s timeout
        hud.record_action("40j", 20.0);
        assert_eq!(hud.text, "40j");
        assert!(!hud.is_pending);
        assert!(hud.is_visible(22.0));
        hud.update(22.5);
        assert_eq!(hud.text, "40j");
        hud.update(23.1);
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

        // 1. Pressing `v`, `p`, `w` (even with >1s pause while visible) accumulates `"vpw"`
        hud.push_key("v", 1.0);
        assert_eq!(hud.text, "v");
        assert_eq!(hud.kind, ShowCmdKind::Visual);
        hud.sync_from_rpc(Some("1"), "", "visual", 1.05);
        assert_eq!(hud.text, "v");

        hud.push_key("p", 2.2);
        assert_eq!(hud.text, "vp");
        assert_eq!(hud.kind, ShowCmdKind::Visual);

        hud.push_key("w", 3.0);
        assert_eq!(hud.text, "vpw");
        assert_eq!(hud.kind, ShowCmdKind::Visual);

        // 2. Pressing `v`, `i`, `(` with Neovim sending `Some("i")` in Visual mode preserves `"vi("`
        let mut vi_hud = ShowCmdState::new(true);
        vi_hud.push_key("v", 5.0);
        vi_hud.sync_from_rpc(Some(""), "", "visual", 5.01);
        assert_eq!(vi_hud.text, "v");
        vi_hud.push_key("i", 5.2);
        assert_eq!(vi_hud.text, "vi");
        // Neovim in visual mode only reports `"i"` in msg_showcmd — must NOT clobber `"vi"` to `"i"`
        vi_hud.sync_from_rpc(Some("i"), "", "visual", 5.21);
        assert_eq!(vi_hud.text, "vi");
        vi_hud.push_key("(", 5.4);
        assert_eq!(vi_hud.text, "vi(");
        vi_hud.sync_from_rpc(Some(""), "", "visual", 5.41);
        assert_eq!(vi_hud.text, "vi(");

        // Typing a new command like `y` then `p` immediately replaces completed `vi(`, never producing `vi( yp`
        vi_hud.push_key("y", 6.0);
        assert_eq!(vi_hud.text, "y");
        assert!(vi_hud.is_pending);
        vi_hud.push_key("p", 6.2);
        assert_eq!(vi_hud.text, "yp");
        assert!(!vi_hud.is_pending);

        // 3. Pressing `c`, `i`, `(` entering Insert mode keeps `"ci("` visible instead of clearing
        let mut ci_hud = ShowCmdState::new(true);
        ci_hud.push_key("c", 7.0);
        ci_hud.sync_from_rpc(Some("c"), "", "operator", 7.01);
        ci_hud.push_key("i", 7.2);
        ci_hud.sync_from_rpc(Some("ci"), "", "operator", 7.21);
        ci_hud.push_key("(", 7.4);
        assert_eq!(ci_hud.text, "ci(");
        ci_hud.sync_from_rpc(Some(""), "", "insert", 7.41);
        assert_eq!(ci_hud.text, "ci(");
        assert!(!ci_hud.is_pending);
        assert!(ci_hud.is_visible(9.5));

        // 4. Pressing `g` then `d` with interleaved RPC frames never clobbers `"gd"` back to `"g"`
        let mut gd_hud = ShowCmdState::new(true);
        gd_hud.push_key("g", 10.0);
        assert_eq!(gd_hud.text, "g");
        gd_hud.sync_from_rpc(Some("g"), "", "normal", 10.01);
        assert_eq!(gd_hud.text, "g");
        assert!(gd_hud.is_pending);

        gd_hud.push_key("d", 10.4);
        assert_eq!(gd_hud.text, "gd");
        // Intermediate frame before Neovim finishes processing `d` (no new showcmd event)
        gd_hud.sync_from_rpc(None, "", "normal", 10.41);
        assert_eq!(gd_hud.text, "gd");
        // Even if a duplicate `Some("g")` arrived, `"gd"` is preserved
        gd_hud.sync_from_rpc(Some("g"), "", "normal", 10.42);
        assert_eq!(gd_hud.text, "gd");
        // Neovim finishes `gd` and clears showcmd -> `"gd"` stays visible as completed action
        gd_hud.sync_from_rpc(Some(""), "", "normal", 10.45);
        assert_eq!(gd_hud.text, "gd");
        assert!(!gd_hud.is_pending);

        // 5. Repeated navigation keys (`j`, `<Down>`) never stack
        gd_hud.push_key("j", 11.0);
        assert_eq!(gd_hud.text, "j");
        gd_hud.push_key("j", 11.1);
        assert_eq!(gd_hud.text, "j");
        gd_hud.push_key("<Down>", 11.2);
        assert_eq!(gd_hud.text, "<Down>");
        gd_hud.push_key("<Down>", 11.3);
        assert_eq!(gd_hud.text, "<Down>");

        // 6. Count + motion (`20j`)
        gd_hud.push_key("2", 15.0);
        gd_hud.push_key("0", 15.1);
        gd_hud.push_key("j", 15.2);
        assert_eq!(gd_hud.text, "20j");
        assert!(!gd_hud.is_pending);
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

        // Pending: 4.5s timeout
        hud.set_pending("d", 100.0);
        assert!(hud.is_visible(104.4));
        assert!(!hud.is_visible(104.6));

        // Completed action: 3.0s timeout
        hud.record_action("dd", 200.0);
        assert!(hud.is_visible(202.9));
        assert!(!hud.is_visible(203.1));
    }
}
