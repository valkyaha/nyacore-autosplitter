//! Main autosplitter runner

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use parking_lot::Mutex;

use super::events::{EventHandler, SplitCallback, SplitEvent};
use super::state::{AutosplitterState, BossFlag, SplitTriggerConfig};
use crate::games::GameRegistry;
use crate::{AutosplitterError, Result};

/// Main autosplitter runner that orchestrates boss/trigger detection
pub struct Autosplitter {
    /// Current state
    state: Arc<Mutex<AutosplitterState>>,
    /// Whether the autosplitter is running
    running: Arc<AtomicBool>,
    /// Signal to reset flag checking
    reset_requested: Arc<AtomicBool>,
    /// Event handler for split callbacks
    events: Arc<Mutex<EventHandler>>,
    /// Game registry for looking up game implementations
    registry: Arc<GameRegistry>,
}

// Safety: All fields are thread-safe (Arc, AtomicBool, Mutex)
unsafe impl Send for Autosplitter {}
unsafe impl Sync for Autosplitter {}

impl Autosplitter {
    /// Create a new autosplitter with the given game registry
    pub fn new(registry: GameRegistry) -> Self {
        Self {
            state: Arc::new(Mutex::new(AutosplitterState::default())),
            running: Arc::new(AtomicBool::new(false)),
            reset_requested: Arc::new(AtomicBool::new(false)),
            events: Arc::new(Mutex::new(EventHandler::new())),
            registry: Arc::new(registry),
        }
    }

    /// Get the current state
    pub fn state(&self) -> AutosplitterState {
        self.state.lock().clone()
    }

    /// Check if the autosplitter is currently running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Register a callback for split events
    pub fn on_split(&self, callback: SplitCallback) {
        self.events.lock().on_split(callback);
    }

    /// Emit a split event
    fn emit_split(&self, event: SplitEvent) {
        self.events.lock().emit(event);
    }

    /// Start the autosplitter for a specific game with boss flags
    pub fn start(&self, game_id: &str, boss_flags: Vec<BossFlag>) -> Result<()> {
        if self.running.load(Ordering::SeqCst) {
            return Err(AutosplitterError::AlreadyRunning);
        }

        // Verify game exists in registry
        if !self.registry.has_game(game_id) {
            return Err(AutosplitterError::GameNotFound(game_id.to_string()));
        }

        // Update state
        {
            let mut state = self.state.lock();
            state.running = true;
            state.game_id = game_id.to_string();
            state.bosses_defeated.clear();
            state.triggers_matched.clear();
            state.boss_kill_counts.clear();
        }

        self.running.store(true, Ordering::SeqCst);
        log::info!("Autosplitter started for game: {}", game_id);

        // TODO: Spawn background thread for detection loop
        // This will be implemented in Phase 7

        Ok(())
    }

    /// Start the autosplitter with custom triggers
    pub fn start_with_triggers(
        &self,
        game_id: &str,
        boss_flags: Vec<BossFlag>,
        triggers: Vec<SplitTriggerConfig>,
    ) -> Result<()> {
        // For now, delegate to start() - triggers will be handled in Phase 7
        self.start(game_id, boss_flags)
    }

    /// Start in auto-detect mode (find any supported running game)
    pub fn start_autodetect(&self) -> Result<()> {
        if self.running.load(Ordering::SeqCst) {
            return Err(AutosplitterError::AlreadyRunning);
        }

        // Update state to scanning mode
        {
            let mut state = self.state.lock();
            state.running = true;
            state.game_id = String::new();
        }

        self.running.store(true, Ordering::SeqCst);
        log::info!("Autosplitter started in auto-detect mode");

        // TODO: Spawn background thread for process scanning
        // This will be implemented in Phase 7

        Ok(())
    }

    /// Stop the autosplitter
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);

        let mut state = self.state.lock();
        state.running = false;
        state.process_attached = false;
        state.process_id = None;

        log::info!("Autosplitter stopped");
    }

    /// Reset the autosplitter (re-check all flags from scratch)
    pub fn reset(&self) {
        self.reset_requested.store(true, Ordering::SeqCst);

        let mut state = self.state.lock();
        state.bosses_defeated.clear();
        state.triggers_matched.clear();
        state.boss_kill_counts.clear();

        log::info!("Autosplitter reset - will re-check all flags");
    }

    /// Get the list of defeated boss IDs
    pub fn get_defeated_bosses(&self) -> Vec<String> {
        self.state.lock().bosses_defeated.clone()
    }

    /// Get the game registry
    pub fn registry(&self) -> &GameRegistry {
        &self.registry
    }
}

impl Default for Autosplitter {
    fn default() -> Self {
        Self::new(GameRegistry::new())
    }
}
