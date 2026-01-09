//! Core autosplitter abstractions
//!
//! This module contains the main types and traits for the autosplitter:
//! - `AutosplitterState` - Current state of the autosplitter
//! - `Autosplitter` - Main runner that orchestrates detection
//! - `SplitEvent` - Events emitted when splits are detected

mod state;
mod runner;
mod events;

pub use state::{AutosplitterState, BossFlag, SplitTriggerConfig};
pub use runner::Autosplitter;
pub use events::SplitEvent;
