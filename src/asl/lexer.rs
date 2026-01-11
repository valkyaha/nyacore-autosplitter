//! ASL Lexer (Tokenizer)

use crate::asl::ParseError;

/// Token types
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    Int(i64),
    Float(f64),
    String(String),
    True,
    False,
    Null,

    // Identifiers and keywords
    Ident(String),
    State,
    Startup,
    Shutdown,
    Init,
    Exit,
    Update,
    Start,
    Split,
    Reset,
    IsLoading,
    GameTime,
    If,
    Else,
    Return,
    Var,
    Current,
    Old,
    Settings,

    // Type keywords
    Bool,
    Byte,
    SByte,
    Short,
    UShort,
    IntType,
    UInt,
    Long,
    ULong,
    FloatType,
    Double,
    StringType,

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Not,
    BitAnd,
    BitOr,
    BitXor,
    BitNot,
    Shl,
    Shr,
    Assign,
    Question,

    // Delimiters
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Colon,
    Semicolon,
    Dot,

    // Special
    Eof,
}

/// Token with position information
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

/// Lexer for ASL scripts
pub struct Lexer<'a> {
    source: &'a str,
    chars: std::iter::Peekable<std::str::CharIndices<'a>>,
    line: usize,
    column: usize,
    current_pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            chars: source.char_indices().peekable(),
            line: 1,
            column: 1,
            current_pos: 0,
        }
    }

    /// Get next token
    pub fn next_token(&mut self) -> Result<Token, ParseError> {
        self.skip_whitespace_and_comments();

        let line = self.line;
        let column = self.column;

        let Some((pos, ch)) = self.advance() else {
            return Ok(Token::new(TokenKind::Eof, line, column));
        };

        self.current_pos = pos;

        let kind = match ch {
            // Single character tokens
            '(' => TokenKind::LParen,
            ')' => TokenKind::RParen,
            '{' => TokenKind::LBrace,
            '}' => TokenKind::RBrace,
            '[' => TokenKind::LBracket,
            ']' => TokenKind::RBracket,
            ',' => TokenKind::Comma,
            ':' => TokenKind::Colon,
            ';' => TokenKind::Semicolon,
            '.' => TokenKind::Dot,
            '?' => TokenKind::Question,
            '+' => TokenKind::Plus,
            '-' => TokenKind::Minus,
            '*' => TokenKind::Star,
            '/' => TokenKind::Slash,
            '%' => TokenKind::Percent,
            '~' => TokenKind::BitNot,
            '^' => TokenKind::BitXor,

            // Multi-character tokens
            '=' => {
                if self.match_char('=') {
                    TokenKind::Eq
                } else {
                    TokenKind::Assign
                }
            }
            '!' => {
                if self.match_char('=') {
                    TokenKind::Ne
                } else {
                    TokenKind::Not
                }
            }
            '<' => {
                if self.match_char('=') {
                    TokenKind::Le
                } else if self.match_char('<') {
                    TokenKind::Shl
                } else {
                    TokenKind::Lt
                }
            }
            '>' => {
                if self.match_char('=') {
                    TokenKind::Ge
                } else if self.match_char('>') {
                    TokenKind::Shr
                } else {
                    TokenKind::Gt
                }
            }
            '&' => {
                if self.match_char('&') {
                    TokenKind::And
                } else {
                    TokenKind::BitAnd
                }
            }
            '|' => {
                if self.match_char('|') {
                    TokenKind::Or
                } else {
                    TokenKind::BitOr
                }
            }

            // Strings
            '"' => self.string()?,

            // Numbers
            '0'..='9' => self.number(ch)?,

            // Hex numbers
            _ if ch == '0' && self.peek_char() == Some('x') => {
                self.advance(); // consume 'x'
                self.hex_number()?
            }

            // Identifiers and keywords
            _ if ch.is_alphabetic() || ch == '_' => self.identifier(ch),

            _ => {
                return Err(ParseError::new(
                    format!("Unexpected character: '{}'", ch),
                    line,
                    column,
                ));
            }
        };

        Ok(Token::new(kind, line, column))
    }

    /// Peek at the next token without consuming it
    pub fn peek_token(&mut self) -> Result<Token, ParseError> {
        // Save state
        let _saved_chars = self.source[self.current_pos..].char_indices().peekable();
        let saved_line = self.line;
        let saved_column = self.column;
        let saved_pos = self.current_pos;

        let token = self.next_token()?;

        // Restore state - we need to recreate the iterator
        self.chars = self.source.char_indices().peekable();
        // Skip to saved position
        while let Some((pos, _)) = self.chars.peek() {
            if *pos >= saved_pos {
                break;
            }
            self.chars.next();
        }
        self.line = saved_line;
        self.column = saved_column;
        self.current_pos = saved_pos;

        Ok(token)
    }

    fn advance(&mut self) -> Option<(usize, char)> {
        let result = self.chars.next();
        if let Some((_, ch)) = result {
            if ch == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
        }
        result
    }

    fn peek_char(&mut self) -> Option<char> {
        self.chars.peek().map(|(_, ch)| *ch)
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.peek_char() == Some(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.peek_char() {
                Some(' ') | Some('\t') | Some('\r') | Some('\n') => {
                    self.advance();
                }
                Some('/') => {
                    // Check for comments
                    let mut temp = self.chars.clone();
                    temp.next(); // consume '/'
                    match temp.peek() {
                        Some((_, '/')) => {
                            // Line comment
                            self.advance(); // consume first '/'
                            self.advance(); // consume second '/'
                            while let Some(ch) = self.peek_char() {
                                if ch == '\n' {
                                    break;
                                }
                                self.advance();
                            }
                        }
                        Some((_, '*')) => {
                            // Block comment
                            self.advance(); // consume '/'
                            self.advance(); // consume '*'
                            loop {
                                match self.advance() {
                                    Some((_, '*')) => {
                                        if self.match_char('/') {
                                            break;
                                        }
                                    }
                                    None => break,
                                    _ => {}
                                }
                            }
                        }
                        _ => break,
                    }
                }
                _ => break,
            }
        }
    }

    fn string(&mut self) -> Result<TokenKind, ParseError> {
        let mut value = String::new();
        let start_line = self.line;
        let start_col = self.column;

        loop {
            match self.advance() {
                Some((_, '"')) => break,
                Some((_, '\\')) => {
                    // Escape sequence
                    match self.advance() {
                        Some((_, 'n')) => value.push('\n'),
                        Some((_, 'r')) => value.push('\r'),
                        Some((_, 't')) => value.push('\t'),
                        Some((_, '\\')) => value.push('\\'),
                        Some((_, '"')) => value.push('"'),
                        Some((_, ch)) => {
                            return Err(ParseError::new(
                                format!("Invalid escape sequence: \\{}", ch),
                                self.line,
                                self.column,
                            ));
                        }
                        None => {
                            return Err(ParseError::new(
                                "Unterminated string",
                                start_line,
                                start_col,
                            ));
                        }
                    }
                }
                Some((_, ch)) => value.push(ch),
                None => {
                    return Err(ParseError::new(
                        "Unterminated string",
                        start_line,
                        start_col,
                    ));
                }
            }
        }

        Ok(TokenKind::String(value))
    }

    fn number(&mut self, first: char) -> Result<TokenKind, ParseError> {
        let mut value = String::from(first);
        let mut is_float = false;

        // Check for hex
        if first == '0' && self.peek_char() == Some('x') {
            self.advance(); // consume 'x'
            return self.hex_number();
        }

        while let Some(ch) = self.peek_char() {
            if ch.is_ascii_digit() {
                value.push(ch);
                self.advance();
            } else if ch == '.' && !is_float {
                // Check if next char is digit (to avoid 123.method())
                let mut temp = self.chars.clone();
                temp.next();
                if let Some((_, next)) = temp.peek() {
                    if next.is_ascii_digit() {
                        is_float = true;
                        value.push('.');
                        self.advance();
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            } else if ch == 'f' || ch == 'F' {
                // Float suffix
                self.advance();
                is_float = true;
                break;
            } else {
                break;
            }
        }

        if is_float {
            let f: f64 = value.parse().map_err(|_| {
                ParseError::new(format!("Invalid float: {}", value), self.line, self.column)
            })?;
            Ok(TokenKind::Float(f))
        } else {
            let i: i64 = value.parse().map_err(|_| {
                ParseError::new(format!("Invalid integer: {}", value), self.line, self.column)
            })?;
            Ok(TokenKind::Int(i))
        }
    }

    fn hex_number(&mut self) -> Result<TokenKind, ParseError> {
        let mut value = String::new();

        while let Some(ch) = self.peek_char() {
            if ch.is_ascii_hexdigit() {
                value.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        if value.is_empty() {
            return Err(ParseError::new(
                "Expected hex digits after 0x",
                self.line,
                self.column,
            ));
        }

        let i = i64::from_str_radix(&value, 16).map_err(|_| {
            ParseError::new(format!("Invalid hex number: 0x{}", value), self.line, self.column)
        })?;

        Ok(TokenKind::Int(i))
    }

    fn identifier(&mut self, first: char) -> TokenKind {
        let mut value = String::from(first);

        while let Some(ch) = self.peek_char() {
            if ch.is_alphanumeric() || ch == '_' {
                value.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        // Check for keywords
        match value.as_str() {
            "state" => TokenKind::State,
            "startup" => TokenKind::Startup,
            "shutdown" => TokenKind::Shutdown,
            "init" => TokenKind::Init,
            "exit" => TokenKind::Exit,
            "update" => TokenKind::Update,
            "start" => TokenKind::Start,
            "split" => TokenKind::Split,
            "reset" => TokenKind::Reset,
            "isLoading" => TokenKind::IsLoading,
            "gameTime" => TokenKind::GameTime,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "return" => TokenKind::Return,
            "var" => TokenKind::Var,
            "current" => TokenKind::Current,
            "old" => TokenKind::Old,
            "settings" => TokenKind::Settings,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "null" => TokenKind::Null,
            // Type keywords
            "bool" => TokenKind::Bool,
            "byte" => TokenKind::Byte,
            "sbyte" => TokenKind::SByte,
            "short" => TokenKind::Short,
            "ushort" => TokenKind::UShort,
            "int" => TokenKind::IntType,
            "uint" => TokenKind::UInt,
            "long" => TokenKind::Long,
            "ulong" => TokenKind::ULong,
            "float" => TokenKind::FloatType,
            "double" => TokenKind::Double,
            "string" => TokenKind::StringType,
            _ => TokenKind::Ident(value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_tokens() {
        let mut lexer = Lexer::new("( ) { } , ;");
        assert_eq!(lexer.next_token().unwrap().kind, TokenKind::LParen);
        assert_eq!(lexer.next_token().unwrap().kind, TokenKind::RParen);
        assert_eq!(lexer.next_token().unwrap().kind, TokenKind::LBrace);
        assert_eq!(lexer.next_token().unwrap().kind, TokenKind::RBrace);
        assert_eq!(lexer.next_token().unwrap().kind, TokenKind::Comma);
        assert_eq!(lexer.next_token().unwrap().kind, TokenKind::Semicolon);
    }

    #[test]
    fn test_numbers() {
        let mut lexer = Lexer::new("42 3.14 0xFF");
        assert_eq!(lexer.next_token().unwrap().kind, TokenKind::Int(42));
        assert_eq!(lexer.next_token().unwrap().kind, TokenKind::Float(3.14));
        assert_eq!(lexer.next_token().unwrap().kind, TokenKind::Int(255));
    }

    #[test]
    fn test_strings() {
        let mut lexer = Lexer::new(r#""hello" "world\n""#);
        assert_eq!(lexer.next_token().unwrap().kind, TokenKind::String("hello".to_string()));
        assert_eq!(lexer.next_token().unwrap().kind, TokenKind::String("world\n".to_string()));
    }

    #[test]
    fn test_keywords() {
        let mut lexer = Lexer::new("state start split if return");
        assert_eq!(lexer.next_token().unwrap().kind, TokenKind::State);
        assert_eq!(lexer.next_token().unwrap().kind, TokenKind::Start);
        assert_eq!(lexer.next_token().unwrap().kind, TokenKind::Split);
        assert_eq!(lexer.next_token().unwrap().kind, TokenKind::If);
        assert_eq!(lexer.next_token().unwrap().kind, TokenKind::Return);
    }

    #[test]
    fn test_operators() {
        let mut lexer = Lexer::new("== != && || < >");
        assert_eq!(lexer.next_token().unwrap().kind, TokenKind::Eq);
        assert_eq!(lexer.next_token().unwrap().kind, TokenKind::Ne);
        assert_eq!(lexer.next_token().unwrap().kind, TokenKind::And);
        assert_eq!(lexer.next_token().unwrap().kind, TokenKind::Or);
        assert_eq!(lexer.next_token().unwrap().kind, TokenKind::Lt);
        assert_eq!(lexer.next_token().unwrap().kind, TokenKind::Gt);
    }

    #[test]
    fn test_comments() {
        let mut lexer = Lexer::new("42 // comment\n43 /* block */ 44");
        assert_eq!(lexer.next_token().unwrap().kind, TokenKind::Int(42));
        assert_eq!(lexer.next_token().unwrap().kind, TokenKind::Int(43));
        assert_eq!(lexer.next_token().unwrap().kind, TokenKind::Int(44));
    }
}
