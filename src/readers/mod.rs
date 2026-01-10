//! Flag reading implementations for different game algorithms
//!
//! Each algorithm corresponds to how a specific FromSoftware game stores event flags.

pub mod flag_reader;
pub mod plugin_reader;

pub use flag_reader::{FlagReader, MemoryContext, create_flag_reader};
