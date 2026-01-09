//! Sekiro: Shadows Die Twice game implementation
//!
//! Based on SoulSplitter by FrankvdStam:
//! https://github.com/FrankvdStam/SoulSplitter
//!
//! Very similar to Dark Souls 3 with different offsets (0x18 instead of 0x10, 0xb0 instead of 0x70)

use std::sync::Arc;

use super::{Game, Position3D, TriggerTypeInfo, AttributeInfo};
use crate::memory::{ProcessContext, MemoryReader, Pointer, parse_pattern, extract_relative_address};
use crate::AutosplitterError;

// Sekiro patterns from SoulSplitter
pub const EVENT_FLAG_MAN_PATTERN: &str = "48 8b 0d ?? ?? ?? ?? 48 89 5c 24 50 48 89 6c 24 58 48 89 74 24 60";
pub const FIELD_AREA_PATTERN: &str = "48 8b 0d ?? ?? ?? ?? 48 85 c9 74 26 44 8b 41 28 48 8d 54 24 40";
pub const WORLD_CHR_MAN_PATTERN: &str = "48 8B 35 ?? ?? ?? ?? 44 0F 28 18";
pub const IGT_PATTERN: &str = "48 8b 05 ?? ?? ?? ?? 32 d2 48 8b 48";
pub const FADE_MAN_IMP_PATTERN: &str = "48 89 35 ?? ?? ?? ?? 48 8b c7 48 8b";
pub const PLAYER_GAME_DATA_PATTERN: &str = "48 8b 0d ?? ?? ?? ?? 48 8b 41 20 c6";

/// Character attributes for Sekiro
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i64)]
pub enum Attribute {
    Vitality = 0x44,
    AttackPower = 0x48,
}

/// Sekiro game implementation
pub struct Sekiro {
    reader: Option<Arc<dyn MemoryReader>>,
    initialized: bool,

    // Core pointers
    event_flag_man: Pointer,
    field_area: Pointer,
    world_chr_man: Pointer,
    igt: Pointer,
    fade_man_imp: Pointer,
    player_game_data: Pointer,

    // Derived pointers
    player_pos: Pointer,
    fade_system: Pointer,
}

impl Sekiro {
    pub fn new() -> Self {
        Self {
            reader: None,
            initialized: false,
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

    fn reader(&self) -> Option<&dyn MemoryReader> {
        self.reader.as_ref().map(|r| r.as_ref())
    }
}

impl Default for Sekiro {
    fn default() -> Self {
        Self::new()
    }
}

impl Game for Sekiro {
    fn id(&self) -> &'static str {
        "sekiro"
    }

    fn name(&self) -> &'static str {
        "Sekiro: Shadows Die Twice"
    }

    fn process_names(&self) -> &[&'static str] {
        &["sekiro.exe"]
    }

    fn init_pointers(&mut self, ctx: &mut ProcessContext) -> Result<(), AutosplitterError> {
        log::info!("Sekiro: Initializing pointers for base 0x{:X}, size 0x{:X}",
            ctx.base_address, ctx.module_size);

        self.reader = Some(ctx.reader());
        let reader = self.reader.as_ref().unwrap();

        // Scan for EventFlagMan
        let pattern = parse_pattern(EVENT_FLAG_MAN_PATTERN);
        let efm_addr = ctx.scan_pattern(&pattern)
            .ok_or_else(|| AutosplitterError::PatternScanFailed(
                "EventFlagMan pattern not found".to_string()
            ))?;

        let efm_resolved = extract_relative_address(reader.as_ref(), efm_addr, 3, 7)
            .ok_or_else(|| AutosplitterError::PatternScanFailed(
                "Failed to resolve EventFlagMan RIP-relative address".to_string()
            ))?;

        self.event_flag_man.initialize(ctx.is_64_bit, efm_resolved as i64, &[0x0]);
        log::info!("Sekiro: EventFlagMan at 0x{:X}", efm_resolved);

        // Scan for FieldArea
        let pattern = parse_pattern(FIELD_AREA_PATTERN);
        if let Some(found) = ctx.scan_pattern(&pattern) {
            if let Some(addr) = extract_relative_address(reader.as_ref(), found, 3, 7) {
                self.field_area.initialize(ctx.is_64_bit, addr as i64, &[]);
                log::info!("Sekiro: FieldArea at 0x{:X}", addr);
            }
        }

        // Scan for WorldChrMan
        let pattern = parse_pattern(WORLD_CHR_MAN_PATTERN);
        if let Some(found) = ctx.scan_pattern(&pattern) {
            if let Some(addr) = extract_relative_address(reader.as_ref(), found, 3, 7) {
                self.world_chr_man.initialize(ctx.is_64_bit, addr as i64, &[0x0]);
                self.player_pos.initialize(ctx.is_64_bit, addr as i64, &[0x0, 0x48, 0x28]);
                log::info!("Sekiro: WorldChrMan at 0x{:X}", addr);
            }
        }

        // Scan for IGT
        let pattern = parse_pattern(IGT_PATTERN);
        if let Some(found) = ctx.scan_pattern(&pattern) {
            if let Some(addr) = extract_relative_address(reader.as_ref(), found, 3, 7) {
                self.igt.initialize(ctx.is_64_bit, addr as i64, &[0x0, 0x9c]);
                log::info!("Sekiro: IGT at 0x{:X}", addr);
            }
        }

        // Scan for FadeManImp
        let pattern = parse_pattern(FADE_MAN_IMP_PATTERN);
        if let Some(found) = ctx.scan_pattern(&pattern) {
            if let Some(addr) = extract_relative_address(reader.as_ref(), found, 3, 7) {
                self.fade_man_imp.initialize(ctx.is_64_bit, addr as i64, &[0x0]);
                self.fade_system.initialize(ctx.is_64_bit, addr as i64, &[0x0, 0x8]);
                log::info!("Sekiro: FadeManImp at 0x{:X}", addr);
            }
        }

        // Scan for PlayerGameData
        let pattern = parse_pattern(PLAYER_GAME_DATA_PATTERN);
        if let Some(found) = ctx.scan_pattern(&pattern) {
            if let Some(addr) = extract_relative_address(reader.as_ref(), found, 3, 7) {
                self.player_game_data.initialize(ctx.is_64_bit, addr as i64, &[0x0, 0x8]);
                log::info!("Sekiro: PlayerGameData at 0x{:X}", addr);
            }
        }

        self.initialized = true;
        log::info!("Sekiro: All pointers initialized successfully");
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

            // Sekiro uses 0x18 offset instead of DS3's 0x10
            let world_info_owner = self.field_area.append(&[0x18]).create_pointer_from_address(reader, None);
            let size = world_info_owner.read_i32(reader, Some(0x8));
            let vector = world_info_owner.append(&[0x10]);

            for i in 0..size {
                let area = vector.read_byte(reader, Some((i as i64 * 0x38) + 0xb)) as i32;

                if area == event_flag_area {
                    let count = vector.read_byte(reader, Some(i as i64 * 0x38 + 0x20));
                    let mut index = 0i64;
                    let mut found = false;
                    let mut world_info_block_vector: Option<Pointer> = None;

                    if count >= 1 {
                        loop {
                            let block_vec = vector.create_pointer_from_address(reader, Some(i as i64 * 0x38 + 0x28));
                            // Sekiro uses 0xb0 stride instead of DS3's 0x70
                            let flag = block_vec.read_i32(reader, Some((index * 0xb0) + 0x8));

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
                            flag_world_block_info_category = block_vec.read_i32(reader, Some((index * 0xb0) + 0x20));
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

            let result = value & mask;
            return result != 0;
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
        let addr = self.player_pos.get_address(reader);
        if addr == 0 {
            return None;
        }

        Some(Position3D {
            x: reader.read_f32((addr + 0x80) as usize).unwrap_or(0.0),
            y: reader.read_f32((addr + 0x84) as usize).unwrap_or(0.0),
            z: reader.read_f32((addr + 0x88) as usize).unwrap_or(0.0),
        })
    }

    fn is_player_loaded(&self) -> Option<bool> {
        if !self.initialized {
            return None;
        }
        let reader = self.reader()?;
        let addr = self.world_chr_man.get_address(reader);
        if addr == 0 {
            return None;
        }
        Some(reader.read_i64((addr + 0x88) as usize).unwrap_or(0) != 0)
    }

    fn is_blackscreen(&self) -> Option<bool> {
        if !self.initialized {
            return None;
        }
        let reader = self.reader()?;
        let addr = self.fade_system.get_address(reader);
        if addr == 0 {
            return None;
        }
        Some(reader.read_i32((addr + 0x2dc) as usize).unwrap_or(0) != 0)
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
            "attack_power" | "attack" | "atk" => Attribute::AttackPower,
            _ => return None,
        };

        reader.read_i32((addr + attribute as i64) as usize)
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
            AttributeInfo { id: "attack_power".to_string(), name: "Attack Power".to_string() },
        ]
    }
}

/// Factory for creating Sekiro instances
pub struct SekiroFactory;

impl crate::games::GameFactory for SekiroFactory {
    fn game_id(&self) -> &'static str {
        "sekiro"
    }

    fn process_names(&self) -> &[&'static str] {
        &["sekiro.exe"]
    }

    fn create(&self) -> crate::games::BoxedGame {
        Box::new(Sekiro::new())
    }
}
