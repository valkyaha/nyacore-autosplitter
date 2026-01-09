//! Cross-platform memory operations
//!
//! This module provides platform-agnostic abstractions for memory reading,
//! with implementations for Windows and Linux.

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
mod linux;

mod pointer;
mod pattern;
mod process;

pub use pointer::Pointer;
pub use pattern::{parse_pattern, scan_pattern, extract_relative_address};
pub use process::{find_process, find_process_by_names, get_module_info, is_process_running, ProcessInfo};

#[cfg(target_os = "windows")]
pub use process::is_process_running_by_handle;

#[cfg(target_os = "windows")]
pub use windows::WindowsMemoryReader;

#[cfg(target_os = "linux")]
pub use linux::LinuxMemoryReader;

/// Platform-agnostic memory reading trait
pub trait MemoryReader: Send + Sync {
    /// Read raw bytes from memory
    fn read_bytes(&self, address: usize, size: usize) -> Option<Vec<u8>>;

    /// Read a u8 value
    fn read_u8(&self, address: usize) -> Option<u8> {
        self.read_bytes(address, 1).map(|b| b[0])
    }

    /// Read a u16 value (little-endian)
    fn read_u16(&self, address: usize) -> Option<u16> {
        self.read_bytes(address, 2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
    }

    /// Read an i16 value (little-endian)
    fn read_i16(&self, address: usize) -> Option<i16> {
        self.read_bytes(address, 2)
            .map(|b| i16::from_le_bytes([b[0], b[1]]))
    }

    /// Read a u32 value (little-endian)
    fn read_u32(&self, address: usize) -> Option<u32> {
        self.read_bytes(address, 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// Read an i32 value (little-endian)
    fn read_i32(&self, address: usize) -> Option<i32> {
        self.read_bytes(address, 4)
            .map(|b| i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// Read a u64 value (little-endian)
    fn read_u64(&self, address: usize) -> Option<u64> {
        self.read_bytes(address, 8).map(|b| {
            u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
        })
    }

    /// Read an i64 value (little-endian)
    fn read_i64(&self, address: usize) -> Option<i64> {
        self.read_bytes(address, 8).map(|b| {
            i64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
        })
    }

    /// Read a f32 value
    fn read_f32(&self, address: usize) -> Option<f32> {
        self.read_bytes(address, 4)
            .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// Read a pointer (platform-dependent size)
    fn read_ptr(&self, address: usize) -> Option<usize> {
        #[cfg(target_pointer_width = "64")]
        {
            self.read_u64(address).map(|v| v as usize)
        }
        #[cfg(target_pointer_width = "32")]
        {
            self.read_u32(address).map(|v| v as usize)
        }
    }

    /// Read a boolean (non-zero = true)
    fn read_bool(&self, address: usize) -> Option<bool> {
        self.read_u8(address).map(|v| v != 0)
    }
}

use std::sync::Arc;

/// Context for a connected process
pub struct ProcessContext {
    /// Memory reader for this process (shared via Arc for game implementations)
    pub reader: Arc<dyn MemoryReader>,
    /// Base address of the main module
    pub base_address: usize,
    /// Size of the main module
    pub module_size: usize,
    /// Process ID
    pub process_id: u32,
    /// Whether this is a 64-bit process
    pub is_64_bit: bool,
}

impl ProcessContext {
    /// Create a new process context
    pub fn new(
        reader: Arc<dyn MemoryReader>,
        base_address: usize,
        module_size: usize,
        process_id: u32,
        is_64_bit: bool,
    ) -> Self {
        Self {
            reader,
            base_address,
            module_size,
            process_id,
            is_64_bit,
        }
    }

    /// Create a new process context from a boxed reader (convenience method)
    pub fn from_boxed(
        reader: Box<dyn MemoryReader>,
        base_address: usize,
        module_size: usize,
        process_id: u32,
        is_64_bit: bool,
    ) -> Self {
        Self {
            reader: Arc::from(reader),
            base_address,
            module_size,
            process_id,
            is_64_bit,
        }
    }

    /// Get a clone of the reader Arc
    pub fn reader(&self) -> Arc<dyn MemoryReader> {
        self.reader.clone()
    }

    /// Read from the process memory
    pub fn read_bytes(&self, address: usize, size: usize) -> Option<Vec<u8>> {
        self.reader.read_bytes(address, size)
    }

    /// Read a pointer value
    pub fn read_ptr(&self, address: usize) -> Option<usize> {
        self.reader.read_ptr(address)
    }

    /// Read a u32 value
    pub fn read_u32(&self, address: usize) -> Option<u32> {
        self.reader.read_u32(address)
    }

    /// Read an i32 value
    pub fn read_i32(&self, address: usize) -> Option<i32> {
        self.reader.read_i32(address)
    }

    /// Read a f32 value
    pub fn read_f32(&self, address: usize) -> Option<f32> {
        self.reader.read_f32(address)
    }

    /// Read a boolean
    pub fn read_bool(&self, address: usize) -> Option<bool> {
        self.reader.read_bool(address)
    }

    /// Scan for a pattern in the module
    pub fn scan_pattern(&self, pattern: &[Option<u8>]) -> Option<usize> {
        scan_pattern(
            &*self.reader,
            self.base_address,
            self.module_size,
            pattern,
        )
    }
}
