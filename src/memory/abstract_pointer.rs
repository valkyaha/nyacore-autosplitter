//! Abstract pointer for testable memory access
//!
//! This module provides a Pointer implementation that works with any MemoryReader,
//! enabling unit testing without actual process handles.

use std::sync::Arc;
use super::traits::MemoryReader;

/// Abstract pointer that works with any MemoryReader implementation
#[derive(Clone)]
pub struct AbstractPointer {
    reader: Arc<dyn MemoryReader>,
    is_64_bit: bool,
    base_address: i64,
    offsets: Vec<i64>,
}

impl AbstractPointer {
    /// Create a new abstract pointer
    pub fn new(reader: Arc<dyn MemoryReader>, is_64_bit: bool, base_address: i64, offsets: Vec<i64>) -> Self {
        Self {
            reader,
            is_64_bit,
            base_address,
            offsets,
        }
    }

    /// Create an uninitialized pointer (null)
    pub fn null(reader: Arc<dyn MemoryReader>) -> Self {
        Self {
            reader,
            is_64_bit: true,
            base_address: 0,
            offsets: Vec::new(),
        }
    }

    /// Initialize/reinitialize the pointer
    pub fn initialize(&mut self, base_address: i64, offsets: &[i64]) {
        self.base_address = base_address;
        self.offsets = offsets.to_vec();
    }

    /// Clear the pointer
    pub fn clear(&mut self) {
        self.base_address = 0;
        self.offsets.clear();
    }

    /// Create a copy of this pointer
    pub fn copy(&self) -> Self {
        Self {
            reader: self.reader.clone(),
            is_64_bit: self.is_64_bit,
            base_address: self.base_address,
            offsets: self.offsets.clone(),
        }
    }

    /// Creates a new pointer with the address of the old pointer as base address
    pub fn create_pointer_from_address(&self, offset: Option<i64>) -> Self {
        let mut offsets = self.offsets.clone();

        if let Some(off) = offset {
            offsets.push(off);
        }
        offsets.push(0);

        let new_base = self.resolve_offsets(&offsets);
        Self {
            reader: self.reader.clone(),
            is_64_bit: self.is_64_bit,
            base_address: new_base,
            offsets: Vec::new(),
        }
    }

    /// Append offsets to create a new pointer
    pub fn append(&self, offsets: &[i64]) -> Self {
        let mut new_offsets = self.offsets.clone();
        new_offsets.extend_from_slice(offsets);
        Self {
            reader: self.reader.clone(),
            is_64_bit: self.is_64_bit,
            base_address: self.base_address,
            offsets: new_offsets,
        }
    }

    /// Resolve offsets and return the final address
    fn resolve_offsets(&self, offsets: &[i64]) -> i64 {
        let mut ptr = self.base_address;

        for (i, &offset) in offsets.iter().enumerate() {
            let address = ptr + offset;

            // Not the last offset = resolve as pointer (dereference)
            if i + 1 < offsets.len() {
                if self.is_64_bit {
                    ptr = match self.reader.read_i64(address as usize) {
                        Some(v) => v,
                        None => return 0,
                    };
                } else {
                    ptr = match self.reader.read_i32(address as usize) {
                        Some(v) => v as i64,
                        None => return 0,
                    };
                }

                if ptr == 0 {
                    return 0;
                }
            } else {
                // Last offset: just add, no dereference
                ptr = address;
            }
        }

        ptr
    }

    /// Check if the pointer resolves to null
    pub fn is_null_ptr(&self) -> bool {
        self.get_address() == 0
    }

    /// Get the resolved address
    pub fn get_address(&self) -> i64 {
        self.resolve_offsets(&self.offsets)
    }

    /// Read i32 at optional offset
    pub fn read_i32(&self, offset: Option<i64>) -> i32 {
        let mut offsets_copy = self.offsets.clone();
        if let Some(off) = offset {
            offsets_copy.push(off);
        }
        let address = self.resolve_offsets(&offsets_copy);
        self.reader.read_i32(address as usize).unwrap_or(0)
    }

    /// Read u32 at optional offset
    pub fn read_u32(&self, offset: Option<i64>) -> u32 {
        let mut offsets_copy = self.offsets.clone();
        if let Some(off) = offset {
            offsets_copy.push(off);
        }
        let address = self.resolve_offsets(&offsets_copy);
        self.reader.read_u32(address as usize).unwrap_or(0)
    }

    /// Read i64 at optional offset
    pub fn read_i64(&self, offset: Option<i64>) -> i64 {
        let mut offsets_copy = self.offsets.clone();
        if let Some(off) = offset {
            offsets_copy.push(off);
        }
        let address = self.resolve_offsets(&offsets_copy);
        self.reader.read_i64(address as usize).unwrap_or(0)
    }

    /// Read u64 at optional offset
    pub fn read_u64(&self, offset: Option<i64>) -> u64 {
        let mut offsets_copy = self.offsets.clone();
        if let Some(off) = offset {
            offsets_copy.push(off);
        }
        let address = self.resolve_offsets(&offsets_copy);
        self.reader.read_u64(address as usize).unwrap_or(0)
    }

    /// Read byte at optional offset
    pub fn read_byte(&self, offset: Option<i64>) -> u8 {
        let mut offsets_copy = self.offsets.clone();
        if let Some(off) = offset {
            offsets_copy.push(off);
        }
        let address = self.resolve_offsets(&offsets_copy);
        self.reader.read_u8(address as usize).unwrap_or(0)
    }

    /// Read f32 at optional offset
    pub fn read_f32(&self, offset: Option<i64>) -> f32 {
        let mut offsets_copy = self.offsets.clone();
        if let Some(off) = offset {
            offsets_copy.push(off);
        }
        let address = self.resolve_offsets(&offsets_copy);
        self.reader.read_f32(address as usize).unwrap_or(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::MockMemoryReader;

    fn create_mock_reader() -> Arc<dyn MemoryReader> {
        Arc::new(MockMemoryReader::new())
    }

    #[test]
    fn test_abstract_pointer_null() {
        let reader = create_mock_reader();
        let ptr = AbstractPointer::null(reader);

        assert!(ptr.is_null_ptr());
        assert_eq!(ptr.get_address(), 0);
    }

    #[test]
    fn test_abstract_pointer_new() {
        let reader = create_mock_reader();
        let ptr = AbstractPointer::new(reader, true, 0x1000, vec![0x10]);

        assert_eq!(ptr.get_address(), 0x1010);
    }

    #[test]
    fn test_abstract_pointer_no_offsets() {
        let reader = create_mock_reader();
        let ptr = AbstractPointer::new(reader, true, 0x1000, vec![]);

        assert_eq!(ptr.get_address(), 0x1000);
    }

    #[test]
    fn test_abstract_pointer_single_offset() {
        let reader = create_mock_reader();
        let ptr = AbstractPointer::new(reader, true, 0x1000, vec![0x20]);

        // Single offset is just added (not dereferenced)
        assert_eq!(ptr.get_address(), 0x1020);
    }

    #[test]
    fn test_abstract_pointer_chain_resolution_64bit() {
        let mut mock = MockMemoryReader::new();

        // Set up chain: 0x1000 -> 0x2000 -> final at 0x2010
        mock.write_i64(0x1000, 0x2000);

        let reader: Arc<dyn MemoryReader> = Arc::new(mock);
        let ptr = AbstractPointer::new(reader, true, 0x1000, vec![0, 0x10]);

        // First offset (0): dereference 0x1000 to get 0x2000
        // Second offset (0x10): add to get 0x2010
        assert_eq!(ptr.get_address(), 0x2010);
    }

    #[test]
    fn test_abstract_pointer_chain_resolution_32bit() {
        let mut mock = MockMemoryReader::new();

        // Set up chain: 0x1000 -> 0x2000 -> final at 0x2010
        mock.write_i32(0x1000, 0x2000);

        let reader: Arc<dyn MemoryReader> = Arc::new(mock);
        let ptr = AbstractPointer::new(reader, false, 0x1000, vec![0, 0x10]);

        assert_eq!(ptr.get_address(), 0x2010);
    }

    #[test]
    fn test_abstract_pointer_multi_level_chain() {
        let mut mock = MockMemoryReader::new();

        // Set up 3-level chain: 0x1000 -> 0x2000 -> 0x3000 -> final at 0x3020
        mock.write_i64(0x1000, 0x2000);
        mock.write_i64(0x2000, 0x3000);

        let reader: Arc<dyn MemoryReader> = Arc::new(mock);
        let ptr = AbstractPointer::new(reader, true, 0x1000, vec![0, 0, 0x20]);

        // 0x1000 + 0 -> deref -> 0x2000
        // 0x2000 + 0 -> deref -> 0x3000
        // 0x3000 + 0x20 -> 0x3020
        assert_eq!(ptr.get_address(), 0x3020);
    }

    #[test]
    fn test_abstract_pointer_null_in_chain() {
        let mut mock = MockMemoryReader::new();

        // First pointer is valid, second is null
        mock.write_i64(0x1000, 0x2000);
        mock.write_i64(0x2000, 0); // null pointer

        let reader: Arc<dyn MemoryReader> = Arc::new(mock);
        let ptr = AbstractPointer::new(reader, true, 0x1000, vec![0, 0, 0x20]);

        // Should return 0 when hitting null
        assert!(ptr.is_null_ptr());
    }

    #[test]
    fn test_abstract_pointer_read_i32() {
        let mut mock = MockMemoryReader::new();
        mock.write_i32(0x1000, -12345);

        let reader: Arc<dyn MemoryReader> = Arc::new(mock);
        let ptr = AbstractPointer::new(reader, true, 0x1000, vec![]);

        assert_eq!(ptr.read_i32(None), -12345);
    }

    #[test]
    fn test_abstract_pointer_read_i32_with_offset() {
        let mut mock = MockMemoryReader::new();
        mock.write_i32(0x1010, -12345);

        let reader: Arc<dyn MemoryReader> = Arc::new(mock);
        let ptr = AbstractPointer::new(reader, true, 0x1000, vec![]);

        assert_eq!(ptr.read_i32(Some(0x10)), -12345);
    }

    #[test]
    fn test_abstract_pointer_read_u32() {
        let mut mock = MockMemoryReader::new();
        mock.write_u32(0x1000, 0xDEADBEEF);

        let reader: Arc<dyn MemoryReader> = Arc::new(mock);
        let ptr = AbstractPointer::new(reader, true, 0x1000, vec![]);

        assert_eq!(ptr.read_u32(None), 0xDEADBEEF);
    }

    #[test]
    fn test_abstract_pointer_read_u64() {
        let mut mock = MockMemoryReader::new();
        mock.write_u64(0x1000, 0x123456789ABCDEF0);

        let reader: Arc<dyn MemoryReader> = Arc::new(mock);
        let ptr = AbstractPointer::new(reader, true, 0x1000, vec![]);

        assert_eq!(ptr.read_u64(None), 0x123456789ABCDEF0);
    }

    #[test]
    fn test_abstract_pointer_read_i64() {
        let mut mock = MockMemoryReader::new();
        mock.write_i64(0x1000, -123456789);

        let reader: Arc<dyn MemoryReader> = Arc::new(mock);
        let ptr = AbstractPointer::new(reader, true, 0x1000, vec![]);

        assert_eq!(ptr.read_i64(None), -123456789);
    }

    #[test]
    fn test_abstract_pointer_read_byte() {
        let mut mock = MockMemoryReader::new();
        mock.write_u8(0x1000, 0x42);

        let reader: Arc<dyn MemoryReader> = Arc::new(mock);
        let ptr = AbstractPointer::new(reader, true, 0x1000, vec![]);

        assert_eq!(ptr.read_byte(None), 0x42);
    }

    #[test]
    fn test_abstract_pointer_read_f32() {
        let mut mock = MockMemoryReader::new();
        let value: f32 = 3.14159;
        mock.write_bytes(0x1000, &value.to_le_bytes());

        let reader: Arc<dyn MemoryReader> = Arc::new(mock);
        let ptr = AbstractPointer::new(reader, true, 0x1000, vec![]);

        let result = ptr.read_f32(None);
        assert!((result - value).abs() < 0.0001);
    }

    #[test]
    fn test_abstract_pointer_append() {
        let reader = create_mock_reader();
        let ptr = AbstractPointer::new(reader, true, 0x1000, vec![0x10]);

        let appended = ptr.append(&[0x20, 0x30]);

        // Original unchanged
        assert_eq!(ptr.get_address(), 0x1010);

        // New pointer has appended offsets
        // Note: with no dereferencing data, this will fail at the first deref
        // Just test that offsets are appended correctly
        assert_eq!(appended.offsets, vec![0x10, 0x20, 0x30]);
    }

    #[test]
    fn test_abstract_pointer_copy() {
        let reader = create_mock_reader();
        let ptr = AbstractPointer::new(reader, true, 0x1000, vec![0x10, 0x20]);

        let copied = ptr.copy();

        assert_eq!(copied.base_address, ptr.base_address);
        assert_eq!(copied.offsets, ptr.offsets);
        assert_eq!(copied.is_64_bit, ptr.is_64_bit);
    }

    #[test]
    fn test_abstract_pointer_initialize() {
        let reader = create_mock_reader();
        let mut ptr = AbstractPointer::null(reader);

        assert!(ptr.is_null_ptr());

        ptr.initialize(0x5000, &[0x10, 0x20]);

        assert_eq!(ptr.base_address, 0x5000);
        assert_eq!(ptr.offsets, vec![0x10, 0x20]);
    }

    #[test]
    fn test_abstract_pointer_clear() {
        let reader = create_mock_reader();
        let mut ptr = AbstractPointer::new(reader, true, 0x1000, vec![0x10, 0x20]);

        ptr.clear();

        assert_eq!(ptr.base_address, 0);
        assert!(ptr.offsets.is_empty());
    }

    #[test]
    fn test_abstract_pointer_create_from_address() {
        let mut mock = MockMemoryReader::new();

        // Set up: 0x1000 -> 0x2000
        mock.write_i64(0x1000, 0x2000);

        let reader: Arc<dyn MemoryReader> = Arc::new(mock);
        let ptr = AbstractPointer::new(reader, true, 0x1000, vec![]);

        // Create new pointer from resolved address
        let new_ptr = ptr.create_pointer_from_address(Some(0x10));

        // New pointer should have base = resolved address + offset
        // Original: base=0x1000, offsets=[0x10, 0]
        // resolve: 0x1000 + 0x10 -> deref -> ?, then + 0 -> ?
        // Actually in create_pointer_from_address, we append offset then 0, then resolve
        // offsets = [0x10, 0], resolve:
        // ptr=0x1000, +0x10=0x1010, deref -> fail (no data there), returns 0
        // So new_ptr.base_address = 0

        // Let's test with proper chain
        let mut mock2 = MockMemoryReader::new();
        mock2.write_i64(0x1010, 0x3000);

        let reader2: Arc<dyn MemoryReader> = Arc::new(mock2);
        let ptr2 = AbstractPointer::new(reader2, true, 0x1000, vec![]);

        let new_ptr2 = ptr2.create_pointer_from_address(Some(0x10));

        // offsets = [0x10, 0]
        // ptr=0x1000, +0x10=0x1010, deref -> 0x3000, +0=0x3000
        assert_eq!(new_ptr2.base_address, 0x3000);
        assert!(new_ptr2.offsets.is_empty());
    }

    // =============================================================================
    // Event flag reading simulation tests
    // =============================================================================

    #[test]
    fn test_ds3_style_event_flag_reading() {
        let mut mock = MockMemoryReader::new();

        // Simulate DS3 event flag structure:
        // base -> categories array
        // Each category is at base + (category_index * 8)
        // Within category: flags as bitfield

        let base = 0x140000000usize;
        let category_base = 0x145000000usize;

        // Set up pointer chain: base -> category_base
        mock.write_i64(base as usize, category_base as i64);

        // Set up category 13000 at offset 13000 * 8 = 0x19640
        // Flag 13000050: category 13000, offset 50/8=6, bit 50%8=2
        let category_13000_addr = category_base + (13000 * 8);
        mock.write_i64(category_13000_addr, 0x146000000); // pointer to category data

        // At category data: byte 6 has bit 2 set (flag 13000050)
        let category_data_addr = 0x146000000usize;
        let mut category_data = vec![0u8; 128];
        category_data[6] = 0b00000100; // bit 2 set
        mock.write_memory_block(category_data_addr, &category_data);

        let reader: Arc<dyn MemoryReader> = Arc::new(mock);

        // Read the flag using pointer chain
        let base_ptr = AbstractPointer::new(reader.clone(), true, base as i64, vec![0]);

        // Get category 13000
        let category_ptr = base_ptr.append(&[(13000 * 8) as i64, 0]);
        let category_data_ptr = category_ptr.get_address();

        // Read the flag byte
        let flag_byte = reader.read_u8((category_data_ptr + 6) as usize).unwrap();
        let is_flag_set = (flag_byte >> 2) & 1 == 1;

        assert!(is_flag_set);
    }

    #[test]
    fn test_elden_ring_style_binary_tree_flag() {
        // Simplified binary tree event flag reading
        let mut mock = MockMemoryReader::new();

        // Tree structure:
        // Node: [left_child: i64, right_child: i64, key: u32, pad: u32, value: u8, ...]

        let root_addr = 0x140000000usize;

        // Root node with key 50000
        mock.write_i64(root_addr, 0x140001000); // left child
        mock.write_i64(root_addr + 8, 0x140002000); // right child
        mock.write_u32(root_addr + 16, 50000); // key
        mock.write_u8(root_addr + 24, 1); // value (flag set)

        // Left child with key 25000
        mock.write_i64(0x140001000, 0); // no left
        mock.write_i64(0x140001008, 0); // no right
        mock.write_u32(0x140001010, 25000); // key
        mock.write_u8(0x140001018, 0); // value

        // Right child with key 75000
        mock.write_i64(0x140002000, 0);
        mock.write_i64(0x140002008, 0);
        mock.write_u32(0x140002010, 75000);
        mock.write_u8(0x140002018, 1); // flag set

        let reader: Arc<dyn MemoryReader> = Arc::new(mock);

        // Search for flag 50000 (at root)
        let node_key = reader.read_u32(root_addr + 16).unwrap();
        let node_value = reader.read_u8(root_addr + 24).unwrap();

        assert_eq!(node_key, 50000);
        assert_eq!(node_value, 1);

        // Search for flag 75000 (right child)
        let right_child = reader.read_i64(root_addr + 8).unwrap() as usize;
        let right_key = reader.read_u32(right_child + 16).unwrap();
        let right_value = reader.read_u8(right_child + 24).unwrap();

        assert_eq!(right_key, 75000);
        assert_eq!(right_value, 1);
    }
}
