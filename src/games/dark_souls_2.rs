//! Dark Souls 2 Scholar of the First Sin autosplitter - port of SoulSplitter's scholar.cs
//! https://github.com/FrankvdStam/SoulSplitter
//!
//! DS2 uses KILL COUNTERS for bosses, not event flags.
//! Each boss has an offset from the BossCounters base address.
//! Killing a boss increments the counter at that offset.

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HANDLE;

#[cfg(target_os = "windows")]
use crate::memory::{parse_pattern, resolve_rip_relative, scan_pattern, read_i32, read_i16, read_f32};
#[cfg(target_os = "windows")]
use crate::memory::pointer::Pointer;

// Memory patterns from SoulSplitter scholar.cs
#[cfg(target_os = "windows")]
pub const GAME_MANAGER_IMP_PATTERN: &str = "48 8b 35 ? ? ? ? 48 8b e9 48 85 f6";
#[cfg(target_os = "windows")]
pub const LOAD_STATE_PATTERN: &str = "48 89 05 ? ? ? ? b0 01 48 83 c4 28";

/// Player position as 3D vector
#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Copy, Default)]
pub struct Vector3f {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// Character attributes for DS2
#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum Attribute {
    SoulLevel = 0xD0,
    Vigor = 0x0,
    Endurance = 0x2,
    Vitality = 0x4,
    Attunement = 0x6,
    Strength = 0x8,
    Dexterity = 0xA,
    Adaptability = 0xC,
    Intelligence = 0xE,
    Faith = 0x10,
}

/// Boss types for DS2 - offsets into boss counter array
#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i64)]
pub enum BossType {
    LastGiant = 0x0,
    Pursuer = 0x4,
    Dragonrider = 0x20,
    OldDragonslayer = 0x24,
    FlexileSentry = 0x28,
    RuinSentinels = 0x2c,
    LostSinner = 0x18,
    BelfryGargoyles = 0x30,
    CovetousDemon = 0x58,
    Mytha = 0x54,
    SmelterDemon = 0x50,
    OldIronKing = 0x1c,
    ScorpionessNajka = 0x3c,
    RoyalRatAuthority = 0x40,
    ProwlingMagus = 0x38,
    DukesDearFreja = 0x14,
    RoyalRatVanguard = 0x34,
    SkeletonLords = 0x44,
    ExecutionersChariot = 0x48,
    Vendrick = 0x74,
    Darklurker = 0x70,
    DragonslayerArmour = 0x60,
    GiantLord = 0x68,
    Guardian = 0x6c,
    LookingGlassKnight = 0x5c,
    DemonOfSong = 0x64,
    VelstadtTheRoyalAegis = 0x78,
    TwinDragonRiders = 0x8,
    NashendraThrone = 0xc,
    AldiaThroneDefender = 0x10,
    // DLC Bosses
    ElanaTheSqualidQueen = 0x80,
    SinhTheSleepingDragon = 0x84,
    AfflictedGraverobber = 0x88,
    FumeKnight = 0x7c,
    SirAlonne = 0x8c,
    BlueSmelterDemon = 0x90,
    AavaTheKingsPet = 0x94,
    BurntIvoryKing = 0x9c,
    LudAndZallen = 0xa0,
}

/// Dark Souls 2 SOTFS autosplitter state
#[cfg(target_os = "windows")]
pub struct DarkSouls2 {
    pub handle: HANDLE,
    // Core pointers
    pub game_manager_imp: Pointer,
    pub load_state: Pointer,
    // Derived pointers
    pub boss_counters: Pointer,
    pub event_flag_manager: Pointer,
    pub position: Pointer,
    pub attributes: Pointer,
}

#[cfg(target_os = "windows")]
impl DarkSouls2 {
    pub fn new() -> Self {
        Self {
            handle: HANDLE::default(),
            game_manager_imp: Pointer::new(),
            load_state: Pointer::new(),
            boss_counters: Pointer::new(),
            event_flag_manager: Pointer::new(),
            position: Pointer::new(),
            attributes: Pointer::new(),
        }
    }

    /// Initialize pointers by scanning for patterns
    pub fn init_pointers(&mut self, handle: HANDLE, base: usize, size: usize) -> bool {
        self.handle = handle;

        // Scan for GameManagerImp
        let pattern = parse_pattern(GAME_MANAGER_IMP_PATTERN);
        let game_manager_addr = match scan_pattern(handle, base, size, &pattern) {
            Some(found) => {
                match resolve_rip_relative(handle, found, 3, 7) {
                    Some(addr) => addr,
                    None => {
                        log::warn!("DS2: Failed to resolve GameManagerImp RIP-relative address");
                        return false;
                    }
                }
            }
            None => {
                log::warn!("DS2: GameManagerImp pattern not found");
                return false;
            }
        };

        self.game_manager_imp.initialize(handle, true, game_manager_addr as i64, &[0x0]);
        log::info!("DS2: GameManagerImp at 0x{:X}", game_manager_addr);

        // Initialize pointer chains from GameManagerImp
        // BossCounters: GameManagerImp -> 0x0 -> 0x70 -> 0x28 -> 0x20 -> 0x8
        self.boss_counters.initialize(handle, true, game_manager_addr as i64, &[0x0, 0x70, 0x28, 0x20, 0x8]);

        // EventFlagManager: GameManagerImp -> 0x0 -> 0x70 -> 0x20
        self.event_flag_manager.initialize(handle, true, game_manager_addr as i64, &[0x0, 0x70, 0x20]);

        // Position: GameManagerImp -> 0x0 -> 0xd0 -> 0x100
        self.position.initialize(handle, true, game_manager_addr as i64, &[0x0, 0xd0, 0x100]);

        // Attributes: GameManagerImp -> 0x0 -> 0xd0 -> 0x490
        self.attributes.initialize(handle, true, game_manager_addr as i64, &[0x0, 0xd0, 0x490]);

        // Scan for LoadState
        let pattern = parse_pattern(LOAD_STATE_PATTERN);
        if let Some(found) = scan_pattern(handle, base, size, &pattern) {
            if let Some(addr) = resolve_rip_relative(handle, found, 3, 7) {
                self.load_state.initialize(handle, true, addr as i64, &[]);
                log::info!("DS2: LoadState at 0x{:X}", addr);
            }
        }

        log::info!("DS2: BossCounters base at 0x{:X}", self.boss_counters.get_address());

        true
    }

    /// Get boss kill count - port of SoulSplitter's GetBossKillCount
    pub fn get_boss_kill_count(&self, boss_type: BossType) -> i32 {
        self.boss_counters.read_i32(Some(boss_type as i64))
    }

    /// Get boss kill count by raw offset
    pub fn get_boss_kill_count_raw(&self, boss_offset: u32) -> i32 {
        self.boss_counters.read_i32(Some(boss_offset as i64))
    }

    /// Read event flag - checks if a boss has been killed (kill count > 0)
    /// For DS2, the flag_id is actually an offset into boss counters, not an event flag
    pub fn read_event_flag(&self, flag_id: u32) -> bool {
        let kill_count = self.get_boss_kill_count_raw(flag_id);
        log::trace!("DS2: read_event_flag(offset={}) = kill_count {}", flag_id, kill_count);
        kill_count > 0
    }

    /// Check if loading screen is active
    pub fn is_loading(&self) -> bool {
        let addr = self.load_state.get_address();
        if addr == 0 {
            return false;
        }
        // LoadState + 0x11c == 1 means loading
        read_i32(self.handle, (addr + 0x11c) as usize).unwrap_or(0) == 1
    }

    /// Get player position
    pub fn get_position(&self) -> Vector3f {
        let addr = self.position.get_address();
        if addr == 0 {
            return Vector3f::default();
        }
        // DS2 position offsets: X at 0x88, Y at 0x80, Z at 0x84
        Vector3f {
            x: read_f32(self.handle, (addr + 0x88) as usize).unwrap_or(0.0),
            y: read_f32(self.handle, (addr + 0x80) as usize).unwrap_or(0.0),
            z: read_f32(self.handle, (addr + 0x84) as usize).unwrap_or(0.0),
        }
    }

    /// Get character attribute value
    pub fn get_attribute(&self, attribute: Attribute) -> i32 {
        let addr = self.attributes.get_address();
        if addr == 0 {
            return -1;
        }

        // SoulLevel is i32, others are i16
        if attribute == Attribute::SoulLevel {
            read_i32(self.handle, (addr + attribute as i64) as usize).unwrap_or(-1)
        } else {
            read_i16(self.handle, (addr + attribute as i64) as usize).unwrap_or(-1) as i32
        }
    }

    /// Get in-game time in milliseconds
    /// Note: DS2 Scholar edition doesn't have a reliable IGT pointer
    pub fn get_in_game_time_milliseconds(&self) -> i32 {
        // Not implemented for DS2 Scholar in SoulSplitter
        0
    }
}

#[cfg(target_os = "windows")]
impl Default for DarkSouls2 {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Linux Implementation (for Proton/Wine)
// =============================================================================

#[cfg(target_os = "linux")]
use crate::memory::{parse_pattern, resolve_rip_relative, scan_pattern, read_i32, read_i16, read_f32};
#[cfg(target_os = "linux")]
use crate::memory::pointer::Pointer;

// Memory patterns (same as Windows)
#[cfg(target_os = "linux")]
pub const GAME_MANAGER_IMP_PATTERN: &str = "48 8b 35 ? ? ? ? 48 8b e9 48 85 f6";
#[cfg(target_os = "linux")]
pub const LOAD_STATE_PATTERN: &str = "48 89 05 ? ? ? ? b0 01 48 83 c4 28";

#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy, Default)]
pub struct Vector3f {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum Attribute {
    SoulLevel = 0xD0,
    Vigor = 0x0,
    Endurance = 0x2,
    Vitality = 0x4,
    Attunement = 0x6,
    Strength = 0x8,
    Dexterity = 0xA,
    Adaptability = 0xC,
    Intelligence = 0xE,
    Faith = 0x10,
}

#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i64)]
pub enum BossType {
    LastGiant = 0x0,
    Pursuer = 0x4,
    Dragonrider = 0x20,
    OldDragonslayer = 0x24,
    FlexileSentry = 0x28,
    RuinSentinels = 0x2c,
    LostSinner = 0x18,
    BelfryGargoyles = 0x30,
    CovetousDemon = 0x58,
    Mytha = 0x54,
    SmelterDemon = 0x50,
    OldIronKing = 0x1c,
    ScorpionessNajka = 0x3c,
    RoyalRatAuthority = 0x40,
    ProwlingMagus = 0x38,
    DukesDearFreja = 0x14,
    RoyalRatVanguard = 0x34,
    SkeletonLords = 0x44,
    ExecutionersChariot = 0x48,
    Vendrick = 0x74,
    Darklurker = 0x70,
    DragonslayerArmour = 0x60,
    GiantLord = 0x68,
    Guardian = 0x6c,
    LookingGlassKnight = 0x5c,
    DemonOfSong = 0x64,
    VelstadtTheRoyalAegis = 0x78,
    TwinDragonRiders = 0x8,
    NashendraThrone = 0xc,
    AldiaThroneDefender = 0x10,
    ElanaTheSqualidQueen = 0x80,
    SinhTheSleepingDragon = 0x84,
    AfflictedGraverobber = 0x88,
    FumeKnight = 0x7c,
    SirAlonne = 0x8c,
    BlueSmelterDemon = 0x90,
    AavaTheKingsPet = 0x94,
    BurntIvoryKing = 0x9c,
    LudAndZallen = 0xa0,
}

#[cfg(target_os = "linux")]
pub struct DarkSouls2 {
    pub pid: i32,
    // Core pointers
    pub game_manager_imp: Pointer,
    pub load_state: Pointer,
    // Derived pointers
    pub boss_counters: Pointer,
    pub event_flag_manager: Pointer,
    pub position: Pointer,
    pub attributes: Pointer,
}

#[cfg(target_os = "linux")]
impl DarkSouls2 {
    pub fn new() -> Self {
        Self {
            pid: 0,
            game_manager_imp: Pointer::new(),
            load_state: Pointer::new(),
            boss_counters: Pointer::new(),
            event_flag_manager: Pointer::new(),
            position: Pointer::new(),
            attributes: Pointer::new(),
        }
    }

    pub fn init_pointers(&mut self, pid: i32, base: usize, size: usize) -> bool {
        self.pid = pid;
        log::info!("DS2: Initializing pointers (Linux), base=0x{:X}, size=0x{:X}", base, size);

        // Scan for GameManagerImp
        let pattern = parse_pattern(GAME_MANAGER_IMP_PATTERN);
        let game_manager_addr = match scan_pattern(pid, base, size, &pattern) {
            Some(found) => {
                match resolve_rip_relative(pid, found, 3, 7) {
                    Some(addr) => addr,
                    None => {
                        log::warn!("DS2: Failed to resolve GameManagerImp RIP-relative address");
                        return false;
                    }
                }
            }
            None => {
                log::warn!("DS2: GameManagerImp pattern not found");
                return false;
            }
        };

        self.game_manager_imp.initialize(pid, true, game_manager_addr as i64, &[0x0]);
        log::info!("DS2: GameManagerImp at 0x{:X}", game_manager_addr);

        // Initialize pointer chains from GameManagerImp
        self.boss_counters.initialize(pid, true, game_manager_addr as i64, &[0x0, 0x70, 0x28, 0x20, 0x8]);
        self.event_flag_manager.initialize(pid, true, game_manager_addr as i64, &[0x0, 0x70, 0x20]);
        self.position.initialize(pid, true, game_manager_addr as i64, &[0x0, 0xd0, 0x100]);
        self.attributes.initialize(pid, true, game_manager_addr as i64, &[0x0, 0xd0, 0x490]);

        // Scan for LoadState
        let pattern = parse_pattern(LOAD_STATE_PATTERN);
        if let Some(found) = scan_pattern(pid, base, size, &pattern) {
            if let Some(addr) = resolve_rip_relative(pid, found, 3, 7) {
                self.load_state.initialize(pid, true, addr as i64, &[]);
                log::info!("DS2: LoadState at 0x{:X}", addr);
            }
        }

        log::info!("DS2: BossCounters base at 0x{:X}", self.boss_counters.get_address());
        true
    }

    pub fn get_boss_kill_count(&self, boss_type: BossType) -> i32 {
        self.boss_counters.read_i32(Some(boss_type as i64))
    }

    pub fn get_boss_kill_count_raw(&self, boss_offset: u32) -> i32 {
        self.boss_counters.read_i32(Some(boss_offset as i64))
    }

    pub fn read_event_flag(&self, flag_id: u32) -> bool {
        let kill_count = self.get_boss_kill_count_raw(flag_id);
        kill_count > 0
    }

    pub fn is_loading(&self) -> bool {
        let addr = self.load_state.get_address();
        if addr == 0 {
            return false;
        }
        read_i32(self.pid, (addr + 0x11c) as usize).unwrap_or(0) == 1
    }

    pub fn get_position(&self) -> Vector3f {
        let addr = self.position.get_address();
        if addr == 0 {
            return Vector3f::default();
        }
        Vector3f {
            x: read_f32(self.pid, (addr + 0x88) as usize).unwrap_or(0.0),
            y: read_f32(self.pid, (addr + 0x80) as usize).unwrap_or(0.0),
            z: read_f32(self.pid, (addr + 0x84) as usize).unwrap_or(0.0),
        }
    }

    pub fn get_attribute(&self, attribute: Attribute) -> i32 {
        let addr = self.attributes.get_address();
        if addr == 0 {
            return -1;
        }

        if attribute == Attribute::SoulLevel {
            read_i32(self.pid, (addr + attribute as i64) as usize).unwrap_or(-1)
        } else {
            read_i16(self.pid, (addr + attribute as i64) as usize).unwrap_or(-1) as i32
        }
    }

    pub fn get_in_game_time_milliseconds(&self) -> i32 {
        0 // Not implemented for DS2 Scholar
    }
}

#[cfg(target_os = "linux")]
impl Default for DarkSouls2 {
    fn default() -> Self {
        Self::new()
    }
}
