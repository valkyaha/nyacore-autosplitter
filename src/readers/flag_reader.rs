//! FlagReader trait and factory for game-specific flag reading algorithms
//!
//! Each game has a unique way of storing and reading boss defeat flags in memory.
//! The FlagReader trait abstracts this away, allowing the autosplitter to work
//! with any game through a common interface.

#[cfg(target_os = "windows")]
use std::collections::HashMap;
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HANDLE;

#[cfg(target_os = "windows")]
use crate::config::{AutosplitterMemoryConfig, PatternConfig};

/// Context containing discovered memory pointers for a game
#[cfg(target_os = "windows")]
#[derive(Debug, Clone)]
pub struct MemoryContext {
    /// Process handle for memory reading
    pub handle: HANDLE,
    /// Named pointers discovered from pattern scanning (e.g., "sprj_event_flag_man" -> address)
    pub pointers: HashMap<String, usize>,
}

/// Trait for reading game-specific boss defeat flags
///
/// Each game uses different memory structures and algorithms to store flags.
/// Implementations handle the game-specific logic for:
/// - Pattern scanning to find base pointers
/// - Pointer chain traversal
/// - Flag bit/byte extraction
#[cfg(target_os = "windows")]
pub trait FlagReader: Send + Sync {
    /// Get the algorithm name (e.g., "ds3", "eldenring", "ds1", "ds2", "sekiro")
    fn algorithm_name(&self) -> &'static str;

    /// Scan for patterns in process memory and return discovered pointers
    ///
    /// # Arguments
    /// * `handle` - Process handle for memory reading
    /// * `base` - Base address of the main module
    /// * `size` - Size of the main module
    /// * `patterns` - Pattern configurations from the plugin
    ///
    /// # Returns
    /// A map of pattern names to their resolved addresses, or None if required patterns failed
    fn scan_patterns(
        &self,
        handle: HANDLE,
        base: usize,
        size: usize,
        patterns: &[PatternConfig],
    ) -> Option<HashMap<String, usize>>;

    /// Check if a specific flag is set
    ///
    /// # Arguments
    /// * `ctx` - Memory context with process handle and discovered pointers
    /// * `flag_id` - The flag ID to check (meaning varies by game)
    ///
    /// # Returns
    /// true if the flag is set (boss defeated), false otherwise
    fn is_flag_set(&self, ctx: &MemoryContext, flag_id: u32) -> bool;

    /// Get all defeated flags from a list of flag IDs
    ///
    /// Default implementation calls is_flag_set for each, but implementations
    /// can override for batch optimization.
    fn get_defeated_flags(&self, ctx: &MemoryContext, flag_ids: &[u32]) -> Vec<u32> {
        flag_ids
            .iter()
            .filter(|&&id| self.is_flag_set(ctx, id))
            .copied()
            .collect()
    }
}

/// Factory function to create a FlagReader from plugin autosplitter configuration
///
/// The configuration must include:
/// - `algorithm`: One of "category_decomposition", "binary_tree", "offset_table", "kill_counter"
/// - Algorithm-specific config (e.g., `category_config`, `tree_config`, etc.)
/// - `patterns`: Pattern configurations for memory scanning
#[cfg(target_os = "windows")]
pub fn create_flag_reader(config: &AutosplitterMemoryConfig) -> Option<Box<dyn FlagReader>> {
    use super::plugin_reader::PluginFlagReader;

    let algorithm = config.algorithm.to_lowercase();

    // Validate that the required config is present for the algorithm
    match algorithm.as_str() {
        "category_decomposition" => {
            if config.category_config.is_none() {
                log::error!("category_decomposition algorithm requires category_config");
                return None;
            }
        }
        "binary_tree" => {
            if config.tree_config.is_none() {
                log::error!("binary_tree algorithm requires tree_config");
                return None;
            }
        }
        "offset_table" => {
            if config.offset_table_config.is_none() {
                log::error!("offset_table algorithm requires offset_table_config");
                return None;
            }
        }
        "kill_counter" => {
            if config.kill_counter_config.is_none() {
                log::error!("kill_counter algorithm requires kill_counter_config");
                return None;
            }
        }
        _ => {
            log::warn!("Unknown flag reader algorithm: {}", algorithm);
            return None;
        }
    }

    Some(Box::new(PluginFlagReader::new(config.clone())))
}

// =============================================================================
// Linux Implementation
// =============================================================================

#[cfg(target_os = "linux")]
use std::collections::HashMap;

#[cfg(target_os = "linux")]
use crate::config::{AutosplitterMemoryConfig, PatternConfig};

/// Context containing discovered memory pointers for a game (Linux)
#[cfg(target_os = "linux")]
#[derive(Debug, Clone)]
pub struct MemoryContext {
    /// Process ID for memory reading
    pub pid: i32,
    /// Named pointers discovered from pattern scanning
    pub pointers: HashMap<String, usize>,
}

/// Trait for reading game-specific boss defeat flags (Linux)
#[cfg(target_os = "linux")]
pub trait FlagReader: Send + Sync {
    fn algorithm_name(&self) -> &'static str;

    fn scan_patterns(
        &self,
        pid: i32,
        base: usize,
        size: usize,
        patterns: &[PatternConfig],
    ) -> Option<HashMap<String, usize>>;

    fn is_flag_set(&self, ctx: &MemoryContext, flag_id: u32) -> bool;

    fn get_defeated_flags(&self, ctx: &MemoryContext, flag_ids: &[u32]) -> Vec<u32> {
        flag_ids
            .iter()
            .filter(|&&id| self.is_flag_set(ctx, id))
            .copied()
            .collect()
    }
}

/// Factory function to create a FlagReader (Linux)
#[cfg(target_os = "linux")]
pub fn create_flag_reader(config: &AutosplitterMemoryConfig) -> Option<Box<dyn FlagReader>> {
    use super::plugin_reader::PluginFlagReader;

    let algorithm = config.algorithm.to_lowercase();

    match algorithm.as_str() {
        "category_decomposition" => {
            if config.category_config.is_none() {
                log::error!("category_decomposition algorithm requires category_config");
                return None;
            }
        }
        "binary_tree" => {
            if config.tree_config.is_none() {
                log::error!("binary_tree algorithm requires tree_config");
                return None;
            }
        }
        "offset_table" => {
            if config.offset_table_config.is_none() {
                log::error!("offset_table algorithm requires offset_table_config");
                return None;
            }
        }
        "kill_counter" => {
            if config.kill_counter_config.is_none() {
                log::error!("kill_counter algorithm requires kill_counter_config");
                return None;
            }
        }
        _ => {
            log::warn!("Unknown flag reader algorithm: {}", algorithm);
            return None;
        }
    }

    Some(Box::new(PluginFlagReader::new(config.clone())))
}
