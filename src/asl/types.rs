//! ASL value types and variable storage

use std::collections::HashMap;

/// Runtime value types for ASL
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    UInt(u64),
    Float(f64),
    String(String),
    ByteArray(Vec<u8>),
}

impl Default for Value {
    fn default() -> Self {
        Value::Null
    }
}

impl Value {
    /// Convert to boolean (for conditions)
    pub fn as_bool(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Bool(b) => *b,
            Value::Int(i) => *i != 0,
            Value::UInt(u) => *u != 0,
            Value::Float(f) => *f != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::ByteArray(b) => !b.is_empty(),
        }
    }

    /// Convert to i64
    pub fn as_int(&self) -> i64 {
        match self {
            Value::Null => 0,
            Value::Bool(b) => if *b { 1 } else { 0 },
            Value::Int(i) => *i,
            Value::UInt(u) => *u as i64,
            Value::Float(f) => *f as i64,
            Value::String(s) => s.parse().unwrap_or(0),
            Value::ByteArray(_) => 0,
        }
    }

    /// Convert to u64
    pub fn as_uint(&self) -> u64 {
        match self {
            Value::Null => 0,
            Value::Bool(b) => if *b { 1 } else { 0 },
            Value::Int(i) => *i as u64,
            Value::UInt(u) => *u,
            Value::Float(f) => *f as u64,
            Value::String(s) => s.parse().unwrap_or(0),
            Value::ByteArray(_) => 0,
        }
    }

    /// Convert to f64
    pub fn as_float(&self) -> f64 {
        match self {
            Value::Null => 0.0,
            Value::Bool(b) => if *b { 1.0 } else { 0.0 },
            Value::Int(i) => *i as f64,
            Value::UInt(u) => *u as f64,
            Value::Float(f) => *f,
            Value::String(s) => s.parse().unwrap_or(0.0),
            Value::ByteArray(_) => 0.0,
        }
    }

    /// Convert to string
    pub fn as_string(&self) -> String {
        match self {
            Value::Null => String::new(),
            Value::Bool(b) => b.to_string(),
            Value::Int(i) => i.to_string(),
            Value::UInt(u) => u.to_string(),
            Value::Float(f) => f.to_string(),
            Value::String(s) => s.clone(),
            Value::ByteArray(b) => format!("{:?}", b),
        }
    }

    /// Check if values are equal
    pub fn equals(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::UInt(a), Value::UInt(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => (a - b).abs() < f64::EPSILON,
            (Value::String(a), Value::String(b)) => a == b,
            // Cross-type numeric comparisons
            (Value::Int(a), Value::UInt(b)) => *a >= 0 && *a as u64 == *b,
            (Value::UInt(a), Value::Int(b)) => *b >= 0 && *a == *b as u64,
            (Value::Int(a), Value::Float(b)) => (*a as f64 - b).abs() < f64::EPSILON,
            (Value::Float(a), Value::Int(b)) => (a - *b as f64).abs() < f64::EPSILON,
            (Value::UInt(a), Value::Float(b)) => (*a as f64 - b).abs() < f64::EPSILON,
            (Value::Float(a), Value::UInt(b)) => (a - *b as f64).abs() < f64::EPSILON,
            _ => false,
        }
    }

    /// Compare values (returns -1, 0, or 1)
    pub fn compare(&self, other: &Value) -> i32 {
        let a = self.as_float();
        let b = other.as_float();
        if (a - b).abs() < f64::EPSILON {
            0
        } else if a < b {
            -1
        } else {
            1
        }
    }
}

/// Variable type for state definitions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarType {
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
    String,
    ByteArray,
}

impl VarType {
    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "bool" | "boolean" => Some(VarType::Bool),
            "byte" | "uint8" => Some(VarType::Byte),
            "sbyte" | "int8" => Some(VarType::SByte),
            "short" | "int16" => Some(VarType::Short),
            "ushort" | "uint16" => Some(VarType::UShort),
            "int" | "int32" => Some(VarType::Int),
            "uint" | "uint32" => Some(VarType::UInt),
            "long" | "int64" => Some(VarType::Long),
            "ulong" | "uint64" => Some(VarType::ULong),
            "float" | "float32" => Some(VarType::Float),
            "double" | "float64" => Some(VarType::Double),
            "string" => Some(VarType::String),
            "byte[]" | "bytearray" => Some(VarType::ByteArray),
            _ => None,
        }
    }

    /// Size in bytes
    pub fn size(&self) -> usize {
        match self {
            VarType::Bool | VarType::Byte | VarType::SByte => 1,
            VarType::Short | VarType::UShort => 2,
            VarType::Int | VarType::UInt | VarType::Float => 4,
            VarType::Long | VarType::ULong | VarType::Double => 8,
            VarType::String | VarType::ByteArray => 0, // Variable size
        }
    }
}

/// Variable definition from state block
#[derive(Debug, Clone)]
pub struct VarDefinition {
    pub name: String,
    pub var_type: VarType,
    pub module: Option<String>,
    pub offsets: Vec<i64>,
    pub string_length: Option<usize>,
}

/// Variable store with current/old state tracking
#[derive(Debug, Clone, Default)]
pub struct VariableStore {
    /// Current values
    pub current: HashMap<String, Value>,
    /// Previous tick values
    pub old: HashMap<String, Value>,
    /// Variable definitions
    pub definitions: Vec<VarDefinition>,
}

impl VariableStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a variable definition
    pub fn add_definition(&mut self, def: VarDefinition) {
        self.definitions.push(def);
    }

    /// Move current values to old, preparing for new tick
    pub fn tick(&mut self) {
        self.old = self.current.clone();
    }

    /// Set a current value
    pub fn set(&mut self, name: &str, value: Value) {
        self.current.insert(name.to_string(), value);
    }

    /// Get current value
    pub fn get_current(&self, name: &str) -> Value {
        self.current.get(name).cloned().unwrap_or(Value::Null)
    }

    /// Get old value
    pub fn get_old(&self, name: &str) -> Value {
        self.old.get(name).cloned().unwrap_or(Value::Null)
    }

    /// Clear all values
    pub fn clear(&mut self) {
        self.current.clear();
        self.old.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_conversions() {
        assert_eq!(Value::Int(42).as_bool(), true);
        assert_eq!(Value::Int(0).as_bool(), false);
        assert_eq!(Value::Float(3.14).as_int(), 3);
        assert_eq!(Value::String("hello".to_string()).as_bool(), true);
        assert_eq!(Value::String("".to_string()).as_bool(), false);
    }

    #[test]
    fn test_value_equality() {
        assert!(Value::Int(42).equals(&Value::Int(42)));
        assert!(Value::Int(42).equals(&Value::UInt(42)));
        assert!(Value::Float(42.0).equals(&Value::Int(42)));
        assert!(!Value::Int(42).equals(&Value::Int(43)));
    }

    #[test]
    fn test_variable_store() {
        let mut store = VariableStore::new();
        store.set("health", Value::Int(100));
        store.tick();
        store.set("health", Value::Int(80));

        assert_eq!(store.get_current("health"), Value::Int(80));
        assert_eq!(store.get_old("health"), Value::Int(100));
    }
}
