//! Scrollable language-server browser opened by `:LspList`.
//!
//! Neovim only sends the rows; selection and scrolling live here so the list
//! stays responsive no matter how many servers are registered. The presentation
//! mirrors Mason's picker: a titled drop-down with a status dot per server
//! (`running` / `installed` / `not installed`) and the filetypes it serves.

use eframe::egui::{self, Align2, Color32, FontId, Painter, Pos2, Rect, Stroke};

const MAX_ROWS: usize = 12;

pub struct LspRow {
    pub name: String,
    pub status: String,
    pub desc: String,
    pub filetypes: String,
}

pub enum PanelAction {
    None,
    /// Ex command to run in Neovim.
    Run(String),
}

#[derive(Default)]
pub struct LspPanel {
    pub open: bool,
    rows: Vec<LspRow>,
    selected: usize,
    /// Index of the first visible row (independent of the selection cursor).
    scroll_offset: usize,
}

impl LspPanel {
    pub fn update(&mut self, rows: Vec<LspRow>, open: bool, _filter: &str) {
        self.rows = rows;
        if open {
            self.open = true;
            self.selected = 0;
            self.scroll_offset = 0;
        }
        self.selected = self.selected.min(self.matches().len().saturating_sub(1));
        self.clamp_offset();
    }

    /// Installed servers first, then alphabetical (rows arrive sorted).
    fn matches(&self) -> Vec<&LspRow> {
        let mut rows: Vec<&LspRow> = self.rows.iter().collect();
        rows.sort_by_key(|row| row.status == "available");
        rows
    }

    fn visible_count(&self) -> usize {
        self.matches().len().clamp(1, MAX_ROWS)
    }

    fn max_offset(&self) -> usize {
        self.matches().len().saturating_sub(self.visible_count())
    }

    fn clamp_offset(&mut self) {
        self.scroll_offset = self.scroll_offset.min(self.max_offset());
    }

    /// Keeps the selected row within the visible window after a move.
    fn ensure_visible(&mut self) {
        let visible = self.visible_count();
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if self.selected >= self.scroll_offset + visible {
            self.scroll_offset = self.selected + 1 - visible;
        }
        self.clamp_offset();
    }

    fn step(&mut self, delta: i32) {
        let total = self.matches().len();
        if total > 0 {
            self.selected =
                (self.selected as i64 + delta as i64).clamp(0, total as i64 - 1) as usize;
            self.ensure_visible();
        }
    }

    pub fn activate_row(&mut self, index: usize) -> PanelAction {
        self.activate(index)
    }

    fn activate(&mut self, index: usize) -> PanelAction {
        let command = self
            .matches()
            .get(index)
            .and_then(|row| match row.status.as_str() {
                "available" => Some(format!("<Esc>:LspInstall {}<CR>", row.name)),
                _ => None,
            });
        self.selected = index;
        self.ensure_visible();
        command.map_or(PanelAction::None, PanelAction::Run)
    }

    pub fn handle_key(&mut self, key: egui::Key, modifiers: egui::Modifiers) -> PanelAction {
        match key {
            egui::Key::Escape => self.open = false,
            egui::Key::ArrowDown | egui::Key::J => self.step(1),
            egui::Key::ArrowUp | egui::Key::K => self.step(-1),
            egui::Key::PageDown => self.step(MAX_ROWS as i32),
            egui::Key::PageUp => self.step(-(MAX_ROWS as i32)),
            egui::Key::Home => {
                self.selected = 0;
                self.ensure_visible();
            }
            egui::Key::End => {
                self.selected = self.matches().len().saturating_sub(1);
                self.ensure_visible();
            }
            egui::Key::Enter => return self.activate(self.selected),
            egui::Key::Delete => {
                let command = self
                    .matches()
                    .get(self.selected)
                    .filter(|row| row.status != "available" && row.status != "installing")
                    .map(|row| format!("<Esc>:LspUninstall {}<CR>", row.name));
                if let Some(command) = command {
                    return PanelAction::Run(command);
                }
            }
            _ => {}
        }
        let _ = modifiers;
        PanelAction::None
    }

    /// Plain text keys: `j`/`k` move the selection (there is no filter box).
    pub fn handle_text(&mut self, text: &str) {
        match text {
            "j" => self.step(1),
            "k" => self.step(-1),
            _ => {}
        }
    }

    /// Geometry of the drop-down panel: `(rect, visible_rows)`.
    fn panel_metrics(&self, rect: Rect, row_height: f32) -> (Rect, usize) {
        let visible = self.visible_count();
        let width = (rect.width() * 0.82).clamp(320.0, 760.0).min(rect.width());
        // Title + `visible` rows + footer.
        let height = (visible + 2) as f32 * row_height;
        let panel = Rect::from_min_size(
            Pos2::new(rect.center().x - width / 2.0, rect.min.y + 12.0),
            egui::vec2(width, height.min(rect.height())),
        );
        (panel, visible)
    }

    pub fn panel_rect(&self, rect: Rect, row_height: f32) -> (Rect, usize) {
        self.panel_metrics(rect, row_height)
    }

    pub fn paint(
        &self,
        painter: &Painter,
        rect: Rect,
        row_height: f32,
        font: &FontId,
        small: &FontId,
        theme: &crate::ui::theme::Theme,
        click: Option<Pos2>,
    ) -> Option<usize> {
        let matches = self.matches();
        let (panel, visible) = self.panel_metrics(rect, row_height);
        let width = panel.width();
        let is_light = theme.is_light();

        // Drop shadow for a floating, Mason-like feel.
        painter.rect_filled(
            panel.translate(egui::vec2(0.0, 4.0)),
            6.0,
            Color32::from_black_alpha(if is_light { 30 } else { 70 }),
        );
        painter.rect_filled(panel, 6.0, theme.surface());
        painter.rect_stroke(
            panel,
            6.0,
            Stroke::new(1.0, theme.border()),
            egui::StrokeKind::Inside,
        );

        // ── Title bar (plain surface — no accent tint) ───────────────────────
        let title_rect = Rect::from_min_size(panel.min, egui::vec2(width, row_height));
        painter.text(
            Pos2::new(title_rect.min.x + 12.0, title_rect.center().y),
            Align2::LEFT_CENTER,
            "LANGUAGE SERVERS",
            font.clone(),
            theme.text,
        );
        let installed = self.rows.iter().filter(|r| r.status != "available").count();
        painter.text(
            Pos2::new(title_rect.max.x - 12.0, title_rect.center().y),
            Align2::RIGHT_CENTER,
            format!("{}/{} installed", installed, self.rows.len()),
            small.clone(),
            theme.muted,
        );
        painter.line_segment(
            [
                Pos2::new(panel.min.x, title_rect.max.y),
                Pos2::new(panel.max.x, title_rect.max.y),
            ],
            Stroke::new(1.0, theme.border()),
        );

        // ── Rows ─────────────────────────────────────────────────────────────
        let list_top = panel.min.y + row_height;
        let list_bottom = panel.max.y - row_height; // reserve footer
        let start = self.scroll_offset.min(self.max_offset());
        let end = (start + visible).min(matches.len());
        let name_x = panel.min.x + 30.0;
        let desc_x = panel.min.x + width * 0.34;
        let status_right = panel.max.x - 18.0;
        let desc_clip = Rect::from_min_max(
            Pos2::new(desc_x, list_top),
            Pos2::new(status_right - 90.0, list_bottom),
        );

        let mut clicked = None;
        for (slot, index) in (start..end).enumerate() {
            let row = matches[index];
            let y = list_top + slot as f32 * row_height;
            let line = Rect::from_min_size(Pos2::new(panel.min.x, y), egui::vec2(width, row_height));
            let is_selected = index == self.selected;

            if is_selected {
                painter.rect_filled(
                    line.shrink2(egui::vec2(6.0, 1.0)),
                    4.0,
                    theme.accent.linear_multiply(if is_light { 0.22 } else { 0.28 }),
                );
            }
            if click.is_some_and(|p| line.contains(p)) {
                clicked = Some(index);
            }

            let (mark, color, label) = match row.status.as_str() {
                "running" => ("●", Color32::from_rgb(80, 210, 120), "running"),
                "installed" => ("●", theme.accent, "installed"),
                "installing" => ("◐", theme.highlight, "installing"),
                _ => ("○", theme.muted, "not installed"),
            };
            painter.text(
                Pos2::new(line.min.x + 12.0, line.center().y),
                Align2::LEFT_CENTER,
                mark,
                small.clone(),
                color,
            );
            painter.text(
                Pos2::new(name_x, line.center().y),
                Align2::LEFT_CENTER,
                &row.name,
                font.clone(),
                theme.text,
            );
            painter.with_clip_rect(desc_clip).text(
                Pos2::new(desc_x, line.center().y),
                Align2::LEFT_CENTER,
                &row.desc,
                small.clone(),
                theme.muted,
            );
            painter.text(
                Pos2::new(status_right, line.center().y),
                Align2::RIGHT_CENTER,
                label,
                small.clone(),
                color,
            );
        }

        if matches.is_empty() {
            painter.text(
                Pos2::new(panel.min.x + 12.0, list_top + row_height * 0.5),
                Align2::LEFT_CENTER,
                "No language servers registered",
                small.clone(),
                theme.muted,
            );
        }

        // ── Scrollbar ────────────────────────────────────────────────────────
        if matches.len() > visible {
            let track = Rect::from_min_max(
                Pos2::new(panel.max.x - 6.0, list_top + 2.0),
                Pos2::new(panel.max.x - 3.0, list_bottom - 2.0),
            );
            painter.rect_filled(track, 1.5, theme.border());
            let frac = visible as f32 / matches.len() as f32;
            let thumb_h = (track.height() * frac).max(14.0);
            let denom = self.max_offset().max(1) as f32;
            let thumb_y = track.min.y + (track.height() - thumb_h) * (start as f32 / denom);
            painter.rect_filled(
                Rect::from_min_size(Pos2::new(track.min.x, thumb_y), egui::vec2(3.0, thumb_h)),
                1.5,
                theme.accent.linear_multiply(0.8),
            );
        }

        // ── Footer ───────────────────────────────────────────────────────────
        painter.text(
            Pos2::new(panel.min.x + 12.0, panel.max.y - row_height * 0.5),
            Align2::LEFT_CENTER,
            "j/k or ↑↓ move · ⏎ install · ⌦ uninstall · esc close",
            small.clone(),
            theme.text.linear_multiply(0.45),
        );
        clicked
    }

    pub fn contains(&self, rect: Rect, row_height: f32, pos: Pos2) -> bool {
        self.panel_metrics(rect, row_height).0.contains(pos)
    }

    /// Wheel scrolling moves the viewport (not the selection cursor).
    pub fn scroll(&mut self, delta: i32) {
        let max = self.max_offset();
        self.scroll_offset = (self.scroll_offset as i64 + delta as i64).clamp(0, max as i64) as usize;
    }
}
