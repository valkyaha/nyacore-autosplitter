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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_config_default() {
        let config: PatternConfig = toml::from_str(r#"
            name = "test"
            pattern = "48 8b 35 ? ? ? ?"
        "#).unwrap();

        assert_eq!(config.name, "test");
        assert_eq!(config.pattern, "48 8b 35 ? ? ? ?");
        assert_eq!(config.rip_offset, 0);
        assert_eq!(config.instruction_len, 0);
        assert!(config.pointer_offsets.is_empty());
        assert!(config.fallback_patterns.is_empty());
    }

    #[test]
    fn test_pattern_config_full() {
        let config: PatternConfig = toml::from_str(r#"
            name = "event_flags"
            pattern = "48 8b 35 ? ? ? ?"
            rip_offset = 3
            instruction_len = 7
            pointer_offsets = [0, 8, 16]
            fallback_patterns = ["48 89 ? ? ? ? ?"]
        "#).unwrap();

        assert_eq!(config.name, "event_flags");
        assert_eq!(config.rip_offset, 3);
        assert_eq!(config.instruction_len, 7);
        assert_eq!(config.pointer_offsets, vec![0, 8, 16]);
        assert_eq!(config.fallback_patterns.len(), 1);
    }

    #[test]
    fn test_pointer_chain_config() {
        let config: PointerChainConfig = toml::from_str(r#"
            name = "player_pos"
            offsets = [0, 0x28, 0x80]
        "#).unwrap();

        assert_eq!(config.name, "player_pos");
        assert_eq!(config.offsets, vec![0, 0x28, 0x80]);
    }

    #[test]
    fn test_derived_pointer_config() {
        let config: DerivedPointerConfig = toml::from_str(r#"
            base = "event_flags"
            offsets = [0x10, 0x20]
        "#).unwrap();

        assert_eq!(config.base, "event_flags");
        assert_eq!(config.offsets, vec![0x10, 0x20]);
    }

    #[test]
    fn test_memory_layout_config_default() {
        let config = MemoryLayoutConfig::default();

        assert!(config.igt_offset.is_none());
        assert!(config.loading_offset.is_none());
        assert!(config.position_offset.is_none());
        assert!(config.category_base_offset.is_none());
        assert!(config.event_flag_tree.is_none());
        assert!(config.boss_offsets.is_none());
    }

    #[test]
    fn test_event_flag_tree_config() {
        let config: EventFlagTreeConfig = toml::from_str(r#"
            divisor_offset = 0x10
            root_offset = 0x20
            first_sub_element = 0x8
            left_child = 0x0
            right_child = 0x10
        "#).unwrap();

        assert_eq!(config.divisor_offset, Some(0x10));
        assert_eq!(config.root_offset, Some(0x20));
        assert_eq!(config.first_sub_element, Some(0x8));
    }

    #[test]
    fn test_category_decomposition_config() {
        let config: CategoryDecompositionConfig = toml::from_str(r#"
            primary_pattern = "event_flags"
            divisor = 1000
            category_size = 0x8
            flag_offset = 0x4
        "#).unwrap();

        assert_eq!(config.primary_pattern, "event_flags");
        assert_eq!(config.divisor, 1000);
        assert_eq!(config.category_size, 0x8);
        assert_eq!(config.flag_offset, 0x4);
    }

    #[test]
    fn test_binary_tree_config() {
        let config: BinaryTreeConfig = toml::from_str(r#"
            primary_pattern = "event_flags"
            root_offset = 0x28
            divisor_offset = 0x1c
        "#).unwrap();

        assert_eq!(config.primary_pattern, "event_flags");
        assert_eq!(config.root_offset, 0x28);
        assert_eq!(config.divisor_offset, 0x1c);
    }

    #[test]
    fn test_offset_table_config() {
        let config: OffsetTableConfig = toml::from_str(r#"
            primary_pattern = "event_flags"
            base_offset = 0x100
            entry_size = 0x10
        "#).unwrap();

        assert_eq!(config.primary_pattern, "event_flags");
        assert_eq!(config.base_offset, 0x100);
        assert_eq!(config.entry_size, 0x10);
    }

    #[test]
    fn test_kill_counter_config() {
        let config: KillCounterConfig = toml::from_str(r#"
            primary_pattern = "boss_counters"
            counter_offset = 0x4
            entry_size = 0x8
            chain_offsets = [0, 0x10, 0x20]
        "#).unwrap();

        assert_eq!(config.primary_pattern, "boss_counters");
        assert_eq!(config.counter_offset, 0x4);
        assert_eq!(config.entry_size, 0x8);
        assert_eq!(config.chain_offsets, vec![0, 0x10, 0x20]);
    }

    #[test]
    fn test_version_config() {
        let config: VersionConfig = toml::from_str(r#"
            name = "1.0.0"
            [[patterns]]
            name = "v1_pattern"
            pattern = "48 89 5c"
        "#).unwrap();

        assert_eq!(config.name, "1.0.0");
        assert_eq!(config.patterns.len(), 1);
        assert!(config.memory_layout.is_none());
    }

    #[test]
    fn test_boss_flag_serialization() {
        let flag = BossFlag {
            boss_id: "asylum_demon".to_string(),
            boss_name: "Asylum Demon".to_string(),
            flag_id: 13000050,
            is_dlc: false,
        };

        let json = serde_json::to_string(&flag).unwrap();
        let parsed: BossFlag = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.boss_id, "asylum_demon");
        assert_eq!(parsed.boss_name, "Asylum Demon");
        assert_eq!(parsed.flag_id, 13000050);
        assert!(!parsed.is_dlc);
    }

    #[test]
    fn test_boss_flag_toml() {
        let flag: BossFlag = toml::from_str(r#"
            boss_id = "sanctuary_guardian"
            boss_name = "Sanctuary Guardian"
            flag_id = 11210000
            is_dlc = true
        "#).unwrap();

        assert_eq!(flag.boss_id, "sanctuary_guardian");
        assert!(flag.is_dlc);
    }

    #[test]
    fn test_autosplitter_state_default() {
        let state = AutosplitterState::default();

        assert!(!state.running);
        assert!(state.game_id.is_empty());
        assert!(!state.process_attached);
        assert!(state.process_id.is_none());
        assert!(state.bosses_defeated.is_empty());
        assert!(state.triggers_matched.is_empty());
        assert!(state.boss_kill_counts.is_empty());
    }

    #[test]
    fn test_autosplitter_state_serialization() {
        let mut state = AutosplitterState {
            running: true,
            game_id: "ds3".to_string(),
            process_attached: true,
            process_id: Some(12345),
            bosses_defeated: vec!["iudex_gundyr".to_string()],
            triggers_matched: vec![0, 1],
            boss_kill_counts: HashMap::new(),
        };
        state.boss_kill_counts.insert("iudex_gundyr".to_string(), 1);

        let json = serde_json::to_string(&state).unwrap();
        let parsed: AutosplitterState = serde_json::from_str(&json).unwrap();

        assert!(parsed.running);
        assert_eq!(parsed.game_id, "ds3");
        assert!(parsed.process_attached);
        assert_eq!(parsed.process_id, Some(12345));
        assert_eq!(parsed.bosses_defeated, vec!["iudex_gundyr"]);
        assert_eq!(parsed.triggers_matched, vec![0, 1]);
        assert_eq!(parsed.boss_kill_counts.get("iudex_gundyr"), Some(&1));
    }

    #[test]
    fn test_autosplitter_memory_config_default() {
        let config = AutosplitterMemoryConfig::default();

        assert!(config.algorithm.is_empty());
        assert!(config.patterns.is_empty());
        assert!(config.pointer_chains.is_empty());
        assert!(config.pointers.is_empty());
        assert!(config.category_config.is_none());
        assert!(config.tree_config.is_none());
        assert!(config.offset_table_config.is_none());
        assert!(config.kill_counter_config.is_none());
    }
}
