//! ASL Memory Integration
//!
//! Provides the bridge between ASL variable definitions and actual memory reading.
//! This module handles:
//! - Pattern resolution (finding base addresses)
//! - Pointer chain following
//! - Type-aware memory reading

use std::collections::HashMap;

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HANDLE;

use crate::asl::types::{Value, VarType, VarDefinition};
use crate::memory::{self, Pointer};

/// Memory context for ASL runtime
/// Holds process handle, resolved patterns, and provides memory reading
#[cfg(target_os = "windows")]
pub struct AslMemoryContext {
    /// Process handle
    pub handle: HANDLE,
    /// Module base address
    pub base: usize,
    /// Module size
    pub size: usize,
    /// Resolved pattern addresses (pattern_name -> address)
    pub patterns: HashMap<String, usize>,
    /// Is 64-bit process
    pub is_64_bit: bool,
}

#[cfg(target_os = "windows")]
impl AslMemoryContext {
    /// Create a new memory context
    pub fn new(handle: HANDLE, base: usize, size: usize) -> Self {
        Self {
            handle,
            base,
            size,
            patterns: HashMap::new(),
            is_64_bit: true,
        }
    }

    /// Resolve a pattern and store its address
    pub fn resolve_pattern(&mut self, name: &str, pattern_str: &str, rip_offset: Option<usize>, instruction_len: Option<usize>) -> bool {
        let pattern = memory::parse_pattern(pattern_str);

        if let Some(addr) = memory::scan_pattern(self.handle, self.base, self.size, &pattern) {
            // If RIP-relative, resolve the actual address
            let final_addr = if let (Some(rip_off), Some(inst_len)) = (rip_offset, instruction_len) {
                memory::resolve_rip_relative(self.handle, addr, rip_off, inst_len).unwrap_or(0)
            } else {
                addr
            };

            if final_addr != 0 {
                log::debug!("Resolved pattern '{}' to 0x{:X}", name, final_addr);
                self.patterns.insert(name.to_string(), final_addr);
                return true;
            }
        }

        log::warn!("Failed to resolve pattern '{}'", name);
        false
    }

    /// Get a resolved pattern address
    pub fn get_pattern(&self, name: &str) -> Option<usize> {
        self.patterns.get(name).copied()
    }

    /// Read a variable from memory based on its definition
    pub fn read_variable(&self, def: &VarDefinition) -> Value {
        // Get base address (from module or pattern)
        let base_addr = if let Some(ref module) = def.module {
            // Try to get from resolved patterns
            match self.get_pattern(module) {
                Some(addr) => addr as i64,
                None => return Value::Null,
            }
        } else if !def.offsets.is_empty() {
            // Direct offset from module base
            self.base as i64
        } else {
            return Value::Null;
        };

        // Create pointer and follow chain
        let mut ptr = Pointer::new();
        ptr.initialize(self.handle, self.is_64_bit, base_addr, &def.offsets);

        let address = ptr.get_address() as usize;
        if address == 0 {
            return Value::Null;
        }

        // Read based on type
        self.read_typed_value(address, def.var_type, def.string_length)
    }

    /// Read a typed value from an address
    fn read_typed_value(&self, address: usize, var_type: VarType, string_len: Option<usize>) -> Value {
        match var_type {
            VarType::Bool => {
                memory::read_u8(self.handle, address)
                    .map(|v| Value::Bool(v != 0))
                    .unwrap_or(Value::Null)
            }
            VarType::Byte => {
                memory::read_u8(self.handle, address)
                    .map(|v| Value::Int(v as i64))
                    .unwrap_or(Value::Null)
            }
            VarType::SByte => {
                memory::read_i8(self.handle, address)
                    .map(|v| Value::Int(v as i64))
                    .unwrap_or(Value::Null)
            }
            VarType::Short => {
                memory::read_i16(self.handle, address)
                    .map(|v| Value::Int(v as i64))
                    .unwrap_or(Value::Null)
            }
            VarType::UShort => {
                memory::read_u16(self.handle, address)
                    .map(|v| Value::Int(v as i64))
                    .unwrap_or(Value::Null)
            }
            VarType::Int => {
                memory::read_i32(self.handle, address)
                    .map(|v| Value::Int(v as i64))
                    .unwrap_or(Value::Null)
            }
            VarType::UInt => {
                memory::read_u32(self.handle, address)
                    .map(|v| Value::UInt(v as u64))
                    .unwrap_or(Value::Null)
            }
            VarType::Long => {
                memory::read_i64(self.handle, address)
                    .map(Value::Int)
                    .unwrap_or(Value::Null)
            }
            VarType::ULong => {
                memory::read_u64(self.handle, address)
                    .map(Value::UInt)
                    .unwrap_or(Value::Null)
            }
            VarType::Float => {
                memory::read_f32(self.handle, address)
                    .map(|v| Value::Float(v as f64))
                    .unwrap_or(Value::Null)
            }
            VarType::Double => {
                memory::read_f64(self.handle, address)
                    .map(Value::Float)
                    .unwrap_or(Value::Null)
            }
            VarType::String => {
                let max_len = string_len.unwrap_or(256);
                memory::read_string(self.handle, address, max_len)
                    .map(Value::String)
                    .unwrap_or(Value::Null)
            }
            VarType::ByteArray => {
                let len = string_len.unwrap_or(16);
                memory::read_bytes(self.handle, address, len)
                    .map(Value::ByteArray)
                    .unwrap_or(Value::Null)
            }
        }
    }

    /// Read all variables from their definitions and return a map
    pub fn read_all_variables(&self, definitions: &[VarDefinition]) -> HashMap<String, Value> {
        let mut values = HashMap::new();
        for def in definitions {
            let value = self.read_variable(def);
            values.insert(def.name.clone(), value);
        }
        values
    }
}

// =============================================================================
// Linux Implementation
// =============================================================================

#[cfg(target_os = "linux")]
pub struct AslMemoryContext {
    /// Process ID
    pub pid: i32,
    /// Module base address
    pub base: usize,
    /// Module size
    pub size: usize,
    /// Resolved pattern addresses (pattern_name -> address)
    pub patterns: HashMap<String, usize>,
    /// Is 64-bit process
    pub is_64_bit: bool,
}

#[cfg(target_os = "linux")]
impl AslMemoryContext {
    /// Create a new memory context
    pub fn new(pid: i32, base: usize, size: usize) -> Self {
        Self {
            pid,
            base,
            size,
            patterns: HashMap::new(),
            is_64_bit: true,
        }
    }

    /// Resolve a pattern and store its address
    pub fn resolve_pattern(&mut self, name: &str, pattern_str: &str, rip_offset: Option<usize>, instruction_len: Option<usize>) -> bool {
        let pattern = memory::parse_pattern(pattern_str);

        if let Some(addr) = memory::scan_pattern(self.pid, self.base, self.size, &pattern) {
            // If RIP-relative, resolve the actual address
            let final_addr = if let (Some(rip_off), Some(inst_len)) = (rip_offset, instruction_len) {
                memory::resolve_rip_relative(self.pid, addr, rip_off, inst_len).unwrap_or(0)
            } else {
                addr
            };

            if final_addr != 0 {
                log::debug!("Resolved pattern '{}' to 0x{:X}", name, final_addr);
                self.patterns.insert(name.to_string(), final_addr);
                return true;
            }
        }

        log::warn!("Failed to resolve pattern '{}'", name);
        false
    }

    /// Get a resolved pattern address
    pub fn get_pattern(&self, name: &str) -> Option<usize> {
        self.patterns.get(name).copied()
    }

    /// Read a variable from memory based on its definition
    pub fn read_variable(&self, def: &VarDefinition) -> Value {
        // Get base address (from module or pattern)
        let base_addr = if let Some(ref module) = def.module {
            match self.get_pattern(module) {
                Some(addr) => addr as i64,
                None => return Value::Null,
            }
        } else if !def.offsets.is_empty() {
            self.base as i64
        } else {
            return Value::Null;
        };

        // Create pointer and follow chain
        let mut ptr = Pointer::new();
        ptr.initialize(self.pid, self.is_64_bit, base_addr, &def.offsets);

        let address = ptr.get_address() as usize;
        if address == 0 {
            return Value::Null;
        }

        // Read based on type
        self.read_typed_value(address, def.var_type, def.string_length)
    }

    /// Read a typed value from an address
    fn read_typed_value(&self, address: usize, var_type: VarType, string_len: Option<usize>) -> Value {
        match var_type {
            VarType::Bool => {
                memory::read_u8(self.pid, address)
                    .map(|v| Value::Bool(v != 0))
                    .unwrap_or(Value::Null)
            }
            VarType::Byte => {
                memory::read_u8(self.pid, address)
                    .map(|v| Value::Int(v as i64))
                    .unwrap_or(Value::Null)
            }
            VarType::SByte => {
                memory::read_i8(self.pid, address)
                    .map(|v| Value::Int(v as i64))
                    .unwrap_or(Value::Null)
            }
            VarType::Short => {
                memory::read_i16(self.pid, address)
                    .map(|v| Value::Int(v as i64))
                    .unwrap_or(Value::Null)
            }
            VarType::UShort => {
                memory::read_u16(self.pid, address)
                    .map(|v| Value::Int(v as i64))
                    .unwrap_or(Value::Null)
            }
            VarType::Int => {
                memory::read_i32(self.pid, address)
                    .map(|v| Value::Int(v as i64))
                    .unwrap_or(Value::Null)
            }
            VarType::UInt => {
                memory::read_u32(self.pid, address)
                    .map(|v| Value::UInt(v as u64))
                    .unwrap_or(Value::Null)
            }
            VarType::Long => {
                memory::read_i64(self.pid, address)
                    .map(Value::Int)
                    .unwrap_or(Value::Null)
            }
            VarType::ULong => {
                memory::read_u64(self.pid, address)
                    .map(Value::UInt)
                    .unwrap_or(Value::Null)
            }
            VarType::Float => {
                memory::read_f32(self.pid, address)
                    .map(|v| Value::Float(v as f64))
                    .unwrap_or(Value::Null)
            }
            VarType::Double => {
                memory::read_f64(self.pid, address)
                    .map(Value::Float)
                    .unwrap_or(Value::Null)
            }
            VarType::String => {
                let max_len = string_len.unwrap_or(256);
                memory::read_string(self.pid, address, max_len)
                    .map(Value::String)
                    .unwrap_or(Value::Null)
            }
            VarType::ByteArray => {
                let len = string_len.unwrap_or(16);
                memory::read_bytes(self.pid, address, len)
                    .map(Value::ByteArray)
                    .unwrap_or(Value::Null)
            }
        }
    }

    /// Read all variables from their definitions and return a map
    pub fn read_all_variables(&self, definitions: &[VarDefinition]) -> HashMap<String, Value> {
        let mut values = HashMap::new();
        for def in definitions {
            let value = self.read_variable(def);
            values.insert(def.name.clone(), value);
        }
        values
    }
}
