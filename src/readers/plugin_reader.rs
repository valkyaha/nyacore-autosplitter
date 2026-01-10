//! Generic plugin-based FlagReader implementation
//!
//! This reader uses algorithm configuration from plugin.toml to read boss defeat flags.
//! Supports four algorithm types:
//! - category_decomposition: DS3/Sekiro style with FieldArea traversal
//! - binary_tree: Elden Ring style tree traversal
//! - offset_table: DS1 style group/area offset tables
//! - kill_counter: DS2 style kill counter arrays
//!
//! Based on SoulSplitter by FrankvdStam (https://github.com/FrankvdStam/SoulSplitter)

#[cfg(target_os = "windows")]
use std::collections::HashMap;
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HANDLE;

#[cfg(target_os = "windows")]
use crate::readers::flag_reader::{FlagReader, MemoryContext};
#[cfg(target_os = "windows")]
use crate::memory::{
    parse_pattern, read_i32, read_ptr, read_u32, read_u64, read_u8, resolve_rip_relative, scan_pattern,
};
#[cfg(target_os = "windows")]
use crate::config::{AutosplitterMemoryConfig, PatternConfig};

/// Generic plugin-based flag reader
#[cfg(target_os = "windows")]
pub struct PluginFlagReader {
    config: AutosplitterMemoryConfig,
    /// Set to true after first successful/failed flag check to avoid spamming logs
    debug_logged: std::sync::atomic::AtomicBool,
}

#[cfg(target_os = "windows")]
impl PluginFlagReader {
    pub fn new(config: AutosplitterMemoryConfig) -> Self {
        Self {
            config,
            debug_logged: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Get a pattern config by name
    fn get_pattern(&self, name: &str) -> Option<&PatternConfig> {
        self.config.patterns.iter().find(|p| p.name == name)
    }

    /// Scan a single pattern and resolve its RIP-relative address
    fn scan_single_pattern(
        &self,
        handle: HANDLE,
        base: usize,
        size: usize,
        pattern_name: &str,
        default_rip_offset: usize,
        default_instr_len: usize,
    ) -> Option<usize> {
        let pattern_config = self.get_pattern(pattern_name)?;

        let pattern_str = &pattern_config.pattern;
        let rip_offset = pattern_config.rip_offset;
        let instr_len = pattern_config.instruction_len;

        let parsed = parse_pattern(pattern_str);
        let found = scan_pattern(handle, base, size, &parsed)?;

        // Use configured values, falling back to defaults
        let actual_rip = if rip_offset > 0 { rip_offset } else { default_rip_offset };
        let actual_len = if instr_len > 0 { instr_len } else { default_instr_len };

        resolve_rip_relative(handle, found, actual_rip, actual_len)
    }

    /// Apply a chain of pointer offsets to resolve a final address
    /// Each offset (except the last) is dereferenced
    fn apply_pointer_offsets(&self, handle: HANDLE, base_addr: usize, offsets: &[i64]) -> Option<usize> {
        if offsets.is_empty() {
            return Some(base_addr);
        }

        let mut current = base_addr;

        // Dereference all offsets except the last one
        for (i, &offset) in offsets.iter().enumerate() {
            // Add the offset to current address
            let addr = if offset >= 0 {
                current.wrapping_add(offset as usize)
            } else {
                current.wrapping_sub((-offset) as usize)
            };

            // If this is not the last offset, dereference it
            if i < offsets.len() - 1 {
                current = read_ptr(handle, addr)?;
                if current == 0 {
                    return None;
                }
            } else {
                // Last offset - just add, don't dereference
                current = addr;
            }
        }

        Some(current)
    }
}

#[cfg(target_os = "windows")]
impl FlagReader for PluginFlagReader {
    fn algorithm_name(&self) -> &'static str {
        match self.config.algorithm.as_str() {
            "category_decomposition" => "category_decomposition",
            "binary_tree" => "binary_tree",
            "offset_table" => "offset_table",
            "kill_counter" => "kill_counter",
            _ => "unknown",
        }
    }

    fn scan_patterns(
        &self,
        handle: HANDLE,
        base: usize,
        size: usize,
        _patterns: &[PatternConfig],
    ) -> Option<HashMap<String, usize>> {
        let mut result = HashMap::new();

        // First, scan all patterns defined in config and store their resolved addresses
        for pattern in &self.config.patterns {
            if let Some(addr) = self.scan_single_pattern(
                handle, base, size,
                &pattern.name,
                pattern.rip_offset,
                pattern.instruction_len,
            ) {
                log::info!("Pattern '{}' found at 0x{:X}", pattern.name, addr);

                // Apply pointer_offsets if specified to get final address
                let final_addr = if !pattern.pointer_offsets.is_empty() {
                    self.apply_pointer_offsets(handle, addr, &pattern.pointer_offsets)
                        .unwrap_or(addr)
                } else {
                    addr
                };

                if final_addr != addr {
                    log::info!("Pattern '{}' after pointer offsets: 0x{:X}", pattern.name, final_addr);
                }

                result.insert(pattern.name.clone(), final_addr);
            }
        }

        // Now resolve derived pointers from [autosplitter.pointers] section
        for (name, derived) in &self.config.pointers {
            if let Some(&base_addr) = result.get(&derived.base) {
                // Apply the offset chain to the base pattern address
                if let Some(final_addr) = self.apply_pointer_offsets(handle, base_addr, &derived.offsets) {
                    log::info!("Derived pointer '{}' (from {}) = 0x{:X}", name, derived.base, final_addr);
                    result.insert(name.clone(), final_addr);
                }
            } else {
                log::warn!("Derived pointer '{}' references unknown base pattern '{}'", name, derived.base);
            }
        }

        // Validate algorithm-specific required patterns
        match self.config.algorithm.as_str() {
            "category_decomposition" => {
                if let Some(cfg) = self.config.category_config.as_ref() {
                    if !result.contains_key(&cfg.primary_pattern) {
                        log::error!("category_decomposition: primary pattern '{}' not found", cfg.primary_pattern);
                        return None;
                    }
                }
            }
            "binary_tree" => {
                if let Some(cfg) = self.config.tree_config.as_ref() {
                    if !result.contains_key(&cfg.primary_pattern) {
                        log::error!("binary_tree: primary pattern '{}' not found", cfg.primary_pattern);
                        return None;
                    }
                }
            }
            "offset_table" => {
                if let Some(cfg) = self.config.offset_table_config.as_ref() {
                    if !result.contains_key(&cfg.primary_pattern) {
                        log::error!("offset_table: primary pattern '{}' not found", cfg.primary_pattern);
                        return None;
                    }
                }
            }
            "kill_counter" => {
                if let Some(cfg) = self.config.kill_counter_config.as_ref() {
                    if !result.contains_key(&cfg.primary_pattern) {
                        log::error!("kill_counter: primary pattern '{}' not found", cfg.primary_pattern);
                        return None;
                    }
                }
            }
            _ => {
                log::error!("Unknown algorithm: {}", self.config.algorithm);
                return None;
            }
        }

        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }

    fn is_flag_set(&self, ctx: &MemoryContext, flag_id: u32) -> bool {
        match self.config.algorithm.as_str() {
            "category_decomposition" => self.is_flag_set_category(ctx, flag_id),
            "binary_tree" => self.is_flag_set_binary_tree(ctx, flag_id),
            "offset_table" => self.is_flag_set_offset_table(ctx, flag_id),
            "kill_counter" => self.is_flag_set_kill_counter(ctx, flag_id),
            _ => false,
        }
    }
}

// =============================================================================
// ALGORITHM IMPLEMENTATIONS
// Based on SoulSplitter by FrankvdStam
// =============================================================================

#[cfg(target_os = "windows")]
impl PluginFlagReader {
    /// Category decomposition algorithm (DS3/Sekiro style)
    /// Ported from SoulSplitter's ReadEventFlag implementation
    fn is_flag_set_category(&self, ctx: &MemoryContext, flag_id: u32) -> bool {
        use std::sync::atomic::Ordering;

        // Log detailed info on first check only
        let should_log = !self.debug_logged.load(Ordering::Relaxed);

        let cfg = match self.config.category_config.as_ref() {
            Some(c) => c,
            None => {
                if should_log {
                    log::warn!("Flag {}: No category_config", flag_id);
                    self.debug_logged.store(true, Ordering::Relaxed);
                }
                return false;
            }
        };

        let event_flag_man_ptr = match ctx.pointers.get(&cfg.primary_pattern) {
            Some(&addr) => addr,
            None => {
                if should_log {
                    log::warn!("Flag {}: No primary pattern pointer '{}'", flag_id, cfg.primary_pattern);
                    self.debug_logged.store(true, Ordering::Relaxed);
                }
                return false;
            }
        };

        // Read the base pointer (SprjEventFlagMan)
        let event_flag_man = match read_ptr(ctx.handle, event_flag_man_ptr) {
            Some(p) if p != 0 => p,
            _ => {
                if should_log {
                    log::warn!("Flag {}: EventFlagMan null at 0x{:X}", flag_id, event_flag_man_ptr);
                    self.debug_logged.store(true, Ordering::Relaxed);
                }
                return false;
            }
        };

        if should_log {
            log::info!("Flag {}: EventFlagMan ptr=0x{:X} -> value=0x{:X}", flag_id, event_flag_man_ptr, event_flag_man);

            // Check validity flag at offset 0x228 (DS3 native code checks this)
            if let Some(validity) = read_u8(ctx.handle, event_flag_man + 0x228) {
                log::info!("Flag {}: Validity byte at EventFlagMan+0x228 = 0x{:02X}", flag_id, validity);
            }

            // Also read raw bytes at ptr1 location to diagnose
            if let Some(raw) = read_u64(ctx.handle, event_flag_man + cfg.base_offset) {
                log::info!("Flag {}: Raw 8 bytes at EventFlagMan+0x{:X} = 0x{:016X}", flag_id, cfg.base_offset, raw);
            }
        }

        // Decompose flag ID into parts
        let event_flag_div_10000000 = ((flag_id / 10_000_000) % 10) as usize;
        let event_flag_area = ((flag_id / 100_000) % 100) as i32;
        let event_flag_div_10000 = ((flag_id / 10_000) % 10) as i32;
        let event_flag_div_1000 = ((flag_id / 1_000) % 10) as usize;

        if should_log {
            log::info!(
                "Flag {} decomposed: div10m={}, area={}, div10k={}, div1k={}",
                flag_id, event_flag_div_10000000, event_flag_area, event_flag_div_10000, event_flag_div_1000
            );
        }

        // Determine flag category
        let flag_category = self.get_flag_category(
            ctx,
            event_flag_area,
            event_flag_div_10000,
            &cfg.secondary_pattern,
            should_log,
        );

        // If category lookup failed, we cannot safely determine the flag state
        // Do NOT try all categories as this causes false positives
        if flag_category < 0 {
            if should_log {
                log::warn!("Flag {}: Category lookup failed, cannot determine flag state", flag_id);
                self.debug_logged.store(true, Ordering::Relaxed);
            }
            return false;
        }

        if should_log {
            log::info!("Flag {}: Category = {}", flag_id, flag_category);
        }

        // Navigate pointer chain: SoulSplitter does _sprjEventFlagMan.Append(0x218, div10m * 0x18, 0x0)
        // With chain [0x218, div10m * 0x18, 0x0]:
        //   - ptr = [base + 0x218] (deref)
        //   - ptr = [ptr + div10m * 0x18] (deref)
        //   - ptr = ptr + 0x0 (no deref, last)
        // So ptr.GetAddress() = [[EventFlagMan + 0x218] + div10m * 0x18]

        // First dereference: [EventFlagMan + 0x218]
        let ptr1 = match read_ptr(ctx.handle, event_flag_man + cfg.base_offset) {
            Some(p) if p != 0 => p,
            _ => {
                if should_log {
                    log::warn!("Flag {}: ptr1 null at EventFlagMan(0x{:X})+0x{:X}", flag_id, event_flag_man, cfg.base_offset);
                    self.debug_logged.store(true, Ordering::Relaxed);
                }
                return false;
            }
        };

        // Second dereference: [[EventFlagMan + 0x218] + div10m * 0x18]
        let entry_base_addr = match read_ptr(ctx.handle, ptr1 + (event_flag_div_10000000 * cfg.entry_size)) {
            Some(p) if p != 0 => p,
            _ => {
                if should_log {
                    log::warn!("Flag {}: ptr2 null at ptr1(0x{:X})+0x{:X}", flag_id, ptr1, event_flag_div_10000000 * cfg.entry_size);
                    self.debug_logged.store(true, Ordering::Relaxed);
                }
                return false;
            }
        };

        if should_log {
            log::info!(
                "Flag {}: Chain: [0x{:X}+0x{:X}]=0x{:X} -> [0x{:X}+0x{:X}]=0x{:X}",
                flag_id,
                event_flag_man, cfg.base_offset, ptr1,
                ptr1, event_flag_div_10000000 * cfg.entry_size, entry_base_addr
            );
        }

        // Calculate bit position
        let mod_1000 = flag_id % 1000;
        let byte_offset = ((mod_1000 >> 5) * 4) as usize;
        let bit_index = 0x1f - ((mod_1000 as u8) & 0x1f);
        let mask = 1u32 << bit_index;

        // Calculate final address using category
        // SoulSplitter formula: (div1k << 4) + ptr.GetAddress() + (category * 0xa8)
        // ptr.GetAddress() = entry_base_addr (the address, not a dereferenced value)
        let result_addr = (event_flag_div_1000 << 4) + entry_base_addr + ((flag_category as usize) * cfg.category_multiplier);

        // Read the flag value directly at result_addr + byte_offset
        if let Some(value) = read_u32(ctx.handle, result_addr + byte_offset) {
            let is_set = (value & mask) != 0;
            if should_log {
                log::info!(
                    "Flag {}: addr=0x{:X}, value=0x{:X}, mask=0x{:X}, is_set={}",
                    flag_id, result_addr + byte_offset, value, mask, is_set
                );
                self.debug_logged.store(true, Ordering::Relaxed);
            }
            return is_set;
        }

        if should_log {
            log::warn!("Flag {}: Failed to read flag value at 0x{:X}", flag_id, result_addr + byte_offset);
            self.debug_logged.store(true, Ordering::Relaxed);
        }
        false
    }

    /// Get the flag category by traversing FieldArea structure
    /// Ported from SoulSplitter
    fn get_flag_category(
        &self,
        ctx: &MemoryContext,
        event_flag_area: i32,
        event_flag_div_10000: i32,
        field_area_pattern: &Option<String>,
        should_log: bool,
    ) -> i32 {
        // If area >= 90 or area + div10000 == 0, category is 0
        if event_flag_area >= 90 || (event_flag_area + event_flag_div_10000 == 0) {
            if should_log {
                log::info!("Category: Using default 0 (area={}, div10k={})", event_flag_area, event_flag_div_10000);
            }
            return 0;
        }

        // Need FieldArea to determine category
        let field_area_ptr = match field_area_pattern {
            Some(pattern) => match ctx.pointers.get(pattern) {
                Some(&addr) => addr,
                None => {
                    if should_log {
                        log::warn!("Category: FieldArea pattern '{}' not found in pointers", pattern);
                    }
                    return -1;
                }
            },
            None => {
                if should_log {
                    log::warn!("Category: No secondary pattern configured");
                }
                return -1;
            }
        };

        // Get world info owner via pointer chain
        // DS3: The pattern points to a global -> [global] is FieldArea static -> [[global]+0x0] is actual instance
        // Chain: [pattern_result] -> [[pattern_result] + base_offset] -> add world_info_offset
        let cfg = self.config.category_config.as_ref().unwrap();
        let field_area_base_offset = cfg.field_area_base_offset.unwrap_or(0);
        let world_info_offset = cfg.world_info_offset.unwrap_or(0x10);

        if should_log {
            log::info!(
                "Category: field_area_ptr=0x{:X}, base_offset=0x{:X}, world_info_offset=0x{:X}",
                field_area_ptr, field_area_base_offset, world_info_offset
            );
        }

        // SoulSplitter chain for DS3: FieldArea.Append(0x0, 0x10).CreatePointerFromAddress()
        // SoulSplitter's Pointer.ResolveOffsets: each offset is ADDED then DEREFERENCED (except last)
        // With chain [0x0, 0x10, 0x0] (trailing 0 from CreatePointerFromAddress):
        //   - ptr = [base + 0x0] = [field_area_ptr]
        //   - ptr = [ptr + 0x10] = [[field_area_ptr] + 0x10]
        //   - ptr = ptr + 0x0 = [[field_area_ptr] + 0x10] (no deref, just add)
        // Result: WorldInfoOwner = [[field_area_ptr] + 0x10] (only TWO dereferences!)

        // First dereference: [field_area_ptr + base_offset]
        let field_area = match read_ptr(ctx.handle, field_area_ptr + field_area_base_offset) {
            Some(p) => {
                if should_log {
                    log::info!("Category: [field_area_ptr+0x{:X}] = 0x{:X}", field_area_base_offset, p);
                }
                if p == 0 {
                    if should_log {
                        log::warn!("Category: FieldArea not initialized yet (null)");
                    }
                    return -1;
                }
                p
            }
            _ => {
                if should_log {
                    log::warn!("Category: Failed to read at field_area_ptr+0x{:X}", field_area_base_offset);
                }
                return -1;
            }
        };

        // Second dereference: [field_area + world_info_offset] = WorldInfoOwner
        let world_info_owner = match read_ptr(ctx.handle, field_area + world_info_offset) {
            Some(p) => {
                if should_log {
                    log::info!("Category: [[field_area_ptr+0x{:X}]+0x{:X}] = WorldInfoOwner = 0x{:X}",
                        field_area_base_offset, world_info_offset, p);
                }
                if p == 0 {
                    if should_log {
                        log::warn!("Category: WorldInfoOwner not initialized (null)");
                    }
                    return -1;
                }
                p
            }
            _ => {
                if should_log {
                    log::warn!("Category: Failed to read WorldInfoOwner at 0x{:X}", field_area + world_info_offset);
                }
                return -1;
            }
        };

        // Read size of world info array at world_info_owner + 0x8
        let size = match read_i32(ctx.handle, world_info_owner + 0x8) {
            Some(s) if s > 0 && s < 1000 => s as usize, // Sanity check: size should be reasonable
            Some(s) => {
                if should_log {
                    log::warn!("Category: WorldInfo size {} at 0x{:X}+0x8 is invalid (too large or negative)", s, world_info_owner);
                }
                return -1;
            }
            _ => {
                if should_log {
                    log::warn!("Category: WorldInfo size read failed at 0x{:X}+0x8", world_info_owner);
                }
                return -1;
            }
        };

        // Read pointer to vector of world info structs
        // SoulSplitter: var vector = worldInfoOwner.Append(0x10);
        // When vector.ReadByte(...) is called, offset 0x10 is dereferenced first
        // So the vector base is at [worldInfoOwner + 0x10]
        let vector = match read_ptr(ctx.handle, world_info_owner + 0x10) {
            Some(v) if v != 0 => v,
            _ => {
                if should_log {
                    log::warn!("Category: WorldInfo vector null at 0x{:X}+0x10", world_info_owner);
                }
                return -1;
            }
        };

        // Struct sizes differ between games
        let world_info_size = cfg.world_info_struct_size.unwrap_or(0x38);
        let world_block_size = cfg.world_block_struct_size.unwrap_or(0x70);

        if should_log {
            log::info!("Category: Searching {} world info entries for area={}", size, event_flag_area);
        }

        // Loop over worldInfo structs
        for i in 0..size {
            let area = read_u8(ctx.handle, vector + (i * world_info_size) + 0xb).unwrap_or(0) as i32;

            if area == event_flag_area {
                let count = read_u8(ctx.handle, vector + (i * world_info_size) + 0x20).unwrap_or(0) as usize;

                if should_log {
                    log::info!("Category: Found area {} at index {}, {} blocks", area, i, count);
                }

                if count >= 1 {
                    // Get pointer to worldBlockInfo vector
                    let block_vector_ptr = vector + (i * world_info_size) + 0x28;
                    let block_vector = match read_ptr(ctx.handle, block_vector_ptr) {
                        Some(p) if p != 0 => p,
                        _ => continue,
                    };

                    // Loop over worldBlockInfo structs
                    for index in 0..count {
                        let flag = read_i32(ctx.handle, block_vector + (index * world_block_size) + 0x8).unwrap_or(0);

                        // Check if this block matches our event flag
                        if ((flag >> 0x10) & 0xff) == event_flag_div_10000 && (flag >> 0x18) == event_flag_area {
                            // Found! Read the category
                            let category = read_i32(ctx.handle, block_vector + (index * world_block_size) + 0x20).unwrap_or(-1);
                            if should_log {
                                log::info!("Category: Found matching block, category={}", category);
                            }
                            if category >= 0 {
                                return category + 1; // Category is 1-indexed
                            }
                        }
                    }
                }
            }
        }

        if should_log {
            log::warn!("Category: No matching world block found for area={}, div10k={}", event_flag_area, event_flag_div_10000);
        }
        -1 // Not found
    }

    /// Binary tree algorithm (Elden Ring style)
    /// Ported from SoulSplitter
    fn is_flag_set_binary_tree(&self, ctx: &MemoryContext, flag_id: u32) -> bool {
        let cfg = match self.config.tree_config.as_ref() {
            Some(c) => c,
            None => return false,
        };

        // Get tree layout offsets from memory_layout.event_flag_tree if available
        let tree_layout = self.config.memory_layout.event_flag_tree.as_ref();
        let first_sub_element = tree_layout.and_then(|t| t.first_sub_element).unwrap_or(0x8);
        let left_child = tree_layout.and_then(|t| t.left_child).unwrap_or(0x0);
        let right_child = tree_layout.and_then(|t| t.right_child).unwrap_or(0x10);
        let leaf_check_offset = tree_layout.and_then(|t| t.leaf_check_offset).unwrap_or(0x19);
        let category_offset = tree_layout.and_then(|t| t.category_offset).unwrap_or(0x20);
        let mystery_value_offset = tree_layout.and_then(|t| t.mystery_value_offset).unwrap_or(0x28);
        let element_value_offset = tree_layout.and_then(|t| t.element_value_offset).unwrap_or(0x30);

        let vmf_ptr = match ctx.pointers.get(&cfg.primary_pattern) {
            Some(&addr) => addr,
            None => return false,
        };

        // Read VirtualMemoryFlag pointer with offset chain
        let mut vmf = match read_ptr(ctx.handle, vmf_ptr) {
            Some(p) if p != 0 => p,
            _ => return false,
        };

        // Apply pointer chain offsets if specified
        for &offset in &cfg.pointer_chain {
            vmf = match read_ptr(ctx.handle, vmf + offset) {
                Some(p) if p != 0 => p,
                _ => return false,
            };
        }

        // Read divisor
        let divisor = match read_i32(ctx.handle, vmf + cfg.divisor_offset) {
            Some(d) if d > 0 => d as u32,
            _ => return false,
        };

        let category = flag_id / divisor;
        let remainder = flag_id - (category * divisor);

        // Tree traversal
        let root = match read_ptr(ctx.handle, vmf + cfg.tree_root_offset) {
            Some(p) if p != 0 => p,
            _ => return false,
        };

        let mut current = match read_ptr(ctx.handle, root + first_sub_element) {
            Some(p) if p != 0 => p,
            _ => return false,
        };

        let mut result_node = current;

        // Navigate tree (max 128 iterations)
        for _ in 0..128 {
            let marker = read_u8(ctx.handle, current + leaf_check_offset).unwrap_or(1);
            if marker != 0 {
                break;
            }

            let node_category = read_u32(ctx.handle, current + category_offset).unwrap_or(0);
            if node_category < category {
                current = match read_ptr(ctx.handle, current + right_child) {
                    Some(p) if p != 0 => p,
                    _ => break,
                };
            } else {
                result_node = current;
                current = match read_ptr(ctx.handle, current + left_child) {
                    Some(p) if p != 0 => p,
                    _ => break,
                };
            }
        }

        // Check category match
        let found_category = read_u32(ctx.handle, result_node + category_offset).unwrap_or(0);
        if found_category != category {
            return false;
        }

        // Get data pointer
        let mystery = read_i32(ctx.handle, result_node + mystery_value_offset).unwrap_or(-1);
        let data_ptr = if mystery == 0 {
            let mult = read_i32(ctx.handle, vmf + cfg.multiplier_offset).unwrap_or(0);
            let offset_val = read_i32(ctx.handle, result_node + element_value_offset).unwrap_or(0);
            let base_addr = read_u64(ctx.handle, vmf + cfg.base_addr_offset).unwrap_or(0);
            ((mult as i64 * offset_val as i64) + base_addr as i64) as usize
        } else if mystery == 1 {
            return false;
        } else {
            read_u64(ctx.handle, result_node + element_value_offset).unwrap_or(0) as usize
        };

        if data_ptr == 0 {
            return false;
        }

        // Read the bit
        let bit_index = 7 - (remainder & 7);
        let byte_offset = (remainder >> 3) as usize;

        if let Some(byte_val) = read_u8(ctx.handle, data_ptr + byte_offset) {
            return (byte_val & (1 << bit_index)) != 0;
        }

        false
    }

    /// Offset table algorithm (DS1 style)
    fn is_flag_set_offset_table(&self, ctx: &MemoryContext, flag_id: u32) -> bool {
        let cfg = match self.config.offset_table_config.as_ref() {
            Some(c) => c,
            None => return false,
        };

        let ef_ptr = match ctx.pointers.get(&cfg.primary_pattern) {
            Some(&addr) => addr,
            None => return false,
        };

        // Read event flags pointer - DS1R uses 32-bit pointers in 64-bit address space
        let event_flags = match read_u32(ctx.handle, ef_ptr) {
            Some(p) if p != 0 => p as usize,
            _ => return false,
        };

        // Special case for small flags (0-99) - direct array lookup
        if flag_id < 100 {
            let value = read_u32(ctx.handle, event_flags + (flag_id as usize * 4)).unwrap_or(0);
            return value != 0;
        }

        // Decompose 8-digit flag ID
        let id_str = format!("{:08}", flag_id);
        if id_str.len() != 8 {
            return false;
        }

        let group = &id_str[0..1];
        let area = &id_str[1..4];
        let section: i32 = match id_str[4..5].parse() {
            Ok(v) => v,
            Err(_) => return false,
        };
        let number: i32 = match id_str[5..8].parse() {
            Ok(v) => v,
            Err(_) => return false,
        };

        if section < 0 || number < 0 {
            return false;
        }

        let group_offset = match cfg.group_offsets.get(group) {
            Some(&o) => o,
            None => return false,
        };

        let area_idx = match cfg.area_indices.get(area) {
            Some(&i) => i,
            None => return false,
        };

        let offset = group_offset
            + (area_idx * 0x500)
            + (section as usize * 128)
            + ((number - (number % 32)) / 8) as usize;
        let mask = 0x80000000u32 >> (number % 32);

        if let Some(value) = read_u32(ctx.handle, event_flags + offset) {
            return (value & mask) != 0;
        }

        false
    }

    /// Kill counter algorithm (DS2 style)
    fn is_flag_set_kill_counter(&self, ctx: &MemoryContext, boss_offset: u32) -> bool {
        let cfg = match self.config.kill_counter_config.as_ref() {
            Some(c) => c,
            None => return false,
        };

        let base_ptr = match ctx.pointers.get(&cfg.primary_pattern) {
            Some(&addr) => addr,
            None => return false,
        };

        // Follow pointer chain
        let mut current = match read_ptr(ctx.handle, base_ptr) {
            Some(p) if p != 0 => p,
            _ => return false,
        };

        for &offset in &cfg.chain_offsets {
            current = match read_ptr(ctx.handle, current + offset) {
                Some(p) if p != 0 => p,
                _ => return false,
            };
        }

        // Read boss kill count
        if let Some(count) = read_i32(ctx.handle, current + boss_offset as usize) {
            return count > 0;
        }

        false
    }
}

// =============================================================================
// Linux Implementation
// =============================================================================

#[cfg(target_os = "linux")]
use std::collections::HashMap;

#[cfg(target_os = "linux")]
use crate::readers::flag_reader::{FlagReader, MemoryContext};
#[cfg(target_os = "linux")]
use crate::memory::{
    parse_pattern, read_i32, read_ptr, read_u32, read_u64, read_u8, resolve_rip_relative, scan_pattern,
};
#[cfg(target_os = "linux")]
use crate::config::{AutosplitterMemoryConfig, PatternConfig};

/// Generic plugin-based flag reader (Linux)
#[cfg(target_os = "linux")]
pub struct PluginFlagReader {
    config: AutosplitterMemoryConfig,
    debug_logged: std::sync::atomic::AtomicBool,
}

#[cfg(target_os = "linux")]
impl PluginFlagReader {
    pub fn new(config: AutosplitterMemoryConfig) -> Self {
        Self {
            config,
            debug_logged: std::sync::atomic::AtomicBool::new(false),
        }
    }

    fn get_pattern(&self, name: &str) -> Option<&PatternConfig> {
        self.config.patterns.iter().find(|p| p.name == name)
    }

    fn scan_single_pattern(
        &self,
        pid: i32,
        base: usize,
        size: usize,
        pattern_name: &str,
        default_rip_offset: usize,
        default_instr_len: usize,
    ) -> Option<usize> {
        let pattern_config = self.get_pattern(pattern_name)?;
        let pattern_str = &pattern_config.pattern;
        let rip_offset = pattern_config.rip_offset;
        let instr_len = pattern_config.instruction_len;

        let parsed = parse_pattern(pattern_str);
        let found = scan_pattern(pid, base, size, &parsed)?;

        let actual_rip = if rip_offset > 0 { rip_offset } else { default_rip_offset };
        let actual_len = if instr_len > 0 { instr_len } else { default_instr_len };

        resolve_rip_relative(pid, found, actual_rip, actual_len)
    }

    fn apply_pointer_offsets(&self, pid: i32, base_addr: usize, offsets: &[i64]) -> Option<usize> {
        if offsets.is_empty() {
            return Some(base_addr);
        }

        let mut current = base_addr;

        for (i, &offset) in offsets.iter().enumerate() {
            let addr = if offset >= 0 {
                current.wrapping_add(offset as usize)
            } else {
                current.wrapping_sub((-offset) as usize)
            };

            if i < offsets.len() - 1 {
                current = read_ptr(pid, addr)?;
                if current == 0 {
                    return None;
                }
            } else {
                current = addr;
            }
        }

        Some(current)
    }
}

#[cfg(target_os = "linux")]
impl FlagReader for PluginFlagReader {
    fn algorithm_name(&self) -> &'static str {
        match self.config.algorithm.as_str() {
            "category_decomposition" => "category_decomposition",
            "binary_tree" => "binary_tree",
            "offset_table" => "offset_table",
            "kill_counter" => "kill_counter",
            _ => "unknown",
        }
    }

    fn scan_patterns(
        &self,
        pid: i32,
        base: usize,
        size: usize,
        _patterns: &[PatternConfig],
    ) -> Option<HashMap<String, usize>> {
        let mut result = HashMap::new();

        for pattern in &self.config.patterns {
            if let Some(addr) = self.scan_single_pattern(
                pid, base, size,
                &pattern.name,
                pattern.rip_offset,
                pattern.instruction_len,
            ) {
                log::info!("Pattern '{}' found at 0x{:X}", pattern.name, addr);

                let final_addr = if !pattern.pointer_offsets.is_empty() {
                    self.apply_pointer_offsets(pid, addr, &pattern.pointer_offsets)
                        .unwrap_or(addr)
                } else {
                    addr
                };

                if final_addr != addr {
                    log::info!("Pattern '{}' after pointer offsets: 0x{:X}", pattern.name, final_addr);
                }

                result.insert(pattern.name.clone(), final_addr);
            }
        }

        for (name, derived) in &self.config.pointers {
            if let Some(&base_addr) = result.get(&derived.base) {
                if let Some(final_addr) = self.apply_pointer_offsets(pid, base_addr, &derived.offsets) {
                    log::info!("Derived pointer '{}' (from {}) = 0x{:X}", name, derived.base, final_addr);
                    result.insert(name.clone(), final_addr);
                }
            }
        }

        match self.config.algorithm.as_str() {
            "category_decomposition" => {
                if let Some(cfg) = self.config.category_config.as_ref() {
                    if !result.contains_key(&cfg.primary_pattern) {
                        log::error!("category_decomposition: primary pattern '{}' not found", cfg.primary_pattern);
                        return None;
                    }
                }
            }
            "binary_tree" => {
                if let Some(cfg) = self.config.tree_config.as_ref() {
                    if !result.contains_key(&cfg.primary_pattern) {
                        log::error!("binary_tree: primary pattern '{}' not found", cfg.primary_pattern);
                        return None;
                    }
                }
            }
            "offset_table" => {
                if let Some(cfg) = self.config.offset_table_config.as_ref() {
                    if !result.contains_key(&cfg.primary_pattern) {
                        log::error!("offset_table: primary pattern '{}' not found", cfg.primary_pattern);
                        return None;
                    }
                }
            }
            "kill_counter" => {
                if let Some(cfg) = self.config.kill_counter_config.as_ref() {
                    if !result.contains_key(&cfg.primary_pattern) {
                        log::error!("kill_counter: primary pattern '{}' not found", cfg.primary_pattern);
                        return None;
                    }
                }
            }
            _ => {
                log::error!("Unknown algorithm: {}", self.config.algorithm);
                return None;
            }
        }

        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }

    fn is_flag_set(&self, ctx: &MemoryContext, flag_id: u32) -> bool {
        match self.config.algorithm.as_str() {
            "category_decomposition" => self.is_flag_set_category(ctx, flag_id),
            "binary_tree" => self.is_flag_set_binary_tree(ctx, flag_id),
            "offset_table" => self.is_flag_set_offset_table(ctx, flag_id),
            "kill_counter" => self.is_flag_set_kill_counter(ctx, flag_id),
            _ => false,
        }
    }
}

// Algorithm implementations for Linux
#[cfg(target_os = "linux")]
impl PluginFlagReader {
    fn is_flag_set_category(&self, ctx: &MemoryContext, flag_id: u32) -> bool {
        use std::sync::atomic::Ordering;

        let should_log = !self.debug_logged.load(Ordering::Relaxed);

        let cfg = match self.config.category_config.as_ref() {
            Some(c) => c,
            None => return false,
        };

        let event_flag_man_ptr = match ctx.pointers.get(&cfg.primary_pattern) {
            Some(&addr) => addr,
            None => return false,
        };

        let event_flag_man = match read_ptr(ctx.pid, event_flag_man_ptr) {
            Some(p) if p != 0 => p,
            _ => return false,
        };

        let event_flag_div_10000000 = ((flag_id / 10_000_000) % 10) as usize;
        let event_flag_area = ((flag_id / 100_000) % 100) as i32;
        let event_flag_div_10000 = ((flag_id / 10_000) % 10) as i32;
        let event_flag_div_1000 = ((flag_id / 1_000) % 10) as usize;

        let flag_category = self.get_flag_category(
            ctx,
            event_flag_area,
            event_flag_div_10000,
            &cfg.secondary_pattern,
            should_log,
        );

        if flag_category < 0 {
            if should_log {
                self.debug_logged.store(true, Ordering::Relaxed);
            }
            return false;
        }

        let ptr1 = match read_ptr(ctx.pid, event_flag_man + cfg.base_offset) {
            Some(p) if p != 0 => p,
            _ => return false,
        };

        let entry_base_addr = match read_ptr(ctx.pid, ptr1 + (event_flag_div_10000000 * cfg.entry_size)) {
            Some(p) if p != 0 => p,
            _ => return false,
        };

        let mod_1000 = flag_id % 1000;
        let byte_offset = ((mod_1000 >> 5) * 4) as usize;
        let bit_index = 0x1f - ((mod_1000 as u8) & 0x1f);
        let mask = 1u32 << bit_index;

        let result_addr = (event_flag_div_1000 << 4) + entry_base_addr + ((flag_category as usize) * cfg.category_multiplier);

        if let Some(value) = read_u32(ctx.pid, result_addr + byte_offset) {
            let is_set = (value & mask) != 0;
            if should_log {
                self.debug_logged.store(true, Ordering::Relaxed);
            }
            return is_set;
        }

        false
    }

    fn get_flag_category(
        &self,
        ctx: &MemoryContext,
        event_flag_area: i32,
        event_flag_div_10000: i32,
        field_area_pattern: &Option<String>,
        _should_log: bool,
    ) -> i32 {
        if event_flag_area >= 90 || (event_flag_area + event_flag_div_10000 == 0) {
            return 0;
        }

        let field_area_ptr = match field_area_pattern {
            Some(pattern) => match ctx.pointers.get(pattern) {
                Some(&addr) => addr,
                None => return -1,
            },
            None => return -1,
        };

        let cfg = self.config.category_config.as_ref().unwrap();
        let field_area_base_offset = cfg.field_area_base_offset.unwrap_or(0);
        let world_info_offset = cfg.world_info_offset.unwrap_or(0x10);

        let field_area = match read_ptr(ctx.pid, field_area_ptr + field_area_base_offset) {
            Some(p) if p != 0 => p,
            _ => return -1,
        };

        let world_info_owner = match read_ptr(ctx.pid, field_area + world_info_offset) {
            Some(p) if p != 0 => p,
            _ => return -1,
        };

        let size = match read_i32(ctx.pid, world_info_owner + 0x8) {
            Some(s) if s > 0 && s < 1000 => s as usize,
            _ => return -1,
        };

        let vector = match read_ptr(ctx.pid, world_info_owner + 0x10) {
            Some(v) if v != 0 => v,
            _ => return -1,
        };

        let world_info_size = cfg.world_info_struct_size.unwrap_or(0x38);
        let world_block_size = cfg.world_block_struct_size.unwrap_or(0x70);

        for i in 0..size {
            let area = read_u8(ctx.pid, vector + (i * world_info_size) + 0xb).unwrap_or(0) as i32;

            if area == event_flag_area {
                let count = read_u8(ctx.pid, vector + (i * world_info_size) + 0x20).unwrap_or(0) as usize;

                if count >= 1 {
                    let block_vector_ptr = vector + (i * world_info_size) + 0x28;
                    let block_vector = match read_ptr(ctx.pid, block_vector_ptr) {
                        Some(p) if p != 0 => p,
                        _ => continue,
                    };

                    for index in 0..count {
                        let flag = read_i32(ctx.pid, block_vector + (index * world_block_size) + 0x8).unwrap_or(0);

                        if ((flag >> 0x10) & 0xff) == event_flag_div_10000 && (flag >> 0x18) == event_flag_area {
                            let category = read_i32(ctx.pid, block_vector + (index * world_block_size) + 0x20).unwrap_or(-1);
                            if category >= 0 {
                                return category + 1;
                            }
                        }
                    }
                }
            }
        }

        -1
    }

    fn is_flag_set_binary_tree(&self, ctx: &MemoryContext, flag_id: u32) -> bool {
        let cfg = match self.config.tree_config.as_ref() {
            Some(c) => c,
            None => return false,
        };

        let tree_layout = self.config.memory_layout.event_flag_tree.as_ref();
        let first_sub_element = tree_layout.and_then(|t| t.first_sub_element).unwrap_or(0x8);
        let left_child = tree_layout.and_then(|t| t.left_child).unwrap_or(0x0);
        let right_child = tree_layout.and_then(|t| t.right_child).unwrap_or(0x10);
        let leaf_check_offset = tree_layout.and_then(|t| t.leaf_check_offset).unwrap_or(0x19);
        let category_offset = tree_layout.and_then(|t| t.category_offset).unwrap_or(0x20);
        let mystery_value_offset = tree_layout.and_then(|t| t.mystery_value_offset).unwrap_or(0x28);
        let element_value_offset = tree_layout.and_then(|t| t.element_value_offset).unwrap_or(0x30);

        let vmf_ptr = match ctx.pointers.get(&cfg.primary_pattern) {
            Some(&addr) => addr,
            None => return false,
        };

        let mut vmf = match read_ptr(ctx.pid, vmf_ptr) {
            Some(p) if p != 0 => p,
            _ => return false,
        };

        for &offset in &cfg.pointer_chain {
            vmf = match read_ptr(ctx.pid, vmf + offset) {
                Some(p) if p != 0 => p,
                _ => return false,
            };
        }

        let divisor = match read_i32(ctx.pid, vmf + cfg.divisor_offset) {
            Some(d) if d > 0 => d as u32,
            _ => return false,
        };

        let category = flag_id / divisor;
        let remainder = flag_id - (category * divisor);

        let root = match read_ptr(ctx.pid, vmf + cfg.tree_root_offset) {
            Some(p) if p != 0 => p,
            _ => return false,
        };

        let mut current = match read_ptr(ctx.pid, root + first_sub_element) {
            Some(p) if p != 0 => p,
            _ => return false,
        };

        let mut result_node = current;

        for _ in 0..128 {
            let marker = read_u8(ctx.pid, current + leaf_check_offset).unwrap_or(1);
            if marker != 0 {
                break;
            }

            let node_category = read_u32(ctx.pid, current + category_offset).unwrap_or(0);
            if node_category < category {
                current = match read_ptr(ctx.pid, current + right_child) {
                    Some(p) if p != 0 => p,
                    _ => break,
                };
            } else {
                result_node = current;
                current = match read_ptr(ctx.pid, current + left_child) {
                    Some(p) if p != 0 => p,
                    _ => break,
                };
            }
        }

        let found_category = read_u32(ctx.pid, result_node + category_offset).unwrap_or(0);
        if found_category != category {
            return false;
        }

        let mystery = read_i32(ctx.pid, result_node + mystery_value_offset).unwrap_or(-1);
        let data_ptr = if mystery == 0 {
            let mult = read_i32(ctx.pid, vmf + cfg.multiplier_offset).unwrap_or(0);
            let offset_val = read_i32(ctx.pid, result_node + element_value_offset).unwrap_or(0);
            let base_addr = read_u64(ctx.pid, vmf + cfg.base_addr_offset).unwrap_or(0);
            ((mult as i64 * offset_val as i64) + base_addr as i64) as usize
        } else if mystery == 1 {
            return false;
        } else {
            read_u64(ctx.pid, result_node + element_value_offset).unwrap_or(0) as usize
        };

        if data_ptr == 0 {
            return false;
        }

        let bit_index = 7 - (remainder & 7);
        let byte_offset = (remainder >> 3) as usize;

        if let Some(byte_val) = read_u8(ctx.pid, data_ptr + byte_offset) {
            return (byte_val & (1 << bit_index)) != 0;
        }

        false
    }

    fn is_flag_set_offset_table(&self, ctx: &MemoryContext, flag_id: u32) -> bool {
        let cfg = match self.config.offset_table_config.as_ref() {
            Some(c) => c,
            None => return false,
        };

        let ef_ptr = match ctx.pointers.get(&cfg.primary_pattern) {
            Some(&addr) => addr,
            None => return false,
        };

        let event_flags = match read_u32(ctx.pid, ef_ptr) {
            Some(p) if p != 0 => p as usize,
            _ => return false,
        };

        if flag_id < 100 {
            let value = read_u32(ctx.pid, event_flags + (flag_id as usize * 4)).unwrap_or(0);
            return value != 0;
        }

        let id_str = format!("{:08}", flag_id);
        if id_str.len() != 8 {
            return false;
        }

        let group = &id_str[0..1];
        let area = &id_str[1..4];
        let section: i32 = match id_str[4..5].parse() {
            Ok(v) => v,
            Err(_) => return false,
        };
        let number: i32 = match id_str[5..8].parse() {
            Ok(v) => v,
            Err(_) => return false,
        };

        if section < 0 || number < 0 {
            return false;
        }

        let group_offset = match cfg.group_offsets.get(group) {
            Some(&o) => o,
            None => return false,
        };

        let area_idx = match cfg.area_indices.get(area) {
            Some(&i) => i,
            None => return false,
        };

        let offset = group_offset
            + (area_idx * 0x500)
            + (section as usize * 128)
            + ((number - (number % 32)) / 8) as usize;
        let mask = 0x80000000u32 >> (number % 32);

        if let Some(value) = read_u32(ctx.pid, event_flags + offset) {
            return (value & mask) != 0;
        }

        false
    }

    fn is_flag_set_kill_counter(&self, ctx: &MemoryContext, boss_offset: u32) -> bool {
        let cfg = match self.config.kill_counter_config.as_ref() {
            Some(c) => c,
            None => return false,
        };

        let base_ptr = match ctx.pointers.get(&cfg.primary_pattern) {
            Some(&addr) => addr,
            None => return false,
        };

        let mut current = match read_ptr(ctx.pid, base_ptr) {
            Some(p) if p != 0 => p,
            _ => return false,
        };

        for &offset in &cfg.chain_offsets {
            current = match read_ptr(ctx.pid, current + offset) {
                Some(p) if p != 0 => p,
                _ => return false,
            };
        }

        if let Some(count) = read_i32(ctx.pid, current + boss_offset as usize) {
            return count > 0;
        }

        false
    }
}
