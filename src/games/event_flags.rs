//! Event flag reading algorithms
//!
//! This module contains the pure algorithms for reading event flags from
//! FromSoftware games. By separating the algorithms from the platform-specific
//! memory access, we can thoroughly unit test them.

use crate::memory::MemoryReader;
use std::sync::Arc;

/// Category decomposition algorithm (DS3/Sekiro/AC6 style)
///
/// Event flags are stored in categories. To read a flag:
/// 1. Divide flag_id by divisor to get category index
/// 2. Get category pointer from category array
/// 3. Use remainder to find byte and bit within category
pub struct CategoryDecomposition {
    reader: Arc<dyn MemoryReader>,
    categories_base: usize,
    divisor: u32,
}

impl CategoryDecomposition {
    /// Create a new category decomposition reader
    pub fn new(reader: Arc<dyn MemoryReader>, categories_base: usize, divisor: u32) -> Self {
        Self {
            reader,
            categories_base,
            divisor,
        }
    }

    /// Read an event flag using category decomposition
    pub fn read_flag(&self, flag_id: u32) -> bool {
        if self.categories_base == 0 {
            return false;
        }

        // Calculate category and position
        let category = (flag_id / self.divisor) as usize;
        let id_in_category = flag_id % self.divisor;
        let byte_offset = (id_in_category / 8) as usize;
        let bit = id_in_category % 8;

        // Get category pointer (categories are stored as 8-byte pointers)
        let category_ptr_addr = self.categories_base + (category * 8);
        let category_ptr = match self.reader.read_u64(category_ptr_addr) {
            Some(ptr) if ptr != 0 => ptr as usize,
            _ => return false,
        };

        // Read the flag byte
        let flag_byte = match self.reader.read_u8(category_ptr + byte_offset) {
            Some(b) => b,
            None => return false,
        };

        // Check the bit
        (flag_byte >> bit) & 1 == 1
    }
}

/// Binary tree algorithm (Elden Ring style)
///
/// Event flags are stored in a binary tree where each node contains:
/// - Left/right child pointers
/// - Flag group key
/// - Flags bitmap
pub struct BinaryTree {
    reader: Arc<dyn MemoryReader>,
    root: usize,
    divisor: u32,
}

/// Binary tree node offsets
#[derive(Clone, Copy)]
pub struct TreeNodeOffsets {
    pub left_child: usize,
    pub right_child: usize,
    pub key: usize,
    pub flags_base: usize,
}

impl Default for TreeNodeOffsets {
    fn default() -> Self {
        Self {
            left_child: 0x0,
            right_child: 0x8,
            key: 0x10,
            flags_base: 0x18,
        }
    }
}

impl BinaryTree {
    /// Create a new binary tree reader
    pub fn new(reader: Arc<dyn MemoryReader>, root: usize, divisor: u32) -> Self {
        Self {
            reader,
            root,
            divisor,
        }
    }

    /// Read an event flag using binary tree traversal
    pub fn read_flag(&self, flag_id: u32) -> bool {
        self.read_flag_with_offsets(flag_id, TreeNodeOffsets::default())
    }

    /// Read an event flag with custom node offsets
    pub fn read_flag_with_offsets(&self, flag_id: u32, offsets: TreeNodeOffsets) -> bool {
        if self.root == 0 {
            return false;
        }

        // Calculate which group and which bit within group
        let group_key = flag_id / self.divisor;
        let id_in_group = flag_id % self.divisor;
        let byte_offset = (id_in_group / 8) as usize;
        let bit = id_in_group % 8;

        // Find the node containing this flag group
        let node = match self.find_node(self.root, group_key, &offsets) {
            Some(n) => n,
            None => return false,
        };

        // Read the flag byte
        let flag_byte = match self.reader.read_u8(node + offsets.flags_base + byte_offset) {
            Some(b) => b,
            None => return false,
        };

        (flag_byte >> bit) & 1 == 1
    }

    /// Find a node in the tree by key
    fn find_node(&self, node_addr: usize, target_key: u32, offsets: &TreeNodeOffsets) -> Option<usize> {
        if node_addr == 0 {
            return None;
        }

        let node_key = self.reader.read_u32(node_addr + offsets.key)?;

        if node_key == target_key {
            return Some(node_addr);
        }

        // Traverse left or right based on key comparison
        let child_addr = if target_key < node_key {
            self.reader.read_u64(node_addr + offsets.left_child)? as usize
        } else {
            self.reader.read_u64(node_addr + offsets.right_child)? as usize
        };

        if child_addr == 0 {
            return None;
        }

        self.find_node(child_addr, target_key, offsets)
    }
}

/// Offset table algorithm (DS1 style)
///
/// Event flags are stored in a simple array indexed by flag_id / 8
pub struct OffsetTable {
    reader: Arc<dyn MemoryReader>,
    base: usize,
}

impl OffsetTable {
    /// Create a new offset table reader
    pub fn new(reader: Arc<dyn MemoryReader>, base: usize) -> Self {
        Self { reader, base }
    }

    /// Read an event flag using offset table
    pub fn read_flag(&self, flag_id: u32) -> bool {
        if self.base == 0 {
            return false;
        }

        let byte_offset = (flag_id / 8) as usize;
        let bit = flag_id % 8;

        let flag_byte = match self.reader.read_u8(self.base + byte_offset) {
            Some(b) => b,
            None => return false,
        };

        (flag_byte >> bit) & 1 == 1
    }
}

/// Kill counter algorithm (DS2 style)
///
/// Boss kills are tracked by counter values instead of flags
pub struct KillCounter {
    reader: Arc<dyn MemoryReader>,
    counters_base: usize,
    entry_size: usize,
}

impl KillCounter {
    /// Create a new kill counter reader
    pub fn new(reader: Arc<dyn MemoryReader>, counters_base: usize, entry_size: usize) -> Self {
        Self {
            reader,
            counters_base,
            entry_size,
        }
    }

    /// Read a kill count for a boss
    pub fn read_count(&self, boss_index: u32) -> u32 {
        if self.counters_base == 0 {
            return 0;
        }

        let offset = (boss_index as usize) * self.entry_size;
        self.reader.read_u32(self.counters_base + offset).unwrap_or(0)
    }

    /// Check if a boss has been killed at least once
    pub fn is_killed(&self, boss_index: u32) -> bool {
        self.read_count(boss_index) > 0
    }

    /// Check if a boss has been killed at least N times
    pub fn has_kills(&self, boss_index: u32, min_kills: u32) -> bool {
        self.read_count(boss_index) >= min_kills
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::MockMemoryReader;

    // =============================================================================
    // CategoryDecomposition tests
    // =============================================================================

    #[test]
    fn test_category_decomposition_flag_set() {
        let mut mock = MockMemoryReader::new();

        // Set up categories base at 0x1000
        let categories_base = 0x1000usize;

        // Category 13000 (for flag 13000050)
        // Category ptr at: 0x1000 + (13000 * 8) = 0x1000 + 0x19640 = 0x1A640
        let category_ptr_addr = categories_base + (13000 * 8);
        let category_data_addr = 0x50000usize;
        mock.write_u64(category_ptr_addr, category_data_addr as u64);

        // Flag 13000050: byte_offset = 50/8 = 6, bit = 50%8 = 2
        // Set bit 2 of byte at offset 6
        let mut category_data = vec![0u8; 16];
        category_data[6] = 0b00000100; // bit 2 set
        mock.write_memory_block(category_data_addr, &category_data);

        let reader: Arc<dyn MemoryReader> = Arc::new(mock);
        let algo = CategoryDecomposition::new(reader, categories_base, 1000);

        assert!(algo.read_flag(13000050));
    }

    #[test]
    fn test_category_decomposition_flag_not_set() {
        let mut mock = MockMemoryReader::new();

        let categories_base = 0x1000usize;
        let category_ptr_addr = categories_base + (13000 * 8);
        let category_data_addr = 0x50000usize;
        mock.write_u64(category_ptr_addr, category_data_addr as u64);

        // All zeros - no flags set
        let category_data = vec![0u8; 16];
        mock.write_memory_block(category_data_addr, &category_data);

        let reader: Arc<dyn MemoryReader> = Arc::new(mock);
        let algo = CategoryDecomposition::new(reader, categories_base, 1000);

        assert!(!algo.read_flag(13000050));
    }

    #[test]
    fn test_category_decomposition_null_base() {
        let mock = MockMemoryReader::new();
        let reader: Arc<dyn MemoryReader> = Arc::new(mock);
        let algo = CategoryDecomposition::new(reader, 0, 1000);

        assert!(!algo.read_flag(13000050));
    }

    #[test]
    fn test_category_decomposition_null_category_ptr() {
        let mock = MockMemoryReader::new();

        let categories_base = 0x1000usize;
        // Don't set up category pointer - it will be null

        let reader: Arc<dyn MemoryReader> = Arc::new(mock);
        let algo = CategoryDecomposition::new(reader, categories_base, 1000);

        assert!(!algo.read_flag(13000050));
    }

    #[test]
    fn test_category_decomposition_multiple_flags() {
        let mut mock = MockMemoryReader::new();

        let categories_base = 0x1000usize;
        let category_ptr_addr = categories_base + (13000 * 8);
        let category_data_addr = 0x50000usize;
        mock.write_u64(category_ptr_addr, category_data_addr as u64);

        // Set multiple flags:
        // 13000000: byte 0, bit 0
        // 13000007: byte 0, bit 7
        // 13000050: byte 6, bit 2
        // 13000100: byte 12, bit 4
        let mut category_data = vec![0u8; 16];
        category_data[0] = 0b10000001; // bits 0 and 7
        category_data[6] = 0b00000100; // bit 2
        category_data[12] = 0b00010000; // bit 4
        mock.write_memory_block(category_data_addr, &category_data);

        let reader: Arc<dyn MemoryReader> = Arc::new(mock);
        let algo = CategoryDecomposition::new(reader, categories_base, 1000);

        assert!(algo.read_flag(13000000));
        assert!(algo.read_flag(13000007));
        assert!(algo.read_flag(13000050));
        assert!(algo.read_flag(13000100));

        // These should not be set
        assert!(!algo.read_flag(13000001));
        assert!(!algo.read_flag(13000051));
    }

    #[test]
    fn test_category_decomposition_different_categories() {
        let mut mock = MockMemoryReader::new();

        let categories_base = 0x1000usize;

        // Set up two categories
        mock.write_u64(categories_base + (10000 * 8), 0x40000);
        mock.write_u64(categories_base + (20000 * 8), 0x50000);

        // Flag in category 10000
        let mut cat1_data = vec![0u8; 16];
        cat1_data[0] = 0b00000001; // flag 10000000
        mock.write_memory_block(0x40000, &cat1_data);

        // Flag in category 20000
        let mut cat2_data = vec![0u8; 16];
        cat2_data[0] = 0b00000010; // flag 20000001
        mock.write_memory_block(0x50000, &cat2_data);

        let reader: Arc<dyn MemoryReader> = Arc::new(mock);
        let algo = CategoryDecomposition::new(reader, categories_base, 1000);

        assert!(algo.read_flag(10000000));
        assert!(!algo.read_flag(10000001));
        assert!(!algo.read_flag(20000000));
        assert!(algo.read_flag(20000001));
    }

    // =============================================================================
    // BinaryTree tests
    // =============================================================================

    #[test]
    fn test_binary_tree_single_node() {
        let mut mock = MockMemoryReader::new();

        let root = 0x1000usize;

        // Single root node with key 5000, no children
        mock.write_u64(root + 0, 0); // left child = null
        mock.write_u64(root + 8, 0); // right child = null
        mock.write_u32(root + 16, 5000); // key
        // Flags at offset 24
        // Flag 5000050: byte_offset = 50/8 = 6, bit = 50%8 = 2
        let mut flags = vec![0u8; 16];
        flags[6] = 0b00000100;
        mock.write_memory_block(root + 24, &flags);

        let reader: Arc<dyn MemoryReader> = Arc::new(mock);
        let algo = BinaryTree::new(reader, root, 1000);

        assert!(algo.read_flag(5000050));
        assert!(!algo.read_flag(5000051));
    }

    #[test]
    fn test_binary_tree_traverse_left() {
        let mut mock = MockMemoryReader::new();

        let root = 0x1000usize;
        let left_child = 0x2000usize;

        // Root node with key 5000
        mock.write_u64(root + 0, left_child as u64); // left child
        mock.write_u64(root + 8, 0); // right child = null
        mock.write_u32(root + 16, 5000); // key

        // Left child with key 3000
        mock.write_u64(left_child + 0, 0);
        mock.write_u64(left_child + 8, 0);
        mock.write_u32(left_child + 16, 3000);
        let mut flags = vec![0u8; 16];
        flags[0] = 0b00000001; // flag 3000000
        mock.write_memory_block(left_child + 24, &flags);

        let reader: Arc<dyn MemoryReader> = Arc::new(mock);
        let algo = BinaryTree::new(reader, root, 1000);

        assert!(algo.read_flag(3000000));
    }

    #[test]
    fn test_binary_tree_traverse_right() {
        let mut mock = MockMemoryReader::new();

        let root = 0x1000usize;
        let right_child = 0x2000usize;

        // Root node with key 5000
        mock.write_u64(root + 0, 0); // left child = null
        mock.write_u64(root + 8, right_child as u64);
        mock.write_u32(root + 16, 5000);

        // Right child with key 7000
        mock.write_u64(right_child + 0, 0);
        mock.write_u64(right_child + 8, 0);
        mock.write_u32(right_child + 16, 7000);
        let mut flags = vec![0u8; 16];
        flags[1] = 0b00000010; // flag 7000009
        mock.write_memory_block(right_child + 24, &flags);

        let reader: Arc<dyn MemoryReader> = Arc::new(mock);
        let algo = BinaryTree::new(reader, root, 1000);

        assert!(algo.read_flag(7000009));
    }

    #[test]
    fn test_binary_tree_deep_traverse() {
        let mut mock = MockMemoryReader::new();

        // Build a tree:
        //        5000
        //       /    \
        //    3000    7000
        //    /  \
        // 1000  4000

        let nodes = [0x1000usize, 0x2000, 0x3000, 0x4000, 0x5000];

        // Root (5000)
        mock.write_u64(nodes[0] + 0, nodes[1] as u64); // left -> 3000
        mock.write_u64(nodes[0] + 8, nodes[2] as u64); // right -> 7000
        mock.write_u32(nodes[0] + 16, 5000);

        // Node 3000
        mock.write_u64(nodes[1] + 0, nodes[3] as u64); // left -> 1000
        mock.write_u64(nodes[1] + 8, nodes[4] as u64); // right -> 4000
        mock.write_u32(nodes[1] + 16, 3000);

        // Node 7000
        mock.write_u64(nodes[2] + 0, 0);
        mock.write_u64(nodes[2] + 8, 0);
        mock.write_u32(nodes[2] + 16, 7000);

        // Node 1000
        mock.write_u64(nodes[3] + 0, 0);
        mock.write_u64(nodes[3] + 8, 0);
        mock.write_u32(nodes[3] + 16, 1000);
        let mut flags1000 = vec![0u8; 16];
        flags1000[0] = 0b00000001; // flag 1000000
        mock.write_memory_block(nodes[3] + 24, &flags1000);

        // Node 4000
        mock.write_u64(nodes[4] + 0, 0);
        mock.write_u64(nodes[4] + 8, 0);
        mock.write_u32(nodes[4] + 16, 4000);
        let mut flags4000 = vec![0u8; 16];
        flags4000[5] = 0b00100000; // flag 4000045
        mock.write_memory_block(nodes[4] + 24, &flags4000);

        let reader: Arc<dyn MemoryReader> = Arc::new(mock);
        let algo = BinaryTree::new(reader, nodes[0], 1000);

        assert!(algo.read_flag(1000000));
        assert!(algo.read_flag(4000045));
    }

    #[test]
    fn test_binary_tree_null_root() {
        let mock = MockMemoryReader::new();
        let reader: Arc<dyn MemoryReader> = Arc::new(mock);
        let algo = BinaryTree::new(reader, 0, 1000);

        assert!(!algo.read_flag(5000050));
    }

    #[test]
    fn test_binary_tree_key_not_found() {
        let mut mock = MockMemoryReader::new();

        let root = 0x1000usize;
        mock.write_u64(root + 0, 0);
        mock.write_u64(root + 8, 0);
        mock.write_u32(root + 16, 5000);

        let reader: Arc<dyn MemoryReader> = Arc::new(mock);
        let algo = BinaryTree::new(reader, root, 1000);

        // Key 3000 doesn't exist
        assert!(!algo.read_flag(3000000));
    }

    // =============================================================================
    // OffsetTable tests
    // =============================================================================

    #[test]
    fn test_offset_table_flag_set() {
        let mut mock = MockMemoryReader::new();

        // Simple offset table
        // Flag 50: byte 6 (50/8), bit 2 (50%8)
        let base = 0x1000usize;
        let mut data = vec![0u8; 16];
        data[6] = 0b00000100;
        mock.write_memory_block(base, &data);

        let reader: Arc<dyn MemoryReader> = Arc::new(mock);
        let algo = OffsetTable::new(reader, base);

        assert!(algo.read_flag(50));
    }

    #[test]
    fn test_offset_table_flag_not_set() {
        let mut mock = MockMemoryReader::new();

        let base = 0x1000usize;
        let data = vec![0u8; 16];
        mock.write_memory_block(base, &data);

        let reader: Arc<dyn MemoryReader> = Arc::new(mock);
        let algo = OffsetTable::new(reader, base);

        assert!(!algo.read_flag(50));
    }

    #[test]
    fn test_offset_table_null_base() {
        let mock = MockMemoryReader::new();
        let reader: Arc<dyn MemoryReader> = Arc::new(mock);
        let algo = OffsetTable::new(reader, 0);

        assert!(!algo.read_flag(50));
    }

    #[test]
    fn test_offset_table_multiple_flags() {
        let mut mock = MockMemoryReader::new();

        let base = 0x1000usize;
        let mut data = vec![0u8; 128];
        data[0] = 0b10000001; // flags 0, 7
        data[1] = 0b00001000; // flag 11
        data[100] = 0b11111111; // flags 800-807
        mock.write_memory_block(base, &data);

        let reader: Arc<dyn MemoryReader> = Arc::new(mock);
        let algo = OffsetTable::new(reader, base);

        assert!(algo.read_flag(0));
        assert!(algo.read_flag(7));
        assert!(algo.read_flag(11));
        assert!(algo.read_flag(800));
        assert!(algo.read_flag(807));

        assert!(!algo.read_flag(1));
        assert!(!algo.read_flag(8));
    }

    // =============================================================================
    // KillCounter tests
    // =============================================================================

    #[test]
    fn test_kill_counter_read_count() {
        let mut mock = MockMemoryReader::new();

        let base = 0x1000usize;
        let entry_size = 4;

        // Boss 0: 5 kills
        mock.write_u32(base + 0, 5);
        // Boss 1: 0 kills
        mock.write_u32(base + 4, 0);
        // Boss 2: 1 kill
        mock.write_u32(base + 8, 1);

        let reader: Arc<dyn MemoryReader> = Arc::new(mock);
        let algo = KillCounter::new(reader, base, entry_size);

        assert_eq!(algo.read_count(0), 5);
        assert_eq!(algo.read_count(1), 0);
        assert_eq!(algo.read_count(2), 1);
    }

    #[test]
    fn test_kill_counter_is_killed() {
        let mut mock = MockMemoryReader::new();

        let base = 0x1000usize;
        mock.write_u32(base + 0, 1);
        mock.write_u32(base + 4, 0);

        let reader: Arc<dyn MemoryReader> = Arc::new(mock);
        let algo = KillCounter::new(reader, base, 4);

        assert!(algo.is_killed(0));
        assert!(!algo.is_killed(1));
    }

    #[test]
    fn test_kill_counter_has_kills() {
        let mut mock = MockMemoryReader::new();

        let base = 0x1000usize;
        mock.write_u32(base + 0, 5);
        mock.write_u32(base + 4, 2);

        let reader: Arc<dyn MemoryReader> = Arc::new(mock);
        let algo = KillCounter::new(reader, base, 4);

        assert!(algo.has_kills(0, 1));
        assert!(algo.has_kills(0, 5));
        assert!(!algo.has_kills(0, 6));

        assert!(algo.has_kills(1, 2));
        assert!(!algo.has_kills(1, 3));
    }

    #[test]
    fn test_kill_counter_null_base() {
        let mock = MockMemoryReader::new();
        let reader: Arc<dyn MemoryReader> = Arc::new(mock);
        let algo = KillCounter::new(reader, 0, 4);

        assert_eq!(algo.read_count(0), 0);
        assert!(!algo.is_killed(0));
    }

    #[test]
    fn test_kill_counter_different_entry_sizes() {
        let mut mock = MockMemoryReader::new();

        // Entry size 8 (with padding)
        let base = 0x1000usize;
        mock.write_u32(base + 0, 3);
        mock.write_u32(base + 8, 7);
        mock.write_u32(base + 16, 1);

        let reader: Arc<dyn MemoryReader> = Arc::new(mock);
        let algo = KillCounter::new(reader, base, 8);

        assert_eq!(algo.read_count(0), 3);
        assert_eq!(algo.read_count(1), 7);
        assert_eq!(algo.read_count(2), 1);
    }
}
