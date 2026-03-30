//! Type expression AST nodes.
//!
//! These represent type annotations in source code, not resolved types.

use crate::ident::Ident;
use crate::span::Span;

/// A type expression as written in source code.
#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpr {
    /// A simple named type, e.g. `int`, `str`, `User`.
    Named {
        name: Ident,
        span: Span,
    },

    /// A generic type application, e.g. `vec[int]`, `map[str, int]`, `result[T, E]`.
    Generic {
        name: Ident,
        args: Vec<TypeExpr>,
        span: Span,
    },

    /// A fixed-size array type, e.g. `[int; 5]`, `[float; 3]`.
    Array {
        element: Box<TypeExpr>,
        size: Box<crate::expr::Expr>,
        span: Span,
    },

    /// A tuple type, e.g. `(int, str, bool)`.
    Tuple {
        elements: Vec<TypeExpr>,
        span: Span,
    },

    /// A closure / function pointer type, e.g. `|int, int| -> int`.
    Closure {
        params: Vec<TypeExpr>,
        ret: Box<TypeExpr>,
        span: Span,
    },

    /// The `void` type (no return value).
    Void { span: Span },

    /// The `never` type (function never returns).
    Never { span: Span },

    /// The `Self` type (inside impl/trait blocks).
    SelfType { span: Span },

    /// A reference/pointer type used in `unsafe` contexts.
    Ref {
        inner: Box<TypeExpr>,
        span: Span,
    },

    /// Inferred type (no annotation provided). Used internally by the parser
    /// when a type annotation is optional and omitted.
    Inferred { span: Span },
}

impl TypeExpr {
    /// Returns the span of this type expression.
    pub fn span(&self) -> Span {
        match self {
            TypeExpr::Named { span, .. }
            | TypeExpr::Generic { span, .. }
            | TypeExpr::Array { span, .. }
            | TypeExpr::Tuple { span, .. }
            | TypeExpr::Closure { span, .. }
            | TypeExpr::Void { span }
            | TypeExpr::Never { span }
            | TypeExpr::SelfType { span }
            | TypeExpr::Ref { span, .. }
            | TypeExpr::Inferred { span } => *span,
        }
    }
}
