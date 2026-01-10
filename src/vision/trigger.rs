//! Trigger system for vision-based autosplitter
//!
//! Connects detections to actions (split, pause, reset)

use super::config::TriggerConfig;
use super::detector::{create_detector, Detector};
use anyhow::Result;
use std::path::Path;
use std::time::{Duration, Instant};

/// Actions that can be triggered
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerAction {
    /// Trigger a split
    Split,
    /// Pause the timer
    Pause,
    /// Resume the timer
    Resume,
    /// Reset the run
    Reset,
    /// Start the timer
    Start,
    /// Undo last split
    Undo,
    /// Skip current split
    Skip,
}

impl std::str::FromStr for TriggerAction {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "split" => Ok(TriggerAction::Split),
            "pause" => Ok(TriggerAction::Pause),
            "resume" => Ok(TriggerAction::Resume),
            "reset" => Ok(TriggerAction::Reset),
            "start" => Ok(TriggerAction::Start),
            "undo" => Ok(TriggerAction::Undo),
            "skip" => Ok(TriggerAction::Skip),
            _ => Err(anyhow::anyhow!("Unknown action: {}", s)),
        }
    }
}

impl std::fmt::Display for TriggerAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TriggerAction::Split => write!(f, "split"),
            TriggerAction::Pause => write!(f, "pause"),
            TriggerAction::Resume => write!(f, "resume"),
            TriggerAction::Reset => write!(f, "reset"),
            TriggerAction::Start => write!(f, "start"),
            TriggerAction::Undo => write!(f, "undo"),
            TriggerAction::Skip => write!(f, "skip"),
        }
    }
}

/// Event emitted when a trigger fires
#[derive(Debug, Clone)]
pub struct TriggerEvent {
    /// Trigger ID
    pub trigger_id: String,
    /// Trigger name
    pub trigger_name: String,
    /// Action to perform
    pub action: TriggerAction,
    /// Associated boss ID (for linking to splits)
    pub boss_id: Option<String>,
    /// Detection confidence
    pub confidence: f32,
    /// Timestamp
    pub timestamp: Instant,
}

/// A configured trigger with its detector and state
pub struct VisionTrigger {
    /// Unique ID
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// The detector to use
    detector: Box<dyn Detector>,
    /// Action to perform when triggered
    action: TriggerAction,
    /// Cooldown between triggers
    cooldown: Duration,
    /// Last time this trigger fired
    last_triggered: Option<Instant>,
    /// Whether this trigger is enabled
    enabled: bool,
    /// Associated boss ID
    boss_id: Option<String>,
    /// Whether this trigger has been consumed (one-shot triggers)
    consumed: bool,
}

impl VisionTrigger {
    /// Create from config
    pub fn from_config(config: &TriggerConfig, base_path: &Path) -> Result<Self> {
        let detector = create_detector(
            &config.detector_type,
            config.template.as_ref(),
            config.color.as_ref(),
            config.ocr.as_ref(),
            config.region.as_ref(),
            base_path,
        )?;

        let action: TriggerAction = config.action.parse()?;

        Ok(Self {
            id: config.id.clone(),
            name: config.name.clone(),
            detector,
            action,
            cooldown: Duration::from_millis(config.cooldown_ms),
            last_triggered: None,
            enabled: config.enabled,
            boss_id: config.boss_id.clone(),
            consumed: false,
        })
    }

    /// Check if trigger is ready to fire (not on cooldown)
    pub fn is_ready(&self) -> bool {
        if !self.enabled || self.consumed {
            return false;
        }

        match self.last_triggered {
            None => true,
            Some(last) => last.elapsed() >= self.cooldown,
        }
    }

    /// Process a frame and return event if triggered
    pub fn process(&mut self, frame: &super::capture::FrameData) -> Result<Option<TriggerEvent>> {
        if !self.is_ready() {
            return Ok(None);
        }

        let result = self.detector.detect(frame)?;

        if result.matched {
            self.last_triggered = Some(Instant::now());

            // For split actions linked to bosses, mark as consumed
            if self.action == TriggerAction::Split && self.boss_id.is_some() {
                self.consumed = true;
            }

            Ok(Some(TriggerEvent {
                trigger_id: self.id.clone(),
                trigger_name: self.name.clone(),
                action: self.action,
                boss_id: self.boss_id.clone(),
                confidence: result.confidence,
                timestamp: Instant::now(),
            }))
        } else {
            Ok(None)
        }
    }

    /// Reset trigger state (for new run)
    pub fn reset(&mut self) {
        self.last_triggered = None;
        self.consumed = false;
    }

    /// Enable/disable this trigger
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if trigger has been consumed
    pub fn is_consumed(&self) -> bool {
        self.consumed
    }

    /// Get the action type
    pub fn action(&self) -> TriggerAction {
        self.action
    }

    /// Get boss ID if any
    pub fn boss_id(&self) -> Option<&str> {
        self.boss_id.as_deref()
    }
}

/// Manages multiple triggers
pub struct TriggerManager {
    triggers: Vec<VisionTrigger>,
}

impl TriggerManager {
    pub fn new() -> Self {
        Self {
            triggers: Vec::new(),
        }
    }

    /// Load triggers from config
    pub fn load_from_configs(configs: &[TriggerConfig], base_path: &Path) -> Result<Self> {
        let mut triggers = Vec::with_capacity(configs.len());
        for config in configs {
            match VisionTrigger::from_config(config, base_path) {
                Ok(trigger) => triggers.push(trigger),
                Err(e) => {
                    log::warn!("Failed to load trigger '{}': {}", config.id, e);
                }
            }
        }
        Ok(Self { triggers })
    }

    /// Add a trigger
    pub fn add(&mut self, trigger: VisionTrigger) {
        self.triggers.push(trigger);
    }

    /// Process a frame through all triggers
    pub fn process(&mut self, frame: &super::capture::FrameData) -> Vec<TriggerEvent> {
        let mut events = Vec::new();

        for trigger in &mut self.triggers {
            match trigger.process(frame) {
                Ok(Some(event)) => {
                    log::info!(
                        "Trigger '{}' fired: action={}, confidence={:.2}",
                        event.trigger_name,
                        event.action,
                        event.confidence
                    );
                    events.push(event);
                }
                Ok(None) => {}
                Err(e) => {
                    log::warn!("Trigger '{}' error: {}", trigger.id, e);
                }
            }
        }

        events
    }

    /// Reset all triggers
    pub fn reset(&mut self) {
        for trigger in &mut self.triggers {
            trigger.reset();
        }
    }

    /// Get number of triggers
    pub fn len(&self) -> usize {
        self.triggers.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.triggers.is_empty()
    }

    /// Get trigger by ID
    pub fn get(&self, id: &str) -> Option<&VisionTrigger> {
        self.triggers.iter().find(|t| t.id == id)
    }

    /// Get mutable trigger by ID
    pub fn get_mut(&mut self, id: &str) -> Option<&mut VisionTrigger> {
        self.triggers.iter_mut().find(|t| t.id == id)
    }

    /// Enable/disable trigger by ID
    pub fn set_trigger_enabled(&mut self, id: &str, enabled: bool) -> bool {
        if let Some(trigger) = self.get_mut(id) {
            trigger.set_enabled(enabled);
            true
        } else {
            false
        }
    }

    /// Get all trigger IDs
    pub fn trigger_ids(&self) -> Vec<&str> {
        self.triggers.iter().map(|t| t.id.as_str()).collect()
    }

    /// Filter to only triggers for specific boss IDs
    pub fn filter_by_boss_ids(&mut self, boss_ids: &[String]) {
        for trigger in &mut self.triggers {
            if let Some(ref trigger_boss_id) = trigger.boss_id {
                trigger.enabled = boss_ids.contains(trigger_boss_id);
            }
        }
    }
}

impl Default for TriggerManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_parse() {
        assert_eq!("split".parse::<TriggerAction>().unwrap(), TriggerAction::Split);
        assert_eq!("pause".parse::<TriggerAction>().unwrap(), TriggerAction::Pause);
        assert_eq!("reset".parse::<TriggerAction>().unwrap(), TriggerAction::Reset);
    }

    #[test]
    fn test_trigger_cooldown() {
        use super::super::config::{ColorConfig, TriggerConfig};
        use std::path::PathBuf;

        let config = TriggerConfig {
            id: "test".to_string(),
            name: "Test Trigger".to_string(),
            detector_type: "color".to_string(),
            region: None,
            template: None,
            color: Some(ColorConfig {
                target: [255, 0, 0],
                tolerance: 10,
                min_match_percent: 50.0,
                detect_presence: true,
            }),
            ocr: None,
            action: "split".to_string(),
            cooldown_ms: 1000,
            enabled: true,
            boss_id: None,
        };

        let mut trigger = VisionTrigger::from_config(&config, &PathBuf::new()).unwrap();

        assert!(trigger.is_ready());

        // Simulate trigger fire
        trigger.last_triggered = Some(Instant::now());
        assert!(!trigger.is_ready());

        // After cooldown
        trigger.last_triggered = Some(Instant::now() - Duration::from_secs(2));
        assert!(trigger.is_ready());
    }
}
