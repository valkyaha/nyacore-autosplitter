//! ASL (Auto Splitter Language) Parser and Runtime
//!
//! This module provides LiveSplit-compatible ASL parsing and execution.
//! It allows defining autosplitters using a simple scripting language.
//!
//! # Example ASL
//!
//! ```asl
//! state("game.exe") {
//!     int health : 0x12345678;
//!     float time : "base.dll", 0x100, 0x20;
//! }
//!
//! start {
//!     return current.health > 0 && old.health == 0;
//! }
//!
//! split {
//!     return current.level > old.level;
//! }
//! ```

mod ast;
mod lexer;
mod memory;
mod parser;
mod runtime;
mod types;

pub use ast::*;
pub use lexer::{Lexer, Token, TokenKind};
pub use memory::AslMemoryContext;
pub use parser::Parser;
pub use runtime::{AslRuntime, AutosplitEvents};
pub use types::{Value, VarType, VarDefinition, VariableStore};

/// Parse an ASL script and create a runtime
pub fn parse_asl(source: &str) -> Result<AslRuntime, ParseError> {
    let lexer = Lexer::new(source);
    let mut parser = Parser::new(lexer);
    let script = parser.parse()?;
    Ok(AslRuntime::new(script))
}

/// Error type for ASL parsing
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Parse error at {}:{}: {}", self.line, self.column, self.message)
    }
}

impl std::error::Error for ParseError {}

impl ParseError {
    pub fn new(message: impl Into<String>, line: usize, column: usize) -> Self {
        Self {
            message: message.into(),
            line,
            column,
        }
    }
}
