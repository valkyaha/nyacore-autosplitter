//! NYA Core Autosplitter
//!
//! A standalone autosplitter library for FromSoftware games.
//! Supports Dark Souls 1/2/3, Elden Ring, Sekiro, and Armored Core 6.
//!
//! This crate can be used as:
//! - A Rust library (rlib) for direct integration
//! - A dynamic library (cdylib) for FFI-based loading
//!
//! ## ASL Support
//!
//! This library also supports parsing LiveSplit ASL (Auto Splitter Language) files
//! and converting them to the internal GameData format. This allows using existing
//! community ASL scripts without modification.
//!
//! ```rust,ignore
//! use nyacore_autosplitter::asl::parse_asl;
//!
//! let asl_content = r#"
//! state("DarkSoulsIII.exe") {
//!     bool boss : "sprj_event_flag_man", 13000050;
//! }
//! split {
//!     if (current.boss && !old.boss) { return true; }
//!     return false;
//! }
//! "#;
//!
//! let game_data = parse_asl(asl_content, Some("ds3")).unwrap();
//! ```

pub mod asl;
pub mod config;
pub mod engine;
pub mod game_data;
pub mod games;
pub mod memory;

// Re-export commonly used types
pub use config::{AutosplitterState, BossFlag};
pub use engine::GenericGame;
pub use game_data::GameData;
pub use games::{ArmoredCore6, DarkSouls1, DarkSouls2, DarkSouls3, EldenRing, Sekiro};
pub use memory::{parse_pattern, resolve_rip_relative, scan_pattern};

// Re-export ASL types
pub use asl::{parse_asl, AslError, AslResult};

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
    /// Generic game using data-driven configuration
    Generic(GenericGame),
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
            GameState::Generic(g) => g.read_event_flag(flag_id),
        }
    }

    fn get_boss_kill_count(&self, flag_id: u32) -> u32 {
        match self {
            GameState::DarkSouls2(g) => g.get_boss_kill_count_raw(flag_id).max(0) as u32,
            GameState::Generic(g) => g.get_kill_count(flag_id),
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
            GameState::Generic(g) => g.handle,
        }
    }

    fn name(&self) -> &str {
        match self {
            GameState::DarkSouls1(_) => "Dark Souls Remastered",
            GameState::DarkSouls2(_) => "Dark Souls 2 SOTFS",
            GameState::DarkSouls3(_) => "Dark Souls 3",
            GameState::EldenRing(_) => "Elden Ring",
            GameState::Sekiro(_) => "Sekiro",
            GameState::ArmoredCore6(_) => "Armored Core 6",
            GameState::Generic(g) => &g.game_data.game.name,
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

    /// Start autosplitter with data-driven game configuration
    #[cfg(target_os = "windows")]
    pub fn start_with_game_data(
        &self,
        game_data: GameData,
        boss_flags: Vec<BossFlag>,
    ) -> Result<(), String> {
        if self.running.load(Ordering::SeqCst) {
            return Err("Autosplitter already running".to_string());
        }

        if boss_flags.is_empty() {
            return Err("No boss flags defined".to_string());
        }

        log::info!(
            "Starting autosplitter for {} (engine: {}) with {} boss flags",
            game_data.game.name,
            game_data.autosplitter.engine,
            boss_flags.len()
        );

        self.running.store(true, Ordering::SeqCst);

        {
            let mut state = self.state.lock().unwrap();
            state.running = true;
            state.process_attached = false;
            state.game_id = game_data.game.id.clone();
            state.process_id = None;
            state.bosses_defeated.clear();
            state.boss_kill_counts.clear();
        }

        let running = self.running.clone();
        let state = self.state.clone();
        let reset_requested = self.reset_requested.clone();
        let process_names = game_data.game.process_names.clone();

        thread::spawn(move || {
            log::info!("Autosplitter thread started (generic engine)");
            run_generic_autosplitter_loop(
                running,
                state,
                reset_requested,
                game_data,
                process_names,
                boss_flags,
            );
        });

        Ok(())
    }

    #[cfg(target_os = "linux")]
    pub fn start_with_game_data(
        &self,
        _game_data: GameData,
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
// Generic Game Loop (Windows) - Uses data-driven configuration
// =============================================================================

#[cfg(target_os = "windows")]
fn run_generic_autosplitter_loop(
    running: Arc<AtomicBool>,
    state: Arc<Mutex<AutosplitterState>>,
    reset_requested: Arc<AtomicBool>,
    game_data: GameData,
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

                // Initialize generic game
                match GenericGame::new(game_data.clone()) {
                    Ok(mut game) => {
                        if game.init(handle, base, size) {
                            log::info!("Connected to {} (generic engine)", game.game_data.game.name);

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

                            game_state = Some(GameState::Generic(game));
                            current_handle = Some(handle);

                            let mut s = state.lock().unwrap();
                            s.process_attached = true;
                            s.process_id = Some(unsafe { GetProcessId(handle) });
                        } else {
                            log::error!("Failed to initialize generic game - patterns not found");
                            unsafe {
                                let _ = CloseHandle(handle);
                            }
                            thread::sleep(Duration::from_millis(2000));
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to create generic game: {}", e);
                        unsafe {
                            let _ = CloseHandle(handle);
                        }
                        thread::sleep(Duration::from_millis(2000));
                    }
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

/// Start autosplitter with data-driven game configuration
/// game_data_toml: TOML string containing game definition
/// boss_flags_json: JSON array of BossFlag objects
/// Returns error message or null on success (caller must free error string)
#[no_mangle]
pub extern "C" fn autosplitter_start_with_game_data(
    game_data_toml: *const c_char,
    boss_flags_json: *const c_char,
) -> *mut c_char {
    if game_data_toml.is_null() || boss_flags_json.is_null() {
        return CString::new("Null pointer passed").unwrap().into_raw();
    }

    let game_data_str = unsafe { std::ffi::CStr::from_ptr(game_data_toml).to_string_lossy() };
    let boss_flags_str = unsafe { std::ffi::CStr::from_ptr(boss_flags_json).to_string_lossy() };

    let game_data: GameData = match GameData::from_toml(&game_data_str) {
        Ok(data) => data,
        Err(e) => return CString::new(format!("Failed to parse game data TOML: {}", e)).unwrap().into_raw(),
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

    match autosplitter.start_with_game_data(game_data, boss_flags) {
        Ok(()) => std::ptr::null_mut(),
        Err(e) => CString::new(e).unwrap().into_raw(),
    }
}

/// Start autosplitter with ASL (LiveSplit Auto Splitter Language) script
/// asl_content: ASL script content as a string
/// boss_flags_json: JSON array of BossFlag objects
/// engine_hint: Optional engine hint (e.g., "ds3", "elden_ring"), can be null
/// Returns error message or null on success (caller must free error string)
#[no_mangle]
pub extern "C" fn autosplitter_start_with_asl(
    asl_content: *const c_char,
    boss_flags_json: *const c_char,
    engine_hint: *const c_char,
) -> *mut c_char {
    if asl_content.is_null() || boss_flags_json.is_null() {
        return CString::new("Null pointer passed").unwrap().into_raw();
    }

    let asl_str = unsafe { std::ffi::CStr::from_ptr(asl_content).to_string_lossy() };
    let boss_flags_str = unsafe { std::ffi::CStr::from_ptr(boss_flags_json).to_string_lossy() };
    let hint = if engine_hint.is_null() {
        None
    } else {
        Some(unsafe { std::ffi::CStr::from_ptr(engine_hint).to_string_lossy() })
    };

    // Parse ASL and convert to GameData
    let game_data = match asl::parse_asl(&asl_str, hint.as_deref()) {
        Ok(data) => data,
        Err(e) => return CString::new(format!("Failed to parse ASL: {}", e)).unwrap().into_raw(),
    };

    let boss_flags: Vec<BossFlag> = match serde_json::from_str(&boss_flags_str) {
        Ok(flags) => flags,
        Err(e) => {
            return CString::new(format!("Failed to parse boss flags: {}", e))
                .unwrap()
                .into_raw()
        }
    };

    let guard = AUTOSPLITTER.lock().unwrap();
    let autosplitter = match guard.as_ref() {
        Some(a) => a,
        None => return CString::new("Autosplitter not initialized").unwrap().into_raw(),
    };

    match autosplitter.start_with_game_data(game_data, boss_flags) {
        Ok(()) => std::ptr::null_mut(),
        Err(e) => CString::new(e).unwrap().into_raw(),
    }
}

/// Parse ASL content and return GameData as TOML string
/// asl_content: ASL script content as a string
/// engine_hint: Optional engine hint (e.g., "ds3", "elden_ring"), can be null
/// Returns TOML string on success, or error message prefixed with "ERROR: " on failure
/// Caller must free the returned string with autosplitter_free_string
#[no_mangle]
pub extern "C" fn autosplitter_parse_asl(
    asl_content: *const c_char,
    engine_hint: *const c_char,
) -> *mut c_char {
    if asl_content.is_null() {
        return CString::new("ERROR: Null pointer passed").unwrap().into_raw();
    }

    let asl_str = unsafe { std::ffi::CStr::from_ptr(asl_content).to_string_lossy() };
    let hint = if engine_hint.is_null() {
        None
    } else {
        Some(unsafe { std::ffi::CStr::from_ptr(engine_hint).to_string_lossy() })
    };

    // Parse ASL and convert to GameData
    let game_data = match asl::parse_asl(&asl_str, hint.as_deref()) {
        Ok(data) => data,
        Err(e) => {
            return CString::new(format!("ERROR: Failed to parse ASL: {}", e))
                .unwrap()
                .into_raw()
        }
    };

    // Serialize to TOML
    match toml::to_string_pretty(&game_data) {
        Ok(toml_str) => CString::new(toml_str).unwrap().into_raw(),
        Err(e) => {
            CString::new(format!("ERROR: Failed to serialize to TOML: {}", e))
                .unwrap()
                .into_raw()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =============================================================================
    // GameType tests
    // =============================================================================

    #[test]
    fn test_game_type_from_process_name_ds1() {
        assert_eq!(
            GameType::from_process_name("DarkSoulsRemastered.exe"),
            Some(GameType::DarkSouls1)
        );
        assert_eq!(
            GameType::from_process_name("darksoulsremastered.exe"),
            Some(GameType::DarkSouls1)
        );
        assert_eq!(
            GameType::from_process_name("DARKSOULSREMASTERED.EXE"),
            Some(GameType::DarkSouls1)
        );
    }

    #[test]
    fn test_game_type_from_process_name_ds2() {
        assert_eq!(
            GameType::from_process_name("DarkSoulsII.exe"),
            Some(GameType::DarkSouls2)
        );
        assert_eq!(
            GameType::from_process_name("darksoulsii.exe"),
            Some(GameType::DarkSouls2)
        );
    }

    #[test]
    fn test_game_type_from_process_name_ds3() {
        assert_eq!(
            GameType::from_process_name("DarkSoulsIII.exe"),
            Some(GameType::DarkSouls3)
        );
        assert_eq!(
            GameType::from_process_name("darksoulsiii.exe"),
            Some(GameType::DarkSouls3)
        );
    }

    #[test]
    fn test_game_type_from_process_name_elden_ring() {
        assert_eq!(
            GameType::from_process_name("eldenring.exe"),
            Some(GameType::EldenRing)
        );
        assert_eq!(
            GameType::from_process_name("EldenRing.exe"),
            Some(GameType::EldenRing)
        );
    }

    #[test]
    fn test_game_type_from_process_name_sekiro() {
        assert_eq!(
            GameType::from_process_name("sekiro.exe"),
            Some(GameType::Sekiro)
        );
        assert_eq!(
            GameType::from_process_name("Sekiro.exe"),
            Some(GameType::Sekiro)
        );
    }

    #[test]
    fn test_game_type_from_process_name_ac6() {
        assert_eq!(
            GameType::from_process_name("armoredcore6.exe"),
            Some(GameType::ArmoredCore6)
        );
        assert_eq!(
            GameType::from_process_name("ArmoredCore6.exe"),
            Some(GameType::ArmoredCore6)
        );
    }

    #[test]
    fn test_game_type_from_process_name_unknown() {
        assert_eq!(GameType::from_process_name("notepad.exe"), None);
        assert_eq!(GameType::from_process_name(""), None);
        assert_eq!(GameType::from_process_name("darksouls.exe"), None); // Not specific enough
    }

    #[test]
    fn test_game_type_from_process_name_ds2_vs_ds3_ordering() {
        // DS3 contains "darksoulsiii", DS2 contains "darksoulsii"
        // The order matters - must check DS3 first because DS2 pattern matches DS3
        assert_eq!(
            GameType::from_process_name("darksoulsiii.exe"),
            Some(GameType::DarkSouls3)
        );
    }

    #[test]
    fn test_game_type_process_names() {
        assert_eq!(
            GameType::DarkSouls1.process_names(),
            &["DarkSoulsRemastered.exe"]
        );
        assert_eq!(
            GameType::DarkSouls2.process_names(),
            &["DarkSoulsII.exe"]
        );
        assert_eq!(
            GameType::DarkSouls3.process_names(),
            &["DarkSoulsIII.exe"]
        );
        assert_eq!(
            GameType::EldenRing.process_names(),
            &["eldenring.exe"]
        );
        assert_eq!(
            GameType::Sekiro.process_names(),
            &["sekiro.exe"]
        );
        assert_eq!(
            GameType::ArmoredCore6.process_names(),
            &["armoredcore6.exe"]
        );
    }

    #[test]
    fn test_game_type_display_name() {
        assert_eq!(
            GameType::DarkSouls1.display_name(),
            "Dark Souls Remastered"
        );
        assert_eq!(
            GameType::DarkSouls2.display_name(),
            "Dark Souls II: Scholar of the First Sin"
        );
        assert_eq!(
            GameType::DarkSouls3.display_name(),
            "Dark Souls III"
        );
        assert_eq!(
            GameType::EldenRing.display_name(),
            "Elden Ring"
        );
        assert_eq!(
            GameType::Sekiro.display_name(),
            "Sekiro: Shadows Die Twice"
        );
        assert_eq!(
            GameType::ArmoredCore6.display_name(),
            "Armored Core VI: Fires of Rubicon"
        );
    }

    #[test]
    fn test_game_type_clone() {
        let game = GameType::DarkSouls3;
        let cloned = game.clone();
        assert_eq!(game, cloned);
    }

    #[test]
    fn test_game_type_debug() {
        let game = GameType::EldenRing;
        let debug_str = format!("{:?}", game);
        assert_eq!(debug_str, "EldenRing");
    }

    #[test]
    fn test_game_type_copy() {
        let game = GameType::Sekiro;
        let copied = game; // Copy, not move
        assert_eq!(game, copied);
    }

    // =============================================================================
    // Autosplitter tests
    // =============================================================================

    #[test]
    fn test_autosplitter_new() {
        let autosplitter = Autosplitter::new();
        assert!(!autosplitter.is_running());
    }

    #[test]
    fn test_autosplitter_default() {
        let autosplitter = Autosplitter::default();
        assert!(!autosplitter.is_running());
    }

    #[test]
    fn test_autosplitter_get_state_default() {
        let autosplitter = Autosplitter::new();
        let state = autosplitter.get_state();

        assert!(!state.running);
        assert!(state.game_id.is_empty());
        assert!(!state.process_attached);
        assert!(state.process_id.is_none());
        assert!(state.bosses_defeated.is_empty());
        assert!(state.boss_kill_counts.is_empty());
    }

    #[test]
    fn test_autosplitter_get_defeated_bosses() {
        let autosplitter = Autosplitter::new();
        let bosses = autosplitter.get_defeated_bosses();
        assert!(bosses.is_empty());
    }

    #[test]
    fn test_autosplitter_stop() {
        let autosplitter = Autosplitter::new();
        autosplitter.stop();
        assert!(!autosplitter.is_running());
    }

    #[test]
    fn test_autosplitter_reset() {
        let autosplitter = Autosplitter::new();
        autosplitter.reset();
        // Reset should clear the bosses_defeated list
        let state = autosplitter.get_state();
        assert!(state.bosses_defeated.is_empty());
        assert!(state.boss_kill_counts.is_empty());
    }

    // =============================================================================
    // BossFlag and AutosplitterState re-export tests
    // =============================================================================

    #[test]
    fn test_boss_flag_reexport() {
        let flag = BossFlag {
            boss_id: "test_boss".to_string(),
            boss_name: "Test Boss".to_string(),
            flag_id: 12345,
            is_dlc: false,
        };

        assert_eq!(flag.boss_id, "test_boss");
        assert_eq!(flag.flag_id, 12345);
    }

    #[test]
    fn test_autosplitter_state_reexport() {
        let state = AutosplitterState::default();
        assert!(!state.running);
    }

    // =============================================================================
    // Module re-export tests
    // =============================================================================

    #[test]
    fn test_parse_pattern_reexport() {
        // Test that parse_pattern is properly re-exported
        let pattern = parse_pattern("48 8b ?");
        assert_eq!(pattern.len(), 3);
    }
}
