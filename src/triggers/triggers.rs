//! Autosplit trigger types for custom split conditions
//!
//! Allows users to configure flexible autosplit triggers based on:
//! - Event flags (boss defeats, item pickups, etc.)
//! - In-game time thresholds
//! - Player position (area transitions, boss rooms)
//! - Loading/blackscreen states
//! - Character attributes
//! - NG+ level
//! - And combinations thereof

use serde::{Deserialize, Serialize};

/// Comparison operators for numeric triggers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Comparison {
    Equal,
    NotEqual,
    GreaterThan,
    GreaterOrEqual,
    LessThan,
    LessOrEqual,
}

impl Comparison {
    pub fn evaluate<T: PartialOrd + PartialEq>(&self, value: T, target: T) -> bool {
        match self {
            Comparison::Equal => value == target,
            Comparison::NotEqual => value != target,
            Comparison::GreaterThan => value > target,
            Comparison::GreaterOrEqual => value >= target,
            Comparison::LessThan => value < target,
            Comparison::LessOrEqual => value <= target,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Comparison::Equal => "==",
            Comparison::NotEqual => "!=",
            Comparison::GreaterThan => ">",
            Comparison::GreaterOrEqual => ">=",
            Comparison::LessThan => "<",
            Comparison::LessOrEqual => "<=",
        }
    }
}

impl Default for Comparison {
    fn default() -> Self {
        Self::GreaterOrEqual
    }
}

/// Character attributes that can be checked
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttributeType {
    // Common
    SoulLevel,
    Vigor,
    Endurance,
    Vitality,
    Strength,
    Dexterity,
    Intelligence,
    Faith,
    // DS1/DS3
    Attunement,
    Luck,
    // DS2
    Adaptability,
    // Sekiro
    AttackPower,
}

impl AttributeType {
    pub fn display_name(&self) -> &'static str {
        match self {
            AttributeType::SoulLevel => "Soul Level",
            AttributeType::Vigor => "Vigor",
            AttributeType::Endurance => "Endurance",
            AttributeType::Vitality => "Vitality",
            AttributeType::Strength => "Strength",
            AttributeType::Dexterity => "Dexterity",
            AttributeType::Intelligence => "Intelligence",
            AttributeType::Faith => "Faith",
            AttributeType::Attunement => "Attunement",
            AttributeType::Luck => "Luck",
            AttributeType::Adaptability => "Adaptability",
            AttributeType::AttackPower => "Attack Power",
        }
    }

    /// Get available attributes for a specific game
    pub fn for_game(game_id: &str) -> Vec<AttributeType> {
        match game_id {
            "dark-souls-remastered" | "dark-souls-1" => vec![
                AttributeType::SoulLevel,
                AttributeType::Vigor,
                AttributeType::Attunement,
                AttributeType::Endurance,
                AttributeType::Strength,
                AttributeType::Dexterity,
                AttributeType::Intelligence,
                AttributeType::Faith,
            ],
            "dark-souls-2" | "dark-souls-2-sotfs" => vec![
                AttributeType::SoulLevel,
                AttributeType::Vigor,
                AttributeType::Endurance,
                AttributeType::Vitality,
                AttributeType::Attunement,
                AttributeType::Strength,
                AttributeType::Dexterity,
                AttributeType::Adaptability,
                AttributeType::Intelligence,
                AttributeType::Faith,
            ],
            "dark-souls-3" => vec![
                AttributeType::SoulLevel,
                AttributeType::Vigor,
                AttributeType::Attunement,
                AttributeType::Endurance,
                AttributeType::Vitality,
                AttributeType::Strength,
                AttributeType::Dexterity,
                AttributeType::Intelligence,
                AttributeType::Faith,
                AttributeType::Luck,
            ],
            "sekiro" => vec![
                AttributeType::Vitality,
                AttributeType::AttackPower,
            ],
            "elden-ring" => vec![
                AttributeType::SoulLevel, // Called "Level" in ER
                AttributeType::Vigor,
                AttributeType::Endurance,
                AttributeType::Strength,
                AttributeType::Dexterity,
                AttributeType::Intelligence,
                AttributeType::Faith,
            ],
            _ => vec![],
        }
    }
}

/// Screen state types (primarily for Elden Ring)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScreenStateType {
    Loading,
    Logo,
    MainMenu,
    InGame,
}

/// A single autosplit trigger condition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AutosplitTrigger {
    /// Trigger on event flag (boss defeat, item pickup, etc.)
    EventFlag {
        flag_id: u32,
        /// If true, trigger when flag becomes set. If false, trigger when flag becomes unset.
        #[serde(default = "default_true")]
        on_set: bool,
    },

    /// Trigger based on in-game time
    InGameTime {
        comparison: Comparison,
        /// Target time in milliseconds
        target_ms: u64,
    },

    /// Trigger when player enters/exits a position radius
    Position {
        x: f32,
        y: f32,
        z: f32,
        /// Radius in game units
        radius: f32,
        /// If true, trigger when entering. If false, trigger when exiting.
        #[serde(default = "default_true")]
        on_enter: bool,
    },

    /// Trigger when player enters specific map area (Elden Ring)
    MapArea {
        area: u8,
        block: u8,
        #[serde(default)]
        region: Option<u8>,
    },

    /// Trigger on loading screen state
    Loading {
        /// If true, trigger when loading starts. If false, trigger when loading ends.
        on_start: bool,
    },

    /// Trigger on blackscreen/fade state
    Blackscreen {
        /// If true, trigger when blackscreen starts. If false, trigger when it ends.
        on_start: bool,
    },

    /// Trigger based on character attribute
    Attribute {
        attribute: AttributeType,
        comparison: Comparison,
        value: i32,
    },

    /// Trigger based on NG+ level
    NGLevel {
        comparison: Comparison,
        level: i32,
    },

    /// Trigger based on player health
    PlayerHealth {
        comparison: Comparison,
        /// Health value (can be percentage or absolute depending on game)
        value: i32,
    },

    /// Trigger when player is loaded/unloaded
    PlayerLoaded {
        /// If true, trigger when player loads. If false, trigger when player unloads.
        on_load: bool,
    },

    /// Trigger based on screen state (Elden Ring)
    ScreenState {
        state: ScreenStateType,
    },

    /// Trigger based on boss kill count (Dark Souls 2)
    BossKillCount {
        /// Boss offset in the kill counter array
        boss_offset: u32,
        comparison: Comparison,
        count: i32,
    },

    /// Trigger when warp is requested (Dark Souls 1)
    WarpRequested,

    /// Trigger when credits are rolling (Dark Souls 1)
    CreditsRolling,

    /// Custom event flag with manual ID input
    CustomFlag {
        flag_id: u32,
        #[serde(default = "default_true")]
        on_set: bool,
    },
}

fn default_true() -> bool {
    true
}

impl AutosplitTrigger {
    /// Get a human-readable description of this trigger
    pub fn description(&self) -> String {
        match self {
            AutosplitTrigger::EventFlag { flag_id, on_set } => {
                if *on_set {
                    format!("Event flag {} becomes set", flag_id)
                } else {
                    format!("Event flag {} becomes unset", flag_id)
                }
            }
            AutosplitTrigger::InGameTime { comparison, target_ms } => {
                let seconds = *target_ms as f64 / 1000.0;
                format!("IGT {} {:.1}s", comparison.as_str(), seconds)
            }
            AutosplitTrigger::Position { x, y, z, radius, on_enter } => {
                if *on_enter {
                    format!("Enter area ({:.1}, {:.1}, {:.1}) r={:.1}", x, y, z, radius)
                } else {
                    format!("Exit area ({:.1}, {:.1}, {:.1}) r={:.1}", x, y, z, radius)
                }
            }
            AutosplitTrigger::MapArea { area, block, region } => {
                if let Some(r) = region {
                    format!("Enter map area {}.{}.{}", area, block, r)
                } else {
                    format!("Enter map area {}.{}", area, block)
                }
            }
            AutosplitTrigger::Loading { on_start } => {
                if *on_start {
                    "Loading screen starts".to_string()
                } else {
                    "Loading screen ends".to_string()
                }
            }
            AutosplitTrigger::Blackscreen { on_start } => {
                if *on_start {
                    "Blackscreen/fade starts".to_string()
                } else {
                    "Blackscreen/fade ends".to_string()
                }
            }
            AutosplitTrigger::Attribute { attribute, comparison, value } => {
                format!("{} {} {}", attribute.display_name(), comparison.as_str(), value)
            }
            AutosplitTrigger::NGLevel { comparison, level } => {
                format!("NG+ level {} {}", comparison.as_str(), level)
            }
            AutosplitTrigger::PlayerHealth { comparison, value } => {
                format!("Player health {} {}", comparison.as_str(), value)
            }
            AutosplitTrigger::PlayerLoaded { on_load } => {
                if *on_load {
                    "Player loads into world".to_string()
                } else {
                    "Player unloads from world".to_string()
                }
            }
            AutosplitTrigger::ScreenState { state } => {
                format!("Screen state is {:?}", state)
            }
            AutosplitTrigger::BossKillCount { boss_offset, comparison, count } => {
                format!("Boss (offset 0x{:X}) kills {} {}", boss_offset, comparison.as_str(), count)
            }
            AutosplitTrigger::WarpRequested => "Warp/bonfire travel requested".to_string(),
            AutosplitTrigger::CreditsRolling => "End credits rolling".to_string(),
            AutosplitTrigger::CustomFlag { flag_id, on_set } => {
                if *on_set {
                    format!("Custom flag {} becomes set", flag_id)
                } else {
                    format!("Custom flag {} becomes unset", flag_id)
                }
            }
        }
    }

    /// Get the trigger type name for UI display
    pub fn type_name(&self) -> &'static str {
        match self {
            AutosplitTrigger::EventFlag { .. } => "Event Flag",
            AutosplitTrigger::InGameTime { .. } => "In-Game Time",
            AutosplitTrigger::Position { .. } => "Position",
            AutosplitTrigger::MapArea { .. } => "Map Area",
            AutosplitTrigger::Loading { .. } => "Loading",
            AutosplitTrigger::Blackscreen { .. } => "Blackscreen",
            AutosplitTrigger::Attribute { .. } => "Attribute",
            AutosplitTrigger::NGLevel { .. } => "NG+ Level",
            AutosplitTrigger::PlayerHealth { .. } => "Player Health",
            AutosplitTrigger::PlayerLoaded { .. } => "Player Loaded",
            AutosplitTrigger::ScreenState { .. } => "Screen State",
            AutosplitTrigger::BossKillCount { .. } => "Boss Kill Count",
            AutosplitTrigger::WarpRequested => "Warp Requested",
            AutosplitTrigger::CreditsRolling => "Credits Rolling",
            AutosplitTrigger::CustomFlag { .. } => "Custom Flag",
        }
    }

    /// Get available trigger types for a specific game
    pub fn available_for_game(game_id: &str) -> Vec<&'static str> {
        let mut types = vec!["Event Flag", "Custom Flag", "In-Game Time"];

        match game_id {
            "dark-souls-remastered" | "dark-souls-1" => {
                types.extend_from_slice(&[
                    "Position",
                    "Attribute",
                    "Player Health",
                    "Player Loaded",
                    "Warp Requested",
                    "Credits Rolling",
                ]);
            }
            "dark-souls-2" | "dark-souls-2-sotfs" => {
                types.extend_from_slice(&[
                    "Position",
                    "Loading",
                    "Attribute",
                    "Boss Kill Count",
                ]);
            }
            "dark-souls-3" => {
                types.extend_from_slice(&[
                    "Position",
                    "Loading",
                    "Blackscreen",
                    "Attribute",
                    "Player Loaded",
                ]);
            }
            "sekiro" => {
                types.extend_from_slice(&[
                    "Position",
                    "Blackscreen",
                    "Attribute",
                    "Player Loaded",
                ]);
            }
            "elden-ring" => {
                types.extend_from_slice(&[
                    "Position",
                    "Map Area",
                    "Blackscreen",
                    "NG+ Level",
                    "Player Loaded",
                    "Screen State",
                ]);
            }
            "armored-core-6" => {
                types.extend_from_slice(&[
                    "Loading",
                ]);
            }
            _ => {}
        }

        types
    }
}

/// Logic for combining multiple triggers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TriggerLogic {
    /// All triggers must be satisfied (AND)
    #[default]
    All,
    /// Any trigger being satisfied is enough (OR)
    Any,
}

/// Complete autosplit configuration for a split
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AutosplitConfig {
    /// Whether autosplit is enabled for this split
    #[serde(default)]
    pub enabled: bool,

    /// The triggers that control when this split completes
    #[serde(default)]
    pub triggers: Vec<AutosplitTrigger>,

    /// How to combine multiple triggers
    #[serde(default)]
    pub logic: TriggerLogic,

    /// Optional: Only trigger once per run (prevents re-triggering on reset to same area)
    #[serde(default = "default_true")]
    pub once_per_run: bool,
}

impl AutosplitConfig {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a simple event flag config (for backwards compatibility with boss_id)
    pub fn from_event_flag(flag_id: u32) -> Self {
        Self {
            enabled: true,
            triggers: vec![AutosplitTrigger::EventFlag {
                flag_id,
                on_set: true,
            }],
            logic: TriggerLogic::All,
            once_per_run: true,
        }
    }

    /// Check if this config has any triggers configured
    pub fn has_triggers(&self) -> bool {
        self.enabled && !self.triggers.is_empty()
    }

    /// Get a summary description of all triggers
    pub fn summary(&self) -> String {
        if self.triggers.is_empty() {
            return "No triggers".to_string();
        }

        if self.triggers.len() == 1 {
            return self.triggers[0].description();
        }

        let logic_str = match self.logic {
            TriggerLogic::All => "AND",
            TriggerLogic::Any => "OR",
        };

        let trigger_descs: Vec<String> = self.triggers.iter().map(|t| t.description()).collect();
        format!("{} ({})", trigger_descs.join(&format!(" {} ", logic_str)), logic_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comparison() {
        assert!(Comparison::Equal.evaluate(5, 5));
        assert!(!Comparison::Equal.evaluate(5, 6));

        assert!(Comparison::GreaterThan.evaluate(10, 5));
        assert!(!Comparison::GreaterThan.evaluate(5, 10));

        assert!(Comparison::LessOrEqual.evaluate(5, 5));
        assert!(Comparison::LessOrEqual.evaluate(4, 5));
    }

    #[test]
    fn test_trigger_serialization() {
        let trigger = AutosplitTrigger::EventFlag {
            flag_id: 12345,
            on_set: true,
        };

        let json = serde_json::to_string(&trigger).unwrap();
        assert!(json.contains("event_flag"));
        assert!(json.contains("12345"));

        let parsed: AutosplitTrigger = serde_json::from_str(&json).unwrap();
        if let AutosplitTrigger::EventFlag { flag_id, on_set } = parsed {
            assert_eq!(flag_id, 12345);
            assert!(on_set);
        } else {
            panic!("Wrong trigger type");
        }
    }

    #[test]
    fn test_config_from_event_flag() {
        let config = AutosplitConfig::from_event_flag(99999);
        assert!(config.enabled);
        assert_eq!(config.triggers.len(), 1);
        assert!(config.has_triggers());
    }
}
