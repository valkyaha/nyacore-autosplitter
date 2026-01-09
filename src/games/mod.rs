//! Game trait and implementations
//!
//! This module defines the `Game` trait that all game implementations must satisfy,
//! along with the `GameRegistry` for managing and discovering games.

mod registry;

// Game implementations
pub mod dark_souls_1;
pub mod dark_souls_2;
pub mod dark_souls_3;
pub mod elden_ring;
pub mod sekiro;
pub mod armored_core_6;

pub use registry::{GameFactory, GameRegistry};

// Re-export game factories
pub use dark_souls_1::DarkSouls1Factory;
pub use dark_souls_2::DarkSouls2Factory;
pub use dark_souls_3::DarkSouls3Factory;
pub use elden_ring::EldenRingFactory;
pub use sekiro::SekiroFactory;
pub use armored_core_6::ArmoredCore6Factory;

use crate::memory::ProcessContext;
use crate::AutosplitterError;

// Re-export Position3D from triggers for convenience
pub use crate::triggers::Position3D;

/// Information about supported trigger types for a game
#[derive(Debug, Clone)]
pub struct TriggerTypeInfo {
    pub id: String,
    pub name: String,
    pub description: String,
}

/// Information about character attributes
#[derive(Debug, Clone)]
pub struct AttributeInfo {
    pub id: String,
    pub name: String,
}

/// Trait that all game implementations must satisfy
pub trait Game: Send + Sync {
    /// Unique identifier for this game (e.g., "dark-souls-3")
    fn id(&self) -> &'static str;

    /// Human-readable game name (e.g., "Dark Souls III")
    fn name(&self) -> &'static str;

    /// Process names to search for (e.g., ["DarkSoulsIII.exe"])
    fn process_names(&self) -> &[&'static str];

    /// Initialize pointers after attaching to process
    fn init_pointers(&mut self, context: &mut ProcessContext) -> Result<(), AutosplitterError>;

    /// Read an event flag by ID
    fn read_event_flag(&self, flag_id: u32) -> bool;

    /// Get boss kill count (for multi-kill tracking like DS2 ascetics)
    /// Default implementation returns 1 if flag is set, 0 otherwise
    fn get_boss_kill_count(&self, flag_id: u32) -> u32 {
        if self.read_event_flag(flag_id) {
            1
        } else {
            0
        }
    }

    /// Check if the process is still alive
    fn is_alive(&self) -> bool;

    /// Get in-game time in milliseconds (optional)
    fn get_igt_milliseconds(&self) -> Option<i32> {
        None
    }

    /// Get player position (optional)
    fn get_position(&self) -> Option<Position3D> {
        None
    }

    /// Check if the game is currently loading (optional)
    fn is_loading(&self) -> Option<bool> {
        None
    }

    /// Check if the player character is loaded (optional)
    fn is_player_loaded(&self) -> Option<bool> {
        None
    }

    /// Check if the screen is black (optional)
    fn is_blackscreen(&self) -> Option<bool> {
        None
    }

    /// Get a character attribute value (optional)
    fn get_attribute(&self, _attr: &str) -> Option<i32> {
        None
    }

    /// Get current NG+ level (optional)
    fn get_ng_level(&self) -> Option<i32> {
        None
    }

    /// Get current player health (optional)
    fn get_player_health(&self) -> Option<i32> {
        None
    }

    /// Get max player health (optional)
    fn get_player_max_health(&self) -> Option<i32> {
        None
    }

    /// Get screen state (Elden Ring specific, returns i32 for ScreenState enum)
    fn get_screen_state(&self) -> Option<i32> {
        None
    }

    /// Check if warp is requested (DS1 specific)
    fn is_warp_requested(&self) -> Option<bool> {
        None
    }

    /// Check if credits are rolling (DS1 specific)
    fn are_credits_rolling(&self) -> Option<bool> {
        None
    }

    /// Get boss kill count raw by offset (DS2 specific)
    fn get_boss_kill_count_raw(&self, _boss_offset: u32) -> Option<i32> {
        None
    }

    /// Get map area info (area, block, region)
    fn get_map_area(&self) -> Option<(u8, u8, u8)> {
        None
    }

    /// Get supported trigger types for this game
    fn supported_triggers(&self) -> Vec<TriggerTypeInfo> {
        vec![
            TriggerTypeInfo {
                id: "event_flag".to_string(),
                name: "Event Flag".to_string(),
                description: "Triggers when an event flag is set or unset".to_string(),
            },
        ]
    }

    /// Get available attributes for this game
    fn available_attributes(&self) -> Vec<AttributeInfo> {
        vec![]
    }
}

/// Boxed game that can be stored and passed around
pub type BoxedGame = Box<dyn Game>;

// Placeholder game implementations will be added in Phase 4
// - dark_souls_1.rs
// - dark_souls_2.rs
// - dark_souls_3.rs
// - elden_ring.rs
// - sekiro.rs
// - armored_core_6.rs
