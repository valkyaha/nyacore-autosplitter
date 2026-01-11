//! Generic game engine for data-driven autosplitter
//!
//! This module provides a unified interface for reading game state
//! using configurations loaded from TOML files.
//!
//! The algorithms are implemented in Rust (too complex for config),
//! but the memory patterns and pointers come from TOML config.

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
        for (name, pointer_def) in &self.game_data.autosplitter.pointers.clone() {
            if let Some(pointer) = self.build_pointer(pointer_def) {
                log::debug!("  Built pointer {}: base=0x{:X}", name, pointer.base_address);
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
                let offset_pos = pattern_def.rip_offset as usize;
                let instruction_len = offset_pos + 4;
                resolve_rip_relative(handle, found, offset_pos, instruction_len)?
            }
            "absolute" => {
                read_i64(handle, found + pattern_def.rip_offset as usize)? as usize
            }
            _ => found,
        };

        Some((resolved as i64 + pattern_def.extra_offset) as usize)
    }

    /// Build a pointer from a definition
    fn build_pointer(&self, pointer_def: &PointerDefinition) -> Option<Pointer> {
        let base_addr = *self.patterns.get(&pointer_def.pattern)?;

        let mut pointer = Pointer::new();
        pointer.initialize(
            self.handle,
            true,
            base_addr as i64,
            &pointer_def.offsets,
        );

        Some(pointer)
    }

    /// Validate that required patterns were found
    fn validate_patterns(&self) -> bool {
        match self.engine_type {
            EngineType::Ds2Sotfs => {
                self.pointers.contains_key("boss_counters")
            }
            EngineType::Ds3 => {
                self.pointers.contains_key("event_flags")
                    && self.pointers.contains_key("field_area")
            }
            EngineType::EldenRing => {
                self.pointers.contains_key("event_flags")
            }
            _ => {
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
            self.read_kill_counter(flag_id).max(0) as u32
        } else {
            if self.read_event_flag(flag_id) { 1 } else { 0 }
        }
    }

    // =========================================================================
    // DS2 SOTFS - Kill Counter System
    // =========================================================================

    fn read_kill_counter(&self, offset: u32) -> i32 {
        let boss_counters = match self.pointers.get("boss_counters") {
            Some(p) => p,
            None => return 0,
        };

        boss_counters.read_i32(Some(offset as i64))
    }

    // =========================================================================
    // DS3 - Area-based Event Flags (port from SoulSplitter)
    // =========================================================================

    fn read_ds3_event_flag(&self, event_flag_id: u32) -> bool {
        let event_flags = match self.pointers.get("event_flags") {
            Some(p) => p,
            None => return false,
        };

        let field_area = match self.pointers.get("field_area") {
            Some(p) => p,
            None => return false,
        };

        // Decompose event flag ID
        let event_flag_id_div_10000000 = ((event_flag_id / 10_000_000) % 10) as i64;
        let event_flag_area = ((event_flag_id / 100_000) % 100) as i32;
        let event_flag_id_div_10000 = ((event_flag_id / 10_000) % 10) as i32;
        let event_flag_id_div_1000 = ((event_flag_id / 1_000) % 10) as i64;

        let mut flag_world_block_info_category: i32 = -1;

        if event_flag_area >= 90 || event_flag_area + event_flag_id_div_10000 == 0 {
            flag_world_block_info_category = 0;
        } else {
            if field_area.is_null_ptr() {
                return false;
            }

            let world_info_owner = field_area.append(&[0x0, 0x10]).create_pointer_from_address(None);
            let size = world_info_owner.read_i32(Some(0x8));
            let vector = world_info_owner.append(&[0x10]);

            for i in 0..size {
                let area = vector.read_byte(Some((i as i64 * 0x38) + 0xb)) as i32;

                if area == event_flag_area {
                    let count = vector.read_byte(Some(i as i64 * 0x38 + 0x20));
                    let mut index = 0i64;
                    let mut found = false;
                    let mut world_info_block_vector: Option<Pointer> = None;

                    if count >= 1 {
                        loop {
                            let block_vec = vector.create_pointer_from_address(Some(i as i64 * 0x38 + 0x28));
                            let flag = block_vec.read_i32(Some((index * 0x70) + 0x8));

                            if ((flag >> 0x10) & 0xff) == event_flag_id_div_10000
                                && (flag >> 0x18) == event_flag_area
                            {
                                found = true;
                                world_info_block_vector = Some(block_vec);
                                break;
                            }

                            index += 1;
                            if count as i64 <= index {
                                break;
                            }
                        }
                    }

                    if found {
                        if let Some(ref block_vec) = world_info_block_vector {
                            flag_world_block_info_category = block_vec.read_i32(Some((index * 0x70) + 0x20));
                            break;
                        }
                    }
                }
            }

            if flag_world_block_info_category >= 0 {
                flag_world_block_info_category += 1;
            }
        }

        let ptr = event_flags.append(&[0x218, event_flag_id_div_10000000 * 0x18, 0x0]);

        if ptr.is_null_ptr() || flag_world_block_info_category < 0 {
            return false;
        }

        let result_base = (event_flag_id_div_1000 << 4)
            + ptr.get_address()
            + (flag_world_block_info_category as i64 * 0xa8);

        let mut result_pointer = Pointer::new();
        result_pointer.initialize(self.handle, true, result_base, &[0x0]);

        if !result_pointer.is_null_ptr() {
            let mod_1000 = (event_flag_id % 1000) as u32;
            let read_offset = ((mod_1000 >> 5) * 4) as i64;
            let value = result_pointer.read_u32(Some(read_offset));

            let bit_shift = 0x1f - ((mod_1000 as u8) & 0x1f);
            let mask = 1u32 << (bit_shift & 0x1f);

            return (value & mask) != 0;
        }

        false
    }

    // =========================================================================
    // Elden Ring - Virtual Memory Flags (port from SoulSplitter)
    // =========================================================================

    fn read_elden_ring_event_flag(&self, event_flag_id: u32) -> bool {
        let event_flags = match self.pointers.get("event_flags") {
            Some(p) => p,
            None => return false,
        };

        // Read divisor from virtual_memory_flag + 0x1c
        let divisor = event_flags.read_i32(Some(0x1c));
        if divisor == 0 {
            return false;
        }

        let category = event_flag_id / divisor as u32;
        let least_significant_digits = event_flag_id - (category * divisor as u32);

        // Binary tree traversal
        let current_element_root = event_flags.create_pointer_from_address(Some(0x38));
        let mut current_element = current_element_root.clone();
        let mut current_sub_element = current_element.create_pointer_from_address(Some(0x8));

        while current_sub_element.read_byte(Some(0x19)) == 0 {
            if (current_sub_element.read_i32(Some(0x20)) as u32) < category {
                current_sub_element = current_sub_element.create_pointer_from_address(Some(0x10));
            } else {
                current_element = current_sub_element.clone();
                current_sub_element = current_sub_element.create_pointer_from_address(Some(0x0));
            }
        }

        let current_elem_addr = current_element.get_address();
        let sub_elem_addr = current_sub_element.get_address();

        if current_elem_addr == sub_elem_addr || category < (current_element.read_i32(Some(0x20)) as u32) {
            current_element = current_sub_element.clone();
        }

        let current_elem_addr = current_element.get_address();
        let sub_elem_addr = current_sub_element.get_address();

        if current_elem_addr == sub_elem_addr {
            return false;
        }

        let mystery_value = read_i32(self.handle, (current_elem_addr + 0x28) as usize).unwrap_or(0) - 1;

        let calculated_pointer: i64;
        if mystery_value == 0 {
            let mult = event_flags.read_i32(Some(0x20));
            let elem_val = read_i32(self.handle, (current_elem_addr + 0x30) as usize).unwrap_or(0);
            let base_addr = event_flags.read_i64(Some(0x28));
            calculated_pointer = (mult as i64 * elem_val as i64) + base_addr;
        } else if mystery_value == 1 {
            return false;
        } else {
            calculated_pointer = read_i64(self.handle, (current_elem_addr + 0x30) as usize).unwrap_or(0);
        }

        if calculated_pointer == 0 {
            return false;
        }

        let thing = 7 - (least_significant_digits & 7);
        let mask = 1i32 << thing;
        let shifted = least_significant_digits >> 3;

        let final_addr = (calculated_pointer + shifted as i64) as usize;
        if let Some(read_value) = read_i32(self.handle, final_addr) {
            return (read_value & mask) != 0;
        }

        false
    }

    // =========================================================================
    // Sekiro - Event Flags (similar to DS3 but simpler)
    // =========================================================================

    fn read_sekiro_event_flag(&self, event_flag_id: u32) -> bool {
        let event_flags = match self.pointers.get("event_flags") {
            Some(p) => p,
            None => return false,
        };

        // Sekiro uses a simpler system similar to DS3 category 0
        let divisor = 1000u32;
        let id_div_by_divisor = event_flag_id / divisor;
        let category = id_div_by_divisor / 100000;
        let sub_category = (id_div_by_divisor % 100000) / 10000;
        let byte_index = id_div_by_divisor % 10000;

        // Navigate to the flag location
        let ptr = event_flags.append(&[
            0x28,                           // Base offset
            (category * 8) as i64,          // Category offset
            0x0,                            // Dereference
            (sub_category * 0x90) as i64,   // Sub-category offset
            0x80,                           // Fixed offset
            (byte_index * 8) as i64,        // Byte index offset
        ]);

        if ptr.is_null_ptr() {
            return false;
        }

        let mod_1000 = event_flag_id % 1000;
        let byte_offset = (mod_1000 / 8) as i64;
        let bit_index = mod_1000 % 8;

        let byte_val = ptr.read_byte(Some(byte_offset));
        let mask = 1u8 << bit_index;

        (byte_val & mask) != 0
    }

    // =========================================================================
    // DS1 Remastered - Event Flags
    // =========================================================================

    fn read_ds1r_event_flag(&self, event_flag_id: u32) -> bool {
        let event_flags = match self.pointers.get("event_flags") {
            Some(p) => p,
            None => return false,
        };

        // DS1R event flag calculation
        let id_div_100000 = (event_flag_id / 100000) as i64;
        let id_mod_100000 = event_flag_id % 100000;
        let _id_div_100000_mod_10 = id_div_100000 % 10;

        let offset_base = match id_div_100000 {
            0 => 0x0,
            1 => 0x500,
            5 => 0x5F00,
            6 => 0x6900,
            7 => 0x7300,
            _ => {
                // Calculate based on area
                let area_offset = if id_div_100000 < 50 {
                    (id_div_100000 - 10) * 0x500 + 0xA00
                } else {
                    (id_div_100000 - 50) * 0x100 + 0x7D00
                };
                area_offset
            }
        };

        let id_div_10000_mod_10 = (id_mod_100000 / 10000) % 10;
        let sub_offset = (id_div_10000_mod_10 as i64) * 0x80;

        let final_offset = offset_base + sub_offset + ((id_mod_100000 % 10000) / 32) as i64 * 4;

        let ptr = event_flags.append(&[final_offset]);
        if ptr.is_null_ptr() {
            return false;
        }

        let value = ptr.read_u32(None);
        let bit = (id_mod_100000 % 32) as u32;
        let mask = 1u32 << bit;

        (value & mask) != 0
    }

    // =========================================================================
    // DS1 PTDE - Event Flags (32-bit)
    // =========================================================================

    fn read_ds1_ptde_event_flag(&self, event_flag_id: u32) -> bool {
        // PTDE uses same algorithm as Remastered but 32-bit pointers
        self.read_ds1r_event_flag(event_flag_id)
    }

    // =========================================================================
    // AC6 - Event Flags (similar to Elden Ring)
    // =========================================================================

    fn read_ac6_event_flag(&self, event_flag_id: u32) -> bool {
        // AC6 uses the same virtual memory flag system as Elden Ring
        self.read_elden_ring_event_flag(event_flag_id)
    }
}

// =========================================================================
// Linux Implementation (stub)
// =========================================================================

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

    pub fn init(&mut self, pid: i32, _base: usize, _size: usize) -> bool {
        self.pid = pid;
        false
    }

    pub fn read_event_flag(&self, _flag_id: u32) -> bool {
        false
    }

    pub fn get_kill_count(&self, _flag_id: u32) -> u32 {
        0
    }
}
