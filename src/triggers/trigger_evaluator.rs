//! Trigger evaluator that checks autosplit conditions against game state
//!
//! This module bridges the trigger definitions with the game-specific implementations,
//! evaluating whether triggers are satisfied based on live game data.

#[cfg(target_os = "windows")]
use super::triggers::{AutosplitTrigger, Comparison, TriggerLogic, AttributeType, ScreenStateType};
use super::triggers::AutosplitConfig;

#[cfg(target_os = "windows")]
use crate::games::{
    dark_souls_1::DarkSouls1,
    dark_souls_2::DarkSouls2,
    dark_souls_3::DarkSouls3,
    elden_ring::{EldenRing, ScreenState},
    sekiro::Sekiro,
    armored_core_6::ArmoredCore6,
};

/// State tracking for trigger evaluation
#[derive(Debug, Clone)]
pub struct TriggerState {
    /// Previous state of position triggers (inside/outside radius)
    pub position_states: Vec<bool>,
    /// Previous loading state
    pub was_loading: bool,
    /// Previous blackscreen state
    pub was_blackscreen: bool,
    /// Previous player loaded state
    pub was_player_loaded: bool,
    /// Event flags that have been triggered this run
    pub triggered_flags: Vec<u32>,
    /// Whether this split has been triggered this run
    pub triggered_this_run: bool,
}

impl Default for TriggerState {
    fn default() -> Self {
        Self {
            position_states: Vec::new(),
            was_loading: false,
            was_blackscreen: false,
            was_player_loaded: false,
            triggered_flags: Vec::new(),
            triggered_this_run: false,
        }
    }
}

impl TriggerState {
    pub fn new(num_position_triggers: usize) -> Self {
        Self {
            position_states: vec![false; num_position_triggers],
            was_loading: false,
            was_blackscreen: false,
            was_player_loaded: false,
            triggered_flags: Vec::new(),
            triggered_this_run: false,
        }
    }

    pub fn reset(&mut self) {
        self.triggered_flags.clear();
        self.triggered_this_run = false;
    }
}

/// Result of evaluating a single trigger
#[derive(Debug, Clone)]
pub struct TriggerResult {
    pub satisfied: bool,
    pub description: String,
}

/// Evaluator for autosplit triggers using game-specific implementations
#[cfg(target_os = "windows")]
pub struct TriggerEvaluator<'a> {
    game: &'a GameStateRef<'a>,
}

/// Reference to any supported game state
#[cfg(target_os = "windows")]
pub enum GameStateRef<'a> {
    DarkSouls1(&'a DarkSouls1),
    DarkSouls2(&'a DarkSouls2),
    DarkSouls3(&'a DarkSouls3),
    EldenRing(&'a EldenRing),
    Sekiro(&'a Sekiro),
    ArmoredCore6(&'a ArmoredCore6),
}

#[cfg(target_os = "windows")]
impl<'a> TriggerEvaluator<'a> {
    pub fn new(game: &'a GameStateRef<'a>) -> Self {
        Self { game }
    }

    /// Evaluate a complete autosplit configuration
    pub fn evaluate_config(&self, config: &AutosplitConfig, state: &mut TriggerState) -> bool {
        if !config.enabled || config.triggers.is_empty() {
            return false;
        }

        // Check if already triggered this run (if once_per_run is set)
        if config.once_per_run && state.triggered_this_run {
            return false;
        }

        let results: Vec<bool> = config.triggers.iter()
            .enumerate()
            .map(|(i, trigger)| self.evaluate_trigger(trigger, state, i))
            .collect();

        let satisfied = match config.logic {
            TriggerLogic::All => results.iter().all(|&r| r),
            TriggerLogic::Any => results.iter().any(|&r| r),
        };

        if satisfied {
            state.triggered_this_run = true;
        }

        satisfied
    }

    /// Evaluate a single trigger
    pub fn evaluate_trigger(&self, trigger: &AutosplitTrigger, state: &mut TriggerState, trigger_index: usize) -> bool {
        match trigger {
            AutosplitTrigger::EventFlag { flag_id, on_set } => {
                self.evaluate_event_flag(*flag_id, *on_set, state)
            }
            AutosplitTrigger::CustomFlag { flag_id, on_set } => {
                self.evaluate_event_flag(*flag_id, *on_set, state)
            }
            AutosplitTrigger::InGameTime { comparison, target_ms } => {
                self.evaluate_igt(*comparison, *target_ms)
            }
            AutosplitTrigger::Position { x, y, z, radius, on_enter } => {
                self.evaluate_position(*x, *y, *z, *radius, *on_enter, state, trigger_index)
            }
            AutosplitTrigger::MapArea { area, block, region } => {
                self.evaluate_map_area(*area, *block, *region)
            }
            AutosplitTrigger::Loading { on_start } => {
                self.evaluate_loading(*on_start, state)
            }
            AutosplitTrigger::Blackscreen { on_start } => {
                self.evaluate_blackscreen(*on_start, state)
            }
            AutosplitTrigger::Attribute { attribute, comparison, value } => {
                self.evaluate_attribute(*attribute, *comparison, *value)
            }
            AutosplitTrigger::NGLevel { comparison, level } => {
                self.evaluate_ng_level(*comparison, *level)
            }
            AutosplitTrigger::PlayerHealth { comparison, value } => {
                self.evaluate_player_health(*comparison, *value)
            }
            AutosplitTrigger::PlayerLoaded { on_load } => {
                self.evaluate_player_loaded(*on_load, state)
            }
            AutosplitTrigger::ScreenState { state: target_state } => {
                self.evaluate_screen_state(*target_state)
            }
            AutosplitTrigger::BossKillCount { boss_offset, comparison, count } => {
                self.evaluate_boss_kill_count(*boss_offset, *comparison, *count)
            }
            AutosplitTrigger::WarpRequested => {
                self.evaluate_warp_requested()
            }
            AutosplitTrigger::CreditsRolling => {
                self.evaluate_credits_rolling()
            }
        }
    }

    fn evaluate_event_flag(&self, flag_id: u32, on_set: bool, state: &mut TriggerState) -> bool {
        let flag_set = match self.game {
            GameStateRef::DarkSouls1(g) => g.read_event_flag(flag_id),
            GameStateRef::DarkSouls2(g) => g.read_event_flag(flag_id),
            GameStateRef::DarkSouls3(g) => g.read_event_flag(flag_id),
            GameStateRef::EldenRing(g) => g.read_event_flag(flag_id),
            GameStateRef::Sekiro(g) => g.read_event_flag(flag_id),
            GameStateRef::ArmoredCore6(g) => g.read_event_flag(flag_id),
        };

        // Check for state change (not just current state) to avoid re-triggering
        let already_triggered = state.triggered_flags.contains(&flag_id);

        if on_set {
            // Trigger when flag becomes set
            if flag_set && !already_triggered {
                state.triggered_flags.push(flag_id);
                return true;
            }
        } else {
            // Trigger when flag becomes unset (rare use case)
            if !flag_set && already_triggered {
                state.triggered_flags.retain(|&f| f != flag_id);
                return true;
            }
        }

        false
    }

    fn evaluate_igt(&self, comparison: Comparison, target_ms: u64) -> bool {
        let current_ms = match self.game {
            GameStateRef::DarkSouls1(g) => g.get_in_game_time_milliseconds(),
            GameStateRef::DarkSouls2(g) => g.get_in_game_time_milliseconds(),
            GameStateRef::DarkSouls3(g) => g.get_in_game_time_milliseconds(),
            GameStateRef::EldenRing(g) => g.get_in_game_time_milliseconds(),
            GameStateRef::Sekiro(g) => g.get_in_game_time_milliseconds(),
            GameStateRef::ArmoredCore6(g) => g.get_in_game_time_milliseconds(),
        } as u64;

        comparison.evaluate(current_ms, target_ms)
    }

    fn evaluate_position(&self, x: f32, y: f32, z: f32, radius: f32, on_enter: bool, state: &mut TriggerState, trigger_index: usize) -> bool {
        let (px, py, pz) = match self.game {
            GameStateRef::DarkSouls1(g) => {
                let pos = g.get_position();
                (pos.x, pos.y, pos.z)
            }
            GameStateRef::DarkSouls2(g) => {
                let pos = g.get_position();
                (pos.x, pos.y, pos.z)
            }
            GameStateRef::DarkSouls3(g) => {
                let pos = g.get_position();
                (pos.x, pos.y, pos.z)
            }
            GameStateRef::EldenRing(g) => {
                let pos = g.get_position();
                (pos.x, pos.y, pos.z)
            }
            GameStateRef::Sekiro(g) => {
                let pos = g.get_player_position();
                (pos.x, pos.y, pos.z)
            }
            GameStateRef::ArmoredCore6(_) => {
                // AC6 doesn't have position tracking
                return false;
            }
        };

        // Check if player is within radius
        let distance_sq = (px - x).powi(2) + (py - y).powi(2) + (pz - z).powi(2);
        let is_inside = distance_sq <= radius.powi(2);

        // Ensure we have state for this trigger
        while state.position_states.len() <= trigger_index {
            state.position_states.push(false);
        }

        let was_inside = state.position_states[trigger_index];
        state.position_states[trigger_index] = is_inside;

        if on_enter {
            // Trigger when entering (was outside, now inside)
            !was_inside && is_inside
        } else {
            // Trigger when exiting (was inside, now outside)
            was_inside && !is_inside
        }
    }

    fn evaluate_map_area(&self, area: u8, block: u8, region: Option<u8>) -> bool {
        if let GameStateRef::EldenRing(g) = self.game {
            let pos = g.get_position();
            if pos.area == area && pos.block == block {
                if let Some(r) = region {
                    return pos.region == r;
                }
                return true;
            }
        }
        false
    }

    fn evaluate_loading(&self, on_start: bool, state: &mut TriggerState) -> bool {
        let is_loading = match self.game {
            GameStateRef::DarkSouls2(g) => g.is_loading(),
            GameStateRef::DarkSouls3(g) => g.is_loading(),
            GameStateRef::ArmoredCore6(g) => g.is_loading_screen_visible(),
            _ => return false,
        };

        let was_loading = state.was_loading;
        state.was_loading = is_loading;

        if on_start {
            // Trigger when loading starts
            !was_loading && is_loading
        } else {
            // Trigger when loading ends
            was_loading && !is_loading
        }
    }

    fn evaluate_blackscreen(&self, on_start: bool, state: &mut TriggerState) -> bool {
        let is_blackscreen = match self.game {
            GameStateRef::DarkSouls3(g) => g.blackscreen_active(),
            GameStateRef::EldenRing(g) => g.is_blackscreen_active(),
            GameStateRef::Sekiro(g) => g.is_blackscreen_active(),
            _ => return false,
        };

        let was_blackscreen = state.was_blackscreen;
        state.was_blackscreen = is_blackscreen;

        if on_start {
            !was_blackscreen && is_blackscreen
        } else {
            was_blackscreen && !is_blackscreen
        }
    }

    fn evaluate_attribute(&self, attribute: AttributeType, comparison: Comparison, target: i32) -> bool {
        let value = match self.game {
            GameStateRef::DarkSouls1(g) => {
                use crate::games::dark_souls_1::Attribute as DS1Attr;
                // DS1 uses "Vitality" for HP (maps to Vigor in other games)
                let attr = match attribute {
                    AttributeType::SoulLevel => DS1Attr::SoulLevel,
                    AttributeType::Vigor => DS1Attr::Vitality, // DS1 calls it Vitality
                    AttributeType::Vitality => DS1Attr::Vitality,
                    AttributeType::Attunement => DS1Attr::Attunement,
                    AttributeType::Endurance => DS1Attr::Endurance,
                    AttributeType::Strength => DS1Attr::Strength,
                    AttributeType::Dexterity => DS1Attr::Dexterity,
                    AttributeType::Intelligence => DS1Attr::Intelligence,
                    AttributeType::Faith => DS1Attr::Faith,
                    _ => return false,
                };
                g.get_attribute(attr)
            }
            GameStateRef::DarkSouls2(g) => {
                use crate::games::dark_souls_2::Attribute as DS2Attr;
                let attr = match attribute {
                    AttributeType::SoulLevel => DS2Attr::SoulLevel,
                    AttributeType::Vigor => DS2Attr::Vigor,
                    AttributeType::Endurance => DS2Attr::Endurance,
                    AttributeType::Vitality => DS2Attr::Vitality,
                    AttributeType::Attunement => DS2Attr::Attunement,
                    AttributeType::Strength => DS2Attr::Strength,
                    AttributeType::Dexterity => DS2Attr::Dexterity,
                    AttributeType::Adaptability => DS2Attr::Adaptability,
                    AttributeType::Intelligence => DS2Attr::Intelligence,
                    AttributeType::Faith => DS2Attr::Faith,
                    _ => return false,
                };
                g.get_attribute(attr)
            }
            GameStateRef::DarkSouls3(g) => {
                use crate::games::dark_souls_3::Attribute as DS3Attr;
                let attr = match attribute {
                    AttributeType::SoulLevel => DS3Attr::SoulLevel,
                    AttributeType::Vigor => DS3Attr::Vigor,
                    AttributeType::Attunement => DS3Attr::Attunement,
                    AttributeType::Endurance => DS3Attr::Endurance,
                    AttributeType::Vitality => DS3Attr::Vitality,
                    AttributeType::Strength => DS3Attr::Strength,
                    AttributeType::Dexterity => DS3Attr::Dexterity,
                    AttributeType::Intelligence => DS3Attr::Intelligence,
                    AttributeType::Faith => DS3Attr::Faith,
                    AttributeType::Luck => DS3Attr::Luck,
                    _ => return false,
                };
                g.read_attribute(attr)
            }
            GameStateRef::Sekiro(g) => {
                use crate::games::sekiro::Attribute as SekiroAttr;
                let attr = match attribute {
                    AttributeType::Vitality => SekiroAttr::Vitality,
                    AttributeType::AttackPower => SekiroAttr::AttackPower,
                    _ => return false,
                };
                g.get_attribute(attr)
            }
            _ => return false,
        };

        comparison.evaluate(value, target)
    }

    fn evaluate_ng_level(&self, comparison: Comparison, target: i32) -> bool {
        let level = match self.game {
            GameStateRef::DarkSouls1(g) => g.ng_count(),
            GameStateRef::EldenRing(g) => g.read_ng_level(),
            _ => return false,
        };

        comparison.evaluate(level, target)
    }

    fn evaluate_player_health(&self, comparison: Comparison, target: i32) -> bool {
        let health = match self.game {
            GameStateRef::DarkSouls1(g) => g.get_player_health(),
            _ => return false,
        };

        comparison.evaluate(health, target)
    }

    fn evaluate_player_loaded(&self, on_load: bool, state: &mut TriggerState) -> bool {
        let is_loaded = match self.game {
            GameStateRef::DarkSouls1(g) => g.is_player_loaded(),
            GameStateRef::DarkSouls3(g) => g.is_player_loaded(),
            GameStateRef::EldenRing(g) => g.is_player_loaded(),
            GameStateRef::Sekiro(g) => g.is_player_loaded(),
            _ => return false,
        };

        let was_loaded = state.was_player_loaded;
        state.was_player_loaded = is_loaded;

        if on_load {
            !was_loaded && is_loaded
        } else {
            was_loaded && !is_loaded
        }
    }

    fn evaluate_screen_state(&self, target_state: ScreenStateType) -> bool {
        if let GameStateRef::EldenRing(g) = self.game {
            let current = g.get_screen_state();
            match (target_state, current) {
                (ScreenStateType::Loading, ScreenState::Loading) => true,
                (ScreenStateType::Logo, ScreenState::Logo) => true,
                (ScreenStateType::MainMenu, ScreenState::MainMenu) => true,
                (ScreenStateType::InGame, ScreenState::InGame) => true,
                _ => false,
            }
        } else {
            false
        }
    }

    fn evaluate_boss_kill_count(&self, boss_offset: u32, comparison: Comparison, target: i32) -> bool {
        if let GameStateRef::DarkSouls2(g) = self.game {
            let count = g.get_boss_kill_count_raw(boss_offset);
            comparison.evaluate(count, target)
        } else {
            false
        }
    }

    fn evaluate_warp_requested(&self) -> bool {
        if let GameStateRef::DarkSouls1(g) = self.game {
            g.is_warp_requested()
        } else {
            false
        }
    }

    fn evaluate_credits_rolling(&self) -> bool {
        if let GameStateRef::DarkSouls1(g) = self.game {
            g.are_credits_rolling()
        } else {
            false
        }
    }
}

/// Non-Windows stub
#[cfg(not(target_os = "windows"))]
pub struct TriggerEvaluator;

#[cfg(not(target_os = "windows"))]
impl TriggerEvaluator {
    pub fn evaluate_config(&self, _config: &AutosplitConfig, _state: &mut TriggerState) -> bool {
        false
    }
}
