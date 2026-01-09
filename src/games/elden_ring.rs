//! Elden Ring game implementation
//!
//! Based on SoulSplitter by FrankvdStam:
//! https://github.com/FrankvdStam/SoulSplitter
//!
//! Uses VirtualMemoryFlag with a tree-based structure for event flags.

use std::sync::Arc;

use super::{Game, Position3D, TriggerTypeInfo, AttributeInfo};
use crate::memory::{ProcessContext, MemoryReader, Pointer, parse_pattern, extract_relative_address};
use crate::AutosplitterError;

// Elden Ring memory patterns from SoulSplitter
pub const VIRTUAL_MEMORY_FLAG_PATTERN: &str = "44 89 7c 24 28 4c 8b 25 ?? ?? ?? ?? 4d 85 e4";
pub const FD4_TIME_PATTERN: &str = "48 8b 05 ?? ?? ?? ?? 4c 8b 40 08 4d 85 c0 74 0d 45 0f b6 80 be 00 00 00 e9 13 00 00 00";
pub const WORLD_CHR_MAN_PATTERN: &str = "48 8b 35 ?? ?? ?? ?? 48 85 f6 ?? ?? bb 01 00 00 00 89 5c 24 20 48 8b b6";
pub const MENU_MAN_IMP_PATTERN: &str = "48 8b 0d ?? ?? ?? ?? 48 8b 53 08 48 8b 92 d8 00 00 00 48 83 c4 20 5b";
pub const GAME_DATA_MAN_PATTERN: &str = "48 8b 05 ?? ?? ?? ?? 48 8d 4d c0 41 b8 10 00 00 00 48 8b 10 48 83 c2 1c";

/// Screen states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ScreenState {
    Unknown = -1,
    Loading = 0,
    Logo = 1,
    MainMenu = 2,
    InGame = 4,
}

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

/// Elden Ring game implementation
pub struct EldenRing {
    reader: Option<Arc<dyn MemoryReader>>,
    initialized: bool,

    // Core pointers
    virtual_memory_flag: Pointer,
    fd4_time: Pointer,
    world_chr_man: Pointer,
    menu_man_imp: Pointer,
    game_data_man: Pointer,

    // Derived pointers
    igt: Pointer,
    player_ins: Pointer,
    ng_level: Pointer,

    // Version-specific offsets
    screen_state_offset: i64,
    position_offset: i64,
    map_id_offset: i64,
    player_ins_offset: i64,
}

impl EldenRing {
    pub fn new() -> Self {
        Self {
            reader: None,
            initialized: false,
            virtual_memory_flag: Pointer::new(),
            fd4_time: Pointer::new(),
            world_chr_man: Pointer::new(),
            menu_man_imp: Pointer::new(),
            game_data_man: Pointer::new(),
            igt: Pointer::new(),
            player_ins: Pointer::new(),
            ng_level: Pointer::new(),
            screen_state_offset: 0x730,
            position_offset: 0x6d4,
            map_id_offset: 0x6d0,
            player_ins_offset: 0x1e508,
        }
    }

    fn reader(&self) -> Option<&dyn MemoryReader> {
        self.reader.as_ref().map(|r| r.as_ref())
    }

    /// Get screen state as the enum type (internal helper)
    pub fn get_screen_state_enum(&self) -> ScreenState {
        let reader = match self.reader() {
            Some(r) => r,
            None => return ScreenState::Unknown,
        };
        let addr = self.menu_man_imp.get_address(reader);
        if addr == 0 {
            return ScreenState::Unknown;
        }
        let val = reader.read_i32((addr + self.screen_state_offset) as usize).unwrap_or(-1);
        ScreenState::from(val)
    }
}

impl Default for EldenRing {
    fn default() -> Self {
        Self::new()
    }
}

impl Game for EldenRing {
    fn id(&self) -> &'static str {
        "elden-ring"
    }

    fn name(&self) -> &'static str {
        "Elden Ring"
    }

    fn process_names(&self) -> &[&'static str] {
        &["eldenring.exe"]
    }

    fn init_pointers(&mut self, ctx: &mut ProcessContext) -> Result<(), AutosplitterError> {
        log::info!("ER: Initializing pointers for base 0x{:X}, size 0x{:X}",
            ctx.base_address, ctx.module_size);

        self.reader = Some(ctx.reader());
        let reader = self.reader.as_ref().unwrap();

        // Scan for VirtualMemoryFlag
        let pattern = parse_pattern(VIRTUAL_MEMORY_FLAG_PATTERN);
        let vmf_addr = ctx.scan_pattern(&pattern)
            .ok_or_else(|| AutosplitterError::PatternScanFailed(
                "VirtualMemoryFlag pattern not found".to_string()
            ))?;

        let vmf_resolved = extract_relative_address(reader.as_ref(), vmf_addr, 8, 7)
            .ok_or_else(|| AutosplitterError::PatternScanFailed(
                "Failed to resolve VirtualMemoryFlag RIP-relative address".to_string()
            ))?;

        self.virtual_memory_flag.initialize(ctx.is_64_bit, vmf_resolved as i64, &[0x5]);
        log::info!("ER: VirtualMemoryFlag at 0x{:X}", vmf_resolved);

        // Scan for FD4Time (IGT)
        let pattern = parse_pattern(FD4_TIME_PATTERN);
        if let Some(found) = ctx.scan_pattern(&pattern) {
            if let Some(addr) = extract_relative_address(reader.as_ref(), found, 3, 7) {
                self.fd4_time.initialize(ctx.is_64_bit, addr as i64, &[0x0]);
                self.igt.initialize(ctx.is_64_bit, addr as i64, &[0x0, 0xa0]);
                log::info!("ER: FD4Time at 0x{:X}", addr);
            }
        }

        // Scan for WorldChrMan
        let pattern = parse_pattern(WORLD_CHR_MAN_PATTERN);
        if let Some(found) = ctx.scan_pattern(&pattern) {
            if let Some(addr) = extract_relative_address(reader.as_ref(), found, 3, 7) {
                self.world_chr_man.initialize(ctx.is_64_bit, addr as i64, &[0x0]);
                self.player_ins.initialize(ctx.is_64_bit, addr as i64, &[0x0, self.player_ins_offset]);
                log::info!("ER: WorldChrMan at 0x{:X}", addr);
            }
        }

        // Scan for MenuManImp
        let pattern = parse_pattern(MENU_MAN_IMP_PATTERN);
        if let Some(found) = ctx.scan_pattern(&pattern) {
            if let Some(addr) = extract_relative_address(reader.as_ref(), found, 3, 7) {
                self.menu_man_imp.initialize(ctx.is_64_bit, addr as i64, &[0x0]);
                log::info!("ER: MenuManImp at 0x{:X}", addr);
            }
        }

        // Scan for GameDataMan
        let pattern = parse_pattern(GAME_DATA_MAN_PATTERN);
        if let Some(found) = ctx.scan_pattern(&pattern) {
            if let Some(addr) = extract_relative_address(reader.as_ref(), found, 3, 7) {
                self.game_data_man.initialize(ctx.is_64_bit, addr as i64, &[0x0]);
                self.ng_level.initialize(ctx.is_64_bit, addr as i64, &[0x0, 0x120]);
                log::info!("ER: GameDataMan at 0x{:X}", addr);
            }
        }

        self.initialized = true;
        log::info!("ER: All pointers initialized successfully");
        Ok(())
    }

    fn read_event_flag(&self, event_flag_id: u32) -> bool {
        if !self.initialized {
            return false;
        }

        let reader = match self.reader() {
            Some(r) => r,
            None => return false,
        };

        let divisor = self.virtual_memory_flag.read_i32(reader, Some(0x1c));
        if divisor == 0 {
            return false;
        }

        let category = event_flag_id / divisor as u32;
        let least_significant_digits = event_flag_id - (category * divisor as u32);

        // Tree traversal for flag lookup
        let current_element_root = self.virtual_memory_flag.create_pointer_from_address(reader, Some(0x38));
        let mut current_element = current_element_root.copy();
        let mut current_sub_element = current_element.create_pointer_from_address(reader, Some(0x8));

        while current_sub_element.read_byte(reader, Some(0x19)) == 0 {
            if (current_sub_element.read_i32(reader, Some(0x20)) as u32) < category {
                current_sub_element = current_sub_element.create_pointer_from_address(reader, Some(0x10));
            } else {
                current_element = current_sub_element.copy();
                current_sub_element = current_sub_element.create_pointer_from_address(reader, Some(0x0));
            }
        }

        let current_elem_addr = current_element.get_address(reader);
        let sub_elem_addr = current_sub_element.get_address(reader);

        if current_elem_addr == sub_elem_addr || category < (current_element.read_i32(reader, Some(0x20)) as u32) {
            current_element = current_sub_element.copy();
        }

        let current_elem_addr = current_element.get_address(reader);
        let sub_elem_addr = current_sub_element.get_address(reader);

        if current_elem_addr == sub_elem_addr {
            return false;
        }

        let mystery_value = reader.read_i32((current_elem_addr + 0x28) as usize).unwrap_or(0) - 1;

        let calculated_pointer: i64;
        if mystery_value == 0 {
            let mult = self.virtual_memory_flag.read_i32(reader, Some(0x20));
            let elem_val = reader.read_i32((current_elem_addr + 0x30) as usize).unwrap_or(0);
            let base_addr = self.virtual_memory_flag.read_i64(reader, Some(0x28));
            calculated_pointer = (mult as i64 * elem_val as i64) + base_addr;
        } else if mystery_value == 1 {
            return false;
        } else {
            calculated_pointer = reader.read_i64((current_elem_addr + 0x30) as usize).unwrap_or(0);
        }

        if calculated_pointer == 0 {
            return false;
        }

        let thing = 7 - (least_significant_digits & 7);
        let mask = 1i32 << thing;
        let shifted = least_significant_digits >> 3;

        let final_addr = (calculated_pointer + shifted as i64) as usize;
        if let Some(read_value) = reader.read_i32(final_addr) {
            return (read_value & mask) != 0;
        }

        false
    }

    fn is_alive(&self) -> bool {
        self.initialized
    }

    fn get_igt_milliseconds(&self) -> Option<i32> {
        if !self.initialized {
            return None;
        }
        let reader = self.reader()?;
        Some(self.igt.read_i32(reader, None))
    }

    fn get_position(&self) -> Option<Position3D> {
        if !self.initialized {
            return None;
        }
        let reader = self.reader()?;
        let addr = self.player_ins.get_address(reader);
        if addr == 0 {
            return None;
        }

        Some(Position3D {
            x: reader.read_f32((addr + self.position_offset) as usize).unwrap_or(0.0),
            y: reader.read_f32((addr + self.position_offset + 4) as usize).unwrap_or(0.0),
            z: reader.read_f32((addr + self.position_offset + 8) as usize).unwrap_or(0.0),
        })
    }

    fn is_loading(&self) -> Option<bool> {
        if !self.initialized {
            return None;
        }
        Some(self.get_screen_state_enum() == ScreenState::Loading)
    }

    fn is_player_loaded(&self) -> Option<bool> {
        if !self.initialized {
            return None;
        }
        let reader = self.reader()?;
        let addr = self.player_ins.get_address(reader);
        if addr == 0 {
            return None;
        }
        Some(reader.read_i64(addr as usize).unwrap_or(0) != 0)
    }

    fn is_blackscreen(&self) -> Option<bool> {
        if !self.initialized {
            return None;
        }
        let reader = self.reader()?;

        let screen_state = self.get_screen_state_enum();
        if screen_state != ScreenState::InGame {
            return Some(false);
        }

        let addr = self.menu_man_imp.get_address(reader);
        if addr == 0 {
            return None;
        }

        let flag = reader.read_u32((addr + 0x18) as usize).unwrap_or(0);
        let bit0 = (flag & 0x1) != 0;
        let bit8 = (flag & 0x100) != 0;
        let bit16 = (flag & 0x10000) != 0;

        Some(bit0 && !bit8 && bit16)
    }

    fn get_screen_state(&self) -> Option<i32> {
        if !self.initialized {
            return None;
        }
        Some(self.get_screen_state_enum() as i32)
    }

    fn get_ng_level(&self) -> Option<i32> {
        if !self.initialized {
            return None;
        }
        let reader = self.reader()?;
        Some(self.ng_level.read_i32(reader, None))
    }

    fn supported_triggers(&self) -> Vec<TriggerTypeInfo> {
        vec![
            TriggerTypeInfo {
                id: "event_flag".to_string(),
                name: "Event Flag".to_string(),
                description: "Triggers when an event flag is set".to_string(),
            },
            TriggerTypeInfo {
                id: "position".to_string(),
                name: "Position".to_string(),
                description: "Triggers when player enters an area".to_string(),
            },
            TriggerTypeInfo {
                id: "loading".to_string(),
                name: "Loading State".to_string(),
                description: "Triggers on loading screen transitions".to_string(),
            },
        ]
    }

    fn available_attributes(&self) -> Vec<AttributeInfo> {
        vec![
            AttributeInfo { id: "vigor".to_string(), name: "Vigor".to_string() },
            AttributeInfo { id: "mind".to_string(), name: "Mind".to_string() },
            AttributeInfo { id: "endurance".to_string(), name: "Endurance".to_string() },
            AttributeInfo { id: "strength".to_string(), name: "Strength".to_string() },
            AttributeInfo { id: "dexterity".to_string(), name: "Dexterity".to_string() },
            AttributeInfo { id: "intelligence".to_string(), name: "Intelligence".to_string() },
            AttributeInfo { id: "faith".to_string(), name: "Faith".to_string() },
            AttributeInfo { id: "arcane".to_string(), name: "Arcane".to_string() },
        ]
    }
}

/// Factory for creating EldenRing instances
pub struct EldenRingFactory;

impl crate::games::GameFactory for EldenRingFactory {
    fn game_id(&self) -> &'static str {
        "elden-ring"
    }

    fn process_names(&self) -> &[&'static str] {
        &["eldenring.exe"]
    }

    fn create(&self) -> crate::games::BoxedGame {
        Box::new(EldenRing::new())
    }
}
