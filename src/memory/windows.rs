//! Windows memory reader implementation

#![cfg(target_os = "windows")]

use super::MemoryReader;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::Diagnostics::Debug::ReadProcessMemory;

/// Windows-specific memory reader using ReadProcessMemory
pub struct WindowsMemoryReader {
    handle: HANDLE,
}

impl WindowsMemoryReader {
    /// Create a new Windows memory reader for the given process handle
    pub fn new(handle: HANDLE) -> Self {
        Self { handle }
    }

    /// Get the underlying handle
    pub fn handle(&self) -> HANDLE {
        self.handle
    }
}

impl MemoryReader for WindowsMemoryReader {
    fn read_bytes(&self, address: usize, size: usize) -> Option<Vec<u8>> {
        let mut buffer = vec![0u8; size];
        let mut bytes_read = 0;

        let result = unsafe {
            ReadProcessMemory(
                self.handle,
                address as *const _,
                buffer.as_mut_ptr() as *mut _,
                size,
                Some(&mut bytes_read),
            )
        };

        if result.is_ok() && bytes_read == size {
            Some(buffer)
        } else {
            None
        }
    }
}

// Note: HANDLE is not Send/Sync by default, but we ensure thread safety
// through proper usage patterns in the autosplitter
unsafe impl Send for WindowsMemoryReader {}
unsafe impl Sync for WindowsMemoryReader {}
