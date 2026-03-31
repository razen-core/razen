//! Mutability Checker for Razen.
//!
//! This pass walks the fully name-resolved, type-checked AST and verifies
//! that immutable bindings are never reassigned.  It reports a
//! `SemanticError::Custom` for every illegal mutation it finds.
//!
//! # What counts as a mutation
//!
//! * `Stmt::Assign` / `Expr::Assign` — direct assignment `x = …`
//! * `Stmt::CompoundAssign` / `Expr::CompoundAssign` — `x += …`, `x -= …`, …
//!
//! The target expression is traced to its root identifier.  A mutation is
//! legal when that identifier is:
//!   * declared with `mut`  (`SymbolKind::Variable { is_mut: true }`)
//!   * declared with `shared` (`SymbolKind::Shared`)
//!   * a field / index expression whose root satisfies one of the above

use razen_ast::expr::Expr;
use razen_ast::item::{FnBody, FnDef, ImplBlock, Item};
use razen_ast::module::Module;
use razen_ast::stmt::Stmt;

use crate::error::SemanticError;
use crate::resolve::SemanticModel;
use crate::symbol::SymbolKind;

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

pub struct MutabilityChecker<'a> {
    model: &'a mut SemanticModel,
}

impl<'a> MutabilityChecker<'a> {
    pub fn new(model: &'a mut SemanticModel) -> Self {
        Self { model }
    }

    /// Check every function body in the module for illegal mutations.
    pub fn check_module(&mut self, module: &Module) {
        for item in &module.items {
            self.check_item(item);
        }
    }

    // -----------------------------------------------------------------------
    // Item dispatch
    // -----------------------------------------------------------------------

    fn check_item(&mut self, item: &Item) {
        match item {
            Item::Function(fndef) => self.check_fn(fndef),
            Item::Impl(iblock) => self.check_impl(iblock),
            Item::Trait(tdef) => {
                for method in &tdef.methods {
                    if !matches!(method.body, FnBody::None) {
                        self.check_fn(method);
                    }
                }
            }
            Item::Const(_)
            | Item::Shared(_)
            | Item::Struct(_)
            | Item::Enum(_)
            | Item::TypeAlias(_)
            | Item::Use(_) => {}
        }
    }

    fn check_impl(&mut self, iblock: &ImplBlock) {
        for method in &iblock.methods {
            self.check_fn(method);
        }
    }

    fn check_fn(&mut self, fndef: &FnDef) {
        match &fndef.body {
            FnBody::Block { stmts, tail, .. } => {
                for stmt in stmts {
                    self.check_stmt(stmt);
                }
                if let Some(tail_expr) = tail {
                    self.check_expr_for_mutations(tail_expr);
                }
            }
            FnBody::Expr(expr) => {
                self.check_expr_for_mutations(expr);
            }
            FnBody::None => {}
        }
    }

    // -----------------------------------------------------------------------
    // Statement traversal
    // -----------------------------------------------------------------------

    fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            // ── Illegal assignment targets ────────────────────────────────
            Stmt::Assign { target, value, .. } => {
                self.check_assignment_target(target, target.span());
                self.check_expr_for_mutations(value);
            }

            Stmt::CompoundAssign { target, value, .. } => {
                self.check_assignment_target(target, target.span());
                self.check_expr_for_mutations(value);
            }

            // ── Passthrough: recurse into sub-expressions / sub-statements
            Stmt::Let { value, .. } => self.check_expr_for_mutations(value),
            Stmt::LetMut { value, .. } => self.check_expr_for_mutations(value),
            Stmt::Const { value, .. } => self.check_expr_for_mutations(value),
            Stmt::Shared { value, .. } => self.check_expr_for_mutations(value),
            Stmt::Expr { expr, .. } => self.check_expr_for_mutations(expr),
            Stmt::Return { value, .. } => {
                if let Some(v) = value {
                    self.check_expr_for_mutations(v);
                }
            }
            Stmt::Break { value, .. } => {
                if let Some(v) = value {
                    self.check_expr_for_mutations(v);
                }
            }
            Stmt::Next { .. } => {}
            Stmt::Defer { body, .. } => self.check_expr_for_mutations(body),
            Stmt::Guard {
                condition,
                else_body,
                ..
            } => {
                self.check_expr_for_mutations(condition);
                self.check_expr_for_mutations(else_body);
            }
            Stmt::Item { item, .. } => self.check_item(item),
        }
    }

    // -----------------------------------------------------------------------
    // Expression traversal (looking for embedded Assign / CompoundAssign)
    // -----------------------------------------------------------------------

    fn check_expr_for_mutations(&mut self, expr: &Expr) {
        match expr {
            // ── Assignments embedded inside expressions ───────────────────
            Expr::Assign { target, value, .. } => {
                self.check_assignment_target(target, expr.span());
                self.check_expr_for_mutations(value);
            }

            Expr::CompoundAssign { target, value, .. } => {
                self.check_assignment_target(target, expr.span());
                self.check_expr_for_mutations(value);
            }

            // ── Recursive descent ─────────────────────────────────────────
            Expr::Binary { left, right, .. } => {
                self.check_expr_for_mutations(left);
                self.check_expr_for_mutations(right);
            }

            Expr::Unary { operand, .. } => self.check_expr_for_mutations(operand),

            Expr::Field { object, .. } => self.check_expr_for_mutations(object),

            Expr::MethodCall { object, args, .. } => {
                self.check_expr_for_mutations(object);
                for arg in args {
                    self.check_expr_for_mutations(arg);
                }
            }

            Expr::Call { callee, args, .. } => {
                self.check_expr_for_mutations(callee);
                for arg in args {
                    self.check_expr_for_mutations(arg);
                }
            }

            Expr::Index { object, index, .. } => {
                self.check_expr_for_mutations(object);
                self.check_expr_for_mutations(index);
            }

            Expr::Block { stmts, tail, .. } => {
                for stmt in stmts {
                    self.check_stmt(stmt);
                }
                if let Some(t) = tail {
                    self.check_expr_for_mutations(t);
                }
            }

            Expr::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.check_expr_for_mutations(condition);
                self.check_expr_for_mutations(then_block);
                if let Some(e) = else_block {
                    self.check_expr_for_mutations(e);
                }
            }

            Expr::IfLet {
                value,
                then_block,
                else_block,
                ..
            } => {
                self.check_expr_for_mutations(value);
                self.check_expr_for_mutations(then_block);
                if let Some(e) = else_block {
                    self.check_expr_for_mutations(e);
                }
            }

            Expr::Match { subject, arms, .. } => {
                self.check_expr_for_mutations(subject);
                for arm in arms {
                    if let Some(guard) = &arm.guard {
                        self.check_expr_for_mutations(guard);
                    }
                    self.check_expr_for_mutations(&arm.body);
                }
            }

            Expr::Loop {
                kind,
                body,
                else_block,
                ..
            } => {
                use razen_ast::expr::LoopKind;
                match kind {
                    LoopKind::While { condition } => self.check_expr_for_mutations(condition),
                    LoopKind::ForIn { iterable, .. } => self.check_expr_for_mutations(iterable),
                    LoopKind::Infinite => {}
                }
                self.check_expr_for_mutations(body);
                if let Some(e) = else_block {
                    self.check_expr_for_mutations(e);
                }
            }

            Expr::LoopLet { value, body, .. } => {
                self.check_expr_for_mutations(value);
                self.check_expr_for_mutations(body);
            }

            Expr::Closure { body, .. } => self.check_expr_for_mutations(body),

            Expr::Tuple { elements, .. }
            | Expr::Vec { elements, .. }
            | Expr::Set { elements, .. }
            | Expr::Array { elements, .. }
            | Expr::Tensor { elements, .. } => {
                for e in elements {
                    self.check_expr_for_mutations(e);
                }
            }

            Expr::Map { entries, .. } => {
                for (k, v) in entries {
                    self.check_expr_for_mutations(k);
                    self.check_expr_for_mutations(v);
                }
            }

            Expr::StructLiteral {
                name,
                fields,
                spread,
                ..
            } => {
                self.check_expr_for_mutations(name);
                for f in fields {
                    self.check_expr_for_mutations(&f.value);
                }
                if let Some(s) = spread {
                    self.check_expr_for_mutations(s);
                }
            }

            Expr::Cast { expr, .. } | Expr::TypeCheck { expr, .. } => {
                self.check_expr_for_mutations(expr);
            }

            Expr::Try { expr, .. }
            | Expr::Await { expr, .. }
            | Expr::Paren { inner: expr, .. }
            | Expr::Unsafe { body: expr, .. } => {
                self.check_expr_for_mutations(expr);
            }

            Expr::Break { value, .. } => {
                if let Some(v) = value {
                    self.check_expr_for_mutations(v);
                }
            }

            Expr::Return { value, .. } => {
                if let Some(v) = value {
                    self.check_expr_for_mutations(v);
                }
            }

            Expr::Fork { kind, .. } => {
                use razen_ast::expr::ForkKind;
                match kind {
                    ForkKind::Block { tasks } => {
                        for t in tasks {
                            self.check_expr_for_mutations(&t.expr);
                        }
                    }
                    ForkKind::Loop { iterable, body, .. } => {
                        self.check_expr_for_mutations(iterable);
                        self.check_expr_for_mutations(body);
                    }
                }
            }

            // Leaves — no sub-expressions to check.
            Expr::Literal { .. }
            | Expr::Ident { .. }
            | Expr::Path { .. }
            | Expr::Next { .. }
            | Expr::Placeholder { .. } => {}
        }
    }

    // -----------------------------------------------------------------------
    // Assignment-target validation
    // -----------------------------------------------------------------------

    /// Check that the assignment target is a mutable binding.
    /// If not, push a `SemanticError::Custom`.
    fn check_assignment_target(&mut self, target: &Expr, span: razen_lexer::Span) {
        if !self.is_mutable_target(target) {
            let name = extract_target_name(target);
            self.model.errors.push(SemanticError::Custom {
                message: format!(
                    "cannot assign to `{}`: not declared `mut` or `shared`",
                    name
                ),
                span,
            });
        }
    }

    /// Returns `true` when `expr` is a legal assignment target
    /// (a mutable root identifier, possibly accessed through fields/indices).
    fn is_mutable_target(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Ident { span, .. } => {
                if let Some(def_id) = self.model.resolutions.get(span) {
                    if let Some(sym) = self.model.symbol_table.get(*def_id) {
                        return matches!(
                            sym.kind,
                            SymbolKind::Variable { is_mut: true } | SymbolKind::Shared
                        );
                    }
                }
                // DefId not found — don't double-report; return true to suppress
                // a spurious mutability error when the identifier was already
                // flagged as undefined by the name resolver.
                true
            }

            // Field / index access inherits mutability from the root object.
            Expr::Field { object, .. } => self.is_mutable_target(object),
            Expr::Index { object, .. } => self.is_mutable_target(object),

            // Paren is transparent.
            Expr::Paren { inner, .. } => self.is_mutable_target(inner),

            // Anything else is not a valid lvalue.
            _ => false,
        }
    }
}

// ---------------------------------------------------------------------------
// Utility
// ---------------------------------------------------------------------------

/// Build a human-readable name for an assignment target for use in error
/// messages.
fn extract_target_name(expr: &Expr) -> String {
    match expr {
        Expr::Ident { ident, .. } => ident.name.clone(),
        Expr::Field { object, field, .. } => {
            format!("{}.{}", extract_target_name(object), field.name)
        }
        Expr::Index { object, .. } => format!("{}[…]", extract_target_name(object)),
        Expr::Paren { inner, .. } => extract_target_name(inner),
        _ => "<expr>".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use crate::analyze;
    use razen_parser::parse;

    fn errors_for(source: &str) -> Vec<String> {
        let module = parse(source).expect("parse failed");
        let sema = analyze(&module);
        sema.errors.iter().map(|e| format!("{}", e)).collect()
    }

    fn has_mut_error(source: &str) -> bool {
        errors_for(source)
            .iter()
            .any(|e| e.contains("cannot assign"))
    }

    #[test]
    fn test_immutable_reassign_rejected() {
        let source = r#"
            act println(x: int) void {}
            act main() void {
                x := 42
                x = 99
            }
        "#;
        assert!(has_mut_error(source), "expected mutability error");
    }

    #[test]
    fn test_mutable_reassign_allowed() {
        let source = r#"
            act println(x: int) void {}
            act main() void {
                mut x: int = 0
                x = 42
            }
        "#;
        assert!(!has_mut_error(source), "should not have mutability error");
    }

    #[test]
    fn test_shared_reassign_allowed() {
        let source = r#"
            act println(x: int) void {}
            act main() void {
                shared counter: int = 0
                counter = 1
            }
        "#;
        assert!(!has_mut_error(source), "shared should be assignable");
    }

    #[test]
    fn test_compound_assign_immutable_rejected() {
        let source = r#"
            act main() void {
                score := 10
                score += 5
            }
        "#;
        assert!(has_mut_error(source), "expected mutability error for +=");
    }

    #[test]
    fn test_compound_assign_mutable_allowed() {
        let source = r#"
            act main() void {
                mut score: int = 10
                score += 5
            }
        "#;
        assert!(!has_mut_error(source));
    }

    #[test]
    fn test_no_false_positive_on_let() {
        let source = r#"
            act main() void {
                x := 42
                y := x + 1
            }
        "#;
        assert!(!has_mut_error(source));
    }
}
