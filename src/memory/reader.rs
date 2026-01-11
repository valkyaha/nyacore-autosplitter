//! Memory reading utilities for the autosplitter
//!
//! Provides cross-platform memory reading primitives and pattern scanning.
//! - Windows: Uses ReadProcessMemory API
//! - Linux: Uses process_vm_readv syscall (for Proton/Wine games)

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HANDLE;
#[cfg(target_os = "windows")]
use windows::Win32::System::Diagnostics::Debug::ReadProcessMemory;

/// Read raw bytes from process memory
#[cfg(target_os = "windows")]
pub fn read_bytes(handle: HANDLE, address: usize, size: usize) -> Option<Vec<u8>> {
    let mut buffer = vec![0u8; size];
    let mut bytes_read = 0usize;

    unsafe {
        if ReadProcessMemory(
            handle,
            address as *const _,
            buffer.as_mut_ptr() as *mut _,
            size,
            Some(&mut bytes_read),
        )
        .is_ok()
            && bytes_read == size
        {
            return Some(buffer);
        }
    }
    None
}

/// Read a u8 from process memory
#[cfg(target_os = "windows")]
pub fn read_u8(handle: HANDLE, address: usize) -> Option<u8> {
    let bytes = read_bytes(handle, address, 1)?;
    Some(bytes[0])
}

/// Read a u32 from process memory
#[cfg(target_os = "windows")]
pub fn read_u32(handle: HANDLE, address: usize) -> Option<u32> {
    let bytes = read_bytes(handle, address, 4)?;
    Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

/// Read an i32 from process memory
#[cfg(target_os = "windows")]
pub fn read_i32(handle: HANDLE, address: usize) -> Option<i32> {
    let bytes = read_bytes(handle, address, 4)?;
    Some(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

/// Read a u64 from process memory
#[cfg(target_os = "windows")]
pub fn read_u64(handle: HANDLE, address: usize) -> Option<u64> {
    let bytes = read_bytes(handle, address, 8)?;
    Some(u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ]))
}

/// Read an i64 from process memory
#[cfg(target_os = "windows")]
pub fn read_i64(handle: HANDLE, address: usize) -> Option<i64> {
    let bytes = read_bytes(handle, address, 8)?;
    Some(i64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ]))
}

/// Read an f32 from process memory
#[cfg(target_os = "windows")]
pub fn read_f32(handle: HANDLE, address: usize) -> Option<f32> {
    let bytes = read_bytes(handle, address, 4)?;
    Some(f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

/// Read an i16 from process memory
#[cfg(target_os = "windows")]
pub fn read_i16(handle: HANDLE, address: usize) -> Option<i16> {
    let bytes = read_bytes(handle, address, 2)?;
    Some(i16::from_le_bytes([bytes[0], bytes[1]]))
}

/// Read a u16 from process memory
#[cfg(target_os = "windows")]
pub fn read_u16(handle: HANDLE, address: usize) -> Option<u16> {
    let bytes = read_bytes(handle, address, 2)?;
    Some(u16::from_le_bytes([bytes[0], bytes[1]]))
}

/// Read an i8 from process memory
#[cfg(target_os = "windows")]
pub fn read_i8(handle: HANDLE, address: usize) -> Option<i8> {
    let bytes = read_bytes(handle, address, 1)?;
    Some(bytes[0] as i8)
}

/// Read an f64 from process memory
#[cfg(target_os = "windows")]
pub fn read_f64(handle: HANDLE, address: usize) -> Option<f64> {
    let bytes = read_bytes(handle, address, 8)?;
    Some(f64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ]))
}

/// Read a null-terminated string from process memory
#[cfg(target_os = "windows")]
pub fn read_string(handle: HANDLE, address: usize, max_len: usize) -> Option<String> {
    let bytes = read_bytes(handle, address, max_len)?;
    let null_pos = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8(bytes[..null_pos].to_vec()).ok()
}

/// Read a pointer (usize) from process memory
#[cfg(target_os = "windows")]
pub fn read_ptr(handle: HANDLE, address: usize) -> Option<usize> {
    read_u64(handle, address).map(|v| v as usize)
}

/// Scan for a pattern in process memory
#[cfg(target_os = "windows")]
pub fn scan_pattern(
    handle: HANDLE,
    base: usize,
    size: usize,
    pattern: &[Option<u8>],
) -> Option<usize> {
    const CHUNK_SIZE: usize = 0x100000;

    for chunk_start in (0..size).step_by(CHUNK_SIZE) {
        let chunk_end = (chunk_start + CHUNK_SIZE + pattern.len()).min(size);
        let chunk_len = chunk_end - chunk_start;

        if let Some(buffer) = read_bytes(handle, base + chunk_start, chunk_len) {
            if let Some(offset) = find_pattern(&buffer, pattern) {
                return Some(base + chunk_start + offset);
            }
        }
    }
    None
}

/// Find a pattern in a byte buffer
fn find_pattern(data: &[u8], pattern: &[Option<u8>]) -> Option<usize> {
    if pattern.is_empty() || data.len() < pattern.len() {
        return None;
    }

    'outer: for i in 0..=(data.len() - pattern.len()) {
        for (j, &p) in pattern.iter().enumerate() {
            if let Some(b) = p {
                if data[i + j] != b {
                    continue 'outer;
                }
            }
        }
        return Some(i);
    }
    None
}

/// Parse a pattern string into bytes (None = wildcard)
pub fn parse_pattern(pattern_str: &str) -> Vec<Option<u8>> {
    pattern_str
        .split_whitespace()
        .map(|s| {
            if s == "?" || s == "??" {
                None
            } else {
                u8::from_str_radix(s, 16).ok()
            }
        })
        .collect()
}

/// Resolve RIP-relative address from an instruction
#[cfg(target_os = "windows")]
pub fn resolve_rip_relative(
    handle: HANDLE,
    instruction_addr: usize,
    offset_pos: usize,
    instruction_len: usize,
) -> Option<usize> {
    let rel_offset = read_i32(handle, instruction_addr + offset_pos)?;
    let rip = instruction_addr + instruction_len;
    Some((rip as i64 + rel_offset as i64) as usize)
}

// =============================================================================
// Linux Implementation (for Proton/Wine games)
// =============================================================================

/// Read raw bytes from process memory using process_vm_readv (Linux)
///
/// This is the most efficient way to read memory from another process on Linux.
/// It works with both native processes and Wine/Proton processes.
#[cfg(target_os = "linux")]
pub fn read_bytes(pid: i32, address: usize, size: usize) -> Option<Vec<u8>> {
    use std::io::IoSliceMut;

    let mut buffer = vec![0u8; size];

    // Use process_vm_readv syscall for efficient memory reading
    let local_iov = [IoSliceMut::new(&mut buffer)];
    let remote_iov = libc::iovec {
        iov_base: address as *mut libc::c_void,
        iov_len: size,
    };

    let bytes_read = unsafe {
        libc::process_vm_readv(
            pid,
            local_iov.as_ptr() as *const libc::iovec,
            1,
            &remote_iov,
            1,
            0,
        )
    };

    if bytes_read == size as isize {
        Some(buffer)
    } else {
        // Fallback: try reading via /proc/[pid]/mem
        read_bytes_via_proc_mem(pid, address, size)
    }
}

/// Fallback memory reading via /proc/[pid]/mem
#[cfg(target_os = "linux")]
fn read_bytes_via_proc_mem(pid: i32, address: usize, size: usize) -> Option<Vec<u8>> {
    use std::fs::File;
    use std::io::{Read, Seek, SeekFrom};

    let mem_path = format!("/proc/{}/mem", pid);
    let mut file = File::open(&mem_path).ok()?;

    file.seek(SeekFrom::Start(address as u64)).ok()?;

    let mut buffer = vec![0u8; size];
    let bytes_read = file.read(&mut buffer).ok()?;

    if bytes_read == size {
        Some(buffer)
    } else {
        None
    }
}

/// Read a u8 from process memory (Linux)
#[cfg(target_os = "linux")]
pub fn read_u8(pid: i32, address: usize) -> Option<u8> {
    let bytes = read_bytes(pid, address, 1)?;
    Some(bytes[0])
}

/// Read a u32 from process memory (Linux)
#[cfg(target_os = "linux")]
pub fn read_u32(pid: i32, address: usize) -> Option<u32> {
    let bytes = read_bytes(pid, address, 4)?;
    Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

/// Read an i32 from process memory (Linux)
#[cfg(target_os = "linux")]
pub fn read_i32(pid: i32, address: usize) -> Option<i32> {
    let bytes = read_bytes(pid, address, 4)?;
    Some(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

/// Read a u64 from process memory (Linux)
#[cfg(target_os = "linux")]
pub fn read_u64(pid: i32, address: usize) -> Option<u64> {
    let bytes = read_bytes(pid, address, 8)?;
    Some(u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ]))
}

/// Read an i64 from process memory (Linux)
#[cfg(target_os = "linux")]
pub fn read_i64(pid: i32, address: usize) -> Option<i64> {
    let bytes = read_bytes(pid, address, 8)?;
    Some(i64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ]))
}

/// Read an f32 from process memory (Linux)
#[cfg(target_os = "linux")]
pub fn read_f32(pid: i32, address: usize) -> Option<f32> {
    let bytes = read_bytes(pid, address, 4)?;
    Some(f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

/// Read an i16 from process memory (Linux)
#[cfg(target_os = "linux")]
pub fn read_i16(pid: i32, address: usize) -> Option<i16> {
    let bytes = read_bytes(pid, address, 2)?;
    Some(i16::from_le_bytes([bytes[0], bytes[1]]))
}

/// Read a u16 from process memory (Linux)
#[cfg(target_os = "linux")]
pub fn read_u16(pid: i32, address: usize) -> Option<u16> {
    let bytes = read_bytes(pid, address, 2)?;
    Some(u16::from_le_bytes([bytes[0], bytes[1]]))
}

/// Read an i8 from process memory (Linux)
#[cfg(target_os = "linux")]
pub fn read_i8(pid: i32, address: usize) -> Option<i8> {
    let bytes = read_bytes(pid, address, 1)?;
    Some(bytes[0] as i8)
}

/// Read an f64 from process memory (Linux)
#[cfg(target_os = "linux")]
pub fn read_f64(pid: i32, address: usize) -> Option<f64> {
    let bytes = read_bytes(pid, address, 8)?;
    Some(f64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ]))
}

/// Read a null-terminated string from process memory (Linux)
#[cfg(target_os = "linux")]
pub fn read_string(pid: i32, address: usize, max_len: usize) -> Option<String> {
    let bytes = read_bytes(pid, address, max_len)?;
    let null_pos = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8(bytes[..null_pos].to_vec()).ok()
}

/// Read a pointer (usize) from process memory (Linux)
#[cfg(target_os = "linux")]
pub fn read_ptr(pid: i32, address: usize) -> Option<usize> {
    read_u64(pid, address).map(|v| v as usize)
}

/// Scan for a pattern in process memory (Linux)
#[cfg(target_os = "linux")]
pub fn scan_pattern(
    pid: i32,
    base: usize,
    size: usize,
    pattern: &[Option<u8>],
) -> Option<usize> {
    const CHUNK_SIZE: usize = 0x100000;

    for chunk_start in (0..size).step_by(CHUNK_SIZE) {
        let chunk_end = (chunk_start + CHUNK_SIZE + pattern.len()).min(size);
        let chunk_len = chunk_end - chunk_start;

        if let Some(buffer) = read_bytes(pid, base + chunk_start, chunk_len) {
            if let Some(offset) = find_pattern(&buffer, pattern) {
                return Some(base + chunk_start + offset);
            }
        }
    }
    None
}

/// Resolve RIP-relative address from an instruction (Linux)
#[cfg(target_os = "linux")]
pub fn resolve_rip_relative(
    pid: i32,
    instruction_addr: usize,
    offset_pos: usize,
    instruction_len: usize,
) -> Option<usize> {
    let rel_offset = read_i32(pid, instruction_addr + offset_pos)?;
    let rip = instruction_addr + instruction_len;
    Some((rip as i64 + rel_offset as i64) as usize)
}
