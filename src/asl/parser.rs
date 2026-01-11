//! ASL Parser - Converts tokens into AST

use crate::asl::ast::*;
use crate::asl::lexer::{Lexer, Token, TokenKind};
use crate::asl::types::{VarDefinition, VarType};
use crate::asl::ParseError;

/// ASL Parser
pub struct Parser<'a> {
    lexer: Lexer<'a>,
    current: Token,
    previous: Token,
}

impl<'a> Parser<'a> {
    pub fn new(mut lexer: Lexer<'a>) -> Self {
        let current = lexer.next_token().unwrap_or(Token::new(TokenKind::Eof, 0, 0));
        Self {
            lexer,
            current: current.clone(),
            previous: current,
        }
    }

    /// Parse the complete ASL script
    pub fn parse(&mut self) -> Result<AslScript, ParseError> {
        let mut script = AslScript::default();

        while !self.is_at_end() {
            match &self.current.kind {
                TokenKind::State => {
                    let state = self.parse_state_block()?;
                    script.states.push(state);
                }
                TokenKind::Startup => {
                    self.advance()?;
                    script.startup = Some(self.parse_action_block()?);
                }
                TokenKind::Shutdown => {
                    self.advance()?;
                    script.shutdown = Some(self.parse_action_block()?);
                }
                TokenKind::Init => {
                    self.advance()?;
                    script.init = Some(self.parse_action_block()?);
                }
                TokenKind::Exit => {
                    self.advance()?;
                    script.exit = Some(self.parse_action_block()?);
                }
                TokenKind::Update => {
                    self.advance()?;
                    script.update = Some(self.parse_action_block()?);
                }
                TokenKind::Start => {
                    self.advance()?;
                    script.start = Some(self.parse_action_block()?);
                }
                TokenKind::Split => {
                    self.advance()?;
                    script.split = Some(self.parse_action_block()?);
                }
                TokenKind::Reset => {
                    self.advance()?;
                    script.reset = Some(self.parse_action_block()?);
                }
                TokenKind::IsLoading => {
                    self.advance()?;
                    script.is_loading = Some(self.parse_action_block()?);
                }
                TokenKind::GameTime => {
                    self.advance()?;
                    script.game_time = Some(self.parse_action_block()?);
                }
                _ => {
                    return Err(self.error(format!(
                        "Expected state or action block, got {:?}",
                        self.current.kind
                    )));
                }
            }
        }

        Ok(script)
    }

    /// Parse a state block: state("process.exe") { ... }
    fn parse_state_block(&mut self) -> Result<StateBlock, ParseError> {
        self.expect(TokenKind::State)?;
        self.expect(TokenKind::LParen)?;

        let mut process_names = Vec::new();

        // Parse process name(s)
        loop {
            if let TokenKind::String(name) = &self.current.kind {
                process_names.push(name.clone());
                self.advance()?;
            } else {
                return Err(self.error("Expected process name string"));
            }

            if self.current.kind == TokenKind::Comma {
                self.advance()?;
            } else {
                break;
            }
        }

        self.expect(TokenKind::RParen)?;
        self.expect(TokenKind::LBrace)?;

        let mut variables = Vec::new();

        while self.current.kind != TokenKind::RBrace && !self.is_at_end() {
            let var = self.parse_var_definition()?;
            variables.push(var);
        }

        self.expect(TokenKind::RBrace)?;

        Ok(StateBlock {
            process_names,
            variables,
        })
    }

    /// Parse a variable definition: type name : offsets;
    fn parse_var_definition(&mut self) -> Result<VarDefinition, ParseError> {
        // Parse type
        let var_type = self.parse_var_type()?;

        // Parse name (can be an identifier or a keyword used as name)
        let name = self.parse_identifier_or_keyword()?;
        self.advance()?;

        self.expect(TokenKind::Colon)?;

        // Parse pointer path: "module.dll", offset1, offset2, ...
        // Or just: offset1, offset2, ...
        let mut module = None;
        let mut offsets = Vec::new();

        if let TokenKind::String(mod_name) = &self.current.kind {
            module = Some(mod_name.clone());
            self.advance()?;
            if self.current.kind == TokenKind::Comma {
                self.advance()?;
            }
        }

        // Parse offsets
        loop {
            if let TokenKind::Int(offset) = self.current.kind {
                offsets.push(offset);
                self.advance()?;
            } else {
                break;
            }

            if self.current.kind == TokenKind::Comma {
                self.advance()?;
            } else {
                break;
            }
        }

        self.expect(TokenKind::Semicolon)?;

        Ok(VarDefinition {
            name,
            var_type,
            module,
            offsets,
            string_length: None,
        })
    }

    /// Parse variable type
    fn parse_var_type(&mut self) -> Result<VarType, ParseError> {
        let var_type = match &self.current.kind {
            TokenKind::Bool => VarType::Bool,
            TokenKind::Byte => VarType::Byte,
            TokenKind::SByte => VarType::SByte,
            TokenKind::Short => VarType::Short,
            TokenKind::UShort => VarType::UShort,
            TokenKind::IntType => VarType::Int,
            TokenKind::UInt => VarType::UInt,
            TokenKind::Long => VarType::Long,
            TokenKind::ULong => VarType::ULong,
            TokenKind::FloatType => VarType::Float,
            TokenKind::Double => VarType::Double,
            TokenKind::StringType => VarType::String,
            TokenKind::Ident(name) => {
                VarType::from_str(name).ok_or_else(|| self.error(format!("Unknown type: {}", name)))?
            }
            _ => return Err(self.error(format!("Expected type, got {:?}", self.current.kind))),
        };
        self.advance()?;
        Ok(var_type)
    }

    /// Parse an action block: { statements... }
    fn parse_action_block(&mut self) -> Result<ActionBlock, ParseError> {
        self.expect(TokenKind::LBrace)?;

        let mut statements = Vec::new();

        while self.current.kind != TokenKind::RBrace && !self.is_at_end() {
            let stmt = self.parse_statement()?;
            statements.push(stmt);
        }

        self.expect(TokenKind::RBrace)?;

        Ok(ActionBlock { statements })
    }

    /// Parse a statement
    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        match &self.current.kind {
            TokenKind::Var => {
                self.advance()?;
                let name = self.parse_identifier_or_keyword()?;
                self.advance()?;
                self.expect(TokenKind::Assign)?;
                let value = self.parse_expression()?;
                self.expect(TokenKind::Semicolon)?;
                Ok(Statement::VarDecl { name, value })
            }
            TokenKind::If => {
                self.advance()?;
                self.expect(TokenKind::LParen)?;
                let condition = self.parse_expression()?;
                self.expect(TokenKind::RParen)?;

                let then_branch = if self.current.kind == TokenKind::LBrace {
                    self.expect(TokenKind::LBrace)?;
                    let mut stmts = Vec::new();
                    while self.current.kind != TokenKind::RBrace && !self.is_at_end() {
                        stmts.push(self.parse_statement()?);
                    }
                    self.expect(TokenKind::RBrace)?;
                    stmts
                } else {
                    vec![self.parse_statement()?]
                };

                let else_branch = if self.current.kind == TokenKind::Else {
                    self.advance()?;
                    if self.current.kind == TokenKind::LBrace {
                        self.expect(TokenKind::LBrace)?;
                        let mut stmts = Vec::new();
                        while self.current.kind != TokenKind::RBrace && !self.is_at_end() {
                            stmts.push(self.parse_statement()?);
                        }
                        self.expect(TokenKind::RBrace)?;
                        Some(stmts)
                    } else {
                        Some(vec![self.parse_statement()?])
                    }
                } else {
                    None
                };

                Ok(Statement::If {
                    condition,
                    then_branch,
                    else_branch,
                })
            }
            TokenKind::Return => {
                self.advance()?;
                let value = if self.current.kind != TokenKind::Semicolon {
                    Some(self.parse_expression()?)
                } else {
                    None
                };
                self.expect(TokenKind::Semicolon)?;
                Ok(Statement::Return { value })
            }
            TokenKind::Ident(_) => {
                // Could be assignment or expression statement
                let expr = self.parse_expression()?;

                if self.current.kind == TokenKind::Assign {
                    // Assignment
                    if let Expression::Variable { name, scope: VarScope::Local } = expr {
                        self.advance()?;
                        let value = self.parse_expression()?;
                        self.expect(TokenKind::Semicolon)?;
                        Ok(Statement::Assignment { target: name, value })
                    } else {
                        Err(self.error("Invalid assignment target"))
                    }
                } else {
                    self.expect(TokenKind::Semicolon)?;
                    Ok(Statement::Expression(expr))
                }
            }
            _ => {
                // Expression statement
                let expr = self.parse_expression()?;
                self.expect(TokenKind::Semicolon)?;
                Ok(Statement::Expression(expr))
            }
        }
    }

    /// Parse an expression
    fn parse_expression(&mut self) -> Result<Expression, ParseError> {
        self.parse_ternary()
    }

    /// Parse ternary: expr ? expr : expr
    fn parse_ternary(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_or()?;

        if self.current.kind == TokenKind::Question {
            self.advance()?;
            let then_expr = self.parse_expression()?;
            self.expect(TokenKind::Colon)?;
            let else_expr = self.parse_expression()?;
            expr = Expression::Ternary {
                condition: Box::new(expr),
                then_expr: Box::new(then_expr),
                else_expr: Box::new(else_expr),
            };
        }

        Ok(expr)
    }

    /// Parse logical OR: expr || expr
    fn parse_or(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_and()?;

        while self.current.kind == TokenKind::Or {
            self.advance()?;
            let right = self.parse_and()?;
            left = Expression::Binary {
                left: Box::new(left),
                op: BinaryOp::Or,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    /// Parse logical AND: expr && expr
    fn parse_and(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_bitwise_or()?;

        while self.current.kind == TokenKind::And {
            self.advance()?;
            let right = self.parse_bitwise_or()?;
            left = Expression::Binary {
                left: Box::new(left),
                op: BinaryOp::And,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    /// Parse bitwise OR
    fn parse_bitwise_or(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_bitwise_xor()?;

        while self.current.kind == TokenKind::BitOr {
            self.advance()?;
            let right = self.parse_bitwise_xor()?;
            left = Expression::Binary {
                left: Box::new(left),
                op: BinaryOp::BitOr,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    /// Parse bitwise XOR
    fn parse_bitwise_xor(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_bitwise_and()?;

        while self.current.kind == TokenKind::BitXor {
            self.advance()?;
            let right = self.parse_bitwise_and()?;
            left = Expression::Binary {
                left: Box::new(left),
                op: BinaryOp::BitXor,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    /// Parse bitwise AND
    fn parse_bitwise_and(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_equality()?;

        while self.current.kind == TokenKind::BitAnd {
            self.advance()?;
            let right = self.parse_equality()?;
            left = Expression::Binary {
                left: Box::new(left),
                op: BinaryOp::BitAnd,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    /// Parse equality: expr == expr, expr != expr
    fn parse_equality(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_comparison()?;

        loop {
            let op = match self.current.kind {
                TokenKind::Eq => BinaryOp::Eq,
                TokenKind::Ne => BinaryOp::Ne,
                _ => break,
            };
            self.advance()?;
            let right = self.parse_comparison()?;
            left = Expression::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    /// Parse comparison: <, <=, >, >=
    fn parse_comparison(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_shift()?;

        loop {
            let op = match self.current.kind {
                TokenKind::Lt => BinaryOp::Lt,
                TokenKind::Le => BinaryOp::Le,
                TokenKind::Gt => BinaryOp::Gt,
                TokenKind::Ge => BinaryOp::Ge,
                _ => break,
            };
            self.advance()?;
            let right = self.parse_shift()?;
            left = Expression::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    /// Parse shift: <<, >>
    fn parse_shift(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_additive()?;

        loop {
            let op = match self.current.kind {
                TokenKind::Shl => BinaryOp::Shl,
                TokenKind::Shr => BinaryOp::Shr,
                _ => break,
            };
            self.advance()?;
            let right = self.parse_additive()?;
            left = Expression::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    /// Parse additive: +, -
    fn parse_additive(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_multiplicative()?;

        loop {
            let op = match self.current.kind {
                TokenKind::Plus => BinaryOp::Add,
                TokenKind::Minus => BinaryOp::Sub,
                _ => break,
            };
            self.advance()?;
            let right = self.parse_multiplicative()?;
            left = Expression::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    /// Parse multiplicative: *, /, %
    fn parse_multiplicative(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_unary()?;

        loop {
            let op = match self.current.kind {
                TokenKind::Star => BinaryOp::Mul,
                TokenKind::Slash => BinaryOp::Div,
                TokenKind::Percent => BinaryOp::Mod,
                _ => break,
            };
            self.advance()?;
            let right = self.parse_unary()?;
            left = Expression::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    /// Parse unary: !, -, ~
    fn parse_unary(&mut self) -> Result<Expression, ParseError> {
        match self.current.kind {
            TokenKind::Not => {
                self.advance()?;
                let expr = self.parse_unary()?;
                Ok(Expression::Unary {
                    op: UnaryOp::Not,
                    expr: Box::new(expr),
                })
            }
            TokenKind::Minus => {
                self.advance()?;
                let expr = self.parse_unary()?;
                Ok(Expression::Unary {
                    op: UnaryOp::Neg,
                    expr: Box::new(expr),
                })
            }
            TokenKind::BitNot => {
                self.advance()?;
                let expr = self.parse_unary()?;
                Ok(Expression::Unary {
                    op: UnaryOp::BitNot,
                    expr: Box::new(expr),
                })
            }
            _ => self.parse_postfix(),
        }
    }

    /// Parse postfix: member access, indexing, function calls
    fn parse_postfix(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_primary()?;

        loop {
            match &self.current.kind {
                TokenKind::Dot => {
                    self.advance()?;
                    let member = self.parse_identifier_or_keyword()?;
                    self.advance()?;
                    expr = Expression::Member {
                        object: Box::new(expr),
                        member,
                    };
                }
                TokenKind::LBracket => {
                    self.advance()?;
                    let index = self.parse_expression()?;
                    self.expect(TokenKind::RBracket)?;
                    expr = Expression::Index {
                        object: Box::new(expr),
                        index: Box::new(index),
                    };
                }
                TokenKind::LParen => {
                    // Function call
                    if let Expression::Variable { name, scope: VarScope::Local } = expr {
                        self.advance()?;
                        let mut args = Vec::new();
                        while self.current.kind != TokenKind::RParen {
                            args.push(self.parse_expression()?);
                            if self.current.kind == TokenKind::Comma {
                                self.advance()?;
                            } else {
                                break;
                            }
                        }
                        self.expect(TokenKind::RParen)?;
                        expr = Expression::Call { name, args };
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    /// Parse primary expression
    fn parse_primary(&mut self) -> Result<Expression, ParseError> {
        let expr = match &self.current.kind {
            TokenKind::Int(i) => {
                let val = *i;
                self.advance()?;
                Expression::Literal(Literal::Int(val))
            }
            TokenKind::Float(f) => {
                let val = *f;
                self.advance()?;
                Expression::Literal(Literal::Float(val))
            }
            TokenKind::String(s) => {
                let val = s.clone();
                self.advance()?;
                Expression::Literal(Literal::String(val))
            }
            TokenKind::True => {
                self.advance()?;
                Expression::Literal(Literal::Bool(true))
            }
            TokenKind::False => {
                self.advance()?;
                Expression::Literal(Literal::Bool(false))
            }
            TokenKind::Null => {
                self.advance()?;
                Expression::Literal(Literal::Null)
            }
            TokenKind::Current => {
                self.advance()?;
                self.expect(TokenKind::Dot)?;
                let name = self.parse_identifier_or_keyword()?;
                self.advance()?;
                Expression::Variable {
                    scope: VarScope::Current,
                    name,
                }
            }
            TokenKind::Old => {
                self.advance()?;
                self.expect(TokenKind::Dot)?;
                let name = self.parse_identifier_or_keyword()?;
                self.advance()?;
                Expression::Variable {
                    scope: VarScope::Old,
                    name,
                }
            }
            TokenKind::Settings => {
                self.advance()?;
                self.expect(TokenKind::Dot)?;
                let name = self.parse_identifier_or_keyword()?;
                self.advance()?;
                Expression::Variable {
                    scope: VarScope::Settings,
                    name,
                }
            }
            TokenKind::Ident(name) => {
                let name = name.clone();
                self.advance()?;
                Expression::Variable {
                    scope: VarScope::Local,
                    name,
                }
            }
            TokenKind::LParen => {
                self.advance()?;
                let expr = self.parse_expression()?;
                self.expect(TokenKind::RParen)?;
                expr
            }
            _ => {
                return Err(self.error(format!(
                    "Unexpected token in expression: {:?}",
                    self.current.kind
                )));
            }
        };

        Ok(expr)
    }

    // Helper methods

    fn advance(&mut self) -> Result<(), ParseError> {
        self.previous = self.current.clone();
        self.current = self.lexer.next_token()?;
        Ok(())
    }

    fn expect(&mut self, kind: TokenKind) -> Result<(), ParseError> {
        if std::mem::discriminant(&self.current.kind) == std::mem::discriminant(&kind) {
            self.advance()?;
            Ok(())
        } else {
            Err(self.error(format!("Expected {:?}, got {:?}", kind, self.current.kind)))
        }
    }

    fn is_at_end(&self) -> bool {
        matches!(self.current.kind, TokenKind::Eof)
    }

    fn error(&self, message: impl Into<String>) -> ParseError {
        ParseError::new(message, self.current.line, self.current.column)
    }

    /// Parse an identifier or a keyword used as an identifier
    /// Some keywords like "state" can be used as variable names
    fn parse_identifier_or_keyword(&self) -> Result<String, ParseError> {
        match &self.current.kind {
            TokenKind::Ident(name) => Ok(name.clone()),
            // Allow keywords to be used as variable names
            TokenKind::State => Ok("state".to_string()),
            TokenKind::Start => Ok("start".to_string()),
            TokenKind::Split => Ok("split".to_string()),
            TokenKind::Reset => Ok("reset".to_string()),
            TokenKind::Update => Ok("update".to_string()),
            TokenKind::Init => Ok("init".to_string()),
            TokenKind::Exit => Ok("exit".to_string()),
            TokenKind::Startup => Ok("startup".to_string()),
            TokenKind::Shutdown => Ok("shutdown".to_string()),
            TokenKind::IsLoading => Ok("isLoading".to_string()),
            TokenKind::GameTime => Ok("gameTime".to_string()),
            TokenKind::Current => Ok("current".to_string()),
            TokenKind::Old => Ok("old".to_string()),
            TokenKind::Settings => Ok("settings".to_string()),
            TokenKind::Var => Ok("var".to_string()),
            TokenKind::If => Ok("if".to_string()),
            TokenKind::Else => Ok("else".to_string()),
            TokenKind::Return => Ok("return".to_string()),
            _ => Err(self.error("Expected identifier")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_state() {
        let source = r#"
            state("game.exe") {
                int health : 0x12345678;
            }
        "#;
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer);
        let script = parser.parse().unwrap();

        assert_eq!(script.states.len(), 1);
        assert_eq!(script.states[0].process_names, vec!["game.exe"]);
        assert_eq!(script.states[0].variables.len(), 1);
        assert_eq!(script.states[0].variables[0].name, "health");
    }

    #[test]
    fn test_parse_simple_action() {
        let source = r#"
            start {
                return current.health > 0;
            }
        "#;
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer);
        let script = parser.parse().unwrap();

        assert!(script.start.is_some());
    }

    #[test]
    fn test_parse_if_statement() {
        let source = r#"
            split {
                if (current.level > old.level) {
                    return true;
                }
                return false;
            }
        "#;
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer);
        let script = parser.parse().unwrap();

        assert!(script.split.is_some());
    }
}
