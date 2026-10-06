//! Reusable MindForge menu container primitive.
//!
//! Owns the common menu geometry, crisp non-rounded borders, positioning bounds
//! checking (screen edge clamping), keyboard navigation, and outside-click dismiss.

use super::menu_item::{render_menu_item, ITEM_HEIGHT};
use super::menu_model::{MenuAction, MenuItem};
use super::menu_separator::{render_menu_separator, SEPARATOR_TOTAL_H};
use crate::ui::theme::Theme;
use eframe::egui::{self, pos2, vec2, Color32, Pos2, Rect, Stroke};

pub const DEFAULT_MENU_WIDTH: f32 = 210.0;
pub const MENU_VERTICAL_PAD: f32 = 4.0;
pub const MENU_BORDER_WIDTH: f32 = 1.0;

/// State for an open menu instance.
#[derive(Debug, Clone)]
pub struct MenuState {
    pub position: Pos2,
    pub items: Vec<MenuItem>,
    pub selected_index: Option<usize>,
    pub just_opened: bool,
}

impl MenuState {
    pub fn new(position: Pos2, items: Vec<MenuItem>) -> Self {
        Self {
            position,
            items,
            selected_index: None,
            just_opened: true,
        }
    }
}

/// Renders a reusable context menu container and handles all interaction.
///
/// Returns `(Option<MenuAction>, should_close_boolean)`.
pub fn render_menu_container(
    ctx: &egui::Context,
    state: &mut MenuState,
    theme: &Theme,
    opacity: f32,
) -> (Option<MenuAction>, bool) {
    let mut triggered_action = None;
    let mut should_close = false;

    let window_rect = ctx.screen_rect();
    let menu_w = DEFAULT_MENU_WIDTH;

    // Calculate total menu content height
    let mut total_h = MENU_VERTICAL_PAD * 2.0;
    for item in &state.items {
        match item {
            MenuItem::Action { .. } => total_h += ITEM_HEIGHT,
            MenuItem::Separator => total_h += SEPARATOR_TOTAL_H,
        }
    }

    // Boundary Repositioning:
    // normal -> open below/right
    // near bottom -> open above
    // near right edge -> shift left
    // near corner -> adjust both
    let margin = 6.0;
    let right_bound = window_rect.max.x - margin;
    let bottom_bound = window_rect.max.y - 38.0; // strictly clear sidebar footer and statusbar

    let mut x = state.position.x;
    let mut y = state.position.y;

    if x + menu_w > right_bound {
        x = state.position.x - menu_w;
    }
    x = x.clamp(
        window_rect.min.x + margin,
        (right_bound - menu_w).max(window_rect.min.x + margin),
    );

    if y + total_h > bottom_bound {
        y = state.position.y - total_h;
    }
    y = y.clamp(
        window_rect.min.y + margin,
        (bottom_bound - total_h).max(window_rect.min.y + margin),
    );

    let menu_rect = Rect::from_min_size(pos2(x, y), vec2(menu_w, total_h));

    // Handle Escape and outside-click dismiss
    let escape_pressed = ctx.input(|i| i.key_pressed(egui::Key::Escape));
    if escape_pressed {
        return (None, true);
    }

    let primary_clicked = ctx.input(|i| i.pointer.primary_clicked());
    let secondary_clicked = ctx.input(|i| i.pointer.secondary_clicked());
    let pointer_pos = ctx.input(|i| i.pointer.interact_pos().or_else(|| i.pointer.hover_pos()));

    if !state.just_opened && (primary_clicked || secondary_clicked) {
        if let Some(pos) = pointer_pos {
            if !menu_rect.contains(pos) {
                return (None, true);
            }
        }
    }
    state.just_opened = false;

    // Keyboard navigation (ArrowUp, ArrowDown, Enter)
    let action_indices: Vec<usize> = state
        .items
        .iter()
        .enumerate()
        .filter_map(|(idx, item)| match item {
            MenuItem::Action {
                disabled: false, ..
            } => Some(idx),
            _ => None,
        })
        .collect();

    if !action_indices.is_empty() {
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
            state.selected_index = match state.selected_index {
                None => action_indices.first().copied(),
                Some(curr) => {
                    let next_pos = action_indices
                        .iter()
                        .position(|&i| i == curr)
                        .map(|p| (p + 1) % action_indices.len())
                        .unwrap_or(0);
                    Some(action_indices[next_pos])
                }
            };
        } else if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
            state.selected_index = match state.selected_index {
                None => action_indices.last().copied(),
                Some(curr) => {
                    let prev_pos = action_indices
                        .iter()
                        .position(|&i| i == curr)
                        .map(|p| {
                            if p == 0 {
                                action_indices.len() - 1
                            } else {
                                p - 1
                            }
                        })
                        .unwrap_or(0);
                    Some(action_indices[prev_pos])
                }
            };
        } else if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
            if let Some(idx) = state.selected_index {
                if let Some(MenuItem::Action {
                    action,
                    disabled: false,
                    ..
                }) = state.items.get(idx)
                {
                    triggered_action = Some(action.clone());
                    should_close = true;
                }
            }
        }
    }

    // Render floating menu using an egui Area layer above everything
    let area = egui::Area::new(egui::Id::new("mindforge_context_menu_area"))
        .fixed_pos(menu_rect.min)
        .order(egui::Order::Foreground);

    area.show(ctx, |ui| {
        ui.set_clip_rect(window_rect);

        // Styling: crisp sharp IDE border, NO rounded corners (radius 0.0), opaque surface
        let bg_color = if opacity >= 0.99 {
            theme.surface()
        } else {
            let alpha = ((opacity * 255.0) as u8).max(235);
            Color32::from_rgba_unmultiplied(
                theme.surface().r(),
                theme.surface().g(),
                theme.surface().b(),
                alpha,
            )
        };

        // Draw shadow / border / sharp background (2px radius)
        let painter = ui.painter();
        painter.rect(
            menu_rect,
            2.0, // Sharp, professional 2px radius
            bg_color,
            Stroke::new(MENU_BORDER_WIDTH, theme.border().gamma_multiply(0.75)),
            egui::StrokeKind::Inside,
        );

        ui.allocate_new_ui(
            egui::UiBuilder::new()
                .max_rect(menu_rect.shrink(MENU_BORDER_WIDTH))
                .layout(egui::Layout::top_down(egui::Align::Min)),
            |ui| {
                ui.add_space(MENU_VERTICAL_PAD);

                let item_w = menu_w - MENU_BORDER_WIDTH * 2.0;

                for (idx, item) in state.items.iter().enumerate() {
                    match item {
                        MenuItem::Separator => {
                            render_menu_separator(ui, item_w, theme.border().gamma_multiply(0.35));
                        }
                        MenuItem::Action {
                            label,
                            icon,
                            shortcut,
                            action,
                            destructive,
                            disabled,
                            ..
                        } => {
                            let is_kbd_sel = state.selected_index == Some(idx);
                            let (_, clicked) = render_menu_item(
                                ui,
                                item_w,
                                label,
                                *icon,
                                *shortcut,
                                *destructive,
                                *disabled,
                                is_kbd_sel,
                                theme,
                            );

                            if clicked {
                                triggered_action = Some(action.clone());
                                should_close = true;
                            }
                        }
                    }
                }
            },
        );
    });

    (triggered_action, should_close)
}
