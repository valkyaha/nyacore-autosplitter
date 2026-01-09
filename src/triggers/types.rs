//! Trigger type definitions

use serde::{Deserialize, Serialize};

/// A 3D position in game space
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Position3D {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Position3D {
    /// Create a new position
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// Calculate distance to another position
    pub fn distance_to(&self, other: &Position3D) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Calculate 2D distance (ignoring Y)
    pub fn distance_2d(&self, other: &Position3D) -> f32 {
        let dx = self.x - other.x;
        let dz = self.z - other.z;
        (dx * dx + dz * dz).sqrt()
    }
}

/// Comparison operators for trigger conditions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComparisonOp {
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
}

impl ComparisonOp {
    /// Compare two values using this operator
    pub fn compare<T: PartialOrd>(&self, a: &T, b: &T) -> bool {
        match self {
            ComparisonOp::Equal => a == b,
            ComparisonOp::NotEqual => a != b,
            ComparisonOp::LessThan => a < b,
            ComparisonOp::LessThanOrEqual => a <= b,
            ComparisonOp::GreaterThan => a > b,
            ComparisonOp::GreaterThanOrEqual => a >= b,
        }
    }
}

/// A condition that can be evaluated against game state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TriggerCondition {
    /// Event flag is set
    FlagSet { flag_id: u32 },

    /// Event flag is not set
    FlagNotSet { flag_id: u32 },

    /// Boss kill count comparison
    KillCount {
        flag_id: u32,
        op: ComparisonOp,
        value: u32,
    },

    /// In-game time comparison (milliseconds)
    Igt {
        op: ComparisonOp,
        value: i32,
    },

    /// Position within radius of target
    PositionRadius {
        target: Position3D,
        radius: f32,
    },

    /// Position in bounding box
    PositionBox {
        min: Position3D,
        max: Position3D,
    },

    /// Loading state check
    IsLoading { expected: bool },

    /// Custom memory read (for advanced users)
    MemoryValue {
        address_offset: usize,
        value_type: ValueType,
        op: ComparisonOp,
        value: i64,
    },
}

/// Value types for custom memory reads
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValueType {
    U8,
    U16,
    U32,
    U64,
    I8,
    I16,
    I32,
    I64,
    F32,
    Bool,
}

/// An autosplit trigger that combines conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutosplitTrigger {
    /// Unique identifier for this trigger
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// Description of what this trigger does
    pub description: Option<String>,

    /// Conditions that must be met (all must be true)
    pub conditions: Vec<TriggerCondition>,

    /// Whether this trigger has been activated this run
    #[serde(default)]
    pub activated: bool,

    /// Whether this trigger should reset on run reset
    #[serde(default = "default_true")]
    pub reset_on_run_reset: bool,

    /// Optional index in the split list (for ordered splits)
    pub split_index: Option<usize>,
}

fn default_true() -> bool {
    true
}

impl AutosplitTrigger {
    /// Create a new trigger
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            conditions: Vec::new(),
            activated: false,
            reset_on_run_reset: true,
            split_index: None,
        }
    }

    /// Add a condition to this trigger
    pub fn with_condition(mut self, condition: TriggerCondition) -> Self {
        self.conditions.push(condition);
        self
    }

    /// Set the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the split index
    pub fn with_split_index(mut self, index: usize) -> Self {
        self.split_index = Some(index);
        self
    }

    /// Reset the activation state
    pub fn reset(&mut self) {
        if self.reset_on_run_reset {
            self.activated = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_distance() {
        let a = Position3D::new(0.0, 0.0, 0.0);
        let b = Position3D::new(3.0, 4.0, 0.0);
        assert!((a.distance_to(&b) - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_comparison_ops() {
        assert!(ComparisonOp::Equal.compare(&5, &5));
        assert!(!ComparisonOp::Equal.compare(&5, &6));
        assert!(ComparisonOp::LessThan.compare(&5, &6));
        assert!(ComparisonOp::GreaterThanOrEqual.compare(&5, &5));
    }

    #[test]
    fn test_trigger_builder() {
        let trigger = AutosplitTrigger::new("test", "Test Trigger")
            .with_condition(TriggerCondition::FlagSet { flag_id: 1000 })
            .with_description("A test trigger")
            .with_split_index(0);

        assert_eq!(trigger.id, "test");
        assert_eq!(trigger.name, "Test Trigger");
        assert_eq!(trigger.conditions.len(), 1);
        assert_eq!(trigger.split_index, Some(0));
    }
}
