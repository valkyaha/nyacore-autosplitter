//! Process finding utilities for the autosplitter
//!
//! Provides cross-platform process detection for the autosplitter.
//! - Windows: Uses Windows API (CreateToolhelp32Snapshot, etc.)
//! - Linux: Parses /proc filesystem for process info (supports Proton/Wine games)

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{CloseHandle, HANDLE};
#[cfg(target_os = "windows")]
use windows::Win32::System::Diagnostics::ToolHelp::*;

#[cfg(target_os = "linux")]
use std::fs;
#[cfg(target_os = "linux")]
use std::path::Path;

/// Find a process by name from a list of target names
/// Returns (pid, process_name) if found
#[cfg(target_os = "windows")]
pub fn find_process_by_name(target_names: &[&str]) -> Option<(u32, String)> {
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).ok()?;

        let mut entry = PROCESSENTRY32W::default();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                let name = String::from_utf16_lossy(&entry.szExeFile)
                    .trim_end_matches('\0')
                    .to_lowercase();

                // Compare against each target (case-insensitive)
                for target in target_names {
                    let target_lower = target.to_lowercase();
                    // Match either full name or name without .exe suffix
                    if name == target_lower || name == format!("{}.exe", target_lower.trim_end_matches(".exe")) {
                        let pid = entry.th32ProcessID;
                        let _ = CloseHandle(snapshot);
                        return Some((pid, name));
                    }
                }

                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }

        let _ = CloseHandle(snapshot);
        None
    }
}

/// Get the base address and size of a process's main module
#[cfg(target_os = "windows")]
pub fn get_module_base_and_size(pid: u32) -> Option<(usize, usize)> {
    unsafe {
        let snapshot =
            CreateToolhelp32Snapshot(TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32, pid).ok()?;

        let mut entry = MODULEENTRY32W::default();
        entry.dwSize = std::mem::size_of::<MODULEENTRY32W>() as u32;

        if Module32FirstW(snapshot, &mut entry).is_ok() {
            let base = entry.modBaseAddr as usize;
            let size = entry.modBaseSize as usize;
            let _ = CloseHandle(snapshot);
            return Some((base, size));
        }

        let _ = CloseHandle(snapshot);
        None
    }
}

/// Check if a process is still running
#[cfg(target_os = "windows")]
pub fn is_process_running(handle: HANDLE) -> bool {
    unsafe {
        let mut exit_code: u32 = 0;
        if windows::Win32::System::Threading::GetExitCodeProcess(handle, &mut exit_code).is_ok() {
            return exit_code == 259; // STILL_ACTIVE
        }
        false
    }
}

// =============================================================================
// Linux Implementation (for Proton/Wine games)
// =============================================================================

/// Find a process by name from a list of target names (Linux)
/// Returns (pid, process_name) if found
///
/// This works with both native Linux processes and Wine/Proton processes.
/// For Proton games, the process name is typically the Windows executable name.
#[cfg(target_os = "linux")]
pub fn find_process_by_name(target_names: &[&str]) -> Option<(u32, String)> {
    let proc_dir = Path::new("/proc");

    // Read all entries in /proc
    let entries = match fs::read_dir(proc_dir) {
        Ok(e) => e,
        Err(_) => return None,
    };

    for entry in entries.flatten() {
        let path = entry.path();

        // Only process numeric directories (PIDs)
        let pid_str = match path.file_name().and_then(|s| s.to_str()) {
            Some(s) => s,
            None => continue,
        };
        let pid: u32 = match pid_str.parse() {
            Ok(p) => p,
            Err(_) => continue, // Skip non-numeric entries like /proc/self, /proc/bus, etc.
        };

        // Try multiple methods to get process name
        // Method 1: Read /proc/[pid]/comm (simple process name)
        if let Some(name) = read_proc_comm(pid) {
            if matches_target(&name, target_names) {
                return Some((pid, name));
            }
        }

        // Method 2: Read /proc/[pid]/cmdline (full command line, useful for Wine)
        if let Some(name) = read_proc_cmdline_exe(pid) {
            if matches_target(&name, target_names) {
                return Some((pid, name));
            }
        }

        // Method 3: Read /proc/[pid]/exe symlink (actual executable)
        if let Some(name) = read_proc_exe(pid) {
            if matches_target(&name, target_names) {
                return Some((pid, name));
            }
        }
    }

    None
}

/// Check if process name matches any target (case-insensitive)
#[cfg(target_os = "linux")]
fn matches_target(name: &str, target_names: &[&str]) -> bool {
    let name_lower = name.to_lowercase();

    for target in target_names {
        let target_lower = target.to_lowercase();
        let target_no_ext = target_lower.trim_end_matches(".exe");

        // Match full name, name without .exe, or if name contains the target
        if name_lower == target_lower
            || name_lower == format!("{}.exe", target_no_ext)
            || name_lower == target_no_ext
            || name_lower.ends_with(&format!("/{}", target_lower))
            || name_lower.ends_with(&format!("/{}.exe", target_no_ext))
        {
            return true;
        }
    }
    false
}

/// Read process name from /proc/[pid]/comm
#[cfg(target_os = "linux")]
fn read_proc_comm(pid: u32) -> Option<String> {
    let path = format!("/proc/{}/comm", pid);
    fs::read_to_string(&path)
        .ok()
        .map(|s| s.trim().to_string())
}

/// Read executable name from /proc/[pid]/cmdline (useful for Wine processes)
#[cfg(target_os = "linux")]
fn read_proc_cmdline_exe(pid: u32) -> Option<String> {
    let path = format!("/proc/{}/cmdline", pid);
    let cmdline = fs::read_to_string(&path).ok()?;

    // cmdline is null-separated, get the first argument (executable)
    let exe = cmdline.split('\0').next()?;

    // Extract just the filename from the path
    let filename = exe.rsplit(['/', '\\']).next()?;

    if filename.is_empty() {
        None
    } else {
        Some(filename.to_string())
    }
}

/// Read executable path from /proc/[pid]/exe symlink
#[cfg(target_os = "linux")]
fn read_proc_exe(pid: u32) -> Option<String> {
    let path = format!("/proc/{}/exe", pid);
    let exe_path = fs::read_link(&path).ok()?;

    // Extract just the filename
    let filename = exe_path.file_name()?.to_str()?;
    Some(filename.to_string())
}

/// Get the base address and size of a process's main module (Linux)
///
/// For Proton/Wine games, this parses /proc/[pid]/maps to find the executable mapping,
/// then reads the PE header to get the actual module size (SizeOfImage).
#[cfg(target_os = "linux")]
pub fn get_module_base_and_size(pid: u32) -> Option<(usize, usize)> {
    let maps_path = format!("/proc/{}/maps", pid);
    let maps = fs::read_to_string(&maps_path).ok()?;

    // For Wine/Proton, we need to find the main executable's mapping
    // First, try to get the executable name from cmdline (most reliable for Wine)
    let exe_name = read_proc_cmdline_exe(pid)
        .unwrap_or_default()
        .to_lowercase();

    let mut base_addr: Option<usize> = None;

    // First pass: look for .exe mapping (Wine/Proton games)
    for line in maps.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 6 {
            continue;
        }

        let addr_range = parts[0];
        // Join parts[5..] to handle paths with spaces (e.g., "ELDEN RING")
        let pathname = parts[5..].join(" ");

        // Look for .exe file mapping - this is the game executable
        let pathname_lower = pathname.to_lowercase();
        let is_target_exe = (!exe_name.is_empty() && pathname_lower.contains(&exe_name))
            || pathname_lower.ends_with(".exe");

        if is_target_exe {
            let addrs: Vec<&str> = addr_range.split('-').collect();
            if addrs.len() == 2 {
                if let Ok(start) = usize::from_str_radix(addrs[0], 16) {
                    base_addr = Some(start);
                    log::debug!("Found .exe mapping at 0x{:x}: {}", start, pathname);
                    break;
                }
            }
        }
    }

    // If we found the base, read the PE header to get actual module size
    if let Some(base) = base_addr {
        if let Some(size) = read_pe_image_size(pid as i32, base) {
            log::debug!("PE SizeOfImage: 0x{:x} ({:.2} MB)", size, size as f64 / (1024.0 * 1024.0));
            return Some((base, size));
        }
        // Fallback: use a large default size for games (100MB)
        log::warn!("Could not read PE header, using default size");
        return Some((base, 0x6400000));
    }

    // Fallback: look for first large executable region
    if let Some(base) = find_first_executable_region(pid) {
        log::debug!("Using fallback executable region at 0x{:x}", base);
        return Some((base, 0x4000000));
    }

    None
}

/// Read the SizeOfImage from a PE header in process memory (Linux)
#[cfg(target_os = "linux")]
fn read_pe_image_size(pid: i32, base: usize) -> Option<usize> {
    use super::memory::read_bytes;

    // Read DOS header (first 64 bytes)
    let dos_header = read_bytes(pid, base, 64)?;

    // Check MZ signature
    if dos_header.len() < 64 || dos_header[0] != b'M' || dos_header[1] != b'Z' {
        log::debug!("Invalid MZ signature at 0x{:x}", base);
        return None;
    }

    // Get PE header offset from DOS header (at offset 0x3C)
    let pe_offset = u32::from_le_bytes([
        dos_header[0x3C],
        dos_header[0x3D],
        dos_header[0x3E],
        dos_header[0x3F],
    ]) as usize;

    // Read PE header and optional header (first 256 bytes should be enough)
    let pe_header = read_bytes(pid, base + pe_offset, 256)?;

    // Check PE signature
    if pe_header.len() < 256 || pe_header[0] != b'P' || pe_header[1] != b'E' {
        log::debug!("Invalid PE signature at 0x{:x}", base + pe_offset);
        return None;
    }

    // PE64 optional header starts at offset 24 from PE signature
    // SizeOfImage is at offset 56 in the optional header (24 + 56 = 80 from PE signature)
    let size_of_image = u32::from_le_bytes([
        pe_header[24 + 56],
        pe_header[24 + 57],
        pe_header[24 + 58],
        pe_header[24 + 59],
    ]) as usize;

    if size_of_image > 0 && size_of_image < 0x100000000 {
        Some(size_of_image)
    } else {
        log::debug!("Invalid SizeOfImage: 0x{:x}", size_of_image);
        None
    }
}

/// Find the first executable region in process memory
#[cfg(target_os = "linux")]
fn find_first_executable_region(pid: u32) -> Option<usize> {
    let maps_path = format!("/proc/{}/maps", pid);
    let maps = fs::read_to_string(&maps_path).ok()?;

    for line in maps.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }

        let perms = parts[1];

        // Look for executable regions with addresses in typical game range (>= 0x10000)
        if perms.contains('x') {
            let addrs: Vec<&str> = parts[0].split('-').collect();
            if let Ok(start) = usize::from_str_radix(addrs[0], 16) {
                if start >= 0x10000 {
                    return Some(start);
                }
            }
        }
    }
    None
}

/// Check if a process is still running (Linux)
/// On Linux, we use the PID directly instead of a handle
#[cfg(target_os = "linux")]
pub fn is_process_running_by_pid(pid: u32) -> bool {
    // Check if /proc/[pid] exists
    let proc_path = format!("/proc/{}", pid);
    Path::new(&proc_path).exists()
}

/// Open a process for memory reading (Linux)
/// Returns the PID if successful (we don't need a handle on Linux)
#[cfg(target_os = "linux")]
pub fn open_process(pid: u32) -> Option<i32> {
    // On Linux, we just need to verify the process exists
    // and that we have permission to read its memory
    let mem_path = format!("/proc/{}/mem", pid);

    // Check if we can access the process memory file
    if Path::new(&mem_path).exists() {
        Some(pid as i32)
    } else {
        None
    }
}
