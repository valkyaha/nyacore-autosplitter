//! Pointer chain resolution
//!
//! Rust port of SoulSplitter's Pointer class behavior.
//! The Pointer class manages a base address and a list of offsets.
//! When resolving, each offset EXCEPT the last is dereferenced.
//! The last offset is just added to get the final address.

use super::MemoryReader;

/// A pointer with offset chain for resolving nested memory addresses
///
/// This is a direct port of SoulSplitter's Pointer class behavior:
/// - All offsets EXCEPT the last are dereferenced (follow the pointer)
/// - The last offset is just added to the current address
#[derive(Debug, Clone)]
pub struct Pointer {
    /// Base address (absolute)
    pub base: i64,
    /// Chain of offsets to follow
    pub offsets: Vec<i64>,
    /// Whether this is a 64-bit process (affects pointer size when dereferencing)
    pub is_64_bit: bool,
}

impl Default for Pointer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pointer {
    /// Create a new uninitialized pointer
    pub fn new() -> Self {
        Self {
            base: 0,
            offsets: Vec::new(),
            is_64_bit: true,
        }
    }

    /// Create a pointer with specific values
    pub fn with_values(base: i64, offsets: Vec<i64>, is_64_bit: bool) -> Self {
        Self { base, offsets, is_64_bit }
    }

    /// Initialize the pointer with base address and offsets
    pub fn initialize(&mut self, is_64_bit: bool, base_address: i64, offsets: &[i64]) {
        self.is_64_bit = is_64_bit;
        self.base = base_address;
        self.offsets = offsets.to_vec();
    }

    /// Clear the pointer
    pub fn clear(&mut self) {
        self.base = 0;
        self.offsets.clear();
    }

    /// Create a copy of this pointer
    pub fn copy(&self) -> Self {
        Self {
            base: self.base,
            offsets: self.offsets.clone(),
            is_64_bit: self.is_64_bit,
        }
    }

    /// Append offsets to create a new pointer
    /// This is equivalent to SoulSplitter's Append
    pub fn append(&self, offsets: &[i64]) -> Self {
        let mut copy = self.copy();
        copy.offsets.extend_from_slice(offsets);
        copy
    }

    /// Creates a new pointer with the resolved address as base address
    /// This is equivalent to SoulSplitter's CreatePointerFromAddress
    ///
    /// The optional offset is added before resolving, and a trailing 0 is added
    /// to force dereferencing. The result has the resolved address as base
    /// and an empty offset list.
    pub fn create_pointer_from_address(&self, reader: &dyn MemoryReader, offset: Option<i64>) -> Self {
        let mut offsets = self.offsets.clone();

        if let Some(off) = offset {
            offsets.push(off);
        }

        // Add trailing 0 - this is what SoulSplitter does to force one more dereference
        offsets.push(0);

        let resolved = self.resolve_offsets(reader, &offsets);

        Self {
            base: resolved,
            offsets: Vec::new(),
            is_64_bit: self.is_64_bit,
        }
    }

    /// Resolve the pointer chain to get the final address
    ///
    /// SoulSplitter logic: all offsets EXCEPT the last are dereferenced.
    /// For each offset except the last:
    ///   1. Add offset to current pointer
    ///   2. Dereference (read pointer at that address)
    /// For the last offset: just add it
    pub fn resolve(&self, reader: &dyn MemoryReader) -> i64 {
        self.resolve_offsets(reader, &self.offsets)
    }

    /// Internal resolve with custom offset list
    fn resolve_offsets(&self, reader: &dyn MemoryReader, offsets: &[i64]) -> i64 {
        let mut ptr = self.base;

        for (i, &offset) in offsets.iter().enumerate() {
            let address = ptr + offset;

            // Not the last offset = resolve as pointer (dereference)
            if i + 1 < offsets.len() {
                if self.is_64_bit {
                    ptr = match reader.read_i64(address as usize) {
                        Some(v) => v,
                        None => return 0,
                    };
                } else {
                    ptr = match reader.read_i32(address as usize) {
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
    pub fn is_null_ptr(&self, reader: &dyn MemoryReader) -> bool {
        self.resolve(reader) == 0
    }

    /// Get the resolved address
    pub fn get_address(&self, reader: &dyn MemoryReader) -> i64 {
        self.resolve(reader)
    }

    /// Read i32 at optional offset
    pub fn read_i32(&self, reader: &dyn MemoryReader, offset: Option<i64>) -> i32 {
        let mut offsets_copy = self.offsets.clone();
        if let Some(off) = offset {
            offsets_copy.push(off);
        }
        let address = self.resolve_offsets(reader, &offsets_copy);
        reader.read_i32(address as usize).unwrap_or(0)
    }

    /// Read u32 at optional offset
    pub fn read_u32(&self, reader: &dyn MemoryReader, offset: Option<i64>) -> u32 {
        let mut offsets_copy = self.offsets.clone();
        if let Some(off) = offset {
            offsets_copy.push(off);
        }
        let address = self.resolve_offsets(reader, &offsets_copy);
        reader.read_u32(address as usize).unwrap_or(0)
    }

    /// Read i64 at optional offset
    pub fn read_i64(&self, reader: &dyn MemoryReader, offset: Option<i64>) -> i64 {
        let mut offsets_copy = self.offsets.clone();
        if let Some(off) = offset {
            offsets_copy.push(off);
        }
        let address = self.resolve_offsets(reader, &offsets_copy);
        reader.read_i64(address as usize).unwrap_or(0)
    }

    /// Read u64 at optional offset
    pub fn read_u64(&self, reader: &dyn MemoryReader, offset: Option<i64>) -> u64 {
        let mut offsets_copy = self.offsets.clone();
        if let Some(off) = offset {
            offsets_copy.push(off);
        }
        let address = self.resolve_offsets(reader, &offsets_copy);
        reader.read_u64(address as usize).unwrap_or(0)
    }

    /// Read byte at optional offset
    pub fn read_byte(&self, reader: &dyn MemoryReader, offset: Option<i64>) -> u8 {
        let mut offsets_copy = self.offsets.clone();
        if let Some(off) = offset {
            offsets_copy.push(off);
        }
        let address = self.resolve_offsets(reader, &offsets_copy);
        reader.read_u8(address as usize).unwrap_or(0)
    }

    /// Read f32 at optional offset
    pub fn read_f32(&self, reader: &dyn MemoryReader, offset: Option<i64>) -> f32 {
        let mut offsets_copy = self.offsets.clone();
        if let Some(off) = offset {
            offsets_copy.push(off);
        }
        let address = self.resolve_offsets(reader, &offsets_copy);
        reader.read_f32(address as usize).unwrap_or(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockReader {
        data: std::collections::HashMap<usize, Vec<u8>>,
    }

    impl MockReader {
        fn new() -> Self {
            Self {
                data: std::collections::HashMap::new(),
            }
        }

        fn set_u64(&mut self, address: usize, value: u64) {
            self.data.insert(address, value.to_le_bytes().to_vec());
        }

        fn set_i64(&mut self, address: usize, value: i64) {
            self.data.insert(address, value.to_le_bytes().to_vec());
        }

        fn set_u32(&mut self, address: usize, value: u32) {
            self.data.insert(address, value.to_le_bytes().to_vec());
        }

        fn set_i32(&mut self, address: usize, value: i32) {
            self.data.insert(address, value.to_le_bytes().to_vec());
        }
    }

    impl MemoryReader for MockReader {
        fn read_bytes(&self, address: usize, size: usize) -> Option<Vec<u8>> {
            self.data.get(&address).and_then(|data| {
                if data.len() >= size {
                    Some(data[..size].to_vec())
                } else {
                    None
                }
            })
        }
    }

    #[test]
    fn test_pointer_no_offsets() {
        let reader = MockReader::new();
        let mut ptr = Pointer::new();
        ptr.initialize(true, 0x1000, &[]);

        assert_eq!(ptr.resolve(&reader), 0x1000);
    }

    #[test]
    fn test_pointer_single_offset_no_dereference() {
        let reader = MockReader::new();
        let mut ptr = Pointer::new();
        // Single offset = just add, no dereference
        ptr.initialize(true, 0x1000, &[0x10]);

        assert_eq!(ptr.resolve(&reader), 0x1010);
    }

    #[test]
    fn test_pointer_chain_dereferencing() {
        let mut reader = MockReader::new();
        // At 0x1000, store pointer to 0x2000
        reader.set_i64(0x1000, 0x2000);
        // At 0x2000, store pointer to 0x3000
        reader.set_i64(0x2000, 0x3000);

        let mut ptr = Pointer::new();
        // offsets: [0x0, 0x0, 0x10]
        // Step 1: 0x1000 + 0x0 = 0x1000, deref -> 0x2000
        // Step 2: 0x2000 + 0x0 = 0x2000, deref -> 0x3000
        // Step 3: 0x3000 + 0x10 = 0x3010 (no deref, last offset)
        ptr.initialize(true, 0x1000, &[0x0, 0x0, 0x10]);

        assert_eq!(ptr.resolve(&reader), 0x3010);
    }

    #[test]
    fn test_pointer_append() {
        let mut reader = MockReader::new();
        reader.set_i64(0x1000, 0x2000);

        let mut ptr = Pointer::new();
        ptr.initialize(true, 0x1000, &[0x0]);

        // Append additional offsets
        let appended = ptr.append(&[0x0, 0x10]);

        // Original: [0x0] -> with append: [0x0, 0x0, 0x10]
        // 0x1000 + 0x0 = 0x1000, deref -> 0x2000
        // 0x2000 + 0x0 = 0x2000, deref -> need another value
        reader.set_i64(0x2000, 0x3000);
        // 0x3000 + 0x10 = 0x3010

        assert_eq!(appended.resolve(&reader), 0x3010);
    }

    #[test]
    fn test_pointer_create_from_address() {
        let mut reader = MockReader::new();
        // Set up: 0x1000 -> 0x2000 -> 0x3000
        reader.set_i64(0x1000, 0x2000);
        reader.set_i64(0x2000, 0x3000);

        let mut ptr = Pointer::new();
        ptr.initialize(true, 0x1000, &[0x0]); // resolves to 0x2000

        // create_pointer_from_address with offset 0x0 and trailing 0
        // offsets become [0x0, 0x0, 0] -> resolve:
        // 0x1000 + 0x0 = 0x1000, deref -> 0x2000
        // 0x2000 + 0x0 = 0x2000, deref -> 0x3000
        // 0x3000 + 0 = 0x3000 (last, no deref)
        let new_ptr = ptr.create_pointer_from_address(&reader, Some(0x0));

        assert_eq!(new_ptr.base, 0x3000);
        assert!(new_ptr.offsets.is_empty());
    }

    #[test]
    fn test_pointer_read_i32() {
        let mut reader = MockReader::new();
        reader.set_i64(0x1000, 0x2000);
        reader.set_i32(0x2010, 42);

        let mut ptr = Pointer::new();
        ptr.initialize(true, 0x1000, &[0x0]);

        // Read at offset 0x10 from resolved address
        // 0x1000 + 0x0 = 0x1000, deref -> 0x2000
        // 0x2000 + 0x10 = 0x2010 (last offset, no deref)
        assert_eq!(ptr.read_i32(&reader, Some(0x10)), 42);
    }

    #[test]
    fn test_null_pointer_detection() {
        let mut reader = MockReader::new();
        reader.set_i64(0x1000, 0); // Null pointer at first dereference

        let mut ptr = Pointer::new();
        ptr.initialize(true, 0x1000, &[0x0, 0x10]);

        assert!(ptr.is_null_ptr(&reader));
    }
}
