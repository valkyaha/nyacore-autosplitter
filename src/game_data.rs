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

    fn create_test_game_data() -> GameData {
        let toml = r#"
[game]
id = "test"
name = "Test Game"
short_name = "TG"
process_names = ["test.exe", "test_debug.exe"]

[autosplitter]
engine = "ds3"

[[autosplitter.patterns]]
name = "event_flags"
pattern = "48 8b 35 ? ? ? ?"
resolve = "rip_relative"
rip_offset = 3

[[autosplitter.patterns]]
name = "world_chr_man"
pattern = "48 89 1d ? ? ? ?"
resolve = "none"

[autosplitter.pointers.player]
pattern = "world_chr_man"
offsets = [0, 0x68]

[[bosses]]
id = "boss1"
name = "First Boss"
flag_id = 1000

[[bosses]]
id = "boss2"
name = "Second Boss"
flag_id = 2000
is_dlc = true
[bosses.custom]
kill_count = 1

[[bosses]]
id = "boss3"
name = "Third Boss"
flag_id = 3000

[[presets]]
id = "any-percent"
name = "Any%"
description = "Any% speedrun category"
bosses = ["boss1", "boss2"]

[[presets]]
id = "all-bosses"
name = "All Bosses"
bosses = ["boss1", "boss2", "boss3"]
[presets.boss_overrides.boss2]
kill_count = 3

[custom_fields.kill_count]
type = "integer"
default = 1
min = 1
max = 10
description = "Number of kills required"
applies_to = "boss"

[custom_fields.difficulty]
type = "select"
default = "normal"
applies_to = "global"
[[custom_fields.difficulty.options]]
value = "easy"
label = "Easy"
[[custom_fields.difficulty.options]]
value = "normal"
label = "Normal"
[[custom_fields.difficulty.options]]
value = "hard"
label = "Hard"

[[attributes]]
id = "vigor"
name = "Vigor"
offset = 0x40

[[attributes]]
id = "endurance"
name = "Endurance"
offset = 0x44
"#;
        GameData::from_toml(toml).unwrap()
    }

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

    #[test]
    fn test_game_info() {
        let data = create_test_game_data();

        assert_eq!(data.game.id, "test");
        assert_eq!(data.game.name, "Test Game");
        assert_eq!(data.game.short_name, Some("TG".to_string()));
        assert_eq!(data.game.process_names, vec!["test.exe", "test_debug.exe"]);
    }

    #[test]
    fn test_autosplitter_config() {
        let data = create_test_game_data();

        assert_eq!(data.autosplitter.engine, "ds3");
        assert_eq!(data.autosplitter.patterns.len(), 2);
        assert_eq!(data.autosplitter.pointers.len(), 1);
    }

    #[test]
    fn test_pattern_definition() {
        let data = create_test_game_data();

        let pattern = &data.autosplitter.patterns[0];
        assert_eq!(pattern.name, "event_flags");
        assert_eq!(pattern.pattern, "48 8b 35 ? ? ? ?");
        assert_eq!(pattern.resolve, "rip_relative");
        assert_eq!(pattern.rip_offset, 3);
        assert_eq!(pattern.extra_offset, 0);

        let pattern2 = &data.autosplitter.patterns[1];
        assert_eq!(pattern2.resolve, "none");
    }

    #[test]
    fn test_pointer_definition() {
        let data = create_test_game_data();

        let pointer = data.autosplitter.pointers.get("player").unwrap();
        assert_eq!(pointer.pattern, "world_chr_man");
        assert_eq!(pointer.offsets, vec![0, 0x68]);
    }

    #[test]
    fn test_boss_definition() {
        let data = create_test_game_data();

        assert_eq!(data.bosses.len(), 3);

        let boss1 = &data.bosses[0];
        assert_eq!(boss1.id, "boss1");
        assert_eq!(boss1.name, "First Boss");
        assert_eq!(boss1.flag_id, 1000);
        assert!(!boss1.is_dlc);
        assert!(boss1.custom.is_empty());

        let boss2 = &data.bosses[1];
        assert_eq!(boss2.id, "boss2");
        assert!(boss2.is_dlc);
        assert_eq!(boss2.custom.get("kill_count").unwrap(), &serde_json::json!(1));
    }

    #[test]
    fn test_preset_definition() {
        let data = create_test_game_data();

        assert_eq!(data.presets.len(), 2);

        let preset = &data.presets[0];
        assert_eq!(preset.id, "any-percent");
        assert_eq!(preset.name, "Any%");
        assert_eq!(preset.description, Some("Any% speedrun category".to_string()));
        assert_eq!(preset.bosses, vec!["boss1", "boss2"]);
    }

    #[test]
    fn test_custom_field_integer() {
        let data = create_test_game_data();

        let field = data.custom_fields.get("kill_count").unwrap();
        assert_eq!(field.field_type, "integer");
        assert_eq!(field.default, Some(serde_json::json!(1)));
        assert_eq!(field.min, Some(1));
        assert_eq!(field.max, Some(10));
        assert_eq!(field.applies_to, "boss");
    }

    #[test]
    fn test_custom_field_select() {
        let data = create_test_game_data();

        let field = data.custom_fields.get("difficulty").unwrap();
        assert_eq!(field.field_type, "select");
        assert_eq!(field.options.len(), 3);
        assert_eq!(field.options[0].value, "easy");
        assert_eq!(field.options[0].label, "Easy");
        assert_eq!(field.applies_to, "global");
    }

    #[test]
    fn test_attribute_definition() {
        let data = create_test_game_data();

        assert_eq!(data.attributes.len(), 2);
        assert_eq!(data.attributes[0].id, "vigor");
        assert_eq!(data.attributes[0].name, "Vigor");
        assert_eq!(data.attributes[0].offset, 0x40);
    }

    #[test]
    fn test_get_boss() {
        let data = create_test_game_data();

        let boss = data.get_boss("boss1");
        assert!(boss.is_some());
        assert_eq!(boss.unwrap().name, "First Boss");

        let missing = data.get_boss("nonexistent");
        assert!(missing.is_none());
    }

    #[test]
    fn test_get_preset() {
        let data = create_test_game_data();

        let preset = data.get_preset("any-percent");
        assert!(preset.is_some());
        assert_eq!(preset.unwrap().name, "Any%");

        let missing = data.get_preset("nonexistent");
        assert!(missing.is_none());
    }

    #[test]
    fn test_get_pattern() {
        let data = create_test_game_data();

        let pattern = data.get_pattern("event_flags");
        assert!(pattern.is_some());
        assert_eq!(pattern.unwrap().pattern, "48 8b 35 ? ? ? ?");

        let missing = data.get_pattern("nonexistent");
        assert!(missing.is_none());
    }

    #[test]
    fn test_get_pointer() {
        let data = create_test_game_data();

        let pointer = data.get_pointer("player");
        assert!(pointer.is_some());
        assert_eq!(pointer.unwrap().pattern, "world_chr_man");

        let missing = data.get_pointer("nonexistent");
        assert!(missing.is_none());
    }

    #[test]
    fn test_get_preset_bosses() {
        let data = create_test_game_data();

        let bosses = data.get_preset_bosses("any-percent");
        assert_eq!(bosses.len(), 2);
        assert_eq!(bosses[0].id, "boss1");
        assert_eq!(bosses[1].id, "boss2");

        let all_bosses = data.get_preset_bosses("all-bosses");
        assert_eq!(all_bosses.len(), 3);

        let empty = data.get_preset_bosses("nonexistent");
        assert!(empty.is_empty());
    }

    #[test]
    fn test_get_boss_custom_value_from_override() {
        let data = create_test_game_data();

        // Test boss override in preset
        let value = data.get_boss_custom_value("all-bosses", "boss2", "kill_count");
        assert!(value.is_some());
        assert_eq!(value.unwrap(), serde_json::json!(3));
    }

    #[test]
    fn test_get_boss_custom_value_from_boss() {
        let data = create_test_game_data();

        // Test boss-level custom value (no override)
        let value = data.get_boss_custom_value("any-percent", "boss2", "kill_count");
        assert!(value.is_some());
        assert_eq!(value.unwrap(), serde_json::json!(1));
    }

    #[test]
    fn test_get_boss_custom_value_from_default() {
        let data = create_test_game_data();

        // Test field default value (boss has no custom value)
        let value = data.get_boss_custom_value("any-percent", "boss1", "kill_count");
        assert!(value.is_some());
        assert_eq!(value.unwrap(), serde_json::json!(1));
    }

    #[test]
    fn test_get_boss_custom_value_missing() {
        let data = create_test_game_data();

        // Test missing field
        let value = data.get_boss_custom_value("any-percent", "boss1", "nonexistent");
        assert!(value.is_none());
    }

    #[test]
    fn test_default_resolve() {
        assert_eq!(default_resolve(), "none");
    }

    #[test]
    fn test_default_applies_to() {
        assert_eq!(default_applies_to(), "boss");
    }

    #[test]
    fn test_minimal_game_data() {
        let toml = r#"
[game]
id = "minimal"
name = "Minimal Game"
process_names = ["game.exe"]

[autosplitter]
engine = "generic"
"#;

        let data = GameData::from_toml(toml).unwrap();
        assert_eq!(data.game.id, "minimal");
        assert!(data.bosses.is_empty());
        assert!(data.presets.is_empty());
        assert!(data.custom_fields.is_empty());
        assert!(data.attributes.is_empty());
    }

    #[test]
    fn test_pattern_default_values() {
        let toml = r#"
[game]
id = "test"
name = "Test"
process_names = ["test.exe"]

[autosplitter]
engine = "test"

[[autosplitter.patterns]]
name = "simple"
pattern = "48 89"
"#;

        let data = GameData::from_toml(toml).unwrap();
        let pattern = &data.autosplitter.patterns[0];

        assert_eq!(pattern.resolve, "none"); // default_resolve()
        assert_eq!(pattern.rip_offset, 0);
        assert_eq!(pattern.extra_offset, 0);
    }

    #[test]
    fn test_invalid_toml() {
        let toml = "invalid toml {{{";
        let result = GameData::from_toml(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_required_fields() {
        // Missing game.id
        let toml = r#"
[game]
name = "Test"
process_names = ["test.exe"]

[autosplitter]
engine = "test"
"#;
        let result = GameData::from_toml(toml);
        assert!(result.is_err());
    }
}
