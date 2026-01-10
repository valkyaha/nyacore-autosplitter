//! LiveSplit ASL (Auto Splitter Language) Engine
//!
//! This engine provides compatibility with LiveSplit's ASL scripts.
//! ASL is a C#-like scripting language used by the speedrunning community.
//!
//! # ASL Script Structure
//!
//! ```asl
//! state("GameName.exe") {
//!     int eventFlags : "GameName.exe", 0x12345, 0x10, 0x20;
//!     bool isLoading : "GameName.exe", 0x12345, 0x30;
//! }
//!
//! start {
//!     return current.eventFlags == 1;
//! }
//!
//! split {
//!     return current.eventFlags > old.eventFlags;
//! }
//!
//! reset {
//!     return current.isLoading && !old.isLoading;
//! }
//!
//! isLoading {
//!     return current.isLoading;
//! }
//!
//! gameTime {
//!     return TimeSpan.FromMilliseconds(current.gameTime);
//! }
//! ```
//!
//! # Supported Features
//!
//! - State variables with pointer chains
//! - start, split, reset, isLoading, gameTime actions
//! - current and old state access
//! - Basic expressions and comparisons

use super::{Engine, EngineContext, EngineType};
use crate::AutosplitterError;
use std::collections::HashMap;

/// ASL variable types
#[derive(Debug, Clone, PartialEq)]
pub enum AslType {
    Bool,
    Byte,
    SByte,
    Short,
    UShort,
    Int,
    UInt,
    Long,
    ULong,
    Float,
    Double,
    String(usize), // String with max length
}

impl AslType {
    fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "bool" | "boolean" => Some(AslType::Bool),
            "byte" => Some(AslType::Byte),
            "sbyte" => Some(AslType::SByte),
            "short" | "int16" => Some(AslType::Short),
            "ushort" | "uint16" => Some(AslType::UShort),
            "int" | "int32" => Some(AslType::Int),
            "uint" | "uint32" => Some(AslType::UInt),
            "long" | "int64" => Some(AslType::Long),
            "ulong" | "uint64" => Some(AslType::ULong),
            "float" => Some(AslType::Float),
            "double" => Some(AslType::Double),
            _ => {
                // Check for string type: string255, string32, etc.
                if s.starts_with("string") {
                    let len_str = &s[6..];
                    if let Ok(len) = len_str.parse::<usize>() {
                        return Some(AslType::String(len));
                    }
                }
                None
            }
        }
    }

    fn size(&self) -> usize {
        match self {
            AslType::Bool | AslType::Byte | AslType::SByte => 1,
            AslType::Short | AslType::UShort => 2,
            AslType::Int | AslType::UInt | AslType::Float => 4,
            AslType::Long | AslType::ULong | AslType::Double => 8,
            AslType::String(len) => *len,
        }
    }
}

/// A state variable definition
#[derive(Debug, Clone)]
pub struct StateVariable {
    pub name: String,
    pub var_type: AslType,
    pub module: String,
    pub offsets: Vec<i64>,
}

/// Parsed ASL state block
#[derive(Debug, Clone)]
pub struct AslState {
    pub process_name: String,
    pub variables: Vec<StateVariable>,
}

/// Runtime value of a state variable
#[derive(Debug, Clone)]
pub enum AslValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
}

impl AslValue {
    pub fn as_bool(&self) -> bool {
        match self {
            AslValue::Bool(b) => *b,
            AslValue::Int(i) => *i != 0,
            AslValue::Float(f) => *f != 0.0,
            AslValue::String(s) => !s.is_empty(),
        }
    }

    pub fn as_int(&self) -> i64 {
        match self {
            AslValue::Bool(b) => if *b { 1 } else { 0 },
            AslValue::Int(i) => *i,
            AslValue::Float(f) => *f as i64,
            AslValue::String(_) => 0,
        }
    }

    pub fn as_float(&self) -> f64 {
        match self {
            AslValue::Bool(b) => if *b { 1.0 } else { 0.0 },
            AslValue::Int(i) => *i as f64,
            AslValue::Float(f) => *f,
            AslValue::String(_) => 0.0,
        }
    }
}

/// Parsed ASL action
#[derive(Debug, Clone)]
pub struct AslAction {
    pub name: String,
    pub code: String,
}

/// Parsed ASL script
#[derive(Debug, Clone)]
pub struct AslScript {
    pub states: Vec<AslState>,
    pub actions: HashMap<String, AslAction>,
}

/// ASL Parser
pub struct AslParser;

impl AslParser {
    /// Parse an ASL script
    pub fn parse(source: &str) -> Result<AslScript, AutosplitterError> {
        let mut states = Vec::new();
        let mut actions = HashMap::new();

        let mut chars = source.chars().peekable();
        let mut current_pos = 0;

        while let Some(&c) = chars.peek() {
            // Skip whitespace
            if c.is_whitespace() {
                chars.next();
                current_pos += 1;
                continue;
            }

            // Skip comments
            if c == '/' {
                chars.next();
                if let Some(&next) = chars.peek() {
                    if next == '/' {
                        // Single-line comment
                        while let Some(&ch) = chars.peek() {
                            chars.next();
                            if ch == '\n' {
                                break;
                            }
                        }
                        continue;
                    } else if next == '*' {
                        // Multi-line comment
                        chars.next();
                        let mut prev = ' ';
                        while let Some(&ch) = chars.peek() {
                            chars.next();
                            if prev == '*' && ch == '/' {
                                break;
                            }
                            prev = ch;
                        }
                        continue;
                    }
                }
            }

            // Read identifier
            if c.is_alphabetic() || c == '_' {
                let mut ident = String::new();
                while let Some(&ch) = chars.peek() {
                    if ch.is_alphanumeric() || ch == '_' {
                        ident.push(ch);
                        chars.next();
                    } else {
                        break;
                    }
                }

                // Skip whitespace after identifier
                while let Some(&ch) = chars.peek() {
                    if ch.is_whitespace() {
                        chars.next();
                    } else {
                        break;
                    }
                }

                if ident == "state" {
                    // Parse state block
                    if let Some(state) = Self::parse_state_block(&mut chars)? {
                        states.push(state);
                    }
                } else {
                    // Parse action block
                    if let Some(&ch) = chars.peek() {
                        if ch == '{' {
                            if let Some(action) = Self::parse_action_block(&ident, &mut chars)? {
                                actions.insert(action.name.clone(), action);
                            }
                        }
                    }
                }
            } else {
                chars.next();
            }
        }

        Ok(AslScript { states, actions })
    }

    fn parse_state_block(chars: &mut std::iter::Peekable<std::str::Chars>) -> Result<Option<AslState>, AutosplitterError> {
        // Expect ("process_name")
        Self::skip_whitespace(chars);

        if chars.peek() != Some(&'(') {
            return Ok(None);
        }
        chars.next();

        // Read process name
        Self::skip_whitespace(chars);
        let process_name = Self::read_string_literal(chars)?;

        Self::skip_whitespace(chars);
        if chars.peek() != Some(&')') {
            return Err(AutosplitterError::ScriptError("Expected ')' after process name".to_string()));
        }
        chars.next();

        // Expect {
        Self::skip_whitespace(chars);
        if chars.peek() != Some(&'{') {
            return Err(AutosplitterError::ScriptError("Expected '{' after state declaration".to_string()));
        }
        chars.next();

        let mut variables = Vec::new();

        // Parse variables until }
        loop {
            Self::skip_whitespace(chars);

            if let Some(&ch) = chars.peek() {
                if ch == '}' {
                    chars.next();
                    break;
                }

                // Parse variable: type name : "module", offset1, offset2, ...;
                if let Some(var) = Self::parse_variable(chars)? {
                    variables.push(var);
                }
            } else {
                break;
            }
        }

        Ok(Some(AslState {
            process_name,
            variables,
        }))
    }

    fn parse_variable(chars: &mut std::iter::Peekable<std::str::Chars>) -> Result<Option<StateVariable>, AutosplitterError> {
        // Read type
        let type_str = Self::read_identifier(chars);
        if type_str.is_empty() {
            return Ok(None);
        }

        let var_type = AslType::from_str(&type_str)
            .ok_or_else(|| AutosplitterError::ScriptError(format!("Unknown type: {}", type_str)))?;

        // Read name
        Self::skip_whitespace(chars);
        let name = Self::read_identifier(chars);
        if name.is_empty() {
            return Err(AutosplitterError::ScriptError("Expected variable name".to_string()));
        }

        // Expect :
        Self::skip_whitespace(chars);
        if chars.peek() != Some(&':') {
            return Err(AutosplitterError::ScriptError("Expected ':' after variable name".to_string()));
        }
        chars.next();

        // Read module name
        Self::skip_whitespace(chars);
        let module = Self::read_string_literal(chars)?;

        // Read offsets
        let mut offsets = Vec::new();
        loop {
            Self::skip_whitespace(chars);

            if let Some(&ch) = chars.peek() {
                if ch == ',' {
                    chars.next();
                    Self::skip_whitespace(chars);

                    // Read offset (hex or decimal)
                    let offset = Self::read_number(chars)?;
                    offsets.push(offset);
                } else if ch == ';' {
                    chars.next();
                    break;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        Ok(Some(StateVariable {
            name,
            var_type,
            module,
            offsets,
        }))
    }

    fn parse_action_block(name: &str, chars: &mut std::iter::Peekable<std::str::Chars>) -> Result<Option<AslAction>, AutosplitterError> {
        // Expect {
        if chars.peek() != Some(&'{') {
            return Ok(None);
        }
        chars.next();

        // Read until matching }
        let mut code = String::new();
        let mut brace_count = 1;

        while let Some(&ch) = chars.peek() {
            chars.next();
            if ch == '{' {
                brace_count += 1;
            } else if ch == '}' {
                brace_count -= 1;
                if brace_count == 0 {
                    break;
                }
            }
            code.push(ch);
        }

        Ok(Some(AslAction {
            name: name.to_string(),
            code: code.trim().to_string(),
        }))
    }

    fn skip_whitespace(chars: &mut std::iter::Peekable<std::str::Chars>) {
        while let Some(&ch) = chars.peek() {
            if ch.is_whitespace() {
                chars.next();
            } else {
                break;
            }
        }
    }

    fn read_identifier(chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
        let mut ident = String::new();
        while let Some(&ch) = chars.peek() {
            if ch.is_alphanumeric() || ch == '_' {
                ident.push(ch);
                chars.next();
            } else {
                break;
            }
        }
        ident
    }

    fn read_string_literal(chars: &mut std::iter::Peekable<std::str::Chars>) -> Result<String, AutosplitterError> {
        if chars.peek() != Some(&'"') {
            return Err(AutosplitterError::ScriptError("Expected string literal".to_string()));
        }
        chars.next();

        let mut s = String::new();
        while let Some(&ch) = chars.peek() {
            chars.next();
            if ch == '"' {
                break;
            }
            if ch == '\\' {
                if let Some(&escaped) = chars.peek() {
                    chars.next();
                    match escaped {
                        'n' => s.push('\n'),
                        't' => s.push('\t'),
                        'r' => s.push('\r'),
                        '\\' => s.push('\\'),
                        '"' => s.push('"'),
                        _ => s.push(escaped),
                    }
                }
            } else {
                s.push(ch);
            }
        }
        Ok(s)
    }

    fn read_number(chars: &mut std::iter::Peekable<std::str::Chars>) -> Result<i64, AutosplitterError> {
        let mut s = String::new();
        let mut is_hex = false;

        // Check for negative
        if chars.peek() == Some(&'-') {
            s.push('-');
            chars.next();
        }

        // Check for hex prefix
        if chars.peek() == Some(&'0') {
            s.push('0');
            chars.next();
            if let Some(&ch) = chars.peek() {
                if ch == 'x' || ch == 'X' {
                    s.push(ch);
                    chars.next();
                    is_hex = true;
                }
            }
        }

        // Read digits
        while let Some(&ch) = chars.peek() {
            if ch.is_ascii_hexdigit() {
                s.push(ch);
                chars.next();
            } else {
                break;
            }
        }

        if is_hex {
            let hex_str = s.trim_start_matches("0x").trim_start_matches("0X");
            i64::from_str_radix(hex_str, 16)
                .map_err(|_| AutosplitterError::ScriptError(format!("Invalid hex number: {}", s)))
        } else {
            s.parse::<i64>()
                .map_err(|_| AutosplitterError::ScriptError(format!("Invalid number: {}", s)))
        }
    }
}

/// ASL scripting engine
pub struct AslEngine {
    /// Parsed script
    script: AslScript,
    /// Current state values
    current: HashMap<String, AslValue>,
    /// Previous state values (from last tick)
    old: HashMap<String, AslValue>,
    /// Module base addresses
    module_bases: HashMap<String, usize>,
}

impl AslEngine {
    /// Create a new ASL engine from script source
    pub fn new(source: &str) -> Result<Self, AutosplitterError> {
        let script = AslParser::parse(source)?;

        log::info!(
            "Parsed ASL script: {} state blocks, {} actions",
            script.states.len(),
            script.actions.len()
        );

        Ok(Self {
            script,
            current: HashMap::new(),
            old: HashMap::new(),
            module_bases: HashMap::new(),
        })
    }

    /// Read a state variable value
    fn read_variable(&self, ctx: &EngineContext, var: &StateVariable) -> Option<AslValue> {
        // Get module base
        let base = self.module_bases.get(&var.module)?;

        // Follow pointer chain
        let mut addr = *base;
        for (i, &offset) in var.offsets.iter().enumerate() {
            if i < var.offsets.len() - 1 {
                // Dereference pointer
                addr = ctx.read_ptr(addr)?;
            }
            addr = if offset >= 0 {
                addr.wrapping_add(offset as usize)
            } else {
                addr.wrapping_sub((-offset) as usize)
            };
        }

        // Read value based on type
        match &var.var_type {
            AslType::Bool => ctx.read_bool(addr).map(AslValue::Bool),
            AslType::Byte => ctx.read_u8(addr).map(|v| AslValue::Int(v as i64)),
            AslType::SByte => ctx.read_u8(addr).map(|v| AslValue::Int(v as i8 as i64)),
            AslType::Short => ctx.read_u16(addr).map(|v| AslValue::Int(v as i16 as i64)),
            AslType::UShort => ctx.read_u16(addr).map(|v| AslValue::Int(v as i64)),
            AslType::Int => ctx.read_i32(addr).map(|v| AslValue::Int(v as i64)),
            AslType::UInt => ctx.read_u32(addr).map(|v| AslValue::Int(v as i64)),
            AslType::Long => ctx.read_i64(addr).map(AslValue::Int),
            AslType::ULong => ctx.read_u64(addr).map(|v| AslValue::Int(v as i64)),
            AslType::Float => ctx.read_f32(addr).map(|v| AslValue::Float(v as f64)),
            AslType::Double => ctx.read_f64(addr).map(AslValue::Float),
            AslType::String(len) => {
                if let Some(buf) = ctx.read_bytes(addr, *len) {
                    let s = buf.iter()
                        .take_while(|&&b| b != 0)
                        .map(|&b| b as char)
                        .collect();
                    Some(AslValue::String(s))
                } else {
                    None
                }
            }
        }
    }

    /// Update all state variables
    fn update_state(&mut self, ctx: &EngineContext) {
        // Save current to old
        self.old = self.current.clone();

        // Read new current values
        for state in &self.script.states {
            for var in &state.variables {
                if let Some(value) = self.read_variable(ctx, var) {
                    self.current.insert(var.name.clone(), value);
                }
            }
        }
    }

    /// Evaluate an action's return expression
    /// This is a simplified evaluator that handles common ASL patterns
    fn evaluate_action(&self, action_name: &str) -> bool {
        let action = match self.script.actions.get(action_name) {
            Some(a) => a,
            None => return false,
        };

        // Simple expression evaluator for common patterns:
        // - return true/false
        // - return current.var == old.var
        // - return current.var > old.var
        // - return current.var && !old.var

        let code = action.code.trim();

        // Handle "return true/false"
        if code == "return true;" || code == "return true" {
            return true;
        }
        if code == "return false;" || code == "return false" {
            return false;
        }

        // Try to evaluate simple expressions
        if code.starts_with("return ") {
            let expr = code.trim_start_matches("return ").trim_end_matches(';').trim();
            return self.evaluate_expression(expr);
        }

        false
    }

    /// Evaluate a simple expression
    fn evaluate_expression(&self, expr: &str) -> bool {
        let expr = expr.trim();

        // Handle boolean literals
        if expr == "true" {
            return true;
        }
        if expr == "false" {
            return false;
        }

        // Handle negation
        if expr.starts_with('!') {
            return !self.evaluate_expression(&expr[1..]);
        }

        // Handle parentheses
        if expr.starts_with('(') && expr.ends_with(')') {
            return self.evaluate_expression(&expr[1..expr.len()-1]);
        }

        // Handle && operator
        if let Some(pos) = expr.find("&&") {
            let left = &expr[..pos];
            let right = &expr[pos+2..];
            return self.evaluate_expression(left) && self.evaluate_expression(right);
        }

        // Handle || operator
        if let Some(pos) = expr.find("||") {
            let left = &expr[..pos];
            let right = &expr[pos+2..];
            return self.evaluate_expression(left) || self.evaluate_expression(right);
        }

        // Handle comparison operators
        for op in &["==", "!=", ">=", "<=", ">", "<"] {
            if let Some(pos) = expr.find(op) {
                let left = expr[..pos].trim();
                let right = expr[pos+op.len()..].trim();
                return self.evaluate_comparison(left, *op, right);
            }
        }

        // Handle simple variable access (current.var or old.var)
        if let Some(value) = self.get_value(expr) {
            return value.as_bool();
        }

        false
    }

    /// Evaluate a comparison
    fn evaluate_comparison(&self, left: &str, op: &str, right: &str) -> bool {
        let left_val = self.get_value(left);
        let right_val = self.get_value(right);

        match (left_val, right_val) {
            (Some(l), Some(r)) => {
                let l_int = l.as_int();
                let r_int = r.as_int();
                match op {
                    "==" => l_int == r_int,
                    "!=" => l_int != r_int,
                    ">" => l_int > r_int,
                    ">=" => l_int >= r_int,
                    "<" => l_int < r_int,
                    "<=" => l_int <= r_int,
                    _ => false,
                }
            }
            _ => false,
        }
    }

    /// Get a value from an expression (current.var, old.var, or literal)
    fn get_value(&self, expr: &str) -> Option<AslValue> {
        let expr = expr.trim();

        // current.var
        if expr.starts_with("current.") {
            let var_name = &expr[8..];
            return self.current.get(var_name).cloned();
        }

        // old.var
        if expr.starts_with("old.") {
            let var_name = &expr[4..];
            return self.old.get(var_name).cloned();
        }

        // Try to parse as number
        if let Ok(n) = expr.parse::<i64>() {
            return Some(AslValue::Int(n));
        }

        // Try to parse as float
        if let Ok(f) = expr.parse::<f64>() {
            return Some(AslValue::Float(f));
        }

        None
    }
}

impl Engine for AslEngine {
    fn engine_type(&self) -> EngineType {
        EngineType::Asl
    }

    fn init(&mut self, ctx: &mut EngineContext) -> Result<(), AutosplitterError> {
        // For ASL, we use the main module as the base for all variables
        // In a full implementation, we'd resolve each module name
        for state in &self.script.states {
            // Use the process name as the module base
            self.module_bases.insert(state.process_name.clone(), ctx.base_address());
        }

        log::info!("AslEngine initialized with {} state blocks", self.script.states.len());
        Ok(())
    }

    fn read_flag(&self, _ctx: &EngineContext, _flag_id: u32) -> Result<bool, AutosplitterError> {
        // ASL doesn't use flag IDs directly - it uses state variables
        // For compatibility, we could map flag_id to a variable if configured
        Ok(false)
    }

    fn update(&mut self, ctx: &mut EngineContext) -> Result<(), AutosplitterError> {
        self.update_state(ctx);

        // Call update action if defined
        if self.script.actions.contains_key("update") {
            // Update action is for side effects, return value ignored
            let _ = self.evaluate_action("update");
        }

        Ok(())
    }

    fn should_split(&self, _ctx: &EngineContext) -> Result<bool, AutosplitterError> {
        Ok(self.evaluate_action("split"))
    }

    fn should_start(&self, _ctx: &EngineContext) -> Result<bool, AutosplitterError> {
        Ok(self.evaluate_action("start"))
    }

    fn should_reset(&self, _ctx: &EngineContext) -> Result<bool, AutosplitterError> {
        Ok(self.evaluate_action("reset"))
    }

    fn is_loading(&self, _ctx: &EngineContext) -> Result<Option<bool>, AutosplitterError> {
        if self.script.actions.contains_key("isLoading") {
            Ok(Some(self.evaluate_action("isLoading")))
        } else {
            Ok(None)
        }
    }

    fn get_igt_milliseconds(&self, _ctx: &EngineContext) -> Result<Option<i32>, AutosplitterError> {
        // gameTime action returns TimeSpan, which we'd need to parse
        // For now, return None - this would need more complex expression evaluation
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asl_type_parsing() {
        assert_eq!(AslType::from_str("int"), Some(AslType::Int));
        assert_eq!(AslType::from_str("bool"), Some(AslType::Bool));
        assert_eq!(AslType::from_str("float"), Some(AslType::Float));
        assert_eq!(AslType::from_str("string255"), Some(AslType::String(255)));
    }

    #[test]
    fn test_asl_parser_basic() {
        let script = r#"
            state("Game.exe") {
                int health : "Game.exe", 0x12345, 0x10;
                bool isLoading : "Game.exe", 0x12345, 0x20;
            }

            start {
                return current.health > 0 && !old.isLoading;
            }

            split {
                return current.health == 0 && old.health > 0;
            }
        "#;

        let parsed = AslParser::parse(script).unwrap();
        assert_eq!(parsed.states.len(), 1);
        assert_eq!(parsed.states[0].variables.len(), 2);
        assert!(parsed.actions.contains_key("start"));
        assert!(parsed.actions.contains_key("split"));
    }
}
