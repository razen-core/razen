//! Literal value nodes.

use crate::span::Span;

/// A literal value in source code.
///
/// Raw text is preserved so the lexer does not need to validate numeric ranges.
/// Full validation is performed during semantic analysis.
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    /// Integer literal, e.g. `42`, `0xFF`, `255u8`.
    /// The string includes any prefix (`0x`, `0b`, `0o`) and suffix (`u8`, `i32`, etc.).
    Int {
        raw: String,
        span: Span,
    },
    /// Floating-point literal, e.g. `3.14`, `0.5f32`.
    Float {
        raw: String,
        span: Span,
    },
    /// String literal (contents only, quotes stripped by lexer).
    Str {
        value: String,
        span: Span,
    },
    /// Character literal.
    Char {
        value: char,
        span: Span,
    },
    /// Boolean literal.
    Bool {
        value: bool,
        span: Span,
    },
}

impl Literal {
    /// Returns the span of this literal.
    pub fn span(&self) -> Span {
        match self {
            Literal::Int { span, .. }
            | Literal::Float { span, .. }
            | Literal::Str { span, .. }
            | Literal::Char { span, .. }
            | Literal::Bool { span, .. } => *span,
        }
    }
}
