//! Memory reading utilities for cross-platform autosplitter
//!
//! Provides memory reading primitives, pattern scanning, and process management.

pub mod reader;
pub mod pointer;
pub mod process;

pub use reader::*;
pub use pointer::Pointer;
pub use process::*;
