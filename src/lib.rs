//! NYA Core Autosplitter Library
//!
//! A memory-based autosplitter for FromSoftware games that can be used
//! as a standalone library or integrated with NYA Core.
//!
//! # Features
//!
//! - **Memory-based detection**: Reads game memory to detect boss defeats
//! - **Cross-platform**: Supports Windows (primary) and Linux (via Proton)
//! - **Pluggable games**: Easy to add new game implementations via traits
//! - **Custom triggers**: Support for complex split conditions beyond boss flags
//! - **Vision autosplitter**: Optional screen capture-based detection (feature-gated)
//!
//! # Example
//!
//! ```no_run
//! use std::path::Path;
//! use nyacore_autosplitter::{Autosplitter, GameRegistry};
//!
//! // Load games from NYA-Core-Assets plugins directory
//! let plugins_dir = Path::new("path/to/NYA-Core-Assets/plugins");
//! let registry = GameRegistry::from_plugins_dir(plugins_dir);
//!
//! let mut autosplitter = Autosplitter::new(registry);
//! autosplitter.start_autodetect().unwrap();
//! ```

pub mod core;
pub mod games;
pub mod memory;
pub mod readers;
pub mod triggers;

#[cfg(feature = "vision")]
pub mod vision;

// Re-export main types for convenience
pub use crate::core::{
    Autosplitter, AutosplitterState, BossFlag, SplitEvent, SplitTriggerConfig,
};
pub use crate::games::{
    Game, GameFactory, GameRegistry,
    CustomTriggerType, CustomTriggerParam, CustomTriggerParamType, CustomTriggerChoice,
    TriggerTypeInfo, AttributeInfo,
    // Game data loading
    config::{GameData, BossDefinition, PresetDefinition, BossesConfig, PresetsConfig},
    registry::{load_all_game_data, load_game_data, get_presets_for_game,
               get_bosses_for_preset, get_boss_flags_for_preset, get_boss_kill_offsets_for_preset},
};
pub use crate::memory::{MemoryReader, ProcessContext};
pub use crate::readers::FlagReader;
pub use crate::triggers::{
    AutosplitConfig, AutosplitTrigger, TriggerLogic,
    TriggerCondition, ComparisonOp, Position3D, TriggerEvaluator,
};

/// Error types for the autosplitter
#[derive(Debug, thiserror::Error)]
pub enum AutosplitterError {
    #[error("Game not found: {0}")]
    GameNotFound(String),

    #[error("Process not found for game: {0}")]
    ProcessNotFound(String),

    #[error("Failed to attach to process: {0}")]
    AttachFailed(String),

    #[error("Pattern scan failed: {0}")]
    PatternScanFailed(String),

    #[error("Memory read failed at address {address:#x}: {reason}")]
    MemoryReadFailed { address: usize, reason: String },

    #[error("Autosplitter already running")]
    AlreadyRunning,

    #[error("Autosplitter not running")]
    NotRunning,

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Platform not supported: {0}")]
    PlatformNotSupported(String),
}

pub type Result<T> = std::result::Result<T, AutosplitterError>;
