//! Custom trigger system for autosplitting
//!
//! This module provides a flexible trigger system that can evaluate
//! various game conditions to determine when to split.

mod types;
mod config;
mod evaluator;

pub use types::{AutosplitTrigger, TriggerCondition, ComparisonOp, Position3D};
pub use config::{AutosplitConfig, TriggerLogic};
pub use evaluator::TriggerEvaluator;
