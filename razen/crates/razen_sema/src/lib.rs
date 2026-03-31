//! # Razen Semantic Analysis (`razen_sema`)
//!
//! This crate implements the complete semantic analysis pipeline for Razen:
//!
//! 1. **Prelude injection** — built-in names (`println`, `some`, `ok`, `err`,
//!    `none`, primitive types, etc.) are pre-populated into the module scope
//!    before name resolution runs.
//!
//! 2. **Name resolution** (`Resolver`) — two-pass traversal that builds the
//!    symbol table, tracks lexical scopes, and maps every identifier span to
//!    its `DefId`.
//!
//! 3. **Type checking** (`TypeChecker`) — bidirectional inference that assigns
//!    a `Ty` to every expression, binding, parameter, and return statement.
//!    Populates `SemanticModel::type_env` and `SemanticModel::expr_types`.
//!
//! 4. **Mutability checking** (`MutabilityChecker`) — verifies that immutable
//!    bindings are never reassigned.
//!
//! The single public entry point is [`analyze`].

pub mod check;
pub mod error;
pub mod infer;
pub mod mutability;
pub mod prelude;
pub mod resolve;
pub mod scope;
pub mod symbol;
pub mod ty;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Re-exports
// ---------------------------------------------------------------------------

pub use check::TypeChecker;
pub use error::SemanticError;
pub use mutability::MutabilityChecker;
pub use resolve::{Resolver, SemanticModel};
pub use scope::{Environment, Scope, ScopeId, ScopeKind};
pub use symbol::{DefId, Symbol, SymbolKind, SymbolTable};
pub use ty::{InferVarId, Ty};

use razen_ast::module::Module;
use scope::Environment as Env;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

/// Run the complete semantic analysis pipeline on a parsed module.
///
/// # Pipeline
///
/// 1. Create a fresh `SemanticModel`.
/// 2. Enter a module-level scope.
/// 3. Inject all prelude names (built-ins) into the scope.
/// 4. Run the two-pass name resolver (`Resolver::resolve_with_model`).
/// 5. Run the type checker (`TypeChecker::check_module`).
/// 6. Run the mutability checker (`MutabilityChecker::check_module`).
/// 7. Return the fully populated `SemanticModel`.
///
/// # Returns
///
/// A [`SemanticModel`] whose `errors` field contains every diagnostic
/// discovered across all four phases.  A non-empty `errors` vec does **not**
/// mean compilation must be aborted — callers may choose to continue with
/// best-effort output for IDE features such as completion and hover.
pub fn analyze(module: &Module) -> SemanticModel {
    // ── Step 1: Create the model and enter the module scope ─────────────────
    let mut model = SemanticModel {
        symbol_table: SymbolTable::new(),
        environment: Env::new(),
        resolutions: HashMap::new(),
        type_env: HashMap::new(),
        expr_types: HashMap::new(),
        errors: Vec::new(),
    };

    model.environment.enter_scope(ScopeKind::Module);

    // ── Step 2: Inject prelude ───────────────────────────────────────────────
    prelude::inject_prelude(
        &mut model.environment,
        &mut model.symbol_table,
        &mut model.type_env,
    );

    // ── Step 3: Name resolution ──────────────────────────────────────────────
    // `resolve_with_model` reuses the pre-populated model so prelude names are
    // already visible during the declaration and body passes.
    let resolver = Resolver::new();
    let mut model = resolver.resolve_with_model(module, model);

    // ── Step 4: Type checking ────────────────────────────────────────────────
    {
        let mut checker = TypeChecker::new(&mut model);
        checker.check_module(module);
    }

    // ── Step 5: Mutability checking ──────────────────────────────────────────
    {
        let mut mut_checker = MutabilityChecker::new(&mut model);
        mut_checker.check_module(module);
    }

    model
}
