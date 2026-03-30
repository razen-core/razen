//! # Razen Semantic Analysis (`razen_sema`)
//!
//! Performs Name Resolution, Scope Tracking, and (eventually) Type Checking.

pub mod error;
pub mod symbol;
pub mod scope;
pub mod resolve;

#[cfg(test)]
mod tests;

pub use error::SemanticError;
pub use symbol::{DefId, Symbol, SymbolKind, SymbolTable};
pub use scope::{Scope, ScopeId, ScopeKind, Environment};
pub use resolve::{SemanticModel, Resolver};

use razen_ast::module::Module;

/// Analyze a parsed AST module, producing a complete SemanticModel.
pub fn analyze(module: &Module) -> SemanticModel {
    let resolver = Resolver::new();
    resolver.resolve(module)
}
