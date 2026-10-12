//! Caret subsystem: cursor state, animated styles, and particle physics.
//!
//! Mirrors the `terminal` module layout — this `mod.rs` declares the public API
//! and re-exports the coordinator, while every caret effect lives in its own file.

pub mod beam;
pub mod candle;
pub mod caret;
pub mod effects;
pub mod fire;
pub mod neon;
pub mod particles;
pub mod rainbow;
pub mod snow;
pub mod water;

// Aliases matching user naming patterns
#[allow(unused_imports)]
pub use beam as beamcaret;
#[allow(unused_imports)]
pub use candle as candlecaret;
#[allow(unused_imports)]
pub use effects as effectscaret;
#[allow(unused_imports)]
pub use fire as firecaret;
#[allow(unused_imports)]
pub use neon as neoncaret;
#[allow(unused_imports)]
pub use particles as particlescaret;
#[allow(unused_imports)]
pub use rainbow as rainbowcaret;
#[allow(unused_imports)]
pub use snow as snowcaret;
#[allow(unused_imports)]
pub use water as watercaret;

pub use caret::{resolve_caret_kind, Caret, CaretKind};
