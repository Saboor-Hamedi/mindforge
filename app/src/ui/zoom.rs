//! Editor zoom management: gesture handling, font metrics scaling, and borderless center HUD.
//!
//! Provides smooth zoom in/out with:
//! - 8% step increments (multiplicative, not additive)
//! - Clamping to [0.5x, 3.0x] range
//! - A floating HUD indicator that appears on zoom change and fades after 1.3s
//! - Mouse wheel + Ctrl gesture support
//!
//! The zoom level is stored in `ZoomState` and applied by the renderer
//! when calculating font sizes and layout metrics.

use eframe::egui::{self, vec2, Color32, FontId, Rect, Stroke};
use crate::ui::theme::Theme;

/// Minimum allowed zoom level (50%)
pub const MIN_ZOOM: f32 = 0.5;
/// Maximum allowed zoom level (300%)
pub const MAX_ZOOM: f32 = 3.0;
/// Default zoom level (100%)
pub const DEFAULT_ZOOM: f32 = 1.0;
/// How long the zoom HUD remains visible (seconds)
pub const HUD_DURATION: f32 = 1.3;

#[derive(Debug, Clone)]
pub struct ZoomState {
    pub level: f32,
    pub hud_time: f64,
}

impl Default for ZoomState {
    fn default() -> Self {
        Self {
            level: DEFAULT_ZOOM,
            hud_time: -10.0,
        }
    }
}

impl ZoomState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset zoom to default 100%
    pub fn reset(&mut self, now: f64) {
        self.level = DEFAULT_ZOOM;
        self.hud_time = now;
    }

    /// Zooms in by a step (+8%)
    pub fn zoom_in(&mut self, now: f64) {
        self.level = (self.level * 1.08).clamp(MIN_ZOOM, MAX_ZOOM);
        self.hud_time = now;
    }

    /// Zooms out by a step (-8%)
    pub fn zoom_out(&mut self, now: f64) {
        self.level = (self.level / 1.08).clamp(MIN_ZOOM, MAX_ZOOM);
        self.hud_time = now;
    }

    /// Sets zoom level directly with clamping
    pub fn set_level(&mut self, level: f32, now: f64) {
        self.level = level.clamp(MIN_ZOOM, MAX_ZOOM);
        self.hud_time = now;
    }

    /// Calculates scaled font size and exact monospace character metrics (cw, lh) for editor text layout.
    pub fn editor_metrics(&self, base_font_size: f32, ctx: &egui::Context) -> (f32, f32, f32) {
        let ed_font_size = (base_font_size * self.level).clamp(8.0, 60.0);
        // Optical scale calibration: egui's monospace glyphs are ~9% larger than proportional glyphs.
        // Calibrating raw monospace font size by 0.92 ensures seamless optical scale when toggling Ctrl+E.
        let raw_font_size = (ed_font_size * 0.92).round().max(8.0);
        let font = crate::services::font_manager::editor_font_id(raw_font_size);
        let (cw, lh) = ctx.fonts(|f| {
            let sample_100 = "MMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMM";
            let g100 = f.layout_no_wrap(sample_100.to_owned(), font.clone(), Color32::WHITE);
            let g1 = f.layout_no_wrap("M".to_owned(), font, Color32::WHITE);
            let cw = (g100.size().x - g1.size().x) / 99.0;
            let lh = (g1.size().y * 1.30).round();
            (cw, lh)
        });
        (raw_font_size, cw, lh)
    }

    /// Processes user zoom input (trackpad pinch, Ctrl+Wheel, Ctrl+0, Ctrl+=, Ctrl+-).
    /// Returns true if zoom level changed.
    pub fn handle_input(
        &mut self,
        ui: &egui::Ui,
        editor_rect: Rect,
        now: f64,
        modals_active: bool,
    ) -> bool {
        if modals_active {
            return false;
        }

        let mut changed = false;
        let pointer_in_editor = ui.rect_contains_pointer(editor_rect);

        if pointer_in_editor {
            // Trackpad pinch-to-zoom
            let zoom_delta = ui.input(|i| i.zoom_delta());
            if (zoom_delta - 1.0).abs() > 0.001 {
                let new_zoom = (self.level * zoom_delta).clamp(MIN_ZOOM, MAX_ZOOM);
                if (new_zoom - self.level).abs() > 0.001 {
                    self.level = new_zoom;
                    changed = true;
                }
            }

            // Ctrl + Mouse Wheel (or smooth trackpad vertical scroll with Ctrl)
            let (ctrl_down, wheel_y) = ui.input(|i| (
                i.modifiers.ctrl || i.modifiers.command,
                if i.raw_scroll_delta.y.abs() > 0.0 {
                    i.raw_scroll_delta.y
                } else {
                    i.smooth_scroll_delta.y
                },
            ));
            if ctrl_down && wheel_y.abs() > 0.0 {
                let factor = (1.0 + wheel_y * 0.002).clamp(0.85, 1.15);
                let new_zoom = (self.level * factor).clamp(MIN_ZOOM, MAX_ZOOM);
                if (new_zoom - self.level).abs() > 0.001 {
                    self.level = new_zoom;
                    changed = true;
                }
            }
        }

        // Global keyboard zoom shortcuts
        let (ctrl_zero, ctrl_plus, ctrl_minus) = ui.input(|i| {
            let ctrl = i.modifiers.ctrl || i.modifiers.command;
            (
                ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::Num0),
                ctrl && (i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals)),
                ctrl && i.key_pressed(egui::Key::Minus),
            )
        });

        if ctrl_zero {
            self.reset(now);
            changed = true;
        } else if ctrl_plus {
            self.zoom_in(now);
            changed = true;
        } else if ctrl_minus {
            self.zoom_out(now);
            changed = true;
        }

        if changed {
            self.hud_time = now;
            ui.ctx().request_repaint();
        }

        changed
    }

    /// Renders the borderless center fading percentage HUD pill.
    pub fn render_hud(
        &self,
        ui: &egui::Ui,
        painter: &egui::Painter,
        editor_rect: Rect,
        theme: &Theme,
        now: f64,
    ) {
        let elapsed = (now - self.hud_time) as f32;
        if elapsed >= HUD_DURATION || self.hud_time <= 0.0 {
            return;
        }

        ui.ctx().request_repaint();
        let alpha = if elapsed < 0.6 {
            1.0
        } else {
            ((HUD_DURATION - elapsed) / 0.7).clamp(0.0, 1.0)
        };

        let pct_text = format!("{:.0}%", (self.level * 100.0).round());
        let hud_font = FontId::monospace(48.0);
        let hud_col = Color32::from_rgba_unmultiplied(
            theme.highlight.r(),
            theme.highlight.g(),
            theme.highlight.b(),
            (alpha * 240.0) as u8,
        );
        let galley = painter.layout_no_wrap(pct_text, hud_font, hud_col);
        let hud_center = editor_rect.center();
        let pill_rect = Rect::from_center_size(hud_center, galley.size() + vec2(48.0, 26.0));

        // Soft ambient drop-shadow (borderless)
        let shadow_alpha = if theme.is_light() {
            (alpha * 28.0) as u8
        } else {
            (alpha * 65.0) as u8
        };
        painter.rect(
            pill_rect.expand(4.0),
            10.0,
            Color32::from_rgba_unmultiplied(0, 0, 0, shadow_alpha),
            Stroke::NONE,
            egui::StrokeKind::Outside,
        );

        // Borderless frosted pill body
        let pill_bg = if theme.is_light() {
            Color32::from_rgba_unmultiplied(255, 255, 255, (alpha * 230.0) as u8)
        } else {
            Color32::from_rgba_unmultiplied(20, 22, 28, (alpha * 230.0) as u8)
        };
        painter.rect(pill_rect, 8.0, pill_bg, Stroke::NONE, egui::StrokeKind::Inside);
        painter.galley(hud_center - galley.size() * 0.5, galley, hud_col);
    }
}
