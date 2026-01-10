//! Autosplitter Engine System
//!
//! This module provides a pluggable engine system for reading game memory and flags.
//! Different engines support different ways of defining game-specific logic:
//!
//! - **Algorithm Engine**: Built-in algorithms (CategoryDecomposition, BinaryTree, etc.)
//! - **Rhai Engine**: Custom scripting using the Rhai language
//! - **ASL Engine**: LiveSplit Auto Splitter Language compatibility
//!
//! # Example
//!
//! ```ignore
//! use nyacore_autosplitter::engines::{Engine, EngineContext};
//!
//! // Engine is selected based on autosplitter.toml config
//! let engine = create_engine_from_config(&config)?;
//!
//! // Initialize with memory context
//! engine.init(&mut ctx)?;
//!
//! // Read flags
//! let is_set = engine.read_flag(&ctx, 13000800)?;
//! ```

pub mod algorithm;
#[cfg(feature = "rhai-scripting")]
pub mod rhai_engine;
pub mod asl;

use crate::memory::MemoryReader;
use crate::AutosplitterError;
use std::collections::HashMap;
use std::sync::Arc;

/// Engine type identifier
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineType {
    /// Built-in algorithm (CategoryDecomposition, BinaryTree, etc.)
    Algorithm,
    /// Rhai scripting engine
    Rhai,
    /// LiveSplit ASL script
    Asl,
}

impl std::str::FromStr for EngineType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "algorithm" | "builtin" => Ok(EngineType::Algorithm),
            "rhai" | "script" => Ok(EngineType::Rhai),
            "asl" | "livesplit" => Ok(EngineType::Asl),
            _ => Err(format!("Unknown engine type: {}", s)),
        }
    }
}

/// Context provided to engines for memory access
pub struct EngineContext {
    /// Memory reader for the target process
    reader: Arc<dyn MemoryReader>,
    /// Base address of the main module
    base_address: usize,
    /// Size of the main module
    module_size: usize,
    /// Process ID
    pid: u32,
    /// Resolved pattern addresses (name -> address)
    pointers: HashMap<String, usize>,
    /// Custom variables that scripts can store
    variables: HashMap<String, EngineValue>,
}

/// Values that can be stored/passed in the engine
#[derive(Debug, Clone)]
pub enum EngineValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Bytes(Vec<u8>),
}

impl EngineContext {
    /// Create a new engine context
    pub fn new(
        reader: Arc<dyn MemoryReader>,
        base_address: usize,
        module_size: usize,
        pid: u32,
    ) -> Self {
        Self {
            reader,
            base_address,
            module_size,
            pid,
            pointers: HashMap::new(),
            variables: HashMap::new(),
        }
    }

    /// Get the memory reader
    pub fn reader(&self) -> &dyn MemoryReader {
        self.reader.as_ref()
    }

    /// Get the memory reader as an Arc (for engines that need to store it)
    pub fn reader_arc(&self) -> Arc<dyn MemoryReader> {
        self.reader.clone()
    }

    /// Get the base address
    pub fn base_address(&self) -> usize {
        self.base_address
    }

    /// Get the module size
    pub fn module_size(&self) -> usize {
        self.module_size
    }

    /// Get the process ID
    pub fn pid(&self) -> u32 {
        self.pid
    }

    /// Store a resolved pointer
    pub fn set_pointer(&mut self, name: &str, address: usize) {
        self.pointers.insert(name.to_string(), address);
    }

    /// Get a resolved pointer
    pub fn get_pointer(&self, name: &str) -> Option<usize> {
        self.pointers.get(name).copied()
    }

    /// Get all pointers
    pub fn pointers(&self) -> &HashMap<String, usize> {
        &self.pointers
    }

    /// Store a variable
    pub fn set_variable(&mut self, name: &str, value: EngineValue) {
        self.variables.insert(name.to_string(), value);
    }

    /// Get a variable
    pub fn get_variable(&self, name: &str) -> Option<&EngineValue> {
        self.variables.get(name)
    }

    // =========================================================================
    // MEMORY READING HELPERS
    // =========================================================================

    /// Read a byte from memory
    pub fn read_u8(&self, address: usize) -> Option<u8> {
        self.reader.read_u8(address)
    }

    /// Read a 16-bit unsigned integer
    pub fn read_u16(&self, address: usize) -> Option<u16> {
        self.reader.read_u16(address)
    }

    /// Read a 32-bit unsigned integer
    pub fn read_u32(&self, address: usize) -> Option<u32> {
        self.reader.read_u32(address)
    }

    /// Read a 32-bit signed integer
    pub fn read_i32(&self, address: usize) -> Option<i32> {
        self.reader.read_i32(address)
    }

    /// Read a 64-bit unsigned integer
    pub fn read_u64(&self, address: usize) -> Option<u64> {
        self.reader.read_u64(address)
    }

    /// Read a 64-bit signed integer
    pub fn read_i64(&self, address: usize) -> Option<i64> {
        self.reader.read_i64(address)
    }

    /// Read a 32-bit float
    pub fn read_f32(&self, address: usize) -> Option<f32> {
        self.reader.read_f32(address)
    }

    /// Read a 64-bit float
    pub fn read_f64(&self, address: usize) -> Option<f64> {
        self.reader.read_bytes(address, 8)
            .map(|b| f64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]))
    }

    /// Read a pointer (platform-dependent size, assumes 64-bit)
    pub fn read_ptr(&self, address: usize) -> Option<usize> {
        self.reader.read_ptr(address)
    }

    /// Read a boolean (non-zero = true)
    pub fn read_bool(&self, address: usize) -> Option<bool> {
        self.reader.read_bool(address)
    }

    /// Read raw bytes from memory
    pub fn read_bytes(&self, address: usize, size: usize) -> Option<Vec<u8>> {
        self.reader.read_bytes(address, size)
    }

    /// Follow a pointer chain
    pub fn follow_pointer_chain(&self, base: usize, offsets: &[i64]) -> Option<usize> {
        let mut addr = base;
        for (i, &offset) in offsets.iter().enumerate() {
            if i < offsets.len() - 1 {
                // Dereference pointer
                addr = self.read_ptr(addr)?;
            }
            // Apply offset
            addr = if offset >= 0 {
                addr.wrapping_add(offset as usize)
            } else {
                addr.wrapping_sub((-offset) as usize)
            };
        }
        Some(addr)
    }
}

/// Trait that all autosplitter engines must implement
pub trait Engine: Send + Sync {
    /// Get the engine type
    fn engine_type(&self) -> EngineType;

    /// Initialize the engine with the given context
    /// This is called once when the process is attached
    fn init(&mut self, ctx: &mut EngineContext) -> Result<(), AutosplitterError>;

    /// Read an event flag by ID
    /// Returns true if the flag is set
    fn read_flag(&self, ctx: &EngineContext, flag_id: u32) -> Result<bool, AutosplitterError>;

    /// Get the kill count for a boss (for games like DS2 with ascetics)
    /// Default implementation returns 1 if flag is set, 0 otherwise
    fn get_kill_count(&self, ctx: &EngineContext, flag_id: u32) -> Result<u32, AutosplitterError> {
        Ok(if self.read_flag(ctx, flag_id)? { 1 } else { 0 })
    }

    /// Get in-game time in milliseconds
    fn get_igt_milliseconds(&self, _ctx: &EngineContext) -> Result<Option<i32>, AutosplitterError> {
        Ok(None)
    }

    /// Check if the game is currently loading
    fn is_loading(&self, _ctx: &EngineContext) -> Result<Option<bool>, AutosplitterError> {
        Ok(None)
    }

    /// Check if the player is loaded into the world
    fn is_player_loaded(&self, _ctx: &EngineContext) -> Result<Option<bool>, AutosplitterError> {
        Ok(None)
    }

    /// Get player position (x, y, z)
    fn get_position(&self, _ctx: &EngineContext) -> Result<Option<(f32, f32, f32)>, AutosplitterError> {
        Ok(None)
    }

    /// Get a character attribute value
    fn get_attribute(&self, _ctx: &EngineContext, _attr: &str) -> Result<Option<i32>, AutosplitterError> {
        Ok(None)
    }

    /// Evaluate a custom trigger
    fn evaluate_trigger(
        &self,
        _ctx: &EngineContext,
        _trigger_id: &str,
        _params: &HashMap<String, String>,
    ) -> Result<bool, AutosplitterError> {
        Ok(false)
    }

    /// Called every tick to update state (for ASL's update action)
    fn update(&mut self, _ctx: &mut EngineContext) -> Result<(), AutosplitterError> {
        Ok(())
    }

    /// Check if a split should occur (for ASL's split action)
    fn should_split(&self, _ctx: &EngineContext) -> Result<bool, AutosplitterError> {
        Ok(false)
    }

    /// Check if the timer should start (for ASL's start action)
    fn should_start(&self, _ctx: &EngineContext) -> Result<bool, AutosplitterError> {
        Ok(false)
    }

    /// Check if the timer should reset (for ASL's reset action)
    fn should_reset(&self, _ctx: &EngineContext) -> Result<bool, AutosplitterError> {
        Ok(false)
    }
}

/// Boxed engine type
pub type BoxedEngine = Box<dyn Engine>;
