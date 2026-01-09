//! Configurable game implementation that loads from TOML config files
//!
//! This replaces hardcoded game implementations with a single flexible
//! game type that can be configured from NYA-Core-Assets plugin files.

use std::collections::HashMap;
use std::sync::Arc;
use std::path::Path;

use super::{
    Game, GameFactory, BoxedGame, Position3D, TriggerTypeInfo, AttributeInfo,
    CustomTriggerType, CustomTriggerParam, CustomTriggerParamType, CustomTriggerChoice,
    config::{
        PluginConfig, AutosplitterConfig, FlagAlgorithm, PatternConfig,
        CategoryConfig, TreeConfig, OffsetTableConfig, KillCounterConfig,
    },
};
use crate::memory::{ProcessContext, MemoryReader, Pointer, parse_pattern, extract_relative_address};
use crate::AutosplitterError;

// =============================================================================
// CONFIGURABLE GAME
// =============================================================================

/// A game implementation that is configured from TOML files
pub struct ConfigurableGame {
    // Metadata
    id: String,
    name: String,
    process_names: Vec<String>,

    // Runtime state
    reader: Option<Arc<dyn MemoryReader>>,
    initialized: bool,

    // Configuration
    algorithm: FlagAlgorithm,
    category_config: Option<CategoryConfig>,
    tree_config: Option<TreeConfig>,
    offset_config: Option<OffsetTableConfig>,
    kill_counter_config: Option<KillCounterConfig>,

    // Memory patterns and pointers
    patterns: Vec<PatternConfig>,
    resolved_patterns: HashMap<String, Pointer>,

    // Memory layout
    igt_offset: Option<i64>,
    loading_offset: Option<i64>,
    blackscreen_offset: Option<i64>,
    position_offsets: Option<(i64, i64, i64)>,
    attributes: HashMap<String, i64>,

    // Custom triggers
    custom_triggers: Vec<CustomTriggerType>,

    // For offset table (DS1)
    group_offsets: HashMap<char, i32>,
    area_indices: HashMap<String, i32>,
}

impl ConfigurableGame {
    /// Create a new configurable game from plugin and autosplitter configs
    pub fn from_configs(plugin: &PluginConfig, autosplitter: &AutosplitterConfig) -> Self {
        let settings = &autosplitter.autosplitter;

        // Build group/area offsets for DS1-style games
        let mut group_offsets = HashMap::new();
        let mut area_indices = HashMap::new();

        if let Some(ref offset_cfg) = settings.offset_config {
            for (k, v) in &offset_cfg.group_offsets {
                if let Some(c) = k.chars().next() {
                    group_offsets.insert(c, *v as i32);
                }
            }
            for (k, v) in &offset_cfg.area_indices {
                area_indices.insert(k.clone(), *v as i32);
            }
        }

        // Build custom triggers
        let custom_triggers = settings.custom_triggers.iter().map(|ct| {
            CustomTriggerType {
                id: ct.id.clone(),
                name: ct.name.clone(),
                description: ct.description.clone(),
                parameters: ct.parameters.iter().map(|p| {
                    CustomTriggerParam {
                        id: p.id.clone(),
                        name: p.name.clone(),
                        param_type: match p.param_type.as_str() {
                            "int" => CustomTriggerParamType::Int,
                            "string" => CustomTriggerParamType::String,
                            "bool" => CustomTriggerParamType::Bool,
                            "select" => CustomTriggerParamType::Select,
                            "comparison" => CustomTriggerParamType::Comparison,
                            _ => CustomTriggerParamType::String,
                        },
                        choices: p.choices.as_ref().map(|choices| {
                            choices.iter().map(|c| CustomTriggerChoice {
                                value: c.value.clone(),
                                label: c.label.clone(),
                                group: c.group.clone(),
                            }).collect()
                        }),
                        default_value: p.default_value.clone(),
                        required: p.required,
                    }
                }).collect(),
            }
        }).collect();

        // Position offsets
        let position_offsets = settings.memory_layout.position_offsets.as_ref()
            .map(|p| (p.x, p.y, p.z));

        Self {
            id: plugin.plugin.id.clone(),
            name: plugin.plugin.name.clone(),
            process_names: plugin.process.names.clone(),
            reader: None,
            initialized: false,
            algorithm: settings.algorithm,
            category_config: settings.category_config.clone(),
            tree_config: settings.tree_config.clone(),
            offset_config: settings.offset_config.clone(),
            kill_counter_config: settings.kill_counter_config.clone(),
            patterns: settings.patterns.clone(),
            resolved_patterns: HashMap::new(),
            igt_offset: settings.memory_layout.igt_offset,
            loading_offset: settings.memory_layout.loading_offset,
            blackscreen_offset: settings.memory_layout.blackscreen_offset,
            position_offsets,
            attributes: settings.memory_layout.attributes.clone(),
            custom_triggers,
            group_offsets,
            area_indices,
        }
    }

    /// Load a game from plugin directory
    pub fn load_from_dir(plugin_dir: &Path) -> Result<Self, AutosplitterError> {
        let plugin_path = plugin_dir.join("plugin.toml");
        let autosplitter_path = plugin_dir.join("autosplitter.toml");

        let plugin = PluginConfig::load(&plugin_path)
            .map_err(|e| AutosplitterError::ConfigError(e.to_string()))?;

        let autosplitter = AutosplitterConfig::load(&autosplitter_path)
            .map_err(|e| AutosplitterError::ConfigError(e.to_string()))?;

        Ok(Self::from_configs(&plugin, &autosplitter))
    }

    fn reader(&self) -> Option<&dyn MemoryReader> {
        self.reader.as_ref().map(|r| r.as_ref())
    }

    /// Get a resolved pattern pointer by name
    fn get_pattern(&self, name: &str) -> Option<&Pointer> {
        self.resolved_patterns.get(name)
    }

    // =========================================================================
    // FLAG READING ALGORITHMS
    // =========================================================================

    /// Read event flag using category decomposition (DS3/Sekiro style)
    fn read_flag_category_decomposition(&self, reader: &dyn MemoryReader, flag_id: u32) -> bool {
        let config = match &self.category_config {
            Some(c) => c,
            None => return false,
        };

        let primary = match self.get_pattern(&config.primary_pattern) {
            Some(p) => p,
            None => return false,
        };

        let secondary = match self.get_pattern(&config.secondary_pattern) {
            Some(p) => p,
            None => return false,
        };

        let flag_div_10000000 = ((flag_id / 10_000_000) % 10) as i64;
        let flag_area = ((flag_id / 100_000) % 100) as i32;
        let flag_div_10000 = ((flag_id / 10_000) % 10) as i32;
        let flag_div_1000 = ((flag_id / 1_000) % 10) as i64;

        let mut category: i32 = -1;

        if flag_area >= 90 || flag_area + flag_div_10000 == 0 {
            category = 0;
        } else {
            if secondary.is_null_ptr(reader) {
                return false;
            }

            let world_info_owner = secondary
                .append(&[config.field_area_base_offset])
                .create_pointer_from_address(reader, Some(config.world_info_offset));
            let size = world_info_owner.read_i32(reader, Some(0x8));
            let vector = world_info_owner.append(&[0x10]);

            for i in 0..size {
                let area = vector.read_byte(reader, Some((i as i64 * config.world_info_struct_size) + 0xb)) as i32;

                if area == flag_area {
                    let count = vector.read_byte(reader, Some(i as i64 * config.world_info_struct_size + 0x20));
                    let mut index = 0i64;
                    let mut found = false;
                    let mut block_vec_result: Option<Pointer> = None;

                    if count >= 1 {
                        loop {
                            let block_vec = vector.create_pointer_from_address(
                                reader,
                                Some(i as i64 * config.world_info_struct_size + 0x28)
                            );
                            let flag = block_vec.read_i32(reader, Some((index * config.world_block_struct_size) + 0x8));

                            if ((flag >> 0x10) & 0xff) == flag_div_10000
                                && (flag >> 0x18) == flag_area
                            {
                                found = true;
                                block_vec_result = Some(block_vec);
                                break;
                            }

                            index += 1;
                            if count as i64 <= index {
                                break;
                            }
                        }
                    }

                    if found {
                        if let Some(ref block_vec) = block_vec_result {
                            category = block_vec.read_i32(reader, Some((index * config.world_block_struct_size) + 0x20));
                            break;
                        }
                    }
                }
            }

            if category >= 0 {
                category += 1;
            }
        }

        let ptr = primary.append(&[config.base_offset, flag_div_10000000 * config.entry_size, 0x0]);

        if ptr.is_null_ptr(reader) || category < 0 {
            return false;
        }

        let result_base = (flag_div_1000 << 4)
            + ptr.get_address(reader)
            + (category as i64 * config.category_multiplier);

        let mut result_ptr = Pointer::new();
        result_ptr.initialize(true, result_base, &[0x0]);

        if !result_ptr.is_null_ptr(reader) {
            let mod_1000 = (flag_id % 1000) as u32;
            let read_offset = ((mod_1000 >> 5) * 4) as i64;
            let value = result_ptr.read_u32(reader, Some(read_offset));

            let bit_shift = 0x1f - ((mod_1000 as u8) & 0x1f);
            let mask = 1u32 << (bit_shift & 0x1f);

            return (value & mask) != 0;
        }

        false
    }

    /// Read event flag using binary tree (Elden Ring/AC6 style)
    fn read_flag_binary_tree(&self, reader: &dyn MemoryReader, flag_id: u32) -> bool {
        let config = match &self.tree_config {
            Some(c) => c,
            None => return false,
        };

        let ptr = match self.get_pattern(&config.primary_pattern) {
            Some(p) => p,
            None => return false,
        };

        let divisor = ptr.read_i32(reader, Some(config.divisor_offset));
        if divisor == 0 {
            return false;
        }

        let category = flag_id / divisor as u32;
        let remainder = flag_id - (category * divisor as u32);

        // Tree traversal
        let root = ptr.create_pointer_from_address(reader, Some(config.tree_root_offset));
        let mut current = root.copy();
        let mut sub = current.create_pointer_from_address(reader, Some(0x8));

        while sub.read_byte(reader, Some(0x19)) == 0 {
            if (sub.read_i32(reader, Some(0x20)) as u32) < category {
                sub = sub.create_pointer_from_address(reader, Some(0x10));
            } else {
                current = sub.copy();
                sub = sub.create_pointer_from_address(reader, Some(0x0));
            }
        }

        let curr_addr = current.get_address(reader);
        let sub_addr = sub.get_address(reader);

        if curr_addr == sub_addr || category < (current.read_i32(reader, Some(0x20)) as u32) {
            current = sub.copy();
        }

        let curr_addr = current.get_address(reader);
        let sub_addr = sub.get_address(reader);

        if curr_addr == sub_addr {
            return false;
        }

        let mystery = reader.read_i32((curr_addr + 0x28) as usize).unwrap_or(0) - 1;

        let calc_ptr: i64 = if mystery == 0 {
            let mult = ptr.read_i32(reader, Some(config.multiplier_offset));
            let elem = reader.read_i32((curr_addr + 0x30) as usize).unwrap_or(0);
            let base = ptr.read_i64(reader, Some(config.base_addr_offset));
            (mult as i64 * elem as i64) + base
        } else if mystery == 1 {
            return false;
        } else {
            reader.read_i64((curr_addr + 0x30) as usize).unwrap_or(0)
        };

        if calc_ptr == 0 {
            return false;
        }

        let thing = 7 - (remainder & 7);
        let mask = 1i32 << thing;
        let shifted = remainder >> 3;

        if let Some(value) = reader.read_i32((calc_ptr + shifted as i64) as usize) {
            return (value & mask) != 0;
        }

        false
    }

    /// Read event flag using offset table (DS1 style)
    fn read_flag_offset_table(&self, reader: &dyn MemoryReader, flag_id: u32) -> bool {
        let config = match &self.offset_config {
            Some(c) => c,
            None => return false,
        };

        let ptr = match self.get_pattern(&config.primary_pattern) {
            Some(p) => p,
            None => return false,
        };

        let id_str = format!("{:08}", flag_id);
        if id_str.len() != 8 {
            return false;
        }

        let group = match id_str.chars().next() {
            Some(c) => c,
            None => return false,
        };
        let area = &id_str[1..4];
        let section: i32 = match id_str[4..5].parse() {
            Ok(v) => v,
            Err(_) => return false,
        };
        let number: i32 = match id_str[5..8].parse() {
            Ok(v) => v,
            Err(_) => return false,
        };

        let group_offset = match self.group_offsets.get(&group) {
            Some(v) => *v,
            None => return false,
        };
        let area_offset = match self.area_indices.get(area) {
            Some(v) => *v,
            None => return false,
        };

        let mut offset = group_offset;
        offset += area_offset * 0x500;
        offset += section * 128;
        offset += (number - (number % 32)) / 8;

        let mask = 0x80000000u32 >> (number % 32);

        let address = ptr.get_address(reader);
        if address == 0 {
            return false;
        }

        if let Some(value) = reader.read_u32((address + offset as i64) as usize) {
            return (value & mask) != 0;
        }

        false
    }

    /// Read boss kill count (DS2 style)
    fn read_kill_count(&self, reader: &dyn MemoryReader, boss_offset: u32) -> i32 {
        let config = match &self.kill_counter_config {
            Some(c) => c,
            None => return 0,
        };

        let ptr = match self.get_pattern(&config.primary_pattern) {
            Some(p) => p,
            None => return 0,
        };

        ptr.read_i32(reader, Some(boss_offset as i64))
    }
}

// =============================================================================
// GAME TRAIT IMPLEMENTATION
// =============================================================================

impl Game for ConfigurableGame {
    fn id(&self) -> &'static str {
        // Leak the string to get a 'static lifetime
        // This is safe because games are long-lived
        Box::leak(self.id.clone().into_boxed_str())
    }

    fn name(&self) -> &'static str {
        Box::leak(self.name.clone().into_boxed_str())
    }

    fn process_names(&self) -> &[&'static str] {
        // Convert Vec<String> to &[&'static str]
        let names: Vec<&'static str> = self.process_names.iter()
            .map(|s| Box::leak(s.clone().into_boxed_str()) as &'static str)
            .collect();
        Box::leak(names.into_boxed_slice())
    }

    fn init_pointers(&mut self, ctx: &mut ProcessContext) -> Result<(), AutosplitterError> {
        log::info!("{}: Initializing pointers for base 0x{:X}, size 0x{:X}",
            self.id, ctx.base_address, ctx.module_size);

        self.reader = Some(ctx.reader());
        let reader = self.reader.as_ref().unwrap();

        // Scan all patterns
        for pattern_cfg in &self.patterns {
            let pattern = parse_pattern(&pattern_cfg.pattern);

            if let Some(found) = ctx.scan_pattern(&pattern) {
                if let Some(addr) = extract_relative_address(
                    reader.as_ref(),
                    found,
                    pattern_cfg.rip_offset,
                    pattern_cfg.instruction_len
                ) {
                    let mut ptr = Pointer::new();
                    ptr.initialize(ctx.is_64_bit, addr as i64, &pattern_cfg.pointer_offsets);
                    self.resolved_patterns.insert(pattern_cfg.name.clone(), ptr);
                    log::info!("{}: {} at 0x{:X}", self.id, pattern_cfg.name, addr);
                }
            } else {
                log::warn!("{}: Pattern '{}' not found", self.id, pattern_cfg.name);
            }
        }

        self.initialized = true;
        log::info!("{}: Initialization complete", self.id);
        Ok(())
    }

    fn read_event_flag(&self, flag_id: u32) -> bool {
        if !self.initialized {
            return false;
        }

        let reader = match self.reader() {
            Some(r) => r,
            None => return false,
        };

        match self.algorithm {
            FlagAlgorithm::CategoryDecomposition => self.read_flag_category_decomposition(reader, flag_id),
            FlagAlgorithm::BinaryTree => self.read_flag_binary_tree(reader, flag_id),
            FlagAlgorithm::OffsetTable => self.read_flag_offset_table(reader, flag_id),
            FlagAlgorithm::KillCounter => self.read_kill_count(reader, flag_id) > 0,
            FlagAlgorithm::None => false,
        }
    }

    fn get_boss_kill_count(&self, flag_id: u32) -> u32 {
        if self.algorithm == FlagAlgorithm::KillCounter {
            let reader = match self.reader() {
                Some(r) => r,
                None => return 0,
            };
            self.read_kill_count(reader, flag_id) as u32
        } else {
            if self.read_event_flag(flag_id) { 1 } else { 0 }
        }
    }

    fn is_alive(&self) -> bool {
        self.initialized
    }

    fn get_igt_milliseconds(&self) -> Option<i32> {
        if !self.initialized {
            return None;
        }
        // This would need the proper pointer - simplified for now
        None
    }

    fn get_position(&self) -> Option<Position3D> {
        if !self.initialized {
            return None;
        }
        // This would need the proper pointer - simplified for now
        None
    }

    fn supported_triggers(&self) -> Vec<TriggerTypeInfo> {
        let mut triggers = vec![
            TriggerTypeInfo {
                id: "event_flag".to_string(),
                name: "Event Flag".to_string(),
                description: "Triggers when an event flag is set".to_string(),
            },
        ];

        if self.algorithm == FlagAlgorithm::KillCounter {
            triggers.push(TriggerTypeInfo {
                id: "kill_count".to_string(),
                name: "Kill Count".to_string(),
                description: "Triggers based on boss kill count".to_string(),
            });
        }

        triggers
    }

    fn available_attributes(&self) -> Vec<AttributeInfo> {
        self.attributes.keys()
            .map(|k| AttributeInfo {
                id: k.clone(),
                name: k.replace('_', " ").to_string(),
            })
            .collect()
    }

    fn custom_triggers(&self) -> Vec<CustomTriggerType> {
        self.custom_triggers.clone()
    }

    fn evaluate_custom_trigger(&self, trigger_id: &str, params: &HashMap<String, String>) -> bool {
        if self.algorithm == FlagAlgorithm::KillCounter && trigger_id == "kill_counter" {
            let boss_offset: u32 = params.get("boss")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            let comparison = params.get("comparison")
                .map(|s| s.as_str())
                .unwrap_or(">=");
            let target: i32 = params.get("count")
                .and_then(|v| v.parse().ok())
                .unwrap_or(1);

            let reader = match self.reader() {
                Some(r) => r,
                None => return false,
            };
            let current = self.read_kill_count(reader, boss_offset);

            return match comparison {
                ">=" => current >= target,
                ">" => current > target,
                "==" | "=" => current == target,
                "<=" => current <= target,
                "<" => current < target,
                "!=" => current != target,
                _ => current >= target,
            };
        }
        false
    }

    fn get_boss_kill_count_raw(&self, boss_offset: u32) -> Option<i32> {
        if self.algorithm == FlagAlgorithm::KillCounter {
            let reader = self.reader()?;
            Some(self.read_kill_count(reader, boss_offset))
        } else {
            None
        }
    }
}

// =============================================================================
// FACTORY
// =============================================================================

/// Factory for creating configurable games from plugin directories
pub struct ConfigurableGameFactory {
    plugin_dir: std::path::PathBuf,
    plugin_config: PluginConfig,
    autosplitter_config: AutosplitterConfig,
}

impl ConfigurableGameFactory {
    /// Create a new factory from a plugin directory
    pub fn from_dir(plugin_dir: &Path) -> Result<Self, AutosplitterError> {
        let plugin_path = plugin_dir.join("plugin.toml");
        let autosplitter_path = plugin_dir.join("autosplitter.toml");

        let plugin_config = PluginConfig::load(&plugin_path)
            .map_err(|e| AutosplitterError::ConfigError(e.to_string()))?;

        let autosplitter_config = AutosplitterConfig::load(&autosplitter_path)
            .map_err(|e| AutosplitterError::ConfigError(e.to_string()))?;

        Ok(Self {
            plugin_dir: plugin_dir.to_path_buf(),
            plugin_config,
            autosplitter_config,
        })
    }
}

impl GameFactory for ConfigurableGameFactory {
    fn game_id(&self) -> &'static str {
        Box::leak(self.plugin_config.plugin.id.clone().into_boxed_str())
    }

    fn process_names(&self) -> &[&'static str] {
        let names: Vec<&'static str> = self.plugin_config.process.names.iter()
            .map(|s| Box::leak(s.clone().into_boxed_str()) as &'static str)
            .collect();
        Box::leak(names.into_boxed_slice())
    }

    fn create(&self) -> BoxedGame {
        Box::new(ConfigurableGame::from_configs(&self.plugin_config, &self.autosplitter_config))
    }
}
