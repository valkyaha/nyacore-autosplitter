//! Category decomposition flag reader (DS3, Sekiro)
//!
//! Flags are organized by category, with each category having a bitmask array.
//! Flag ID format: CCCCXXXX where CCCC is category, XXXX is the flag within category.

use super::FlagReader;
use crate::memory::MemoryReader;

/// Category-based flag reader used by DS3 and Sekiro
pub struct CategoryDecomposition {
    /// Base address of the event flag manager
    base_address: usize,
    /// Offset to the flag data from the manager
    flag_data_offset: usize,
    /// Category divisor (usually 1000 or 10000)
    category_divisor: u32,
}

impl CategoryDecomposition {
    /// Create a new category decomposition reader
    pub fn new(base_address: usize, flag_data_offset: usize, category_divisor: u32) -> Self {
        Self {
            base_address,
            flag_data_offset,
            category_divisor,
        }
    }

    /// Calculate category and bit position from flag ID
    fn decompose(&self, flag_id: u32) -> (u32, u32, u32) {
        let category = flag_id / self.category_divisor;
        let id_within_category = flag_id % self.category_divisor;
        let byte_offset = id_within_category / 8;
        let bit_position = id_within_category % 8;
        (category, byte_offset, bit_position)
    }
}

impl FlagReader for CategoryDecomposition {
    fn is_flag_set(&self, reader: &dyn MemoryReader, flag_id: u32) -> bool {
        let (category, byte_offset, bit_position) = self.decompose(flag_id);

        // Read the flag manager pointer
        let Some(manager_ptr) = reader.read_ptr(self.base_address) else {
            return false;
        };

        if manager_ptr == 0 {
            return false;
        }

        // Navigate to the category data
        // This is a simplified version - actual implementation needs category lookup
        let flag_data_base = manager_ptr + self.flag_data_offset;

        // Read the byte containing the flag
        let byte_address = flag_data_base + (category as usize * 0x100) + byte_offset as usize;
        let Some(byte_value) = reader.read_bytes(byte_address, 1) else {
            return false;
        };

        // Check the specific bit
        (byte_value[0] & (1 << bit_position)) != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decompose() {
        let reader = CategoryDecomposition::new(0, 0, 10000);

        // Flag 13000100 -> category 1300, id 100
        let (cat, byte, bit) = reader.decompose(13000100);
        assert_eq!(cat, 1300);
        assert_eq!(byte, 12); // 100 / 8
        assert_eq!(bit, 4);   // 100 % 8
    }
}
