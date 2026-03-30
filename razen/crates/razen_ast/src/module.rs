//! Module root AST node.
//!
//! A `Module` is the top-level AST node produced by the parser.
//! It represents a single Razen source file.

use crate::item::Item;
use crate::span::Span;

/// The root AST node representing a single Razen source file.
#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    /// All top-level items in the file, in source order.
    pub items: Vec<Item>,
    pub span: Span,
}

impl Module {
    pub fn new(items: Vec<Item>, span: Span) -> Self {
        Self { items, span }
    }
}
