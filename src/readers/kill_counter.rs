//! Kill counter reader (Dark Souls 2)
//!
//! DS2 tracks boss kills with counters instead of simple flags.

use super::FlagReader;
use crate::memory::MemoryReader;

/// Kill counter-based reader used by Dark Souls 2
pub struct KillCounter {
    /// Base address of the kill counter data
    base_address: usize,
    /// Offset from base to counter array
    counter_array_offset: usize,
    /// Size of each counter entry
    entry_size: usize,
}

impl KillCounter {
    /// Create a new kill counter reader
    pub fn new(base_address: usize, counter_array_offset: usize, entry_size: usize) -> Self {
        Self {
            base_address,
            counter_array_offset,
            entry_size,
        }
    }

    /// Read the actual kill count value
    pub fn read_count(&self, reader: &dyn MemoryReader, flag_id: u32) -> u32 {
        // Read base pointer
        let Some(base_ptr) = reader.read_ptr(self.base_address) else {
            return 0;
        };

        if base_ptr == 0 {
            return 0;
        }

        // Navigate to counter array
        let counter_base = base_ptr + self.counter_array_offset;

        // Calculate entry position (flag_id maps to entry index)
        let entry_address = counter_base + (flag_id as usize * self.entry_size);

        // Read the counter value (typically u32)
        reader.read_u32(entry_address).unwrap_or(0)
    }
}

impl FlagReader for KillCounter {
    fn is_flag_set(&self, reader: &dyn MemoryReader, flag_id: u32) -> bool {
        self.read_count(reader, flag_id) > 0
    }

    fn get_kill_count(&self, reader: &dyn MemoryReader, flag_id: u32) -> u32 {
        self.read_count(reader, flag_id)
    }
}
