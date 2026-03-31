//! Name Resolution pass.
//!
//! Traverses the AST, builds the symbol table, tracks scopes, and maps
//! every identifier span to its true `DefId`.

use std::collections::HashMap;

use razen_ast::expr::{Expr, ForkKind, LoopKind};
use razen_ast::item::{FnDef, Item};
use razen_ast::module::Module;
use razen_ast::pat::Pattern;
use razen_ast::span::Span;
use razen_ast::stmt::Stmt;
use razen_ast::types::TypeExpr;

use crate::error::SemanticError;
use crate::scope::{Environment, ScopeKind};
use crate::symbol::{DefId, SymbolKind, SymbolTable};
use crate::ty::Ty;

/// The combined output of all semantic analysis phases.
///
/// Populated in three passes:
///   1. **Name resolution** (`Resolver`) — fills `resolutions`, `symbol_table`, `environment`.
///   2. **Type checking** (`TypeChecker`) — fills `type_env`, `expr_types`.
///   3. **Mutability checking** (`MutabilityChecker`) — may add more entries to `errors`.
#[derive(Debug)]
pub struct SemanticModel {
    pub symbol_table: SymbolTable,
    pub environment: Environment,
    /// Maps the `Span` of an identifier usage to its resolved `DefId`.
    pub resolutions: HashMap<Span, DefId>,
    /// Maps every binding `DefId` to its resolved `Ty`.
    /// Filled by the type-checker; may be empty before that phase runs.
    pub type_env: HashMap<DefId, Ty>,
    /// Maps the `Span` of every expression to its resolved `Ty`.
    /// Filled by the type-checker.
    pub expr_types: HashMap<Span, Ty>,
    pub errors: Vec<SemanticError>,
}

/// The Name Resolver.
pub struct Resolver {
    model: SemanticModel,
}

impl Resolver {
    pub fn new() -> Self {
        Self {
            model: SemanticModel {
                symbol_table: SymbolTable::new(),
                environment: Environment::new(),
                resolutions: HashMap::new(),
                type_env: HashMap::new(),
                expr_types: HashMap::new(),
                errors: Vec::new(),
            },
        }
    }

    /// Resolve an entire module using a freshly created `SemanticModel`.
    /// The prelude is NOT injected here; call `resolve_with_model` if you
    /// need a pre-populated model (e.g. with prelude names already in scope).
    pub fn resolve(self, module: &Module) -> SemanticModel {
        let model = SemanticModel {
            symbol_table: SymbolTable::new(),
            environment: Environment::new(),
            resolutions: HashMap::new(),
            type_env: HashMap::new(),
            expr_types: HashMap::new(),
            errors: Vec::new(),
        };
        self.resolve_with_model(module, model)
    }

    /// Resolve a module using an existing `SemanticModel`.
    ///
    /// This is the preferred entry point when the prelude has already been
    /// injected into `model.environment` and `model.type_env`.
    pub fn resolve_with_model(mut self, module: &Module, model: SemanticModel) -> SemanticModel {
        self.model = model;

        // The caller is responsible for having entered a module-level scope
        // (or we create one here if the environment is empty).
        let needs_scope = self.model.environment.active_depth() == 0;
        if needs_scope {
            self.model.environment.enter_scope(ScopeKind::Module);
        }

        // Pass 1: Global declarations — order-independent at module level.
        for item in &module.items {
            self.declare_item(item);
        }

        // Pass 2: Bodies — visit all items now that names are declared.
        for item in &module.items {
            self.visit_item(item);
        }

        if needs_scope {
            self.model.environment.exit_scope();
        }

        self.model
    }

    fn push_error(&mut self, err: SemanticError) {
        self.model.errors.push(err);
    }

    // --- Declarations ---

    /// Define a top-level item in the current scope (module scope).
    fn declare_item(&mut self, item: &Item) {
        match item {
            Item::Function(fndef) => {
                let id = self
                    .model
                    .symbol_table
                    .add(&fndef.name, SymbolKind::Function);
                self.model.environment.define(fndef.name.name.clone(), id);
            }
            Item::Struct(sdef) => {
                let id = self.model.symbol_table.add(&sdef.name, SymbolKind::Struct);
                self.model.environment.define(sdef.name.name.clone(), id);
            }
            Item::Enum(edef) => {
                let id = self.model.symbol_table.add(&edef.name, SymbolKind::Enum);
                self.model.environment.define(edef.name.name.clone(), id);
                // In Razen, it's common for enum variants to be namespaced under the Enum,
                // but some languages put them in the module scope.
                // Given `Use Enum { Variant }` we treat them as namespaced, so we don't bind them globally here.
            }
            Item::Trait(tdef) => {
                let id = self.model.symbol_table.add(&tdef.name, SymbolKind::Trait);
                self.model.environment.define(tdef.name.name.clone(), id);
            }
            Item::TypeAlias(tdef) => {
                let id = self
                    .model
                    .symbol_table
                    .add(&tdef.name, SymbolKind::TypeAlias);
                self.model.environment.define(tdef.name.name.clone(), id);
            }
            Item::Const(cdef) => {
                let id = self.model.symbol_table.add(&cdef.name, SymbolKind::Const);
                self.model.environment.define(cdef.name.name.clone(), id);
            }
            Item::Shared(sdef) => {
                let id = self.model.symbol_table.add(&sdef.name, SymbolKind::Shared);
                self.model.environment.define(sdef.name.name.clone(), id);
            }
            // Use and Impl might declare things differently. For 'Use' we just bind the alias.
            Item::Use(_) => {
                // To be robust, Use handling is slightly more involved (module resolution).
            }
            Item::Impl(_) => {}
        }
    }

    // --- Traversal ---

    fn visit_item(&mut self, item: &Item) {
        match item {
            Item::Function(fndef) => self.visit_fn(fndef),
            Item::Struct(sdef) => {
                for bound in &sdef.where_clause {
                    for ty in &bound.bounds {
                        self.visit_type(ty);
                    }
                }
            }
            Item::Enum(edef) => {
                for bound in &edef.where_clause {
                    for ty in &bound.bounds {
                        self.visit_type(ty);
                    }
                }
                for variant in &edef.variants {
                    if let razen_ast::item::EnumVariantKind::Positional { fields } = &variant.kind {
                        for ty in fields {
                            self.visit_type(ty);
                        }
                    } else if let razen_ast::item::EnumVariantKind::Named { fields } = &variant.kind
                    {
                        for field in fields {
                            self.visit_type(&field.ty);
                        }
                    }
                }
            }
            Item::Trait(tdef) => {
                for method in &tdef.methods {
                    self.visit_fn(method);
                }
            }
            Item::Impl(iblock) => {
                self.visit_type(&iblock.target);
                for method in &iblock.methods {
                    self.visit_fn(method);
                }
            }
            Item::Const(cdef) => {
                self.visit_type(&cdef.ty);
                self.visit_expr(&cdef.value);
            }
            Item::Shared(sdef) => {
                if let Some(ty) = &sdef.ty {
                    self.visit_type(ty);
                }
                self.visit_expr(&sdef.value);
            }
            Item::TypeAlias(tdef) => self.visit_type(&tdef.ty),
            Item::Use(_) => {}
        }
    }

    fn visit_fn(&mut self, fndef: &FnDef) {
        self.model.environment.enter_scope(ScopeKind::Function);

        // Define parameters
        for param in &fndef.params {
            if let Some(ty) = &param.ty {
                self.visit_type(ty);
            }
            self.visit_pattern(&param.pattern, param.is_mut);
        }

        if let Some(ty) = &fndef.return_type {
            self.visit_type(ty);
        }

        match &fndef.body {
            razen_ast::item::FnBody::Block { stmts, tail, .. } => {
                self.model.environment.enter_scope(ScopeKind::Block);
                for stmt in stmts {
                    self.visit_stmt(stmt);
                }
                if let Some(expr) = tail {
                    self.visit_expr(expr);
                }
                self.model.environment.exit_scope();
            }
            razen_ast::item::FnBody::Expr(expr) => {
                self.visit_expr(expr);
            }
            razen_ast::item::FnBody::None => {}
        }

        self.model.environment.exit_scope();
    }

    fn visit_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let {
                pattern, ty, value, ..
            } => {
                self.visit_expr(value); // evaluate value before binding (no recursive self-references)
                if let Some(t) = ty {
                    self.visit_type(t);
                }
                self.visit_pattern(pattern, false);
            }
            Stmt::LetMut {
                name, ty, value, ..
            } => {
                self.visit_expr(value);
                self.visit_type(ty);
                let id = self
                    .model
                    .symbol_table
                    .add(name, SymbolKind::Variable { is_mut: true });
                self.model.environment.define(name.name.clone(), id);
                self.model.resolutions.insert(name.span, id);
            }
            Stmt::Const {
                name, ty, value, ..
            } => {
                self.visit_expr(value);
                self.visit_type(ty);
                let id = self.model.symbol_table.add(name, SymbolKind::Const);
                self.model.environment.define(name.name.clone(), id);
                self.model.resolutions.insert(name.span, id);
            }
            Stmt::Shared {
                name, ty, value, ..
            } => {
                self.visit_expr(value);
                if let Some(t) = ty {
                    self.visit_type(t);
                }
                let id = self.model.symbol_table.add(name, SymbolKind::Shared);
                self.model.environment.define(name.name.clone(), id);
                self.model.resolutions.insert(name.span, id);
            }
            Stmt::Expr { expr, .. } => self.visit_expr(expr),
            Stmt::Return { value, .. } => {
                if let Some(v) = value {
                    self.visit_expr(v);
                }
            }
            Stmt::Break { value, .. } => {
                if let Some(v) = value {
                    self.visit_expr(v);
                }
            }
            Stmt::Next { .. } => {}
            Stmt::Defer { body, .. } => self.visit_expr(body),
            Stmt::Guard {
                condition,
                else_body,
                ..
            } => {
                self.visit_expr(condition);
                // enter a block scope for the else body
                self.model.environment.enter_scope(ScopeKind::Block);
                if let Expr::Block { stmts, tail, .. } = else_body {
                    for stmt in stmts {
                        self.visit_stmt(stmt);
                    }
                    if let Some(t) = tail {
                        self.visit_expr(t);
                    }
                } else {
                    self.visit_expr(else_body);
                }
                self.model.environment.exit_scope();
            }
            Stmt::Item { item, .. } => {
                // Local items
                self.declare_item(item);
                self.visit_item(item);
            }
            Stmt::Assign { target, value, .. } => {
                self.visit_expr(value);
                self.visit_expr(target);
            }
            Stmt::CompoundAssign { target, value, .. } => {
                self.visit_expr(value);
                self.visit_expr(target);
            }
        }
    }

    fn visit_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Ident { ident, span } => {
                if ident.name == "self" || ident.name == "_" {
                    // Ignored or special
                } else {
                    match self.model.environment.resolve(&ident.name) {
                        Some(def_id) => {
                            self.model.resolutions.insert(*span, def_id);
                        }
                        None => {
                            self.push_error(SemanticError::UndefinedIdentifier {
                                name: ident.name.clone(),
                                span: *span,
                            });
                        }
                    }
                }
            }
            Expr::Binary { left, right, .. } => {
                self.visit_expr(left);
                self.visit_expr(right);
            }
            Expr::Unary { operand, .. } => {
                self.visit_expr(operand);
            }
            Expr::Field { object, .. } => {
                // The field identifier is not resolved directly here, as its resolution
                // depends on the type of `object` (which happens in Type Analysis).
                self.visit_expr(object);
            }
            Expr::MethodCall { object, args, .. } => {
                self.visit_expr(object);
                for arg in args {
                    self.visit_expr(arg);
                }
            }
            Expr::Call { callee, args, .. } => {
                self.visit_expr(callee);
                for arg in args {
                    self.visit_expr(arg);
                }
            }
            Expr::Index { object, index, .. } => {
                self.visit_expr(object);
                self.visit_expr(index);
            }
            Expr::Block { stmts, tail, .. } => {
                self.model.environment.enter_scope(ScopeKind::Block);
                for stmt in stmts {
                    self.visit_stmt(stmt);
                }
                if let Some(t) = tail {
                    self.visit_expr(t);
                }
                self.model.environment.exit_scope();
            }
            Expr::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.visit_expr(condition);
                // `then_block` is historically an Expr::Block, which opens its own scope via visit_expr
                self.visit_expr(then_block);
                if let Some(e) = else_block {
                    self.visit_expr(e);
                }
            }
            Expr::IfLet {
                pattern,
                value,
                then_block,
                else_block,
                ..
            } => {
                self.visit_expr(value);
                self.model.environment.enter_scope(ScopeKind::Block);
                self.visit_pattern(pattern, false);
                self.visit_expr(then_block);
                self.model.environment.exit_scope();

                if let Some(e) = else_block {
                    self.visit_expr(e);
                }
            }
            Expr::Match { subject, arms, .. } => {
                self.visit_expr(subject);
                for arm in arms {
                    self.model.environment.enter_scope(ScopeKind::MatchArm);
                    self.visit_pattern(&arm.pattern, false);
                    if let Some(guard) = &arm.guard {
                        self.visit_expr(guard);
                    }
                    self.visit_expr(&arm.body);
                    self.model.environment.exit_scope();
                }
            }
            Expr::Loop {
                kind,
                body,
                else_block,
                ..
            } => {
                self.model.environment.enter_scope(ScopeKind::Loop);
                match kind {
                    LoopKind::Infinite => {}
                    LoopKind::While { condition } => self.visit_expr(condition),
                    LoopKind::ForIn { binding, iterable } => {
                        self.visit_expr(iterable);
                        self.visit_pattern(binding, false);
                    }
                }
                self.visit_expr(body);
                self.model.environment.exit_scope(); // loop body scope exits
                if let Some(e) = else_block {
                    self.visit_expr(e);
                }
            }
            Expr::LoopLet {
                pattern,
                value,
                body,
                ..
            } => {
                self.model.environment.enter_scope(ScopeKind::Loop);
                self.visit_expr(value);
                self.visit_pattern(pattern, false);
                self.visit_expr(body);
                self.model.environment.exit_scope();
            }
            Expr::Closure { params, body, .. } => {
                self.model.environment.enter_scope(ScopeKind::Function);
                for param in params {
                    if let Some(ty) = &param.ty {
                        self.visit_type(ty);
                    }
                    let id = self
                        .model
                        .symbol_table
                        .add(&param.name, SymbolKind::Variable { is_mut: false });
                    self.model.environment.define(param.name.name.clone(), id);
                }
                self.visit_expr(body);
                self.model.environment.exit_scope();
            }
            Expr::Tuple { elements, .. }
            | Expr::Vec { elements, .. }
            | Expr::Set { elements, .. }
            | Expr::Array { elements, .. }
            | Expr::Tensor { elements, .. } => {
                for el in elements {
                    self.visit_expr(el);
                }
            }
            Expr::Map { entries, .. } => {
                for (k, v) in entries {
                    self.visit_expr(k);
                    self.visit_expr(v);
                }
            }
            Expr::StructLiteral {
                name,
                fields,
                spread,
                ..
            } => {
                self.visit_expr(name);
                for f in fields {
                    self.visit_expr(&f.value);
                }
                if let Some(s) = spread {
                    self.visit_expr(s);
                }
            }
            Expr::Cast { expr, ty, .. } | Expr::TypeCheck { expr, ty, .. } => {
                self.visit_expr(expr);
                self.visit_type(ty);
            }
            Expr::Try { expr, .. }
            | Expr::Await { expr, .. }
            | Expr::Paren { inner: expr, .. }
            | Expr::Unsafe { body: expr, .. } => {
                self.visit_expr(expr);
            }
            Expr::Assign { target, value, .. } => {
                self.visit_expr(value);
                self.visit_expr(target);
            }
            Expr::CompoundAssign { target, value, .. } => {
                self.visit_expr(value);
                self.visit_expr(target);
            }
            Expr::Fork { kind, .. } => {
                self.model.environment.enter_scope(ScopeKind::Block);
                match kind {
                    ForkKind::Block { tasks } => {
                        for t in tasks {
                            // First, fork expr can refer to outside scope
                            self.visit_expr(&t.expr);
                            // Then bind its output name in the fork scope
                            if let Some(ident) = &t.binding {
                                let id = self
                                    .model
                                    .symbol_table
                                    .add(ident, SymbolKind::Variable { is_mut: false });
                                self.model.environment.define(ident.name.clone(), id);
                            }
                        }
                    }
                    ForkKind::Loop {
                        binding,
                        iterable,
                        body,
                    } => {
                        self.visit_expr(iterable);
                        self.visit_pattern(binding, false);
                        self.visit_expr(body);
                    }
                }
                self.model.environment.exit_scope();
            }
            Expr::Path { segments, span } => {
                if let Some(first) = segments.first() {
                    if let Some(def_id) = self.model.environment.resolve(&first.name) {
                        self.model.resolutions.insert(first.span, def_id);
                        // The rest of the segments (Enum.Variant) are object-relative and resolved in type-check
                    } else {
                        self.push_error(SemanticError::UndefinedIdentifier {
                            name: first.name.clone(),
                            span: *span,
                        });
                    }
                }
            }
            // Leaves
            Expr::Literal { .. }
            | Expr::Break { .. }
            | Expr::Next { .. }
            | Expr::Return { .. }
            | Expr::Placeholder { .. } => {}
        }
    }

    fn visit_type(&mut self, ty: &TypeExpr) {
        match ty {
            TypeExpr::Named { name, span } => {
                // Primitive types like `int`, `float`, `str` will not be in Environment yet,
                // unless we pre-populate the environment. For now, if we don't find them, we don't error
                // if they are standard primitives, but a clean way is to pre-populate.
                // For now, attempt resolution.
                if let Some(def_id) = self.model.environment.resolve(&name.name) {
                    self.model.resolutions.insert(*span, def_id);
                }
            }
            TypeExpr::Generic { name, args, span } => {
                if let Some(def_id) = self.model.environment.resolve(&name.name) {
                    self.model.resolutions.insert(*span, def_id);
                }
                for a in args {
                    self.visit_type(a);
                }
            }
            TypeExpr::Array { element, size, .. } => {
                self.visit_type(element);
                self.visit_expr(size);
            }
            TypeExpr::Tuple { elements, .. } => {
                for e in elements {
                    self.visit_type(e);
                }
            }
            TypeExpr::Closure { params, ret, .. } => {
                for p in params {
                    self.visit_type(p);
                }
                self.visit_type(ret);
            }
            TypeExpr::Ref { inner, .. } => self.visit_type(inner),
            TypeExpr::Inferred { .. }
            | TypeExpr::Void { .. }
            | TypeExpr::Never { .. }
            | TypeExpr::SelfType { .. } => {}
        }
    }

    fn visit_pattern(&mut self, pat: &Pattern, is_mut: bool) {
        match pat {
            Pattern::Binding { name, span } => {
                let id = self
                    .model
                    .symbol_table
                    .add(name, SymbolKind::Variable { is_mut });
                self.model.environment.define(name.name.clone(), id);
                // Also record the definition span so the type-checker can
                // look up the DefId from a pattern-binding span.
                self.model.resolutions.insert(*span, id);
            }
            Pattern::Tuple { elements, .. } => {
                for e in elements {
                    self.visit_pattern(e, is_mut);
                }
            }
            Pattern::Struct { fields, .. } => {
                for field in fields {
                    // If rename is provided, bind the rename. Else bind the field name.
                    // OR if there's a sub-pattern, bind that.
                    if let Some(p) = &field.pattern {
                        self.visit_pattern(p, is_mut);
                    } else if let Some(rename) = &field.rename {
                        let id = self
                            .model
                            .symbol_table
                            .add(rename, SymbolKind::Variable { is_mut });
                        self.model.environment.define(rename.name.clone(), id);
                    } else {
                        let id = self
                            .model
                            .symbol_table
                            .add(&field.name, SymbolKind::Variable { is_mut });
                        self.model.environment.define(field.name.name.clone(), id);
                    }
                }
            }
            Pattern::TupleStruct { fields, .. } => {
                for p in fields {
                    self.visit_pattern(p, is_mut);
                }
            }
            Pattern::EnumPositional { args, .. } => {
                for p in args {
                    self.visit_pattern(p, is_mut);
                }
            }
            Pattern::EnumNamed { fields, .. } => {
                for f in fields {
                    if let Some(p) = &f.pattern {
                        self.visit_pattern(p, is_mut);
                    } else if let Some(rename) = &f.rename {
                        let id = self
                            .model
                            .symbol_table
                            .add(rename, SymbolKind::Variable { is_mut });
                        self.model.environment.define(rename.name.clone(), id);
                    } else {
                        let id = self
                            .model
                            .symbol_table
                            .add(&f.name, SymbolKind::Variable { is_mut });
                        self.model.environment.define(f.name.name.clone(), id);
                    }
                }
            }
            Pattern::Some { inner, .. }
            | Pattern::Ok { inner, .. }
            | Pattern::Err { inner, .. } => {
                self.visit_pattern(inner, is_mut);
            }
            Pattern::Or { patterns, .. } => {
                for p in patterns {
                    self.visit_pattern(p, is_mut);
                }
            }
            Pattern::Wildcard { .. }
            | Pattern::Literal { .. }
            | Pattern::None { .. }
            | Pattern::EnumUnit { .. }
            | Pattern::Range { .. } => {}
        }
    }
}
