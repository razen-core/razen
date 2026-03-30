//! # Razen AST
//!
//! This crate defines the complete Abstract Syntax Tree for the Razen programming language.
//!
//! The AST is produced by `razen_parser` and consumed by downstream compiler phases
//! such as `razen_sema` (semantic analysis) and `razen_mir` (MIR lowering).
//!
//! ## Design Principles
//!
//! - Every node carries a `Span` for diagnostics.
//! - Literals store raw text; numeric validation is deferred to semantic analysis.
//! - The tree is fully owned — no reference lifetimes — so it can be freely moved
//!   between compiler phases.

pub mod span;
pub mod ident;
pub mod lit;
pub mod ops;
pub mod types;
pub mod pat;
pub mod expr;
pub mod stmt;
pub mod item;
pub mod module;

// Re-export key types at the crate root for convenience.
pub use span::Span;
pub use ident::Ident;
pub use lit::Literal;
pub use ops::{BinOp, UnaryOp, CompoundOp};
pub use types::TypeExpr;
pub use pat::Pattern;
pub use expr::Expr;
pub use stmt::Stmt;
pub use item::{Item, Attribute, Visibility};
pub use module::Module;
