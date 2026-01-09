//! Game configuration types for loading games from TOML files
//!
//! This module defines the configuration structure that matches the
//! autosplitter.toml format in NYA-Core-Assets plugins.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

// =============================================================================
// MAIN CONFIGURATION STRUCTURES
// =============================================================================

/// Main plugin configuration from plugin.toml
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PluginConfig {
    pub plugin: PluginMetadata,
    pub process: ProcessConfig,
}

/// Plugin metadata
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PluginMetadata {
    pub id: String,
    pub name: String,
    pub short_name: Option<String>,
    pub version: String,
    pub author: Option<String>,
    pub description: Option<String>,
}

/// Process configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProcessConfig {
    pub names: Vec<String>,
}

/// Autosplitter configuration from autosplitter.toml
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AutosplitterConfig {
    pub autosplitter: AutosplitterSettings,
}

/// Main autosplitter settings
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AutosplitterSettings {
    pub enabled: bool,
    pub algorithm: FlagAlgorithm,

    /// Category decomposition config (DS3/Sekiro style)
    #[serde(default)]
    pub category_config: Option<CategoryConfig>,

    /// Binary tree config (Elden Ring/AC6 style)
    #[serde(default)]
    pub tree_config: Option<TreeConfig>,

    /// Offset table config (DS1 style)
    #[serde(default)]
    pub offset_config: Option<OffsetTableConfig>,

    /// Kill counter config (DS2 style)
    #[serde(default)]
    pub kill_counter_config: Option<KillCounterConfig>,

    /// Memory patterns to scan for
    #[serde(default)]
    pub patterns: Vec<PatternConfig>,

    /// Derived pointer chains
    #[serde(default)]
    pub pointers: HashMap<String, PointerConfig>,

    /// Memory layout offsets
    #[serde(default)]
    pub memory_layout: MemoryLayout,

    /// Custom triggers (like DS2's kill counter)
    #[serde(default)]
    pub custom_triggers: Vec<CustomTriggerConfig>,
}

// =============================================================================
// FLAG READING ALGORITHM TYPES
// =============================================================================

/// Flag reading algorithm type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FlagAlgorithm {
    /// DS3/Sekiro style - category decomposition
    CategoryDecomposition,
    /// Elden Ring/AC6 style - binary tree traversal
    BinaryTree,
    /// DS1 style - offset table lookup
    OffsetTable,
    /// DS2 style - kill counters
    KillCounter,
    /// No flag reading (position-only triggers, etc.)
    None,
}

impl Default for FlagAlgorithm {
    fn default() -> Self {
        Self::None
    }
}

/// Category decomposition configuration (DS3/Sekiro)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CategoryConfig {
    pub primary_pattern: String,
    pub secondary_pattern: String,
    #[serde(default = "default_base_offset")]
    pub base_offset: i64,          // 0x218 for DS3
    #[serde(default = "default_entry_size")]
    pub entry_size: i64,           // 0x18 for DS3
    #[serde(default = "default_category_multiplier")]
    pub category_multiplier: i64,  // 0xa8 for DS3

    // Field area offsets
    #[serde(default)]
    pub field_area_base_offset: i64,    // 0x0 for DS3, 0x18 for Sekiro
    #[serde(default = "default_world_info_offset")]
    pub world_info_offset: i64,         // 0x10 for DS3
    #[serde(default = "default_world_info_struct_size")]
    pub world_info_struct_size: i64,    // 0x38
    #[serde(default = "default_world_block_struct_size")]
    pub world_block_struct_size: i64,   // 0x70 for DS3, 0xb0 for Sekiro
}

fn default_base_offset() -> i64 { 0x218 }
fn default_entry_size() -> i64 { 0x18 }
fn default_category_multiplier() -> i64 { 0xa8 }
fn default_world_info_offset() -> i64 { 0x10 }
fn default_world_info_struct_size() -> i64 { 0x38 }
fn default_world_block_struct_size() -> i64 { 0x70 }

/// Binary tree configuration (Elden Ring/AC6)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TreeConfig {
    pub pattern: String,
    #[serde(default = "default_divisor_offset")]
    pub divisor_offset: i64,      // 0x1c
    #[serde(default = "default_tree_root_offset")]
    pub tree_root_offset: i64,    // 0x38
    #[serde(default = "default_mult_offset")]
    pub mult_offset: i64,         // 0x20
    #[serde(default = "default_base_addr_offset")]
    pub base_addr_offset: i64,    // 0x28
}

fn default_divisor_offset() -> i64 { 0x1c }
fn default_tree_root_offset() -> i64 { 0x38 }
fn default_mult_offset() -> i64 { 0x20 }
fn default_base_addr_offset() -> i64 { 0x28 }

/// Offset table configuration (DS1)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OffsetTableConfig {
    pub pattern: String,
    #[serde(default)]
    pub group_offsets: HashMap<String, i64>,
    #[serde(default)]
    pub area_indices: HashMap<String, i64>,
}

/// Kill counter configuration (DS2)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KillCounterConfig {
    pub pattern: String,
    #[serde(default)]
    pub pointer_chain: Vec<i64>,
    #[serde(default)]
    pub bosses: Vec<BossConfig>,
}

/// Boss configuration for kill counter
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BossConfig {
    pub id: String,
    pub name: String,
    pub offset: i64,
    #[serde(default)]
    pub group: Option<String>,
}

// =============================================================================
// MEMORY PATTERNS AND POINTERS
// =============================================================================

/// Memory pattern configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PatternConfig {
    pub name: String,
    pub pattern: String,
    #[serde(default = "default_rip_offset")]
    pub rip_offset: usize,
    #[serde(default = "default_instruction_len")]
    pub instruction_len: usize,
    #[serde(default)]
    pub pointer_offsets: Vec<i64>,
}

fn default_rip_offset() -> usize { 3 }
fn default_instruction_len() -> usize { 7 }

/// Pointer chain configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PointerConfig {
    pub base: String,
    pub offsets: Vec<i64>,
}

/// Memory layout offsets
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct MemoryLayout {
    #[serde(default)]
    pub igt_offset: Option<i64>,
    #[serde(default)]
    pub loading_offset: Option<i64>,
    #[serde(default)]
    pub blackscreen_offset: Option<i64>,
    #[serde(default)]
    pub player_health_offset: Option<i64>,
    #[serde(default)]
    pub player_max_health_offset: Option<i64>,
    #[serde(default)]
    pub ng_level_offset: Option<i64>,
    #[serde(default)]
    pub position_offsets: Option<PositionOffsets>,
    #[serde(default)]
    pub attributes: HashMap<String, i64>,
}

/// Position reading offsets
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PositionOffsets {
    pub x: i64,
    pub y: i64,
    pub z: i64,
}

// =============================================================================
// BOSSES AND PRESETS
// =============================================================================

/// Bosses configuration file (bosses.toml)
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct BossesConfig {
    #[serde(default)]
    pub bosses: Vec<BossDefinition>,
}

/// A boss definition
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BossDefinition {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub is_dlc: bool,
    #[serde(default)]
    pub dlc_name: Option<String>,
    /// Event flag ID for standard flag-based detection
    #[serde(default)]
    pub flag_id: Option<u32>,
    /// Kill counter offset for DS2-style detection
    #[serde(default)]
    pub kill_offset: Option<u32>,
    #[serde(default)]
    pub order: Option<i32>,
    #[serde(default)]
    pub aliases: Vec<String>,
}

/// Presets configuration file (presets.toml)
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PresetsConfig {
    #[serde(default)]
    pub presets: Vec<PresetDefinition>,
}

/// A preset definition
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PresetDefinition {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub boss_ids: Vec<String>,
}

impl BossesConfig {
    /// Load bosses configuration from a TOML file
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ConfigError::IoError(e.to_string()))?;
        toml::from_str(&content)
            .map_err(|e| ConfigError::ParseError(e.to_string()))
    }
}

impl PresetsConfig {
    /// Load presets configuration from a TOML file
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ConfigError::IoError(e.to_string()))?;
        toml::from_str(&content)
            .map_err(|e| ConfigError::ParseError(e.to_string()))
    }
}

// =============================================================================
// FULL GAME DATA
// =============================================================================

/// Complete game data loaded from a plugin directory
#[derive(Debug, Clone)]
pub struct GameData {
    pub plugin: PluginConfig,
    pub autosplitter: AutosplitterConfig,
    pub bosses: BossesConfig,
    pub presets: PresetsConfig,
}

impl GameData {
    /// Load all game data from a plugin directory
    pub fn load_from_dir(plugin_dir: &Path) -> Result<Self, ConfigError> {
        let plugin_path = plugin_dir.join("plugin.toml");
        let autosplitter_path = plugin_dir.join("autosplitter.toml");
        let bosses_path = plugin_dir.join("bosses.toml");
        let presets_path = plugin_dir.join("presets.toml");

        let plugin = PluginConfig::load(&plugin_path)?;
        let autosplitter = AutosplitterConfig::load(&autosplitter_path)?;

        // Bosses and presets are optional
        let bosses = if bosses_path.exists() {
            BossesConfig::load(&bosses_path)?
        } else {
            BossesConfig::default()
        };

        let presets = if presets_path.exists() {
            PresetsConfig::load(&presets_path)?
        } else {
            PresetsConfig::default()
        };

        Ok(Self {
            plugin,
            autosplitter,
            bosses,
            presets,
        })
    }

    /// Get boss by ID
    pub fn get_boss(&self, boss_id: &str) -> Option<&BossDefinition> {
        self.bosses.bosses.iter().find(|b| b.id == boss_id)
    }

    /// Get preset by ID
    pub fn get_preset(&self, preset_id: &str) -> Option<&PresetDefinition> {
        self.presets.presets.iter().find(|p| p.id == preset_id)
    }

    /// Get all bosses for a preset
    pub fn get_bosses_for_preset(&self, preset_id: &str) -> Vec<&BossDefinition> {
        if let Some(preset) = self.get_preset(preset_id) {
            preset.boss_ids.iter()
                .filter_map(|id| self.get_boss(id))
                .collect()
        } else {
            vec![]
        }
    }
}

// =============================================================================
// CUSTOM TRIGGERS
// =============================================================================

/// Custom trigger configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CustomTriggerConfig {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub parameters: Vec<CustomTriggerParamConfig>,
}

/// Custom trigger parameter configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CustomTriggerParamConfig {
    pub id: String,
    pub name: String,
    pub param_type: String,
    #[serde(default)]
    pub choices: Option<Vec<CustomTriggerChoiceConfig>>,
    #[serde(default)]
    pub default_value: Option<String>,
    #[serde(default = "default_true")]
    pub required: bool,
}

fn default_true() -> bool { true }

/// Custom trigger choice configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CustomTriggerChoiceConfig {
    pub value: String,
    pub label: String,
    #[serde(default)]
    pub group: Option<String>,
}

// =============================================================================
// LOADING FUNCTIONS
// =============================================================================

impl PluginConfig {
    /// Load plugin configuration from a TOML file
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ConfigError::IoError(e.to_string()))?;
        toml::from_str(&content)
            .map_err(|e| ConfigError::ParseError(e.to_string()))
    }
}

impl AutosplitterConfig {
    /// Load autosplitter configuration from a TOML file
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ConfigError::IoError(e.to_string()))?;
        toml::from_str(&content)
            .map_err(|e| ConfigError::ParseError(e.to_string()))
    }
}

/// Configuration loading errors
#[derive(Debug, Clone)]
pub enum ConfigError {
    IoError(String),
    ParseError(String),
    MissingField(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::IoError(e) => write!(f, "IO error: {}", e),
            ConfigError::ParseError(e) => write!(f, "Parse error: {}", e),
            ConfigError::MissingField(e) => write!(f, "Missing field: {}", e),
        }
    }
}

impl std::error::Error for ConfigError {}
