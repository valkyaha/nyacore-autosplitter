//! Vision autosplitter configuration

use serde::{Deserialize, Serialize};
use super::VisionTrigger;

/// Configuration for vision-based autosplitting
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VisionConfig {
    /// Name of this configuration
    pub name: String,
    /// Target window name (optional)
    pub window_name: Option<String>,
    /// Capture interval in milliseconds
    #[serde(default = "default_interval")]
    pub capture_interval_ms: u32,
    /// List of triggers
    pub triggers: Vec<VisionTrigger>,
}

fn default_interval() -> u32 {
    100 // 10 FPS by default
}

impl VisionConfig {
    /// Create a new configuration
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            window_name: None,
            capture_interval_ms: default_interval(),
            triggers: Vec::new(),
        }
    }

    /// Set the target window
    pub fn with_window(mut self, window: impl Into<String>) -> Self {
        self.window_name = Some(window.into());
        self
    }

    /// Set the capture interval
    pub fn with_interval(mut self, ms: u32) -> Self {
        self.capture_interval_ms = ms;
        self
    }

    /// Add a trigger
    pub fn with_trigger(mut self, trigger: VisionTrigger) -> Self {
        self.triggers.push(trigger);
        self
    }

    /// Reset all triggers
    pub fn reset(&mut self) {
        for trigger in &mut self.triggers {
            trigger.reset();
        }
    }
}
