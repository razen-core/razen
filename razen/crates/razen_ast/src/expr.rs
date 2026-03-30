//! Expression AST nodes.
//!
//! Expressions are the core of the Razen language — almost everything is an expression,
//! including `if`, `match`, `loop`, and blocks.

use crate::ident::Ident;
use crate::item::MatchArm;
use crate::lit::Literal;
use crate::ops::{BinOp, CompoundOp, UnaryOp};
use crate::pat::Pattern;
use crate::span::Span;
use crate::stmt::Stmt;
use crate::types::TypeExpr;

/// An expression node.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// A literal value: `42`, `"hello"`, `true`, etc.
    Literal {
        lit: Literal,
        span: Span,
    },

    /// An identifier reference: `x`, `user_name`.
    Ident {
        ident: Ident,
        span: Span,
    },

    /// A binary operation: `a + b`, `x && y`, `0..10`.
    Binary {
        left: Box<Expr>,
        op: BinOp,
        right: Box<Expr>,
        span: Span,
    },

    /// A unary (prefix) operation: `-x`, `!flag`, `~bits`.
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
        span: Span,
    },

    /// Field access: `user.name`, `point.x`.
    Field {
        object: Box<Expr>,
        field: Ident,
        span: Span,
    },

    /// Method call: `text.len()`, `vec.push(item)`.
    MethodCall {
        object: Box<Expr>,
        method: Ident,
        args: Vec<Expr>,
        span: Span,
    },

    /// Function / constructor call: `greet("Alice")`, `User { ... }`.
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },

    /// Index access: `arr[0]`, `matrix[i]`.
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },

    /// A block expression: `{ stmt; stmt; expr }`.
    Block {
        stmts: Vec<Stmt>,
        /// The final expression whose value is the block's value (may be absent).
        tail: Option<Box<Expr>>,
        span: Span,
    },

    /// If expression: `if cond { ... } else { ... }`.
    If {
        condition: Box<Expr>,
        then_block: Box<Expr>,
        else_block: Option<Box<Expr>>,
        span: Span,
    },

    /// If-let expression: `if let pattern = expr { ... }`.
    IfLet {
        pattern: Pattern,
        value: Box<Expr>,
        then_block: Box<Expr>,
        else_block: Option<Box<Expr>>,
        span: Span,
    },

    /// Match expression.
    Match {
        subject: Box<Expr>,
        arms: Vec<MatchArm>,
        span: Span,
    },

    /// Loop expression (universal: condition, range, infinite).
    Loop {
        /// Optional label, e.g. `'outer`.
        label: Option<String>,
        kind: LoopKind,
        body: Box<Expr>,
        /// Optional `else` block for loop-as-expression.
        else_block: Option<Box<Expr>>,
        span: Span,
    },

    /// Loop-let expression: `loop let some(x) = iter.next() { ... }`.
    LoopLet {
        label: Option<String>,
        pattern: Pattern,
        value: Box<Expr>,
        body: Box<Expr>,
        span: Span,
    },

    /// A closure / lambda: `|x: int| x * 2`.
    Closure {
        params: Vec<ClosureParam>,
        body: Box<Expr>,
        span: Span,
    },

    /// A tuple literal: `(1, "hello", true)`.
    Tuple {
        elements: Vec<Expr>,
        span: Span,
    },

    /// A vec literal: `vec[1, 2, 3]`.
    Vec {
        elements: Vec<Expr>,
        span: Span,
    },

    /// A map literal: `map["key": value, ...]`.
    Map {
        entries: Vec<(Expr, Expr)>,
        span: Span,
    },

    /// A set literal: `set[1, 2, 3]`.
    Set {
        elements: Vec<Expr>,
        span: Span,
    },

    /// An array literal: `[1, 2, 3, 4, 5]`.
    Array {
        elements: Vec<Expr>,
        span: Span,
    },

    /// A struct literal: `User { id: 1, name: "Alice" }`.
    StructLiteral {
        name: Box<Expr>,
        fields: Vec<StructLiteralField>,
        /// Struct update syntax: `..source`.
        spread: Option<Box<Expr>>,
        span: Span,
    },

    /// Type cast: `score as float`.
    Cast {
        expr: Box<Expr>,
        ty: TypeExpr,
        span: Span,
    },

    /// Type check: `value is int`.
    TypeCheck {
        expr: Box<Expr>,
        ty: TypeExpr,
        span: Span,
    },

    /// Error propagation: `expr?`.
    Try {
        expr: Box<Expr>,
        span: Span,
    },

    /// Await: `expr.await`.
    Await {
        expr: Box<Expr>,
        span: Span,
    },

    /// Assignment: `x = 42`.
    Assign {
        target: Box<Expr>,
        value: Box<Expr>,
        span: Span,
    },

    /// Compound assignment: `x += 1`.
    CompoundAssign {
        target: Box<Expr>,
        op: CompoundOp,
        value: Box<Expr>,
        span: Span,
    },

    /// `break` with optional label and value.
    Break {
        label: Option<String>,
        value: Option<Box<Expr>>,
        span: Span,
    },

    /// `next` (continue) with optional label.
    Next {
        label: Option<String>,
        span: Span,
    },

    /// `ret` (early return) with optional value.
    Return {
        value: Option<Box<Expr>>,
        span: Span,
    },

    /// Fork expression for structured concurrency.
    Fork {
        kind: ForkKind,
        span: Span,
    },

    /// A path expression for enum variants, module access, etc.: `Direction.North`.
    Path {
        segments: Vec<Ident>,
        span: Span,
    },

    /// A grouped expression `(expr)`.
    Paren {
        inner: Box<Expr>,
        span: Span,
    },

    /// Tensor literal: `tensor[1.0, 2.0, 3.0]`.
    Tensor {
        elements: Vec<Expr>,
        span: Span,
    },

    /// `unsafe { ... }` block.
    Unsafe {
        body: Box<Expr>,
        span: Span,
    },

    /// Placeholder for features not yet fully implemented.
    /// Allows the parser to make forward progress.
    Placeholder {
        description: String,
        span: Span,
    },
}

/// The kind of loop.
#[derive(Debug, Clone, PartialEq)]
pub enum LoopKind {
    /// Infinite loop: `loop { ... }`.
    Infinite,
    /// Condition loop (while-style): `loop condition { ... }`.
    While { condition: Box<Expr> },
    /// Range / collection loop: `loop var in iterable { ... }`.
    ForIn {
        binding: Pattern,
        iterable: Box<Expr>,
    },
}

/// A fork expression kind.
#[derive(Debug, Clone, PartialEq)]
pub enum ForkKind {
    /// `fork { expr1, expr2, ... }` — run multiple tasks concurrently.
    Block { tasks: Vec<ForkTask> },
    /// `fork loop var in collection { expr }` — spawn per-item tasks.
    Loop {
        binding: Pattern,
        iterable: Box<Expr>,
        body: Box<Expr>,
    },
}

/// A single task inside a `fork { ... }` block.
#[derive(Debug, Clone, PartialEq)]
pub struct ForkTask {
    /// Optional named binding: `user_data <- expr`.
    pub binding: Option<Ident>,
    pub expr: Expr,
    pub span: Span,
}

/// A parameter in a closure.
#[derive(Debug, Clone, PartialEq)]
pub struct ClosureParam {
    pub name: Ident,
    pub ty: Option<TypeExpr>,
    pub span: Span,
}

/// A field in a struct literal.
#[derive(Debug, Clone, PartialEq)]
pub struct StructLiteralField {
    pub name: Ident,
    pub value: Expr,
    pub span: Span,
}

impl Expr {
    /// Returns the span of this expression.
    pub fn span(&self) -> Span {
        match self {
            Expr::Literal { span, .. }
            | Expr::Ident { span, .. }
            | Expr::Binary { span, .. }
            | Expr::Unary { span, .. }
            | Expr::Field { span, .. }
            | Expr::MethodCall { span, .. }
            | Expr::Call { span, .. }
            | Expr::Index { span, .. }
            | Expr::Block { span, .. }
            | Expr::If { span, .. }
            | Expr::IfLet { span, .. }
            | Expr::Match { span, .. }
            | Expr::Loop { span, .. }
            | Expr::LoopLet { span, .. }
            | Expr::Closure { span, .. }
            | Expr::Tuple { span, .. }
            | Expr::Vec { span, .. }
            | Expr::Map { span, .. }
            | Expr::Set { span, .. }
            | Expr::Array { span, .. }
            | Expr::StructLiteral { span, .. }
            | Expr::Cast { span, .. }
            | Expr::TypeCheck { span, .. }
            | Expr::Try { span, .. }
            | Expr::Await { span, .. }
            | Expr::Assign { span, .. }
            | Expr::CompoundAssign { span, .. }
            | Expr::Break { span, .. }
            | Expr::Next { span, .. }
            | Expr::Return { span, .. }
            | Expr::Fork { span, .. }
            | Expr::Path { span, .. }
            | Expr::Paren { span, .. }
            | Expr::Tensor { span, .. }
            | Expr::Unsafe { span, .. }
            | Expr::Placeholder { span, .. } => *span,
        }
    }
}
