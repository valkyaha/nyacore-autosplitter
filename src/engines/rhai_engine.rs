//! Rhai Scripting Engine
//!
//! This engine allows games to define their autosplitter logic using Rhai scripts.
//! Rhai is a simple, safe scripting language embedded in Rust.
//!
//! # Script API
//!
//! Scripts have access to the following functions:
//!
//! ## Memory Reading
//! - `read_u8(address)` - Read unsigned byte
//! - `read_u16(address)` - Read unsigned 16-bit integer
//! - `read_u32(address)` - Read unsigned 32-bit integer
//! - `read_u64(address)` - Read unsigned 64-bit integer
//! - `read_i32(address)` - Read signed 32-bit integer
//! - `read_i64(address)` - Read signed 64-bit integer
//! - `read_f32(address)` - Read 32-bit float
//! - `read_ptr(address)` - Read pointer (64-bit)
//! - `read_bool(address)` - Read boolean (non-zero = true)
//!
//! ## Pointer Operations
//! - `get_pointer(name)` - Get a resolved pattern pointer
//!
//! ## Variables
//! - `get_var(name)` - Get a stored variable
//! - `set_var(name, value)` - Store a variable
//!
//! ## Required Functions
//! - `read_flag(flag_id)` - Read an event flag
//!
//! ## Optional Functions
//! - `init()` - Called once when process is attached
//! - `get_igt()` - Get in-game time in milliseconds
//! - `is_loading()` - Check if game is loading
//! - `is_player_loaded()` - Check if player is in world
//! - `get_position()` - Get player position (returns [x, y, z])
//! - `get_attribute(attr_name)` - Get character attribute
//! - `get_kill_count(flag_id)` - Get boss kill count
//! - `update()` - Called every tick
//! - `should_split()` - Check if should trigger split
//! - `should_start()` - Check if timer should start
//! - `should_reset()` - Check if timer should reset

use super::{Engine, EngineContext, EngineType};
use crate::games::config::PatternConfig;
use crate::memory::{parse_pattern, scan_pattern, extract_relative_address, MemoryReader};
use crate::AutosplitterError;
use rhai::{Engine as RhaiVM, Scope, AST, Dynamic, Array};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Shared state between Rhai script and Rust
struct SharedState {
    pointers: HashMap<String, usize>,
    variables: HashMap<String, Dynamic>,
    reader: Option<Arc<dyn MemoryReader>>,
}

/// Rhai scripting engine
pub struct RhaiEngine {
    /// The Rhai VM
    vm: RhaiVM,
    /// Compiled script AST
    ast: Option<AST>,
    /// Script source code
    #[allow(dead_code)]
    source: String,
    /// Pattern configurations
    patterns: Vec<PatternConfig>,
    /// Shared state
    state: Arc<RwLock<SharedState>>,
}

impl RhaiEngine {
    /// Create a new Rhai engine from script source
    pub fn new(source: String, patterns: Vec<PatternConfig>) -> Result<Self, AutosplitterError> {
        let mut vm = RhaiVM::new();

        // Limit script capabilities for safety
        vm.set_max_expr_depths(64, 64);
        vm.set_max_call_levels(32);
        vm.set_max_operations(1_000_000);
        vm.set_max_string_size(10_000);
        vm.set_max_array_size(10_000);
        vm.set_max_map_size(1_000);

        let state = Arc::new(RwLock::new(SharedState {
            pointers: HashMap::new(),
            variables: HashMap::new(),
            reader: None,
        }));

        // Register memory reading functions
        Self::register_memory_functions(&mut vm, state.clone());

        // Register pointer/variable functions
        Self::register_state_functions(&mut vm, state.clone());

        // Compile the script
        let ast = vm.compile(&source)
            .map_err(|e| AutosplitterError::ScriptError(format!("Compilation error: {}", e)))?;

        Ok(Self {
            vm,
            ast: Some(ast),
            source,
            patterns,
            state,
        })
    }

    fn register_memory_functions(vm: &mut RhaiVM, state: Arc<RwLock<SharedState>>) {
        // read_u8
        let s = state.clone();
        vm.register_fn("read_u8", move |address: i64| -> i64 {
            let guard = s.read().unwrap();
            if let Some(ref reader) = guard.reader {
                reader.read_u8(address as usize).map(|v| v as i64).unwrap_or(0)
            } else {
                0
            }
        });

        // read_u16
        let s = state.clone();
        vm.register_fn("read_u16", move |address: i64| -> i64 {
            let guard = s.read().unwrap();
            if let Some(ref reader) = guard.reader {
                reader.read_u16(address as usize).map(|v| v as i64).unwrap_or(0)
            } else {
                0
            }
        });

        // read_u32
        let s = state.clone();
        vm.register_fn("read_u32", move |address: i64| -> i64 {
            let guard = s.read().unwrap();
            if let Some(ref reader) = guard.reader {
                reader.read_u32(address as usize).map(|v| v as i64).unwrap_or(0)
            } else {
                0
            }
        });

        // read_i32
        let s = state.clone();
        vm.register_fn("read_i32", move |address: i64| -> i64 {
            let guard = s.read().unwrap();
            if let Some(ref reader) = guard.reader {
                reader.read_i32(address as usize).map(|v| v as i64).unwrap_or(0)
            } else {
                0
            }
        });

        // read_u64
        let s = state.clone();
        vm.register_fn("read_u64", move |address: i64| -> i64 {
            let guard = s.read().unwrap();
            if let Some(ref reader) = guard.reader {
                reader.read_u64(address as usize).map(|v| v as i64).unwrap_or(0)
            } else {
                0
            }
        });

        // read_i64
        let s = state.clone();
        vm.register_fn("read_i64", move |address: i64| -> i64 {
            let guard = s.read().unwrap();
            if let Some(ref reader) = guard.reader {
                reader.read_i64(address as usize).unwrap_or(0)
            } else {
                0
            }
        });

        // read_f32
        let s = state.clone();
        vm.register_fn("read_f32", move |address: i64| -> f64 {
            let guard = s.read().unwrap();
            if let Some(ref reader) = guard.reader {
                reader.read_f32(address as usize).map(|v| v as f64).unwrap_or(0.0)
            } else {
                0.0
            }
        });

        // read_ptr
        let s = state.clone();
        vm.register_fn("read_ptr", move |address: i64| -> i64 {
            let guard = s.read().unwrap();
            if let Some(ref reader) = guard.reader {
                reader.read_ptr(address as usize).map(|v| v as i64).unwrap_or(0)
            } else {
                0
            }
        });

        // read_bool
        let s = state.clone();
        vm.register_fn("read_bool", move |address: i64| -> bool {
            let guard = s.read().unwrap();
            if let Some(ref reader) = guard.reader {
                reader.read_bool(address as usize).unwrap_or(false)
            } else {
                false
            }
        });

        // read_bytes
        let s = state.clone();
        vm.register_fn("read_bytes", move |address: i64, size: i64| -> Array {
            let guard = s.read().unwrap();
            if let Some(ref reader) = guard.reader {
                if let Some(bytes) = reader.read_bytes(address as usize, size as usize) {
                    bytes.into_iter().map(|b| Dynamic::from(b as i64)).collect()
                } else {
                    Array::new()
                }
            } else {
                Array::new()
            }
        });
    }

    fn register_state_functions(vm: &mut RhaiVM, state: Arc<RwLock<SharedState>>) {
        // get_pointer(name) -> i64
        let s = state.clone();
        vm.register_fn("get_pointer", move |name: &str| -> i64 {
            let guard = s.read().unwrap();
            guard.pointers.get(name).copied().unwrap_or(0) as i64
        });

        // has_pointer(name) -> bool
        let s = state.clone();
        vm.register_fn("has_pointer", move |name: &str| -> bool {
            let guard = s.read().unwrap();
            guard.pointers.contains_key(name)
        });

        // get_var(name) -> Dynamic
        let s = state.clone();
        vm.register_fn("get_var", move |name: &str| -> Dynamic {
            let guard = s.read().unwrap();
            guard.variables.get(name).cloned().unwrap_or(Dynamic::UNIT)
        });

        // set_var(name, value)
        let s = state.clone();
        vm.register_fn("set_var", move |name: &str, value: Dynamic| {
            let mut guard = s.write().unwrap();
            guard.variables.insert(name.to_string(), value);
        });

        // log(message)
        vm.register_fn("log", |message: &str| {
            log::info!("[Rhai] {}", message);
        });

        vm.register_fn("log_debug", |message: &str| {
            log::debug!("[Rhai] {}", message);
        });
    }

    /// Set the memory reader
    fn set_reader(&self, reader: Arc<dyn MemoryReader>) {
        let mut guard = self.state.write().unwrap();
        guard.reader = Some(reader);
    }

    /// Scan and resolve patterns
    fn scan_patterns(&mut self, ctx: &mut EngineContext) -> Result<(), AutosplitterError> {
        let base = ctx.base_address();
        let size = ctx.module_size();

        for pattern_config in &self.patterns {
            let pattern = parse_pattern(&pattern_config.pattern);

            if let Some(match_addr) = scan_pattern(ctx.reader(), base, size, &pattern) {
                let resolved = if pattern_config.rip_offset > 0 {
                    extract_relative_address(
                        ctx.reader(),
                        match_addr,
                        pattern_config.rip_offset,
                        pattern_config.instruction_len,
                    ).unwrap_or(0)
                } else {
                    match_addr
                };

                // Follow pointer chain if specified
                let final_addr = if !pattern_config.pointer_offsets.is_empty() {
                    ctx.follow_pointer_chain(resolved, &pattern_config.pointer_offsets)
                        .unwrap_or(resolved)
                } else {
                    resolved
                };

                log::debug!(
                    "Pattern '{}': match=0x{:X}, final=0x{:X}",
                    pattern_config.name, match_addr, final_addr
                );

                // Store in both engine context and shared state
                ctx.set_pointer(&pattern_config.name, final_addr);
                let mut state = self.state.write().unwrap();
                state.pointers.insert(pattern_config.name.clone(), final_addr);
            } else {
                log::warn!("Pattern '{}' not found", pattern_config.name);
            }
        }

        Ok(())
    }

    /// Call a script function
    fn call_fn<T: Clone + Send + Sync + 'static>(&self, fn_name: &str, args: impl rhai::FuncArgs) -> Result<T, AutosplitterError> {
        let ast = self.ast.as_ref()
            .ok_or_else(|| AutosplitterError::ScriptError("Script not compiled".to_string()))?;

        self.vm.call_fn::<T>(&mut Scope::new(), ast, fn_name, args)
            .map_err(|e| AutosplitterError::ScriptError(format!("{}: {}", fn_name, e)))
    }

    /// Check if a function exists in the script
    fn has_function(&self, fn_name: &str) -> bool {
        if let Some(ast) = &self.ast {
            ast.iter_functions().any(|f| f.name == fn_name)
        } else {
            false
        }
    }
}

impl Engine for RhaiEngine {
    fn engine_type(&self) -> EngineType {
        EngineType::Rhai
    }

    fn init(&mut self, ctx: &mut EngineContext) -> Result<(), AutosplitterError> {
        // Store reader reference
        self.set_reader(ctx.reader_arc());

        // Scan patterns first
        self.scan_patterns(ctx)?;

        // Call init function if it exists
        if self.has_function("init") {
            let ast = self.ast.as_ref().unwrap();
            let _: () = self.vm.call_fn(&mut Scope::new(), ast, "init", ())
                .map_err(|e| AutosplitterError::ScriptError(format!("init: {}", e)))?;
        }

        log::info!("RhaiEngine initialized");
        Ok(())
    }

    fn read_flag(&self, _ctx: &EngineContext, flag_id: u32) -> Result<bool, AutosplitterError> {
        self.call_fn("read_flag", (flag_id as i64,))
    }

    fn get_kill_count(&self, ctx: &EngineContext, flag_id: u32) -> Result<u32, AutosplitterError> {
        if self.has_function("get_kill_count") {
            let result: i64 = self.call_fn("get_kill_count", (flag_id as i64,))?;
            Ok(result as u32)
        } else {
            // Default implementation
            Ok(if self.read_flag(ctx, flag_id)? { 1 } else { 0 })
        }
    }

    fn get_igt_milliseconds(&self, _ctx: &EngineContext) -> Result<Option<i32>, AutosplitterError> {
        if self.has_function("get_igt") {
            let result: i64 = self.call_fn("get_igt", ())?;
            Ok(Some(result as i32))
        } else {
            Ok(None)
        }
    }

    fn is_loading(&self, _ctx: &EngineContext) -> Result<Option<bool>, AutosplitterError> {
        if self.has_function("is_loading") {
            let result: bool = self.call_fn("is_loading", ())?;
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }

    fn is_player_loaded(&self, _ctx: &EngineContext) -> Result<Option<bool>, AutosplitterError> {
        if self.has_function("is_player_loaded") {
            let result: bool = self.call_fn("is_player_loaded", ())?;
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }

    fn get_position(&self, _ctx: &EngineContext) -> Result<Option<(f32, f32, f32)>, AutosplitterError> {
        if self.has_function("get_position") {
            let result: Array = self.call_fn("get_position", ())?;
            if result.len() >= 3 {
                let x = result[0].clone().cast::<f64>() as f32;
                let y = result[1].clone().cast::<f64>() as f32;
                let z = result[2].clone().cast::<f64>() as f32;
                Ok(Some((x, y, z)))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    fn get_attribute(&self, _ctx: &EngineContext, attr: &str) -> Result<Option<i32>, AutosplitterError> {
        if self.has_function("get_attribute") {
            let result: i64 = self.call_fn("get_attribute", (attr.to_string(),))?;
            Ok(Some(result as i32))
        } else {
            Ok(None)
        }
    }

    fn update(&mut self, _ctx: &mut EngineContext) -> Result<(), AutosplitterError> {
        if self.has_function("update") {
            let ast = self.ast.as_ref().unwrap();
            let _: () = self.vm.call_fn(&mut Scope::new(), ast, "update", ())
                .map_err(|e| AutosplitterError::ScriptError(format!("update: {}", e)))?;
        }
        Ok(())
    }

    fn should_split(&self, _ctx: &EngineContext) -> Result<bool, AutosplitterError> {
        if self.has_function("should_split") {
            self.call_fn("should_split", ())
        } else {
            Ok(false)
        }
    }

    fn should_start(&self, _ctx: &EngineContext) -> Result<bool, AutosplitterError> {
        if self.has_function("should_start") {
            self.call_fn("should_start", ())
        } else {
            Ok(false)
        }
    }

    fn should_reset(&self, _ctx: &EngineContext) -> Result<bool, AutosplitterError> {
        if self.has_function("should_reset") {
            self.call_fn("should_reset", ())
        } else {
            Ok(false)
        }
    }
}
