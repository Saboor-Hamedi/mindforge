#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod accent;
pub mod agent;
mod app;
mod caret;
pub mod command;
mod editor;
mod input;
mod language;
pub mod layout;
mod lunaline;
mod modals;
mod mode;
mod notes;
pub mod services;
pub mod setting;
pub mod state;
mod statusbar;
mod types;
pub mod ui;
pub mod ui_components;
mod view_editor;
pub mod vim;
pub use setting as settings;
pub mod rightsidebar;
pub mod views;
pub mod wikilink;
pub mod workspace;
pub mod workspace_import;

#[path = "sidebar/sidebar.rs"]
mod sidebar;

pub use types::{snapshot, visual_line};

use app::App;
use eframe::egui;

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let is_new_window = args.iter().any(|a| a == "--new-window");
    let mut initial_pos = None;
    for i in 0..args.len() {
        if args[i] == "--pos-x" && i + 1 < args.len() {
            if let Ok(x) = args[i + 1].parse::<f32>() {
                for j in 0..args.len() {
                    if args[j] == "--pos-y" && j + 1 < args.len() {
                        if let Ok(y) = args[j + 1].parse::<f32>() {
                            initial_pos = Some([x, y]);
                            break;
                        }
                    }
                }
            }
        }
    }

    let icon_bytes = include_bytes!("../assets/icon_64.rgba");
    let icon_data = egui::IconData {
        rgba: icon_bytes.to_vec(),
        width: 64,
        height: 64,
    };
    let mut viewport = egui::ViewportBuilder::default()
        .with_icon(icon_data)
        .with_app_id("app.mindforge.MindForge")
        .with_decorations(false)
        .with_transparent(true)
        .with_inner_size([1120.0, 740.0])
        .with_min_inner_size([700.0, 500.0])
        .with_max_inner_size([2560.0, 1440.0])
        .with_resizable(true)
        .with_drag_and_drop(true);

    if let Some(pos) = initial_pos {
        viewport = viewport.with_position(pos);
    }

    let options = eframe::NativeOptions {
        viewport,
        vsync: true,
        ..Default::default()
    };

    eframe::run_native(
        "mindforge",
        options,
        Box::new(move |cc| {
            let app = App::new_with_flags(is_new_window);
            services::font_manager::apply_font(&cc.egui_ctx, &app.misc.selected_font);
            Ok(Box::new(app))
        }),
    )
}
