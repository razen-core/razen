//! Pattern AST nodes.
//!
//! Patterns appear in `match` arms, `if let`, `loop let`, variable destructuring,
//! and function parameters.

use crate::expr::Expr;
use crate::ident::Ident;
use crate::lit::Literal;
use crate::span::Span;

/// A pattern used for matching / destructuring.
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// Wildcard `_` — matches anything, binds nothing.
    Wildcard { span: Span },

    /// A simple binding, e.g. `x`, `name`.
    Binding {
        name: Ident,
        span: Span,
    },

    /// A literal pattern, e.g. `42`, `"hello"`, `true`.
    Literal {
        lit: Literal,
        span: Span,
    },

    /// A tuple pattern, e.g. `(a, b, _)`.
    Tuple {
        elements: Vec<Pattern>,
        span: Span,
    },

    /// A struct destructuring pattern, e.g. `{ name, age, _ }`.
    Struct {
        fields: Vec<StructPatternField>,
        has_rest: bool,
        span: Span,
    },

    /// An enum variant pattern with positional data, e.g. `Shape.Circle(r)`.
    EnumPositional {
        path: Vec<Ident>,
        args: Vec<Pattern>,
        span: Span,
    },

    /// An enum variant pattern with named fields, e.g. `Event.Click { x, y }`.
    EnumNamed {
        path: Vec<Ident>,
        fields: Vec<StructPatternField>,
        has_rest: bool,
        span: Span,
    },

    /// A unit enum variant pattern, e.g. `Direction.North`.
    EnumUnit {
        path: Vec<Ident>,
        span: Span,
    },

    /// `some(pattern)` — option unwrap pattern.
    Some {
        inner: Box<Pattern>,
        span: Span,
    },

    /// `none` — option empty pattern.
    None { span: Span },

    /// `ok(pattern)` — result success pattern.
    Ok {
        inner: Box<Pattern>,
        span: Span,
    },

    /// `err(pattern)` — result error pattern.
    Err {
        inner: Box<Pattern>,
        span: Span,
    },

    /// A range pattern, e.g. `0..=9`, `'a'..='z'`.
    Range {
        start: Box<Expr>,
        end: Box<Expr>,
        inclusive: bool,
        span: Span,
    },

    /// An or-pattern, e.g. `1 | 2 | 3`.
    Or {
        patterns: Vec<Pattern>,
        span: Span,
    },

    /// A tuple-struct / newtype pattern, e.g. `UserId(n)`.
    TupleStruct {
        name: Ident,
        fields: Vec<Pattern>,
        span: Span,
    },
}

/// A single field in a struct destructuring pattern.
#[derive(Debug, Clone, PartialEq)]
pub struct StructPatternField {
    /// The field name being matched.
    pub name: Ident,
    /// Optional rename binding, e.g. `name: alias`.
    pub rename: Option<Ident>,
    /// Optional nested pattern, e.g. `key: some(v)`.
    pub pattern: Option<Pattern>,
    pub span: Span,
}

impl Pattern {
    /// Returns the span of this pattern.
    pub fn span(&self) -> Span {
        match self {
            Pattern::Wildcard { span }
            | Pattern::Binding { span, .. }
            | Pattern::Literal { span, .. }
            | Pattern::Tuple { span, .. }
            | Pattern::Struct { span, .. }
            | Pattern::EnumPositional { span, .. }
            | Pattern::EnumNamed { span, .. }
            | Pattern::EnumUnit { span, .. }
            | Pattern::Some { span, .. }
            | Pattern::None { span }
            | Pattern::Ok { span, .. }
            | Pattern::Err { span, .. }
            | Pattern::Range { span, .. }
            | Pattern::Or { span, .. }
            | Pattern::TupleStruct { span, .. } => *span,
        }
    }
}
