//! Statement AST nodes.
//!
//! Statements are the building blocks of function bodies and blocks.
//! In Razen, most things are expressions, so many "statement" forms are
//! thin wrappers around an expression followed by an optional semicolon.

use crate::expr::Expr;
use crate::ident::Ident;
use crate::ops::CompoundOp;
use crate::pat::Pattern;
use crate::span::Span;
use crate::types::TypeExpr;

/// A statement node.
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// Immutable binding: `name := expr` or `name: Type := expr`.
    Let {
        pattern: Pattern,
        ty: Option<TypeExpr>,
        value: Expr,
        span: Span,
    },

    /// Mutable binding: `mut name: Type = expr`.
    LetMut {
        name: Ident,
        ty: TypeExpr,
        value: Expr,
        span: Span,
    },

    /// Constant: `const NAME: Type = expr`.
    Const {
        name: Ident,
        ty: TypeExpr,
        value: Expr,
        span: Span,
    },

    /// Shared binding: `shared name: Type = expr` or `shared name = expr`.
    Shared {
        name: Ident,
        ty: Option<TypeExpr>,
        value: Expr,
        span: Span,
    },

    /// An expression used as a statement (value discarded unless it's the tail).
    Expr {
        expr: Expr,
        span: Span,
    },

    /// Early return: `ret expr`.
    Return {
        value: Option<Expr>,
        span: Span,
    },

    /// Break: `break` / `break 'label` / `break value`.
    Break {
        label: Option<String>,
        value: Option<Expr>,
        span: Span,
    },

    /// Next (continue): `next` / `next 'label`.
    Next {
        label: Option<String>,
        span: Span,
    },

    /// Defer: `defer expr`.
    Defer {
        body: Expr,
        span: Span,
    },

    /// Guard: `guard condition else { ... }`.
    Guard {
        condition: Expr,
        else_body: Expr,
        span: Span,
    },

    /// Assignment: `target = value`.
    Assign {
        target: Expr,
        value: Expr,
        span: Span,
    },

    /// Compound assignment: `target += value`.
    CompoundAssign {
        target: Expr,
        op: CompoundOp,
        value: Expr,
        span: Span,
    },

    /// An item declaration used at statement level (e.g. a nested function).
    Item {
        item: Box<crate::item::Item>,
        span: Span,
    },
}

impl Stmt {
    /// Returns the span of this statement.
    pub fn span(&self) -> Span {
        match self {
            Stmt::Let { span, .. }
            | Stmt::LetMut { span, .. }
            | Stmt::Const { span, .. }
            | Stmt::Shared { span, .. }
            | Stmt::Expr { span, .. }
            | Stmt::Return { span, .. }
            | Stmt::Break { span, .. }
            | Stmt::Next { span, .. }
            | Stmt::Defer { span, .. }
            | Stmt::Guard { span, .. }
            | Stmt::Assign { span, .. }
            | Stmt::CompoundAssign { span, .. }
            | Stmt::Item { span, .. } => *span,
        }
    }
}
