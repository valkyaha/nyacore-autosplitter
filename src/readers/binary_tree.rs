//! Binary tree flag reader (Elden Ring)
//!
//! Elden Ring uses a binary search tree for event flags.

use super::FlagReader;
use crate::memory::MemoryReader;

/// Binary tree-based flag reader used by Elden Ring
pub struct BinaryTreeReader {
    /// Base address of the event flag manager
    base_address: usize,
    /// Offset from manager to tree root
    tree_root_offset: usize,
}

impl BinaryTreeReader {
    /// Create a new binary tree reader
    pub fn new(base_address: usize, tree_root_offset: usize) -> Self {
        Self {
            base_address,
            tree_root_offset,
        }
    }

    /// Search the tree for a flag
    fn search_tree(&self, reader: &dyn MemoryReader, root: usize, flag_id: u32) -> bool {
        if root == 0 {
            return false;
        }

        // Tree node structure (approximate):
        // +0x00: left child pointer
        // +0x08: right child pointer
        // +0x10: parent pointer
        // +0x18: flag ID
        // +0x1C: flag value or bit index

        let Some(node_flag_id) = reader.read_u32(root + 0x18) else {
            return false;
        };

        if node_flag_id == flag_id {
            // Found the node, check if flag is set
            let Some(flag_value) = reader.read_u32(root + 0x1C) else {
                return false;
            };
            return flag_value != 0;
        }

        // Binary search
        let child_offset = if flag_id < node_flag_id { 0x00 } else { 0x08 };
        let Some(child) = reader.read_ptr(root + child_offset) else {
            return false;
        };

        self.search_tree(reader, child, flag_id)
    }
}

impl FlagReader for BinaryTreeReader {
    fn is_flag_set(&self, reader: &dyn MemoryReader, flag_id: u32) -> bool {
        // Read the flag manager pointer
        let Some(manager_ptr) = reader.read_ptr(self.base_address) else {
            return false;
        };

        if manager_ptr == 0 {
            return false;
        }

        // Get tree root
        let Some(tree_root) = reader.read_ptr(manager_ptr + self.tree_root_offset) else {
            return false;
        };

        self.search_tree(reader, tree_root, flag_id)
    }
}
