//! Trigger evaluation engine

use super::{AutosplitTrigger, TriggerCondition};
use crate::games::Game;
use crate::memory::MemoryReader;

/// Evaluates trigger conditions against game state
pub struct TriggerEvaluator<'a> {
    game: &'a dyn Game,
    reader: &'a dyn MemoryReader,
}

impl<'a> TriggerEvaluator<'a> {
    /// Create a new evaluator for the given game
    pub fn new(game: &'a dyn Game, reader: &'a dyn MemoryReader) -> Self {
        Self { game, reader }
    }

    /// Evaluate a single trigger
    /// Returns true if all conditions are met
    pub fn evaluate(&self, trigger: &AutosplitTrigger) -> bool {
        if trigger.activated {
            return false;
        }

        // All conditions must be true
        trigger.conditions.iter().all(|c| self.evaluate_condition(c))
    }

    /// Evaluate a single condition
    pub fn evaluate_condition(&self, condition: &TriggerCondition) -> bool {
        match condition {
            TriggerCondition::FlagSet { flag_id } => {
                self.game.read_event_flag(*flag_id)
            }

            TriggerCondition::FlagNotSet { flag_id } => {
                !self.game.read_event_flag(*flag_id)
            }

            TriggerCondition::KillCount { flag_id, op, value } => {
                let count = self.game.get_boss_kill_count(*flag_id);
                op.compare(&count, value)
            }

            TriggerCondition::Igt { op, value } => {
                if let Some(igt) = self.game.get_igt_milliseconds() {
                    op.compare(&igt, value)
                } else {
                    false
                }
            }

            TriggerCondition::PositionRadius { target, radius } => {
                if let Some(pos) = self.game.get_position() {
                    pos.distance_to(target) <= *radius
                } else {
                    false
                }
            }

            TriggerCondition::PositionBox { min, max } => {
                if let Some(pos) = self.game.get_position() {
                    pos.x >= min.x && pos.x <= max.x &&
                    pos.y >= min.y && pos.y <= max.y &&
                    pos.z >= min.z && pos.z <= max.z
                } else {
                    false
                }
            }

            TriggerCondition::IsLoading { expected } => {
                if let Some(loading) = self.game.is_loading() {
                    loading == *expected
                } else {
                    false
                }
            }

            TriggerCondition::MemoryValue { address_offset, value_type, op, value } => {
                // Custom memory reads need base address from game
                // For now, return false - implementation depends on game specifics
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::ProcessContext;
    use crate::AutosplitterError;
    use crate::triggers::{ComparisonOp, Position3D};

    // Mock game for testing
    struct MockGame {
        flags: std::collections::HashMap<u32, bool>,
        kill_counts: std::collections::HashMap<u32, u32>,
        igt: Option<i32>,
        position: Option<Position3D>,
        loading: Option<bool>,
    }

    impl MockGame {
        fn new() -> Self {
            Self {
                flags: std::collections::HashMap::new(),
                kill_counts: std::collections::HashMap::new(),
                igt: None,
                position: None,
                loading: None,
            }
        }
    }

    impl Game for MockGame {
        fn id(&self) -> &'static str { "mock" }
        fn name(&self) -> &'static str { "Mock Game" }
        fn process_names(&self) -> &[&'static str] { &["mock.exe"] }
        fn init_pointers(&mut self, _ctx: &mut ProcessContext) -> Result<(), AutosplitterError> { Ok(()) }
        fn read_event_flag(&self, flag_id: u32) -> bool {
            *self.flags.get(&flag_id).unwrap_or(&false)
        }
        fn get_boss_kill_count(&self, flag_id: u32) -> u32 {
            *self.kill_counts.get(&flag_id).unwrap_or(&0)
        }
        fn is_alive(&self) -> bool { true }
        fn get_igt_milliseconds(&self) -> Option<i32> { self.igt }
        fn get_position(&self) -> Option<Position3D> { self.position }
        fn is_loading(&self) -> Option<bool> { self.loading }
    }

    // Mock reader
    struct MockReader;
    impl MemoryReader for MockReader {
        fn read_bytes(&self, _address: usize, _size: usize) -> Option<Vec<u8>> { None }
    }

    #[test]
    fn test_flag_set_condition() {
        let mut game = MockGame::new();
        game.flags.insert(1000, true);

        let reader = MockReader;
        let evaluator = TriggerEvaluator::new(&game, &reader);

        assert!(evaluator.evaluate_condition(&TriggerCondition::FlagSet { flag_id: 1000 }));
        assert!(!evaluator.evaluate_condition(&TriggerCondition::FlagSet { flag_id: 2000 }));
    }

    #[test]
    fn test_kill_count_condition() {
        let mut game = MockGame::new();
        game.kill_counts.insert(1000, 5);

        let reader = MockReader;
        let evaluator = TriggerEvaluator::new(&game, &reader);

        assert!(evaluator.evaluate_condition(&TriggerCondition::KillCount {
            flag_id: 1000,
            op: ComparisonOp::GreaterThan,
            value: 3,
        }));

        assert!(!evaluator.evaluate_condition(&TriggerCondition::KillCount {
            flag_id: 1000,
            op: ComparisonOp::GreaterThan,
            value: 10,
        }));
    }

    #[test]
    fn test_position_radius_condition() {
        let mut game = MockGame::new();
        game.position = Some(Position3D::new(10.0, 0.0, 10.0));

        let reader = MockReader;
        let evaluator = TriggerEvaluator::new(&game, &reader);

        assert!(evaluator.evaluate_condition(&TriggerCondition::PositionRadius {
            target: Position3D::new(10.0, 0.0, 10.0),
            radius: 5.0,
        }));

        assert!(!evaluator.evaluate_condition(&TriggerCondition::PositionRadius {
            target: Position3D::new(100.0, 0.0, 100.0),
            radius: 5.0,
        }));
    }

    #[test]
    fn test_trigger_evaluation() {
        let mut game = MockGame::new();
        game.flags.insert(1000, true);
        game.kill_counts.insert(2000, 1);

        let reader = MockReader;
        let evaluator = TriggerEvaluator::new(&game, &reader);

        let trigger = AutosplitTrigger::new("test", "Test")
            .with_condition(TriggerCondition::FlagSet { flag_id: 1000 })
            .with_condition(TriggerCondition::KillCount {
                flag_id: 2000,
                op: ComparisonOp::GreaterThanOrEqual,
                value: 1,
            });

        assert!(evaluator.evaluate(&trigger));
    }

    #[test]
    fn test_already_activated_trigger() {
        let game = MockGame::new();
        let reader = MockReader;
        let evaluator = TriggerEvaluator::new(&game, &reader);

        let mut trigger = AutosplitTrigger::new("test", "Test");
        trigger.activated = true;

        assert!(!evaluator.evaluate(&trigger));
    }
}
