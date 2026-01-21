//! ASL (Auto Splitter Language) parser module
//!
//! This module provides support for parsing LiveSplit ASL files and converting
//! them to the GameData format used by the generic autosplitter engine.
//!
//! # Supported ASL Features
//!
//! - `state()` block with process name and variable definitions
//! - Variable types: `bool`, `int`, `byte`, `float`
//! - Pointer references with flag IDs or offset chains
//! - `split`, `reset`, `isLoading` blocks with simple conditions
//! - `startup` and `init` blocks (parsed but not executed)
//!
//! # Example ASL
//!
//! ```asl
//! state("DarkSoulsIII.exe") {
//!     bool bossDefeated : "sprj_event_flag_man", 13000050;
//! }
//!
//! split {
//!     if (current.bossDefeated && !old.bossDefeated) { return true; }
//!     return false;
//! }
//! ```

mod error;
mod lexer;
mod parser;
mod converter;

pub use error::{AslError, AslResult};
pub use lexer::{Token, TokenKind, Lexer};
pub use parser::{AslScript, AslVariable, AslType, AslBlock, AslStatement, AslCondition, AslExpression, Parser};
pub use converter::{asl_to_game_data, detect_engine};

use crate::game_data::GameData;

/// Parse an ASL script string and convert it to GameData
///
/// This is the main entry point for ASL support. It handles the full pipeline:
/// 1. Tokenize the input
/// 2. Parse tokens into an AST
/// 3. Convert AST to GameData
///
/// # Arguments
///
/// * `asl_content` - The ASL script content as a string
/// * `engine_hint` - Optional engine type hint (e.g., "ds3", "elden_ring")
///
/// # Returns
///
/// A `GameData` struct that can be used with the generic autosplitter engine
pub fn parse_asl(asl_content: &str, engine_hint: Option<&str>) -> AslResult<GameData> {
    // Step 1: Tokenize
    let mut lexer = Lexer::new(asl_content);
    let tokens = lexer.tokenize()?;

    // Step 2: Parse
    let mut parser = Parser::new(tokens);
    let script = parser.parse()?;

    // Step 3: Convert to GameData
    let game_data = asl_to_game_data(&script, engine_hint)?;

    Ok(game_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_asl() {
        let asl = r#"
state("DarkSoulsIII.exe") {
    bool testBoss : "sprj_event_flag_man", 13000050;
}

split {
    if (current.testBoss && !old.testBoss) { return true; }
    return false;
}

reset {
    return false;
}

isLoading {
    return false;
}
"#;

        let result = parse_asl(asl, Some("ds3"));
        assert!(result.is_ok(), "Failed to parse ASL: {:?}", result.err());

        let game_data = result.unwrap();
        assert_eq!(game_data.game.process_names, vec!["DarkSoulsIII.exe"]);
        assert_eq!(game_data.bosses.len(), 1);
        assert_eq!(game_data.bosses[0].id, "testBoss");
        assert_eq!(game_data.bosses[0].flag_id, 13000050);
    }

    #[test]
    fn test_parse_ds2_style_asl() {
        let asl = r#"
state("DarkSoulsII.exe") {
    int lastGiant : "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x00;
}

split {
    if (current.lastGiant > 0 && old.lastGiant == 0) { return true; }
    return false;
}
"#;

        let result = parse_asl(asl, Some("ds2_sotfs"));
        assert!(result.is_ok(), "Failed to parse DS2 ASL: {:?}", result.err());

        let game_data = result.unwrap();
        assert_eq!(game_data.game.process_names, vec!["DarkSoulsII.exe"]);
        assert_eq!(game_data.bosses.len(), 1);
        assert_eq!(game_data.bosses[0].id, "lastGiant");
    }

    #[test]
    fn test_parse_real_ds3_asl() {
        // Inline DS3 ASL for testing
        let asl = r#"
state("DarkSoulsIII.exe") {
    bool iudexGundyr : "sprj_event_flag_man", 13000050;
    bool vordt : "sprj_event_flag_man", 13000800;
    bool curseRottedGreatwood : "sprj_event_flag_man", 13000830;
    bool crystalSage : "sprj_event_flag_man", 13100800;
    bool abyssWatchers : "sprj_event_flag_man", 13300850;
    bool dancer : "sprj_event_flag_man", 13000890;
    bool soulOfCinder : "sprj_event_flag_man", 14100800;
    bool friede : "sprj_event_flag_man", 14500860;
    bool gael : "sprj_event_flag_man", 15110800;
}

split {
    if (current.iudexGundyr && !old.iudexGundyr) { return true; }
    if (current.vordt && !old.vordt) { return true; }
    if (current.soulOfCinder && !old.soulOfCinder) { return true; }
    return false;
}

reset {
    return false;
}

isLoading {
    return false;
}
"#;
        let result = parse_asl(asl, None);
        assert!(result.is_ok(), "Failed to parse DS3 ASL: {:?}", result.err());

        let game_data = result.unwrap();
        assert_eq!(game_data.autosplitter.engine, "ds3");
        assert_eq!(game_data.bosses.len(), 9);

        // Check specific bosses
        let iudex = game_data.bosses.iter().find(|b| b.id == "iudexGundyr").unwrap();
        assert_eq!(iudex.flag_id, 13000050);
        assert!(!iudex.is_dlc);

        let friede = game_data.bosses.iter().find(|b| b.id == "friede").unwrap();
        assert!(friede.is_dlc, "Friede should be marked as DLC boss");

        let gael = game_data.bosses.iter().find(|b| b.id == "gael").unwrap();
        assert!(gael.is_dlc, "Gael should be marked as DLC boss");
    }

    #[test]
    fn test_parse_real_ds2_asl() {
        let asl = r#"
state("DarkSoulsII.exe") {
    int lastGiant : "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x00;
    int pursuer : "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x04;
    int lostSinner : "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x14;
    int nashandra : "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x74;
    int fumeKnight : "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x88;
}

split {
    if (current.lastGiant > 0 && old.lastGiant == 0) { return true; }
    if (current.pursuer > 0 && old.pursuer == 0) { return true; }
    return false;
}
"#;
        let result = parse_asl(asl, None);
        assert!(result.is_ok(), "Failed to parse DS2 ASL: {:?}", result.err());

        let game_data = result.unwrap();
        assert_eq!(game_data.autosplitter.engine, "ds2_sotfs");
        assert_eq!(game_data.bosses.len(), 5);

        // Check that offsets are used as flag_ids
        let last_giant = game_data.bosses.iter().find(|b| b.id == "lastGiant").unwrap();
        assert_eq!(last_giant.flag_id, 0x00);

        let pursuer = game_data.bosses.iter().find(|b| b.id == "pursuer").unwrap();
        assert_eq!(pursuer.flag_id, 0x04);
    }

    #[test]
    fn test_parse_real_elden_ring_asl() {
        let asl = r#"
state("eldenring.exe") {
    bool margit : "virtual_memory_flag", 10000800;
    bool godrick : "virtual_memory_flag", 10000850;
    bool rennala : "virtual_memory_flag", 14000850;
    bool radahn : "virtual_memory_flag", 30030800;
    bool morgott : "virtual_memory_flag", 11000850;
    bool maliketh : "virtual_memory_flag", 13000850;
    bool radagonEldenBeast : "virtual_memory_flag", 19000800;
    bool malenia : "virtual_memory_flag", 15000800;
}

split {
    if (current.margit && !old.margit) { return true; }
    if (current.godrick && !old.godrick) { return true; }
    return false;
}
"#;
        let result = parse_asl(asl, None);
        assert!(result.is_ok(), "Failed to parse Elden Ring ASL: {:?}", result.err());

        let game_data = result.unwrap();
        assert_eq!(game_data.autosplitter.engine, "elden_ring");
        assert_eq!(game_data.bosses.len(), 8);
    }

    #[test]
    fn test_parse_real_sekiro_asl() {
        let asl = r#"
state("sekiro.exe") {
    bool gyoubu : "event_flag_man", 11105520;
    bool genichiro : "event_flag_man", 11105810;
    bool guardianApe : "event_flag_man", 11505800;
    bool isshinSwordSaint : "event_flag_man", 11105850;
    bool demonOfHatred : "event_flag_man", 11105821;
}

split {
    if (current.gyoubu && !old.gyoubu) { return true; }
    if (current.genichiro && !old.genichiro) { return true; }
    return false;
}
"#;
        let result = parse_asl(asl, None);
        assert!(result.is_ok(), "Failed to parse Sekiro ASL: {:?}", result.err());

        let game_data = result.unwrap();
        assert_eq!(game_data.autosplitter.engine, "sekiro");
        assert_eq!(game_data.bosses.len(), 5);
    }

    #[test]
    fn test_parse_real_ac6_asl() {
        let asl = r#"
state("armoredcore6.exe") {
    bool balteus : "cs_event_flag_man", 30200200;
    bool seaSpider : "cs_event_flag_man", 30200500;
    bool iceWorm : "cs_event_flag_man", 30300500;
    bool handlerWalter : "cs_event_flag_man", 30500400;
    bool allMind : "cs_event_flag_man", 30500500;
}

split {
    if (current.balteus && !old.balteus) { return true; }
    if (current.allMind && !old.allMind) { return true; }
    return false;
}
"#;
        let result = parse_asl(asl, None);
        assert!(result.is_ok(), "Failed to parse AC6 ASL: {:?}", result.err());

        let game_data = result.unwrap();
        assert_eq!(game_data.autosplitter.engine, "ac6");
        assert_eq!(game_data.bosses.len(), 5);
    }

    #[test]
    fn test_game_data_to_toml_roundtrip() {
        let asl = r#"
state("DarkSoulsIII.exe") {
    bool boss1 : "sprj_event_flag_man", 13000050;
    bool boss2 : "sprj_event_flag_man", 13000800;
}

split {
    if (current.boss1 && !old.boss1) { return true; }
    return false;
}
"#;
        let game_data = parse_asl(asl, Some("ds3")).unwrap();

        // Convert to TOML
        let toml_str = toml::to_string(&game_data).unwrap();

        // Parse back from TOML
        let parsed: crate::game_data::GameData = toml::from_str(&toml_str).unwrap();

        assert_eq!(parsed.game.id, game_data.game.id);
        assert_eq!(parsed.bosses.len(), game_data.bosses.len());
        assert_eq!(parsed.autosplitter.engine, game_data.autosplitter.engine);
    }
}
