//! Dark Souls 1 (Remastered) game implementation
//!
//! Based on SoulSplitter by FrankvdStam:
//! https://github.com/FrankvdStam/SoulSplitter
//!
//! Credit to JKAnderson for the original event flag reading code (DSR-Gadget)
//!
//! DS1 uses an offset table approach for event flags.

use std::collections::HashMap;
use std::sync::Arc;

use super::{Game, Position3D, TriggerTypeInfo, AttributeInfo};
use crate::memory::{ProcessContext, MemoryReader, Pointer, parse_pattern, extract_relative_address};
use crate::AutosplitterError;

// DS1 Remastered patterns from SoulSplitter
pub const EVENT_FLAGS_PATTERN: &str = "48 8B 0D ?? ?? ?? ?? 99 33 C2 45 33 C0 2B C2 8D 50 F6";
pub const GAME_DATA_MAN_PATTERN: &str = "48 8b 05 ?? ?? ?? ?? 48 8b 50 10 48 89 54 24 60";
pub const GAME_MAN_PATTERN: &str = "48 8b 05 ?? ?? ?? ?? c6 40 18 00";
pub const WORLD_CHR_MAN_PATTERN: &str = "48 8b 0d ?? ?? ?? ?? 0f 28 f1 48 85 c9 74 ?? 48 89 7c";
pub const MENU_MAN_PATTERN: &str = "48 8b 15 ?? ?? ?? ?? 89 82 7c 08 00 00";

/// Character attributes for DS1
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

/// Dark Souls 1 Remastered game implementation
pub struct DarkSouls1 {
    reader: Option<Arc<dyn MemoryReader>>,
    initialized: bool,

    // Core pointers
    event_flags: Pointer,
    game_data_man: Pointer,
    game_man: Pointer,
    world_chr_man: Pointer,
    menu_man: Pointer,

    // Derived pointers
    player_game_data: Pointer,
    player_ins: Pointer,
    player_pos: Pointer,

    // Offset maps for event flags
    event_flag_groups: HashMap<char, i32>,
    event_flag_areas: HashMap<&'static str, i32>,

    // Version-specific offsets
    player_ctrl_offset: i64,
}

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
            reader: None,
            initialized: false,
            event_flags: Pointer::new(),
            game_data_man: Pointer::new(),
            game_man: Pointer::new(),
            world_chr_man: Pointer::new(),
            menu_man: Pointer::new(),
            player_game_data: Pointer::new(),
            player_ins: Pointer::new(),
            player_pos: Pointer::new(),
            event_flag_groups,
            event_flag_areas,
            player_ctrl_offset: 0x68,
        }
    }

    fn reader(&self) -> Option<&dyn MemoryReader> {
        self.reader.as_ref().map(|r| r.as_ref())
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
}

impl Default for DarkSouls1 {
    fn default() -> Self {
        Self::new()
    }
}

impl Game for DarkSouls1 {
    fn id(&self) -> &'static str {
        "dark-souls-1"
    }

    fn name(&self) -> &'static str {
        "Dark Souls: Remastered"
    }

    fn process_names(&self) -> &[&'static str] {
        &["DarkSoulsRemastered.exe", "DARKSOULS.exe"]
    }

    fn init_pointers(&mut self, ctx: &mut ProcessContext) -> Result<(), AutosplitterError> {
        log::info!("DS1R: Initializing pointers for base 0x{:X}, size 0x{:X}",
            ctx.base_address, ctx.module_size);

        self.reader = Some(ctx.reader());
        let reader = self.reader.as_ref().unwrap();

        // Scan for EventFlags
        let pattern = parse_pattern(EVENT_FLAGS_PATTERN);
        let event_flags_addr = ctx.scan_pattern(&pattern)
            .ok_or_else(|| AutosplitterError::PatternScanFailed(
                "EventFlags pattern not found".to_string()
            ))?;

        let event_flags_resolved = extract_relative_address(reader.as_ref(), event_flags_addr, 3, 7)
            .ok_or_else(|| AutosplitterError::PatternScanFailed(
                "Failed to resolve EventFlags RIP-relative address".to_string()
            ))?;

        // DSProcess does TWO dereferences: *(*eventFlagPtr + 0) + 0
        self.event_flags.initialize(ctx.is_64_bit, event_flags_resolved as i64, &[0x0, 0x0, 0x0]);
        log::info!("DS1R: EventFlags at 0x{:X}", event_flags_resolved);

        // Scan for GameDataMan
        let pattern = parse_pattern(GAME_DATA_MAN_PATTERN);
        if let Some(found) = ctx.scan_pattern(&pattern) {
            if let Some(addr) = extract_relative_address(reader.as_ref(), found, 3, 7) {
                self.game_data_man.initialize(ctx.is_64_bit, addr as i64, &[0x0]);
                self.player_game_data.initialize(ctx.is_64_bit, addr as i64, &[0x0, 0x10]);
                log::info!("DS1R: GameDataMan at 0x{:X}", addr);
            }
        }

        // Scan for GameMan
        let pattern = parse_pattern(GAME_MAN_PATTERN);
        if let Some(found) = ctx.scan_pattern(&pattern) {
            if let Some(addr) = extract_relative_address(reader.as_ref(), found, 3, 7) {
                self.game_man.initialize(ctx.is_64_bit, addr as i64, &[0x0]);
                log::info!("DS1R: GameMan at 0x{:X}", addr);
            }
        }

        // Scan for WorldChrMan
        let pattern = parse_pattern(WORLD_CHR_MAN_PATTERN);
        if let Some(found) = ctx.scan_pattern(&pattern) {
            if let Some(addr) = extract_relative_address(reader.as_ref(), found, 3, 7) {
                self.world_chr_man.initialize(ctx.is_64_bit, addr as i64, &[0x0]);
                self.player_ins.initialize(ctx.is_64_bit, addr as i64, &[0x0, self.player_ctrl_offset]);
                self.player_pos.initialize(ctx.is_64_bit, addr as i64, &[0x0, self.player_ctrl_offset, 0x28]);
                log::info!("DS1R: WorldChrMan at 0x{:X}", addr);
            }
        }

        // Scan for MenuMan
        let pattern = parse_pattern(MENU_MAN_PATTERN);
        if let Some(found) = ctx.scan_pattern(&pattern) {
            if let Some(addr) = extract_relative_address(reader.as_ref(), found, 3, 7) {
                self.menu_man.initialize(ctx.is_64_bit, addr as i64, &[0x0]);
                log::info!("DS1R: MenuMan at 0x{:X}", addr);
            }
        }

        self.initialized = true;
        log::info!("DS1R: All pointers initialized successfully");
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

        if let Some((offset, mask)) = self.get_event_flag_offset(event_flag_id) {
            let address = self.event_flags.get_address(reader);
            if address == 0 {
                return false;
            }

            let read_addr = (address + offset as i64) as usize;
            if let Some(value) = reader.read_u32(read_addr) {
                return (value & mask) != 0;
            }
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
        let addr = self.game_data_man.get_address(reader);
        if addr == 0 {
            return None;
        }
        reader.read_i32((addr + 0xa4) as usize)
    }

    fn get_position(&self) -> Option<Position3D> {
        if !self.initialized {
            return None;
        }
        let reader = self.reader()?;
        let addr = self.player_pos.get_address(reader);
        if addr == 0 {
            return None;
        }

        Some(Position3D {
            x: reader.read_f32((addr + 0x10) as usize).unwrap_or(0.0),
            y: reader.read_f32((addr + 0x14) as usize).unwrap_or(0.0),
            z: reader.read_f32((addr + 0x18) as usize).unwrap_or(0.0),
        })
    }

    fn is_player_loaded(&self) -> Option<bool> {
        if !self.initialized {
            return None;
        }
        let reader = self.reader()?;
        Some(!self.player_ins.is_null_ptr(reader))
    }

    fn get_attribute(&self, attr: &str) -> Option<i32> {
        if !self.initialized {
            return None;
        }
        let reader = self.reader()?;
        let addr = self.player_game_data.get_address(reader);
        if addr == 0 {
            return None;
        }

        let attribute = match attr.to_lowercase().as_str() {
            "vitality" | "vit" => Attribute::Vitality,
            "attunement" | "att" => Attribute::Attunement,
            "endurance" | "end" => Attribute::Endurance,
            "strength" | "str" => Attribute::Strength,
            "dexterity" | "dex" => Attribute::Dexterity,
            "resistance" | "res" => Attribute::Resistance,
            "intelligence" | "int" => Attribute::Intelligence,
            "faith" | "fai" => Attribute::Faith,
            "humanity" | "hum" => Attribute::Humanity,
            "soul_level" | "sl" | "level" => Attribute::SoulLevel,
            _ => return None,
        };

        reader.read_i32((addr + 0x8 + attribute as i64) as usize)
    }

    fn get_ng_level(&self) -> Option<i32> {
        if !self.initialized {
            return None;
        }
        let reader = self.reader()?;
        let addr = self.game_data_man.get_address(reader);
        if addr == 0 {
            return None;
        }
        reader.read_i32((addr + 0x78) as usize)
    }

    fn get_player_health(&self) -> Option<i32> {
        if !self.initialized {
            return None;
        }
        let reader = self.reader()?;
        let addr = self.player_ins.get_address(reader);
        if addr == 0 {
            return None;
        }
        reader.read_i32((addr + 0x2d4) as usize)
    }

    fn is_warp_requested(&self) -> Option<bool> {
        if !self.initialized {
            return None;
        }
        let reader = self.reader()?;
        let addr = self.game_man.get_address(reader);
        if addr == 0 {
            return None;
        }
        let warp_flag = reader.read_i32((addr + 0x18) as usize).unwrap_or(0);
        Some(warp_flag == 1)
    }

    fn are_credits_rolling(&self) -> Option<bool> {
        if !self.initialized {
            return None;
        }
        let reader = self.reader()?;
        let addr = self.menu_man.get_address(reader);
        if addr == 0 {
            return None;
        }
        let state = reader.read_i32((addr + 0x87c) as usize).unwrap_or(0);
        Some(state == 5)
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
        ]
    }

    fn available_attributes(&self) -> Vec<AttributeInfo> {
        vec![
            AttributeInfo { id: "vitality".to_string(), name: "Vitality".to_string() },
            AttributeInfo { id: "attunement".to_string(), name: "Attunement".to_string() },
            AttributeInfo { id: "endurance".to_string(), name: "Endurance".to_string() },
            AttributeInfo { id: "strength".to_string(), name: "Strength".to_string() },
            AttributeInfo { id: "dexterity".to_string(), name: "Dexterity".to_string() },
            AttributeInfo { id: "resistance".to_string(), name: "Resistance".to_string() },
            AttributeInfo { id: "intelligence".to_string(), name: "Intelligence".to_string() },
            AttributeInfo { id: "faith".to_string(), name: "Faith".to_string() },
        ]
    }
}

/// Factory for creating DarkSouls1 instances
pub struct DarkSouls1Factory;

impl crate::games::GameFactory for DarkSouls1Factory {
    fn game_id(&self) -> &'static str {
        "dark-souls-1"
    }

    fn process_names(&self) -> &[&'static str] {
        &["DarkSoulsRemastered.exe", "DARKSOULS.exe"]
    }

    fn create(&self) -> crate::games::BoxedGame {
        Box::new(DarkSouls1::new())
    }
}
