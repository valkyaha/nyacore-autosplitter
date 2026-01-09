//! Game registry for discovering and creating games

use std::collections::HashMap;
use std::path::Path;
use super::BoxedGame;
use super::configurable::ConfigurableGameFactory;
use super::config::{GameData, BossDefinition, PresetDefinition};

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

    /// Create a new registry and load all games from a plugins directory
    /// This is the preferred way to create a registry - games are loaded from
    /// TOML configuration files in NYA-Core-Assets/plugins
    pub fn from_plugins_dir(plugins_dir: &Path) -> Self {
        let mut registry = Self::new();
        registry.register_from_plugins_dir(plugins_dir);
        registry
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

    /// Register built-in hardcoded games (fallback when configs unavailable)
    /// DEPRECATED: Use register_from_plugins_dir() instead
    #[deprecated(note = "Use register_from_plugins_dir() to load games from TOML configs")]
    pub fn register_builtin(&mut self) {
        use super::{
            DarkSouls1Factory,
            DarkSouls2Factory,
            DarkSouls3Factory,
            EldenRingFactory,
            SekiroFactory,
            ArmoredCore6Factory,
        };

        log::warn!("Using deprecated register_builtin() - prefer register_from_plugins_dir()");

        self.register(Box::new(DarkSouls1Factory));
        self.register(Box::new(DarkSouls2Factory));
        self.register(Box::new(DarkSouls3Factory));
        self.register(Box::new(EldenRingFactory));
        self.register(Box::new(SekiroFactory));
        self.register(Box::new(ArmoredCore6Factory));

        log::info!("Registered {} built-in games", self.factories.len());
    }

    /// Register games from a plugins directory (NYA-Core-Assets/plugins)
    ///
    /// Each subdirectory should contain plugin.toml and autosplitter.toml
    pub fn register_from_plugins_dir(&mut self, plugins_dir: &Path) {
        if !plugins_dir.exists() {
            log::warn!("Plugins directory does not exist: {:?}", plugins_dir);
            return;
        }

        log::info!("Loading games from plugins directory: {:?}", plugins_dir);

        let entries = match std::fs::read_dir(plugins_dir) {
            Ok(e) => e,
            Err(e) => {
                log::error!("Failed to read plugins directory: {}", e);
                return;
            }
        };

        let mut loaded = 0;
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let plugin_toml = path.join("plugin.toml");
            let autosplitter_toml = path.join("autosplitter.toml");

            if !plugin_toml.exists() || !autosplitter_toml.exists() {
                log::debug!("Skipping {:?}: missing config files", path);
                continue;
            }

            match ConfigurableGameFactory::from_dir(&path) {
                Ok(factory) => {
                    let game_id = factory.game_id().to_string();
                    log::info!("Loaded configurable game: {}", game_id);
                    self.register(Box::new(factory));
                    loaded += 1;
                }
                Err(e) => {
                    log::error!("Failed to load game from {:?}: {}", path, e);
                }
            }
        }

        log::info!("Loaded {} games from plugins directory", loaded);
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

// =============================================================================
// GAME DATA LOADING (STANDALONE FUNCTIONS)
// =============================================================================

/// Load all game data from a plugins directory
/// Returns a map of game_id -> GameData
pub fn load_all_game_data(plugins_dir: &Path) -> HashMap<String, GameData> {
    let mut games = HashMap::new();

    if !plugins_dir.exists() {
        log::warn!("Plugins directory does not exist: {:?}", plugins_dir);
        return games;
    }

    let entries = match std::fs::read_dir(plugins_dir) {
        Ok(e) => e,
        Err(e) => {
            log::error!("Failed to read plugins directory: {}", e);
            return games;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        match GameData::load_from_dir(&path) {
            Ok(data) => {
                let game_id = data.plugin.plugin.id.clone();
                log::debug!("Loaded game data for: {}", game_id);
                games.insert(game_id, data);
            }
            Err(e) => {
                log::debug!("Skipping {:?}: {}", path, e);
            }
        }
    }

    log::info!("Loaded game data for {} games", games.len());
    games
}

/// Load game data for a specific game ID
pub fn load_game_data(plugins_dir: &Path, game_id: &str) -> Option<GameData> {
    let game_dir = plugins_dir.join(game_id);
    if !game_dir.exists() {
        return None;
    }

    GameData::load_from_dir(&game_dir).ok()
}

/// Get all available presets for a game
pub fn get_presets_for_game(game_data: &GameData) -> Vec<&PresetDefinition> {
    game_data.presets.presets.iter().collect()
}

/// Get all bosses for a specific preset
pub fn get_bosses_for_preset<'a>(game_data: &'a GameData, preset_id: &str) -> Vec<&'a BossDefinition> {
    game_data.get_bosses_for_preset(preset_id)
}

/// Get boss flag IDs for a preset (for autosplitter)
pub fn get_boss_flags_for_preset(game_data: &GameData, preset_id: &str) -> Vec<(String, u32)> {
    game_data.get_bosses_for_preset(preset_id)
        .into_iter()
        .filter_map(|boss| {
            boss.flag_id.map(|flag| (boss.id.clone(), flag))
        })
        .collect()
}

/// Get boss kill offsets for a preset (for DS2-style games)
pub fn get_boss_kill_offsets_for_preset(game_data: &GameData, preset_id: &str) -> Vec<(String, u32)> {
    game_data.get_bosses_for_preset(preset_id)
        .into_iter()
        .filter_map(|boss| {
            boss.kill_offset.map(|offset| (boss.id.clone(), offset))
        })
        .collect()
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
