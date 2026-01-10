//! Configuration types for the autosplitter
//!
//! These types define the structure of autosplitter configurations loaded from TOML files.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Memory pattern configuration for scanning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternConfig {
    /// Name of this pattern (e.g., "sprj_event_flag_man", "field_area")
    pub name: String,
    /// Byte pattern with wildcards (e.g., "48 c7 05 ? ? ? ? 00 00 00 00")
    pub pattern: String,
    /// Position of RIP-relative offset in the pattern
    #[serde(default)]
    pub rip_offset: usize,
    /// Total instruction length for RIP resolution
    #[serde(default)]
    pub instruction_len: usize,
    /// Pointer offset chain to apply after pattern resolution
    #[serde(default)]
    pub pointer_offsets: Vec<i64>,
    /// Optional fallback patterns if primary doesn't match
    #[serde(default)]
    pub fallback_patterns: Vec<String>,
}

/// Named pointer chain configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointerChainConfig {
    /// Name of this pointer chain
    pub name: String,
    /// Offsets to follow from the base pointer
    pub offsets: Vec<i64>,
}

/// Derived pointer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DerivedPointerConfig {
    /// Base pattern name this pointer is derived from
    pub base: String,
    /// Offset chain to follow from the base pointer
    #[serde(default)]
    pub offsets: Vec<i64>,
}

/// Memory layout configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryLayoutConfig {
    #[serde(default)]
    pub igt_offset: Option<usize>,
    #[serde(default)]
    pub loading_offset: Option<usize>,
    #[serde(default)]
    pub position_offset: Option<usize>,
    #[serde(default)]
    pub category_base_offset: Option<usize>,
    #[serde(default)]
    pub category_entry_size: Option<usize>,
    #[serde(default)]
    pub category_count: Option<usize>,
    #[serde(default)]
    pub event_flag_tree: Option<EventFlagTreeConfig>,
    #[serde(default)]
    pub boss_offsets: Option<HashMap<String, usize>>,
}

/// Event flag tree configuration for binary tree algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventFlagTreeConfig {
    #[serde(default)]
    pub divisor_offset: Option<usize>,
    #[serde(default)]
    pub root_offset: Option<usize>,
    #[serde(default)]
    pub first_sub_element: Option<usize>,
    #[serde(default)]
    pub left_child: Option<usize>,
    #[serde(default)]
    pub right_child: Option<usize>,
    #[serde(default)]
    pub leaf_check_offset: Option<usize>,
    #[serde(default)]
    pub category_offset: Option<usize>,
    #[serde(default)]
    pub mystery_value_offset: Option<usize>,
    #[serde(default)]
    pub element_value_offset: Option<usize>,
    #[serde(default)]
    pub base_address_offset: Option<usize>,
}

/// Category decomposition algorithm config (DS3/Sekiro style)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryDecompositionConfig {
    /// Primary pattern name for this algorithm
    #[serde(default)]
    pub primary_pattern: String,
    pub divisor: u32,
    pub category_size: usize,
    #[serde(default)]
    pub flag_offset: usize,
}

/// Binary tree algorithm config (Elden Ring style)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryTreeConfig {
    /// Primary pattern name for this algorithm
    #[serde(default)]
    pub primary_pattern: String,
    #[serde(default)]
    pub root_offset: usize,
    #[serde(default)]
    pub divisor_offset: usize,
}

/// Offset table algorithm config (DS1 style)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OffsetTableConfig {
    /// Primary pattern name for this algorithm
    #[serde(default)]
    pub primary_pattern: String,
    #[serde(default)]
    pub base_offset: usize,
    #[serde(default)]
    pub entry_size: usize,
}

/// Kill counter algorithm config (DS2 style)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KillCounterConfig {
    /// Primary pattern name for this algorithm
    #[serde(default)]
    pub primary_pattern: String,
    #[serde(default)]
    pub counter_offset: usize,
    #[serde(default)]
    pub entry_size: usize,
    #[serde(default)]
    pub chain_offsets: Vec<usize>,
}

/// Version-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionConfig {
    pub name: String,
    #[serde(default)]
    pub patterns: Vec<PatternConfig>,
    #[serde(default)]
    pub memory_layout: Option<MemoryLayoutConfig>,
}

/// Full autosplitter memory configuration
#[derive(Debug, Clone, Default)]
pub struct AutosplitterMemoryConfig {
    /// Algorithm: "category_decomposition", "binary_tree", "offset_table", "kill_counter"
    pub algorithm: String,
    /// Memory scanning patterns
    pub patterns: Vec<PatternConfig>,
    /// Named pointer chains
    pub pointer_chains: Vec<PointerChainConfig>,
    /// Derived pointers
    pub pointers: HashMap<String, DerivedPointerConfig>,
    /// Memory layout configuration
    pub memory_layout: MemoryLayoutConfig,
    /// Version-specific configurations
    pub versions: Vec<VersionConfig>,
    /// Algorithm-specific configs
    pub category_config: Option<CategoryDecompositionConfig>,
    pub tree_config: Option<BinaryTreeConfig>,
    pub offset_table_config: Option<OffsetTableConfig>,
    pub kill_counter_config: Option<KillCounterConfig>,
    /// Legacy fields
    pub base_address: String,
    pub pointer_chain: Vec<i64>,
}

/// Boss flag information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BossFlag {
    pub boss_id: String,
    pub boss_name: String,
    pub flag_id: u32,
    #[serde(default)]
    pub is_dlc: bool,
}

/// Autosplitter state (serializable for FFI)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AutosplitterState {
    pub running: bool,
    pub game_id: String,
    pub process_attached: bool,
    pub process_id: Option<u32>,
    pub bosses_defeated: Vec<String>,
    pub triggers_matched: Vec<usize>,
    #[serde(default)]
    pub boss_kill_counts: HashMap<String, u32>,
}
