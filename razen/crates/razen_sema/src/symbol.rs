//! Symbol Table and Definition IDs for Semantic Analysis.

use std::collections::HashMap;
use razen_ast::span::Span;
use razen_ast::ident::Ident;

/// A unique identifier for a resolved declaration (variable, function, struct, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DefId(pub usize);

/// The kind of symbol being defined.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolKind {
    /// A local or formal parameter binding (`n`, `user`).
    Variable { is_mut: bool },
    /// A constant (`MAX_SCORE`).
    Const,
    /// A shared state binding (`cache`).
    Shared,
    /// A function defined via `act`.
    Function,
    /// A user-defined struct.
    Struct,
    /// A user-defined enum.
    Enum,
    /// An enum variant.
    Variant,
    /// A trait definition.
    Trait,
    /// A type alias (`alias T = ...`).
    TypeAlias,
    /// A module imported via `use`.
    Module,
}

/// A resolved symbol.
#[derive(Debug, Clone)]
pub struct Symbol {
    pub id: DefId,
    pub name: String,
    pub kind: SymbolKind,
    /// The span of the definition (e.g., the span of the identifier in `x := 10`).
    pub span: Span,
}

/// The global symbol table holding all definitions across the compilation unit.
#[derive(Debug, Default)]
pub struct SymbolTable {
    symbols: Vec<Symbol>,
    /// Optimization: secondary index to quickly find a symbol by span (e.g. for IDE support).
    span_index: HashMap<Span, DefId>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            symbols: Vec::new(),
            span_index: HashMap::new(),
        }
    }

    /// Register a new symbol in the symbol table, yielding its fresh `DefId`.
    pub fn add(&mut self, ident: &Ident, kind: SymbolKind) -> DefId {
        let id = DefId(self.symbols.len());
        let symbol = Symbol {
            id,
            name: ident.name.clone(),
            kind,
            span: ident.span,
        };
        self.symbols.push(symbol);
        self.span_index.insert(ident.span, id);
        id
    }

    /// Retrieve a symbol by its ID.
    pub fn get(&self, id: DefId) -> Option<&Symbol> {
        self.symbols.get(id.0)
    }
}
