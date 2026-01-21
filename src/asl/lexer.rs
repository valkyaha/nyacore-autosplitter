//! ASL Lexer - Tokenizes ASL script content
//!
//! The lexer converts raw ASL text into a stream of tokens that can be
//! consumed by the parser.

use super::error::{AslError, AslResult};

/// Token kind enum
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords
    State,
    Startup,
    Init,
    Split,
    Reset,
    IsLoading,
    If,
    Return,
    True,
    False,

    // Type keywords
    Bool,
    Int,
    Byte,
    Float,
    String,
    Short,
    Long,
    UInt,
    UShort,
    ULong,

    // Special identifiers
    Current,
    Old,

    // Symbols
    LeftBrace,    // {
    RightBrace,   // }
    LeftParen,    // (
    RightParen,   // )
    LeftBracket,  // [
    RightBracket, // ]
    Colon,        // :
    Semicolon,    // ;
    Comma,        // ,
    Dot,          // .

    // Operators
    And,       // &&
    Or,        // ||
    Not,       // !
    Equals,    // ==
    NotEquals, // !=
    Greater,   // >
    Less,      // <
    GreaterEq, // >=
    LessEq,    // <=
    Assign,    // =

    // Literals
    Identifier(String),
    StringLiteral(String),
    NumberLiteral(i64),
    HexLiteral(u64),
    FloatLiteral(f64),

    // Special
    Eof,
}

/// A token with its location information
#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub column: usize,
}

impl Token {
    pub fn new(kind: TokenKind, line: usize, column: usize) -> Self {
        Self { kind, line, column }
    }
}

/// ASL Lexer
pub struct Lexer<'a> {
    input: &'a str,
    chars: std::iter::Peekable<std::str::CharIndices<'a>>,
    line: usize,
    column: usize,
    current_pos: usize,
}

impl<'a> Lexer<'a> {
    /// Create a new lexer for the given input
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            chars: input.char_indices().peekable(),
            line: 1,
            column: 1,
            current_pos: 0,
        }
    }

    /// Tokenize the entire input
    pub fn tokenize(&mut self) -> AslResult<Vec<Token>> {
        let mut tokens = Vec::new();

        loop {
            let token = self.next_token()?;
            let is_eof = token.kind == TokenKind::Eof;
            tokens.push(token);
            if is_eof {
                break;
            }
        }

        Ok(tokens)
    }

    /// Get the next token
    fn next_token(&mut self) -> AslResult<Token> {
        self.skip_whitespace_and_comments();

        let line = self.line;
        let column = self.column;

        match self.peek_char() {
            None => Ok(Token::new(TokenKind::Eof, line, column)),
            Some(ch) => {
                match ch {
                    // Single-character tokens
                    '{' => {
                        self.advance();
                        Ok(Token::new(TokenKind::LeftBrace, line, column))
                    }
                    '}' => {
                        self.advance();
                        Ok(Token::new(TokenKind::RightBrace, line, column))
                    }
                    '(' => {
                        self.advance();
                        Ok(Token::new(TokenKind::LeftParen, line, column))
                    }
                    ')' => {
                        self.advance();
                        Ok(Token::new(TokenKind::RightParen, line, column))
                    }
                    '[' => {
                        self.advance();
                        Ok(Token::new(TokenKind::LeftBracket, line, column))
                    }
                    ']' => {
                        self.advance();
                        Ok(Token::new(TokenKind::RightBracket, line, column))
                    }
                    ':' => {
                        self.advance();
                        Ok(Token::new(TokenKind::Colon, line, column))
                    }
                    ';' => {
                        self.advance();
                        Ok(Token::new(TokenKind::Semicolon, line, column))
                    }
                    ',' => {
                        self.advance();
                        Ok(Token::new(TokenKind::Comma, line, column))
                    }
                    '.' => {
                        self.advance();
                        Ok(Token::new(TokenKind::Dot, line, column))
                    }

                    // Two-character operators
                    '&' => {
                        self.advance();
                        if self.peek_char() == Some('&') {
                            self.advance();
                            Ok(Token::new(TokenKind::And, line, column))
                        } else {
                            Err(AslError::lexer("Expected '&&'", line, column))
                        }
                    }
                    '|' => {
                        self.advance();
                        if self.peek_char() == Some('|') {
                            self.advance();
                            Ok(Token::new(TokenKind::Or, line, column))
                        } else {
                            Err(AslError::lexer("Expected '||'", line, column))
                        }
                    }
                    '!' => {
                        self.advance();
                        if self.peek_char() == Some('=') {
                            self.advance();
                            Ok(Token::new(TokenKind::NotEquals, line, column))
                        } else {
                            Ok(Token::new(TokenKind::Not, line, column))
                        }
                    }
                    '=' => {
                        self.advance();
                        if self.peek_char() == Some('=') {
                            self.advance();
                            Ok(Token::new(TokenKind::Equals, line, column))
                        } else {
                            Ok(Token::new(TokenKind::Assign, line, column))
                        }
                    }
                    '>' => {
                        self.advance();
                        if self.peek_char() == Some('=') {
                            self.advance();
                            Ok(Token::new(TokenKind::GreaterEq, line, column))
                        } else {
                            Ok(Token::new(TokenKind::Greater, line, column))
                        }
                    }
                    '<' => {
                        self.advance();
                        if self.peek_char() == Some('=') {
                            self.advance();
                            Ok(Token::new(TokenKind::LessEq, line, column))
                        } else {
                            Ok(Token::new(TokenKind::Less, line, column))
                        }
                    }

                    // String literals
                    '"' => self.read_string_literal(line, column),

                    // Numbers (including hex)
                    '0'..='9' => self.read_number(line, column),

                    // Identifiers and keywords
                    'a'..='z' | 'A'..='Z' | '_' => self.read_identifier(line, column),

                    _ => Err(AslError::lexer(
                        format!("Unexpected character: '{}'", ch),
                        line,
                        column,
                    )),
                }
            }
        }
    }

    /// Peek at the current character without consuming it
    fn peek_char(&mut self) -> Option<char> {
        self.chars.peek().map(|(_, ch)| *ch)
    }

    /// Advance to the next character
    fn advance(&mut self) -> Option<char> {
        if let Some((pos, ch)) = self.chars.next() {
            self.current_pos = pos;
            if ch == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
            Some(ch)
        } else {
            None
        }
    }

    /// Skip whitespace and comments
    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.peek_char() {
                Some(ch) if ch.is_whitespace() => {
                    self.advance();
                }
                Some('/') => {
                    // Check for comments
                    let saved_chars = self.chars.clone();
                    let saved_line = self.line;
                    let saved_column = self.column;
                    let saved_pos = self.current_pos;

                    self.advance(); // consume '/'

                    match self.peek_char() {
                        Some('/') => {
                            // Line comment - skip to end of line
                            while let Some(ch) = self.peek_char() {
                                if ch == '\n' {
                                    break;
                                }
                                self.advance();
                            }
                        }
                        Some('*') => {
                            // Block comment - skip until */
                            self.advance(); // consume '*'
                            loop {
                                match self.advance() {
                                    Some('*') => {
                                        if self.peek_char() == Some('/') {
                                            self.advance();
                                            break;
                                        }
                                    }
                                    None => break, // Unterminated comment, let parser handle
                                    _ => {}
                                }
                            }
                        }
                        _ => {
                            // Not a comment, restore state
                            self.chars = saved_chars;
                            self.line = saved_line;
                            self.column = saved_column;
                            self.current_pos = saved_pos;
                            return;
                        }
                    }
                }
                _ => return,
            }
        }
    }

    /// Read a string literal
    fn read_string_literal(&mut self, line: usize, column: usize) -> AslResult<Token> {
        self.advance(); // consume opening quote

        let mut value = String::new();

        loop {
            match self.advance() {
                Some('"') => break,
                Some('\\') => {
                    // Handle escape sequences
                    match self.advance() {
                        Some('n') => value.push('\n'),
                        Some('r') => value.push('\r'),
                        Some('t') => value.push('\t'),
                        Some('\\') => value.push('\\'),
                        Some('"') => value.push('"'),
                        Some(ch) => value.push(ch),
                        None => {
                            return Err(AslError::lexer("Unterminated string literal", line, column))
                        }
                    }
                }
                Some(ch) => value.push(ch),
                None => return Err(AslError::lexer("Unterminated string literal", line, column)),
            }
        }

        Ok(Token::new(TokenKind::StringLiteral(value), line, column))
    }

    /// Read a number (decimal, hex, or float)
    fn read_number(&mut self, line: usize, column: usize) -> AslResult<Token> {
        // Capture start position from the peek (the position of the first digit)
        let start_pos = self.chars.peek().map(|(pos, _)| *pos).unwrap_or(0);

        // Check for hex prefix
        if self.peek_char() == Some('0') {
            self.advance();
            if let Some('x') | Some('X') = self.peek_char() {
                self.advance();
                return self.read_hex_number(line, column);
            }
            // Put back if not hex - we already consumed '0'
            // Continue reading as decimal
        }

        // Read decimal digits
        let mut has_dot = false;
        while let Some(ch) = self.peek_char() {
            if ch.is_ascii_digit() {
                self.advance();
            } else if ch == '.' && !has_dot {
                // Check if next char is digit (float) or not (method call)
                let saved_chars = self.chars.clone();
                let saved_line = self.line;
                let saved_column = self.column;
                let saved_pos = self.current_pos;

                self.advance(); // consume '.'

                if self.peek_char().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                    has_dot = true;
                } else {
                    // Not a float, restore and stop
                    self.chars = saved_chars;
                    self.line = saved_line;
                    self.column = saved_column;
                    self.current_pos = saved_pos;
                    break;
                }
            } else {
                break;
            }
        }

        // Get the numeric string
        let end_pos = self
            .chars
            .peek()
            .map(|(pos, _)| *pos)
            .unwrap_or(self.input.len());
        let num_str = &self.input[start_pos..end_pos];

        if has_dot {
            let value: f64 = num_str
                .parse()
                .map_err(|_| AslError::lexer(format!("Invalid float: {}", num_str), line, column))?;
            Ok(Token::new(TokenKind::FloatLiteral(value), line, column))
        } else {
            let value: i64 = num_str
                .parse()
                .map_err(|_| AslError::lexer(format!("Invalid number: {}", num_str), line, column))?;
            Ok(Token::new(TokenKind::NumberLiteral(value), line, column))
        }
    }

    /// Read a hexadecimal number (after 0x prefix)
    fn read_hex_number(&mut self, line: usize, column: usize) -> AslResult<Token> {
        let mut hex_str = String::new();

        while let Some(ch) = self.peek_char() {
            if ch.is_ascii_hexdigit() {
                hex_str.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        if hex_str.is_empty() {
            return Err(AslError::lexer("Expected hex digits after 0x", line, column));
        }

        let value = u64::from_str_radix(&hex_str, 16)
            .map_err(|_| AslError::lexer(format!("Invalid hex number: 0x{}", hex_str), line, column))?;

        Ok(Token::new(TokenKind::HexLiteral(value), line, column))
    }

    /// Read an identifier or keyword
    fn read_identifier(&mut self, line: usize, column: usize) -> AslResult<Token> {
        let mut ident = String::new();

        while let Some(ch) = self.peek_char() {
            if ch.is_alphanumeric() || ch == '_' {
                ident.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        // Check for keywords
        let kind = match ident.as_str() {
            // Block keywords
            "state" => TokenKind::State,
            "startup" => TokenKind::Startup,
            "init" => TokenKind::Init,
            "split" => TokenKind::Split,
            "reset" => TokenKind::Reset,
            "isLoading" => TokenKind::IsLoading,

            // Control flow
            "if" => TokenKind::If,
            "return" => TokenKind::Return,
            "true" => TokenKind::True,
            "false" => TokenKind::False,

            // Types
            "bool" => TokenKind::Bool,
            "int" => TokenKind::Int,
            "byte" => TokenKind::Byte,
            "sbyte" => TokenKind::Byte, // Signed byte, treat same
            "float" => TokenKind::Float,
            "double" => TokenKind::Float, // Treat double as float
            "string" => TokenKind::String,
            "short" => TokenKind::Short,
            "long" => TokenKind::Long,
            "uint" => TokenKind::UInt,
            "ushort" => TokenKind::UShort,
            "ulong" => TokenKind::ULong,

            // Special identifiers
            "current" => TokenKind::Current,
            "old" => TokenKind::Old,

            // Regular identifier
            _ => TokenKind::Identifier(ident),
        };

        Ok(Token::new(kind, line, column))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_tokens() {
        let mut lexer = Lexer::new("{ } ( ) : ; ,");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0].kind, TokenKind::LeftBrace);
        assert_eq!(tokens[1].kind, TokenKind::RightBrace);
        assert_eq!(tokens[2].kind, TokenKind::LeftParen);
        assert_eq!(tokens[3].kind, TokenKind::RightParen);
        assert_eq!(tokens[4].kind, TokenKind::Colon);
        assert_eq!(tokens[5].kind, TokenKind::Semicolon);
        assert_eq!(tokens[6].kind, TokenKind::Comma);
        assert_eq!(tokens[7].kind, TokenKind::Eof);
    }

    #[test]
    fn test_operators() {
        let mut lexer = Lexer::new("&& || ! == != > < >= <=");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0].kind, TokenKind::And);
        assert_eq!(tokens[1].kind, TokenKind::Or);
        assert_eq!(tokens[2].kind, TokenKind::Not);
        assert_eq!(tokens[3].kind, TokenKind::Equals);
        assert_eq!(tokens[4].kind, TokenKind::NotEquals);
        assert_eq!(tokens[5].kind, TokenKind::Greater);
        assert_eq!(tokens[6].kind, TokenKind::Less);
        assert_eq!(tokens[7].kind, TokenKind::GreaterEq);
        assert_eq!(tokens[8].kind, TokenKind::LessEq);
    }

    #[test]
    fn test_keywords() {
        let mut lexer = Lexer::new("state startup init split reset isLoading if return true false");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0].kind, TokenKind::State);
        assert_eq!(tokens[1].kind, TokenKind::Startup);
        assert_eq!(tokens[2].kind, TokenKind::Init);
        assert_eq!(tokens[3].kind, TokenKind::Split);
        assert_eq!(tokens[4].kind, TokenKind::Reset);
        assert_eq!(tokens[5].kind, TokenKind::IsLoading);
        assert_eq!(tokens[6].kind, TokenKind::If);
        assert_eq!(tokens[7].kind, TokenKind::Return);
        assert_eq!(tokens[8].kind, TokenKind::True);
        assert_eq!(tokens[9].kind, TokenKind::False);
    }

    #[test]
    fn test_types() {
        let mut lexer = Lexer::new("bool int byte float string");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0].kind, TokenKind::Bool);
        assert_eq!(tokens[1].kind, TokenKind::Int);
        assert_eq!(tokens[2].kind, TokenKind::Byte);
        assert_eq!(tokens[3].kind, TokenKind::Float);
        assert_eq!(tokens[4].kind, TokenKind::String);
    }

    #[test]
    fn test_special_identifiers() {
        let mut lexer = Lexer::new("current old");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0].kind, TokenKind::Current);
        assert_eq!(tokens[1].kind, TokenKind::Old);
    }

    #[test]
    fn test_identifiers() {
        let mut lexer = Lexer::new("myVar _private camelCase snake_case");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0].kind, TokenKind::Identifier("myVar".to_string()));
        assert_eq!(tokens[1].kind, TokenKind::Identifier("_private".to_string()));
        assert_eq!(tokens[2].kind, TokenKind::Identifier("camelCase".to_string()));
        assert_eq!(tokens[3].kind, TokenKind::Identifier("snake_case".to_string()));
    }

    #[test]
    fn test_string_literals() {
        let mut lexer = Lexer::new(r#""hello" "with spaces" "escape\"quote""#);
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0].kind, TokenKind::StringLiteral("hello".to_string()));
        assert_eq!(tokens[1].kind, TokenKind::StringLiteral("with spaces".to_string()));
        assert_eq!(tokens[2].kind, TokenKind::StringLiteral("escape\"quote".to_string()));
    }

    #[test]
    fn test_numbers() {
        let mut lexer = Lexer::new("123 0 999999");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0].kind, TokenKind::NumberLiteral(123));
        assert_eq!(tokens[1].kind, TokenKind::NumberLiteral(0));
        assert_eq!(tokens[2].kind, TokenKind::NumberLiteral(999999));
        // Note: negative numbers like -42 would be parsed as two tokens
        // (unary minus operator and the number) which is not supported in ASL
    }

    #[test]
    fn test_hex_numbers() {
        let mut lexer = Lexer::new("0x00 0xFF 0x1A2B 0xDEADBEEF");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0].kind, TokenKind::HexLiteral(0x00));
        assert_eq!(tokens[1].kind, TokenKind::HexLiteral(0xFF));
        assert_eq!(tokens[2].kind, TokenKind::HexLiteral(0x1A2B));
        assert_eq!(tokens[3].kind, TokenKind::HexLiteral(0xDEADBEEF));
    }

    #[test]
    fn test_line_comment() {
        let mut lexer = Lexer::new("token1 // this is a comment\ntoken2");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0].kind, TokenKind::Identifier("token1".to_string()));
        assert_eq!(tokens[1].kind, TokenKind::Identifier("token2".to_string()));
    }

    #[test]
    fn test_block_comment() {
        let mut lexer = Lexer::new("token1 /* block comment */ token2");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0].kind, TokenKind::Identifier("token1".to_string()));
        assert_eq!(tokens[1].kind, TokenKind::Identifier("token2".to_string()));
    }

    #[test]
    fn test_multiline_block_comment() {
        let mut lexer = Lexer::new("token1 /* multi\nline\ncomment */ token2");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0].kind, TokenKind::Identifier("token1".to_string()));
        assert_eq!(tokens[1].kind, TokenKind::Identifier("token2".to_string()));
    }

    #[test]
    fn test_state_block() {
        let input = r#"state("DarkSoulsIII.exe") {
            bool testVar : "pointer", 12345;
        }"#;
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0].kind, TokenKind::State);
        assert_eq!(tokens[1].kind, TokenKind::LeftParen);
        assert_eq!(tokens[2].kind, TokenKind::StringLiteral("DarkSoulsIII.exe".to_string()));
        assert_eq!(tokens[3].kind, TokenKind::RightParen);
        assert_eq!(tokens[4].kind, TokenKind::LeftBrace);
        assert_eq!(tokens[5].kind, TokenKind::Bool);
        assert_eq!(tokens[6].kind, TokenKind::Identifier("testVar".to_string()));
        assert_eq!(tokens[7].kind, TokenKind::Colon);
        assert_eq!(tokens[8].kind, TokenKind::StringLiteral("pointer".to_string()));
        assert_eq!(tokens[9].kind, TokenKind::Comma);
        assert_eq!(tokens[10].kind, TokenKind::NumberLiteral(12345));
        assert_eq!(tokens[11].kind, TokenKind::Semicolon);
        assert_eq!(tokens[12].kind, TokenKind::RightBrace);
    }

    #[test]
    fn test_split_block() {
        let input = r#"split {
            if (current.boss && !old.boss) { return true; }
            return false;
        }"#;
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0].kind, TokenKind::Split);
        assert_eq!(tokens[1].kind, TokenKind::LeftBrace);
        assert_eq!(tokens[2].kind, TokenKind::If);
        assert_eq!(tokens[3].kind, TokenKind::LeftParen);
        assert_eq!(tokens[4].kind, TokenKind::Current);
        assert_eq!(tokens[5].kind, TokenKind::Dot);
        assert_eq!(tokens[6].kind, TokenKind::Identifier("boss".to_string()));
        assert_eq!(tokens[7].kind, TokenKind::And);
        assert_eq!(tokens[8].kind, TokenKind::Not);
        assert_eq!(tokens[9].kind, TokenKind::Old);
    }

    #[test]
    fn test_line_tracking() {
        let input = "line1\nline2\nline3";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0].line, 1);
        assert_eq!(tokens[1].line, 2);
        assert_eq!(tokens[2].line, 3);
    }

    #[test]
    fn test_ds2_offset_chain() {
        let input = r#"int boss : "pattern", 0x0, 0x70, 0x28;"#;
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0].kind, TokenKind::Int);
        assert_eq!(tokens[1].kind, TokenKind::Identifier("boss".to_string()));
        assert_eq!(tokens[2].kind, TokenKind::Colon);
        assert_eq!(tokens[3].kind, TokenKind::StringLiteral("pattern".to_string()));
        assert_eq!(tokens[4].kind, TokenKind::Comma);
        assert_eq!(tokens[5].kind, TokenKind::HexLiteral(0x0));
        assert_eq!(tokens[6].kind, TokenKind::Comma);
        assert_eq!(tokens[7].kind, TokenKind::HexLiteral(0x70));
        assert_eq!(tokens[8].kind, TokenKind::Comma);
        assert_eq!(tokens[9].kind, TokenKind::HexLiteral(0x28));
        assert_eq!(tokens[10].kind, TokenKind::Semicolon);
    }
}
