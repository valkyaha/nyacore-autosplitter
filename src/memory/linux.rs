//! Linux memory reader implementation

#![cfg(target_os = "linux")]

use super::MemoryReader;
use std::io::{Read, Seek, SeekFrom};
use std::fs::File;

/// Linux-specific memory reader using /proc/[pid]/mem
pub struct LinuxMemoryReader {
    pid: i32,
    mem_file: Option<File>,
}

impl LinuxMemoryReader {
    /// Create a new Linux memory reader for the given process ID
    pub fn new(pid: i32) -> Self {
        let mem_path = format!("/proc/{}/mem", pid);
        let mem_file = File::open(&mem_path).ok();

        Self { pid, mem_file }
    }

    /// Get the process ID
    pub fn pid(&self) -> i32 {
        self.pid
    }
}

impl MemoryReader for LinuxMemoryReader {
    fn read_bytes(&self, address: usize, size: usize) -> Option<Vec<u8>> {
        // Try using process_vm_readv first (more efficient)
        let mut buffer = vec![0u8; size];

        let local_iov = libc::iovec {
            iov_base: buffer.as_mut_ptr() as *mut _,
            iov_len: size,
        };

        let remote_iov = libc::iovec {
            iov_base: address as *mut _,
            iov_len: size,
        };

        let result = unsafe {
            libc::process_vm_readv(
                self.pid,
                &local_iov,
                1,
                &remote_iov,
                1,
                0,
            )
        };

        if result == size as isize {
            Some(buffer)
        } else {
            None
        }
    }
}

unsafe impl Send for LinuxMemoryReader {}
unsafe impl Sync for LinuxMemoryReader {}
