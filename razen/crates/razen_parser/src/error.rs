//! Parser error types.

use razen_lexer::{Span, TokenKind};
use std::fmt;

/// A parse error with source location and descriptive message.
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
    pub expected: Vec<String>,
}

impl ParseError {
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
            expected: Vec::new(),
        }
    }

    pub fn expected(message: impl Into<String>, span: Span, expected: Vec<String>) -> Self {
        Self {
            message: message.into(),
            span,
            expected,
        }
    }

    pub fn unexpected_token(got: &TokenKind, span: Span) -> Self {
        Self {
            message: format!("unexpected token: {:?}", got),
            span,
            expected: Vec::new(),
        }
    }

    pub fn unexpected_eof(span: Span) -> Self {
        Self {
            message: "unexpected end of input".to_string(),
            span,
            expected: Vec::new(),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "error at {}..{}: {}",
            self.span.start, self.span.end, self.message
        )?;
        if !self.expected.is_empty() {
            write!(f, " (expected: {})", self.expected.join(", "))?;
        }
        Ok(())
    }
}

impl std::error::Error for ParseError {}
