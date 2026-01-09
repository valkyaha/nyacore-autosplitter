//! Process finding and module information

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HANDLE;

/// Information about a running process
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    /// Process ID
    pub pid: u32,
    /// Process name
    pub name: String,
    /// Base address of the main module
    pub base_address: usize,
    /// Size of the main module
    pub module_size: usize,
    /// Whether the process is 64-bit
    pub is_64_bit: bool,
}

/// Information about a module loaded in a process
#[derive(Debug, Clone)]
pub struct ModuleInfo {
    /// Module name
    pub name: String,
    /// Base address
    pub base_address: usize,
    /// Size in bytes
    pub size: usize,
}

/// Find a process by name
///
/// Returns process info if found, None otherwise
#[cfg(target_os = "windows")]
pub fn find_process(process_name: &str) -> Option<ProcessInfo> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };

    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).ok()?;

        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };

        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                let name = String::from_utf16_lossy(
                    &entry.szExeFile[..entry
                        .szExeFile
                        .iter()
                        .position(|&c| c == 0)
                        .unwrap_or(entry.szExeFile.len())],
                );

                if name.to_lowercase() == process_name.to_lowercase() {
                    let _ = CloseHandle(snapshot);

                    // Get module info
                    if let Some(module_info) = get_module_info(entry.th32ProcessID, &name) {
                        // Check if 64-bit
                        let is_64_bit = check_is_64_bit(entry.th32ProcessID);

                        return Some(ProcessInfo {
                            pid: entry.th32ProcessID,
                            name,
                            base_address: module_info.base_address,
                            module_size: module_info.size,
                            is_64_bit,
                        });
                    }
                    return None;
                }

                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }

        let _ = CloseHandle(snapshot);
    }

    None
}

/// Find a process by name (Linux implementation)
#[cfg(target_os = "linux")]
pub fn find_process(process_name: &str) -> Option<ProcessInfo> {
    use std::fs;
    use std::path::Path;

    // Iterate through /proc/[pid] directories
    let proc_dir = Path::new("/proc");

    for entry in fs::read_dir(proc_dir).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();

        // Check if this is a process directory (numeric name)
        if let Some(pid_str) = path.file_name().and_then(|n| n.to_str()) {
            if let Ok(pid) = pid_str.parse::<u32>() {
                // Read the process name from /proc/[pid]/comm
                let comm_path = path.join("comm");
                if let Ok(name) = fs::read_to_string(&comm_path) {
                    let name = name.trim();

                    // Also check cmdline for full executable name
                    let cmdline_path = path.join("cmdline");
                    let exe_name = fs::read_to_string(&cmdline_path)
                        .ok()
                        .and_then(|s| s.split('\0').next().map(|s| s.to_string()))
                        .and_then(|s| Path::new(&s).file_name().map(|n| n.to_string_lossy().to_string()))
                        .unwrap_or_else(|| name.to_string());

                    if name.to_lowercase() == process_name.to_lowercase()
                        || exe_name.to_lowercase() == process_name.to_lowercase()
                    {
                        // Get module info from /proc/[pid]/maps
                        if let Some((base, size)) = get_module_base_from_maps(pid) {
                            return Some(ProcessInfo {
                                pid,
                                name: exe_name,
                                base_address: base,
                                module_size: size,
                                is_64_bit: std::mem::size_of::<usize>() == 8,
                            });
                        }
                    }
                }
            }
        }
    }

    None
}

/// Get module information for a process
#[cfg(target_os = "windows")]
pub fn get_module_info(pid: u32, module_name: &str) -> Option<ModuleInfo> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Module32FirstW, Module32NextW, MODULEENTRY32W,
        TH32CS_SNAPMODULE, TH32CS_SNAPMODULE32,
    };

    unsafe {
        let snapshot = CreateToolhelp32Snapshot(
            TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32,
            pid,
        )
        .ok()?;

        let mut entry = MODULEENTRY32W {
            dwSize: std::mem::size_of::<MODULEENTRY32W>() as u32,
            ..Default::default()
        };

        if Module32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                let name = String::from_utf16_lossy(
                    &entry.szModule[..entry
                        .szModule
                        .iter()
                        .position(|&c| c == 0)
                        .unwrap_or(entry.szModule.len())],
                );

                if name.to_lowercase() == module_name.to_lowercase() {
                    let _ = CloseHandle(snapshot);
                    return Some(ModuleInfo {
                        name,
                        base_address: entry.modBaseAddr as usize,
                        size: entry.modBaseSize as usize,
                    });
                }

                if Module32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }

        let _ = CloseHandle(snapshot);
    }

    None
}

/// Get module information (Linux stub - returns main module info)
#[cfg(target_os = "linux")]
pub fn get_module_info(pid: u32, _module_name: &str) -> Option<ModuleInfo> {
    let (base, size) = get_module_base_from_maps(pid)?;
    Some(ModuleInfo {
        name: String::new(),
        base_address: base,
        size,
    })
}

/// Check if a process is 64-bit (Windows)
#[cfg(target_os = "windows")]
fn check_is_64_bit(pid: u32) -> bool {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{
        IsWow64Process, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    unsafe {
        if let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
            let mut is_wow64 = windows::Win32::Foundation::BOOL(0);
            if IsWow64Process(handle, &mut is_wow64).is_ok() {
                let _ = CloseHandle(handle);
                // If it's WOW64, it's a 32-bit process on 64-bit Windows
                // If not WOW64 and we're on 64-bit, it's 64-bit
                return !is_wow64.as_bool();
            }
            let _ = CloseHandle(handle);
        }
    }

    // Default to current process architecture
    std::mem::size_of::<usize>() == 8
}

/// Parse /proc/[pid]/maps to get base address and size
#[cfg(target_os = "linux")]
fn get_module_base_from_maps(pid: u32) -> Option<(usize, usize)> {
    use std::fs;

    let maps_path = format!("/proc/{}/maps", pid);
    let maps = fs::read_to_string(&maps_path).ok()?;

    let mut base_address = None;
    let mut end_address = 0usize;

    for line in maps.lines() {
        // Skip non-executable or special mappings
        if !line.contains("r-x") && !line.contains("r--") {
            continue;
        }

        // Parse the address range
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        let range: Vec<&str> = parts[0].split('-').collect();
        if range.len() != 2 {
            continue;
        }

        let start = usize::from_str_radix(range[0], 16).ok()?;
        let end = usize::from_str_radix(range[1], 16).ok()?;

        if base_address.is_none() {
            base_address = Some(start);
        }
        end_address = end;
    }

    let base = base_address?;
    Some((base, end_address - base))
}

/// Check if a process is still running by its HANDLE
#[cfg(target_os = "windows")]
pub fn is_process_running_by_handle(handle: HANDLE) -> bool {
    use windows::Win32::System::Threading::GetExitCodeProcess;

    // STILL_ACTIVE is 259 (STATUS_PENDING)
    const STILL_ACTIVE: u32 = 259;

    if handle.is_invalid() {
        return false;
    }

    unsafe {
        let mut exit_code = 0u32;
        if GetExitCodeProcess(handle, &mut exit_code).is_ok() {
            return exit_code == STILL_ACTIVE;
        }
    }

    false
}

/// Check if a process is still running by its PID
#[cfg(target_os = "windows")]
pub fn is_process_running(pid: u32) -> bool {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};

    unsafe {
        if let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
            let result = is_process_running_by_handle(handle);
            let _ = CloseHandle(handle);
            return result;
        }
    }

    false
}

/// Check if a process is still running by its PID (Linux)
#[cfg(target_os = "linux")]
pub fn is_process_running(pid: u32) -> bool {
    use std::path::Path;
    Path::new(&format!("/proc/{}", pid)).exists()
}

/// Find a process by any of the given names
/// Returns (pid, name) tuple
#[cfg(target_os = "windows")]
pub fn find_process_by_names(process_names: &[&str]) -> Option<ProcessInfo> {
    for name in process_names {
        if let Some(info) = find_process(name) {
            return Some(info);
        }
    }
    None
}

/// Find a process by any of the given names (Linux)
#[cfg(target_os = "linux")]
pub fn find_process_by_names(process_names: &[&str]) -> Option<ProcessInfo> {
    for name in process_names {
        if let Some(info) = find_process(name) {
            return Some(info);
        }
    }
    None
}
