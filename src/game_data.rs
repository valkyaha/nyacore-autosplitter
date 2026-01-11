//! Game data structures for data-driven autosplitter
//!
//! This module defines the TOML schema for game definitions.
//! Games can be fully defined in external TOML files, allowing:
//! - Adding new games without recompiling
//! - Community-contributed game definitions
//! - Custom presets with special fields (like DS2 kill counts)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Root game data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameData {
    pub game: GameInfo,
    pub autosplitter: AutosplitterConfig,
    #[serde(default)]
    pub bosses: Vec<BossDefinition>,
    #[serde(default)]
    pub presets: Vec<PresetDefinition>,
    #[serde(default)]
    pub custom_fields: HashMap<String, CustomFieldDefinition>,
    #[serde(default)]
    pub attributes: Vec<AttributeDefinition>,
}

/// Basic game information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameInfo {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub short_name: Option<String>,
    pub process_names: Vec<String>,
}

/// Autosplitter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutosplitterConfig {
    /// Engine type determines the reading algorithm
    /// Supported: "ds1_ptde", "ds1_remaster", "ds2_sotfs", "ds3", "elden_ring", "sekiro", "ac6"
    pub engine: String,
    /// Memory patterns to scan for
    #[serde(default)]
    pub patterns: Vec<PatternDefinition>,
    /// Pointer chains for accessing game data
    #[serde(default)]
    pub pointers: HashMap<String, PointerDefinition>,
}

/// Memory pattern definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternDefinition {
    pub name: String,
    pub pattern: String,
    /// How to resolve the address: "rip_relative", "absolute", "none"
    #[serde(default = "default_resolve")]
    pub resolve: String,
    /// Offset to RIP-relative address in pattern (for rip_relative)
    #[serde(default)]
    pub rip_offset: i64,
    /// Additional offset after resolution
    #[serde(default)]
    pub extra_offset: i64,
}

fn default_resolve() -> String {
    "none".to_string()
}

/// Pointer chain definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointerDefinition {
    /// Pattern name to use as base
    pub pattern: String,
    /// Offset chain to follow
    #[serde(default)]
    pub offsets: Vec<i64>,
}

/// Boss definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BossDefinition {
    pub id: String,
    pub name: String,
    /// For event flag engines: actual flag ID (e.g., 13000050)
    /// For kill counter engines: offset from base (e.g., 0, 4, 8)
    pub flag_id: u32,
    #[serde(default)]
    pub is_dlc: bool,
    /// Custom field values for this boss
    #[serde(default)]
    pub custom: HashMap<String, serde_json::Value>,
}

/// Preset definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetDefinition {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    /// List of boss IDs in order
    pub bosses: Vec<String>,
    /// Custom field values for the entire preset
    #[serde(default)]
    pub custom: HashMap<String, serde_json::Value>,
    /// Per-boss custom values (boss_id -> field -> value)
    #[serde(default)]
    pub boss_overrides: HashMap<String, HashMap<String, serde_json::Value>>,
}

/// Custom field definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomFieldDefinition {
    /// Field type: "integer", "boolean", "string", "select"
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(default)]
    pub default: Option<serde_json::Value>,
    #[serde(default)]
    pub min: Option<i64>,
    #[serde(default)]
    pub max: Option<i64>,
    #[serde(default)]
    pub options: Vec<SelectOption>,
    #[serde(default)]
    pub description: Option<String>,
    /// Where this field applies: "boss", "split", "global"
    #[serde(default = "default_applies_to")]
    pub applies_to: String,
}

fn default_applies_to() -> String {
    "boss".to_string()
}

/// Option for select-type fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}

/// Character attribute definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeDefinition {
    pub id: String,
    pub name: String,
    /// Offset from attributes base pointer
    pub offset: i64,
}

impl GameData {
    /// Load game data from a TOML string
    pub fn from_toml(toml_str: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_str)
    }

    /// Load game data from a file
    pub fn from_file(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        Ok(Self::from_toml(&content)?)
    }

    /// Get a boss by ID
    pub fn get_boss(&self, id: &str) -> Option<&BossDefinition> {
        self.bosses.iter().find(|b| b.id == id)
    }

    /// Get a preset by ID
    pub fn get_preset(&self, id: &str) -> Option<&PresetDefinition> {
        self.presets.iter().find(|p| p.id == id)
    }

    /// Get a pattern by name
    pub fn get_pattern(&self, name: &str) -> Option<&PatternDefinition> {
        self.autosplitter.patterns.iter().find(|p| p.name == name)
    }

    /// Get a pointer definition by name
    pub fn get_pointer(&self, name: &str) -> Option<&PointerDefinition> {
        self.autosplitter.pointers.get(name)
    }

    /// Get bosses for a preset, with their full definitions
    pub fn get_preset_bosses(&self, preset_id: &str) -> Vec<&BossDefinition> {
        self.get_preset(preset_id)
            .map(|p| {
                p.bosses
                    .iter()
                    .filter_map(|boss_id| self.get_boss(boss_id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get custom field value for a boss in a preset
    pub fn get_boss_custom_value(
        &self,
        preset_id: &str,
        boss_id: &str,
        field_name: &str,
    ) -> Option<serde_json::Value> {
        // First check preset-level boss overrides
        if let Some(preset) = self.get_preset(preset_id) {
            if let Some(overrides) = preset.boss_overrides.get(boss_id) {
                if let Some(value) = overrides.get(field_name) {
                    return Some(value.clone());
                }
            }
        }

        // Then check boss-level custom values
        if let Some(boss) = self.get_boss(boss_id) {
            if let Some(value) = boss.custom.get(field_name) {
                return Some(value.clone());
            }
        }

        // Finally return the field default
        if let Some(field_def) = self.custom_fields.get(field_name) {
            return field_def.default.clone();
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_game_data() {
        let toml = r#"
[game]
id = "test"
name = "Test Game"
process_names = ["test.exe"]

[autosplitter]
engine = "ds3"

[[autosplitter.patterns]]
name = "event_flags"
pattern = "48 8b 35 ? ? ? ?"
resolve = "rip_relative"
rip_offset = 3

[[bosses]]
id = "boss1"
name = "First Boss"
flag_id = 1000

[[presets]]
id = "any-percent"
name = "Any%"
bosses = ["boss1"]
"#;

        let data = GameData::from_toml(toml).unwrap();
        assert_eq!(data.game.id, "test");
        assert_eq!(data.autosplitter.engine, "ds3");
        assert_eq!(data.bosses.len(), 1);
        assert_eq!(data.presets.len(), 1);
    }
}
