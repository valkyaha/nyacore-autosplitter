//! Memory reading utilities for cross-platform autosplitter
//!
//! Provides memory reading primitives, pattern scanning, and process management.

pub mod reader;
pub mod pointer;
pub mod process;
pub mod traits;
pub mod abstract_pointer;

pub use reader::*;
pub use pointer::Pointer;
pub use process::*;
pub use traits::{MemoryReader, ProcessFinder, MockMemoryReader, MockProcessFinder};
pub use abstract_pointer::AbstractPointer;
