//! Embedded terminal subsystem for MINDFORGE's `:term` command.
//!
//! The terminal runs as a real PTY process (via `egui_term` / `alacritty_terminal`)
//! rather than an emulator, ensuring full compatibility with CLI tools, TUI apps,
//! and shell scripts. The subsystem is split by responsibility:
//!
//! - [`process`]  — shell discovery and PTY process lifecycle
//! - [`session`]  — individual terminal sessions and their management
//! - [`events`]   — PTY output pumping and exit handling
//! - [`input`]    — keyboard → ANSI translation and event forwarding
//! - [`render`]   — the docked terminal UI and resize splitter
//! - [`state`]    — the dock visibility/focus state owned by the application
//! - [`terminal`] — the top-level coordinator and shared types

pub mod events;
pub mod input;
pub mod process;
pub mod render;
pub mod session;
pub mod state;
pub mod terminal;

pub use state::TerminalState;
pub use terminal::{TerminalAction, TerminalPane};
