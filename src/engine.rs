//! Generic game engine for data-driven autosplitter
//!
//! This module provides a unified interface for reading game state
//! using configurations loaded from TOML files.

use crate::game_data::{GameData, PatternDefinition, PointerDefinition};
use crate::memory::pointer::Pointer;
use crate::memory::{parse_pattern, resolve_rip_relative, scan_pattern};
use std::collections::HashMap;

#[cfg(target_os = "windows")]
use crate::memory::{read_i32, read_i64};
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HANDLE;

/// Engine type determines which reading algorithm to use
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineType {
    /// Dark Souls 1 PTDE - older event flag system
    Ds1Ptde,
    /// Dark Souls Remastered - updated event flag system
    Ds1Remaster,
    /// Dark Souls 2 SOTFS - kill counter system
    Ds2Sotfs,
    /// Dark Souls 3 - area-based event flags
    Ds3,
    /// Elden Ring - virtual memory flags
    EldenRing,
    /// Sekiro - event flags similar to DS3
    Sekiro,
    /// Armored Core 6 - event flags
    Ac6,
}

impl EngineType {
    /// Parse engine type from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "ds1_ptde" | "ds1ptde" => Some(Self::Ds1Ptde),
            "ds1_remaster" | "ds1remaster" | "ds1r" => Some(Self::Ds1Remaster),
            "ds2_sotfs" | "ds2sotfs" | "ds2" => Some(Self::Ds2Sotfs),
            "ds3" | "dark_souls_3" => Some(Self::Ds3),
            "elden_ring" | "eldenring" | "er" => Some(Self::EldenRing),
            "sekiro" => Some(Self::Sekiro),
            "ac6" | "armored_core_6" => Some(Self::Ac6),
            _ => None,
        }
    }

    /// Check if this engine uses kill counters (vs event flags)
    pub fn uses_kill_counters(&self) -> bool {
        matches!(self, Self::Ds2Sotfs)
    }
}

/// Scanned pattern result
#[derive(Debug, Clone)]
pub struct ScannedPattern {
    pub name: String,
    pub address: usize,
}

/// Generic game instance that uses data-driven configuration
#[cfg(target_os = "windows")]
pub struct GenericGame {
    pub handle: HANDLE,
    pub game_data: GameData,
    pub engine_type: EngineType,
    /// Resolved pattern addresses
    pub patterns: HashMap<String, usize>,
    /// Resolved pointers
    pub pointers: HashMap<String, Pointer>,
}

#[cfg(target_os = "windows")]
impl GenericGame {
    /// Create a new generic game instance
    pub fn new(game_data: GameData) -> Result<Self, String> {
        let engine_type = EngineType::from_str(&game_data.autosplitter.engine)
            .ok_or_else(|| format!("Unknown engine type: {}", game_data.autosplitter.engine))?;

        Ok(Self {
            handle: HANDLE::default(),
            game_data,
            engine_type,
            patterns: HashMap::new(),
            pointers: HashMap::new(),
        })
    }

    /// Initialize by scanning for patterns in memory
    pub fn init(&mut self, handle: HANDLE, base: usize, size: usize) -> bool {
        self.handle = handle;
        self.patterns.clear();
        self.pointers.clear();

        log::info!(
            "{}: Scanning for patterns (engine: {:?})",
            self.game_data.game.id,
            self.engine_type
        );

        // Scan for all patterns
        for pattern_def in &self.game_data.autosplitter.patterns {
            if let Some(addr) = self.scan_pattern(handle, base, size, pattern_def) {
                log::info!("  Found {}: 0x{:X}", pattern_def.name, addr);
                self.patterns.insert(pattern_def.name.clone(), addr);
            } else {
                log::warn!("  Pattern not found: {}", pattern_def.name);
            }
        }

        // Build pointers from pattern results
        for (name, pointer_def) in &self.game_data.autosplitter.pointers {
            if let Some(pointer) = self.build_pointer(pointer_def) {
                log::debug!("  Built pointer {}: {:?}", name, pointer.offsets);
                self.pointers.insert(name.clone(), pointer);
            }
        }

        // Check if we have the minimum required patterns
        self.validate_patterns()
    }

    /// Scan for a single pattern
    fn scan_pattern(
        &self,
        handle: HANDLE,
        base: usize,
        size: usize,
        pattern_def: &PatternDefinition,
    ) -> Option<usize> {
        let pattern = parse_pattern(&pattern_def.pattern);
        let found = scan_pattern(handle, base, size, &pattern)?;

        // Apply resolution
        let resolved = match pattern_def.resolve.as_str() {
            "rip_relative" => {
                // RIP-relative addressing: the pattern contains a 4-byte offset
                // that is relative to the end of the instruction
                let offset_pos = pattern_def.rip_offset as usize;
                // Standard x64 RIP-relative: offset is 4 bytes, instruction ends after it
                let instruction_len = offset_pos + 4;
                resolve_rip_relative(handle, found, offset_pos, instruction_len)?
            }
            "absolute" => {
                // Read absolute address from pattern location
                read_i64(handle, found + pattern_def.rip_offset as usize)? as usize
            }
            _ => found,
        };

        // Apply extra offset
        Some((resolved as i64 + pattern_def.extra_offset) as usize)
    }

    /// Build a pointer from a definition
    fn build_pointer(&self, pointer_def: &PointerDefinition) -> Option<Pointer> {
        let base_addr = *self.patterns.get(&pointer_def.pattern)?;

        let mut pointer = Pointer::new();
        pointer.base_address = base_addr as i64;
        pointer.offsets = pointer_def.offsets.clone();
        pointer.handle = self.handle;

        Some(pointer)
    }

    /// Validate that required patterns were found
    fn validate_patterns(&self) -> bool {
        match self.engine_type {
            EngineType::Ds2Sotfs => {
                // DS2 needs boss_counters pointer
                self.pointers.contains_key("boss_counters")
            }
            _ => {
                // Event flag games need event_flags pointer
                self.pointers.contains_key("event_flags")
            }
        }
    }

    /// Read an event flag or kill counter
    pub fn read_event_flag(&self, flag_id: u32) -> bool {
        match self.engine_type {
            EngineType::Ds2Sotfs => self.read_kill_counter(flag_id) > 0,
            EngineType::Ds3 => self.read_ds3_event_flag(flag_id),
            EngineType::EldenRing => self.read_elden_ring_event_flag(flag_id),
            EngineType::Sekiro => self.read_sekiro_event_flag(flag_id),
            EngineType::Ds1Remaster => self.read_ds1r_event_flag(flag_id),
            EngineType::Ds1Ptde => self.read_ds1_ptde_event_flag(flag_id),
            EngineType::Ac6 => self.read_ac6_event_flag(flag_id),
        }
    }

    /// Get raw kill count (for DS2)
    pub fn get_kill_count(&self, flag_id: u32) -> u32 {
        if self.engine_type == EngineType::Ds2Sotfs {
            self.read_kill_counter(flag_id) as u32
        } else {
            if self.read_event_flag(flag_id) { 1 } else { 0 }
        }
    }

    // =========================================================================
    // Engine-specific implementations
    // =========================================================================

    /// DS2: Read kill counter at offset
    fn read_kill_counter(&self, offset: u32) -> i32 {
        let boss_counters = match self.pointers.get("boss_counters") {
            Some(p) => p,
            None => return 0,
        };

        let addr = boss_counters.get_address();
        if addr == 0 {
            return 0;
        }

        read_i32(self.handle, (addr as usize) + (offset as usize)).unwrap_or(0)
    }

    /// DS3: Read event flag using area-based lookup
    fn read_ds3_event_flag(&self, event_flag_id: u32) -> bool {
        // Simplified DS3 event flag reading
        // Full implementation would match dark_souls_3.rs
        let event_flags = match self.pointers.get("event_flags") {
            Some(p) => p,
            None => return false,
        };

        let addr = event_flags.get_address();
        if addr == 0 {
            return false;
        }

        // DS3 event flag algorithm (simplified)
        // The full algorithm is in games/dark_souls_3.rs
        self.read_generic_event_flag(addr, event_flag_id)
    }

    /// Elden Ring: Read virtual memory flag
    fn read_elden_ring_event_flag(&self, event_flag_id: u32) -> bool {
        let event_flags = match self.pointers.get("event_flags") {
            Some(p) => p,
            None => return false,
        };

        let addr = event_flags.get_address();
        if addr == 0 {
            return false;
        }

        // Elden Ring uses virtual memory flags with a divisor
        // Full implementation in games/elden_ring.rs
        self.read_generic_event_flag(addr, event_flag_id)
    }

    /// Sekiro: Read event flag
    fn read_sekiro_event_flag(&self, event_flag_id: u32) -> bool {
        let event_flags = match self.pointers.get("event_flags") {
            Some(p) => p,
            None => return false,
        };

        let addr = event_flags.get_address();
        if addr == 0 {
            return false;
        }

        self.read_generic_event_flag(addr, event_flag_id)
    }

    /// DS1 Remastered: Read event flag
    fn read_ds1r_event_flag(&self, event_flag_id: u32) -> bool {
        let event_flags = match self.pointers.get("event_flags") {
            Some(p) => p,
            None => return false,
        };

        let addr = event_flags.get_address();
        if addr == 0 {
            return false;
        }

        self.read_generic_event_flag(addr, event_flag_id)
    }

    /// DS1 PTDE: Read event flag
    fn read_ds1_ptde_event_flag(&self, event_flag_id: u32) -> bool {
        let event_flags = match self.pointers.get("event_flags") {
            Some(p) => p,
            None => return false,
        };

        let addr = event_flags.get_address();
        if addr == 0 {
            return false;
        }

        self.read_generic_event_flag(addr, event_flag_id)
    }

    /// AC6: Read event flag
    fn read_ac6_event_flag(&self, event_flag_id: u32) -> bool {
        let event_flags = match self.pointers.get("event_flags") {
            Some(p) => p,
            None => return false,
        };

        let addr = event_flags.get_address();
        if addr == 0 {
            return false;
        }

        self.read_generic_event_flag(addr, event_flag_id)
    }

    /// Generic event flag reading (fallback)
    /// Note: This is a simplified version. Full implementations are in games/*.rs
    fn read_generic_event_flag(&self, _base_addr: i64, _event_flag_id: u32) -> bool {
        // TODO: Implement based on engine type
        // For now, delegate to the existing game implementations
        false
    }
}

#[cfg(target_os = "linux")]
pub struct GenericGame {
    pub pid: i32,
    pub game_data: GameData,
    pub engine_type: EngineType,
    pub patterns: HashMap<String, usize>,
    pub pointers: HashMap<String, Pointer>,
}

#[cfg(target_os = "linux")]
impl GenericGame {
    pub fn new(game_data: GameData) -> Result<Self, String> {
        let engine_type = EngineType::from_str(&game_data.autosplitter.engine)
            .ok_or_else(|| format!("Unknown engine type: {}", game_data.autosplitter.engine))?;

        Ok(Self {
            pid: 0,
            game_data,
            engine_type,
            patterns: HashMap::new(),
            pointers: HashMap::new(),
        })
    }

    pub fn init(&mut self, pid: i32, base: usize, size: usize) -> bool {
        self.pid = pid;
        // Linux implementation would go here
        false
    }

    pub fn read_event_flag(&self, _flag_id: u32) -> bool {
        false
    }

    pub fn get_kill_count(&self, _flag_id: u32) -> u32 {
        0
    }
}
