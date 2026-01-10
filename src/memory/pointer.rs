//! Rust port of SoulSplitter's Pointer class
//! https://github.com/FrankvdStam/SoulSplitter
//!
//! The Pointer class manages a base address and a list of offsets.
//! When resolving, each offset EXCEPT the last is dereferenced.
//! The last offset is just added to get the final address.

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HANDLE;

#[cfg(target_os = "windows")]
use crate::memory::reader::{read_i32, read_i64, read_u8, read_u32, read_u64};

/// Rust port of SoulSplitter's Pointer class
#[cfg(target_os = "windows")]
#[derive(Clone)]
pub struct Pointer {
    pub handle: HANDLE,
    pub is_64_bit: bool,
    pub base_address: i64,
    pub offsets: Vec<i64>,
}

#[cfg(target_os = "windows")]
impl Pointer {
    /// Create a new uninitialized pointer
    pub fn new() -> Self {
        Self {
            handle: HANDLE::default(),
            is_64_bit: true,
            base_address: 0,
            offsets: Vec::new(),
        }
    }

    /// Initialize the pointer with process handle, base address and offsets
    pub fn initialize(&mut self, handle: HANDLE, is_64_bit: bool, base_address: i64, offsets: &[i64]) {
        self.handle = handle;
        self.is_64_bit = is_64_bit;
        self.base_address = base_address;
        self.offsets = offsets.to_vec();
    }

    /// Clear the pointer
    pub fn clear(&mut self) {
        self.base_address = 0;
        self.offsets.clear();
    }

    /// Create a copy of this pointer
    pub fn copy(&self) -> Self {
        Self {
            handle: self.handle,
            is_64_bit: self.is_64_bit,
            base_address: self.base_address,
            offsets: self.offsets.clone(),
        }
    }

    /// Creates a new pointer with the address of the old pointer as base address
    /// This is equivalent to SoulSplitter's CreatePointerFromAddress
    pub fn create_pointer_from_address(&self, offset: Option<i64>) -> Self {
        let mut copy = self.copy();
        let mut offsets = self.offsets.clone();

        if let Some(off) = offset {
            offsets.push(off);
        }

        // Add trailing 0 - this is what SoulSplitter does
        offsets.push(0);

        copy.base_address = self.resolve_offsets(&offsets);
        copy.offsets.clear();
        copy
    }

    /// Append offsets to create a new pointer
    /// This is equivalent to SoulSplitter's Append
    pub fn append(&self, offsets: &[i64]) -> Self {
        let mut copy = self.copy();
        copy.offsets.extend_from_slice(offsets);
        copy
    }

    /// Resolve offsets and return the final address
    /// SoulSplitter logic: all offsets EXCEPT the last are dereferenced
    fn resolve_offsets(&self, offsets: &[i64]) -> i64 {
        let mut ptr = self.base_address;

        for (i, &offset) in offsets.iter().enumerate() {
            let address = ptr + offset;

            // Not the last offset = resolve as pointer (dereference)
            if i + 1 < offsets.len() {
                if self.is_64_bit {
                    ptr = match read_i64(self.handle, address as usize) {
                        Some(v) => v,
                        None => return 0,
                    };
                } else {
                    ptr = match read_i32(self.handle, address as usize) {
                        Some(v) => v as i64,
                        None => return 0,
                    };
                }

                if ptr == 0 {
                    return 0;
                }
            } else {
                // Last offset: just add, no dereference
                ptr = address;
            }
        }

        ptr
    }

    /// Check if the pointer resolves to null
    pub fn is_null_ptr(&self) -> bool {
        self.get_address() == 0
    }

    /// Get the resolved address
    pub fn get_address(&self) -> i64 {
        self.resolve_offsets(&self.offsets)
    }

    /// Read i32 at optional offset
    pub fn read_i32(&self, offset: Option<i64>) -> i32 {
        let mut offsets_copy = self.offsets.clone();
        if let Some(off) = offset {
            offsets_copy.push(off);
        }
        let address = self.resolve_offsets(&offsets_copy);
        read_i32(self.handle, address as usize).unwrap_or(0)
    }

    /// Read u32 at optional offset
    pub fn read_u32(&self, offset: Option<i64>) -> u32 {
        let mut offsets_copy = self.offsets.clone();
        if let Some(off) = offset {
            offsets_copy.push(off);
        }
        let address = self.resolve_offsets(&offsets_copy);
        read_u32(self.handle, address as usize).unwrap_or(0)
    }

    /// Read i64 at optional offset
    pub fn read_i64(&self, offset: Option<i64>) -> i64 {
        let mut offsets_copy = self.offsets.clone();
        if let Some(off) = offset {
            offsets_copy.push(off);
        }
        let address = self.resolve_offsets(&offsets_copy);
        read_i64(self.handle, address as usize).unwrap_or(0)
    }

    /// Read u64 at optional offset
    pub fn read_u64(&self, offset: Option<i64>) -> u64 {
        let mut offsets_copy = self.offsets.clone();
        if let Some(off) = offset {
            offsets_copy.push(off);
        }
        let address = self.resolve_offsets(&offsets_copy);
        read_u64(self.handle, address as usize).unwrap_or(0)
    }

    /// Read byte at optional offset
    pub fn read_byte(&self, offset: Option<i64>) -> u8 {
        let mut offsets_copy = self.offsets.clone();
        if let Some(off) = offset {
            offsets_copy.push(off);
        }
        let address = self.resolve_offsets(&offsets_copy);
        read_u8(self.handle, address as usize).unwrap_or(0)
    }
}

#[cfg(target_os = "windows")]
impl Default for Pointer {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Linux Implementation
// =============================================================================

#[cfg(target_os = "linux")]
use crate::memory::reader::{read_i32, read_i64, read_u8, read_u32, read_u64};

/// Rust port of SoulSplitter's Pointer class (Linux version)
#[cfg(target_os = "linux")]
#[derive(Clone)]
pub struct Pointer {
    pub pid: i32,
    pub is_64_bit: bool,
    pub base_address: i64,
    pub offsets: Vec<i64>,
}

#[cfg(target_os = "linux")]
impl Pointer {
    /// Create a new uninitialized pointer
    pub fn new() -> Self {
        Self {
            pid: 0,
            is_64_bit: true,
            base_address: 0,
            offsets: Vec::new(),
        }
    }

    /// Initialize the pointer with process PID, base address and offsets
    pub fn initialize(&mut self, pid: i32, is_64_bit: bool, base_address: i64, offsets: &[i64]) {
        self.pid = pid;
        self.is_64_bit = is_64_bit;
        self.base_address = base_address;
        self.offsets = offsets.to_vec();
    }

    /// Clear the pointer
    pub fn clear(&mut self) {
        self.base_address = 0;
        self.offsets.clear();
    }

    /// Create a copy of this pointer
    pub fn copy(&self) -> Self {
        Self {
            pid: self.pid,
            is_64_bit: self.is_64_bit,
            base_address: self.base_address,
            offsets: self.offsets.clone(),
        }
    }

    /// Creates a new pointer with the address of the old pointer as base address
    pub fn create_pointer_from_address(&self, offset: Option<i64>) -> Self {
        let mut copy = self.copy();
        let mut offsets = self.offsets.clone();

        if let Some(off) = offset {
            offsets.push(off);
        }

        // Add trailing 0 - this is what SoulSplitter does
        offsets.push(0);

        copy.base_address = self.resolve_offsets(&offsets);
        copy.offsets.clear();
        copy
    }

    /// Append offsets to create a new pointer
    pub fn append(&self, offsets: &[i64]) -> Self {
        let mut copy = self.copy();
        copy.offsets.extend_from_slice(offsets);
        copy
    }

    /// Resolve offsets and return the final address
    fn resolve_offsets(&self, offsets: &[i64]) -> i64 {
        let mut ptr = self.base_address;

        for (i, &offset) in offsets.iter().enumerate() {
            let address = ptr + offset;

            // Not the last offset = resolve as pointer (dereference)
            if i + 1 < offsets.len() {
                if self.is_64_bit {
                    ptr = match read_i64(self.pid, address as usize) {
                        Some(v) => v,
                        None => return 0,
                    };
                } else {
                    ptr = match read_i32(self.pid, address as usize) {
                        Some(v) => v as i64,
                        None => return 0,
                    };
                }

                if ptr == 0 {
                    return 0;
                }
            } else {
                // Last offset: just add, no dereference
                ptr = address;
            }
        }

        ptr
    }

    /// Check if the pointer resolves to null
    pub fn is_null_ptr(&self) -> bool {
        self.get_address() == 0
    }

    /// Get the resolved address
    pub fn get_address(&self) -> i64 {
        self.resolve_offsets(&self.offsets)
    }

    /// Read i32 at optional offset
    pub fn read_i32(&self, offset: Option<i64>) -> i32 {
        let mut offsets_copy = self.offsets.clone();
        if let Some(off) = offset {
            offsets_copy.push(off);
        }
        let address = self.resolve_offsets(&offsets_copy);
        read_i32(self.pid, address as usize).unwrap_or(0)
    }

    /// Read u32 at optional offset
    pub fn read_u32(&self, offset: Option<i64>) -> u32 {
        let mut offsets_copy = self.offsets.clone();
        if let Some(off) = offset {
            offsets_copy.push(off);
        }
        let address = self.resolve_offsets(&offsets_copy);
        read_u32(self.pid, address as usize).unwrap_or(0)
    }

    /// Read i64 at optional offset
    pub fn read_i64(&self, offset: Option<i64>) -> i64 {
        let mut offsets_copy = self.offsets.clone();
        if let Some(off) = offset {
            offsets_copy.push(off);
        }
        let address = self.resolve_offsets(&offsets_copy);
        read_i64(self.pid, address as usize).unwrap_or(0)
    }

    /// Read u64 at optional offset
    pub fn read_u64(&self, offset: Option<i64>) -> u64 {
        let mut offsets_copy = self.offsets.clone();
        if let Some(off) = offset {
            offsets_copy.push(off);
        }
        let address = self.resolve_offsets(&offsets_copy);
        read_u64(self.pid, address as usize).unwrap_or(0)
    }

    /// Read byte at optional offset
    pub fn read_byte(&self, offset: Option<i64>) -> u8 {
        let mut offsets_copy = self.offsets.clone();
        if let Some(off) = offset {
            offsets_copy.push(off);
        }
        let address = self.resolve_offsets(&offsets_copy);
        read_u8(self.pid, address as usize).unwrap_or(0)
    }
}

#[cfg(target_os = "linux")]
impl Default for Pointer {
    fn default() -> Self {
        Self::new()
    }
}
