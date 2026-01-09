//! Autosplitter state types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::triggers::AutosplitConfig;

/// A boss flag configuration for the autosplitter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BossFlag {
    /// Unique identifier for this boss (e.g., "iudex-gundyr")
    pub boss_id: String,
    /// Human-readable boss name
    pub boss_name: String,
    /// Memory flag ID to check
    pub flag_id: u32,
    /// Whether this boss is from DLC content
    pub is_dlc: bool,
}

impl BossFlag {
    /// Create a new boss flag
    pub fn new(boss_id: impl Into<String>, boss_name: impl Into<String>, flag_id: u32) -> Self {
        Self {
            boss_id: boss_id.into(),
            boss_name: boss_name.into(),
            flag_id,
            is_dlc: false,
        }
    }

    /// Create a new DLC boss flag
    pub fn new_dlc(boss_id: impl Into<String>, boss_name: impl Into<String>, flag_id: u32) -> Self {
        Self {
            boss_id: boss_id.into(),
            boss_name: boss_name.into(),
            flag_id,
            is_dlc: true,
        }
    }
}

/// Split trigger configuration for custom autosplit triggers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplitTriggerConfig {
    /// Index of the split this trigger applies to
    pub split_index: usize,
    /// Name of the split for display
    pub split_name: String,
    /// The trigger configuration
    pub config: AutosplitConfig,
}

impl SplitTriggerConfig {
    /// Create a new split trigger configuration
    pub fn new(split_index: usize, split_name: impl Into<String>, config: AutosplitConfig) -> Self {
        Self {
            split_index,
            split_name: split_name.into(),
            config,
        }
    }
}

/// Current state of the autosplitter
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AutosplitterState {
    /// Whether the autosplitter is running
    pub running: bool,
    /// ID of the game being tracked (e.g., "dark-souls-3")
    pub game_id: String,
    /// Whether a game process is currently attached
    pub process_attached: bool,
    /// Process ID if attached
    pub process_id: Option<u32>,
    /// List of defeated boss IDs
    pub bosses_defeated: Vec<String>,
    /// Split indices where custom triggers have been matched
    pub triggers_matched: Vec<usize>,
    /// Boss kill counts for multi-kill tracking (DS2 ascetic runs)
    /// Maps boss_id -> current kill count
    #[serde(default)]
    pub boss_kill_counts: HashMap<String, u32>,
}

impl AutosplitterState {
    /// Create a new default state
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if a boss has been defeated
    pub fn is_boss_defeated(&self, boss_id: &str) -> bool {
        self.bosses_defeated.contains(&boss_id.to_string())
    }

    /// Get the kill count for a boss
    pub fn get_kill_count(&self, boss_id: &str) -> u32 {
        self.boss_kill_counts.get(boss_id).copied().unwrap_or(0)
    }

    /// Check if a trigger has been matched
    pub fn is_trigger_matched(&self, split_index: usize) -> bool {
        self.triggers_matched.contains(&split_index)
    }

    /// Reset the state (clear defeated bosses and triggers)
    pub fn reset(&mut self) {
        self.bosses_defeated.clear();
        self.triggers_matched.clear();
        self.boss_kill_counts.clear();
    }

    /// Mark a boss as defeated
    pub fn mark_boss_defeated(&mut self, boss_id: impl Into<String>) {
        let id = boss_id.into();
        if !self.bosses_defeated.contains(&id) {
            self.bosses_defeated.push(id);
        }
    }

    /// Update kill count for a boss
    pub fn set_kill_count(&mut self, boss_id: impl Into<String>, count: u32) {
        self.boss_kill_counts.insert(boss_id.into(), count);
    }

    /// Mark a trigger as matched
    pub fn mark_trigger_matched(&mut self, split_index: usize) {
        if !self.triggers_matched.contains(&split_index) {
            self.triggers_matched.push(split_index);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boss_flag_creation() {
        let flag = BossFlag::new("iudex", "Iudex Gundyr", 12345);
        assert_eq!(flag.boss_id, "iudex");
        assert_eq!(flag.boss_name, "Iudex Gundyr");
        assert_eq!(flag.flag_id, 12345);
        assert!(!flag.is_dlc);
    }

    #[test]
    fn test_boss_flag_dlc() {
        let flag = BossFlag::new_dlc("gael", "Slave Knight Gael", 54321);
        assert!(flag.is_dlc);
    }

    #[test]
    fn test_state_boss_tracking() {
        let mut state = AutosplitterState::new();
        assert!(!state.is_boss_defeated("iudex"));

        state.mark_boss_defeated("iudex");
        assert!(state.is_boss_defeated("iudex"));

        // Marking again should not duplicate
        state.mark_boss_defeated("iudex");
        assert_eq!(state.bosses_defeated.len(), 1);
    }

    #[test]
    fn test_state_kill_counts() {
        let mut state = AutosplitterState::new();
        assert_eq!(state.get_kill_count("boss"), 0);

        state.set_kill_count("boss", 3);
        assert_eq!(state.get_kill_count("boss"), 3);
    }

    #[test]
    fn test_state_reset() {
        let mut state = AutosplitterState::new();
        state.mark_boss_defeated("boss1");
        state.mark_trigger_matched(0);
        state.set_kill_count("boss1", 2);

        state.reset();
        assert!(state.bosses_defeated.is_empty());
        assert!(state.triggers_matched.is_empty());
        assert!(state.boss_kill_counts.is_empty());
    }
}
