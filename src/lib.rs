//! NYA Core Autosplitter
//!
//! A standalone autosplitter library for FromSoftware games.
//! Supports Dark Souls 1/2/3, Elden Ring, Sekiro, and Armored Core 6.
//!
//! This crate can be used as:
//! - A Rust library (rlib) for direct integration
//! - A dynamic library (cdylib) for FFI-based loading

pub mod config;
pub mod games;
pub mod memory;

#[cfg(feature = "vision")]
pub mod vision;

// Re-export commonly used types
pub use config::{AutosplitterState, BossFlag};
pub use games::{ArmoredCore6, DarkSouls1, DarkSouls2, DarkSouls3, EldenRing, Sekiro};
pub use memory::{parse_pattern, scan_pattern, resolve_rip_relative};

use std::collections::HashMap;
use std::ffi::CString;
use std::os::raw::c_char;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::Duration;

use once_cell::sync::Lazy;

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{CloseHandle, HANDLE};
#[cfg(target_os = "windows")]
use windows::Win32::System::Threading::{
    GetProcessId, OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
};

/// Supported game types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameType {
    DarkSouls1,
    DarkSouls2,
    DarkSouls3,
    EldenRing,
    Sekiro,
    ArmoredCore6,
}

impl GameType {
    /// Get game type from process name
    pub fn from_process_name(name: &str) -> Option<Self> {
        let name_lower = name.to_lowercase();
        if name_lower.contains("darksoulsremastered") {
            Some(GameType::DarkSouls1)
        } else if name_lower.contains("darksoulsiii") {
            Some(GameType::DarkSouls3)
        } else if name_lower.contains("darksoulsii") {
            Some(GameType::DarkSouls2)
        } else if name_lower.contains("eldenring") {
            Some(GameType::EldenRing)
        } else if name_lower.contains("sekiro") {
            Some(GameType::Sekiro)
        } else if name_lower.contains("armoredcore6") {
            Some(GameType::ArmoredCore6)
        } else {
            None
        }
    }

    /// Get process names for this game
    pub fn process_names(&self) -> &'static [&'static str] {
        match self {
            GameType::DarkSouls1 => &["DarkSoulsRemastered.exe"],
            GameType::DarkSouls2 => &["DarkSoulsII.exe"],
            GameType::DarkSouls3 => &["DarkSoulsIII.exe"],
            GameType::EldenRing => &["eldenring.exe"],
            GameType::Sekiro => &["sekiro.exe"],
            GameType::ArmoredCore6 => &["armoredcore6.exe"],
        }
    }

    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            GameType::DarkSouls1 => "Dark Souls Remastered",
            GameType::DarkSouls2 => "Dark Souls II: Scholar of the First Sin",
            GameType::DarkSouls3 => "Dark Souls III",
            GameType::EldenRing => "Elden Ring",
            GameType::Sekiro => "Sekiro: Shadows Die Twice",
            GameType::ArmoredCore6 => "Armored Core VI: Fires of Rubicon",
        }
    }
}

/// Game state holder for any supported game
#[cfg(target_os = "windows")]
enum GameState {
    DarkSouls1(DarkSouls1),
    DarkSouls2(DarkSouls2),
    DarkSouls3(DarkSouls3),
    EldenRing(EldenRing),
    Sekiro(Sekiro),
    ArmoredCore6(ArmoredCore6),
}

#[cfg(target_os = "windows")]
impl GameState {
    fn read_event_flag(&self, flag_id: u32) -> bool {
        match self {
            GameState::DarkSouls1(g) => g.read_event_flag(flag_id),
            GameState::DarkSouls2(g) => g.read_event_flag(flag_id),
            GameState::DarkSouls3(g) => g.read_event_flag(flag_id),
            GameState::EldenRing(g) => g.read_event_flag(flag_id),
            GameState::Sekiro(g) => g.read_event_flag(flag_id),
            GameState::ArmoredCore6(g) => g.read_event_flag(flag_id),
        }
    }

    fn get_boss_kill_count(&self, flag_id: u32) -> u32 {
        match self {
            GameState::DarkSouls2(g) => g.get_boss_kill_count_raw(flag_id).max(0) as u32,
            _ => {
                if self.read_event_flag(flag_id) {
                    1
                } else {
                    0
                }
            }
        }
    }

    fn get_handle(&self) -> HANDLE {
        match self {
            GameState::DarkSouls1(g) => g.handle,
            GameState::DarkSouls2(g) => g.handle,
            GameState::DarkSouls3(g) => g.handle,
            GameState::EldenRing(g) => g.handle,
            GameState::Sekiro(g) => g.handle,
            GameState::ArmoredCore6(g) => g.handle,
        }
    }

    fn name(&self) -> &'static str {
        match self {
            GameState::DarkSouls1(_) => "Dark Souls Remastered",
            GameState::DarkSouls2(_) => "Dark Souls 2 SOTFS",
            GameState::DarkSouls3(_) => "Dark Souls 3",
            GameState::EldenRing(_) => "Elden Ring",
            GameState::Sekiro(_) => "Sekiro",
            GameState::ArmoredCore6(_) => "Armored Core 6",
        }
    }
}

/// Initialize game from process info
#[cfg(target_os = "windows")]
fn init_game(
    game_type: GameType,
    handle: HANDLE,
    base: usize,
    size: usize,
) -> Option<GameState> {
    match game_type {
        GameType::DarkSouls1 => {
            let mut game = DarkSouls1::new();
            if game.init_pointers(handle, base, size) {
                Some(GameState::DarkSouls1(game))
            } else {
                None
            }
        }
        GameType::DarkSouls2 => {
            let mut game = DarkSouls2::new();
            if game.init_pointers(handle, base, size) {
                Some(GameState::DarkSouls2(game))
            } else {
                None
            }
        }
        GameType::DarkSouls3 => {
            let mut game = DarkSouls3::new();
            if game.init_pointers(handle, base, size) {
                Some(GameState::DarkSouls3(game))
            } else {
                None
            }
        }
        GameType::EldenRing => {
            let mut game = EldenRing::new();
            if game.init_pointers(handle, base, size) {
                Some(GameState::EldenRing(game))
            } else {
                None
            }
        }
        GameType::Sekiro => {
            let mut game = Sekiro::new();
            if game.init_pointers(handle, base, size) {
                Some(GameState::Sekiro(game))
            } else {
                None
            }
        }
        GameType::ArmoredCore6 => {
            let mut game = ArmoredCore6::new();
            if game.init_pointers(handle, base, size) {
                Some(GameState::ArmoredCore6(game))
            } else {
                None
            }
        }
    }
}

/// Main Autosplitter instance
pub struct Autosplitter {
    state: Arc<Mutex<AutosplitterState>>,
    running: Arc<AtomicBool>,
    reset_requested: Arc<AtomicBool>,
}

unsafe impl Send for Autosplitter {}
unsafe impl Sync for Autosplitter {}

impl Default for Autosplitter {
    fn default() -> Self {
        Self::new()
    }
}

impl Autosplitter {
    /// Create a new autosplitter instance
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(AutosplitterState::default())),
            running: Arc::new(AtomicBool::new(false)),
            reset_requested: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Get current state
    pub fn get_state(&self) -> AutosplitterState {
        self.state.lock().unwrap().clone()
    }

    /// Check if running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Stop the autosplitter
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        let mut state = self.state.lock().unwrap();
        state.running = false;
        state.process_attached = false;
        state.process_id = None;
        log::info!("Autosplitter stopped");
    }

    /// Reset the autosplitter (re-check all flags)
    pub fn reset(&self) {
        self.reset_requested.store(true, Ordering::SeqCst);
        let mut state = self.state.lock().unwrap();
        state.bosses_defeated.clear();
        state.boss_kill_counts.clear();
        log::info!("Autosplitter reset - will re-check all flags");
    }

    /// Get list of defeated boss IDs
    pub fn get_defeated_bosses(&self) -> Vec<String> {
        self.state.lock().unwrap().bosses_defeated.clone()
    }

    /// Start autosplitter for a specific game with boss flags
    #[cfg(target_os = "windows")]
    pub fn start(
        &self,
        game_type: GameType,
        boss_flags: Vec<BossFlag>,
    ) -> Result<(), String> {
        if self.running.load(Ordering::SeqCst) {
            return Err("Autosplitter already running".to_string());
        }

        if boss_flags.is_empty() {
            return Err("No boss flags defined".to_string());
        }

        log::info!(
            "Starting autosplitter for {} with {} boss flags",
            game_type.display_name(),
            boss_flags.len()
        );

        self.running.store(true, Ordering::SeqCst);

        {
            let mut state = self.state.lock().unwrap();
            state.running = true;
            state.process_attached = false;
            state.game_id = format!("{:?}", game_type);
            state.process_id = None;
            state.bosses_defeated.clear();
            state.boss_kill_counts.clear();
        }

        let running = self.running.clone();
        let state = self.state.clone();
        let reset_requested = self.reset_requested.clone();
        let process_names: Vec<String> = game_type
            .process_names()
            .iter()
            .map(|s| s.to_string())
            .collect();

        thread::spawn(move || {
            log::info!("Autosplitter thread started");
            run_autosplitter_loop(
                running,
                state,
                reset_requested,
                game_type,
                process_names,
                boss_flags,
            );
        });

        Ok(())
    }

    #[cfg(target_os = "linux")]
    pub fn start(
        &self,
        _game_type: GameType,
        _boss_flags: Vec<BossFlag>,
    ) -> Result<(), String> {
        Err("Linux support not yet implemented in standalone library".to_string())
    }
}

// =============================================================================
// Main Loop (Windows)
// =============================================================================

#[cfg(target_os = "windows")]
fn run_autosplitter_loop(
    running: Arc<AtomicBool>,
    state: Arc<Mutex<AutosplitterState>>,
    reset_requested: Arc<AtomicBool>,
    game_type: GameType,
    process_names: Vec<String>,
    boss_flags: Vec<BossFlag>,
) {
    let mut game_state: Option<GameState> = None;
    let mut current_handle: Option<HANDLE> = None;
    let mut checked_flags: HashMap<u32, bool> = HashMap::new();

    while running.load(Ordering::SeqCst) {
        // Check for reset
        if reset_requested.swap(false, Ordering::SeqCst) {
            log::info!("Autosplitter: Reset detected");
            if let Some(ref game) = game_state {
                checked_flags.clear();
                for boss in &boss_flags {
                    if game.read_event_flag(boss.flag_id) {
                        checked_flags.insert(boss.flag_id, true);
                    }
                }
            } else {
                checked_flags.clear();
            }
            let mut s = state.lock().unwrap();
            s.bosses_defeated.clear();
            s.boss_kill_counts.clear();
            s.triggers_matched.clear();
        }

        if let Some(ref game) = game_state {
            // Check if process still running
            if !memory::process::is_process_running(game.get_handle()) {
                log::info!("{} process exited", game.name());
                if let Some(handle) = current_handle.take() {
                    unsafe {
                        let _ = CloseHandle(handle);
                    }
                }
                game_state = None;
                checked_flags.clear();

                let mut s = state.lock().unwrap();
                s.process_attached = false;
                s.process_id = None;
                s.bosses_defeated.clear();
                s.boss_kill_counts.clear();
                thread::sleep(Duration::from_millis(1000));
                continue;
            }

            // Check boss flags
            for boss in &boss_flags {
                let kill_count = game.get_boss_kill_count(boss.flag_id);

                if kill_count > 0 {
                    let mut s = state.lock().unwrap();

                    let prev_count = s.boss_kill_counts.get(&boss.boss_id).copied().unwrap_or(0);
                    if kill_count > prev_count {
                        s.boss_kill_counts.insert(boss.boss_id.clone(), kill_count);
                        log::info!(
                            "Boss kill count updated: {} - count: {} -> {}",
                            boss.boss_name,
                            prev_count,
                            kill_count
                        );
                    }

                    if !s.bosses_defeated.contains(&boss.boss_id) {
                        s.bosses_defeated.push(boss.boss_id.clone());
                        checked_flags.insert(boss.flag_id, true);
                        log::info!(
                            "Boss defeated: {} (id={}, flag={})",
                            boss.boss_name,
                            boss.boss_id,
                            boss.flag_id
                        );
                    }
                }
            }
        } else {
            // Try to connect
            let process_name_refs: Vec<&str> = process_names.iter().map(|s| s.as_str()).collect();
            if let Some((pid, name)) = memory::process::find_process_by_name(&process_name_refs) {
                let handle = unsafe {
                    match OpenProcess(PROCESS_VM_READ | PROCESS_QUERY_INFORMATION, false, pid) {
                        Ok(h) => h,
                        Err(_) => {
                            thread::sleep(Duration::from_millis(2000));
                            continue;
                        }
                    }
                };

                // Get module info
                let mut base = 0usize;
                let mut size = 0usize;
                for attempt in 0..5 {
                    if let Some((b, s)) = memory::process::get_module_base_and_size(pid) {
                        base = b;
                        size = s;
                        break;
                    }
                    if attempt < 4 {
                        thread::sleep(Duration::from_millis(500));
                    }
                }

                if base == 0 {
                    log::warn!("Failed to get module info for {}", name);
                    unsafe {
                        let _ = CloseHandle(handle);
                    }
                    thread::sleep(Duration::from_millis(2000));
                    continue;
                }

                log::info!(
                    "Found '{}' (PID: {}), base=0x{:X}, size=0x{:X}",
                    name,
                    pid,
                    base,
                    size
                );

                // Initialize game
                if let Some(game) = init_game(game_type, handle, base, size) {
                    log::info!("Connected to {}", game.name());

                    // Wait for save data to stabilize
                    log::info!("Waiting for game save data to stabilize...");
                    thread::sleep(Duration::from_millis(1500));

                    // Pre-populate checked flags
                    checked_flags.clear();
                    let mut pre_populated = Vec::new();
                    for boss in &boss_flags {
                        if game.read_event_flag(boss.flag_id) {
                            checked_flags.insert(boss.flag_id, true);
                            pre_populated.push(boss.boss_name.clone());
                        }
                    }

                    if !pre_populated.is_empty() {
                        log::info!(
                            "Pre-populated {} already-defeated bosses",
                            pre_populated.len()
                        );
                    }

                    game_state = Some(game);
                    current_handle = Some(handle);

                    let mut s = state.lock().unwrap();
                    s.process_attached = true;
                    s.process_id = Some(unsafe { GetProcessId(handle) });
                } else {
                    log::error!("Failed to initialize game for {}", name);
                    unsafe {
                        let _ = CloseHandle(handle);
                    }
                    thread::sleep(Duration::from_millis(2000));
                }
            } else {
                thread::sleep(Duration::from_millis(2000));
            }
        }

        thread::sleep(Duration::from_millis(100));
    }

    // Cleanup
    if let Some(handle) = current_handle {
        unsafe {
            let _ = CloseHandle(handle);
        }
    }

    let mut s = state.lock().unwrap();
    s.running = false;
    s.process_attached = false;
    s.process_id = None;
}

// =============================================================================
// FFI Interface for Dynamic Loading
// =============================================================================

static AUTOSPLITTER: Lazy<Mutex<Option<Autosplitter>>> = Lazy::new(|| Mutex::new(None));

/// Initialize the autosplitter (call once at startup)
#[no_mangle]
pub extern "C" fn autosplitter_init() -> bool {
    let mut guard = AUTOSPLITTER.lock().unwrap();
    if guard.is_none() {
        *guard = Some(Autosplitter::new());
        true
    } else {
        false
    }
}

/// Check if autosplitter is initialized
#[no_mangle]
pub extern "C" fn autosplitter_is_initialized() -> bool {
    AUTOSPLITTER.lock().unwrap().is_some()
}

/// Stop the autosplitter
#[no_mangle]
pub extern "C" fn autosplitter_stop() {
    if let Some(ref autosplitter) = *AUTOSPLITTER.lock().unwrap() {
        autosplitter.stop();
    }
}

/// Reset the autosplitter
#[no_mangle]
pub extern "C" fn autosplitter_reset() {
    if let Some(ref autosplitter) = *AUTOSPLITTER.lock().unwrap() {
        autosplitter.reset();
    }
}

/// Check if autosplitter is running
#[no_mangle]
pub extern "C" fn autosplitter_is_running() -> bool {
    AUTOSPLITTER
        .lock()
        .unwrap()
        .as_ref()
        .map(|a| a.is_running())
        .unwrap_or(false)
}

/// Get autosplitter state as JSON string
/// Caller must free the returned string with autosplitter_free_string
#[no_mangle]
pub extern "C" fn autosplitter_get_state_json() -> *mut c_char {
    let state = AUTOSPLITTER
        .lock()
        .unwrap()
        .as_ref()
        .map(|a| a.get_state())
        .unwrap_or_default();

    let json = serde_json::to_string(&state).unwrap_or_else(|_| "{}".to_string());
    CString::new(json).unwrap().into_raw()
}

/// Free a string returned by the autosplitter
#[no_mangle]
pub extern "C" fn autosplitter_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            let _ = CString::from_raw(s);
        }
    }
}

/// Get library version
#[no_mangle]
pub extern "C" fn autosplitter_version() -> *const c_char {
    static VERSION: &[u8] = b"0.1.0\0";
    VERSION.as_ptr() as *const c_char
}

/// Start autosplitter for a specific game
/// game_type: "DarkSouls1", "DarkSouls2", "DarkSouls3", "EldenRing", "Sekiro", "ArmoredCore6"
/// boss_flags_json: JSON array of BossFlag objects
/// Returns error message or null on success (caller must free error string)
#[no_mangle]
pub extern "C" fn autosplitter_start(
    game_type: *const c_char,
    boss_flags_json: *const c_char,
) -> *mut c_char {
    if game_type.is_null() || boss_flags_json.is_null() {
        return CString::new("Null pointer passed").unwrap().into_raw();
    }

    let game_type_str = unsafe { std::ffi::CStr::from_ptr(game_type).to_string_lossy() };
    let boss_flags_str = unsafe { std::ffi::CStr::from_ptr(boss_flags_json).to_string_lossy() };

    let game = match game_type_str.as_ref() {
        "DarkSouls1" => GameType::DarkSouls1,
        "DarkSouls2" => GameType::DarkSouls2,
        "DarkSouls3" => GameType::DarkSouls3,
        "EldenRing" => GameType::EldenRing,
        "Sekiro" => GameType::Sekiro,
        "ArmoredCore6" => GameType::ArmoredCore6,
        _ => return CString::new(format!("Unknown game type: {}", game_type_str)).unwrap().into_raw(),
    };

    let boss_flags: Vec<BossFlag> = match serde_json::from_str(&boss_flags_str) {
        Ok(flags) => flags,
        Err(e) => return CString::new(format!("Failed to parse boss flags: {}", e)).unwrap().into_raw(),
    };

    let guard = AUTOSPLITTER.lock().unwrap();
    let autosplitter = match guard.as_ref() {
        Some(a) => a,
        None => return CString::new("Autosplitter not initialized").unwrap().into_raw(),
    };

    match autosplitter.start(game, boss_flags) {
        Ok(()) => std::ptr::null_mut(), // null means success
        Err(e) => CString::new(e).unwrap().into_raw(),
    }
}

/// Start autosplitter in autodetect mode (scans for any supported game)
/// process_names_json: JSON array of process names to watch for
/// boss_flags_json: JSON array of BossFlag objects
/// Returns error message or null on success (caller must free error string)
#[no_mangle]
pub extern "C" fn autosplitter_start_autodetect(
    process_names_json: *const c_char,
    boss_flags_json: *const c_char,
) -> *mut c_char {
    if process_names_json.is_null() || boss_flags_json.is_null() {
        return CString::new("Null pointer passed").unwrap().into_raw();
    }

    let process_names_str = unsafe { std::ffi::CStr::from_ptr(process_names_json).to_string_lossy() };
    let boss_flags_str = unsafe { std::ffi::CStr::from_ptr(boss_flags_json).to_string_lossy() };

    let process_names: Vec<String> = match serde_json::from_str(&process_names_str) {
        Ok(names) => names,
        Err(e) => return CString::new(format!("Failed to parse process names: {}", e)).unwrap().into_raw(),
    };

    let boss_flags: Vec<BossFlag> = match serde_json::from_str(&boss_flags_str) {
        Ok(flags) => flags,
        Err(e) => return CString::new(format!("Failed to parse boss flags: {}", e)).unwrap().into_raw(),
    };

    let guard = AUTOSPLITTER.lock().unwrap();
    let autosplitter = match guard.as_ref() {
        Some(a) => a,
        None => return CString::new("Autosplitter not initialized").unwrap().into_raw(),
    };

    // Detect game type from process names
    let game_type = process_names.iter()
        .find_map(|name| GameType::from_process_name(name));

    match game_type {
        Some(game) => match autosplitter.start(game, boss_flags) {
            Ok(()) => std::ptr::null_mut(),
            Err(e) => CString::new(e).unwrap().into_raw(),
        },
        None => CString::new("No supported game detected from process names").unwrap().into_raw(),
    }
}
