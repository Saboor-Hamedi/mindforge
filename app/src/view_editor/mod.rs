//! Main text editor view: header, text canvas, smooth scrolling, and caret rendering.

pub mod inline;
pub mod ligatures;
pub mod preview;
pub mod tabs;
pub mod titlebar;

pub use preview::{render_markdown_document, render_markdown_preview};
pub use tabs::{render_tab_bar, TabAction, TabItem, TAB_ROW_H};
pub use titlebar::{render_full_titlebar, TitlebarAction};
