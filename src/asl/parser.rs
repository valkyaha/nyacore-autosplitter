//! ASL Parser - Parses tokens into an AST
//!
//! This parser converts the token stream from the lexer into an Abstract Syntax Tree
//! that can be converted to GameData.

use super::error::{AslError, AslResult};
use super::lexer::{Token, TokenKind};

/// Parsed ASL script
#[derive(Debug, Clone)]
pub struct AslScript {
    /// Process name from state() block
    pub process_name: String,
    /// Variable definitions from state() block
    pub variables: Vec<AslVariable>,
    /// startup block contents
    pub startup: Option<AslBlock>,
    /// init block contents
    pub init: Option<AslBlock>,
    /// split block contents
    pub split: Option<AslBlock>,
    /// reset block contents
    pub reset: Option<AslBlock>,
    /// isLoading block contents
    pub is_loading: Option<AslBlock>,
}

/// Variable definition from state() block
#[derive(Debug, Clone)]
pub struct AslVariable {
    /// Variable type
    pub var_type: AslType,
    /// Variable name
    pub name: String,
    /// Pattern/pointer name to reference
    pub pointer_name: String,
    /// Offset chain - can be a single flag_id or multiple offsets
    pub offsets: Vec<i64>,
}

/// Variable type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AslType {
    Bool,
    Int,
    Byte,
    Short,
    Long,
    UInt,
    UShort,
    ULong,
    Float,
    String,
}

impl AslType {
    /// Get the size in bytes
    pub fn size(&self) -> usize {
        match self {
            AslType::Bool | AslType::Byte => 1,
            AslType::Short | AslType::UShort => 2,
            AslType::Int | AslType::UInt | AslType::Float => 4,
            AslType::Long | AslType::ULong => 8,
            AslType::String => 0, // Variable size
        }
    }
}

/// Block of statements (split, reset, isLoading, etc.)
#[derive(Debug, Clone)]
pub struct AslBlock {
    pub statements: Vec<AslStatement>,
}

/// Statement in a block
#[derive(Debug, Clone)]
pub enum AslStatement {
    /// if (condition) { statements }
    If {
        condition: AslCondition,
        body: Vec<AslStatement>,
    },
    /// return true; or return false;
    Return(bool),
    /// Unrecognized statement (stored as raw text for future use)
    Unknown(String),
}

/// Condition in an if statement
#[derive(Debug, Clone)]
pub struct AslCondition {
    /// Left side of comparison
    pub left: AslExpression,
    /// Comparison operator (None if just evaluating truthiness)
    pub op: Option<CompareOp>,
    /// Right side of comparison (None if just evaluating truthiness)
    pub right: Option<AslExpression>,
    /// Logical combinator with next condition
    pub combinator: Option<LogicalOp>,
    /// Next condition in chain (for && and ||)
    pub next: Option<Box<AslCondition>>,
}

/// Comparison operator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareOp {
    Equals,
    NotEquals,
    Greater,
    Less,
    GreaterEq,
    LessEq,
}

/// Logical operator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalOp {
    And,
    Or,
}

/// Expression in a condition
#[derive(Debug, Clone)]
pub enum AslExpression {
    /// current.varName
    CurrentVar(String),
    /// old.varName
    OldVar(String),
    /// !expression
    Not(Box<AslExpression>),
    /// true
    True,
    /// false
    False,
    /// Integer literal
    IntLiteral(i64),
    /// Hex literal
    HexLiteral(u64),
    /// Float literal
    FloatLiteral(f64),
    /// Plain identifier
    Identifier(String),
}

/// ASL Parser
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    /// Create a new parser with the given tokens
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    /// Parse the token stream into an ASL script
    pub fn parse(&mut self) -> AslResult<AslScript> {
        let mut script = AslScript {
            process_name: String::new(),
            variables: Vec::new(),
            startup: None,
            init: None,
            split: None,
            reset: None,
            is_loading: None,
        };

        while !self.is_at_end() {
            match self.current_kind() {
                TokenKind::State => {
                    let (process_name, variables) = self.parse_state_block()?;
                    script.process_name = process_name;
                    script.variables = variables;
                }
                TokenKind::Startup => {
                    script.startup = Some(self.parse_action_block("startup")?);
                }
                TokenKind::Init => {
                    script.init = Some(self.parse_action_block("init")?);
                }
                TokenKind::Split => {
                    script.split = Some(self.parse_action_block("split")?);
                }
                TokenKind::Reset => {
                    script.reset = Some(self.parse_action_block("reset")?);
                }
                TokenKind::IsLoading => {
                    script.is_loading = Some(self.parse_action_block("isLoading")?);
                }
                TokenKind::Eof => break,
                _ => {
                    // Skip unknown top-level tokens
                    self.advance();
                }
            }
        }

        if script.process_name.is_empty() {
            return Err(AslError::parser("No state() block found"));
        }

        Ok(script)
    }

    /// Parse a state("process.exe") { ... } block
    fn parse_state_block(&mut self) -> AslResult<(String, Vec<AslVariable>)> {
        self.expect(TokenKind::State)?;
        self.expect(TokenKind::LeftParen)?;

        let process_name = self.expect_string_literal()?;

        self.expect(TokenKind::RightParen)?;
        self.expect(TokenKind::LeftBrace)?;

        let mut variables = Vec::new();

        while !self.check(TokenKind::RightBrace) && !self.is_at_end() {
            if let Some(var) = self.parse_variable_definition()? {
                variables.push(var);
            }
        }

        self.expect(TokenKind::RightBrace)?;

        Ok((process_name, variables))
    }

    /// Parse a variable definition: type name : "pointer", offset1, offset2, ...;
    fn parse_variable_definition(&mut self) -> AslResult<Option<AslVariable>> {
        // Parse type
        let var_type = match self.current_kind() {
            TokenKind::Bool => AslType::Bool,
            TokenKind::Int => AslType::Int,
            TokenKind::Byte => AslType::Byte,
            TokenKind::Short => AslType::Short,
            TokenKind::Long => AslType::Long,
            TokenKind::UInt => AslType::UInt,
            TokenKind::UShort => AslType::UShort,
            TokenKind::ULong => AslType::ULong,
            TokenKind::Float => AslType::Float,
            TokenKind::String => AslType::String,
            _ => {
                // Skip non-variable tokens (comments, empty lines parsed as tokens, etc.)
                self.advance();
                return Ok(None);
            }
        };
        self.advance();

        // Parse name
        let name = self.expect_identifier()?;

        self.expect(TokenKind::Colon)?;

        // Parse pointer name (string literal)
        let pointer_name = self.expect_string_literal()?;

        // Parse offsets
        let mut offsets = Vec::new();

        while self.check(TokenKind::Comma) {
            self.advance(); // consume comma

            // Parse offset (can be decimal or hex)
            let offset = match self.current_kind() {
                TokenKind::NumberLiteral(n) => {
                    let val = n;
                    self.advance();
                    val
                }
                TokenKind::HexLiteral(n) => {
                    let val = n as i64;
                    self.advance();
                    val
                }
                _ => {
                    return Err(AslError::parser_at(
                        "Expected offset value",
                        self.current_line(),
                        self.current_column(),
                    ))
                }
            };
            offsets.push(offset);
        }

        self.expect(TokenKind::Semicolon)?;

        Ok(Some(AslVariable {
            var_type,
            name,
            pointer_name,
            offsets,
        }))
    }

    /// Parse an action block (split, reset, isLoading, startup, init)
    fn parse_action_block(&mut self, block_name: &str) -> AslResult<AslBlock> {
        self.advance(); // consume block keyword
        self.expect(TokenKind::LeftBrace)?;

        let mut statements = Vec::new();

        while !self.check(TokenKind::RightBrace) && !self.is_at_end() {
            if let Some(stmt) = self.parse_statement()? {
                statements.push(stmt);
            }
        }

        if !self.check(TokenKind::RightBrace) {
            return Err(AslError::parser(format!(
                "Unterminated {} block",
                block_name
            )));
        }
        self.advance(); // consume '}'

        Ok(AslBlock { statements })
    }

    /// Parse a statement
    fn parse_statement(&mut self) -> AslResult<Option<AslStatement>> {
        match self.current_kind() {
            TokenKind::If => {
                let stmt = self.parse_if_statement()?;
                Ok(Some(stmt))
            }
            TokenKind::Return => {
                let stmt = self.parse_return_statement()?;
                Ok(Some(stmt))
            }
            TokenKind::RightBrace => {
                // End of block
                Ok(None)
            }
            _ => {
                // Skip unknown tokens until we hit something meaningful
                self.advance();
                Ok(None)
            }
        }
    }

    /// Parse an if statement
    fn parse_if_statement(&mut self) -> AslResult<AslStatement> {
        self.expect(TokenKind::If)?;
        self.expect(TokenKind::LeftParen)?;

        let condition = self.parse_condition()?;

        self.expect(TokenKind::RightParen)?;
        self.expect(TokenKind::LeftBrace)?;

        let mut body = Vec::new();
        while !self.check(TokenKind::RightBrace) && !self.is_at_end() {
            if let Some(stmt) = self.parse_statement()? {
                body.push(stmt);
            }
        }

        self.expect(TokenKind::RightBrace)?;

        Ok(AslStatement::If { condition, body })
    }

    /// Parse a return statement
    fn parse_return_statement(&mut self) -> AslResult<AslStatement> {
        self.expect(TokenKind::Return)?;

        let value = match self.current_kind() {
            TokenKind::True => {
                self.advance();
                true
            }
            TokenKind::False => {
                self.advance();
                false
            }
            _ => {
                return Err(AslError::parser_at(
                    "Expected true or false after return",
                    self.current_line(),
                    self.current_column(),
                ))
            }
        };

        self.expect(TokenKind::Semicolon)?;

        Ok(AslStatement::Return(value))
    }

    /// Parse a condition
    fn parse_condition(&mut self) -> AslResult<AslCondition> {
        let left = self.parse_expression()?;

        // Check for comparison operator
        let (op, right) = match self.current_kind() {
            TokenKind::Equals => {
                self.advance();
                let right = self.parse_expression()?;
                (Some(CompareOp::Equals), Some(right))
            }
            TokenKind::NotEquals => {
                self.advance();
                let right = self.parse_expression()?;
                (Some(CompareOp::NotEquals), Some(right))
            }
            TokenKind::Greater => {
                self.advance();
                let right = self.parse_expression()?;
                (Some(CompareOp::Greater), Some(right))
            }
            TokenKind::Less => {
                self.advance();
                let right = self.parse_expression()?;
                (Some(CompareOp::Less), Some(right))
            }
            TokenKind::GreaterEq => {
                self.advance();
                let right = self.parse_expression()?;
                (Some(CompareOp::GreaterEq), Some(right))
            }
            TokenKind::LessEq => {
                self.advance();
                let right = self.parse_expression()?;
                (Some(CompareOp::LessEq), Some(right))
            }
            _ => (None, None),
        };

        // Check for logical combinator
        let (combinator, next) = match self.current_kind() {
            TokenKind::And => {
                self.advance();
                let next = self.parse_condition()?;
                (Some(LogicalOp::And), Some(Box::new(next)))
            }
            TokenKind::Or => {
                self.advance();
                let next = self.parse_condition()?;
                (Some(LogicalOp::Or), Some(Box::new(next)))
            }
            _ => (None, None),
        };

        Ok(AslCondition {
            left,
            op,
            right,
            combinator,
            next,
        })
    }

    /// Parse an expression
    fn parse_expression(&mut self) -> AslResult<AslExpression> {
        // Handle NOT prefix
        if self.check(TokenKind::Not) {
            self.advance();
            let expr = self.parse_expression()?;
            return Ok(AslExpression::Not(Box::new(expr)));
        }

        // Handle parenthesized expressions (for grouped conditions)
        if self.check(TokenKind::LeftParen) {
            self.advance();
            let expr = self.parse_expression()?;

            // Check for comparison after the expression
            let result = match self.current_kind() {
                TokenKind::And | TokenKind::Or | TokenKind::RightParen => expr,
                _ => expr, // Just return the expression
            };

            if self.check(TokenKind::RightParen) {
                self.advance();
            }

            return Ok(result);
        }

        match self.current_kind() {
            TokenKind::Current => {
                self.advance();
                self.expect(TokenKind::Dot)?;
                let var_name = self.expect_identifier()?;
                Ok(AslExpression::CurrentVar(var_name))
            }
            TokenKind::Old => {
                self.advance();
                self.expect(TokenKind::Dot)?;
                let var_name = self.expect_identifier()?;
                Ok(AslExpression::OldVar(var_name))
            }
            TokenKind::True => {
                self.advance();
                Ok(AslExpression::True)
            }
            TokenKind::False => {
                self.advance();
                Ok(AslExpression::False)
            }
            TokenKind::NumberLiteral(n) => {
                let val = n;
                self.advance();
                Ok(AslExpression::IntLiteral(val))
            }
            TokenKind::HexLiteral(n) => {
                let val = n;
                self.advance();
                Ok(AslExpression::HexLiteral(val))
            }
            TokenKind::FloatLiteral(n) => {
                let val = n;
                self.advance();
                Ok(AslExpression::FloatLiteral(val))
            }
            TokenKind::Identifier(ref name) => {
                let name = name.clone();
                self.advance();
                Ok(AslExpression::Identifier(name))
            }
            _ => Err(AslError::parser_at(
                format!("Unexpected token in expression: {:?}", self.current_kind()),
                self.current_line(),
                self.current_column(),
            )),
        }
    }

    // Helper methods

    fn is_at_end(&self) -> bool {
        self.pos >= self.tokens.len() || self.current_kind() == TokenKind::Eof
    }

    fn current(&self) -> &Token {
        &self.tokens[self.pos.min(self.tokens.len() - 1)]
    }

    fn current_kind(&self) -> TokenKind {
        self.current().kind.clone()
    }

    fn current_line(&self) -> usize {
        self.current().line
    }

    fn current_column(&self) -> usize {
        self.current().column
    }

    fn advance(&mut self) {
        if !self.is_at_end() {
            self.pos += 1;
        }
    }

    fn check(&self, kind: TokenKind) -> bool {
        self.current_kind() == kind
    }

    fn expect(&mut self, kind: TokenKind) -> AslResult<()> {
        if self.check(kind.clone()) {
            self.advance();
            Ok(())
        } else {
            Err(AslError::parser_at(
                format!("Expected {:?}, got {:?}", kind, self.current_kind()),
                self.current_line(),
                self.current_column(),
            ))
        }
    }

    fn expect_identifier(&mut self) -> AslResult<String> {
        if let TokenKind::Identifier(name) = self.current_kind() {
            self.advance();
            Ok(name)
        } else {
            Err(AslError::parser_at(
                format!("Expected identifier, got {:?}", self.current_kind()),
                self.current_line(),
                self.current_column(),
            ))
        }
    }

    fn expect_string_literal(&mut self) -> AslResult<String> {
        if let TokenKind::StringLiteral(value) = self.current_kind() {
            self.advance();
            Ok(value)
        } else {
            Err(AslError::parser_at(
                format!("Expected string literal, got {:?}", self.current_kind()),
                self.current_line(),
                self.current_column(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asl::lexer::Lexer;

    fn parse(input: &str) -> AslResult<AslScript> {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(tokens);
        parser.parse()
    }

    #[test]
    fn test_parse_state_block() {
        let input = r#"
state("DarkSoulsIII.exe") {
    bool testBoss : "pointer", 12345;
}
"#;
        let script = parse(input).unwrap();

        assert_eq!(script.process_name, "DarkSoulsIII.exe");
        assert_eq!(script.variables.len(), 1);
        assert_eq!(script.variables[0].name, "testBoss");
        assert_eq!(script.variables[0].var_type, AslType::Bool);
        assert_eq!(script.variables[0].pointer_name, "pointer");
        assert_eq!(script.variables[0].offsets, vec![12345]);
    }

    #[test]
    fn test_parse_hex_offsets() {
        let input = r#"
state("DarkSoulsII.exe") {
    int boss : "pattern", 0x0, 0x70, 0x28, 0x20;
}
"#;
        let script = parse(input).unwrap();

        assert_eq!(script.variables[0].offsets, vec![0x0, 0x70, 0x28, 0x20]);
    }

    #[test]
    fn test_parse_multiple_variables() {
        let input = r#"
state("game.exe") {
    bool var1 : "ptr1", 100;
    int var2 : "ptr2", 200;
    byte var3 : "ptr3", 300;
}
"#;
        let script = parse(input).unwrap();

        assert_eq!(script.variables.len(), 3);
        assert_eq!(script.variables[0].name, "var1");
        assert_eq!(script.variables[0].var_type, AslType::Bool);
        assert_eq!(script.variables[1].name, "var2");
        assert_eq!(script.variables[1].var_type, AslType::Int);
        assert_eq!(script.variables[2].name, "var3");
        assert_eq!(script.variables[2].var_type, AslType::Byte);
    }

    #[test]
    fn test_parse_split_block() {
        let input = r#"
state("game.exe") {
    bool boss : "ptr", 100;
}

split {
    if (current.boss && !old.boss) { return true; }
    return false;
}
"#;
        let script = parse(input).unwrap();

        assert!(script.split.is_some());
        let split = script.split.unwrap();
        assert_eq!(split.statements.len(), 2);

        // First statement is if
        if let AslStatement::If { condition, body } = &split.statements[0] {
            // Check condition has current.boss
            if let AslExpression::CurrentVar(name) = &condition.left {
                assert_eq!(name, "boss");
            } else {
                panic!("Expected CurrentVar");
            }

            // Check body has return true
            assert_eq!(body.len(), 1);
            if let AslStatement::Return(true) = &body[0] {
                // OK
            } else {
                panic!("Expected return true");
            }
        } else {
            panic!("Expected If statement");
        }

        // Second statement is return false
        if let AslStatement::Return(false) = &split.statements[1] {
            // OK
        } else {
            panic!("Expected return false");
        }
    }

    #[test]
    fn test_parse_reset_block() {
        let input = r#"
state("game.exe") {
    bool flag : "ptr", 100;
}

reset {
    return false;
}
"#;
        let script = parse(input).unwrap();

        assert!(script.reset.is_some());
        let reset = script.reset.unwrap();
        assert_eq!(reset.statements.len(), 1);

        if let AslStatement::Return(false) = &reset.statements[0] {
            // OK
        } else {
            panic!("Expected return false");
        }
    }

    #[test]
    fn test_parse_is_loading_block() {
        let input = r#"
state("game.exe") {
    bool loading : "ptr", 100;
}

isLoading {
    return false;
}
"#;
        let script = parse(input).unwrap();

        assert!(script.is_loading.is_some());
    }

    #[test]
    fn test_parse_comparison_operators() {
        let input = r#"
state("game.exe") {
    int count : "ptr", 100;
}

split {
    if (current.count > 0 && old.count == 0) { return true; }
    return false;
}
"#;
        let script = parse(input).unwrap();

        let split = script.split.unwrap();
        if let AslStatement::If { condition, .. } = &split.statements[0] {
            assert_eq!(condition.op, Some(CompareOp::Greater));
            assert_eq!(condition.combinator, Some(LogicalOp::And));

            // Check next condition
            if let Some(next) = &condition.next {
                assert_eq!(next.op, Some(CompareOp::Equals));
            } else {
                panic!("Expected next condition");
            }
        } else {
            panic!("Expected If statement");
        }
    }

    #[test]
    fn test_parse_full_ds3_style() {
        let input = r#"
state("DarkSoulsIII.exe") {
    bool iudexGundyr : "sprj_event_flag_man", 13000050;
    bool vordt : "sprj_event_flag_man", 13000800;
}

startup {
}

init {
}

split {
    if (current.iudexGundyr && !old.iudexGundyr) { return true; }
    if (current.vordt && !old.vordt) { return true; }
    return false;
}

reset {
    return false;
}

isLoading {
    return false;
}
"#;
        let script = parse(input).unwrap();

        assert_eq!(script.process_name, "DarkSoulsIII.exe");
        assert_eq!(script.variables.len(), 2);
        assert!(script.startup.is_some());
        assert!(script.init.is_some());
        assert!(script.split.is_some());
        assert!(script.reset.is_some());
        assert!(script.is_loading.is_some());

        let split = script.split.unwrap();
        assert_eq!(split.statements.len(), 3); // 2 if statements + 1 return
    }

    #[test]
    fn test_parse_comments() {
        let input = r#"
// This is a comment
state("game.exe") {
    // Variable comment
    bool flag : "ptr", 100;
}

/* Block comment */
split {
    return false;
}
"#;
        let script = parse(input).unwrap();

        assert_eq!(script.variables.len(), 1);
        assert!(script.split.is_some());
    }

    #[test]
    fn test_error_missing_state() {
        let input = r#"
split {
    return false;
}
"#;
        let result = parse(input);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("No state() block"));
    }

    #[test]
    fn test_asl_type_size() {
        assert_eq!(AslType::Bool.size(), 1);
        assert_eq!(AslType::Byte.size(), 1);
        assert_eq!(AslType::Short.size(), 2);
        assert_eq!(AslType::Int.size(), 4);
        assert_eq!(AslType::Long.size(), 8);
        assert_eq!(AslType::Float.size(), 4);
    }
}
