//! Algorithm-based Engine
//!
//! This engine uses built-in algorithms for reading flags from memory.
//! Algorithms are based on LiveSplit's SoulMemory/SoulSplitter implementations.
//!
//! Supported algorithms:
//! - `category_decomposition`: DS3/Sekiro style flag reading
//! - `binary_tree`: Elden Ring/AC6 style flag reading
//! - `offset_table`: DS1 style flag reading
//! - `kill_counter`: DS2 style kill count tracking

use super::{Engine, EngineContext, EngineType};
use crate::games::config::{
    AutosplitterConfig, CategoryConfig, TreeConfig, OffsetTableConfig, KillCounterConfig,
    PatternConfig,
};
use crate::memory::{parse_pattern, scan_pattern, extract_relative_address};
use crate::AutosplitterError;
use std::collections::HashMap;

/// Algorithm type for flag reading
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlgorithmType {
    /// Category decomposition (DS3/Sekiro)
    CategoryDecomposition,
    /// Binary tree traversal (Elden Ring/AC6)
    BinaryTree,
    /// Offset table lookup (DS1)
    OffsetTable,
    /// Kill counter array (DS2)
    KillCounter,
}

impl std::str::FromStr for AlgorithmType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "category_decomposition" | "categorydecomposition" => Ok(AlgorithmType::CategoryDecomposition),
            "binary_tree" | "binarytree" => Ok(AlgorithmType::BinaryTree),
            "offset_table" | "offsettable" => Ok(AlgorithmType::OffsetTable),
            "kill_counter" | "killcounter" => Ok(AlgorithmType::KillCounter),
            _ => Err(format!("Unknown algorithm: {}", s)),
        }
    }
}

/// Algorithm-based autosplitter engine
pub struct AlgorithmEngine {
    /// The algorithm to use
    algorithm: AlgorithmType,
    /// Pattern configurations
    patterns: Vec<PatternConfig>,
    /// Category decomposition config (if applicable)
    category_config: Option<CategoryConfig>,
    /// Binary tree config (if applicable)
    tree_config: Option<TreeConfig>,
    /// Offset table config (if applicable)
    offset_table_config: Option<OffsetTableConfig>,
    /// Kill counter config (if applicable)
    kill_counter_config: Option<KillCounterConfig>,
    /// Memory layout offsets
    memory_layout: HashMap<String, i64>,
    /// Attribute offsets
    attribute_offsets: HashMap<String, i64>,
}

impl AlgorithmEngine {
    /// Create a new algorithm engine from config
    pub fn from_config(config: &AutosplitterConfig) -> Result<Self, AutosplitterError> {
        use crate::games::config::FlagAlgorithm;

        let algorithm = match config.autosplitter.algorithm {
            FlagAlgorithm::CategoryDecomposition => AlgorithmType::CategoryDecomposition,
            FlagAlgorithm::BinaryTree => AlgorithmType::BinaryTree,
            FlagAlgorithm::OffsetTable => AlgorithmType::OffsetTable,
            FlagAlgorithm::KillCounter => AlgorithmType::KillCounter,
            FlagAlgorithm::None => {
                return Err(AutosplitterError::ConfigError(
                    "No algorithm specified for algorithm engine".to_string()
                ));
            }
        };

        // Extract attribute offsets from memory layout
        let attribute_offsets = config.autosplitter.memory_layout.attributes.iter()
            .map(|(k, &v)| (k.clone(), v))
            .collect();

        Ok(Self {
            algorithm,
            patterns: config.autosplitter.patterns.clone(),
            category_config: config.autosplitter.category_config.clone(),
            tree_config: config.autosplitter.tree_config.clone(),
            offset_table_config: config.autosplitter.offset_config.clone(),
            kill_counter_config: config.autosplitter.kill_counter_config.clone(),
            memory_layout: HashMap::new(),
            attribute_offsets,
        })
    }

    /// Scan and resolve all patterns
    fn scan_patterns(&self, ctx: &mut EngineContext) -> Result<(), AutosplitterError> {
        let base = ctx.base_address();
        let size = ctx.module_size();

        for pattern_config in &self.patterns {
            let pattern = parse_pattern(&pattern_config.pattern);

            if let Some(match_addr) = scan_pattern(ctx.reader(), base, size, &pattern) {
                let resolved = if pattern_config.rip_offset > 0 {
                    extract_relative_address(
                        ctx.reader(),
                        match_addr,
                        pattern_config.rip_offset as usize,
                        pattern_config.instruction_len as usize,
                    ).unwrap_or(0)
                } else {
                    match_addr
                };

                // Follow pointer chain if specified
                let final_addr = if !pattern_config.pointer_offsets.is_empty() {
                    ctx.follow_pointer_chain(resolved, &pattern_config.pointer_offsets)
                        .unwrap_or(resolved)
                } else {
                    resolved
                };

                log::debug!(
                    "Pattern '{}': match=0x{:X}, resolved=0x{:X}, final=0x{:X}",
                    pattern_config.name, match_addr, resolved, final_addr
                );

                ctx.set_pointer(&pattern_config.name, final_addr);
            } else {
                log::warn!("Pattern '{}' not found", pattern_config.name);
            }
        }

        Ok(())
    }

    // =========================================================================
    // CATEGORY DECOMPOSITION (DS3/Sekiro)
    // =========================================================================

    fn read_flag_category_decomposition(&self, ctx: &EngineContext, flag_id: u32) -> bool {
        let config = match &self.category_config {
            Some(c) => c,
            None => return false,
        };

        let base_ptr = match ctx.get_pointer(&config.primary_pattern) {
            Some(p) => p,
            None => return false,
        };

        // Parse flag ID components
        let div_10m = (flag_id / 10_000_000) as i64;
        let area = ((flag_id % 10_000_000) / 10_000) as i32;
        let div_10k = ((flag_id % 10_000) / 1_000) as i32;
        let div_1k = (flag_id % 1_000) as i64;

        // Determine category
        let category = if area == 0 && div_10k == 0 {
            div_10m
        } else if area >= 10 && area <= 99 && div_10k == 0 {
            0
        } else {
            (area % 10) as i64
        };

        if category >= config.category_count as i64 {
            return false;
        }

        // Calculate entry address
        let entry_offset = config.base_offset + (category * config.category_multiplier);
        let entry_addr = match ctx.read_ptr(base_ptr + entry_offset as usize) {
            Some(a) if a != 0 => a,
            _ => return false,
        };

        // Read the flag
        let read_offset = div_1k / 32 * 4;
        let bit_mask = 0x8000_0000u32 >> (div_1k % 32);

        let value = ctx.read_u32(entry_addr + read_offset as usize).unwrap_or(0);
        (value & bit_mask) != 0
    }

    // =========================================================================
    // BINARY TREE (Elden Ring/AC6)
    // =========================================================================

    fn read_flag_binary_tree(&self, ctx: &EngineContext, flag_id: u32) -> bool {
        let config = match &self.tree_config {
            Some(c) => c,
            None => return false,
        };

        let base_ptr = match ctx.get_pointer(&config.primary_pattern) {
            Some(p) => p,
            None => return false,
        };

        // Get tree parameters
        let divisor = ctx.read_i32(base_ptr + config.divisor_offset as usize).unwrap_or(0);
        if divisor == 0 {
            return false;
        }

        let category = (flag_id / divisor as u32) as i32;
        let remainder = flag_id % divisor as u32;

        // Find the tree node
        let root = ctx.read_ptr(base_ptr + config.tree_root_offset as usize).unwrap_or(0);
        if root == 0 {
            return false;
        }

        // Traverse the binary tree
        let node = self.find_tree_node(ctx, root, category);
        if node == 0 {
            return false;
        }

        // Read the flag from the node
        let multiplier = ctx.read_i32(base_ptr + config.multiplier_offset as usize).unwrap_or(127);
        let base_addr = ctx.read_ptr(node + config.base_addr_offset as usize).unwrap_or(0);
        if base_addr == 0 {
            return false;
        }

        let offset = (remainder / 8) as usize;
        let bit = remainder % 8;
        let value = ctx.read_u8(base_addr + offset).unwrap_or(0);

        (value & (1 << bit)) != 0
    }

    fn find_tree_node(&self, ctx: &EngineContext, root: usize, target_category: i32) -> usize {
        let mut current = ctx.read_ptr(root + 0x8).unwrap_or(0); // First child

        while current != 0 {
            // Check if this is a leaf node
            let is_leaf = ctx.read_u8(current + 0x19).unwrap_or(0) != 0;

            let node_category = ctx.read_i32(current + 0x20).unwrap_or(0);

            if node_category == target_category {
                return current;
            }

            // Navigate left or right
            if target_category < node_category {
                current = ctx.read_ptr(current + 0x0).unwrap_or(0); // Left child
            } else {
                current = ctx.read_ptr(current + 0x10).unwrap_or(0); // Right child
            }
        }

        0
    }

    // =========================================================================
    // OFFSET TABLE (DS1)
    // =========================================================================

    fn read_flag_offset_table(&self, ctx: &EngineContext, flag_id: u32) -> bool {
        let config = match &self.offset_table_config {
            Some(c) => c,
            None => return false,
        };

        let base_ptr = match ctx.get_pointer(&config.primary_pattern) {
            Some(p) => p,
            None => return false,
        };

        // Parse flag ID
        let group = flag_id / 10_000_000;
        let area = (flag_id % 10_000_000) / 10_000;
        let section = (flag_id % 10_000) / 1_000;
        let number = flag_id % 1_000;

        // Look up group offset
        let group_offset = config.group_offsets.get(&group.to_string())
            .copied()
            .unwrap_or(0) as usize;

        // Look up area index
        let area_key = format!("{:03}", area);
        let area_index = config.area_indices.get(&area_key)
            .copied()
            .unwrap_or(0) as usize;

        // Calculate final offset
        let offset = group_offset
            + (area_index * 0x500)
            + (section as usize * 128)
            + ((number - (number % 32)) / 8) as usize;

        let mask = 0x8000_0000u32 >> (number % 32);
        let value = ctx.read_u32(base_ptr + offset).unwrap_or(0);

        (value & mask) != 0
    }

    // =========================================================================
    // KILL COUNTER (DS2)
    // =========================================================================

    fn read_flag_kill_counter(&self, ctx: &EngineContext, flag_id: u32) -> bool {
        self.get_kill_count_raw(ctx, flag_id) > 0
    }

    fn get_kill_count_raw(&self, ctx: &EngineContext, boss_offset: u32) -> i32 {
        let config = match &self.kill_counter_config {
            Some(c) => c,
            None => return 0,
        };

        let base_ptr = match ctx.get_pointer(&config.primary_pattern) {
            Some(p) => p,
            None => return 0,
        };

        // Follow the pointer chain
        let final_addr = ctx.follow_pointer_chain(base_ptr, &config.chain_offsets);
        match final_addr {
            Some(addr) => ctx.read_i32(addr + boss_offset as usize).unwrap_or(0),
            None => 0,
        }
    }
}

impl Engine for AlgorithmEngine {
    fn engine_type(&self) -> EngineType {
        EngineType::Algorithm
    }

    fn init(&mut self, ctx: &mut EngineContext) -> Result<(), AutosplitterError> {
        self.scan_patterns(ctx)?;
        log::info!("AlgorithmEngine initialized with {:?}", self.algorithm);
        Ok(())
    }

    fn read_flag(&self, ctx: &EngineContext, flag_id: u32) -> Result<bool, AutosplitterError> {
        let result = match self.algorithm {
            AlgorithmType::CategoryDecomposition => self.read_flag_category_decomposition(ctx, flag_id),
            AlgorithmType::BinaryTree => self.read_flag_binary_tree(ctx, flag_id),
            AlgorithmType::OffsetTable => self.read_flag_offset_table(ctx, flag_id),
            AlgorithmType::KillCounter => self.read_flag_kill_counter(ctx, flag_id),
        };
        Ok(result)
    }

    fn get_kill_count(&self, ctx: &EngineContext, flag_id: u32) -> Result<u32, AutosplitterError> {
        if self.algorithm == AlgorithmType::KillCounter {
            Ok(self.get_kill_count_raw(ctx, flag_id) as u32)
        } else {
            // Default: 1 if flag is set, 0 otherwise
            Ok(if self.read_flag(ctx, flag_id)? { 1 } else { 0 })
        }
    }

    fn get_attribute(&self, ctx: &EngineContext, attr: &str) -> Result<Option<i32>, AutosplitterError> {
        if let Some(&offset) = self.attribute_offsets.get(attr) {
            // Get player data pointer and read attribute
            // This is game-specific and may need customization
            if let Some(player_data) = ctx.get_pointer("player_game_data") {
                return Ok(ctx.read_i32(player_data + offset as usize));
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_algorithm_type_parsing() {
        assert_eq!("category_decomposition".parse::<AlgorithmType>().unwrap(), AlgorithmType::CategoryDecomposition);
        assert_eq!("binary_tree".parse::<AlgorithmType>().unwrap(), AlgorithmType::BinaryTree);
        assert_eq!("offset_table".parse::<AlgorithmType>().unwrap(), AlgorithmType::OffsetTable);
        assert_eq!("kill_counter".parse::<AlgorithmType>().unwrap(), AlgorithmType::KillCounter);
    }
}
