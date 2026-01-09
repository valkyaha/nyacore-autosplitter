//! Armored Core 6: Fires of Rubicon game implementation
//!
//! Based on SoulSplitter by FrankvdStam:
//! https://github.com/FrankvdStam/SoulSplitter
//!
//! Uses CSEventFlagMan with a tree-based structure similar to Elden Ring

use std::sync::Arc;

use super::{Game, Position3D, TriggerTypeInfo, AttributeInfo};
use crate::memory::{ProcessContext, MemoryReader, Pointer, parse_pattern, extract_relative_address};
use crate::AutosplitterError;

// AC6 patterns from SoulSplitter
pub const CS_EVENT_FLAG_MAN_PATTERN: &str = "48 8b 35 ?? ?? ?? ?? 83 f8 ff 0f 44 c1";
pub const FD4_TIME_PATTERN: &str = "48 8b 0d ?? ?? ?? ?? 0f 28 c8 f3 0f 59 0d";
pub const CS_MENU_MAN_PATTERN: &str = "48 8b 35 ?? ?? ?? ?? 33 db 89 5c 24 20";

/// Armored Core 6 game implementation
pub struct ArmoredCore6 {
    reader: Option<Arc<dyn MemoryReader>>,
    initialized: bool,

    // Core pointers
    cs_event_flag_man: Pointer,
    fd4_time: Pointer,
    cs_menu_man: Pointer,

    // Derived pointers
    igt: Pointer,
}

impl ArmoredCore6 {
    pub fn new() -> Self {
        Self {
            reader: None,
            initialized: false,
            cs_event_flag_man: Pointer::new(),
            fd4_time: Pointer::new(),
            cs_menu_man: Pointer::new(),
            igt: Pointer::new(),
        }
    }

    fn reader(&self) -> Option<&dyn MemoryReader> {
        self.reader.as_ref().map(|r| r.as_ref())
    }
}

impl Default for ArmoredCore6 {
    fn default() -> Self {
        Self::new()
    }
}

impl Game for ArmoredCore6 {
    fn id(&self) -> &'static str {
        "armored-core-6"
    }

    fn name(&self) -> &'static str {
        "Armored Core VI: Fires of Rubicon"
    }

    fn process_names(&self) -> &[&'static str] {
        &["armoredcore6.exe"]
    }

    fn init_pointers(&mut self, ctx: &mut ProcessContext) -> Result<(), AutosplitterError> {
        log::info!("AC6: Initializing pointers for base 0x{:X}, size 0x{:X}",
            ctx.base_address, ctx.module_size);

        self.reader = Some(ctx.reader());
        let reader = self.reader.as_ref().unwrap();

        // Scan for CSEventFlagMan
        let pattern = parse_pattern(CS_EVENT_FLAG_MAN_PATTERN);
        let cs_efm_addr = ctx.scan_pattern(&pattern)
            .ok_or_else(|| AutosplitterError::PatternScanFailed(
                "CSEventFlagMan pattern not found".to_string()
            ))?;

        let cs_efm_resolved = extract_relative_address(reader.as_ref(), cs_efm_addr, 3, 7)
            .ok_or_else(|| AutosplitterError::PatternScanFailed(
                "Failed to resolve CSEventFlagMan RIP-relative address".to_string()
            ))?;

        self.cs_event_flag_man.initialize(ctx.is_64_bit, cs_efm_resolved as i64, &[0x0, 0x0]);
        log::info!("AC6: CSEventFlagMan at 0x{:X}", cs_efm_resolved);

        // Scan for FD4Time (IGT)
        let pattern = parse_pattern(FD4_TIME_PATTERN);
        if let Some(found) = ctx.scan_pattern(&pattern) {
            if let Some(addr) = extract_relative_address(reader.as_ref(), found, 3, 7) {
                self.fd4_time.initialize(ctx.is_64_bit, addr as i64, &[0x0, 0x0]);
                self.igt.initialize(ctx.is_64_bit, addr as i64, &[0x0, 0x0]);
                log::info!("AC6: FD4Time at 0x{:X}", addr);
            }
        }

        // Scan for CSMenuMan
        let pattern = parse_pattern(CS_MENU_MAN_PATTERN);
        if let Some(found) = ctx.scan_pattern(&pattern) {
            if let Some(addr) = extract_relative_address(reader.as_ref(), found, 3, 7) {
                self.cs_menu_man.initialize(ctx.is_64_bit, addr as i64, &[0x0, 0x0]);
                log::info!("AC6: CSMenuMan at 0x{:X}", addr);
            }
        }

        self.initialized = true;
        log::info!("AC6: All pointers initialized successfully");
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

        let divisor = self.cs_event_flag_man.read_i32(reader, Some(0x1c));
        if divisor == 0 {
            return false;
        }

        let category = event_flag_id / divisor as u32;
        let least_significant_digits = event_flag_id - (category * divisor as u32);

        // Tree traversal - same as Elden Ring
        let current_element_root = self.cs_event_flag_man.create_pointer_from_address(reader, Some(0x38));
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
            let mult = self.cs_event_flag_man.read_i32(reader, Some(0x20));
            let elem_val = reader.read_i32((current_elem_addr + 0x30) as usize).unwrap_or(0);
            let base_addr = self.cs_event_flag_man.read_i64(reader, Some(0x28));
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
        Some(self.igt.read_i32(reader, Some(0x114)))
    }

    fn is_loading(&self) -> Option<bool> {
        if !self.initialized {
            return None;
        }
        let reader = self.reader()?;
        let addr = self.cs_menu_man.get_address(reader);
        if addr == 0 {
            return None;
        }
        Some(reader.read_i32((addr + 0x8e4) as usize).unwrap_or(0) != 0)
    }

    fn get_position(&self) -> Option<Position3D> {
        // AC6 position reading not implemented yet
        None
    }

    fn supported_triggers(&self) -> Vec<TriggerTypeInfo> {
        vec![
            TriggerTypeInfo {
                id: "event_flag".to_string(),
                name: "Event Flag".to_string(),
                description: "Triggers when an event flag is set".to_string(),
            },
            TriggerTypeInfo {
                id: "mission_complete".to_string(),
                name: "Mission Complete".to_string(),
                description: "Triggers when a mission is completed".to_string(),
            },
        ]
    }

    fn available_attributes(&self) -> Vec<AttributeInfo> {
        vec![] // AC6 doesn't have character attributes
    }
}

/// Factory for creating ArmoredCore6 instances
pub struct ArmoredCore6Factory;

impl crate::games::GameFactory for ArmoredCore6Factory {
    fn game_id(&self) -> &'static str {
        "armored-core-6"
    }

    fn process_names(&self) -> &[&'static str] {
        &["armoredcore6.exe"]
    }

    fn create(&self) -> crate::games::BoxedGame {
        Box::new(ArmoredCore6::new())
    }
}
