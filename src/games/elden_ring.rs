//! Elden Ring autosplitter - port of SoulSplitter's EldenRing.cs
//! https://github.com/FrankvdStam/SoulSplitter
//!
//! Uses VirtualMemoryFlag with a tree-based structure

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HANDLE;

#[cfg(target_os = "windows")]
use crate::memory::{parse_pattern, resolve_rip_relative, scan_pattern, read_i32, read_i64, read_f32, read_u32};
#[cfg(target_os = "windows")]
use crate::memory::pointer::Pointer;

// Elden Ring patterns from SoulSplitter
#[cfg(target_os = "windows")]
pub const VIRTUAL_MEMORY_FLAG_PATTERN: &str = "44 89 7c 24 28 4c 8b 25 ? ? ? ? 4d 85 e4";
#[cfg(target_os = "windows")]
pub const FD4_TIME_PATTERN: &str = "48 8b 05 ? ? ? ? 4c 8b 40 08 4d 85 c0 74 0d 45 0f b6 80 be 00 00 00 e9 13 00 00 00";
#[cfg(target_os = "windows")]
pub const WORLD_CHR_MAN_PATTERN: &str = "48 8b 35 ? ? ? ? 48 85 f6 ? ? bb 01 00 00 00 89 5c 24 20 48 8b b6";
#[cfg(target_os = "windows")]
pub const MENU_MAN_IMP_PATTERN: &str = "48 8b 0d ? ? ? ? 48 8b 53 08 48 8b 92 d8 00 00 00 48 83 c4 20 5b";
#[cfg(target_os = "windows")]
pub const GAME_DATA_MAN_PATTERN: &str = "48 8b 05 ? ? ? ? 48 8d 4d c0 41 b8 10 00 00 00 48 8b 10 48 83 c2 1c";

/// Player position with map info
#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Copy, Default)]
pub struct Position {
    pub area: u8,
    pub block: u8,
    pub region: u8,
    pub size: u8,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// Screen states
#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ScreenState {
    Unknown = -1,
    Loading = 0,
    Logo = 1,
    MainMenu = 2,
    InGame = 4,
}

#[cfg(target_os = "windows")]
impl From<i32> for ScreenState {
    fn from(val: i32) -> Self {
        match val {
            0 => ScreenState::Loading,
            1 => ScreenState::Logo,
            2 => ScreenState::MainMenu,
            4 => ScreenState::InGame,
            _ => ScreenState::Unknown,
        }
    }
}

/// Elden Ring autosplitter state
#[cfg(target_os = "windows")]
pub struct EldenRing {
    pub handle: HANDLE,
    // Core pointers
    pub virtual_memory_flag: Pointer,
    pub fd4_time: Pointer,
    pub world_chr_man: Pointer,
    pub menu_man_imp: Pointer,
    pub game_data_man: Pointer,
    // Derived pointers
    pub igt: Pointer,
    pub player_ins: Pointer,
    pub ng_level: Pointer,
    pub player_game_data: Pointer,
    // Version-specific offsets
    screen_state_offset: i64,
    position_offset: i64,
    map_id_offset: i64,
    player_ins_offset: i64,
}

#[cfg(target_os = "windows")]
impl EldenRing {
    pub fn new() -> Self {
        Self {
            handle: HANDLE::default(),
            virtual_memory_flag: Pointer::new(),
            fd4_time: Pointer::new(),
            world_chr_man: Pointer::new(),
            menu_man_imp: Pointer::new(),
            game_data_man: Pointer::new(),
            igt: Pointer::new(),
            player_ins: Pointer::new(),
            ng_level: Pointer::new(),
            player_game_data: Pointer::new(),
            // Default offsets for latest version
            screen_state_offset: 0x730,
            position_offset: 0x6d4,
            map_id_offset: 0x6d0,
            player_ins_offset: 0x1e508,
        }
    }

    /// Initialize pointers by scanning for patterns
    pub fn init_pointers(&mut self, handle: HANDLE, base: usize, size: usize) -> bool {
        self.handle = handle;

        // Scan for VirtualMemoryFlag
        let pattern = parse_pattern(VIRTUAL_MEMORY_FLAG_PATTERN);
        let vmf_addr = match scan_pattern(handle, base, size, &pattern) {
            Some(found) => {
                match resolve_rip_relative(handle, found, 8, 7) {
                    Some(addr) => addr,
                    None => {
                        log::warn!("ER: Failed to resolve VirtualMemoryFlag RIP-relative address");
                        return false;
                    }
                }
            }
            None => {
                log::warn!("ER: VirtualMemoryFlag pattern not found");
                return false;
            }
        };
        self.virtual_memory_flag.initialize(handle, true, vmf_addr as i64, &[0x5]);
        log::info!("ER: VirtualMemoryFlag at 0x{:X}", vmf_addr);

        // Scan for FD4Time (IGT)
        let pattern = parse_pattern(FD4_TIME_PATTERN);
        if let Some(found) = scan_pattern(handle, base, size, &pattern) {
            if let Some(addr) = resolve_rip_relative(handle, found, 3, 7) {
                self.fd4_time.initialize(handle, true, addr as i64, &[0x0]);
                self.igt.initialize(handle, true, addr as i64, &[0x0, 0xa0]);
                log::info!("ER: FD4Time at 0x{:X}", addr);
            }
        }

        // Scan for WorldChrMan
        let pattern = parse_pattern(WORLD_CHR_MAN_PATTERN);
        if let Some(found) = scan_pattern(handle, base, size, &pattern) {
            if let Some(addr) = resolve_rip_relative(handle, found, 3, 7) {
                self.world_chr_man.initialize(handle, true, addr as i64, &[0x0]);
                self.player_ins.initialize(handle, true, addr as i64, &[0x0, self.player_ins_offset]);
                log::info!("ER: WorldChrMan at 0x{:X}", addr);
            }
        }

        // Scan for MenuManImp
        let pattern = parse_pattern(MENU_MAN_IMP_PATTERN);
        if let Some(found) = scan_pattern(handle, base, size, &pattern) {
            if let Some(addr) = resolve_rip_relative(handle, found, 3, 7) {
                self.menu_man_imp.initialize(handle, true, addr as i64, &[0x0]);
                log::info!("ER: MenuManImp at 0x{:X}", addr);
            }
        }

        // Scan for GameDataMan
        let pattern = parse_pattern(GAME_DATA_MAN_PATTERN);
        if let Some(found) = scan_pattern(handle, base, size, &pattern) {
            if let Some(addr) = resolve_rip_relative(handle, found, 3, 7) {
                self.game_data_man.initialize(handle, true, addr as i64, &[0x0]);
                self.ng_level.initialize(handle, true, addr as i64, &[0x0, 0x120]);
                self.player_game_data.initialize(handle, true, addr as i64, &[0x0, 0x8]);
                log::info!("ER: GameDataMan at 0x{:X}", addr);
            }
        }

        true
    }

    /// Read event flag - port of SoulSplitter's ReadEventFlag for Elden Ring
    pub fn read_event_flag(&self, event_flag_id: u32) -> bool {
        let divisor = self.virtual_memory_flag.read_i32(Some(0x1c));
        if divisor == 0 {
            return false;
        }

        let category = event_flag_id / divisor as u32;
        let least_significant_digits = event_flag_id - (category * divisor as u32);

        let current_element_root = self.virtual_memory_flag.create_pointer_from_address(Some(0x38));
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
            let mult = self.virtual_memory_flag.read_i32(Some(0x20));
            let elem_val = read_i32(self.handle, (current_elem_addr + 0x30) as usize).unwrap_or(0);
            let base_addr = self.virtual_memory_flag.read_i64(Some(0x28));
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

    /// Get in-game time in milliseconds
    pub fn get_in_game_time_milliseconds(&self) -> i32 {
        self.igt.read_i32(None)
    }

    /// Read NG+ level
    pub fn read_ng_level(&self) -> i32 {
        self.ng_level.read_i32(None)
    }

    /// Check if player is loaded
    pub fn is_player_loaded(&self) -> bool {
        let addr = self.player_ins.get_address();
        if addr == 0 {
            return false;
        }
        read_i64(self.handle, addr as usize).unwrap_or(0) != 0
    }

    /// Get current screen state
    pub fn get_screen_state(&self) -> ScreenState {
        let addr = self.menu_man_imp.get_address();
        if addr == 0 {
            return ScreenState::Unknown;
        }
        let val = read_i32(self.handle, (addr + self.screen_state_offset) as usize).unwrap_or(-1);
        ScreenState::from(val)
    }

    /// Check if blackscreen/fade is active
    pub fn is_blackscreen_active(&self) -> bool {
        let screen_state = self.get_screen_state();
        if screen_state != ScreenState::InGame {
            return false;
        }

        let addr = self.menu_man_imp.get_address();
        if addr == 0 {
            return false;
        }

        let flag = read_u32(self.handle, (addr + 0x18) as usize).unwrap_or(0);
        // Bit 0 set, bit 8 clear, bit 16 set
        let bit0 = (flag & 0x1) != 0;
        let bit8 = (flag & 0x100) != 0;
        let bit16 = (flag & 0x10000) != 0;

        bit0 && !bit8 && bit16
    }

    /// Get player position with map info
    pub fn get_position(&self) -> Position {
        let addr = self.player_ins.get_address();
        if addr == 0 {
            return Position::default();
        }

        // Read map ID
        let map_id = read_u32(self.handle, (addr + self.map_id_offset) as usize).unwrap_or(0);

        Position {
            area: ((map_id >> 24) & 0xFF) as u8,
            block: ((map_id >> 16) & 0xFF) as u8,
            region: ((map_id >> 8) & 0xFF) as u8,
            size: (map_id & 0xFF) as u8,
            x: read_f32(self.handle, (addr + self.position_offset) as usize).unwrap_or(0.0),
            y: read_f32(self.handle, (addr + self.position_offset + 4) as usize).unwrap_or(0.0),
            z: read_f32(self.handle, (addr + self.position_offset + 8) as usize).unwrap_or(0.0),
        }
    }
}

#[cfg(target_os = "windows")]
impl Default for EldenRing {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Linux Implementation (for Proton/Wine)
// =============================================================================

#[cfg(target_os = "linux")]
use crate::memory::{parse_pattern, resolve_rip_relative, scan_pattern, read_i32, read_i64, read_f32, read_u32};
#[cfg(target_os = "linux")]
use crate::memory::pointer::Pointer;

// Memory patterns (same as Windows)
#[cfg(target_os = "linux")]
pub const VIRTUAL_MEMORY_FLAG_PATTERN: &str = "44 89 7c 24 28 4c 8b 25 ? ? ? ? 4d 85 e4";
#[cfg(target_os = "linux")]
pub const FD4_TIME_PATTERN: &str = "48 8b 05 ? ? ? ? 4c 8b 40 08 4d 85 c0 74 0d 45 0f b6 80 be 00 00 00 e9 13 00 00 00";
#[cfg(target_os = "linux")]
pub const WORLD_CHR_MAN_PATTERN: &str = "48 8b 35 ? ? ? ? 48 85 f6 ? ? bb 01 00 00 00 89 5c 24 20 48 8b b6";
#[cfg(target_os = "linux")]
pub const MENU_MAN_IMP_PATTERN: &str = "48 8b 0d ? ? ? ? 48 8b 53 08 48 8b 92 d8 00 00 00 48 83 c4 20 5b";
#[cfg(target_os = "linux")]
pub const GAME_DATA_MAN_PATTERN: &str = "48 8b 05 ? ? ? ? 48 8d 4d c0 41 b8 10 00 00 00 48 8b 10 48 83 c2 1c";

#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy, Default)]
pub struct Position {
    pub area: u8,
    pub block: u8,
    pub region: u8,
    pub size: u8,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ScreenState {
    Unknown = -1,
    Loading = 0,
    Logo = 1,
    MainMenu = 2,
    InGame = 4,
}

#[cfg(target_os = "linux")]
impl From<i32> for ScreenState {
    fn from(val: i32) -> Self {
        match val {
            0 => ScreenState::Loading,
            1 => ScreenState::Logo,
            2 => ScreenState::MainMenu,
            4 => ScreenState::InGame,
            _ => ScreenState::Unknown,
        }
    }
}

#[cfg(target_os = "linux")]
pub struct EldenRing {
    pub pid: i32,
    // Core pointers
    pub virtual_memory_flag: Pointer,
    pub fd4_time: Pointer,
    pub world_chr_man: Pointer,
    pub menu_man_imp: Pointer,
    pub game_data_man: Pointer,
    // Derived pointers
    pub igt: Pointer,
    pub player_ins: Pointer,
    pub ng_level: Pointer,
    pub player_game_data: Pointer,
    // Version-specific offsets
    screen_state_offset: i64,
    position_offset: i64,
    map_id_offset: i64,
    player_ins_offset: i64,
}

#[cfg(target_os = "linux")]
impl EldenRing {
    pub fn new() -> Self {
        Self {
            pid: 0,
            virtual_memory_flag: Pointer::new(),
            fd4_time: Pointer::new(),
            world_chr_man: Pointer::new(),
            menu_man_imp: Pointer::new(),
            game_data_man: Pointer::new(),
            igt: Pointer::new(),
            player_ins: Pointer::new(),
            ng_level: Pointer::new(),
            player_game_data: Pointer::new(),
            screen_state_offset: 0x730,
            position_offset: 0x6d4,
            map_id_offset: 0x6d0,
            player_ins_offset: 0x1e508,
        }
    }

    pub fn init_pointers(&mut self, pid: i32, base: usize, size: usize) -> bool {
        self.pid = pid;
        log::info!("ER: Initializing pointers (Linux), base=0x{:X}, size=0x{:X}", base, size);

        // Scan for VirtualMemoryFlag
        let pattern = parse_pattern(VIRTUAL_MEMORY_FLAG_PATTERN);
        let vmf_addr = match scan_pattern(pid, base, size, &pattern) {
            Some(found) => {
                match resolve_rip_relative(pid, found, 8, 7) {
                    Some(addr) => addr,
                    None => {
                        log::warn!("ER: Failed to resolve VirtualMemoryFlag RIP-relative address");
                        return false;
                    }
                }
            }
            None => {
                log::warn!("ER: VirtualMemoryFlag pattern not found");
                return false;
            }
        };
        self.virtual_memory_flag.initialize(pid, true, vmf_addr as i64, &[0x5]);
        log::info!("ER: VirtualMemoryFlag at 0x{:X}", vmf_addr);

        // Scan for FD4Time (IGT)
        let pattern = parse_pattern(FD4_TIME_PATTERN);
        if let Some(found) = scan_pattern(pid, base, size, &pattern) {
            if let Some(addr) = resolve_rip_relative(pid, found, 3, 7) {
                self.fd4_time.initialize(pid, true, addr as i64, &[0x0]);
                self.igt.initialize(pid, true, addr as i64, &[0x0, 0xa0]);
                log::info!("ER: FD4Time at 0x{:X}", addr);
            }
        }

        // Scan for WorldChrMan
        let pattern = parse_pattern(WORLD_CHR_MAN_PATTERN);
        if let Some(found) = scan_pattern(pid, base, size, &pattern) {
            if let Some(addr) = resolve_rip_relative(pid, found, 3, 7) {
                self.world_chr_man.initialize(pid, true, addr as i64, &[0x0]);
                self.player_ins.initialize(pid, true, addr as i64, &[0x0, self.player_ins_offset]);
                log::info!("ER: WorldChrMan at 0x{:X}", addr);
            }
        }

        // Scan for MenuManImp
        let pattern = parse_pattern(MENU_MAN_IMP_PATTERN);
        if let Some(found) = scan_pattern(pid, base, size, &pattern) {
            if let Some(addr) = resolve_rip_relative(pid, found, 3, 7) {
                self.menu_man_imp.initialize(pid, true, addr as i64, &[0x0]);
                log::info!("ER: MenuManImp at 0x{:X}", addr);
            }
        }

        // Scan for GameDataMan
        let pattern = parse_pattern(GAME_DATA_MAN_PATTERN);
        if let Some(found) = scan_pattern(pid, base, size, &pattern) {
            if let Some(addr) = resolve_rip_relative(pid, found, 3, 7) {
                self.game_data_man.initialize(pid, true, addr as i64, &[0x0]);
                self.ng_level.initialize(pid, true, addr as i64, &[0x0, 0x120]);
                self.player_game_data.initialize(pid, true, addr as i64, &[0x0, 0x8]);
                log::info!("ER: GameDataMan at 0x{:X}", addr);
            }
        }

        true
    }

    pub fn read_event_flag(&self, event_flag_id: u32) -> bool {
        let divisor = self.virtual_memory_flag.read_i32(Some(0x1c));
        if divisor == 0 {
            return false;
        }

        let category = event_flag_id / divisor as u32;
        let least_significant_digits = event_flag_id - (category * divisor as u32);

        let current_element_root = self.virtual_memory_flag.create_pointer_from_address(Some(0x38));
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

        let mystery_value = read_i32(self.pid, (current_elem_addr + 0x28) as usize).unwrap_or(0) - 1;

        let calculated_pointer: i64;
        if mystery_value == 0 {
            let mult = self.virtual_memory_flag.read_i32(Some(0x20));
            let elem_val = read_i32(self.pid, (current_elem_addr + 0x30) as usize).unwrap_or(0);
            let base_addr = self.virtual_memory_flag.read_i64(Some(0x28));
            calculated_pointer = (mult as i64 * elem_val as i64) + base_addr;
        } else if mystery_value == 1 {
            return false;
        } else {
            calculated_pointer = read_i64(self.pid, (current_elem_addr + 0x30) as usize).unwrap_or(0);
        }

        if calculated_pointer == 0 {
            return false;
        }

        let thing = 7 - (least_significant_digits & 7);
        let mask = 1i32 << thing;
        let shifted = least_significant_digits >> 3;

        let final_addr = (calculated_pointer + shifted as i64) as usize;
        if let Some(read_value) = read_i32(self.pid, final_addr) {
            return (read_value & mask) != 0;
        }

        false
    }

    pub fn get_in_game_time_milliseconds(&self) -> i32 {
        self.igt.read_i32(None)
    }

    pub fn read_ng_level(&self) -> i32 {
        self.ng_level.read_i32(None)
    }

    pub fn is_player_loaded(&self) -> bool {
        let addr = self.player_ins.get_address();
        if addr == 0 {
            return false;
        }
        read_i64(self.pid, addr as usize).unwrap_or(0) != 0
    }

    pub fn get_screen_state(&self) -> ScreenState {
        let addr = self.menu_man_imp.get_address();
        if addr == 0 {
            return ScreenState::Unknown;
        }
        let val = read_i32(self.pid, (addr + self.screen_state_offset) as usize).unwrap_or(-1);
        ScreenState::from(val)
    }

    pub fn is_blackscreen_active(&self) -> bool {
        let screen_state = self.get_screen_state();
        if screen_state != ScreenState::InGame {
            return false;
        }

        let addr = self.menu_man_imp.get_address();
        if addr == 0 {
            return false;
        }

        let flag = read_u32(self.pid, (addr + 0x18) as usize).unwrap_or(0);
        let bit0 = (flag & 0x1) != 0;
        let bit8 = (flag & 0x100) != 0;
        let bit16 = (flag & 0x10000) != 0;

        bit0 && !bit8 && bit16
    }

    pub fn get_position(&self) -> Position {
        let addr = self.player_ins.get_address();
        if addr == 0 {
            return Position::default();
        }

        let map_id = read_u32(self.pid, (addr + self.map_id_offset) as usize).unwrap_or(0);

        Position {
            area: ((map_id >> 24) & 0xFF) as u8,
            block: ((map_id >> 16) & 0xFF) as u8,
            region: ((map_id >> 8) & 0xFF) as u8,
            size: (map_id & 0xFF) as u8,
            x: read_f32(self.pid, (addr + self.position_offset) as usize).unwrap_or(0.0),
            y: read_f32(self.pid, (addr + self.position_offset + 4) as usize).unwrap_or(0.0),
            z: read_f32(self.pid, (addr + self.position_offset + 8) as usize).unwrap_or(0.0),
        }
    }
}

#[cfg(target_os = "linux")]
impl Default for EldenRing {
    fn default() -> Self {
        Self::new()
    }
}
