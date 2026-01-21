//! Error types for ASL parsing

use std::fmt;

/// Result type for ASL operations
pub type AslResult<T> = Result<T, AslError>;

/// Error type for ASL parsing and conversion
#[derive(Debug, Clone)]
pub struct AslError {
    pub kind: AslErrorKind,
    pub message: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

/// The kind of ASL error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AslErrorKind {
    /// Lexer error - invalid token or character
    LexerError,
    /// Parser error - unexpected token or syntax error
    ParseError,
    /// Conversion error - cannot convert to GameData
    ConversionError,
    /// Unsupported feature
    UnsupportedFeature,
}

impl AslError {
    /// Create a new lexer error
    pub fn lexer(message: impl Into<String>, line: usize, column: usize) -> Self {
        Self {
            kind: AslErrorKind::LexerError,
            message: message.into(),
            line: Some(line),
            column: Some(column),
        }
    }

    /// Create a new parser error
    pub fn parser(message: impl Into<String>) -> Self {
        Self {
            kind: AslErrorKind::ParseError,
            message: message.into(),
            line: None,
            column: None,
        }
    }

    /// Create a new parser error with location
    pub fn parser_at(message: impl Into<String>, line: usize, column: usize) -> Self {
        Self {
            kind: AslErrorKind::ParseError,
            message: message.into(),
            line: Some(line),
            column: Some(column),
        }
    }

    /// Create a new conversion error
    pub fn conversion(message: impl Into<String>) -> Self {
        Self {
            kind: AslErrorKind::ConversionError,
            message: message.into(),
            line: None,
            column: None,
        }
    }

    /// Create an unsupported feature error
    pub fn unsupported(message: impl Into<String>) -> Self {
        Self {
            kind: AslErrorKind::UnsupportedFeature,
            message: message.into(),
            line: None,
            column: None,
        }
    }
}

impl fmt::Display for AslError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind_str = match self.kind {
            AslErrorKind::LexerError => "Lexer error",
            AslErrorKind::ParseError => "Parse error",
            AslErrorKind::ConversionError => "Conversion error",
            AslErrorKind::UnsupportedFeature => "Unsupported feature",
        };

        match (self.line, self.column) {
            (Some(line), Some(col)) => {
                write!(f, "{} at line {}, column {}: {}", kind_str, line, col, self.message)
            }
            (Some(line), None) => {
                write!(f, "{} at line {}: {}", kind_str, line, self.message)
            }
            _ => write!(f, "{}: {}", kind_str, self.message),
        }
    }
}

impl std::error::Error for AslError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer_error() {
        let err = AslError::lexer("unexpected character", 5, 10);
        assert_eq!(err.kind, AslErrorKind::LexerError);
        assert_eq!(err.line, Some(5));
        assert_eq!(err.column, Some(10));
        assert!(err.to_string().contains("line 5"));
        assert!(err.to_string().contains("column 10"));
    }

    #[test]
    fn test_parser_error() {
        let err = AslError::parser("unexpected token");
        assert_eq!(err.kind, AslErrorKind::ParseError);
        assert!(err.line.is_none());
    }

    #[test]
    fn test_parser_error_at() {
        let err = AslError::parser_at("expected '{'", 10, 5);
        assert_eq!(err.kind, AslErrorKind::ParseError);
        assert_eq!(err.line, Some(10));
    }

    #[test]
    fn test_conversion_error() {
        let err = AslError::conversion("unknown engine type");
        assert_eq!(err.kind, AslErrorKind::ConversionError);
    }

    #[test]
    fn test_unsupported_error() {
        let err = AslError::unsupported("complex expressions not supported");
        assert_eq!(err.kind, AslErrorKind::UnsupportedFeature);
    }

    #[test]
    fn test_display() {
        let err = AslError::lexer("bad char", 1, 1);
        let s = format!("{}", err);
        assert!(s.contains("Lexer error"));
        assert!(s.contains("bad char"));
    }
}
