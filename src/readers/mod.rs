//! Flag reading algorithms for different games
//!
//! Each game uses a different data structure to store event flags.
//! This module provides implementations for each algorithm.

mod category;
mod binary_tree;
mod offset_table;
mod kill_counter;

pub use category::CategoryDecomposition;
pub use binary_tree::BinaryTreeReader;
pub use offset_table::OffsetTableReader;
pub use kill_counter::KillCounter;

use crate::memory::MemoryReader;

/// Trait for reading event flags from game memory
pub trait FlagReader: Send + Sync {
    /// Check if a flag is set
    fn is_flag_set(&self, reader: &dyn MemoryReader, flag_id: u32) -> bool;

    /// Get the kill count for a boss flag (returns 1 if flag is set, 0 otherwise by default)
    fn get_kill_count(&self, reader: &dyn MemoryReader, flag_id: u32) -> u32 {
        if self.is_flag_set(reader, flag_id) { 1 } else { 0 }
    }
}

/// Context for flag reading with pre-resolved addresses
pub struct FlagReaderContext {
    /// Base address for flag storage
    pub base_address: usize,
    /// Additional game-specific data
    pub extra_data: Vec<usize>,
}

impl FlagReaderContext {
    /// Create a new flag reader context
    pub fn new(base_address: usize) -> Self {
        Self {
            base_address,
            extra_data: Vec::new(),
        }
    }

    /// Add extra data (game-specific addresses)
    pub fn with_extra(mut self, data: usize) -> Self {
        self.extra_data.push(data);
        self
    }
}
