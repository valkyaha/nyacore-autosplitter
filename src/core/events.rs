//! Events emitted by the autosplitter

use std::time::Instant;

/// Event emitted when a split condition is met
#[derive(Debug, Clone)]
pub struct SplitEvent {
    /// The boss ID that was defeated
    pub boss_id: String,
    /// Human-readable boss name
    pub boss_name: String,
    /// The flag ID that triggered this event
    pub flag_id: u32,
    /// When the event occurred
    pub timestamp: Instant,
    /// The split index if this was a custom trigger
    pub split_index: Option<usize>,
    /// Current kill count (for multi-kill bosses like DS2)
    pub kill_count: u32,
}

impl SplitEvent {
    /// Create a new split event for a boss defeat
    pub fn boss_defeated(
        boss_id: impl Into<String>,
        boss_name: impl Into<String>,
        flag_id: u32,
    ) -> Self {
        Self {
            boss_id: boss_id.into(),
            boss_name: boss_name.into(),
            flag_id,
            timestamp: Instant::now(),
            split_index: None,
            kill_count: 1,
        }
    }

    /// Create a new split event for a custom trigger
    pub fn trigger_matched(split_index: usize, split_name: impl Into<String>) -> Self {
        Self {
            boss_id: String::new(),
            boss_name: split_name.into(),
            flag_id: 0,
            timestamp: Instant::now(),
            split_index: Some(split_index),
            kill_count: 0,
        }
    }

    /// Set the kill count for multi-kill tracking
    pub fn with_kill_count(mut self, count: u32) -> Self {
        self.kill_count = count;
        self
    }
}

/// Callback type for split events
pub type SplitCallback = Box<dyn Fn(SplitEvent) + Send + Sync>;

/// Event handler that can have multiple listeners
pub struct EventHandler {
    callbacks: Vec<SplitCallback>,
}

impl EventHandler {
    /// Create a new event handler
    pub fn new() -> Self {
        Self {
            callbacks: Vec::new(),
        }
    }

    /// Add a callback for split events
    pub fn on_split(&mut self, callback: SplitCallback) {
        self.callbacks.push(callback);
    }

    /// Emit a split event to all listeners
    pub fn emit(&self, event: SplitEvent) {
        for callback in &self.callbacks {
            callback(event.clone());
        }
    }

    /// Check if there are any listeners
    pub fn has_listeners(&self) -> bool {
        !self.callbacks.is_empty()
    }
}

impl Default for EventHandler {
    fn default() -> Self {
        Self::new()
    }
}
