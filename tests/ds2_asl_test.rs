//! Test DS2 ASL script parsing and execution

use nyacore_autosplitter::asl::{parse_asl, Value};

#[test]
fn test_ds2_style_autosplitter() {
    // Test DS2-style autosplitter with module pointer paths
    let script = r#"
        state("DarkSoulsII.exe") {
            int lastGiant      : "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x00;
            int pursuer        : "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x04;
            int flexileSentry  : "game_manager_imp", 0x0, 0x70, 0x28, 0x20, 0x8, 0x0C;
        }

        startup {
            // Initialize settings
        }

        init {
            // Called when game process is detected
        }

        split {
            // Split when any boss kill count goes from 0 to > 0
            if (current.lastGiant > 0 && old.lastGiant == 0) { return true; }
            if (current.pursuer > 0 && old.pursuer == 0) { return true; }
            if (current.flexileSentry > 0 && old.flexileSentry == 0) { return true; }
            return false;
        }

        reset {
            return false;
        }

        isLoading {
            return false;
        }
    "#;

    let runtime = parse_asl(script);
    assert!(runtime.is_ok(), "DS2 script should parse successfully: {:?}", runtime.err());

    let mut runtime = runtime.unwrap();

    // Verify process name is correct
    assert_eq!(runtime.process_names(), &["DarkSoulsII.exe"]);

    // Verify variable definitions exist
    let defs = runtime.variable_definitions();
    assert_eq!(defs.len(), 3);
    assert_eq!(defs[0].name, "lastGiant");
    assert_eq!(defs[0].module, Some("game_manager_imp".to_string()));

    // Simulate boss kill: lastGiant goes from 0 to 1
    runtime.set_variable("lastGiant", Value::Int(0));
    runtime.set_variable("pursuer", Value::Int(0));
    runtime.set_variable("flexileSentry", Value::Int(0));
    runtime.tick();
    runtime.set_variable("lastGiant", Value::Int(1));
    runtime.set_variable("pursuer", Value::Int(0));
    runtime.set_variable("flexileSentry", Value::Int(0));

    let events = runtime.run_tick();
    assert!(events.split, "Should split when boss is killed");
}

#[test]
fn test_ds2_full_script_parsing() {
    // Test parsing the full DS2 ASL script
    let script = include_str!("../scripts/ds2_sotfs.asl");
    let result = parse_asl(script);

    match result {
        Ok(runtime) => {
            // Verify process name
            assert_eq!(runtime.process_names(), &["DarkSoulsII.exe"]);

            // Verify we have all boss variables defined
            let defs = runtime.variable_definitions();
            assert!(defs.len() >= 38, "Should have at least 38 boss variables, got {}", defs.len());

            // Verify first few bosses
            let names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
            assert!(names.contains(&"lastGiant"), "Should have lastGiant");
            assert!(names.contains(&"pursuer"), "Should have pursuer");
            assert!(names.contains(&"nashandra"), "Should have nashandra");
            assert!(names.contains(&"aldia"), "Should have aldia");
            assert!(names.contains(&"fumeKnight"), "Should have fumeKnight (DLC)");

            println!("DS2 script parsed successfully with {} variables", defs.len());
        }
        Err(e) => {
            panic!("Failed to parse DS2 script: {}", e);
        }
    }
}
