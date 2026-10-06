//! MindForge reusable context menu system.
//!
//! Provides a standardized, professional IDE context menu component
//! (`menu_container`) with consistent geometry tokens, sharp borders, screen-edge
//! clamping, and decoupled action models.

pub mod menu_container;
pub mod menu_item;
pub mod menu_model;
pub mod menu_separator;

pub use menu_container::{render_menu_container, MenuState, DEFAULT_MENU_WIDTH};
pub use menu_item::render_menu_item;
pub use menu_model::{file_menu, folder_menu, root_menu, MenuAction, MenuItem, MenuTarget};
pub use menu_separator::render_menu_separator;
