//! Armored Core 6: Fires of Rubicon autosplitter - port of SoulSplitter's ArmoredCore6.cs
//! https://github.com/FrankvdStam/SoulSplitter
//!
//! Uses CSEventFlagMan with a tree-based structure similar to Elden Ring

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HANDLE;

#[cfg(target_os = "windows")]
use crate::memory::{parse_pattern, resolve_rip_relative, scan_pattern, read_i32, read_i64};
#[cfg(target_os = "windows")]
use crate::memory::pointer::Pointer;

// AC6 patterns from SoulSplitter
#[cfg(target_os = "windows")]
pub const CS_EVENT_FLAG_MAN_PATTERN: &str = "48 8b 35 ? ? ? ? 83 f8 ff 0f 44 c1";
#[cfg(target_os = "windows")]
pub const FD4_TIME_PATTERN: &str = "48 8b 0d ? ? ? ? 0f 28 c8 f3 0f 59 0d";
#[cfg(target_os = "windows")]
pub const CS_MENU_MAN_PATTERN: &str = "48 8b 35 ? ? ? ? 33 db 89 5c 24 20";

/// Armored Core 6 autosplitter state
#[cfg(target_os = "windows")]
pub struct ArmoredCore6 {
    pub handle: HANDLE,
    // Core pointers
    pub cs_event_flag_man: Pointer,
    pub fd4_time: Pointer,
    pub cs_menu_man: Pointer,
    // Derived pointers
    pub igt: Pointer,
}

#[cfg(target_os = "windows")]
impl ArmoredCore6 {
    pub fn new() -> Self {
        Self {
            handle: HANDLE::default(),
            cs_event_flag_man: Pointer::new(),
            fd4_time: Pointer::new(),
            cs_menu_man: Pointer::new(),
            igt: Pointer::new(),
        }
    }

    /// Initialize pointers by scanning for patterns
    pub fn init_pointers(&mut self, handle: HANDLE, base: usize, size: usize) -> bool {
        self.handle = handle;

        // Scan for CSEventFlagMan
        let pattern = parse_pattern(CS_EVENT_FLAG_MAN_PATTERN);
        let cs_efm_addr = match scan_pattern(handle, base, size, &pattern) {
            Some(found) => {
                match resolve_rip_relative(handle, found, 3, 7) {
                    Some(addr) => addr,
                    None => {
                        log::warn!("AC6: Failed to resolve CSEventFlagMan RIP-relative address");
                        return false;
                    }
                }
            }
            None => {
                log::warn!("AC6: CSEventFlagMan pattern not found");
                return false;
            }
        };
        self.cs_event_flag_man.initialize(handle, true, cs_efm_addr as i64, &[0x0, 0x0]);
        log::info!("AC6: CSEventFlagMan at 0x{:X}", cs_efm_addr);

        // Scan for FD4Time (IGT)
        let pattern = parse_pattern(FD4_TIME_PATTERN);
        if let Some(found) = scan_pattern(handle, base, size, &pattern) {
            if let Some(addr) = resolve_rip_relative(handle, found, 3, 7) {
                self.fd4_time.initialize(handle, true, addr as i64, &[0x0, 0x0]);
                self.igt.initialize(handle, true, addr as i64, &[0x0, 0x0]);
                log::info!("AC6: FD4Time at 0x{:X}", addr);
            }
        }

        // Scan for CSMenuMan
        let pattern = parse_pattern(CS_MENU_MAN_PATTERN);
        if let Some(found) = scan_pattern(handle, base, size, &pattern) {
            if let Some(addr) = resolve_rip_relative(handle, found, 3, 7) {
                self.cs_menu_man.initialize(handle, true, addr as i64, &[0x0, 0x0]);
                log::info!("AC6: CSMenuMan at 0x{:X}", addr);
            }
        }

        true
    }

    /// Read event flag - port of SoulSplitter's ReadEventFlag for AC6
    /// Uses the same tree-based structure as Elden Ring
    pub fn read_event_flag(&self, event_flag_id: u32) -> bool {
        let divisor = self.cs_event_flag_man.read_i32(Some(0x1c));
        if divisor == 0 {
            return false;
        }

        let category = event_flag_id / divisor as u32;
        let least_significant_digits = event_flag_id - (category * divisor as u32);

        // Tree traversal - same as Elden Ring
        let current_element_root = self.cs_event_flag_man.create_pointer_from_address(Some(0x38));
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
            let mult = self.cs_event_flag_man.read_i32(Some(0x20));
            let elem_val = read_i32(self.handle, (current_elem_addr + 0x30) as usize).unwrap_or(0);
            let base_addr = self.cs_event_flag_man.read_i64(Some(0x28));
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
        self.igt.read_i32(Some(0x114))
    }

    /// Check if loading screen is visible
    pub fn is_loading_screen_visible(&self) -> bool {
        let addr = self.cs_menu_man.get_address();
        if addr == 0 {
            return false;
        }
        read_i32(self.handle, (addr + 0x8e4) as usize).unwrap_or(0) != 0
    }
}

#[cfg(target_os = "windows")]
impl Default for ArmoredCore6 {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Linux Implementation (for Proton/Wine)
// =============================================================================

#[cfg(target_os = "linux")]
use crate::memory::{parse_pattern, resolve_rip_relative, scan_pattern, read_i32, read_i64};
#[cfg(target_os = "linux")]
use crate::memory::pointer::Pointer;

// Memory patterns (same as Windows)
#[cfg(target_os = "linux")]
pub const CS_EVENT_FLAG_MAN_PATTERN: &str = "48 8b 35 ? ? ? ? 83 f8 ff 0f 44 c1";
#[cfg(target_os = "linux")]
pub const FD4_TIME_PATTERN: &str = "48 8b 0d ? ? ? ? 0f 28 c8 f3 0f 59 0d";
#[cfg(target_os = "linux")]
pub const CS_MENU_MAN_PATTERN: &str = "48 8b 35 ? ? ? ? 33 db 89 5c 24 20";

#[cfg(target_os = "linux")]
pub struct ArmoredCore6 {
    pub pid: i32,
    // Core pointers
    pub cs_event_flag_man: Pointer,
    pub fd4_time: Pointer,
    pub cs_menu_man: Pointer,
    // Derived pointers
    pub igt: Pointer,
}

#[cfg(target_os = "linux")]
impl ArmoredCore6 {
    pub fn new() -> Self {
        Self {
            pid: 0,
            cs_event_flag_man: Pointer::new(),
            fd4_time: Pointer::new(),
            cs_menu_man: Pointer::new(),
            igt: Pointer::new(),
        }
    }

    pub fn init_pointers(&mut self, pid: i32, base: usize, size: usize) -> bool {
        self.pid = pid;
        log::info!("AC6: Initializing pointers (Linux), base=0x{:X}, size=0x{:X}", base, size);

        // Scan for CSEventFlagMan
        let pattern = parse_pattern(CS_EVENT_FLAG_MAN_PATTERN);
        let cs_efm_addr = match scan_pattern(pid, base, size, &pattern) {
            Some(found) => {
                match resolve_rip_relative(pid, found, 3, 7) {
                    Some(addr) => addr,
                    None => {
                        log::warn!("AC6: Failed to resolve CSEventFlagMan RIP-relative address");
                        return false;
                    }
                }
            }
            None => {
                log::warn!("AC6: CSEventFlagMan pattern not found");
                return false;
            }
        };
        self.cs_event_flag_man.initialize(pid, true, cs_efm_addr as i64, &[0x0, 0x0]);
        log::info!("AC6: CSEventFlagMan at 0x{:X}", cs_efm_addr);

        // Scan for FD4Time (IGT)
        let pattern = parse_pattern(FD4_TIME_PATTERN);
        if let Some(found) = scan_pattern(pid, base, size, &pattern) {
            if let Some(addr) = resolve_rip_relative(pid, found, 3, 7) {
                self.fd4_time.initialize(pid, true, addr as i64, &[0x0, 0x0]);
                self.igt.initialize(pid, true, addr as i64, &[0x0, 0x0]);
                log::info!("AC6: FD4Time at 0x{:X}", addr);
            }
        }

        // Scan for CSMenuMan
        let pattern = parse_pattern(CS_MENU_MAN_PATTERN);
        if let Some(found) = scan_pattern(pid, base, size, &pattern) {
            if let Some(addr) = resolve_rip_relative(pid, found, 3, 7) {
                self.cs_menu_man.initialize(pid, true, addr as i64, &[0x0, 0x0]);
                log::info!("AC6: CSMenuMan at 0x{:X}", addr);
            }
        }

        true
    }

    pub fn read_event_flag(&self, event_flag_id: u32) -> bool {
        let divisor = self.cs_event_flag_man.read_i32(Some(0x1c));
        if divisor == 0 {
            return false;
        }

        let category = event_flag_id / divisor as u32;
        let least_significant_digits = event_flag_id - (category * divisor as u32);

        // Tree traversal - same as Elden Ring
        let current_element_root = self.cs_event_flag_man.create_pointer_from_address(Some(0x38));
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
            let mult = self.cs_event_flag_man.read_i32(Some(0x20));
            let elem_val = read_i32(self.pid, (current_elem_addr + 0x30) as usize).unwrap_or(0);
            let base_addr = self.cs_event_flag_man.read_i64(Some(0x28));
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
        self.igt.read_i32(Some(0x114))
    }

    pub fn is_loading_screen_visible(&self) -> bool {
        let addr = self.cs_menu_man.get_address();
        if addr == 0 {
            return false;
        }
        read_i32(self.pid, (addr + 0x8e4) as usize).unwrap_or(0) != 0
    }
}

#[cfg(target_os = "linux")]
impl Default for ArmoredCore6 {
    fn default() -> Self {
        Self::new()
    }
}
