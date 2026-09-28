#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
pub mod accent;
mod statusbar;
mod caret;
pub mod command;
mod db_worker;
mod editor;
mod fuzzy;
mod input;
pub mod layout;
mod modals;
mod mode;
mod notes;
pub mod palette;
pub mod settings;
pub mod lunaline;
#[path = "sidebar/sidebar.rs"]
mod sidebar;
mod sound;
mod theme;
mod types;
mod view_editor;
mod view_stats;
pub mod hybrid;
pub mod vim;
mod showcmd;
mod updater;
mod docs;
mod help_panel;
mod scan_history_view;
mod scan_view;
pub mod terminal_pane;
pub mod ui_components;
pub mod zoom;
pub mod agent;
pub mod blur;
pub mod font_manager;
pub mod view_dashboard;
pub mod workspace_import;
pub mod wikilink;
pub mod rightsidebar;

pub use types::{snapshot, visual_line};

use app::App;
use eframe::egui;

fn main() -> eframe::Result<()> {
    let icon_bytes = include_bytes!("../assets/icon_64.rgba");
    let icon_data = egui::IconData {
        rgba: icon_bytes.to_vec(),
        width: 64,
        height: 64,
    };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_icon(icon_data)
            .with_app_id("app.mindforge.MindForge")
            .with_decorations(false) // Borderless: NO title bar, NO borders
            .with_transparent(true)  // Enables opacity and rounded edges
            .with_inner_size([1120.0, 740.0])
            .with_min_inner_size([700.0, 500.0])
            .with_max_inner_size([2560.0, 1440.0])
            .with_resizable(true)
            .with_drag_and_drop(true),
        vsync: true,
        ..Default::default()
    };

    eframe::run_native(
        "mindforge",
        options,
        Box::new(|cc| {
            let app = App::new();
            font_manager::apply_font(&cc.egui_ctx, &app.selected_font);
            Ok(Box::new(app))
        }),
    )
}
