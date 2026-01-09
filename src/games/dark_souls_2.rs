//! Dark Souls 2 (Scholar of the First Sin) game implementation
//!
//! Based on SoulSplitter by FrankvdStam:
//! https://github.com/FrankvdStam/SoulSplitter
//!
//! DS2 uses KILL COUNTERS for bosses, not event flags.
//! Each boss has an offset from the BossCounters base address.
//! Killing a boss increments the counter at that offset.

use std::collections::HashMap;
use std::sync::Arc;

use super::{
    Game, GameFactory, BoxedGame, Position3D, TriggerTypeInfo, AttributeInfo,
    CustomTriggerType, CustomTriggerParam, CustomTriggerParamType, CustomTriggerChoice,
    common::{standard_event_flag_trigger, standard_position_trigger},
};
use crate::memory::{ProcessContext, MemoryReader, Pointer, parse_pattern, extract_relative_address};
use crate::AutosplitterError;

// =============================================================================
// CONSTANTS
// =============================================================================

/// Game metadata
pub const GAME_ID: &str = "dark-souls-2";
pub const GAME_NAME: &str = "Dark Souls II: Scholar of the First Sin";
pub const PROCESS_NAMES: &[&str] = &["DarkSoulsII.exe"];

/// Memory patterns from SoulSplitter
pub const GAME_MANAGER_IMP_PATTERN: &str = "48 8b 35 ?? ?? ?? ?? 48 8b e9 48 85 f6";
pub const LOAD_STATE_PATTERN: &str = "48 89 05 ?? ?? ?? ?? b0 01 48 83 c4 28";

// =============================================================================
// ATTRIBUTES
// =============================================================================

/// Character attributes for DS2
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

// =============================================================================
// BOSS TYPES
// =============================================================================

/// Boss types for DS2 - offsets into boss counter array
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

// =============================================================================
// GAME IMPLEMENTATION
// =============================================================================

/// Dark Souls 2 SOTFS game implementation
pub struct DarkSouls2 {
    // Core state
    reader: Option<Arc<dyn MemoryReader>>,
    initialized: bool,

    // Memory pointers
    game_manager_imp: Pointer,
    load_state: Pointer,

    // Derived pointers
    boss_counters: Pointer,
    position: Pointer,
    attributes: Pointer,
}

impl DarkSouls2 {
    pub fn new() -> Self {
        Self {
            reader: None,
            initialized: false,
            game_manager_imp: Pointer::new(),
            load_state: Pointer::new(),
            boss_counters: Pointer::new(),
            position: Pointer::new(),
            attributes: Pointer::new(),
        }
    }

    /// Get the memory reader if available
    fn reader(&self) -> Option<&dyn MemoryReader> {
        self.reader.as_ref().map(|r| r.as_ref())
    }

    /// Get boss kill count by BossType enum
    pub fn get_boss_kill_count_by_type(&self, boss_type: BossType) -> i32 {
        let reader = match self.reader() {
            Some(r) => r,
            None => return 0,
        };
        self.boss_counters.read_i32(reader, Some(boss_type as i64))
    }

    /// Build the list of boss choices for custom triggers
    fn build_boss_choices() -> Vec<CustomTriggerChoice> {
        vec![
            // Base Game - Forest of Fallen Giants / Heide's Tower
            CustomTriggerChoice { value: "0".to_string(), label: "Last Giant".to_string(), group: Some("Base Game".to_string()) },
            CustomTriggerChoice { value: "4".to_string(), label: "The Pursuer".to_string(), group: Some("Base Game".to_string()) },
            CustomTriggerChoice { value: "32".to_string(), label: "Dragonrider".to_string(), group: Some("Base Game".to_string()) },
            CustomTriggerChoice { value: "36".to_string(), label: "Old Dragonslayer".to_string(), group: Some("Base Game".to_string()) },
            // No-Man's Wharf / Lost Bastille
            CustomTriggerChoice { value: "40".to_string(), label: "Flexile Sentry".to_string(), group: Some("Base Game".to_string()) },
            CustomTriggerChoice { value: "44".to_string(), label: "Ruin Sentinels".to_string(), group: Some("Base Game".to_string()) },
            CustomTriggerChoice { value: "24".to_string(), label: "Lost Sinner".to_string(), group: Some("Base Game".to_string()) },
            CustomTriggerChoice { value: "48".to_string(), label: "Belfry Gargoyles".to_string(), group: Some("Base Game".to_string()) },
            // Harvest Valley / Earthen Peak / Iron Keep
            CustomTriggerChoice { value: "88".to_string(), label: "Covetous Demon".to_string(), group: Some("Base Game".to_string()) },
            CustomTriggerChoice { value: "84".to_string(), label: "Mytha, the Baneful Queen".to_string(), group: Some("Base Game".to_string()) },
            CustomTriggerChoice { value: "80".to_string(), label: "Smelter Demon".to_string(), group: Some("Base Game".to_string()) },
            CustomTriggerChoice { value: "28".to_string(), label: "Old Iron King".to_string(), group: Some("Base Game".to_string()) },
            // Shaded Woods / Doors of Pharros / Brightstone Cove
            CustomTriggerChoice { value: "60".to_string(), label: "Scorpioness Najka".to_string(), group: Some("Base Game".to_string()) },
            CustomTriggerChoice { value: "64".to_string(), label: "Royal Rat Authority".to_string(), group: Some("Base Game".to_string()) },
            CustomTriggerChoice { value: "56".to_string(), label: "Prowling Magus".to_string(), group: Some("Base Game".to_string()) },
            CustomTriggerChoice { value: "20".to_string(), label: "The Duke's Dear Freja".to_string(), group: Some("Base Game".to_string()) },
            // Grave of Saints / The Gutter / Black Gulch
            CustomTriggerChoice { value: "52".to_string(), label: "Royal Rat Vanguard".to_string(), group: Some("Base Game".to_string()) },
            // Huntsman's Copse / Undead Purgatory
            CustomTriggerChoice { value: "68".to_string(), label: "Skeleton Lords".to_string(), group: Some("Base Game".to_string()) },
            CustomTriggerChoice { value: "72".to_string(), label: "Executioner's Chariot".to_string(), group: Some("Base Game".to_string()) },
            // Drangleic Castle / King's Passage
            CustomTriggerChoice { value: "92".to_string(), label: "Looking Glass Knight".to_string(), group: Some("Base Game".to_string()) },
            CustomTriggerChoice { value: "96".to_string(), label: "Dragonslayer Armour".to_string(), group: Some("Base Game".to_string()) },
            // Shrine of Amana / Undead Crypt
            CustomTriggerChoice { value: "100".to_string(), label: "Demon of Song".to_string(), group: Some("Base Game".to_string()) },
            CustomTriggerChoice { value: "120".to_string(), label: "Velstadt, the Royal Aegis".to_string(), group: Some("Base Game".to_string()) },
            CustomTriggerChoice { value: "116".to_string(), label: "Vendrick".to_string(), group: Some("Base Game".to_string()) },
            // Memory of the Old Iron King / Aldia's Keep / Dragon Aerie
            CustomTriggerChoice { value: "104".to_string(), label: "Giant Lord".to_string(), group: Some("Base Game".to_string()) },
            CustomTriggerChoice { value: "108".to_string(), label: "Guardian Dragon".to_string(), group: Some("Base Game".to_string()) },
            // Throne of Want
            CustomTriggerChoice { value: "8".to_string(), label: "Twin Dragonriders".to_string(), group: Some("Base Game".to_string()) },
            CustomTriggerChoice { value: "12".to_string(), label: "Nashandra".to_string(), group: Some("Base Game".to_string()) },
            CustomTriggerChoice { value: "16".to_string(), label: "Aldia, Scholar of the First Sin".to_string(), group: Some("Base Game".to_string()) },
            // Dark Chasm of Old
            CustomTriggerChoice { value: "112".to_string(), label: "Darklurker".to_string(), group: Some("Base Game".to_string()) },
            // DLC - Crown of the Sunken King
            CustomTriggerChoice { value: "128".to_string(), label: "Elana, the Squalid Queen".to_string(), group: Some("DLC - Sunken King".to_string()) },
            CustomTriggerChoice { value: "132".to_string(), label: "Sinh, the Slumbering Dragon".to_string(), group: Some("DLC - Sunken King".to_string()) },
            CustomTriggerChoice { value: "136".to_string(), label: "Afflicted Graverobber Trio".to_string(), group: Some("DLC - Sunken King".to_string()) },
            // DLC - Crown of the Old Iron King
            CustomTriggerChoice { value: "124".to_string(), label: "Fume Knight".to_string(), group: Some("DLC - Old Iron King".to_string()) },
            CustomTriggerChoice { value: "140".to_string(), label: "Sir Alonne".to_string(), group: Some("DLC - Old Iron King".to_string()) },
            CustomTriggerChoice { value: "144".to_string(), label: "Blue Smelter Demon".to_string(), group: Some("DLC - Old Iron King".to_string()) },
            // DLC - Crown of the Ivory King
            CustomTriggerChoice { value: "148".to_string(), label: "Aava, the King's Pet".to_string(), group: Some("DLC - Ivory King".to_string()) },
            CustomTriggerChoice { value: "156".to_string(), label: "Burnt Ivory King".to_string(), group: Some("DLC - Ivory King".to_string()) },
            CustomTriggerChoice { value: "160".to_string(), label: "Lud and Zallen".to_string(), group: Some("DLC - Ivory King".to_string()) },
        ]
    }
}

impl Default for DarkSouls2 {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// GAME TRAIT IMPLEMENTATION
// =============================================================================

impl Game for DarkSouls2 {
    fn id(&self) -> &'static str {
        GAME_ID
    }

    fn name(&self) -> &'static str {
        GAME_NAME
    }

    fn process_names(&self) -> &[&'static str] {
        PROCESS_NAMES
    }

    fn init_pointers(&mut self, ctx: &mut ProcessContext) -> Result<(), AutosplitterError> {
        log::info!("DS2: Initializing pointers for base 0x{:X}, size 0x{:X}",
            ctx.base_address, ctx.module_size);

        self.reader = Some(ctx.reader());
        let reader = self.reader.as_ref().unwrap();

        // GameManagerImp (required)
        let pattern = parse_pattern(GAME_MANAGER_IMP_PATTERN);
        let game_manager_addr = ctx.scan_pattern(&pattern)
            .ok_or_else(|| AutosplitterError::PatternScanFailed(
                "GameManagerImp pattern not found".to_string()
            ))?;

        let game_manager_resolved = extract_relative_address(reader.as_ref(), game_manager_addr, 3, 7)
            .ok_or_else(|| AutosplitterError::PatternScanFailed(
                "Failed to resolve GameManagerImp RIP-relative address".to_string()
            ))?;

        self.game_manager_imp.initialize(ctx.is_64_bit, game_manager_resolved as i64, &[0x0]);
        log::info!("DS2: GameManagerImp at 0x{:X}", game_manager_resolved);

        // Initialize pointer chains from GameManagerImp
        // BossCounters: GameManagerImp -> 0x0 -> 0x70 -> 0x28 -> 0x20 -> 0x8
        self.boss_counters.initialize(ctx.is_64_bit, game_manager_resolved as i64, &[0x0, 0x70, 0x28, 0x20, 0x8]);

        // Position: GameManagerImp -> 0x0 -> 0xd0 -> 0x100
        self.position.initialize(ctx.is_64_bit, game_manager_resolved as i64, &[0x0, 0xd0, 0x100]);

        // Attributes: GameManagerImp -> 0x0 -> 0xd0 -> 0x490
        self.attributes.initialize(ctx.is_64_bit, game_manager_resolved as i64, &[0x0, 0xd0, 0x490]);

        // LoadState (optional)
        let pattern = parse_pattern(LOAD_STATE_PATTERN);
        if let Some(found) = ctx.scan_pattern(&pattern) {
            if let Some(addr) = extract_relative_address(reader.as_ref(), found, 3, 7) {
                self.load_state.initialize(ctx.is_64_bit, addr as i64, &[]);
                log::info!("DS2: LoadState at 0x{:X}", addr);
            }
        }

        self.initialized = true;
        log::info!("DS2: All pointers initialized successfully");
        Ok(())
    }

    fn read_event_flag(&self, flag_id: u32) -> bool {
        // DS2 uses boss kill counters instead of event flags
        // For compatibility, treat flag_id as a boss offset
        self.get_boss_kill_count(flag_id) > 0
    }

    fn get_boss_kill_count(&self, flag_id: u32) -> u32 {
        if !self.initialized {
            return 0;
        }
        let reader = match self.reader() {
            Some(r) => r,
            None => return 0,
        };
        self.boss_counters.read_i32(reader, Some(flag_id as i64)) as u32
    }

    fn is_alive(&self) -> bool {
        self.initialized
    }

    fn get_igt_milliseconds(&self) -> Option<i32> {
        // DS2 Scholar edition doesn't have a reliable IGT pointer
        None
    }

    fn get_position(&self) -> Option<Position3D> {
        if !self.initialized {
            return None;
        }
        let reader = self.reader()?;
        let addr = self.position.get_address(reader);
        if addr == 0 {
            return None;
        }

        // DS2 position offsets: X at 0x88, Y at 0x80, Z at 0x84
        Some(Position3D {
            x: reader.read_f32((addr + 0x88) as usize).unwrap_or(0.0),
            y: reader.read_f32((addr + 0x80) as usize).unwrap_or(0.0),
            z: reader.read_f32((addr + 0x84) as usize).unwrap_or(0.0),
        })
    }

    fn is_loading(&self) -> Option<bool> {
        if !self.initialized {
            return None;
        }
        let reader = self.reader()?;
        let addr = self.load_state.get_address(reader);
        if addr == 0 {
            return None;
        }
        // LoadState + 0x11c == 1 means loading
        Some(reader.read_i32((addr + 0x11c) as usize).unwrap_or(0) == 1)
    }

    fn get_attribute(&self, attr: &str) -> Option<i32> {
        if !self.initialized {
            return None;
        }
        let reader = self.reader()?;
        let addr = self.attributes.get_address(reader);
        if addr == 0 {
            return None;
        }

        let attribute = match attr.to_lowercase().as_str() {
            "soul_level" | "sl" | "level" => Attribute::SoulLevel,
            "vigor" | "vig" => Attribute::Vigor,
            "endurance" | "end" => Attribute::Endurance,
            "vitality" | "vit" => Attribute::Vitality,
            "attunement" | "att" => Attribute::Attunement,
            "strength" | "str" => Attribute::Strength,
            "dexterity" | "dex" => Attribute::Dexterity,
            "adaptability" | "adp" => Attribute::Adaptability,
            "intelligence" | "int" => Attribute::Intelligence,
            "faith" | "fai" => Attribute::Faith,
            _ => return None,
        };

        // SoulLevel is i32, others are i16
        if attribute == Attribute::SoulLevel {
            reader.read_i32((addr + attribute as i64) as usize)
        } else {
            reader.read_i16((addr + attribute as i64) as usize).map(|v| v as i32)
        }
    }

    fn get_boss_kill_count_raw(&self, boss_offset: u32) -> Option<i32> {
        if !self.initialized {
            return None;
        }
        let reader = self.reader()?;
        Some(self.boss_counters.read_i32(reader, Some(boss_offset as i64)))
    }

    fn supported_triggers(&self) -> Vec<TriggerTypeInfo> {
        vec![
            standard_event_flag_trigger(),
            TriggerTypeInfo {
                id: "kill_count".to_string(),
                name: "Kill Count".to_string(),
                description: "Triggers based on boss kill count (supports ascetics)".to_string(),
            },
            standard_position_trigger(),
        ]
    }

    fn available_attributes(&self) -> Vec<AttributeInfo> {
        vec![
            AttributeInfo { id: "vigor".to_string(), name: "Vigor".to_string() },
            AttributeInfo { id: "endurance".to_string(), name: "Endurance".to_string() },
            AttributeInfo { id: "vitality".to_string(), name: "Vitality".to_string() },
            AttributeInfo { id: "attunement".to_string(), name: "Attunement".to_string() },
            AttributeInfo { id: "strength".to_string(), name: "Strength".to_string() },
            AttributeInfo { id: "dexterity".to_string(), name: "Dexterity".to_string() },
            AttributeInfo { id: "adaptability".to_string(), name: "Adaptability".to_string() },
            AttributeInfo { id: "intelligence".to_string(), name: "Intelligence".to_string() },
            AttributeInfo { id: "faith".to_string(), name: "Faith".to_string() },
        ]
    }

    fn custom_triggers(&self) -> Vec<CustomTriggerType> {
        vec![
            CustomTriggerType {
                id: "kill_counter".to_string(),
                name: "Kill Counter".to_string(),
                description: "Triggers when boss kill count reaches a threshold (supports ascetics)".to_string(),
                parameters: vec![
                    CustomTriggerParam {
                        id: "boss".to_string(),
                        name: "Boss".to_string(),
                        param_type: CustomTriggerParamType::Select,
                        choices: Some(Self::build_boss_choices()),
                        default_value: Some("0".to_string()),
                        required: true,
                    },
                    CustomTriggerParam {
                        id: "comparison".to_string(),
                        name: "Comparison".to_string(),
                        param_type: CustomTriggerParamType::Comparison,
                        choices: None,
                        default_value: Some(">=".to_string()),
                        required: true,
                    },
                    CustomTriggerParam {
                        id: "count".to_string(),
                        name: "Kill Count".to_string(),
                        param_type: CustomTriggerParamType::Int,
                        choices: None,
                        default_value: Some("1".to_string()),
                        required: true,
                    },
                ],
            },
        ]
    }

    fn evaluate_custom_trigger(&self, trigger_id: &str, params: &HashMap<String, String>) -> bool {
        match trigger_id {
            "kill_counter" => {
                let boss_offset: u32 = params.get("boss")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0);
                let comparison = params.get("comparison")
                    .map(|s| s.as_str())
                    .unwrap_or(">=");
                let target_count: i32 = params.get("count")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(1);

                let current_count = self.get_boss_kill_count_raw(boss_offset).unwrap_or(0);

                match comparison {
                    ">=" => current_count >= target_count,
                    ">" => current_count > target_count,
                    "==" | "=" => current_count == target_count,
                    "<=" => current_count <= target_count,
                    "<" => current_count < target_count,
                    "!=" => current_count != target_count,
                    _ => current_count >= target_count,
                }
            }
            _ => false,
        }
    }
}

// =============================================================================
// FACTORY
// =============================================================================

pub struct DarkSouls2Factory;

impl GameFactory for DarkSouls2Factory {
    fn game_id(&self) -> &'static str {
        GAME_ID
    }

    fn process_names(&self) -> &[&'static str] {
        PROCESS_NAMES
    }

    fn create(&self) -> BoxedGame {
        Box::new(DarkSouls2::new())
    }
}
