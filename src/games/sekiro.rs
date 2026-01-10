//! Sekiro: Shadows Die Twice autosplitter - port of SoulSplitter's Sekiro.cs
//! https://github.com/FrankvdStam/SoulSplitter
//!
//! Very similar to Dark Souls 3 - uses the same SprjEventFlagMan structure

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HANDLE;

#[cfg(target_os = "windows")]
use crate::memory::{parse_pattern, resolve_rip_relative, scan_pattern, read_i32, read_i64, read_f32};
#[cfg(target_os = "windows")]
use crate::memory::pointer::Pointer;

// Sekiro patterns from SoulSplitter
#[cfg(target_os = "windows")]
pub const EVENT_FLAG_MAN_PATTERN: &str = "48 8b 0d ? ? ? ? 48 89 5c 24 50 48 89 6c 24 58 48 89 74 24 60";
#[cfg(target_os = "windows")]
pub const FIELD_AREA_PATTERN: &str = "48 8b 0d ? ? ? ? 48 85 c9 74 26 44 8b 41 28 48 8d 54 24 40";
#[cfg(target_os = "windows")]
pub const WORLD_CHR_MAN_PATTERN: &str = "48 8B 35 ? ? ? ? 44 0F 28 18";
#[cfg(target_os = "windows")]
pub const IGT_PATTERN: &str = "48 8b 05 ? ? ? ? 32 d2 48 8b 48";
#[cfg(target_os = "windows")]
pub const FADE_MAN_IMP_PATTERN: &str = "48 89 35 ? ? ? ? 48 8b c7 48 8b";
#[cfg(target_os = "windows")]
pub const PLAYER_GAME_DATA_PATTERN: &str = "48 8b 0d ? ? ? ? 48 8b 41 20 c6";

/// Player position as 3D vector
#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Copy, Default)]
pub struct Vector3f {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// Character attributes for Sekiro
#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i64)]
pub enum Attribute {
    Vitality = 0x44,      // +9 from base
    AttackPower = 0x48,
}

/// Sekiro autosplitter state
#[cfg(target_os = "windows")]
pub struct Sekiro {
    pub handle: HANDLE,
    // Core pointers
    pub event_flag_man: Pointer,
    pub field_area: Pointer,
    pub world_chr_man: Pointer,
    pub igt: Pointer,
    pub fade_man_imp: Pointer,
    pub player_game_data: Pointer,
    // Derived pointers
    pub player_pos: Pointer,
    pub fade_system: Pointer,
}

#[cfg(target_os = "windows")]
impl Sekiro {
    pub fn new() -> Self {
        Self {
            handle: HANDLE::default(),
            event_flag_man: Pointer::new(),
            field_area: Pointer::new(),
            world_chr_man: Pointer::new(),
            igt: Pointer::new(),
            fade_man_imp: Pointer::new(),
            player_game_data: Pointer::new(),
            player_pos: Pointer::new(),
            fade_system: Pointer::new(),
        }
    }

    /// Initialize pointers by scanning for patterns
    pub fn init_pointers(&mut self, handle: HANDLE, base: usize, size: usize) -> bool {
        self.handle = handle;

        // Scan for EventFlagMan
        let efm_pattern = parse_pattern(EVENT_FLAG_MAN_PATTERN);
        let efm_addr = match scan_pattern(handle, base, size, &efm_pattern) {
            Some(found) => {
                match resolve_rip_relative(handle, found, 3, 7) {
                    Some(addr) => addr,
                    None => {
                        log::warn!("Sekiro: Failed to resolve EventFlagMan RIP-relative address");
                        return false;
                    }
                }
            }
            None => {
                log::warn!("Sekiro: EventFlagMan pattern not found");
                return false;
            }
        };
        self.event_flag_man.initialize(handle, true, efm_addr as i64, &[0x0]);
        log::info!("Sekiro: EventFlagMan at 0x{:X}", efm_addr);

        // Scan for FieldArea
        let fa_pattern = parse_pattern(FIELD_AREA_PATTERN);
        if let Some(found) = scan_pattern(handle, base, size, &fa_pattern) {
            if let Some(addr) = resolve_rip_relative(handle, found, 3, 7) {
                self.field_area.initialize(handle, true, addr as i64, &[]);
                log::info!("Sekiro: FieldArea at 0x{:X}", addr);
            }
        }

        // Scan for WorldChrMan
        let pattern = parse_pattern(WORLD_CHR_MAN_PATTERN);
        if let Some(found) = scan_pattern(handle, base, size, &pattern) {
            if let Some(addr) = resolve_rip_relative(handle, found, 3, 7) {
                self.world_chr_man.initialize(handle, true, addr as i64, &[0x0]);
                // PlayerPos: WorldChrMan -> 0x48 -> 0x28
                self.player_pos.initialize(handle, true, addr as i64, &[0x0, 0x48, 0x28]);
                log::info!("Sekiro: WorldChrMan at 0x{:X}", addr);
            }
        }

        // Scan for IGT
        let pattern = parse_pattern(IGT_PATTERN);
        if let Some(found) = scan_pattern(handle, base, size, &pattern) {
            if let Some(addr) = resolve_rip_relative(handle, found, 3, 7) {
                self.igt.initialize(handle, true, addr as i64, &[0x0, 0x9c]);
                log::info!("Sekiro: IGT at 0x{:X}", addr);
            }
        }

        // Scan for FadeManImp
        let pattern = parse_pattern(FADE_MAN_IMP_PATTERN);
        if let Some(found) = scan_pattern(handle, base, size, &pattern) {
            if let Some(addr) = resolve_rip_relative(handle, found, 3, 7) {
                self.fade_man_imp.initialize(handle, true, addr as i64, &[0x0]);
                // FadeSystem: FadeManImp -> 0x0 -> 0x8
                self.fade_system.initialize(handle, true, addr as i64, &[0x0, 0x8]);
                log::info!("Sekiro: FadeManImp at 0x{:X}", addr);
            }
        }

        // Scan for PlayerGameData
        let pattern = parse_pattern(PLAYER_GAME_DATA_PATTERN);
        if let Some(found) = scan_pattern(handle, base, size, &pattern) {
            if let Some(addr) = resolve_rip_relative(handle, found, 3, 7) {
                self.player_game_data.initialize(handle, true, addr as i64, &[0x0, 0x8]);
                log::info!("Sekiro: PlayerGameData at 0x{:X}", addr);
            }
        }

        true
    }

    /// Read event flag - port of SoulSplitter's ReadEventFlag for Sekiro
    /// Very similar to DS3 but with slightly different offsets (0x18 instead of 0x10, 0xb0 instead of 0x70)
    pub fn read_event_flag(&self, event_flag_id: u32) -> bool {
        let event_flag_id_div_10000000 = ((event_flag_id / 10_000_000) % 10) as i64;
        let event_flag_area = ((event_flag_id / 100_000) % 100) as i32;
        let event_flag_id_div_10000 = ((event_flag_id / 10_000) % 10) as i32;
        let event_flag_id_div_1000 = ((event_flag_id / 1_000) % 10) as i64;

        let mut flag_world_block_info_category: i32 = -1;

        if event_flag_area >= 90 || event_flag_area + event_flag_id_div_10000 == 0 {
            flag_world_block_info_category = 0;
        } else {
            if self.field_area.is_null_ptr() {
                return false;
            }

            // Sekiro uses 0x18 offset instead of DS3's 0x10
            let world_info_owner = self.field_area.append(&[0x18]).create_pointer_from_address(None);
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
                            // Sekiro uses 0xb0 stride instead of DS3's 0x70
                            let flag = block_vec.read_i32(Some((index * 0xb0) + 0x8));

                            if ((flag >> 0x10) & 0xff) == event_flag_id_div_10000
                                && (flag >> 0x18) == event_flag_area
                            {
                                found = true;
                                world_info_block_vector = Some(block_vec);
                                break;
                            }

                            index += 1;
                            if count as i64 <= index {
                                found = false;
                                break;
                            }
                        }
                    }

                    if found {
                        if let Some(ref block_vec) = world_info_block_vector {
                            // Sekiro uses 0xb0 stride
                            flag_world_block_info_category = block_vec.read_i32(Some((index * 0xb0) + 0x20));
                            break;
                        }
                    }
                }
            }

            if flag_world_block_info_category >= 0 {
                flag_world_block_info_category += 1;
            }
        }

        let ptr = self.event_flag_man.append(&[0x218, event_flag_id_div_10000000 * 0x18, 0x0]);

        if ptr.is_null_ptr() || flag_world_block_info_category < 0 {
            return false;
        }

        // Sekiro uses 0xa8 multiplier (same as DS3)
        let result_base = (event_flag_id_div_1000 << 4)
            + ptr.get_address()
            + (flag_world_block_info_category as i64 * 0xa8);

        let mut result_pointer_address = Pointer::new();
        result_pointer_address.initialize(self.handle, true, result_base, &[0x0]);

        if !result_pointer_address.is_null_ptr() {
            let mod_1000 = (event_flag_id % 1000) as u32;
            let read_offset = ((mod_1000 >> 5) * 4) as i64;
            let value = result_pointer_address.read_u32(Some(read_offset));

            let bit_shift = 0x1f - ((mod_1000 as u8) & 0x1f);
            let mask = 1u32 << (bit_shift & 0x1f);

            let result = value & mask;
            return result != 0;
        }

        false
    }

    /// Get in-game time in milliseconds
    pub fn get_in_game_time_milliseconds(&self) -> i32 {
        self.igt.read_i32(None)
    }

    /// Check if player is loaded
    pub fn is_player_loaded(&self) -> bool {
        let addr = self.world_chr_man.get_address();
        if addr == 0 {
            return false;
        }
        read_i64(self.handle, (addr + 0x88) as usize).unwrap_or(0) != 0
    }

    /// Get player position
    pub fn get_player_position(&self) -> Vector3f {
        let addr = self.player_pos.get_address();
        if addr == 0 {
            return Vector3f::default();
        }
        Vector3f {
            x: read_f32(self.handle, (addr + 0x80) as usize).unwrap_or(0.0),
            y: read_f32(self.handle, (addr + 0x84) as usize).unwrap_or(0.0),
            z: read_f32(self.handle, (addr + 0x88) as usize).unwrap_or(0.0),
        }
    }

    /// Check if blackscreen/fade is active
    pub fn is_blackscreen_active(&self) -> bool {
        let addr = self.fade_system.get_address();
        if addr == 0 {
            return false;
        }
        read_i32(self.handle, (addr + 0x2dc) as usize).unwrap_or(0) != 0
    }

    /// Get character attribute value
    pub fn get_attribute(&self, attribute: Attribute) -> i32 {
        let addr = self.player_game_data.get_address();
        if addr == 0 {
            return -1;
        }
        read_i32(self.handle, (addr + attribute as i64) as usize).unwrap_or(-1)
    }
}

#[cfg(target_os = "windows")]
impl Default for Sekiro {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Linux Implementation (for Proton/Wine)
// =============================================================================

#[cfg(target_os = "linux")]
use crate::memory::{parse_pattern, resolve_rip_relative, scan_pattern, read_i32, read_i64, read_f32};
#[cfg(target_os = "linux")]
use crate::memory::pointer::Pointer;

// Memory patterns (same as Windows)
#[cfg(target_os = "linux")]
pub const EVENT_FLAG_MAN_PATTERN: &str = "48 8b 0d ? ? ? ? 48 89 5c 24 50 48 89 6c 24 58 48 89 74 24 60";
#[cfg(target_os = "linux")]
pub const FIELD_AREA_PATTERN: &str = "48 8b 0d ? ? ? ? 48 85 c9 74 26 44 8b 41 28 48 8d 54 24 40";
#[cfg(target_os = "linux")]
pub const WORLD_CHR_MAN_PATTERN: &str = "48 8B 35 ? ? ? ? 44 0F 28 18";
#[cfg(target_os = "linux")]
pub const IGT_PATTERN: &str = "48 8b 05 ? ? ? ? 32 d2 48 8b 48";
#[cfg(target_os = "linux")]
pub const FADE_MAN_IMP_PATTERN: &str = "48 89 35 ? ? ? ? 48 8b c7 48 8b";
#[cfg(target_os = "linux")]
pub const PLAYER_GAME_DATA_PATTERN: &str = "48 8b 0d ? ? ? ? 48 8b 41 20 c6";

#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy, Default)]
pub struct Vector3f {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i64)]
pub enum Attribute {
    Vitality = 0x44,
    AttackPower = 0x48,
}

#[cfg(target_os = "linux")]
pub struct Sekiro {
    pub pid: i32,
    // Core pointers
    pub event_flag_man: Pointer,
    pub field_area: Pointer,
    pub world_chr_man: Pointer,
    pub igt: Pointer,
    pub fade_man_imp: Pointer,
    pub player_game_data: Pointer,
    // Derived pointers
    pub player_pos: Pointer,
    pub fade_system: Pointer,
}

#[cfg(target_os = "linux")]
impl Sekiro {
    pub fn new() -> Self {
        Self {
            pid: 0,
            event_flag_man: Pointer::new(),
            field_area: Pointer::new(),
            world_chr_man: Pointer::new(),
            igt: Pointer::new(),
            fade_man_imp: Pointer::new(),
            player_game_data: Pointer::new(),
            player_pos: Pointer::new(),
            fade_system: Pointer::new(),
        }
    }

    pub fn init_pointers(&mut self, pid: i32, base: usize, size: usize) -> bool {
        self.pid = pid;
        log::info!("Sekiro: Initializing pointers (Linux), base=0x{:X}, size=0x{:X}", base, size);

        // Scan for EventFlagMan
        let efm_pattern = parse_pattern(EVENT_FLAG_MAN_PATTERN);
        let efm_addr = match scan_pattern(pid, base, size, &efm_pattern) {
            Some(found) => {
                match resolve_rip_relative(pid, found, 3, 7) {
                    Some(addr) => addr,
                    None => {
                        log::warn!("Sekiro: Failed to resolve EventFlagMan RIP-relative address");
                        return false;
                    }
                }
            }
            None => {
                log::warn!("Sekiro: EventFlagMan pattern not found");
                return false;
            }
        };
        self.event_flag_man.initialize(pid, true, efm_addr as i64, &[0x0]);
        log::info!("Sekiro: EventFlagMan at 0x{:X}", efm_addr);

        // Scan for FieldArea
        let fa_pattern = parse_pattern(FIELD_AREA_PATTERN);
        if let Some(found) = scan_pattern(pid, base, size, &fa_pattern) {
            if let Some(addr) = resolve_rip_relative(pid, found, 3, 7) {
                self.field_area.initialize(pid, true, addr as i64, &[]);
                log::info!("Sekiro: FieldArea at 0x{:X}", addr);
            }
        }

        // Scan for WorldChrMan
        let pattern = parse_pattern(WORLD_CHR_MAN_PATTERN);
        if let Some(found) = scan_pattern(pid, base, size, &pattern) {
            if let Some(addr) = resolve_rip_relative(pid, found, 3, 7) {
                self.world_chr_man.initialize(pid, true, addr as i64, &[0x0]);
                self.player_pos.initialize(pid, true, addr as i64, &[0x0, 0x48, 0x28]);
                log::info!("Sekiro: WorldChrMan at 0x{:X}", addr);
            }
        }

        // Scan for IGT
        let pattern = parse_pattern(IGT_PATTERN);
        if let Some(found) = scan_pattern(pid, base, size, &pattern) {
            if let Some(addr) = resolve_rip_relative(pid, found, 3, 7) {
                self.igt.initialize(pid, true, addr as i64, &[0x0, 0x9c]);
                log::info!("Sekiro: IGT at 0x{:X}", addr);
            }
        }

        // Scan for FadeManImp
        let pattern = parse_pattern(FADE_MAN_IMP_PATTERN);
        if let Some(found) = scan_pattern(pid, base, size, &pattern) {
            if let Some(addr) = resolve_rip_relative(pid, found, 3, 7) {
                self.fade_man_imp.initialize(pid, true, addr as i64, &[0x0]);
                self.fade_system.initialize(pid, true, addr as i64, &[0x0, 0x8]);
                log::info!("Sekiro: FadeManImp at 0x{:X}", addr);
            }
        }

        // Scan for PlayerGameData
        let pattern = parse_pattern(PLAYER_GAME_DATA_PATTERN);
        if let Some(found) = scan_pattern(pid, base, size, &pattern) {
            if let Some(addr) = resolve_rip_relative(pid, found, 3, 7) {
                self.player_game_data.initialize(pid, true, addr as i64, &[0x0, 0x8]);
                log::info!("Sekiro: PlayerGameData at 0x{:X}", addr);
            }
        }

        true
    }

    pub fn read_event_flag(&self, event_flag_id: u32) -> bool {
        let event_flag_id_div_10000000 = ((event_flag_id / 10_000_000) % 10) as i64;
        let event_flag_area = ((event_flag_id / 100_000) % 100) as i32;
        let event_flag_id_div_10000 = ((event_flag_id / 10_000) % 10) as i32;
        let event_flag_id_div_1000 = ((event_flag_id / 1_000) % 10) as i64;

        let mut flag_world_block_info_category: i32 = -1;

        if event_flag_area >= 90 || event_flag_area + event_flag_id_div_10000 == 0 {
            flag_world_block_info_category = 0;
        } else {
            if self.field_area.is_null_ptr() {
                return false;
            }

            let world_info_owner = self.field_area.append(&[0x18]).create_pointer_from_address(None);
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
                            let flag = block_vec.read_i32(Some((index * 0xb0) + 0x8));

                            if ((flag >> 0x10) & 0xff) == event_flag_id_div_10000
                                && (flag >> 0x18) == event_flag_area
                            {
                                found = true;
                                world_info_block_vector = Some(block_vec);
                                break;
                            }

                            index += 1;
                            if count as i64 <= index {
                                found = false;
                                break;
                            }
                        }
                    }

                    if found {
                        if let Some(ref block_vec) = world_info_block_vector {
                            flag_world_block_info_category = block_vec.read_i32(Some((index * 0xb0) + 0x20));
                            break;
                        }
                    }
                }
            }

            if flag_world_block_info_category >= 0 {
                flag_world_block_info_category += 1;
            }
        }

        let ptr = self.event_flag_man.append(&[0x218, event_flag_id_div_10000000 * 0x18, 0x0]);

        if ptr.is_null_ptr() || flag_world_block_info_category < 0 {
            return false;
        }

        let result_base = (event_flag_id_div_1000 << 4)
            + ptr.get_address()
            + (flag_world_block_info_category as i64 * 0xa8);

        let mut result_pointer_address = Pointer::new();
        result_pointer_address.initialize(self.pid, true, result_base, &[0x0]);

        if !result_pointer_address.is_null_ptr() {
            let mod_1000 = (event_flag_id % 1000) as u32;
            let read_offset = ((mod_1000 >> 5) * 4) as i64;
            let value = result_pointer_address.read_u32(Some(read_offset));

            let bit_shift = 0x1f - ((mod_1000 as u8) & 0x1f);
            let mask = 1u32 << (bit_shift & 0x1f);

            let result = value & mask;
            return result != 0;
        }

        false
    }

    pub fn get_in_game_time_milliseconds(&self) -> i32 {
        self.igt.read_i32(None)
    }

    pub fn is_player_loaded(&self) -> bool {
        let addr = self.world_chr_man.get_address();
        if addr == 0 {
            return false;
        }
        read_i64(self.pid, (addr + 0x88) as usize).unwrap_or(0) != 0
    }

    pub fn get_player_position(&self) -> Vector3f {
        let addr = self.player_pos.get_address();
        if addr == 0 {
            return Vector3f::default();
        }
        Vector3f {
            x: read_f32(self.pid, (addr + 0x80) as usize).unwrap_or(0.0),
            y: read_f32(self.pid, (addr + 0x84) as usize).unwrap_or(0.0),
            z: read_f32(self.pid, (addr + 0x88) as usize).unwrap_or(0.0),
        }
    }

    pub fn is_blackscreen_active(&self) -> bool {
        let addr = self.fade_system.get_address();
        if addr == 0 {
            return false;
        }
        read_i32(self.pid, (addr + 0x2dc) as usize).unwrap_or(0) != 0
    }

    pub fn get_attribute(&self, attribute: Attribute) -> i32 {
        let addr = self.player_game_data.get_address();
        if addr == 0 {
            return -1;
        }
        read_i32(self.pid, (addr + attribute as i64) as usize).unwrap_or(-1)
    }
}

#[cfg(target_os = "linux")]
impl Default for Sekiro {
    fn default() -> Self {
        Self::new()
    }
}
