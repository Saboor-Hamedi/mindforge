//! View renderers for MINDFORGE's major UI screens.
//!
//! Each module owns one full-screen or large-panel view:
//! - `dashboard`: Welcome screen and quick-action launcher
//! - `stats`: Learning statistics and writing analytics
//! - `scan`: Web security scan report display
//! - `scan_history`: Historical scan results browser

pub mod dashboard;
pub mod scan;
pub mod scan_history;
pub mod stats;
