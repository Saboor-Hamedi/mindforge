//! Background services and utility engines for MINDFORGE.
//!
//! These modules run independently of the UI thread or provide shared
//! infrastructure used across multiple views:
//! - `db_worker`: Async background storage queue (prevents UI blocking)
//! - `updater`: GitHub release checker and binary downloader
//! - `font_manager`: Dynamic font discovery and egui typography loading
//! - `sound`: Mechanical keyboard audio synthesizer (16-channel PCM)
//! - `fuzzy`: Subsequence fuzzy matching for search and palette
//! - `blur`: Windows DWM Acrylic/Mica backdrop integration

pub mod blur;
pub mod db_worker;
pub mod font_manager;
pub mod fuzzy;
pub mod sound;
pub mod updater;
