//! Trigger system for custom autosplit conditions
//!
//! Provides a flexible trigger evaluation system beyond simple boss flags.

pub mod triggers;
pub mod trigger_evaluator;

pub use triggers::{AutosplitConfig, AutosplitTrigger, AttributeType, Comparison, TriggerLogic, ScreenStateType};
pub use trigger_evaluator::{TriggerEvaluator, TriggerState, TriggerResult};

#[cfg(target_os = "windows")]
pub use trigger_evaluator::GameStateRef;
