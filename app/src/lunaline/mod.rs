//! LunaLine — sleek, modular statusline under the editor.
//!
//! Inspired by Neovim's lualine with modern, tactile GUI aesthetics.
//! Provides multiple style presets (Pill, Powerline, Floating, Minimal),
//! mode-reactive dynamic colors, and customizable components (word count, reading time,
//! cursor position, progress percentage, encoding badge, and dirty indicators).

pub mod render;
pub mod types;

pub use render::{render_lunaline, render_lunaline_preview, LunaLineRenderParams};
pub use types::{LunaColorMode, LunaLineConfig, LunaStyle};
