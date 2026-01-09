//! Trigger configuration types

use serde::{Deserialize, Serialize};
use super::AutosplitTrigger;

/// Logic for combining multiple triggers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TriggerLogic {
    /// Any trigger can fire independently
    #[default]
    Any,

    /// Triggers must fire in order
    Sequential,

    /// All triggers must be ready before any can fire
    AllRequired,
}

/// Configuration for autosplit triggers
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AutosplitConfig {
    /// Name of this configuration
    pub name: String,

    /// Game ID this config is for
    pub game_id: String,

    /// Logic for combining triggers
    #[serde(default)]
    pub logic: TriggerLogic,

    /// List of triggers
    pub triggers: Vec<AutosplitTrigger>,

    /// Whether to auto-start on game launch
    #[serde(default)]
    pub auto_start: bool,

    /// Whether to auto-reset when game resets
    #[serde(default)]
    pub auto_reset: bool,
}

impl AutosplitConfig {
    /// Create a new empty configuration
    pub fn new(name: impl Into<String>, game_id: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            game_id: game_id.into(),
            logic: TriggerLogic::default(),
            triggers: Vec::new(),
            auto_start: false,
            auto_reset: false,
        }
    }

    /// Add a trigger to this configuration
    pub fn with_trigger(mut self, trigger: AutosplitTrigger) -> Self {
        self.triggers.push(trigger);
        self
    }

    /// Set the trigger logic
    pub fn with_logic(mut self, logic: TriggerLogic) -> Self {
        self.logic = logic;
        self
    }

    /// Reset all triggers
    pub fn reset(&mut self) {
        for trigger in &mut self.triggers {
            trigger.reset();
        }
    }

    /// Get the next trigger in sequence (for Sequential logic)
    pub fn next_trigger(&self) -> Option<&AutosplitTrigger> {
        match self.logic {
            TriggerLogic::Sequential => {
                self.triggers.iter().find(|t| !t.activated)
            }
            _ => None,
        }
    }

    /// Get all active (non-triggered) triggers
    pub fn active_triggers(&self) -> impl Iterator<Item = &AutosplitTrigger> {
        self.triggers.iter().filter(|t| !t.activated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::triggers::TriggerCondition;

    #[test]
    fn test_config_builder() {
        let config = AutosplitConfig::new("Test Config", "dark-souls-3")
            .with_logic(TriggerLogic::Sequential)
            .with_trigger(
                AutosplitTrigger::new("boss1", "First Boss")
                    .with_condition(TriggerCondition::FlagSet { flag_id: 1000 })
            );

        assert_eq!(config.name, "Test Config");
        assert_eq!(config.game_id, "dark-souls-3");
        assert_eq!(config.logic, TriggerLogic::Sequential);
        assert_eq!(config.triggers.len(), 1);
    }

    #[test]
    fn test_reset() {
        let mut config = AutosplitConfig::new("Test", "test")
            .with_trigger(AutosplitTrigger::new("t1", "T1"))
            .with_trigger(AutosplitTrigger::new("t2", "T2"));

        config.triggers[0].activated = true;
        config.triggers[1].activated = true;

        config.reset();

        assert!(!config.triggers[0].activated);
        assert!(!config.triggers[1].activated);
    }

    #[test]
    fn test_next_trigger_sequential() {
        let mut config = AutosplitConfig::new("Test", "test")
            .with_logic(TriggerLogic::Sequential)
            .with_trigger(AutosplitTrigger::new("t1", "T1"))
            .with_trigger(AutosplitTrigger::new("t2", "T2"));

        assert_eq!(config.next_trigger().map(|t| &t.id), Some(&"t1".to_string()));

        config.triggers[0].activated = true;
        assert_eq!(config.next_trigger().map(|t| &t.id), Some(&"t2".to_string()));

        config.triggers[1].activated = true;
        assert!(config.next_trigger().is_none());
    }
}
