//! Game registry for discovering and creating games

use std::collections::HashMap;
use super::BoxedGame;

/// Factory for creating game instances
pub trait GameFactory: Send + Sync {
    /// Unique identifier for this game
    fn game_id(&self) -> &'static str;

    /// Process names that this game can attach to
    fn process_names(&self) -> &[&'static str];

    /// Create a new instance of this game
    fn create(&self) -> BoxedGame;
}

/// Registry for discovering and creating games
pub struct GameRegistry {
    factories: HashMap<String, Box<dyn GameFactory>>,
    /// Map from process name to game ID for quick lookup
    process_map: HashMap<String, String>,
}

impl GameRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            factories: HashMap::new(),
            process_map: HashMap::new(),
        }
    }

    /// Register a game factory
    pub fn register(&mut self, factory: Box<dyn GameFactory>) {
        let game_id = factory.game_id().to_string();

        // Map process names to game ID
        for process_name in factory.process_names() {
            self.process_map
                .insert(process_name.to_lowercase(), game_id.clone());
        }

        self.factories.insert(game_id, factory);
    }

    /// Register all built-in games
    pub fn register_builtin(&mut self) {
        use super::{
            DarkSouls1Factory,
            DarkSouls2Factory,
            DarkSouls3Factory,
            EldenRingFactory,
            SekiroFactory,
            ArmoredCore6Factory,
        };

        log::info!("Registering built-in games");

        self.register(Box::new(DarkSouls1Factory));
        self.register(Box::new(DarkSouls2Factory));
        self.register(Box::new(DarkSouls3Factory));
        self.register(Box::new(EldenRingFactory));
        self.register(Box::new(SekiroFactory));
        self.register(Box::new(ArmoredCore6Factory));

        log::info!("Registered {} built-in games", self.factories.len());
    }

    /// Check if a game is registered
    pub fn has_game(&self, game_id: &str) -> bool {
        self.factories.contains_key(game_id)
    }

    /// Create a game instance by ID
    pub fn create_game(&self, game_id: &str) -> Option<BoxedGame> {
        self.factories.get(game_id).map(|f| f.create())
    }

    /// Find game ID by process name
    pub fn find_game_by_process(&self, process_name: &str) -> Option<&str> {
        self.process_map
            .get(&process_name.to_lowercase())
            .map(|s| s.as_str())
    }

    /// Get all registered game IDs
    pub fn game_ids(&self) -> Vec<&str> {
        self.factories.keys().map(|s| s.as_str()).collect()
    }

    /// Get all process names that can be detected
    pub fn all_process_names(&self) -> Vec<&str> {
        self.factories
            .values()
            .flat_map(|f| f.process_names().iter().copied())
            .collect()
    }
}

impl Default for GameRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games::Game;
    use crate::memory::ProcessContext;
    use crate::AutosplitterError;

    // Test game implementation
    struct TestGame;

    impl Game for TestGame {
        fn id(&self) -> &'static str {
            "test-game"
        }
        fn name(&self) -> &'static str {
            "Test Game"
        }
        fn process_names(&self) -> &[&'static str] {
            &["test.exe", "test_alt.exe"]
        }
        fn init_pointers(&mut self, _ctx: &mut ProcessContext) -> Result<(), AutosplitterError> {
            Ok(())
        }
        fn read_event_flag(&self, _flag_id: u32) -> bool {
            false
        }
        fn is_alive(&self) -> bool {
            true
        }
    }

    struct TestGameFactory;

    impl GameFactory for TestGameFactory {
        fn game_id(&self) -> &'static str {
            "test-game"
        }
        fn process_names(&self) -> &[&'static str] {
            &["test.exe", "test_alt.exe"]
        }
        fn create(&self) -> BoxedGame {
            Box::new(TestGame)
        }
    }

    #[test]
    fn test_registry_registration() {
        let mut registry = GameRegistry::new();
        registry.register(Box::new(TestGameFactory));

        assert!(registry.has_game("test-game"));
        assert!(!registry.has_game("unknown-game"));
    }

    #[test]
    fn test_registry_process_lookup() {
        let mut registry = GameRegistry::new();
        registry.register(Box::new(TestGameFactory));

        assert_eq!(
            registry.find_game_by_process("test.exe"),
            Some("test-game")
        );
        assert_eq!(
            registry.find_game_by_process("TEST.EXE"),
            Some("test-game")
        );
        assert_eq!(
            registry.find_game_by_process("test_alt.exe"),
            Some("test-game")
        );
        assert_eq!(registry.find_game_by_process("unknown.exe"), None);
    }

    #[test]
    fn test_registry_create_game() {
        let mut registry = GameRegistry::new();
        registry.register(Box::new(TestGameFactory));

        let game = registry.create_game("test-game");
        assert!(game.is_some());

        let game = game.unwrap();
        assert_eq!(game.id(), "test-game");
        assert_eq!(game.name(), "Test Game");
    }
}
