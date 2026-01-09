//! Offset table flag reader (Dark Souls 1)
//!
//! DS1 uses a simple offset table for event flags.

use super::FlagReader;
use crate::memory::MemoryReader;

/// Offset table-based flag reader used by Dark Souls 1
pub struct OffsetTableReader {
    /// Base address of the event flag data
    base_address: usize,
    /// Size of each flag block
    block_size: usize,
}

impl OffsetTableReader {
    /// Create a new offset table reader
    pub fn new(base_address: usize, block_size: usize) -> Self {
        Self {
            base_address,
            block_size,
        }
    }
}

impl FlagReader for OffsetTableReader {
    fn is_flag_set(&self, reader: &dyn MemoryReader, flag_id: u32) -> bool {
        // Calculate byte and bit position
        let byte_offset = (flag_id / 8) as usize;
        let bit_position = flag_id % 8;

        // Read the flag manager pointer first
        let Some(flags_ptr) = reader.read_ptr(self.base_address) else {
            return false;
        };

        if flags_ptr == 0 {
            return false;
        }

        // Read the byte containing the flag
        let Some(byte_value) = reader.read_bytes(flags_ptr + byte_offset, 1) else {
            return false;
        };

        // Check the specific bit
        (byte_value[0] & (1 << bit_position)) != 0
    }
}
