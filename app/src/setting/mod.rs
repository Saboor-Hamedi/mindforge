//! Settings module: modal container, tab navigation, feature panels, and properties.

pub mod ai_engine;
pub mod appearance;
pub mod backup;
pub mod carets;
pub mod keybindings_tab;
pub mod keymap;
pub mod lunaline_tab;
pub mod properties;
pub mod setting_container;
pub mod setting_font;
pub mod setting_panel;
pub mod setting_tab;
pub mod shortcuts;
pub mod sound;
pub mod sounds;
pub mod tabs;
pub mod theme;
pub mod types;
pub mod updates;

pub use setting_container::render_setting_container;
pub use setting_panel::{render_setting_panel, SettingPanelAction};
pub use setting_tab::{render_setting_tabs, SettingTab};
