//! ASL to GameData converter
//!
//! Converts parsed ASL scripts into GameData structures that can be used
//! by the generic autosplitter engine.

use std::collections::HashMap;

use super::error::AslResult;
use super::parser::{AslScript, AslVariable};
use crate::game_data::{
    AutosplitterConfig, BossDefinition, GameData, GameInfo, PatternDefinition, PointerDefinition,
    PresetDefinition,
};

/// Engine type for known games
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineType {
    Ds1Ptde,
    Ds1Remaster,
    Ds2Sotfs,
    Ds3,
    EldenRing,
    Sekiro,
    Ac6,
    Generic,
}

impl EngineType {
    /// Convert to engine string for GameData
    pub fn as_str(&self) -> &'static str {
        match self {
            EngineType::Ds1Ptde => "ds1_ptde",
            EngineType::Ds1Remaster => "ds1_remaster",
            EngineType::Ds2Sotfs => "ds2_sotfs",
            EngineType::Ds3 => "ds3",
            EngineType::EldenRing => "elden_ring",
            EngineType::Sekiro => "sekiro",
            EngineType::Ac6 => "ac6",
            EngineType::Generic => "generic",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "ds1_ptde" | "ds1ptde" | "darksouls1ptde" => EngineType::Ds1Ptde,
            "ds1_remaster" | "ds1remaster" | "ds1" | "darksouls1" | "darksoulsremastered" => {
                EngineType::Ds1Remaster
            }
            "ds2_sotfs" | "ds2sotfs" | "ds2" | "darksouls2" | "darksoulsii" => EngineType::Ds2Sotfs,
            "ds3" | "darksouls3" | "darksoulsiii" => EngineType::Ds3,
            "elden_ring" | "eldenring" | "er" => EngineType::EldenRing,
            "sekiro" => EngineType::Sekiro,
            "ac6" | "armoredcore6" => EngineType::Ac6,
            _ => EngineType::Generic,
        }
    }
}

/// Detect engine type from process name
pub fn detect_engine(process_name: &str, hint: Option<&str>) -> EngineType {
    // If we have a hint, use it
    if let Some(hint) = hint {
        return EngineType::from_str(hint);
    }

    // Otherwise detect from process name
    let name_lower = process_name.to_lowercase();

    if name_lower.contains("darksoulsiii") {
        EngineType::Ds3
    } else if name_lower.contains("darksoulsii") {
        EngineType::Ds2Sotfs
    } else if name_lower.contains("darksoulsremastered") {
        EngineType::Ds1Remaster
    } else if name_lower.contains("darksouls") {
        // Could be PTDE or remaster
        EngineType::Ds1Remaster
    } else if name_lower.contains("eldenring") {
        EngineType::EldenRing
    } else if name_lower.contains("sekiro") {
        EngineType::Sekiro
    } else if name_lower.contains("armoredcore6") {
        EngineType::Ac6
    } else {
        EngineType::Generic
    }
}

/// Convert an ASL script to GameData
pub fn asl_to_game_data(script: &AslScript, engine_hint: Option<&str>) -> AslResult<GameData> {
    let engine = detect_engine(&script.process_name, engine_hint);

    // Extract game ID from process name
    let game_id = script
        .process_name
        .to_lowercase()
        .replace(".exe", "")
        .replace(" ", "_");

    // Create display name from process name
    let display_name = humanize_process_name(&script.process_name);

    // Convert variables to boss definitions
    let bosses: Vec<BossDefinition> = script
        .variables
        .iter()
        .map(|v| variable_to_boss(v, &engine))
        .collect();

    // Extract patterns from variables
    let patterns = extract_patterns(&script.variables, &engine);

    // Extract pointers from variables
    let pointers = extract_pointers(&script.variables, &engine);

    // Create default preset with all bosses
    let preset = PresetDefinition {
        id: "all_bosses".to_string(),
        name: "All Bosses".to_string(),
        description: Some("All bosses from ASL file".to_string()),
        bosses: bosses.iter().map(|b| b.id.clone()).collect(),
        custom: HashMap::new(),
        boss_overrides: HashMap::new(),
    };

    Ok(GameData {
        game: GameInfo {
            id: game_id,
            name: display_name,
            short_name: None,
            process_names: vec![script.process_name.clone()],
        },
        autosplitter: AutosplitterConfig {
            engine: engine.as_str().to_string(),
            patterns,
            pointers,
        },
        bosses,
        presets: vec![preset],
        custom_fields: HashMap::new(),
        attributes: Vec::new(),
    })
}

/// Convert a variable definition to a boss definition
fn variable_to_boss(var: &AslVariable, engine: &EngineType) -> BossDefinition {
    // For DS2-style offset chains, the last offset is the flag_id
    // For DS3-style single value, it's the flag_id directly
    let flag_id = if var.offsets.is_empty() {
        0
    } else if var.offsets.len() == 1 {
        // Single value - this is the flag ID
        var.offsets[0] as u32
    } else {
        // DS2 style - last offset is the actual offset from base
        // For the generic engine, we use a combined identifier
        match engine {
            EngineType::Ds2Sotfs => {
                // For DS2, the flag_id represents the final offset in the chain
                *var.offsets.last().unwrap_or(&0) as u32
            }
            _ => {
                // For other games with offset chains, use the last offset
                *var.offsets.last().unwrap_or(&0) as u32
            }
        }
    };

    // Detect if this is a DLC boss (heuristic based on flag ranges)
    let is_dlc = is_dlc_boss(&var.name, flag_id, engine);

    BossDefinition {
        id: var.name.clone(),
        name: humanize_name(&var.name),
        flag_id,
        is_dlc,
        custom: HashMap::new(),
    }
}

/// Check if a boss is DLC based on name or flag range
fn is_dlc_boss(name: &str, flag_id: u32, engine: &EngineType) -> bool {
    let name_lower = name.to_lowercase();

    // Common DLC boss name patterns
    if name_lower.contains("dlc")
        || name_lower.contains("friede")
        || name_lower.contains("gael")
        || name_lower.contains("midir")
        || name_lower.contains("halflight")
        || name_lower.contains("demonprince")
        || name_lower.contains("gravetender")
    {
        return true;
    }

    // Check flag ranges for DLC areas
    match engine {
        EngineType::Ds3 => {
            // DS3 DLC flags are in 14500000+ and 15000000+ ranges
            flag_id >= 14500000 || (flag_id >= 15000000 && flag_id < 20000000)
        }
        EngineType::EldenRing => {
            // Elden Ring DLC flags (Shadow of the Erdtree)
            // Placeholder - add actual ranges when known
            false
        }
        EngineType::Ds2Sotfs => {
            // DS2 DLC bosses are at higher offsets (0x7C+)
            flag_id >= 0x7C
        }
        _ => false,
    }
}

/// Extract pattern definitions from variables
fn extract_patterns(variables: &[AslVariable], engine: &EngineType) -> Vec<PatternDefinition> {
    let mut pattern_names: Vec<String> = variables
        .iter()
        .map(|v| v.pointer_name.clone())
        .collect();
    pattern_names.sort();
    pattern_names.dedup();

    // Get default patterns for known engines
    let known_patterns = get_engine_patterns(engine);

    pattern_names
        .into_iter()
        .map(|name| {
            known_patterns
                .get(&name)
                .cloned()
                .unwrap_or_else(|| PatternDefinition {
                    name: name.clone(),
                    pattern: String::new(), // Will need to be filled in
                    resolve: "rip_relative".to_string(),
                    rip_offset: 3,
                    extra_offset: 0,
                })
        })
        .collect()
}

/// Extract pointer definitions from variables
fn extract_pointers(
    variables: &[AslVariable],
    engine: &EngineType,
) -> HashMap<String, PointerDefinition> {
    let mut pointers = HashMap::new();

    // Group variables by pointer name to extract common offset chains
    let mut by_pattern: HashMap<String, Vec<&AslVariable>> = HashMap::new();
    for var in variables {
        by_pattern
            .entry(var.pointer_name.clone())
            .or_default()
            .push(var);
    }

    // For DS2-style offset chains, create pointer definitions
    if *engine == EngineType::Ds2Sotfs {
        for (pattern_name, vars) in by_pattern {
            if let Some(first) = vars.first() {
                if first.offsets.len() > 1 {
                    // Use all offsets except the last one (which is the boss-specific offset)
                    let base_offsets: Vec<i64> = first.offsets[..first.offsets.len() - 1].to_vec();
                    pointers.insert(
                        format!("{}_base", pattern_name),
                        PointerDefinition {
                            pattern: pattern_name.clone(),
                            offsets: base_offsets,
                        },
                    );
                }
            }
        }
    }

    pointers
}

/// Get known patterns for an engine type
fn get_engine_patterns(engine: &EngineType) -> HashMap<String, PatternDefinition> {
    let mut patterns = HashMap::new();

    match engine {
        EngineType::Ds3 => {
            patterns.insert(
                "sprj_event_flag_man".to_string(),
                PatternDefinition {
                    name: "sprj_event_flag_man".to_string(),
                    pattern: "48 c7 05 ? ? ? ? 00 00 00 00 48 8b 7c 24 38 c7 46 54 ff ff ff ff 48 83 c4 20 5e c3".to_string(),
                    resolve: "rip_relative".to_string(),
                    rip_offset: 3,
                    extra_offset: 11,
                },
            );
        }
        EngineType::Ds2Sotfs => {
            patterns.insert(
                "game_manager_imp".to_string(),
                PatternDefinition {
                    name: "game_manager_imp".to_string(),
                    pattern: "48 8b 35 ? ? ? ? 48 8b e9 48 85 f6".to_string(),
                    resolve: "rip_relative".to_string(),
                    rip_offset: 3,
                    extra_offset: 0,
                },
            );
        }
        EngineType::EldenRing => {
            patterns.insert(
                "virtual_memory_flag".to_string(),
                PatternDefinition {
                    name: "virtual_memory_flag".to_string(),
                    pattern: "44 89 7c 24 28 4c 8b 25 ? ? ? ? 4d 85 e4".to_string(),
                    resolve: "rip_relative".to_string(),
                    rip_offset: 8,
                    extra_offset: 0,
                },
            );
        }
        EngineType::Ds1Remaster => {
            patterns.insert(
                "event_flags".to_string(),
                PatternDefinition {
                    name: "event_flags".to_string(),
                    pattern: "48 8b 0d ? ? ? ? 99 41 0f b6 d8".to_string(),
                    resolve: "rip_relative".to_string(),
                    rip_offset: 3,
                    extra_offset: 0,
                },
            );
        }
        EngineType::Sekiro => {
            patterns.insert(
                "sprj_event_flag_man".to_string(),
                PatternDefinition {
                    name: "sprj_event_flag_man".to_string(),
                    pattern: "48 8b 0d ? ? ? ? 48 89 5c 24 50 48 89 6c 24 58".to_string(),
                    resolve: "rip_relative".to_string(),
                    rip_offset: 3,
                    extra_offset: 0,
                },
            );
        }
        EngineType::Ac6 => {
            patterns.insert(
                "sprj_event_flag_man".to_string(),
                PatternDefinition {
                    name: "sprj_event_flag_man".to_string(),
                    pattern: "48 8b 3d ? ? ? ? 48 85 ff 0f 84 ? ? ? ? 48 8b 1f".to_string(),
                    resolve: "rip_relative".to_string(),
                    rip_offset: 3,
                    extra_offset: 0,
                },
            );
        }
        _ => {}
    }

    patterns
}

/// Convert camelCase or snake_case variable name to human readable
fn humanize_name(name: &str) -> String {
    let mut result = String::new();
    let mut prev_lower = false;

    for ch in name.chars() {
        if ch == '_' {
            result.push(' ');
            prev_lower = false;
        } else if ch.is_uppercase() && prev_lower {
            result.push(' ');
            result.push(ch);
            prev_lower = false;
        } else {
            if result.is_empty() || result.ends_with(' ') {
                result.push(ch.to_ascii_uppercase());
            } else {
                result.push(ch);
            }
            prev_lower = ch.is_lowercase();
        }
    }

    result
}

/// Convert process name to human readable game name
fn humanize_process_name(process_name: &str) -> String {
    let base = process_name.replace(".exe", "").replace(".EXE", "");

    match base.to_lowercase().as_str() {
        "darksoulsiii" => "Dark Souls III".to_string(),
        "darksoulsii" => "Dark Souls II: Scholar of the First Sin".to_string(),
        "darksoulsremastered" => "Dark Souls Remastered".to_string(),
        "eldenring" => "Elden Ring".to_string(),
        "sekiro" => "Sekiro: Shadows Die Twice".to_string(),
        "armoredcore6" => "Armored Core VI: Fires of Rubicon".to_string(),
        _ => humanize_name(&base),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asl::lexer::Lexer;
    use crate::asl::parser::Parser;

    fn parse_and_convert(input: &str, hint: Option<&str>) -> AslResult<GameData> {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(tokens);
        let script = parser.parse()?;
        asl_to_game_data(&script, hint)
    }

    #[test]
    fn test_detect_engine() {
        assert_eq!(
            detect_engine("DarkSoulsIII.exe", None),
            EngineType::Ds3
        );
        assert_eq!(
            detect_engine("DarkSoulsII.exe", None),
            EngineType::Ds2Sotfs
        );
        assert_eq!(
            detect_engine("eldenring.exe", None),
            EngineType::EldenRing
        );
        assert_eq!(
            detect_engine("DarkSoulsRemastered.exe", None),
            EngineType::Ds1Remaster
        );
        assert_eq!(detect_engine("sekiro.exe", None), EngineType::Sekiro);
        assert_eq!(
            detect_engine("armoredcore6.exe", None),
            EngineType::Ac6
        );
        assert_eq!(
            detect_engine("unknown.exe", None),
            EngineType::Generic
        );
    }

    #[test]
    fn test_detect_engine_with_hint() {
        assert_eq!(
            detect_engine("custom.exe", Some("ds3")),
            EngineType::Ds3
        );
        assert_eq!(
            detect_engine("custom.exe", Some("elden_ring")),
            EngineType::EldenRing
        );
    }

    #[test]
    fn test_humanize_name() {
        assert_eq!(humanize_name("iudexGundyr"), "Iudex Gundyr");
        assert_eq!(humanize_name("vordt"), "Vordt");
        assert_eq!(humanize_name("soul_of_cinder"), "Soul Of Cinder");
        assert_eq!(humanize_name("lastGiant"), "Last Giant");
    }

    #[test]
    fn test_humanize_process_name() {
        assert_eq!(
            humanize_process_name("DarkSoulsIII.exe"),
            "Dark Souls III"
        );
        assert_eq!(humanize_process_name("eldenring.exe"), "Elden Ring");
    }

    #[test]
    fn test_convert_ds3_style() {
        let input = r#"
state("DarkSoulsIII.exe") {
    bool iudexGundyr : "sprj_event_flag_man", 13000050;
    bool vordt : "sprj_event_flag_man", 13000800;
}

split {
    if (current.iudexGundyr && !old.iudexGundyr) { return true; }
    return false;
}
"#;
        let game_data = parse_and_convert(input, None).unwrap();

        assert_eq!(game_data.game.id, "darksoulsiii");
        assert_eq!(game_data.game.name, "Dark Souls III");
        assert_eq!(game_data.autosplitter.engine, "ds3");
        assert_eq!(game_data.bosses.len(), 2);

        assert_eq!(game_data.bosses[0].id, "iudexGundyr");
        assert_eq!(game_data.bosses[0].flag_id, 13000050);
        assert_eq!(game_data.bosses[0].name, "Iudex Gundyr");

        assert_eq!(game_data.bosses[1].id, "vordt");
        assert_eq!(game_data.bosses[1].flag_id, 13000800);
    }

    #[test]
    fn test_convert_ds2_style() {
        let input = r#"
state("DarkSoulsII.exe") {
    int lastGiant : "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x00;
    int pursuer : "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x04;
}

split {
    if (current.lastGiant > 0 && old.lastGiant == 0) { return true; }
    return false;
}
"#;
        let game_data = parse_and_convert(input, None).unwrap();

        assert_eq!(game_data.game.id, "darksoulsii");
        assert_eq!(game_data.autosplitter.engine, "ds2_sotfs");
        assert_eq!(game_data.bosses.len(), 2);

        // DS2 style - flag_id is the last offset
        assert_eq!(game_data.bosses[0].id, "lastGiant");
        assert_eq!(game_data.bosses[0].flag_id, 0x00);
        assert_eq!(game_data.bosses[1].flag_id, 0x04);
    }

    #[test]
    fn test_convert_elden_ring() {
        let input = r#"
state("eldenring.exe") {
    bool margit : "virtual_memory_flag", 10000800;
    bool godrick : "virtual_memory_flag", 10000850;
}

split {
    if (current.margit && !old.margit) { return true; }
    return false;
}
"#;
        let game_data = parse_and_convert(input, None).unwrap();

        assert_eq!(game_data.game.id, "eldenring");
        assert_eq!(game_data.autosplitter.engine, "elden_ring");
    }

    #[test]
    fn test_dlc_detection_ds3() {
        let input = r#"
state("DarkSoulsIII.exe") {
    bool vordt : "sprj_event_flag_man", 13000800;
    bool friede : "sprj_event_flag_man", 14500860;
    bool gael : "sprj_event_flag_man", 15110800;
}
"#;
        let game_data = parse_and_convert(input, Some("ds3")).unwrap();

        assert!(!game_data.bosses[0].is_dlc); // vordt
        assert!(game_data.bosses[1].is_dlc); // friede
        assert!(game_data.bosses[2].is_dlc); // gael
    }

    #[test]
    fn test_preset_generation() {
        let input = r#"
state("game.exe") {
    bool boss1 : "ptr", 100;
    bool boss2 : "ptr", 200;
}
"#;
        let game_data = parse_and_convert(input, None).unwrap();

        assert_eq!(game_data.presets.len(), 1);
        assert_eq!(game_data.presets[0].id, "all_bosses");
        assert_eq!(game_data.presets[0].bosses.len(), 2);
    }

    #[test]
    fn test_engine_type_as_str() {
        assert_eq!(EngineType::Ds3.as_str(), "ds3");
        assert_eq!(EngineType::EldenRing.as_str(), "elden_ring");
        assert_eq!(EngineType::Ds2Sotfs.as_str(), "ds2_sotfs");
    }

    #[test]
    fn test_engine_type_from_str() {
        assert_eq!(EngineType::from_str("ds3"), EngineType::Ds3);
        assert_eq!(EngineType::from_str("elden_ring"), EngineType::EldenRing);
        assert_eq!(EngineType::from_str("ds2_sotfs"), EngineType::Ds2Sotfs);
        assert_eq!(EngineType::from_str("unknown"), EngineType::Generic);
    }

    #[test]
    fn test_pattern_extraction() {
        let input = r#"
state("DarkSoulsIII.exe") {
    bool boss : "sprj_event_flag_man", 13000050;
}
"#;
        let game_data = parse_and_convert(input, Some("ds3")).unwrap();

        assert!(!game_data.autosplitter.patterns.is_empty());
        let pattern = &game_data.autosplitter.patterns[0];
        assert_eq!(pattern.name, "sprj_event_flag_man");
        assert!(!pattern.pattern.is_empty());
    }
}
