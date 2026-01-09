//! Dark Souls III game implementation
//!
//! Based on SoulSplitter by FrankvdStam:
//! https://github.com/FrankvdStam/SoulSplitter
//!
//! Uses Category Decomposition algorithm for event flag reading.

use std::sync::Arc;

use super::{
    Game, GameFactory, BoxedGame, Position3D, TriggerTypeInfo, AttributeInfo,
    common::{standard_event_flag_trigger, standard_position_trigger, standard_loading_trigger, standard_igt_trigger},
};
use crate::memory::{ProcessContext, MemoryReader, Pointer, parse_pattern, extract_relative_address};
use crate::AutosplitterError;

// =============================================================================
// CONSTANTS
// =============================================================================

/// Game metadata
pub const GAME_ID: &str = "dark-souls-3";
pub const GAME_NAME: &str = "Dark Souls III";
pub const PROCESS_NAMES: &[&str] = &["DarkSoulsIII.exe"];

/// Memory patterns from SoulSplitter
pub const SPRJ_EVENT_FLAG_MAN_PATTERN: &str = "48 c7 05 ?? ?? ?? ?? 00 00 00 00 48 8b 7c 24 38 c7 46 54 ff ff ff ff 48 83 c4 20 5e c3";
pub const FIELD_AREA_PATTERN: &str = "4c 8b 3d ?? ?? ?? ?? 8b 45 87 83 f8 ff 74 69 48 8d 4d 8f 48 89 4d 9f 89 45 8f 48 8d 55 8f 49 8b 4f 10";
pub const NEW_MENU_SYSTEM_PATTERN: &str = "48 8b 0d ?? ?? ?? ?? 48 8b 7c 24 20 48 8b 5c 24 30 48 85 c9";
pub const GAME_DATA_MAN_PATTERN: &str = "48 8b 0d ?? ?? ?? ?? 4c 8d 44 24 40 45 33 c9 48 8b d3 40 88";
pub const PLAYER_INS_PATTERN: &str = "48 8b 0d ?? ?? ?? ?? 45 33 c0 48 8d 55 e7 e8 ?? ?? ?? ?? 0f 2f";
pub const LOADING_PATTERN: &str = "c6 05 ?? ?? ?? ?? ?? e8 ?? ?? ?? ?? 84 c0 0f 94 c0 e9";
pub const SPRJ_FADE_IMP_PATTERN: &str = "48 8b 0d ?? ?? ?? ?? 4c 8d 4c 24 38 4c 8d 44 24 48 33 d2";

// =============================================================================
// ATTRIBUTES
// =============================================================================

/// Character attributes for DS3
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

// =============================================================================
// GAME IMPLEMENTATION
// =============================================================================

/// Dark Souls III game implementation
pub struct DarkSouls3 {
    // Core state
    reader: Option<Arc<dyn MemoryReader>>,
    initialized: bool,

    // Memory pointers
    sprj_event_flag_man: Pointer,
    field_area: Pointer,
    new_menu_system: Pointer,
    game_data_man: Pointer,
    player_ins: Pointer,
    loading: Pointer,
    sprj_fade_imp: Pointer,

    // Derived pointers
    player_game_data: Pointer,
    sprj_chr_physics_module: Pointer,
    blackscreen: Pointer,

    // Version-specific offsets
    igt_offset: i64,
}

impl DarkSouls3 {
    pub fn new() -> Self {
        Self {
            reader: None,
            initialized: false,
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
            igt_offset: 0xa4, // 0x9c for older versions
        }
    }

    /// Get the memory reader if available
    fn reader(&self) -> Option<&dyn MemoryReader> {
        self.reader.as_ref().map(|r| r.as_ref())
    }

    /// Read event flag using Category Decomposition algorithm
    fn read_flag_category_decomposition(&self, reader: &dyn MemoryReader, event_flag_id: u32) -> bool {
        let event_flag_id_div_10000000 = ((event_flag_id / 10_000_000) % 10) as i64;
        let event_flag_area = ((event_flag_id / 100_000) % 100) as i32;
        let event_flag_id_div_10000 = ((event_flag_id / 10_000) % 10) as i32;
        let event_flag_id_div_1000 = ((event_flag_id / 1_000) % 10) as i64;

        let mut flag_world_block_info_category: i32 = -1;

        if event_flag_area >= 90 || event_flag_area + event_flag_id_div_10000 == 0 {
            flag_world_block_info_category = 0;
        } else {
            if self.field_area.is_null_ptr(reader) {
                return false;
            }

            let world_info_owner = self.field_area.append(&[0x0, 0x10]).create_pointer_from_address(reader, None);
            let size = world_info_owner.read_i32(reader, Some(0x8));
            let vector = world_info_owner.append(&[0x10]);

            for i in 0..size {
                let area = vector.read_byte(reader, Some((i as i64 * 0x38) + 0xb)) as i32;

                if area == event_flag_area {
                    let count = vector.read_byte(reader, Some(i as i64 * 0x38 + 0x20));
                    let mut index = 0;
                    let mut found = false;
                    let mut world_info_block_vector: Option<Pointer> = None;

                    if count >= 1 {
                        loop {
                            let block_vec = vector.create_pointer_from_address(reader, Some(i as i64 * 0x38 + 0x28));
                            let flag = block_vec.read_i32(reader, Some((index * 0x70) + 0x8));

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
                            flag_world_block_info_category = block_vec.read_i32(reader, Some((index * 0x70) + 0x20));
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

        if ptr.is_null_ptr(reader) || flag_world_block_info_category < 0 {
            return false;
        }

        let result_base = (event_flag_id_div_1000 << 4)
            + ptr.get_address(reader)
            + (flag_world_block_info_category as i64 * 0xa8);

        let mut result_pointer_address = Pointer::new();
        result_pointer_address.initialize(true, result_base, &[0x0]);

        if !result_pointer_address.is_null_ptr(reader) {
            let mod_1000 = (event_flag_id % 1000) as u32;
            let read_offset = ((mod_1000 >> 5) * 4) as i64;
            let value = result_pointer_address.read_u32(reader, Some(read_offset));

            let bit_shift = 0x1f - ((mod_1000 as u8) & 0x1f);
            let mask = 1u32 << (bit_shift & 0x1f);

            return (value & mask) != 0;
        }

        false
    }
}

impl Default for DarkSouls3 {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// GAME TRAIT IMPLEMENTATION
// =============================================================================

impl Game for DarkSouls3 {
    fn id(&self) -> &'static str {
        GAME_ID
    }

    fn name(&self) -> &'static str {
        GAME_NAME
    }

    fn process_names(&self) -> &[&'static str] {
        PROCESS_NAMES
    }

    fn init_pointers(&mut self, ctx: &mut ProcessContext) -> Result<(), AutosplitterError> {
        log::info!("DS3: Initializing pointers for base 0x{:X}, size 0x{:X}",
            ctx.base_address, ctx.module_size);

        self.reader = Some(ctx.reader());
        let reader = self.reader.as_ref().unwrap();

        // SprjEventFlagMan (required)
        let pattern = parse_pattern(SPRJ_EVENT_FLAG_MAN_PATTERN);
        let sprj_addr = ctx.scan_pattern(&pattern)
            .ok_or_else(|| AutosplitterError::PatternScanFailed(
                "SprjEventFlagMan pattern not found".to_string()
            ))?;

        let sprj_resolved = extract_relative_address(reader.as_ref(), sprj_addr, 3, 11)
            .ok_or_else(|| AutosplitterError::PatternScanFailed(
                "Failed to resolve SprjEventFlagMan RIP-relative address".to_string()
            ))?;

        self.sprj_event_flag_man.initialize(ctx.is_64_bit, sprj_resolved as i64, &[0x0]);
        log::info!("DS3: SprjEventFlagMan at 0x{:X}", sprj_resolved);

        // FieldArea (optional)
        let pattern = parse_pattern(FIELD_AREA_PATTERN);
        if let Some(found) = ctx.scan_pattern(&pattern) {
            if let Some(addr) = extract_relative_address(reader.as_ref(), found, 3, 7) {
                self.field_area.initialize(ctx.is_64_bit, addr as i64, &[]);
                log::info!("DS3: FieldArea at 0x{:X}", addr);
            }
        }

        // NewMenuSystem (optional)
        let pattern = parse_pattern(NEW_MENU_SYSTEM_PATTERN);
        if let Some(found) = ctx.scan_pattern(&pattern) {
            if let Some(addr) = extract_relative_address(reader.as_ref(), found, 3, 7) {
                self.new_menu_system.initialize(ctx.is_64_bit, addr as i64, &[0x0]);
                log::info!("DS3: NewMenuSystem at 0x{:X}", addr);
            }
        }

        // GameDataMan (optional)
        let pattern = parse_pattern(GAME_DATA_MAN_PATTERN);
        if let Some(found) = ctx.scan_pattern(&pattern) {
            if let Some(addr) = extract_relative_address(reader.as_ref(), found, 3, 7) {
                self.game_data_man.initialize(ctx.is_64_bit, addr as i64, &[0x0]);
                self.player_game_data.initialize(ctx.is_64_bit, addr as i64, &[0x0, 0x10]);
                log::info!("DS3: GameDataMan at 0x{:X}", addr);
            }
        }

        // PlayerIns (optional)
        let pattern = parse_pattern(PLAYER_INS_PATTERN);
        if let Some(found) = ctx.scan_pattern(&pattern) {
            if let Some(addr) = extract_relative_address(reader.as_ref(), found, 3, 7) {
                self.player_ins.initialize(ctx.is_64_bit, addr as i64, &[0x0]);
                self.sprj_chr_physics_module.initialize(ctx.is_64_bit, addr as i64, &[0x0, 0x80, 0x40, 0x28]);
                log::info!("DS3: PlayerIns at 0x{:X}", addr);
            }
        }

        // Loading (optional)
        let pattern = parse_pattern(LOADING_PATTERN);
        if let Some(found) = ctx.scan_pattern(&pattern) {
            if let Some(addr) = extract_relative_address(reader.as_ref(), found, 2, 7) {
                self.loading.initialize(ctx.is_64_bit, addr as i64, &[]);
                log::info!("DS3: Loading at 0x{:X}", addr);
            }
        }

        // SprjFadeImp / Blackscreen (optional)
        let pattern = parse_pattern(SPRJ_FADE_IMP_PATTERN);
        if let Some(found) = ctx.scan_pattern(&pattern) {
            if let Some(addr) = extract_relative_address(reader.as_ref(), found, 3, 7) {
                self.sprj_fade_imp.initialize(ctx.is_64_bit, addr as i64, &[0x0]);
                self.blackscreen.initialize(ctx.is_64_bit, addr as i64, &[0x0, 0x8]);
                log::info!("DS3: SprjFadeImp at 0x{:X}", addr);
            }
        }

        self.initialized = true;
        log::info!("DS3: All pointers initialized successfully");
        Ok(())
    }

    fn read_event_flag(&self, event_flag_id: u32) -> bool {
        if !self.initialized {
            return false;
        }
        match self.reader() {
            Some(reader) => self.read_flag_category_decomposition(reader, event_flag_id),
            None => false,
        }
    }

    fn is_alive(&self) -> bool {
        self.initialized
    }

    fn get_igt_milliseconds(&self) -> Option<i32> {
        if !self.initialized {
            return None;
        }
        let reader = self.reader()?;
        let addr = self.game_data_man.get_address(reader);
        if addr == 0 {
            return None;
        }
        reader.read_i32((addr + self.igt_offset) as usize)
    }

    fn get_position(&self) -> Option<Position3D> {
        if !self.initialized {
            return None;
        }
        let reader = self.reader()?;
        let addr = self.sprj_chr_physics_module.get_address(reader);
        if addr == 0 {
            return None;
        }
        Some(Position3D {
            x: reader.read_f32((addr + 0x80) as usize).unwrap_or(0.0),
            y: reader.read_f32((addr + 0x84) as usize).unwrap_or(0.0),
            z: reader.read_f32((addr + 0x88) as usize).unwrap_or(0.0),
        })
    }

    fn is_loading(&self) -> Option<bool> {
        if !self.initialized {
            return None;
        }
        let reader = self.reader()?;
        let addr = self.loading.get_address(reader);
        if addr == 0 {
            return None;
        }
        Some(reader.read_i32((addr - 1) as usize).unwrap_or(0) != 0)
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
        let addr = self.blackscreen.get_address(reader);
        if addr == 0 {
            return None;
        }
        Some(reader.read_i32((addr + 0x2ec) as usize).unwrap_or(0) != 0)
    }

    fn get_attribute(&self, attr: &str) -> Option<i32> {
        if !self.initialized {
            return None;
        }
        let reader = self.reader()?;

        if !self.is_player_loaded().unwrap_or(false) {
            return None;
        }

        let menu_addr = self.new_menu_system.get_address(reader);
        if menu_addr != 0 {
            let menu_state = reader.read_i32(menu_addr as usize).unwrap_or(0);
            if menu_state == 3 {
                return None;
            }
        }

        let attribute = match attr.to_lowercase().as_str() {
            "vigor" | "vig" => Attribute::Vigor,
            "attunement" | "att" => Attribute::Attunement,
            "endurance" | "end" => Attribute::Endurance,
            "vitality" | "vit" => Attribute::Vitality,
            "strength" | "str" => Attribute::Strength,
            "dexterity" | "dex" => Attribute::Dexterity,
            "intelligence" | "int" => Attribute::Intelligence,
            "faith" | "fai" => Attribute::Faith,
            "luck" | "lck" => Attribute::Luck,
            "soul_level" | "sl" | "level" => Attribute::SoulLevel,
            _ => return None,
        };

        let addr = self.player_game_data.get_address(reader);
        if addr == 0 {
            return None;
        }
        reader.read_i32((addr + attribute as i64) as usize)
    }

    fn supported_triggers(&self) -> Vec<TriggerTypeInfo> {
        vec![
            standard_event_flag_trigger(),
            standard_position_trigger(),
            standard_loading_trigger(),
            standard_igt_trigger(),
        ]
    }

    fn available_attributes(&self) -> Vec<AttributeInfo> {
        vec![
            AttributeInfo { id: "vigor".to_string(), name: "Vigor".to_string() },
            AttributeInfo { id: "attunement".to_string(), name: "Attunement".to_string() },
            AttributeInfo { id: "endurance".to_string(), name: "Endurance".to_string() },
            AttributeInfo { id: "vitality".to_string(), name: "Vitality".to_string() },
            AttributeInfo { id: "strength".to_string(), name: "Strength".to_string() },
            AttributeInfo { id: "dexterity".to_string(), name: "Dexterity".to_string() },
            AttributeInfo { id: "intelligence".to_string(), name: "Intelligence".to_string() },
            AttributeInfo { id: "faith".to_string(), name: "Faith".to_string() },
            AttributeInfo { id: "luck".to_string(), name: "Luck".to_string() },
            AttributeInfo { id: "soul_level".to_string(), name: "Soul Level".to_string() },
        ]
    }
}

// =============================================================================
// FACTORY
// =============================================================================

pub struct DarkSouls3Factory;

impl GameFactory for DarkSouls3Factory {
    fn game_id(&self) -> &'static str {
        GAME_ID
    }

    fn process_names(&self) -> &[&'static str] {
        PROCESS_NAMES
    }

    fn create(&self) -> BoxedGame {
        Box::new(DarkSouls3::new())
    }
}
