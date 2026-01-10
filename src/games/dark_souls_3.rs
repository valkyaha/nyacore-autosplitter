//! Dark Souls III autosplitter - exact port of SoulSplitter's DarkSouls3.cs
//! https://github.com/FrankvdStam/SoulSplitter
//!
//! This is a direct 1:1 port of the ReadEventFlag method from SoulSplitter.

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HANDLE;

#[cfg(target_os = "windows")]
use crate::memory::pointer::Pointer;
#[cfg(target_os = "windows")]
use crate::memory::{parse_pattern, scan_pattern, resolve_rip_relative, read_i32, read_i64, read_f32};

// DS3 patterns from SoulSplitter (used on both Windows and Linux)
pub const SPRJ_EVENT_FLAG_MAN_PATTERN: &str = "48 c7 05 ? ? ? ? 00 00 00 00 48 8b 7c 24 38 c7 46 54 ff ff ff ff 48 83 c4 20 5e c3";
pub const FIELD_AREA_PATTERN: &str = "4c 8b 3d ? ? ? ? 8b 45 87 83 f8 ff 74 69 48 8d 4d 8f 48 89 4d 9f 89 45 8f 48 8d 55 8f 49 8b 4f 10";
pub const NEW_MENU_SYSTEM_PATTERN: &str = "48 8b 0d ? ? ? ? 48 8b 7c 24 20 48 8b 5c 24 30 48 85 c9";
pub const GAME_DATA_MAN_PATTERN: &str = "48 8b 0d ? ? ? ? 4c 8d 44 24 40 45 33 c9 48 8b d3 40 88";
pub const PLAYER_INS_PATTERN: &str = "48 8b 0d ? ? ? ? 45 33 c0 48 8d 55 e7 e8 ? ? ? ? 0f 2f";
pub const LOADING_PATTERN: &str = "c6 05 ? ? ? ? ? e8 ? ? ? ? 84 c0 0f 94 c0 e9";
pub const SPRJ_FADE_IMP_PATTERN: &str = "48 8b 0d ? ? ? ? 4c 8d 4c 24 38 4c 8d 44 24 48 33 d2";

/// Player position as 3D vector
#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Copy, Default)]
pub struct Vector3f {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// Character attributes for DS3
#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i64)]
pub enum Attribute {
    Vigor = 0x44,
    Attunement = 0x48,
    Endurance = 0x4C,
    Vitality = 0x6C,
    Strength = 0x50,
    Dexterity = 0x54,
    Intelligence = 0x58,
    Faith = 0x5C,
    Luck = 0x60,
    SoulLevel = 0x68,
}

/// Dark Souls III autosplitter state
#[cfg(target_os = "windows")]
pub struct DarkSouls3 {
    pub handle: HANDLE,
    pub is_64_bit: bool,
    // Core pointers
    pub sprj_event_flag_man: Pointer,
    pub field_area: Pointer,
    pub new_menu_system: Pointer,
    pub game_data_man: Pointer,
    pub player_ins: Pointer,
    pub loading: Pointer,
    pub sprj_fade_imp: Pointer,
    // Derived pointers
    pub player_game_data: Pointer,
    pub sprj_chr_physics_module: Pointer,
    pub blackscreen: Pointer,
    // Version-specific offset for IGT
    igt_offset: i64,
}

#[cfg(target_os = "windows")]
impl DarkSouls3 {
    pub fn new() -> Self {
        Self {
            handle: HANDLE::default(),
            is_64_bit: true,
            sprj_event_flag_man: Pointer::new(),
            field_area: Pointer::new(),
            new_menu_system: Pointer::new(),
            game_data_man: Pointer::new(),
            player_ins: Pointer::new(),
            loading: Pointer::new(),
            sprj_fade_imp: Pointer::new(),
            player_game_data: Pointer::new(),
            sprj_chr_physics_module: Pointer::new(),
            blackscreen: Pointer::new(),
            igt_offset: 0xa4,  // Default, 0x9c for older versions
        }
    }

    /// Initialize pointers by scanning for patterns
    pub fn init_pointers(&mut self, handle: HANDLE, base: usize, size: usize) -> bool {
        self.handle = handle;
        self.is_64_bit = true;

        log::info!("DS3: Scanning for patterns in memory region 0x{:X}-0x{:X}", base, base + size);

        // Scan for SprjEventFlagMan
        let sprj_pattern = parse_pattern(SPRJ_EVENT_FLAG_MAN_PATTERN);
        let sprj_addr = match scan_pattern(handle, base, size, &sprj_pattern) {
            Some(found) => {
                log::info!("DS3: SprjEventFlagMan pattern found at 0x{:X}", found);
                match resolve_rip_relative(handle, found, 3, 11) {
                    Some(addr) => addr,
                    None => {
                        log::error!("DS3: Failed to resolve SprjEventFlagMan RIP-relative address");
                        return false;
                    }
                }
            }
            None => {
                log::error!("DS3: SprjEventFlagMan pattern NOT FOUND");
                return false;
            }
        };
        self.sprj_event_flag_man.initialize(handle, true, sprj_addr as i64, &[0x0]);
        log::info!("DS3: SprjEventFlagMan at 0x{:X}", sprj_addr);

        // Scan for FieldArea
        let field_pattern = parse_pattern(FIELD_AREA_PATTERN);
        if let Some(found) = scan_pattern(handle, base, size, &field_pattern) {
            if let Some(addr) = resolve_rip_relative(handle, found, 3, 7) {
                self.field_area.initialize(handle, true, addr as i64, &[]);
                log::info!("DS3: FieldArea at 0x{:X}", addr);
            }
        }

        // Scan for NewMenuSystem
        let pattern = parse_pattern(NEW_MENU_SYSTEM_PATTERN);
        if let Some(found) = scan_pattern(handle, base, size, &pattern) {
            if let Some(addr) = resolve_rip_relative(handle, found, 3, 7) {
                self.new_menu_system.initialize(handle, true, addr as i64, &[0x0]);
                log::info!("DS3: NewMenuSystem at 0x{:X}", addr);
            }
        }

        // Scan for GameDataMan
        let pattern = parse_pattern(GAME_DATA_MAN_PATTERN);
        if let Some(found) = scan_pattern(handle, base, size, &pattern) {
            if let Some(addr) = resolve_rip_relative(handle, found, 3, 7) {
                self.game_data_man.initialize(handle, true, addr as i64, &[0x0]);
                // PlayerGameData: GameDataMan -> 0x10
                self.player_game_data.initialize(handle, true, addr as i64, &[0x0, 0x10]);
                log::info!("DS3: GameDataMan at 0x{:X}", addr);
            }
        }

        // Scan for PlayerIns
        let pattern = parse_pattern(PLAYER_INS_PATTERN);
        if let Some(found) = scan_pattern(handle, base, size, &pattern) {
            if let Some(addr) = resolve_rip_relative(handle, found, 3, 7) {
                self.player_ins.initialize(handle, true, addr as i64, &[0x0]);
                // SprjChrPhysicsModule: PlayerIns -> 0x80 -> 0x40 -> 0x28
                self.sprj_chr_physics_module.initialize(handle, true, addr as i64, &[0x0, 0x80, 0x40, 0x28]);
                log::info!("DS3: PlayerIns at 0x{:X}", addr);
            }
        }

        // Scan for Loading
        let pattern = parse_pattern(LOADING_PATTERN);
        if let Some(found) = scan_pattern(handle, base, size, &pattern) {
            if let Some(addr) = resolve_rip_relative(handle, found, 2, 7) {
                self.loading.initialize(handle, true, addr as i64, &[]);
                log::info!("DS3: Loading at 0x{:X}", addr);
            }
        }

        // Scan for SprjFadeImp (blackscreen)
        let pattern = parse_pattern(SPRJ_FADE_IMP_PATTERN);
        if let Some(found) = scan_pattern(handle, base, size, &pattern) {
            if let Some(addr) = resolve_rip_relative(handle, found, 3, 7) {
                self.sprj_fade_imp.initialize(handle, true, addr as i64, &[0x0]);
                // Blackscreen: SprjFadeImp -> 0x0 -> 0x8 -> 0x2ec
                self.blackscreen.initialize(handle, true, addr as i64, &[0x0, 0x8]);
                log::info!("DS3: SprjFadeImp at 0x{:X}", addr);
            }
        }

        log::info!("DS3: All pointers initialized successfully");
        true
    }

    /// Read event flag - exact port of SoulSplitter's ReadEventFlag
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

            let world_info_owner = self.field_area.append(&[0x0, 0x10]).create_pointer_from_address(None);
            let size = world_info_owner.read_i32(Some(0x8));
            let vector = world_info_owner.append(&[0x10]);

            for i in 0..size {
                let area = vector.read_byte(Some((i as i64 * 0x38) + 0xb)) as i32;

                if area == event_flag_area {
                    let count = vector.read_byte(Some(i as i64 * 0x38 + 0x20));
                    let mut index = 0;
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
                                found = false;
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

        let ptr = self.sprj_event_flag_man.append(&[0x218, event_flag_id_div_10000000 * 0x18, 0x0]);

        if ptr.is_null_ptr() || flag_world_block_info_category < 0 {
            return false;
        }

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

    /// Check if loading screen is active
    pub fn is_loading(&self) -> bool {
        let addr = self.loading.get_address();
        if addr == 0 {
            return false;
        }
        // Reading at offset -1 (0xff...ff becomes previous byte in signed)
        read_i32(self.handle, (addr - 1) as usize).unwrap_or(0) != 0
    }

    /// Check if blackscreen is active (fade effect)
    pub fn blackscreen_active(&self) -> bool {
        let addr = self.blackscreen.get_address();
        if addr == 0 {
            return false;
        }
        read_i32(self.handle, (addr + 0x2ec) as usize).unwrap_or(0) != 0
    }

    /// Check if player is loaded
    pub fn is_player_loaded(&self) -> bool {
        let addr = self.player_ins.get_address();
        if addr == 0 {
            return false;
        }
        read_i64(self.handle, addr as usize).unwrap_or(0) != 0
    }

    /// Get player position
    pub fn get_position(&self) -> Vector3f {
        let addr = self.sprj_chr_physics_module.get_address();
        if addr == 0 {
            return Vector3f::default();
        }
        Vector3f {
            x: read_f32(self.handle, (addr + 0x80) as usize).unwrap_or(0.0),
            y: read_f32(self.handle, (addr + 0x84) as usize).unwrap_or(0.0),
            z: read_f32(self.handle, (addr + 0x88) as usize).unwrap_or(0.0),
        }
    }

    /// Get in-game time in milliseconds
    pub fn get_in_game_time_milliseconds(&self) -> i32 {
        let addr = self.game_data_man.get_address();
        if addr == 0 {
            return 0;
        }
        read_i32(self.handle, (addr + self.igt_offset) as usize).unwrap_or(0)
    }

    /// Get character attribute value
    pub fn read_attribute(&self, attribute: Attribute) -> i32 {
        // Check if player is loaded and not in menu
        if !self.is_player_loaded() {
            return -1;
        }

        // Check menu state (if menu state == 3, don't read)
        let menu_addr = self.new_menu_system.get_address();
        if menu_addr != 0 {
            let menu_state = read_i32(self.handle, menu_addr as usize).unwrap_or(0);
            if menu_state == 3 {
                return -1;
            }
        }

        let addr = self.player_game_data.get_address();
        if addr == 0 {
            return -1;
        }
        read_i32(self.handle, (addr + attribute as i64) as usize).unwrap_or(-1)
    }
}

#[cfg(target_os = "windows")]
impl Default for DarkSouls3 {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Linux Implementation (for Proton/Wine)
// =============================================================================

#[cfg(target_os = "linux")]
use crate::memory::pointer::Pointer;
#[cfg(target_os = "linux")]
use crate::memory::{parse_pattern, scan_pattern, resolve_rip_relative, read_i32, read_i64, read_f32};

/// Player position as 3D vector (Linux)
#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy, Default)]
pub struct Vector3f {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// Character attributes for DS3 (Linux)
#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i64)]
pub enum Attribute {
    Vigor = 0x44,
    Attunement = 0x48,
    Endurance = 0x4C,
    Vitality = 0x6C,
    Strength = 0x50,
    Dexterity = 0x54,
    Intelligence = 0x58,
    Faith = 0x5C,
    Luck = 0x60,
    SoulLevel = 0x68,
}

/// Dark Souls III autosplitter state (Linux)
#[cfg(target_os = "linux")]
pub struct DarkSouls3 {
    pub pid: i32,
    pub is_64_bit: bool,
    // Core pointers
    pub sprj_event_flag_man: Pointer,
    pub field_area: Pointer,
    pub new_menu_system: Pointer,
    pub game_data_man: Pointer,
    pub player_ins: Pointer,
    pub loading: Pointer,
    pub sprj_fade_imp: Pointer,
    // Derived pointers
    pub player_game_data: Pointer,
    pub sprj_chr_physics_module: Pointer,
    pub blackscreen: Pointer,
    // Version-specific offset for IGT
    igt_offset: i64,
}

#[cfg(target_os = "linux")]
impl DarkSouls3 {
    pub fn new() -> Self {
        Self {
            pid: 0,
            is_64_bit: true,
            sprj_event_flag_man: Pointer::new(),
            field_area: Pointer::new(),
            new_menu_system: Pointer::new(),
            game_data_man: Pointer::new(),
            player_ins: Pointer::new(),
            loading: Pointer::new(),
            sprj_fade_imp: Pointer::new(),
            player_game_data: Pointer::new(),
            sprj_chr_physics_module: Pointer::new(),
            blackscreen: Pointer::new(),
            igt_offset: 0xa4,
        }
    }

    /// Initialize pointers by scanning for patterns (Linux/Proton)
    pub fn init_pointers(&mut self, pid: i32, base: usize, size: usize) -> bool {
        self.pid = pid;
        self.is_64_bit = true;

        log::info!("DS3 (Linux): Scanning for patterns in memory region 0x{:X}-0x{:X}", base, base + size);

        // Scan for SprjEventFlagMan
        let sprj_pattern = parse_pattern(SPRJ_EVENT_FLAG_MAN_PATTERN);
        let sprj_addr = match scan_pattern(pid, base, size, &sprj_pattern) {
            Some(found) => {
                log::info!("DS3: SprjEventFlagMan pattern found at 0x{:X}", found);
                match resolve_rip_relative(pid, found, 3, 11) {
                    Some(addr) => addr,
                    None => {
                        log::error!("DS3: Failed to resolve SprjEventFlagMan RIP-relative address");
                        return false;
                    }
                }
            }
            None => {
                log::error!("DS3: SprjEventFlagMan pattern NOT FOUND");
                return false;
            }
        };
        self.sprj_event_flag_man.initialize(pid, true, sprj_addr as i64, &[0x0]);
        log::info!("DS3: SprjEventFlagMan at 0x{:X}", sprj_addr);

        // Scan for FieldArea
        let field_pattern = parse_pattern(FIELD_AREA_PATTERN);
        if let Some(found) = scan_pattern(pid, base, size, &field_pattern) {
            if let Some(addr) = resolve_rip_relative(pid, found, 3, 7) {
                self.field_area.initialize(pid, true, addr as i64, &[]);
                log::info!("DS3: FieldArea at 0x{:X}", addr);
            }
        }

        // Scan for NewMenuSystem
        let pattern = parse_pattern(NEW_MENU_SYSTEM_PATTERN);
        if let Some(found) = scan_pattern(pid, base, size, &pattern) {
            if let Some(addr) = resolve_rip_relative(pid, found, 3, 7) {
                self.new_menu_system.initialize(pid, true, addr as i64, &[0x0]);
                log::info!("DS3: NewMenuSystem at 0x{:X}", addr);
            }
        }

        // Scan for GameDataMan
        let pattern = parse_pattern(GAME_DATA_MAN_PATTERN);
        if let Some(found) = scan_pattern(pid, base, size, &pattern) {
            if let Some(addr) = resolve_rip_relative(pid, found, 3, 7) {
                self.game_data_man.initialize(pid, true, addr as i64, &[0x0]);
                self.player_game_data.initialize(pid, true, addr as i64, &[0x0, 0x10]);
                log::info!("DS3: GameDataMan at 0x{:X}", addr);
            }
        }

        // Scan for PlayerIns
        let pattern = parse_pattern(PLAYER_INS_PATTERN);
        if let Some(found) = scan_pattern(pid, base, size, &pattern) {
            if let Some(addr) = resolve_rip_relative(pid, found, 3, 7) {
                self.player_ins.initialize(pid, true, addr as i64, &[0x0]);
                self.sprj_chr_physics_module.initialize(pid, true, addr as i64, &[0x0, 0x80, 0x40, 0x28]);
                log::info!("DS3: PlayerIns at 0x{:X}", addr);
            }
        }

        // Scan for Loading
        let pattern = parse_pattern(LOADING_PATTERN);
        if let Some(found) = scan_pattern(pid, base, size, &pattern) {
            if let Some(addr) = resolve_rip_relative(pid, found, 2, 7) {
                self.loading.initialize(pid, true, addr as i64, &[]);
                log::info!("DS3: Loading at 0x{:X}", addr);
            }
        }

        // Scan for SprjFadeImp (blackscreen)
        let pattern = parse_pattern(SPRJ_FADE_IMP_PATTERN);
        if let Some(found) = scan_pattern(pid, base, size, &pattern) {
            if let Some(addr) = resolve_rip_relative(pid, found, 3, 7) {
                self.sprj_fade_imp.initialize(pid, true, addr as i64, &[0x0]);
                self.blackscreen.initialize(pid, true, addr as i64, &[0x0, 0x8]);
                log::info!("DS3: SprjFadeImp at 0x{:X}", addr);
            }
        }

        log::info!("DS3 (Linux): All pointers initialized successfully");
        true
    }

    /// Read event flag - exact port of SoulSplitter's ReadEventFlag
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

            let world_info_owner = self.field_area.append(&[0x0, 0x10]).create_pointer_from_address(None);
            let size = world_info_owner.read_i32(Some(0x8));
            let vector = world_info_owner.append(&[0x10]);

            for i in 0..size {
                let area = vector.read_byte(Some((i as i64 * 0x38) + 0xb)) as i32;

                if area == event_flag_area {
                    let count = vector.read_byte(Some(i as i64 * 0x38 + 0x20));
                    let mut index = 0;
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
                                found = false;
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

        let ptr = self.sprj_event_flag_man.append(&[0x218, event_flag_id_div_10000000 * 0x18, 0x0]);

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

    /// Check if loading screen is active
    pub fn is_loading(&self) -> bool {
        let addr = self.loading.get_address();
        if addr == 0 {
            return false;
        }
        read_i32(self.pid, (addr - 1) as usize).unwrap_or(0) != 0
    }

    /// Check if blackscreen is active
    pub fn blackscreen_active(&self) -> bool {
        let addr = self.blackscreen.get_address();
        if addr == 0 {
            return false;
        }
        read_i32(self.pid, (addr + 0x2ec) as usize).unwrap_or(0) != 0
    }

    /// Check if player is loaded
    pub fn is_player_loaded(&self) -> bool {
        let addr = self.player_ins.get_address();
        if addr == 0 {
            return false;
        }
        read_i64(self.pid, addr as usize).unwrap_or(0) != 0
    }

    /// Get player position
    pub fn get_position(&self) -> Vector3f {
        let addr = self.sprj_chr_physics_module.get_address();
        if addr == 0 {
            return Vector3f::default();
        }
        Vector3f {
            x: read_f32(self.pid, (addr + 0x80) as usize).unwrap_or(0.0),
            y: read_f32(self.pid, (addr + 0x84) as usize).unwrap_or(0.0),
            z: read_f32(self.pid, (addr + 0x88) as usize).unwrap_or(0.0),
        }
    }

    /// Get in-game time in milliseconds
    pub fn get_in_game_time_milliseconds(&self) -> i32 {
        let addr = self.game_data_man.get_address();
        if addr == 0 {
            return 0;
        }
        read_i32(self.pid, (addr + self.igt_offset) as usize).unwrap_or(0)
    }

    /// Get character attribute value
    pub fn read_attribute(&self, attribute: Attribute) -> i32 {
        if !self.is_player_loaded() {
            return -1;
        }

        let menu_addr = self.new_menu_system.get_address();
        if menu_addr != 0 {
            let menu_state = read_i32(self.pid, menu_addr as usize).unwrap_or(0);
            if menu_state == 3 {
                return -1;
            }
        }

        let addr = self.player_game_data.get_address();
        if addr == 0 {
            return -1;
        }
        read_i32(self.pid, (addr + attribute as i64) as usize).unwrap_or(-1)
    }
}

#[cfg(target_os = "linux")]
impl Default for DarkSouls3 {
    fn default() -> Self {
        Self::new()
    }
}
