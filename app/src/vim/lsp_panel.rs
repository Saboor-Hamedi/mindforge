//! Scrollable, filterable language-server browser opened by `:LspList`.
//! Neovim only sends the rows; selection, filtering and drawing live here so
//! the list stays responsive no matter how many servers are registered.

use eframe::egui::{self, Align2, FontId, Painter, Pos2, Rect, Stroke};

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
    filter: String,
    selected: usize,
}

impl LspPanel {
    pub fn update(&mut self, rows: Vec<LspRow>, open: bool, filter: &str) {
        self.rows = rows;
        if open {
            self.open = true;
            self.filter = filter.trim().to_owned();
            self.selected = 0;
        }
        self.selected = self.selected.min(self.matches().len().saturating_sub(1));
    }

    fn matches(&self) -> Vec<&LspRow> {
        let needle = self.filter.to_lowercase();
        let mut rows: Vec<&LspRow> = self
            .rows
            .iter()
            .filter(|row| {
                needle.is_empty()
                    || [&row.name, &row.desc, &row.filetypes, &row.status]
                        .iter()
                        .any(|field| field.to_lowercase().contains(&needle))
            })
            .collect();
        // Installed servers first, then alphabetical (rows arrive sorted).
        rows.sort_by_key(|row| row.status == "available");
        rows
    }

    fn step(&mut self, delta: i32) {
        let total = self.matches().len();
        if total > 0 {
            self.selected = (self.selected as i64 + delta as i64).clamp(0, total as i64 - 1) as usize;
        }
    }

    pub fn activate_row(&mut self, index: usize) -> PanelAction {
        self.activate(index)
    }

    fn activate(&mut self, index: usize) -> PanelAction {
        let command = self.matches().get(index).and_then(|row| match row.status.as_str() {
            "available" => Some(format!("<Esc>:LspInstall {}<CR>", row.name)),
            _ => None,
        });
        self.selected = index;
        command.map_or(PanelAction::None, PanelAction::Run)
    }

    pub fn handle_key(&mut self, key: egui::Key, modifiers: egui::Modifiers) -> PanelAction {
        match key {
            egui::Key::Escape => self.open = false,
            egui::Key::ArrowDown => self.step(1),
            egui::Key::ArrowUp => self.step(-1),
            egui::Key::PageDown => self.step(MAX_ROWS as i32),
            egui::Key::PageUp => self.step(-(MAX_ROWS as i32)),
            egui::Key::Home => self.selected = 0,
            egui::Key::End => self.selected = self.matches().len().saturating_sub(1),
            egui::Key::J if modifiers.ctrl => self.step(1),
            egui::Key::K if modifiers.ctrl => self.step(-1),
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
            egui::Key::Backspace => {
                self.filter.pop();
                self.selected = 0;
            }
            _ => {}
        }
        PanelAction::None
    }

    pub fn handle_text(&mut self, text: &str) {
        if text.chars().all(|c| !c.is_control()) {
            self.filter.push_str(text);
            self.selected = 0;
        }
    }

    /// Draws the panel centred in `rect`; returns the row clicked, if any.
    pub fn panel_rect(&self, rect: Rect, row_height: f32) -> (Rect, usize) {
        let matches = self.matches();
        let visible = matches.len().clamp(1, MAX_ROWS);
        let width = (rect.width() * 0.8).clamp(280.0, 720.0).min(rect.width());
        let height = (visible + 2) as f32 * row_height;
        let panel = Rect::from_min_size(
            Pos2::new(rect.center().x - width / 2.0, rect.min.y + 12.0),
            egui::vec2(width, height.min(rect.height())),
        );
        (panel, visible)
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
        let (panel, visible) = self.panel_rect(rect, row_height);
        let width = panel.width();
        painter.rect_filled(panel, 4.0, theme.surface());
        painter.rect_stroke(panel, 4.0, Stroke::new(1.0, theme.border()), egui::StrokeKind::Outside);

        let installed = self.rows.iter().filter(|r| r.status != "available").count();
        let header = format!(
            "Language servers {}/{} installed  ▸ filter: {}▏",
            installed,
            self.rows.len(),
            self.filter
        );
        painter.text(panel.min + egui::vec2(8.0, 2.0), Align2::LEFT_TOP, header, font.clone(), theme.text);

        let start = (self.selected + 1).saturating_sub(visible).min(matches.len().saturating_sub(visible));
        let mut clicked = None;
        for (slot, index) in (start..(start + visible).min(matches.len())).enumerate() {
            let row = matches[index];
            let y = panel.min.y + (slot + 1) as f32 * row_height;
            let line = Rect::from_min_size(Pos2::new(panel.min.x, y), egui::vec2(width, row_height));
            if index == self.selected {
                painter.rect_filled(line, 0.0, theme.accent.linear_multiply(0.25));
            }
            if click.is_some_and(|p| line.contains(p)) {
                clicked = Some(index);
            }
            let (mark, color) = match row.status.as_str() {
                "running" => ("●", theme.accent),
                "installed" => ("●", theme.text),
                "installing" => ("◐", theme.highlight),
                _ => ("○", theme.text.linear_multiply(0.45)),
            };
            painter.text(Pos2::new(line.min.x + 8.0, y), Align2::LEFT_TOP, mark, small.clone(), color);
            painter.text(Pos2::new(line.min.x + 24.0, y), Align2::LEFT_TOP, &row.name, font.clone(), theme.text);
            painter.text(
                Pos2::new(line.min.x + 24.0 + width * 0.22, y),
                Align2::LEFT_TOP,
                &row.desc,
                small.clone(),
                theme.text.linear_multiply(0.6),
            );
            painter.text(
                Pos2::new(line.max.x - 8.0, y),
                Align2::RIGHT_TOP,
                &row.status,
                small.clone(),
                theme.text.linear_multiply(0.45),
            );
        }
        if matches.is_empty() {
            painter.text(
                panel.min + egui::vec2(8.0, row_height),
                Align2::LEFT_TOP,
                "No matching language server",
                small.clone(),
                theme.text.linear_multiply(0.5),
            );
        }
        painter.text(
            Pos2::new(panel.max.x - 8.0, panel.max.y - 2.0),
            Align2::RIGHT_BOTTOM,
            "↑↓ move · Enter install · Del uninstall · Esc close",
            small.clone(),
            theme.text.linear_multiply(0.4),
        );
        clicked
    }

    pub fn contains(&self, rect: Rect, row_height: f32, pos: Pos2) -> bool {
        self.panel_rect(rect, row_height).0.contains(pos)
    }

    pub fn scroll(&mut self, delta: i32) {
        self.step(delta);
    }
}
