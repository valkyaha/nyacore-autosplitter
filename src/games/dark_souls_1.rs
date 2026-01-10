//! Dark Souls Remastered autosplitter - port of SoulSplitter's Remastered.cs
//! https://github.com/FrankvdStam/SoulSplitter
//!
//! Credit to JKAnderson for the original event flag reading code (DSR-Gadget)

#[cfg(target_os = "windows")]
use std::collections::HashMap;
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HANDLE;

#[cfg(target_os = "windows")]
use crate::memory::{parse_pattern, resolve_rip_relative, scan_pattern, read_u32, read_i32, read_f32};
#[cfg(target_os = "windows")]
use crate::memory::pointer::Pointer;

// Memory patterns from SoulSplitter
#[cfg(target_os = "windows")]
pub const EVENT_FLAGS_PATTERN: &str = "48 8B 0D ? ? ? ? 99 33 C2 45 33 C0 2B C2 8D 50 F6";
#[cfg(target_os = "windows")]
pub const GAME_DATA_MAN_PATTERN: &str = "48 8b 05 ? ? ? ? 48 8b 50 10 48 89 54 24 60";
#[cfg(target_os = "windows")]
pub const GAME_MAN_PATTERN: &str = "48 8b 05 ? ? ? ? c6 40 18 00";
#[cfg(target_os = "windows")]
pub const WORLD_CHR_MAN_PATTERN: &str = "48 8b 0d ? ? ? ? 0f 28 f1 48 85 c9 74 ? 48 89 7c";
#[cfg(target_os = "windows")]
pub const MENU_MAN_PATTERN: &str = "48 8b 15 ? ? ? ? 89 82 7c 08 00 00";
#[cfg(target_os = "windows")]
pub const BONFIRE_DB_PATTERN: &str = "48 83 3d ? ? ? ? 00 48 8b f1";

/// Player position as 3D vector
#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Copy, Default)]
pub struct Vector3f {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// Character attributes
#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum Attribute {
    Vitality = 0x0,
    Attunement = 0x4,
    Endurance = 0x8,
    Strength = 0xc,
    Dexterity = 0x10,
    Resistance = 0x14,
    Intelligence = 0x18,
    Faith = 0x1c,
    Humanity = 0x24,
    SoulLevel = 0x28,
}

/// Bonfire states
#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BonfireState {
    Unknown = 0,
    Discovered = 1,
    Unlocked = 2,
    Kindled1 = 3,  // 10 estus
    Kindled2 = 4,  // 15 estus
    Kindled3 = 5,  // 20 estus
}

/// Dark Souls Remastered autosplitter state
#[cfg(target_os = "windows")]
pub struct DarkSouls1 {
    pub handle: HANDLE,
    // Core pointers
    pub event_flags: Pointer,
    pub game_data_man: Pointer,
    pub game_man: Pointer,
    pub world_chr_man: Pointer,
    pub menu_man: Pointer,
    pub bonfire_db: Pointer,
    // Derived pointers
    pub player_game_data: Pointer,
    pub player_ins: Pointer,
    pub player_pos: Pointer,
    // Offset maps
    event_flag_groups: HashMap<char, i32>,
    event_flag_areas: HashMap<&'static str, i32>,
    // Version-specific offsets
    player_ctrl_offset: i64,
    current_save_slot_offset: i64,
}

#[cfg(target_os = "windows")]
impl DarkSouls1 {
    pub fn new() -> Self {
        let mut event_flag_groups = HashMap::new();
        event_flag_groups.insert('0', 0x00000);
        event_flag_groups.insert('1', 0x00500);
        event_flag_groups.insert('5', 0x05F00);
        event_flag_groups.insert('6', 0x0B900);
        event_flag_groups.insert('7', 0x11300);

        let mut event_flag_areas = HashMap::new();
        event_flag_areas.insert("000", 0);
        event_flag_areas.insert("100", 1);
        event_flag_areas.insert("101", 2);
        event_flag_areas.insert("102", 3);
        event_flag_areas.insert("110", 4);
        event_flag_areas.insert("120", 5);
        event_flag_areas.insert("121", 6);
        event_flag_areas.insert("130", 7);
        event_flag_areas.insert("131", 8);
        event_flag_areas.insert("132", 9);
        event_flag_areas.insert("140", 10);
        event_flag_areas.insert("141", 11);
        event_flag_areas.insert("150", 12);
        event_flag_areas.insert("151", 13);
        event_flag_areas.insert("160", 14);
        event_flag_areas.insert("170", 15);
        event_flag_areas.insert("180", 16);
        event_flag_areas.insert("181", 17);
        event_flag_areas.insert("200", 18);  // DLC
        event_flag_areas.insert("210", 19);  // DLC

        Self {
            handle: HANDLE::default(),
            event_flags: Pointer::new(),
            game_data_man: Pointer::new(),
            game_man: Pointer::new(),
            world_chr_man: Pointer::new(),
            menu_man: Pointer::new(),
            bonfire_db: Pointer::new(),
            player_game_data: Pointer::new(),
            player_ins: Pointer::new(),
            player_pos: Pointer::new(),
            event_flag_groups,
            event_flag_areas,
            player_ctrl_offset: 0x68,      // Default, 0x48 for v1.0.1
            current_save_slot_offset: 0xaa0, // Default, 0xa90 for v1.0.1
        }
    }

    /// Initialize pointers by scanning for patterns
    pub fn init_pointers(&mut self, handle: HANDLE, base: usize, size: usize) -> bool {
        self.handle = handle;
        log::info!("DS1R: Initializing pointers, base=0x{:X}, size=0x{:X}", base, size);

        // Scan for EventFlags
        let pattern = parse_pattern(EVENT_FLAGS_PATTERN);
        log::debug!("DS1R: Scanning for EventFlags pattern: {}", EVENT_FLAGS_PATTERN);

        let event_flags_addr = match scan_pattern(handle, base, size, &pattern) {
            Some(found) => {
                log::debug!("DS1R: EventFlags pattern found at 0x{:X}", found);
                match resolve_rip_relative(handle, found, 3, 7) {
                    Some(addr) => {
                        log::debug!("DS1R: RIP-relative resolved to 0x{:X}", addr);
                        addr
                    },
                    None => {
                        log::warn!("DS1R: Failed to resolve EventFlags RIP-relative address");
                        return false;
                    }
                }
            }
            None => {
                log::warn!("DS1R: EventFlags pattern not found");
                return false;
            }
        };
        // DSProcess does TWO dereferences: *(*eventFlagPtr + 0) + 0
        // With our Pointer class, we need 3 offsets (last one doesn't deref)
        self.event_flags.initialize(handle, true, event_flags_addr as i64, &[0x0, 0x0, 0x0]);

        // Immediately test the pointer resolution
        let resolved_addr = self.event_flags.get_address();
        log::info!("DS1R: EventFlags pointer at 0x{:X}, resolves to 0x{:X}", event_flags_addr, resolved_addr);

        if resolved_addr == 0 {
            log::warn!("DS1R: EventFlags pointer resolves to NULL - game may still be loading");
        }

        // Scan for GameDataMan
        let pattern = parse_pattern(GAME_DATA_MAN_PATTERN);
        if let Some(found) = scan_pattern(handle, base, size, &pattern) {
            if let Some(addr) = resolve_rip_relative(handle, found, 3, 7) {
                self.game_data_man.initialize(handle, true, addr as i64, &[0x0]);
                // PlayerGameData is at GameDataMan + 0x10
                self.player_game_data.initialize(handle, true, addr as i64, &[0x0, 0x10]);
                log::info!("DS1R: GameDataMan at 0x{:X}", addr);
            }
        }

        // Scan for GameMan
        let pattern = parse_pattern(GAME_MAN_PATTERN);
        if let Some(found) = scan_pattern(handle, base, size, &pattern) {
            if let Some(addr) = resolve_rip_relative(handle, found, 3, 7) {
                self.game_man.initialize(handle, true, addr as i64, &[0x0]);
                log::info!("DS1R: GameMan at 0x{:X}", addr);
            }
        }

        // Scan for WorldChrMan (player instance)
        let pattern = parse_pattern(WORLD_CHR_MAN_PATTERN);
        if let Some(found) = scan_pattern(handle, base, size, &pattern) {
            if let Some(addr) = resolve_rip_relative(handle, found, 3, 7) {
                self.world_chr_man.initialize(handle, true, addr as i64, &[0x0]);
                // PlayerIns at WorldChrMan + 0x68
                self.player_ins.initialize(handle, true, addr as i64, &[0x0, self.player_ctrl_offset]);
                // PlayerPos at PlayerIns + 0x28
                self.player_pos.initialize(handle, true, addr as i64, &[0x0, self.player_ctrl_offset, 0x28]);
                log::info!("DS1R: WorldChrMan at 0x{:X}", addr);
            }
        }

        // Scan for MenuMan
        let pattern = parse_pattern(MENU_MAN_PATTERN);
        if let Some(found) = scan_pattern(handle, base, size, &pattern) {
            if let Some(addr) = resolve_rip_relative(handle, found, 3, 7) {
                self.menu_man.initialize(handle, true, addr as i64, &[0x0]);
                log::info!("DS1R: MenuMan at 0x{:X}", addr);
            }
        }

        // Scan for BonfireDb
        let pattern = parse_pattern(BONFIRE_DB_PATTERN);
        if let Some(found) = scan_pattern(handle, base, size, &pattern) {
            if let Some(addr) = resolve_rip_relative(handle, found, 3, 8) {
                self.bonfire_db.initialize(handle, true, addr as i64, &[0x0]);
                log::info!("DS1R: BonfireDb at 0x{:X}", addr);
            }
        }

        true
    }

    /// Get the offset and mask for an event flag
    fn get_event_flag_offset(&self, event_flag_id: u32) -> Option<(i32, u32)> {
        let id_string = format!("{:08}", event_flag_id);
        if id_string.len() != 8 {
            return None;
        }

        let group = id_string.chars().next()?;
        let area = &id_string[1..4];
        let section: i32 = id_string[4..5].parse().ok()?;
        let number: i32 = id_string[5..8].parse().ok()?;

        let group_offset = self.event_flag_groups.get(&group)?;
        let area_offset = self.event_flag_areas.get(area)?;

        let mut offset = *group_offset;
        offset += area_offset * 0x500;
        offset += section * 128;
        offset += (number - (number % 32)) / 8;

        let mask = 0x80000000u32 >> (number % 32);
        Some((offset, mask))
    }

    /// Read event flag - port of SoulSplitter's ReadEventFlag
    pub fn read_event_flag(&self, event_flag_id: u32) -> bool {
        if let Some((offset, mask)) = self.get_event_flag_offset(event_flag_id) {
            let address = self.event_flags.get_address();
            if address == 0 {
                // Log at debug level periodically to help diagnose null pointer issues
                static LAST_NULL_LOG: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let last = LAST_NULL_LOG.load(std::sync::atomic::Ordering::Relaxed);
                if now > last + 5 {
                    LAST_NULL_LOG.store(now, std::sync::atomic::Ordering::Relaxed);
                    log::warn!("DS1R: EventFlags pointer is NULL - save data may not be loaded yet");
                }
                return false;
            }

            let read_addr = (address + offset as i64) as usize;
            if let Some(value) = read_u32(self.handle, read_addr) {
                let result = (value & mask) != 0;
                if result {
                    log::info!("DS1R: Flag {} is SET (base=0x{:X}, offset=0x{:X}, addr=0x{:X}, mask=0x{:X}, value=0x{:X})",
                        event_flag_id, address, offset, read_addr, mask, value);
                }
                return result;
            } else {
                // Memory read failed - log at debug level to help diagnose
                static LAST_READ_FAIL_LOG: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let last = LAST_READ_FAIL_LOG.load(std::sync::atomic::Ordering::Relaxed);
                if now > last + 5 {
                    LAST_READ_FAIL_LOG.store(now, std::sync::atomic::Ordering::Relaxed);
                    log::warn!("DS1R: Failed to read memory at 0x{:X} for flag {} (base=0x{:X}, offset=0x{:X})",
                        read_addr, event_flag_id, address, offset);
                }
            }
        } else {
            log::warn!("DS1R: Could not calculate offset for flag {} (invalid format)", event_flag_id);
        }
        false
    }

    /// Get in-game time in milliseconds
    pub fn get_in_game_time_milliseconds(&self) -> i32 {
        let addr = self.game_data_man.get_address();
        if addr == 0 {
            return 0;
        }
        read_i32(self.handle, (addr + 0xa4) as usize).unwrap_or(0)
    }

    /// Check if player is loaded
    pub fn is_player_loaded(&self) -> bool {
        !self.player_ins.is_null_ptr()
    }

    /// Get player position
    pub fn get_position(&self) -> Vector3f {
        let addr = self.player_pos.get_address();
        if addr == 0 {
            return Vector3f::default();
        }
        Vector3f {
            x: read_f32(self.handle, (addr + 0x10) as usize).unwrap_or(0.0),
            y: read_f32(self.handle, (addr + 0x14) as usize).unwrap_or(0.0),
            z: read_f32(self.handle, (addr + 0x18) as usize).unwrap_or(0.0),
        }
    }

    /// Get character attribute value
    pub fn get_attribute(&self, attribute: Attribute) -> i32 {
        let addr = self.player_game_data.get_address();
        if addr == 0 {
            return -1;
        }
        read_i32(self.handle, (addr + 0x8 + attribute as i64) as usize).unwrap_or(-1)
    }

    /// Get NG+ count
    pub fn ng_count(&self) -> i32 {
        let addr = self.game_data_man.get_address();
        if addr == 0 {
            return 0;
        }
        read_i32(self.handle, (addr + 0x78) as usize).unwrap_or(0)
    }

    /// Check if warp is requested (for quitout detection)
    pub fn is_warp_requested(&self) -> bool {
        let game_man_addr = self.game_man.get_address();
        if game_man_addr == 0 {
            return false;
        }

        // Check GameMan + 0x19 == 1
        let warp_flag = read_u32(self.handle, (game_man_addr + 0x19) as usize).unwrap_or(0) as u8;
        warp_flag == 1
    }

    /// Check if credits are rolling
    pub fn are_credits_rolling(&self) -> bool {
        let addr = self.menu_man.get_address();
        if addr == 0 {
            return false;
        }

        // MenuMan+0x80==0 && MenuMan+0xc8==1 && MenuMan+0xd4==1
        let val_80 = read_i32(self.handle, (addr + 0x80) as usize).unwrap_or(-1);
        let val_c8 = read_i32(self.handle, (addr + 0xc8) as usize).unwrap_or(0);
        let val_d4 = read_i32(self.handle, (addr + 0xd4) as usize).unwrap_or(0);

        val_80 == 0 && val_c8 == 1 && val_d4 == 1
    }

    /// Get current save slot
    pub fn get_current_save_slot(&self) -> i32 {
        let addr = self.game_data_man.get_address();
        if addr == 0 {
            return -1;
        }
        read_i32(self.handle, (addr + self.current_save_slot_offset) as usize).unwrap_or(-1)
    }

    /// Get player health
    pub fn get_player_health(&self) -> i32 {
        let addr = self.player_ins.get_address();
        if addr == 0 {
            return 0;
        }
        read_i32(self.handle, (addr + 0x3e8) as usize).unwrap_or(0)
    }
}

#[cfg(target_os = "windows")]
impl Default for DarkSouls1 {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Linux Implementation (for Proton/Wine)
// =============================================================================

#[cfg(target_os = "linux")]
use std::collections::HashMap;

#[cfg(target_os = "linux")]
use crate::memory::{parse_pattern, resolve_rip_relative, scan_pattern, read_u32, read_i32, read_f32};
#[cfg(target_os = "linux")]
use crate::memory::pointer::Pointer;

// Memory patterns (same as Windows - reading same executable in Wine)
#[cfg(target_os = "linux")]
pub const EVENT_FLAGS_PATTERN: &str = "48 8B 0D ? ? ? ? 99 33 C2 45 33 C0 2B C2 8D 50 F6";
#[cfg(target_os = "linux")]
pub const GAME_DATA_MAN_PATTERN: &str = "48 8b 05 ? ? ? ? 48 8b 50 10 48 89 54 24 60";
#[cfg(target_os = "linux")]
pub const GAME_MAN_PATTERN: &str = "48 8b 05 ? ? ? ? c6 40 18 00";
#[cfg(target_os = "linux")]
pub const WORLD_CHR_MAN_PATTERN: &str = "48 8b 0d ? ? ? ? 0f 28 f1 48 85 c9 74 ? 48 89 7c";
#[cfg(target_os = "linux")]
pub const MENU_MAN_PATTERN: &str = "48 8b 15 ? ? ? ? 89 82 7c 08 00 00";
#[cfg(target_os = "linux")]
pub const BONFIRE_DB_PATTERN: &str = "48 83 3d ? ? ? ? 00 48 8b f1";

#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy, Default)]
pub struct Vector3f {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum Attribute {
    Vitality = 0x0,
    Attunement = 0x4,
    Endurance = 0x8,
    Strength = 0xc,
    Dexterity = 0x10,
    Resistance = 0x14,
    Intelligence = 0x18,
    Faith = 0x1c,
    Humanity = 0x24,
    SoulLevel = 0x28,
}

#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BonfireState {
    Unknown = 0,
    Discovered = 1,
    Unlocked = 2,
    Kindled1 = 3,
    Kindled2 = 4,
    Kindled3 = 5,
}

#[cfg(target_os = "linux")]
pub struct DarkSouls1 {
    pub pid: i32,
    // Core pointers
    pub event_flags: Pointer,
    pub game_data_man: Pointer,
    pub game_man: Pointer,
    pub world_chr_man: Pointer,
    pub menu_man: Pointer,
    pub bonfire_db: Pointer,
    // Derived pointers
    pub player_game_data: Pointer,
    pub player_ins: Pointer,
    pub player_pos: Pointer,
    // Offset maps
    event_flag_groups: HashMap<char, i32>,
    event_flag_areas: HashMap<&'static str, i32>,
    // Version-specific offsets
    player_ctrl_offset: i64,
    current_save_slot_offset: i64,
}

#[cfg(target_os = "linux")]
impl DarkSouls1 {
    pub fn new() -> Self {
        let mut event_flag_groups = HashMap::new();
        event_flag_groups.insert('0', 0x00000);
        event_flag_groups.insert('1', 0x00500);
        event_flag_groups.insert('5', 0x05F00);
        event_flag_groups.insert('6', 0x0B900);
        event_flag_groups.insert('7', 0x11300);

        let mut event_flag_areas = HashMap::new();
        event_flag_areas.insert("000", 0);
        event_flag_areas.insert("100", 1);
        event_flag_areas.insert("101", 2);
        event_flag_areas.insert("102", 3);
        event_flag_areas.insert("110", 4);
        event_flag_areas.insert("120", 5);
        event_flag_areas.insert("121", 6);
        event_flag_areas.insert("130", 7);
        event_flag_areas.insert("131", 8);
        event_flag_areas.insert("132", 9);
        event_flag_areas.insert("140", 10);
        event_flag_areas.insert("141", 11);
        event_flag_areas.insert("150", 12);
        event_flag_areas.insert("151", 13);
        event_flag_areas.insert("160", 14);
        event_flag_areas.insert("170", 15);
        event_flag_areas.insert("180", 16);
        event_flag_areas.insert("181", 17);
        event_flag_areas.insert("200", 18);
        event_flag_areas.insert("210", 19);

        Self {
            pid: 0,
            event_flags: Pointer::new(),
            game_data_man: Pointer::new(),
            game_man: Pointer::new(),
            world_chr_man: Pointer::new(),
            menu_man: Pointer::new(),
            bonfire_db: Pointer::new(),
            player_game_data: Pointer::new(),
            player_ins: Pointer::new(),
            player_pos: Pointer::new(),
            event_flag_groups,
            event_flag_areas,
            player_ctrl_offset: 0x68,
            current_save_slot_offset: 0xaa0,
        }
    }

    pub fn init_pointers(&mut self, pid: i32, base: usize, size: usize) -> bool {
        self.pid = pid;
        log::info!("DS1R: Initializing pointers (Linux), base=0x{:X}, size=0x{:X}", base, size);

        // Scan for EventFlags
        let pattern = parse_pattern(EVENT_FLAGS_PATTERN);
        let event_flags_addr = match scan_pattern(pid, base, size, &pattern) {
            Some(found) => {
                match resolve_rip_relative(pid, found, 3, 7) {
                    Some(addr) => addr,
                    None => {
                        log::warn!("DS1R: Failed to resolve EventFlags RIP-relative address");
                        return false;
                    }
                }
            }
            None => {
                log::warn!("DS1R: EventFlags pattern not found");
                return false;
            }
        };
        self.event_flags.initialize(pid, true, event_flags_addr as i64, &[0x0, 0x0, 0x0]);
        log::info!("DS1R: EventFlags at 0x{:X}", event_flags_addr);

        // Scan for GameDataMan
        let pattern = parse_pattern(GAME_DATA_MAN_PATTERN);
        if let Some(found) = scan_pattern(pid, base, size, &pattern) {
            if let Some(addr) = resolve_rip_relative(pid, found, 3, 7) {
                self.game_data_man.initialize(pid, true, addr as i64, &[0x0]);
                self.player_game_data.initialize(pid, true, addr as i64, &[0x0, 0x10]);
                log::info!("DS1R: GameDataMan at 0x{:X}", addr);
            }
        }

        // Scan for GameMan
        let pattern = parse_pattern(GAME_MAN_PATTERN);
        if let Some(found) = scan_pattern(pid, base, size, &pattern) {
            if let Some(addr) = resolve_rip_relative(pid, found, 3, 7) {
                self.game_man.initialize(pid, true, addr as i64, &[0x0]);
                log::info!("DS1R: GameMan at 0x{:X}", addr);
            }
        }

        // Scan for WorldChrMan
        let pattern = parse_pattern(WORLD_CHR_MAN_PATTERN);
        if let Some(found) = scan_pattern(pid, base, size, &pattern) {
            if let Some(addr) = resolve_rip_relative(pid, found, 3, 7) {
                self.world_chr_man.initialize(pid, true, addr as i64, &[0x0]);
                self.player_ins.initialize(pid, true, addr as i64, &[0x0, self.player_ctrl_offset]);
                self.player_pos.initialize(pid, true, addr as i64, &[0x0, self.player_ctrl_offset, 0x28]);
                log::info!("DS1R: WorldChrMan at 0x{:X}", addr);
            }
        }

        // Scan for MenuMan
        let pattern = parse_pattern(MENU_MAN_PATTERN);
        if let Some(found) = scan_pattern(pid, base, size, &pattern) {
            if let Some(addr) = resolve_rip_relative(pid, found, 3, 7) {
                self.menu_man.initialize(pid, true, addr as i64, &[0x0]);
                log::info!("DS1R: MenuMan at 0x{:X}", addr);
            }
        }

        // Scan for BonfireDb
        let pattern = parse_pattern(BONFIRE_DB_PATTERN);
        if let Some(found) = scan_pattern(pid, base, size, &pattern) {
            if let Some(addr) = resolve_rip_relative(pid, found, 3, 8) {
                self.bonfire_db.initialize(pid, true, addr as i64, &[0x0]);
                log::info!("DS1R: BonfireDb at 0x{:X}", addr);
            }
        }

        true
    }

    fn get_event_flag_offset(&self, event_flag_id: u32) -> Option<(i32, u32)> {
        let id_string = format!("{:08}", event_flag_id);
        if id_string.len() != 8 {
            return None;
        }

        let group = id_string.chars().next()?;
        let area = &id_string[1..4];
        let section: i32 = id_string[4..5].parse().ok()?;
        let number: i32 = id_string[5..8].parse().ok()?;

        let group_offset = self.event_flag_groups.get(&group)?;
        let area_offset = self.event_flag_areas.get(area)?;

        let mut offset = *group_offset;
        offset += area_offset * 0x500;
        offset += section * 128;
        offset += (number - (number % 32)) / 8;

        let mask = 0x80000000u32 >> (number % 32);
        Some((offset, mask))
    }

    pub fn read_event_flag(&self, event_flag_id: u32) -> bool {
        if let Some((offset, mask)) = self.get_event_flag_offset(event_flag_id) {
            let address = self.event_flags.get_address();
            if address == 0 {
                return false;
            }

            let read_addr = (address + offset as i64) as usize;
            if let Some(value) = read_u32(self.pid, read_addr) {
                let result = (value & mask) != 0;
                if result {
                    log::info!("DS1R: Flag {} is SET", event_flag_id);
                }
                return result;
            }
        }
        false
    }

    pub fn get_in_game_time_milliseconds(&self) -> i32 {
        let addr = self.game_data_man.get_address();
        if addr == 0 {
            return 0;
        }
        read_i32(self.pid, (addr + 0xa4) as usize).unwrap_or(0)
    }

    pub fn is_player_loaded(&self) -> bool {
        !self.player_ins.is_null_ptr()
    }

    pub fn get_position(&self) -> Vector3f {
        let addr = self.player_pos.get_address();
        if addr == 0 {
            return Vector3f::default();
        }
        Vector3f {
            x: read_f32(self.pid, (addr + 0x10) as usize).unwrap_or(0.0),
            y: read_f32(self.pid, (addr + 0x14) as usize).unwrap_or(0.0),
            z: read_f32(self.pid, (addr + 0x18) as usize).unwrap_or(0.0),
        }
    }

    pub fn get_attribute(&self, attribute: Attribute) -> i32 {
        let addr = self.player_game_data.get_address();
        if addr == 0 {
            return -1;
        }
        read_i32(self.pid, (addr + 0x8 + attribute as i64) as usize).unwrap_or(-1)
    }

    pub fn ng_count(&self) -> i32 {
        let addr = self.game_data_man.get_address();
        if addr == 0 {
            return 0;
        }
        read_i32(self.pid, (addr + 0x78) as usize).unwrap_or(0)
    }

    pub fn is_warp_requested(&self) -> bool {
        let game_man_addr = self.game_man.get_address();
        if game_man_addr == 0 {
            return false;
        }
        let warp_flag = read_u32(self.pid, (game_man_addr + 0x19) as usize).unwrap_or(0) as u8;
        warp_flag == 1
    }

    pub fn are_credits_rolling(&self) -> bool {
        let addr = self.menu_man.get_address();
        if addr == 0 {
            return false;
        }
        let val_80 = read_i32(self.pid, (addr + 0x80) as usize).unwrap_or(-1);
        let val_c8 = read_i32(self.pid, (addr + 0xc8) as usize).unwrap_or(0);
        let val_d4 = read_i32(self.pid, (addr + 0xd4) as usize).unwrap_or(0);
        val_80 == 0 && val_c8 == 1 && val_d4 == 1
    }

    pub fn get_current_save_slot(&self) -> i32 {
        let addr = self.game_data_man.get_address();
        if addr == 0 {
            return -1;
        }
        read_i32(self.pid, (addr + self.current_save_slot_offset) as usize).unwrap_or(-1)
    }

    pub fn get_player_health(&self) -> i32 {
        let addr = self.player_ins.get_address();
        if addr == 0 {
            return 0;
        }
        read_i32(self.pid, (addr + 0x3e8) as usize).unwrap_or(0)
    }
}

#[cfg(target_os = "linux")]
impl Default for DarkSouls1 {
    fn default() -> Self {
        Self::new()
    }
}
