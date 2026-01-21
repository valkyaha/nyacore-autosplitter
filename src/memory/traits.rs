//! Traits for memory reading and process management
//!
//! These traits allow for dependency injection, enabling mock implementations
//! for testing without requiring actual running processes.

use std::collections::HashMap;

/// Trait for reading memory from a process
pub trait MemoryReader: Send + Sync {
    /// Read raw bytes from memory
    fn read_bytes(&self, address: usize, size: usize) -> Option<Vec<u8>>;

    /// Read a u8 from memory
    fn read_u8(&self, address: usize) -> Option<u8> {
        let bytes = self.read_bytes(address, 1)?;
        Some(bytes[0])
    }

    /// Read a u16 from memory
    fn read_u16(&self, address: usize) -> Option<u16> {
        let bytes = self.read_bytes(address, 2)?;
        Some(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    /// Read an i16 from memory
    fn read_i16(&self, address: usize) -> Option<i16> {
        let bytes = self.read_bytes(address, 2)?;
        Some(i16::from_le_bytes([bytes[0], bytes[1]]))
    }

    /// Read a u32 from memory
    fn read_u32(&self, address: usize) -> Option<u32> {
        let bytes = self.read_bytes(address, 4)?;
        Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// Read an i32 from memory
    fn read_i32(&self, address: usize) -> Option<i32> {
        let bytes = self.read_bytes(address, 4)?;
        Some(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// Read a u64 from memory
    fn read_u64(&self, address: usize) -> Option<u64> {
        let bytes = self.read_bytes(address, 8)?;
        Some(u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3],
            bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    /// Read an i64 from memory
    fn read_i64(&self, address: usize) -> Option<i64> {
        let bytes = self.read_bytes(address, 8)?;
        Some(i64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3],
            bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    /// Read an f32 from memory
    fn read_f32(&self, address: usize) -> Option<f32> {
        let bytes = self.read_bytes(address, 4)?;
        Some(f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// Read an f64 from memory
    fn read_f64(&self, address: usize) -> Option<f64> {
        let bytes = self.read_bytes(address, 8)?;
        Some(f64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3],
            bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    /// Read a pointer (usize) from memory
    fn read_ptr(&self, address: usize) -> Option<usize> {
        self.read_u64(address).map(|v| v as usize)
    }

    /// Check if the reader is still valid (process still running)
    fn is_valid(&self) -> bool;

    /// Get the base address of the main module
    fn base_address(&self) -> usize;

    /// Get the size of the main module
    fn module_size(&self) -> usize;
}

/// Trait for finding and attaching to processes
pub trait ProcessFinder: Send + Sync {
    /// Find a process by name from a list of target names
    /// Returns (pid, process_name) if found
    fn find_process(&self, target_names: &[&str]) -> Option<(u32, String)>;

    /// Open a process and create a memory reader
    fn open_process(&self, pid: u32) -> Option<Box<dyn MemoryReader>>;
}

// =============================================================================
// Mock Implementations for Testing
// =============================================================================

/// Mock memory reader that returns data from a pre-configured memory map
#[derive(Default)]
pub struct MockMemoryReader {
    /// Memory contents: address -> bytes
    memory: HashMap<usize, Vec<u8>>,
    /// Base address of the module
    base: usize,
    /// Size of the module
    size: usize,
    /// Whether the process is "running"
    valid: bool,
}

impl MockMemoryReader {
    /// Create a new mock memory reader
    pub fn new() -> Self {
        Self {
            memory: HashMap::new(),
            base: 0x140000000,
            size: 0x4000000,
            valid: true,
        }
    }

    /// Set the base address
    pub fn with_base(mut self, base: usize) -> Self {
        self.base = base;
        self
    }

    /// Set the module size
    pub fn with_size(mut self, size: usize) -> Self {
        self.size = size;
        self
    }

    /// Set whether the process is valid
    pub fn with_valid(mut self, valid: bool) -> Self {
        self.valid = valid;
        self
    }

    /// Write bytes to mock memory
    pub fn write_bytes(&mut self, address: usize, data: &[u8]) {
        self.memory.insert(address, data.to_vec());
    }

    /// Write a u8 to mock memory
    pub fn write_u8(&mut self, address: usize, value: u8) {
        self.write_bytes(address, &[value]);
    }

    /// Write a u16 to mock memory
    pub fn write_u16(&mut self, address: usize, value: u16) {
        self.write_bytes(address, &value.to_le_bytes());
    }

    /// Write a u32 to mock memory
    pub fn write_u32(&mut self, address: usize, value: u32) {
        self.write_bytes(address, &value.to_le_bytes());
    }

    /// Write an i32 to mock memory
    pub fn write_i32(&mut self, address: usize, value: i32) {
        self.write_bytes(address, &value.to_le_bytes());
    }

    /// Write a u64 to mock memory
    pub fn write_u64(&mut self, address: usize, value: u64) {
        self.write_bytes(address, &value.to_le_bytes());
    }

    /// Write an i64 to mock memory
    pub fn write_i64(&mut self, address: usize, value: i64) {
        self.write_bytes(address, &value.to_le_bytes());
    }

    /// Write a pointer to mock memory
    pub fn write_ptr(&mut self, address: usize, value: usize) {
        self.write_u64(address, value as u64);
    }

    /// Write a contiguous block of memory
    pub fn write_memory_block(&mut self, start_address: usize, data: &[u8]) {
        self.memory.insert(start_address, data.to_vec());
    }

    /// Invalidate the process (simulate process exit)
    pub fn invalidate(&mut self) {
        self.valid = false;
    }
}

impl MemoryReader for MockMemoryReader {
    fn read_bytes(&self, address: usize, size: usize) -> Option<Vec<u8>> {
        if !self.valid {
            return None;
        }

        // Check for exact match first
        if let Some(data) = self.memory.get(&address) {
            if data.len() >= size {
                return Some(data[..size].to_vec());
            }
        }

        // Check if the address falls within any stored block
        for (&block_start, block_data) in &self.memory {
            if address >= block_start && address < block_start + block_data.len() {
                let offset = address - block_start;
                if offset + size <= block_data.len() {
                    return Some(block_data[offset..offset + size].to_vec());
                }
            }
        }

        None
    }

    fn is_valid(&self) -> bool {
        self.valid
    }

    fn base_address(&self) -> usize {
        self.base
    }

    fn module_size(&self) -> usize {
        self.size
    }
}

/// Mock process finder for testing
#[derive(Default)]
pub struct MockProcessFinder {
    /// List of mock processes: (pid, name)
    processes: Vec<(u32, String)>,
    /// Memory readers to return for each process
    readers: HashMap<u32, MockMemoryReader>,
}

impl MockProcessFinder {
    /// Create a new mock process finder
    pub fn new() -> Self {
        Self {
            processes: Vec::new(),
            readers: HashMap::new(),
        }
    }

    /// Add a mock process
    pub fn add_process(&mut self, pid: u32, name: &str) {
        self.processes.push((pid, name.to_string()));
    }

    /// Add a mock process with a memory reader
    pub fn add_process_with_reader(&mut self, pid: u32, name: &str, reader: MockMemoryReader) {
        self.processes.push((pid, name.to_string()));
        self.readers.insert(pid, reader);
    }
}

impl ProcessFinder for MockProcessFinder {
    fn find_process(&self, target_names: &[&str]) -> Option<(u32, String)> {
        for (pid, name) in &self.processes {
            let name_lower = name.to_lowercase();
            for target in target_names {
                let target_lower = target.to_lowercase();
                if name_lower == target_lower
                    || name_lower == format!("{}.exe", target_lower.trim_end_matches(".exe"))
                {
                    return Some((*pid, name.clone()));
                }
            }
        }
        None
    }

    fn open_process(&self, pid: u32) -> Option<Box<dyn MemoryReader>> {
        self.readers
            .get(&pid)
            .cloned()
            .map(|r| Box::new(r) as Box<dyn MemoryReader>)
    }
}

impl Clone for MockMemoryReader {
    fn clone(&self) -> Self {
        Self {
            memory: self.memory.clone(),
            base: self.base,
            size: self.size,
            valid: self.valid,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =============================================================================
    // MockMemoryReader tests
    // =============================================================================

    #[test]
    fn test_mock_memory_reader_new() {
        let reader = MockMemoryReader::new();
        assert!(reader.is_valid());
        assert_eq!(reader.base_address(), 0x140000000);
        assert_eq!(reader.module_size(), 0x4000000);
    }

    #[test]
    fn test_mock_memory_reader_with_base() {
        let reader = MockMemoryReader::new().with_base(0x7FFE0000);
        assert_eq!(reader.base_address(), 0x7FFE0000);
    }

    #[test]
    fn test_mock_memory_reader_with_size() {
        let reader = MockMemoryReader::new().with_size(0x1000000);
        assert_eq!(reader.module_size(), 0x1000000);
    }

    #[test]
    fn test_mock_memory_reader_with_valid() {
        let reader = MockMemoryReader::new().with_valid(false);
        assert!(!reader.is_valid());
    }

    #[test]
    fn test_mock_memory_reader_write_and_read_u8() {
        let mut reader = MockMemoryReader::new();
        reader.write_u8(0x1000, 0x42);

        assert_eq!(reader.read_u8(0x1000), Some(0x42));
    }

    #[test]
    fn test_mock_memory_reader_write_and_read_u16() {
        let mut reader = MockMemoryReader::new();
        reader.write_u16(0x1000, 0x1234);

        assert_eq!(reader.read_u16(0x1000), Some(0x1234));
    }

    #[test]
    fn test_mock_memory_reader_write_and_read_u32() {
        let mut reader = MockMemoryReader::new();
        reader.write_u32(0x1000, 0x12345678);

        assert_eq!(reader.read_u32(0x1000), Some(0x12345678));
    }

    #[test]
    fn test_mock_memory_reader_write_and_read_i32() {
        let mut reader = MockMemoryReader::new();
        reader.write_i32(0x1000, -12345);

        assert_eq!(reader.read_i32(0x1000), Some(-12345));
    }

    #[test]
    fn test_mock_memory_reader_write_and_read_u64() {
        let mut reader = MockMemoryReader::new();
        reader.write_u64(0x1000, 0x123456789ABCDEF0);

        assert_eq!(reader.read_u64(0x1000), Some(0x123456789ABCDEF0));
    }

    #[test]
    fn test_mock_memory_reader_write_and_read_i64() {
        let mut reader = MockMemoryReader::new();
        reader.write_i64(0x1000, -123456789);

        assert_eq!(reader.read_i64(0x1000), Some(-123456789));
    }

    #[test]
    fn test_mock_memory_reader_write_and_read_ptr() {
        let mut reader = MockMemoryReader::new();
        reader.write_ptr(0x1000, 0x7FFE00001234);

        assert_eq!(reader.read_ptr(0x1000), Some(0x7FFE00001234));
    }

    #[test]
    fn test_mock_memory_reader_write_and_read_bytes() {
        let mut reader = MockMemoryReader::new();
        let data = vec![0x01, 0x02, 0x03, 0x04, 0x05];
        reader.write_bytes(0x1000, &data);

        assert_eq!(reader.read_bytes(0x1000, 5), Some(data));
    }

    #[test]
    fn test_mock_memory_reader_read_partial_block() {
        let mut reader = MockMemoryReader::new();
        let data = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        reader.write_memory_block(0x1000, &data);

        // Read subset
        assert_eq!(reader.read_bytes(0x1000, 3), Some(vec![0x01, 0x02, 0x03]));
        // Read from offset
        assert_eq!(reader.read_bytes(0x1002, 3), Some(vec![0x03, 0x04, 0x05]));
        // Read u32 from offset
        assert_eq!(reader.read_u32(0x1004), Some(0x08070605));
    }

    #[test]
    fn test_mock_memory_reader_read_nonexistent() {
        let reader = MockMemoryReader::new();
        assert_eq!(reader.read_bytes(0x9999, 4), None);
        assert_eq!(reader.read_u32(0x9999), None);
    }

    #[test]
    fn test_mock_memory_reader_read_invalid() {
        let mut reader = MockMemoryReader::new();
        reader.write_u32(0x1000, 0x12345678);
        reader.invalidate();

        assert_eq!(reader.read_u32(0x1000), None);
        assert!(!reader.is_valid());
    }

    #[test]
    fn test_mock_memory_reader_f32() {
        let mut reader = MockMemoryReader::new();
        let value: f32 = 3.14159;
        reader.write_bytes(0x1000, &value.to_le_bytes());

        let read_value = reader.read_f32(0x1000).unwrap();
        assert!((read_value - value).abs() < 0.0001);
    }

    #[test]
    fn test_mock_memory_reader_f64() {
        let mut reader = MockMemoryReader::new();
        let value: f64 = 3.14159265358979;
        reader.write_bytes(0x1000, &value.to_le_bytes());

        let read_value = reader.read_f64(0x1000).unwrap();
        assert!((read_value - value).abs() < 0.0000001);
    }

    #[test]
    fn test_mock_memory_reader_i16() {
        let mut reader = MockMemoryReader::new();
        reader.write_bytes(0x1000, &(-1234i16).to_le_bytes());

        assert_eq!(reader.read_i16(0x1000), Some(-1234));
    }

    #[test]
    fn test_mock_memory_reader_clone() {
        let mut reader = MockMemoryReader::new();
        reader.write_u32(0x1000, 0x12345678);

        let cloned = reader.clone();
        assert_eq!(cloned.read_u32(0x1000), Some(0x12345678));
        assert_eq!(cloned.base_address(), reader.base_address());
        assert_eq!(cloned.module_size(), reader.module_size());
    }

    // =============================================================================
    // MockProcessFinder tests
    // =============================================================================

    #[test]
    fn test_mock_process_finder_new() {
        let finder = MockProcessFinder::new();
        assert_eq!(finder.find_process(&["notepad.exe"]), None);
    }

    #[test]
    fn test_mock_process_finder_add_process() {
        let mut finder = MockProcessFinder::new();
        finder.add_process(1234, "DarkSoulsIII.exe");

        let result = finder.find_process(&["DarkSoulsIII.exe"]);
        assert_eq!(result, Some((1234, "DarkSoulsIII.exe".to_string())));
    }

    #[test]
    fn test_mock_process_finder_case_insensitive() {
        let mut finder = MockProcessFinder::new();
        finder.add_process(1234, "DarkSoulsIII.exe");

        let result = finder.find_process(&["darksoulsiii.exe"]);
        assert_eq!(result, Some((1234, "DarkSoulsIII.exe".to_string())));
    }

    #[test]
    fn test_mock_process_finder_multiple_targets() {
        let mut finder = MockProcessFinder::new();
        finder.add_process(1234, "eldenring.exe");

        // Should match from multiple targets
        let result = finder.find_process(&["DarkSoulsIII.exe", "eldenring.exe"]);
        assert_eq!(result, Some((1234, "eldenring.exe".to_string())));
    }

    #[test]
    fn test_mock_process_finder_not_found() {
        let mut finder = MockProcessFinder::new();
        finder.add_process(1234, "DarkSoulsIII.exe");

        assert_eq!(finder.find_process(&["notepad.exe"]), None);
    }

    #[test]
    fn test_mock_process_finder_with_reader() {
        let mut finder = MockProcessFinder::new();

        let mut reader = MockMemoryReader::new();
        reader.write_u32(0x1000, 0xDEADBEEF);

        finder.add_process_with_reader(1234, "DarkSoulsIII.exe", reader);

        let opened = finder.open_process(1234);
        assert!(opened.is_some());

        let reader = opened.unwrap();
        assert_eq!(reader.read_u32(0x1000), Some(0xDEADBEEF));
    }

    #[test]
    fn test_mock_process_finder_open_unknown() {
        let finder = MockProcessFinder::new();
        assert!(finder.open_process(9999).is_none());
    }

    // =============================================================================
    // Pointer chain resolution tests
    // =============================================================================

    #[test]
    fn test_pointer_chain_resolution() {
        let mut reader = MockMemoryReader::new();

        // Set up a pointer chain:
        // 0x1000 -> 0x2000 -> 0x3000 -> value at 0x3010
        reader.write_ptr(0x1000, 0x2000);
        reader.write_ptr(0x2000, 0x3000);
        reader.write_u32(0x3010, 0xCAFEBABE);

        // Manually resolve the chain
        let ptr1 = reader.read_ptr(0x1000).unwrap();
        assert_eq!(ptr1, 0x2000);

        let ptr2 = reader.read_ptr(ptr1).unwrap();
        assert_eq!(ptr2, 0x3000);

        let value = reader.read_u32(ptr2 + 0x10).unwrap();
        assert_eq!(value, 0xCAFEBABE);
    }

    // =============================================================================
    // Event flag simulation tests (DS3-style)
    // =============================================================================

    #[test]
    fn test_event_flag_category_decomposition() {
        // DS3-style event flag reading:
        // flag_id / 1000 = category
        // (flag_id % 1000) / 8 = byte offset
        // (flag_id % 1000) % 8 = bit

        let mut reader = MockMemoryReader::new();

        // Simulate a category at 0x2000 with flags
        // Flag 13000050: category 13000, byte offset 6, bit 2
        // Set bit 2 of byte at offset 6
        let mut category_data = vec![0u8; 128];
        category_data[6] = 0b00000100; // bit 2 set

        reader.write_memory_block(0x2000, &category_data);

        // Read the flag
        let flag_id: u32 = 13000050;
        let category = flag_id / 1000;
        let remainder = flag_id % 1000;
        let byte_offset = (remainder / 8) as usize;
        let bit = remainder % 8;

        assert_eq!(category, 13000);
        assert_eq!(byte_offset, 6);
        assert_eq!(bit, 2);

        let byte_value = reader.read_u8(0x2000 + byte_offset).unwrap();
        let flag_set = (byte_value >> bit) & 1 == 1;

        assert!(flag_set);
    }

    #[test]
    fn test_event_flag_not_set() {
        let mut reader = MockMemoryReader::new();

        // Category at 0x2000 with no flags set
        let category_data = vec![0u8; 128];
        reader.write_memory_block(0x2000, &category_data);

        // Flag 13000050: byte offset 6, bit 2
        let byte_value = reader.read_u8(0x2000 + 6).unwrap();
        let flag_set = (byte_value >> 2) & 1 == 1;

        assert!(!flag_set);
    }
}
