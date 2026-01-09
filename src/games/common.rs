//! Common utilities and macros for game implementations
//!
//! This module provides shared functionality and a standardized pattern
//! for implementing new games in the autosplitter.
//!
//! # Adding a New Game
//!
//! 1. Create a new file: `src/games/my_game.rs`
//! 2. Follow the template structure below
//! 3. Add the module and factory exports to `src/games/mod.rs`
//! 4. Register the factory in `src/games/registry.rs`
//!
//! # Template Structure
//!
//! ```ignore
//! use std::collections::HashMap;
//! use std::sync::Arc;
//!
//! use super::{
//!     Game, GameFactory, BoxedGame, Position3D,
//!     TriggerTypeInfo, AttributeInfo,
//!     CustomTriggerType, CustomTriggerParam, CustomTriggerParamType, CustomTriggerChoice,
//! };
//! use crate::memory::{ProcessContext, MemoryReader, Pointer, parse_pattern, extract_relative_address};
//! use crate::AutosplitterError;
//!
//! // =============================================================================
//! // CONSTANTS
//! // =============================================================================
//!
//! /// Game metadata
//! pub const GAME_ID: &str = "my-game";
//! pub const GAME_NAME: &str = "My Game";
//! pub const PROCESS_NAMES: &[&str] = &["mygame.exe"];
//!
//! /// Memory patterns (document source!)
//! pub const SOME_PATTERN: &str = "48 8b 05 ?? ?? ?? ??";
//!
//! // =============================================================================
//! // GAME IMPLEMENTATION
//! // =============================================================================
//!
//! pub struct MyGame {
//!     reader: Option<Arc<dyn MemoryReader>>,
//!     initialized: bool,
//!     // Add pointers here
//! }
//!
//! impl MyGame {
//!     pub fn new() -> Self { ... }
//!     fn reader(&self) -> Option<&dyn MemoryReader> { ... }
//! }
//!
//! impl Default for MyGame {
//!     fn default() -> Self { Self::new() }
//! }
//!
//! impl Game for MyGame { ... }
//!
//! // =============================================================================
//! // FACTORY
//! // =============================================================================
//!
//! pub struct MyGameFactory;
//!
//! impl GameFactory for MyGameFactory {
//!     fn game_id(&self) -> &'static str { GAME_ID }
//!     fn process_names(&self) -> &[&'static str] { PROCESS_NAMES }
//!     fn create(&self) -> BoxedGame { Box::new(MyGame::new()) }
//! }
//! ```

use std::sync::Arc;
use crate::memory::MemoryReader;

/// Common trait for games that provides a reader accessor
pub trait GameReader {
    /// Get the memory reader if available
    fn reader(&self) -> Option<&dyn MemoryReader>;

    /// Check if pointers are initialized
    fn is_initialized(&self) -> bool;
}

/// Helper to get reader from Arc<dyn MemoryReader>
pub fn get_reader(reader: &Option<Arc<dyn MemoryReader>>) -> Option<&dyn MemoryReader> {
    reader.as_ref().map(|r| r.as_ref())
}

/// Standard trigger types that most games support
pub fn standard_event_flag_trigger() -> super::TriggerTypeInfo {
    super::TriggerTypeInfo {
        id: "event_flag".to_string(),
        name: "Event Flag".to_string(),
        description: "Triggers when an event flag is set".to_string(),
    }
}

pub fn standard_position_trigger() -> super::TriggerTypeInfo {
    super::TriggerTypeInfo {
        id: "position".to_string(),
        name: "Position".to_string(),
        description: "Triggers when player enters an area".to_string(),
    }
}

pub fn standard_loading_trigger() -> super::TriggerTypeInfo {
    super::TriggerTypeInfo {
        id: "loading".to_string(),
        name: "Loading State".to_string(),
        description: "Triggers on loading screen transitions".to_string(),
    }
}

pub fn standard_igt_trigger() -> super::TriggerTypeInfo {
    super::TriggerTypeInfo {
        id: "igt".to_string(),
        name: "In-Game Time".to_string(),
        description: "Triggers based on in-game time".to_string(),
    }
}

/// Macro to implement the standard GameFactory pattern
///
/// Usage:
/// ```ignore
/// impl_game_factory!(MyGameFactory, MyGame, GAME_ID, PROCESS_NAMES);
/// ```
#[macro_export]
macro_rules! impl_game_factory {
    ($factory:ident, $game:ident, $id:expr, $processes:expr) => {
        pub struct $factory;

        impl $crate::games::GameFactory for $factory {
            fn game_id(&self) -> &'static str {
                $id
            }

            fn process_names(&self) -> &[&'static str] {
                $processes
            }

            fn create(&self) -> $crate::games::BoxedGame {
                Box::new($game::new())
            }
        }
    };
}

/// Macro to implement common Game trait methods
///
/// Usage:
/// ```ignore
/// impl Game for MyGame {
///     impl_game_basics!(GAME_ID, GAME_NAME, PROCESS_NAMES);
///     // ... rest of implementation
/// }
/// ```
#[macro_export]
macro_rules! impl_game_basics {
    ($id:expr, $name:expr, $processes:expr) => {
        fn id(&self) -> &'static str {
            $id
        }

        fn name(&self) -> &'static str {
            $name
        }

        fn process_names(&self) -> &[&'static str] {
            $processes
        }

        fn is_alive(&self) -> bool {
            self.initialized
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_triggers() {
        let trigger = standard_event_flag_trigger();
        assert_eq!(trigger.id, "event_flag");

        let trigger = standard_position_trigger();
        assert_eq!(trigger.id, "position");
    }
}
