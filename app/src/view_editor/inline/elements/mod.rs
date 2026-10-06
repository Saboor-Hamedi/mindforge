//! Dedicated modular components for preview elements.
//!
//! Each element (block_quote, code_wrapper, table, heading, task)
//! is encapsulated in its own focused and DRY module.

pub mod block_quote;
pub mod code_wrapper;
pub mod heading;
pub mod table;
pub mod task;

pub use block_quote::quote_color;
pub use code_wrapper::{code_block_copy_button_rect, render_code_block_card};
pub use heading::heading_color;
pub use table::{
    cell_color, render_table_block_decorations, render_table_row_decorations, table_metrics,
};
pub use task::render_task_checkbox;
