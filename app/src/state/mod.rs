//! Sub-structs for splitting the monolithic `App` state into logical groups.
//!
//! Each module owns one cohesive group of fields (sidebar, modals, tabs, etc.)
//! to keep the main `App` struct readable and maintainable.

pub mod activity;
pub mod command_bar;
pub mod editor;
pub mod misc;
pub mod modal;
pub mod notes;
pub mod right_pane;
pub mod scan;
pub mod services;
pub mod sidebar;
pub mod tabs;
pub mod terminal;
