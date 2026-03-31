//! Bidirectional Type Checker for Razen.
//!
//! This module walks the fully name-resolved AST and assigns a `Ty` to every
//! expression, binding, function parameter, and return statement.  It uses the
//! `InferCtx` for Hindley-Milner-style unification so that expressions without
//! explicit annotations are still given concrete types wherever possible.
//!
//! # Two-pass design
//!
//! 1. `collect_definitions` — first pass: build `struct_fields`, `enum_variants`,
//!    `method_sigs`, and record top-level function / const / shared types into
//!    `model.type_env`.
//!
//! 2. `check_item` — second pass: type-check every function / impl / const body.
//!
//! 3. `finalize` — apply remaining inference substitutions and default any
//!    un-solved `Ty::Infer` variables to sensible concrete types.

use std::collections::HashMap;

use razen_ast::expr::{ClosureParam, Expr, ForkKind, LoopKind, StructLiteralField};
use razen_ast::ident::Ident;
use razen_ast::item::{
    ConstDef, EnumDef, EnumVariantKind, FnBody, FnDef, ImplBlock, Item, SharedDef, StructDef,
    StructKind, TraitDef, TypeAliasDef,
};
use razen_ast::lit::Literal;
use razen_ast::module::Module;
use razen_ast::ops::{BinOp, CompoundOp, UnaryOp};
use razen_ast::pat::Pattern;
use razen_ast::span::Span;
use razen_ast::stmt::Stmt;
use razen_ast::types::TypeExpr;

use crate::error::SemanticError;
use crate::infer::InferCtx;
use crate::resolve::SemanticModel;
use crate::symbol::{DefId, SymbolKind};
use crate::ty::Ty;

// ---------------------------------------------------------------------------
// Method signature record
// ---------------------------------------------------------------------------

/// Resolved signature of a single method (excluding the `self` parameter).
#[derive(Debug, Clone)]
pub struct MethodSig {
    pub params: Vec<Ty>,
    pub ret: Ty,
    pub is_async: bool,
}

// ---------------------------------------------------------------------------
// TypeChecker
// ---------------------------------------------------------------------------

pub struct TypeChecker<'a> {
    pub model: &'a mut SemanticModel,
    infer: InferCtx,

    // Current function context ------------------------------------------------
    /// Declared return type of the function currently being checked.
    /// `None` at module level.
    current_fn_ret: Option<Ty>,
    /// `Self` type inside an `impl` or `trait` block.
    current_self_ty: Option<Ty>,

    // Structural definitions collected in the first pass ----------------------
    /// struct DefId → vec of (field_name, field_ty)
    struct_fields: HashMap<DefId, Vec<(String, Ty)>>,
    /// enum DefId → vec of (variant_name, payload_tys)
    enum_variants: HashMap<DefId, Vec<(String, Vec<Ty>)>>,
    /// (impl-target DefId, method_name) → MethodSig
    method_sigs: HashMap<(DefId, String), MethodSig>,
}

impl<'a> TypeChecker<'a> {
    // -----------------------------------------------------------------------
    // Construction
    // -----------------------------------------------------------------------

    pub fn new(model: &'a mut SemanticModel) -> Self {
        Self {
            model,
            infer: InferCtx::new(),
            current_fn_ret: None,
            current_self_ty: None,
            struct_fields: HashMap::new(),
            enum_variants: HashMap::new(),
            method_sigs: HashMap::new(),
        }
    }

    // -----------------------------------------------------------------------
    // Public entry point
    // -----------------------------------------------------------------------

    pub fn check_module(&mut self, module: &Module) {
        // Pass 1 — collect structural definitions so field / method lookup
        //           works during Pass 2.
        for item in &module.items {
            self.collect_definitions(item);
        }
        // Pass 2 — type-check bodies.
        for item in &module.items {
            self.check_item(item);
        }
        // Pass 3 — apply substitutions & default unsolved variables.
        self.finalize();
    }

    // -----------------------------------------------------------------------
    // Pass 1: collect structural definitions
    // -----------------------------------------------------------------------

    fn collect_definitions(&mut self, item: &Item) {
        match item {
            // ── Struct ─────────────────────────────────────────────────────
            Item::Struct(sdef) => self.collect_struct(sdef),

            // ── Enum ───────────────────────────────────────────────────────
            Item::Enum(edef) => self.collect_enum(edef),

            // ── Impl ───────────────────────────────────────────────────────
            Item::Impl(iblock) => self.collect_impl(iblock),

            // ── Function (top-level) ───────────────────────────────────────
            Item::Function(fndef) => {
                let sig = self.resolve_fn_sig(fndef);
                if let Some(def_id) = self.lookup_def(&fndef.name) {
                    self.model.type_env.insert(def_id, sig);
                }
            }

            // ── Const ──────────────────────────────────────────────────────
            Item::Const(cdef) => {
                let ty = self.resolve_type_expr(&cdef.ty);
                if let Some(def_id) = self.lookup_def(&cdef.name) {
                    self.model.type_env.insert(def_id, ty);
                }
            }

            // ── Shared ─────────────────────────────────────────────────────
            Item::Shared(sdef) => {
                let ty = if let Some(ty_expr) = &sdef.ty {
                    Ty::Shared(Box::new(self.resolve_type_expr(ty_expr)))
                } else {
                    Ty::Shared(Box::new(self.infer.new_var()))
                };
                if let Some(def_id) = self.lookup_def(&sdef.name) {
                    self.model.type_env.insert(def_id, ty);
                }
            }

            // ── Type alias ─────────────────────────────────────────────────
            Item::TypeAlias(tdef) => {
                let ty = self.resolve_type_expr(&tdef.ty);
                if let Some(def_id) = self.lookup_def(&tdef.name) {
                    self.model.type_env.insert(def_id, ty);
                }
            }

            Item::Trait(_) | Item::Use(_) => {}
        }
    }

    fn collect_struct(&mut self, sdef: &StructDef) {
        let def_id = match self.lookup_def(&sdef.name) {
            Some(id) => id,
            None => return,
        };

        let fields: Vec<(String, Ty)> = match &sdef.kind {
            StructKind::Named { fields } => fields
                .iter()
                .map(|f| (f.name.name.clone(), self.resolve_type_expr(&f.ty)))
                .collect(),
            StructKind::Tuple { fields } => fields
                .iter()
                .enumerate()
                .map(|(i, ty)| (i.to_string(), self.resolve_type_expr(ty)))
                .collect(),
            StructKind::Unit => vec![],
        };

        self.struct_fields.insert(def_id, fields);
        // Register the struct type itself.
        self.model.type_env.insert(
            def_id,
            Ty::Named {
                def_id,
                name: sdef.name.name.clone(),
                generics: sdef
                    .generic_params
                    .iter()
                    .map(|p| Ty::Param(p.name.name.clone()))
                    .collect(),
            },
        );
    }

    fn collect_enum(&mut self, edef: &EnumDef) {
        let def_id = match self.lookup_def(&edef.name) {
            Some(id) => id,
            None => return,
        };

        let variants: Vec<(String, Vec<Ty>)> = edef
            .variants
            .iter()
            .map(|v| {
                let payload = match &v.kind {
                    EnumVariantKind::Unit => vec![],
                    EnumVariantKind::Positional { fields } => {
                        fields.iter().map(|t| self.resolve_type_expr(t)).collect()
                    }
                    EnumVariantKind::Named { fields } => fields
                        .iter()
                        .map(|f| self.resolve_type_expr(&f.ty))
                        .collect(),
                };
                (v.name.name.clone(), payload)
            })
            .collect();

        self.enum_variants.insert(def_id, variants);
        self.model.type_env.insert(
            def_id,
            Ty::Named {
                def_id,
                name: edef.name.name.clone(),
                generics: edef
                    .generic_params
                    .iter()
                    .map(|p| Ty::Param(p.name.name.clone()))
                    .collect(),
            },
        );
    }

    fn collect_impl(&mut self, iblock: &ImplBlock) {
        let self_ty = self.resolve_type_expr(&iblock.target);
        let target_def_id = match &self_ty {
            Ty::Named { def_id, .. } => *def_id,
            _ => return,
        };

        for method in &iblock.methods {
            let sig = self.resolve_fn_sig(method);
            if let Ty::Fn {
                ref params,
                ref ret,
                is_async,
            } = sig
            {
                let method_sig = MethodSig {
                    params: params.clone(),
                    ret: *ret.clone(),
                    is_async,
                };
                self.method_sigs
                    .insert((target_def_id, method.name.name.clone()), method_sig);
            }
            // Also put the method in type_env keyed by its own DefId if resolvable.
            if let Some(def_id) = self.lookup_def(&method.name) {
                self.model.type_env.insert(def_id, sig);
            }
        }
    }

    // -----------------------------------------------------------------------
    // Pass 2: type-check items
    // -----------------------------------------------------------------------

    fn check_item(&mut self, item: &Item) {
        match item {
            Item::Function(fndef) => self.check_fn(fndef, None),
            Item::Impl(iblock) => {
                let self_ty = self.resolve_type_expr(&iblock.target);
                let saved = self.current_self_ty.clone();
                self.current_self_ty = Some(self_ty.clone());
                for method in &iblock.methods {
                    self.check_fn(method, Some(self_ty.clone()));
                }
                self.current_self_ty = saved;
            }
            Item::Trait(tdef) => {
                for method in &tdef.methods {
                    if !matches!(method.body, FnBody::None) {
                        self.check_fn(method, None);
                    }
                }
            }
            Item::Const(cdef) => {
                let declared = self.resolve_type_expr(&cdef.ty);
                self.check_expr(&cdef.value, declared);
            }
            Item::Shared(sdef) => {
                let declared = if let Some(ty_expr) = &sdef.ty {
                    self.resolve_type_expr(ty_expr)
                } else {
                    self.infer.new_var()
                };
                self.check_expr(&sdef.value, declared);
            }
            Item::Struct(_) | Item::Enum(_) | Item::TypeAlias(_) | Item::Use(_) => {}
        }
    }

    fn check_fn(&mut self, fndef: &FnDef, self_ty: Option<Ty>) {
        // Save context.
        let saved_ret = self.current_fn_ret.clone();
        let saved_self = self.current_self_ty.clone();

        let ret_ty = fndef
            .return_type
            .as_ref()
            .map(|t| self.resolve_type_expr(t))
            .unwrap_or(Ty::Void);

        self.current_fn_ret = Some(ret_ty.clone());
        if let Some(st) = self_ty {
            self.current_self_ty = Some(st);
        }

        // Register parameter types.
        for param in &fndef.params {
            let param_ty = param
                .ty
                .as_ref()
                .map(|t| self.resolve_type_expr(t))
                .unwrap_or_else(|| self.infer.new_var());

            self.bind_pattern_types(&param.pattern, param_ty, param.is_mut);
        }

        // Check body.
        match &fndef.body {
            FnBody::Block { stmts, tail, .. } => {
                for stmt in stmts {
                    self.check_stmt(stmt);
                }
                if let Some(tail_expr) = tail {
                    // The tail is the implicit return value.
                    self.check_expr(tail_expr, ret_ty);
                }
            }
            FnBody::Expr(expr) => {
                self.check_expr(expr, ret_ty);
            }
            FnBody::None => {}
        }

        // Restore context.
        self.current_fn_ret = saved_ret;
        self.current_self_ty = saved_self;
    }

    // -----------------------------------------------------------------------
    // Type checking helpers
    // -----------------------------------------------------------------------

    /// Infer the type of `expr`, record it, and return it.
    pub fn infer_expr(&mut self, expr: &Expr) -> Ty {
        let ty = self.infer_expr_inner(expr);
        self.record_expr_type(expr, ty.clone());
        ty
    }

    /// Check that `expr` has type `expected`, emitting an error on mismatch.
    pub fn check_expr(&mut self, expr: &Expr, expected: Ty) {
        let inferred = self.infer_expr(expr);
        let mut errors = std::mem::take(&mut self.model.errors);
        self.infer
            .unify(inferred, expected, expr.span(), &mut errors);
        self.model.errors = errors;
    }

    fn record_expr_type(&mut self, expr: &Expr, ty: Ty) {
        self.model.expr_types.insert(expr.span(), ty);
    }

    // -----------------------------------------------------------------------
    // Core expression inference
    // -----------------------------------------------------------------------

    fn infer_expr_inner(&mut self, expr: &Expr) -> Ty {
        match expr {
            // ── Literals ──────────────────────────────────────────────────
            Expr::Literal { lit, .. } => self.infer_literal(lit),

            // ── Identifier ────────────────────────────────────────────────
            Expr::Ident { ident, span } => self.infer_ident(&ident.name, *span),

            // ── Parenthesised ─────────────────────────────────────────────
            Expr::Paren { inner, .. } => self.infer_expr(inner),

            // ── Unsafe block ──────────────────────────────────────────────
            Expr::Unsafe { body, .. } => self.infer_expr(body),

            // ── Binary operation ──────────────────────────────────────────
            Expr::Binary {
                left,
                op,
                right,
                span,
            } => {
                let lt = self.infer_expr(left);
                let rt = self.infer_expr(right);
                self.infer_binary(*op, lt, rt, *span)
            }

            // ── Unary operation ───────────────────────────────────────────
            Expr::Unary { op, operand, span } => {
                let ot = self.infer_expr(operand);
                self.infer_unary(*op, ot, *span)
            }

            // ── Field access ──────────────────────────────────────────────
            Expr::Field {
                object,
                field,
                span,
            } => {
                let obj_ty = self.infer_expr(object);
                self.lookup_field(&obj_ty, &field.name, *span)
            }

            // ── Method call ───────────────────────────────────────────────
            Expr::MethodCall {
                object,
                method,
                args,
                span,
            } => {
                let obj_ty = self.infer_expr(object);
                let arg_tys: Vec<Ty> = args.iter().map(|a| self.infer_expr(a)).collect();
                self.infer_method_call(&obj_ty, &method.name, &arg_tys, *span)
            }

            // ── Function call ─────────────────────────────────────────────
            Expr::Call { callee, args, span } => {
                let callee_ty = self.infer_expr(callee);
                let arg_tys: Vec<Ty> = args.iter().map(|a| self.infer_expr(a)).collect();
                self.infer_call(callee_ty, &arg_tys, *span)
            }

            // ── Block ─────────────────────────────────────────────────────
            Expr::Block { stmts, tail, .. } => {
                for stmt in stmts {
                    self.check_stmt(stmt);
                }
                if let Some(tail) = tail {
                    self.infer_expr(tail)
                } else {
                    Ty::Void
                }
            }

            // ── If / else ─────────────────────────────────────────────────
            Expr::If {
                condition,
                then_block,
                else_block,
                span,
            } => {
                self.check_expr(condition, Ty::Bool);
                let then_ty = self.infer_expr(then_block);
                if let Some(else_expr) = else_block {
                    let else_ty = self.infer_expr(else_expr);
                    let mut errors = std::mem::take(&mut self.model.errors);
                    let unified = self.infer.unify(then_ty, else_ty, *span, &mut errors);
                    self.model.errors = errors;
                    unified
                } else {
                    Ty::Void
                }
            }

            // ── If let ────────────────────────────────────────────────────
            Expr::IfLet {
                pattern,
                value,
                then_block,
                else_block,
                span,
            } => {
                let val_ty = self.infer_expr(value);
                self.bind_pattern_types(pattern, val_ty, false);
                let then_ty = self.infer_expr(then_block);
                if let Some(else_expr) = else_block {
                    let else_ty = self.infer_expr(else_expr);
                    let mut errors = std::mem::take(&mut self.model.errors);
                    let unified = self.infer.unify(then_ty, else_ty, *span, &mut errors);
                    self.model.errors = errors;
                    unified
                } else {
                    Ty::Void
                }
            }

            // ── Match ─────────────────────────────────────────────────────
            Expr::Match {
                subject,
                arms,
                span,
            } => {
                let subj_ty = self.infer_expr(subject);
                let mut result_ty = self.infer.new_var();

                for arm in arms {
                    self.bind_pattern_types(&arm.pattern, subj_ty.clone(), false);
                    if let Some(guard) = &arm.guard {
                        self.check_expr(guard, Ty::Bool);
                    }
                    let arm_ty = self.infer_expr(&arm.body);
                    let mut errors = std::mem::take(&mut self.model.errors);
                    result_ty = self
                        .infer
                        .unify(result_ty.clone(), arm_ty, *span, &mut errors);
                    self.model.errors = errors;
                }

                self.infer.apply(result_ty)
            }

            // ── Loop ──────────────────────────────────────────────────────
            Expr::Loop {
                kind,
                body,
                else_block,
                span,
                ..
            } => {
                match kind {
                    LoopKind::Infinite => {}
                    LoopKind::While { condition } => {
                        self.check_expr(condition, Ty::Bool);
                    }
                    LoopKind::ForIn { binding, iterable } => {
                        let iter_ty = self.infer_expr(iterable);
                        let elem_ty = self.elem_type_of(&iter_ty, *span);
                        self.bind_pattern_types(binding, elem_ty, false);
                    }
                }
                self.infer_expr(body);
                if let Some(else_expr) = else_block {
                    self.infer_expr(else_expr)
                } else {
                    Ty::Void
                }
            }

            // ── Loop-let ──────────────────────────────────────────────────
            Expr::LoopLet {
                pattern,
                value,
                body,
                ..
            } => {
                let val_ty = self.infer_expr(value);
                let inner_ty = match val_ty {
                    Ty::Option(t) => *t,
                    _ => self.infer.new_var(),
                };
                self.bind_pattern_types(pattern, inner_ty, false);
                self.infer_expr(body);
                Ty::Void
            }

            // ── Closure ───────────────────────────────────────────────────
            Expr::Closure { params, body, .. } => {
                let param_tys: Vec<Ty> = params
                    .iter()
                    .map(|p: &ClosureParam| {
                        let ty =
                            p.ty.as_ref()
                                .map(|te| self.resolve_type_expr(te))
                                .unwrap_or_else(|| self.infer.new_var());
                        // Bind the parameter name.
                        if let Some(def_id) = self.lookup_span_def(p.name.span) {
                            self.model.type_env.insert(def_id, ty.clone());
                        }
                        ty
                    })
                    .collect();

                let ret_ty = self.infer_expr(body);
                Ty::Fn {
                    params: param_tys,
                    ret: Box::new(ret_ty),
                    is_async: false,
                }
            }

            // ── Tuple ─────────────────────────────────────────────────────
            Expr::Tuple { elements, .. } => {
                Ty::Tuple(elements.iter().map(|e| self.infer_expr(e)).collect())
            }

            // ── Vec literal ───────────────────────────────────────────────
            Expr::Vec { elements, span } => {
                if elements.is_empty() {
                    Ty::Vec(Box::new(self.infer.new_var()))
                } else {
                    let first_ty = self.infer_expr(&elements[0]);
                    let mut elem_ty = first_ty;
                    for el in &elements[1..] {
                        let t = self.infer_expr(el);
                        let mut errors = std::mem::take(&mut self.model.errors);
                        elem_ty = self.infer.unify(elem_ty, t, *span, &mut errors);
                        self.model.errors = errors;
                    }
                    Ty::Vec(Box::new(self.infer.apply(elem_ty)))
                }
            }

            // ── Map literal ───────────────────────────────────────────────
            Expr::Map { entries, span } => {
                if entries.is_empty() {
                    Ty::Map(
                        Box::new(self.infer.new_var()),
                        Box::new(self.infer.new_var()),
                    )
                } else {
                    let (fk, fv) = &entries[0];
                    let mut key_ty = self.infer_expr(fk);
                    let mut val_ty = self.infer_expr(fv);
                    for (k, v) in &entries[1..] {
                        let kt = self.infer_expr(k);
                        let vt = self.infer_expr(v);
                        let mut errors = std::mem::take(&mut self.model.errors);
                        key_ty = self.infer.unify(key_ty, kt, *span, &mut errors);
                        val_ty = self.infer.unify(val_ty, vt, *span, &mut errors);
                        self.model.errors = errors;
                    }
                    Ty::Map(
                        Box::new(self.infer.apply(key_ty)),
                        Box::new(self.infer.apply(val_ty)),
                    )
                }
            }

            // ── Set literal ───────────────────────────────────────────────
            Expr::Set { elements, span } => {
                if elements.is_empty() {
                    Ty::Set(Box::new(self.infer.new_var()))
                } else {
                    let first_ty = self.infer_expr(&elements[0]);
                    let mut elem_ty = first_ty;
                    for el in &elements[1..] {
                        let t = self.infer_expr(el);
                        let mut errors = std::mem::take(&mut self.model.errors);
                        elem_ty = self.infer.unify(elem_ty, t, *span, &mut errors);
                        self.model.errors = errors;
                    }
                    Ty::Set(Box::new(self.infer.apply(elem_ty)))
                }
            }

            // ── Array literal ─────────────────────────────────────────────
            Expr::Array { elements, span } => {
                if elements.is_empty() {
                    Ty::Array {
                        element: Box::new(self.infer.new_var()),
                        size: 0,
                    }
                } else {
                    let first_ty = self.infer_expr(&elements[0]);
                    let mut elem_ty = first_ty;
                    for el in &elements[1..] {
                        let t = self.infer_expr(el);
                        let mut errors = std::mem::take(&mut self.model.errors);
                        elem_ty = self.infer.unify(elem_ty, t, *span, &mut errors);
                        self.model.errors = errors;
                    }
                    Ty::Array {
                        element: Box::new(self.infer.apply(elem_ty)),
                        size: elements.len() as u64,
                    }
                }
            }

            // ── Tensor literal ────────────────────────────────────────────
            Expr::Tensor { elements, .. } => {
                for el in elements {
                    self.infer_expr(el);
                }
                Ty::Tensor
            }

            // ── Struct literal ────────────────────────────────────────────
            Expr::StructLiteral {
                name,
                fields,
                spread,
                ..
            } => {
                let struct_ty = self.infer_expr(name);
                for f in fields {
                    let field_ty = self.lookup_field(&struct_ty, &f.name.name, f.span);
                    self.check_expr(&f.value, field_ty);
                }
                if let Some(spread_expr) = spread {
                    self.check_expr(spread_expr, struct_ty.clone());
                }
                struct_ty
            }

            // ── Cast (`as`) ───────────────────────────────────────────────
            Expr::Cast { expr, ty, .. } => {
                self.infer_expr(expr);
                self.resolve_type_expr(ty)
            }

            // ── Type check (`is`) ─────────────────────────────────────────
            Expr::TypeCheck { expr, ty, .. } => {
                self.infer_expr(expr);
                self.resolve_type_expr(ty);
                Ty::Bool
            }

            // ── Error propagation (`?`) ───────────────────────────────────
            Expr::Try { expr, span } => {
                let et = self.infer_expr(expr);
                match et {
                    Ty::Result(t, _) => *t,
                    Ty::Option(t) => *t,
                    Ty::Infer(_) => self.infer.new_var(),
                    Ty::Error => Ty::Error,
                    other => {
                        self.push_error(
                            format!("`?` requires `result` or `option`, found `{}`", other),
                            *span,
                        );
                        Ty::Error
                    }
                }
            }

            // ── Await (`.await`) ──────────────────────────────────────────
            Expr::Await { expr, .. } => {
                let et = self.infer_expr(expr);
                match et {
                    Ty::Result(t, _) => *t,
                    Ty::Option(t) => *t,
                    Ty::Fn { ret, .. } => *ret,
                    other => other,
                }
            }

            // ── Assignment ────────────────────────────────────────────────
            Expr::Assign { target, value, .. } => {
                let target_ty = self.infer_expr(target);
                self.check_expr(value, target_ty);
                Ty::Void
            }

            // ── Compound assignment ───────────────────────────────────────
            Expr::CompoundAssign { target, value, .. } => {
                let target_ty = self.infer_expr(target);
                self.check_expr(value, target_ty);
                Ty::Void
            }

            // ── Break ─────────────────────────────────────────────────────
            Expr::Break { value, .. } => {
                if let Some(v) = value {
                    self.infer_expr(v);
                }
                Ty::Never
            }

            // ── Next (continue) ───────────────────────────────────────────
            Expr::Next { .. } => Ty::Never,

            // ── Return ────────────────────────────────────────────────────
            Expr::Return { value, span } => {
                if let Some(v) = value {
                    if let Some(ret_ty) = self.current_fn_ret.clone() {
                        self.check_expr(v, ret_ty);
                    } else {
                        self.infer_expr(v);
                    }
                }
                Ty::Never
            }

            // ── Fork ──────────────────────────────────────────────────────
            Expr::Fork { kind, span } => match kind {
                ForkKind::Block { tasks } => {
                    let tys: Vec<Ty> = tasks.iter().map(|t| self.infer_expr(&t.expr)).collect();
                    Ty::Tuple(tys)
                }
                ForkKind::Loop {
                    binding,
                    iterable,
                    body,
                } => {
                    let iter_ty = self.infer_expr(iterable);
                    let elem_ty = self.elem_type_of(&iter_ty, *span);
                    self.bind_pattern_types(binding, elem_ty, false);
                    let body_ty = self.infer_expr(body);
                    Ty::Vec(Box::new(body_ty))
                }
            },

            // ── Path (Enum.Variant, module.item) ──────────────────────────
            Expr::Path { segments, span } => {
                if let Some(first) = segments.first() {
                    let ty = self.infer_ident(&first.name, first.span);
                    // For single-segment paths just return the type directly.
                    if segments.len() == 1 {
                        return ty;
                    }
                    // Multi-segment: resolve enum variant if the first segment
                    // is an enum type.
                    if let Ty::Named {
                        def_id, ref name, ..
                    } = ty
                    {
                        if let Some(variants) = self.enum_variants.get(&def_id) {
                            if let Some(second) = segments.get(1) {
                                if let Some((_, payload)) =
                                    variants.iter().find(|(vname, _)| *vname == second.name)
                                {
                                    if payload.is_empty() {
                                        return ty;
                                    } else {
                                        // Positional variant — return the enum type.
                                        return ty;
                                    }
                                }
                            }
                        }
                    }
                    ty
                } else {
                    Ty::Error
                }
            }

            // ── Index ─────────────────────────────────────────────────────
            Expr::Index {
                object,
                index,
                span,
            } => {
                self.check_expr(index, Ty::Uint);
                let obj_ty = self.infer_expr(object);
                match obj_ty {
                    Ty::Vec(t) | Ty::Array { element: t, .. } => *t,
                    Ty::Map(_, v) => *v,
                    Ty::Str => Ty::Char,
                    Ty::Infer(_) => self.infer.new_var(),
                    Ty::Error => Ty::Error,
                    other => {
                        self.push_error(format!("cannot index into `{}`", other), *span);
                        Ty::Error
                    }
                }
            }

            // ── Placeholder ───────────────────────────────────────────────
            Expr::Placeholder { .. } => Ty::Error,
        }
    }

    // -----------------------------------------------------------------------
    // Literal inference
    // -----------------------------------------------------------------------

    fn infer_literal(&self, lit: &Literal) -> Ty {
        match lit {
            Literal::Int { .. } => Ty::Int,
            Literal::Float { .. } => Ty::Float,
            Literal::Str { .. } => Ty::Str,
            Literal::Char { .. } => Ty::Char,
            Literal::Bool { .. } => Ty::Bool,
        }
    }

    // -----------------------------------------------------------------------
    // Identifier lookup
    // -----------------------------------------------------------------------

    fn infer_ident(&mut self, name: &str, span: Span) -> Ty {
        if name == "self" {
            return self.current_self_ty.clone().unwrap_or(Ty::SelfTy);
        }
        if name == "_" {
            return self.infer.new_var();
        }
        if let Some(def_id) = self.model.resolutions.get(&span).copied() {
            if let Some(ty) = self.model.type_env.get(&def_id).cloned() {
                return ty;
            }
        }
        // Fallback — identifier not in type_env yet (may be a forward reference
        // or a prelude name whose type_env entry has a different span).
        Ty::Error
    }

    // -----------------------------------------------------------------------
    // Binary / unary inference
    // -----------------------------------------------------------------------

    fn infer_binary(&mut self, op: BinOp, lt: Ty, rt: Ty, span: Span) -> Ty {
        match op {
            // Arithmetic: both sides must be the same numeric type.
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod | BinOp::Pow => {
                let mut errors = std::mem::take(&mut self.model.errors);
                let unified = self.infer.unify(lt, rt, span, &mut errors);
                self.model.errors = errors;
                unified
            }

            // Comparison: operands must match; result is bool.
            BinOp::Eq | BinOp::NotEq | BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq => {
                let mut errors = std::mem::take(&mut self.model.errors);
                self.infer.unify(lt, rt, span, &mut errors);
                self.model.errors = errors;
                Ty::Bool
            }

            // Logical: both sides and result are bool.
            BinOp::And | BinOp::Or => {
                let mut errors = std::mem::take(&mut self.model.errors);
                self.infer.unify(lt.clone(), Ty::Bool, span, &mut errors);
                self.infer.unify(rt, Ty::Bool, span, &mut errors);
                self.model.errors = errors;
                Ty::Bool
            }

            // Bitwise: both sides should be integral.
            BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor | BinOp::Shl | BinOp::Shr => {
                let mut errors = std::mem::take(&mut self.model.errors);
                let unified = self.infer.unify(lt, rt, span, &mut errors);
                self.model.errors = errors;
                unified
            }

            // Ranges — produce a Range-like opaque type for now.
            BinOp::Range | BinOp::RangeInclusive => {
                let mut errors = std::mem::take(&mut self.model.errors);
                let elem = self.infer.unify(lt, rt, span, &mut errors);
                self.model.errors = errors;
                Ty::Named {
                    def_id: crate::symbol::DefId(usize::MAX - 10),
                    name: "Range".into(),
                    generics: vec![elem],
                }
            }

            // Async pipeline ~>: result type is the right-hand type.
            BinOp::AsyncPipe => rt,
        }
    }

    fn infer_unary(&mut self, op: UnaryOp, ot: Ty, span: Span) -> Ty {
        match op {
            UnaryOp::Neg => ot,
            UnaryOp::Not => {
                let mut errors = std::mem::take(&mut self.model.errors);
                self.infer.unify(ot, Ty::Bool, span, &mut errors);
                self.model.errors = errors;
                Ty::Bool
            }
            UnaryOp::BitNot => ot,
            UnaryOp::Ref => ot,
            UnaryOp::Deref => ot,
        }
    }

    // -----------------------------------------------------------------------
    // Call inference
    // -----------------------------------------------------------------------

    fn infer_call(&mut self, callee_ty: Ty, arg_tys: &[Ty], span: Span) -> Ty {
        match callee_ty {
            Ty::Fn { params, ret, .. } => {
                // Check argument count.
                if params.len() != arg_tys.len() {
                    // Variadic prelude functions (Param types in params) — skip.
                    if !params.iter().any(|p| matches!(p, Ty::Param(_))) {
                        self.push_error(
                            format!(
                                "expected {} argument(s), found {}",
                                params.len(),
                                arg_tys.len()
                            ),
                            span,
                        );
                    }
                    return *ret;
                }
                // Check each argument.
                for (param_ty, arg_ty) in params.iter().zip(arg_tys.iter()) {
                    // Skip polymorphic params.
                    if matches!(param_ty, Ty::Param(_)) {
                        continue;
                    }
                    let mut errors = std::mem::take(&mut self.model.errors);
                    self.infer
                        .unify(arg_ty.clone(), param_ty.clone(), span, &mut errors);
                    self.model.errors = errors;
                }
                *ret
            }
            // Struct constructor call: `User { ... }` is already handled by
            // StructLiteral. A plain `User(arg)` (tuple struct) returns Named.
            Ty::Named {
                def_id,
                name,
                generics,
            } => Ty::Named {
                def_id,
                name,
                generics,
            },
            Ty::Error => Ty::Error,
            Ty::Infer(_) => self.infer.new_var(),
            _ => {
                self.push_error(format!("`{}` is not a function", callee_ty), span);
                Ty::Error
            }
        }
    }

    // -----------------------------------------------------------------------
    // Method call inference
    // -----------------------------------------------------------------------

    fn infer_method_call(&mut self, recv_ty: &Ty, method: &str, arg_tys: &[Ty], span: Span) -> Ty {
        // Peel shared wrapper — `shared T` behaves the same as `T` for methods.
        let base_ty = recv_ty.peel_shared().clone();

        // 1. Try built-in methods.
        if let Some(ret) = self.builtin_method_ret(&base_ty, method, arg_tys.len()) {
            return ret;
        }

        // 2. Try user-defined methods from impl blocks.
        if let Ty::Named { def_id, .. } = &base_ty {
            if let Some(sig) = self
                .method_sigs
                .get(&(*def_id, method.to_string()))
                .cloned()
            {
                return sig.ret.clone();
            }
        }

        // 3. `to_string` works on anything.
        if method == "to_string" {
            return Ty::Str;
        }
        if method == "clone" {
            return base_ty;
        }

        // 4. Unknown method — don't error immediately (the type may not be
        //    fully resolved yet); return a fresh inference variable.
        self.infer.new_var()
    }

    /// Return the type produced by a built-in method call, or `None` if the
    /// method is not a known built-in for the given receiver type.
    fn builtin_method_ret(&mut self, recv: &Ty, method: &str, _argc: usize) -> Option<Ty> {
        match recv {
            // ── str ────────────────────────────────────────────────────────
            Ty::Str => match method {
                "len" => Some(Ty::Uint),
                "is_empty" => Some(Ty::Bool),
                "to_upper_case" | "to_lower_case" | "trim" | "trim_start" | "trim_end" => {
                    Some(Ty::Str)
                }
                "contains" | "starts_with" | "ends_with" => Some(Ty::Bool),
                "find" => Some(Ty::Option(Box::new(Ty::Uint))),
                "replace" => Some(Ty::Str),
                "split" => Some(Ty::Vec(Box::new(Ty::Str))),
                "join" => Some(Ty::Str),
                "chars" => Some(Ty::Vec(Box::new(Ty::Char))),
                "to_string" | "clone" => Some(Ty::Str),
                "parse_int" => Some(Ty::Result(Box::new(Ty::Int), Box::new(Ty::Str))),
                "parse_float" => Some(Ty::Result(Box::new(Ty::Float), Box::new(Ty::Str))),
                "parse_bool" => Some(Ty::Result(Box::new(Ty::Bool), Box::new(Ty::Str))),
                "parse_char" => Some(Ty::Result(Box::new(Ty::Char), Box::new(Ty::Str))),
                "parse" => Some(Ty::Result(
                    Box::new(self.infer.new_var()),
                    Box::new(Ty::Str),
                )),
                "bytes" => Some(Ty::Vec(Box::new(Ty::U8))),
                "repeat" => Some(Ty::Str),
                _ => None,
            },

            // ── vec[T] ─────────────────────────────────────────────────────
            Ty::Vec(elem) => {
                let t = *elem.clone();
                match method {
                    "len" | "count" => Some(Ty::Uint),
                    "is_empty" => Some(Ty::Bool),
                    "push" | "clear" | "sort" | "sort_by" | "reverse" | "dedup" => Some(Ty::Void),
                    "pop" | "first" | "last" => Some(Ty::Option(Box::new(t.clone()))),
                    "get" => Some(Ty::Option(Box::new(t.clone()))),
                    "remove" | "swap_remove" => Some(t.clone()),
                    "insert" => Some(Ty::Void),
                    "contains" => Some(Ty::Bool),
                    "iter" | "clone" | "into_iter" => Some(Ty::Vec(Box::new(t.clone()))),
                    "enumerate" => Some(Ty::Vec(Box::new(Ty::Tuple(vec![Ty::Uint, t.clone()])))),
                    "zip" => Some(Ty::Vec(Box::new(Ty::Tuple(vec![
                        t.clone(),
                        self.infer.new_var(),
                    ])))),
                    "map" => Some(Ty::Vec(Box::new(self.infer.new_var()))),
                    "filter" => Some(Ty::Vec(Box::new(t.clone()))),
                    "filter_map" => Some(Ty::Vec(Box::new(self.infer.new_var()))),
                    "flat_map" | "flatten" => Some(Ty::Vec(Box::new(self.infer.new_var()))),
                    "fold" | "reduce" => Some(self.infer.new_var()),
                    "any" | "all" => Some(Ty::Bool),
                    "find" | "find_map" => Some(Ty::Option(Box::new(t.clone()))),
                    "position" | "index_of" => Some(Ty::Option(Box::new(Ty::Uint))),
                    "collect" => Some(Ty::Vec(Box::new(t.clone()))),
                    "join" => Some(Ty::Str),
                    "concat" => Some(Ty::Str),
                    "extend" | "append" => Some(Ty::Void),
                    "split_at" => Some(Ty::Tuple(vec![
                        Ty::Vec(Box::new(t.clone())),
                        Ty::Vec(Box::new(t.clone())),
                    ])),
                    "chunks" | "windows" => Some(Ty::Vec(Box::new(Ty::Vec(Box::new(t.clone()))))),
                    "with_capacity" | "new" => Some(Ty::Vec(Box::new(t.clone()))),
                    "capacity" => Some(Ty::Uint),
                    "retain" => Some(Ty::Void),
                    "sum" => Some(t.clone()),
                    "min" | "max" => Some(Ty::Option(Box::new(t.clone()))),
                    "step" => Some(Ty::Vec(Box::new(t.clone()))),
                    _ => None,
                }
            }

            // ── map[K, V] ──────────────────────────────────────────────────
            Ty::Map(key, val) => {
                let k = *key.clone();
                let v = *val.clone();
                match method {
                    "len" => Some(Ty::Uint),
                    "is_empty" => Some(Ty::Bool),
                    "get" | "get_mut" => Some(Ty::Option(Box::new(v.clone()))),
                    "insert" => Some(Ty::Option(Box::new(v.clone()))),
                    "remove" => Some(Ty::Option(Box::new(v.clone()))),
                    "contains" | "contains_key" => Some(Ty::Bool),
                    "keys" => Some(Ty::Vec(Box::new(k.clone()))),
                    "values" => Some(Ty::Vec(Box::new(v.clone()))),
                    "entries" | "iter" => Some(Ty::Vec(Box::new(Ty::Tuple(vec![k, v])))),
                    "clear" => Some(Ty::Void),
                    "clone" => Some(Ty::Map(Box::new(k), Box::new(v.clone()))),
                    "len_keys" => Some(Ty::Uint),
                    _ => None,
                }
            }

            // ── set[T] ─────────────────────────────────────────────────────
            Ty::Set(elem) => {
                let t = *elem.clone();
                match method {
                    "len" => Some(Ty::Uint),
                    "is_empty" => Some(Ty::Bool),
                    "insert" | "remove" | "contains" => Some(Ty::Bool),
                    "union" | "intersection" | "difference" | "symmetric_difference" => {
                        Some(Ty::Set(Box::new(t.clone())))
                    }
                    "is_subset" | "is_superset" => Some(Ty::Bool),
                    "iter" | "clone" => Some(Ty::Set(Box::new(t.clone()))),
                    "clear" => Some(Ty::Void),
                    _ => None,
                }
            }

            // ── option[T] ─────────────────────────────────────────────────
            Ty::Option(inner) => {
                let t = *inner.clone();
                match method {
                    "is_some" | "is_none" => Some(Ty::Bool),
                    "unwrap" | "unwrap_or" | "unwrap_or_default" => Some(t.clone()),
                    "unwrap_or_else" => Some(t.clone()),
                    "expect" => Some(t.clone()),
                    "map" | "flat_map" | "and_then" | "filter_map" => {
                        Some(Ty::Option(Box::new(self.infer.new_var())))
                    }
                    "filter" => Some(Ty::Option(Box::new(t.clone()))),
                    "or" | "or_else" => Some(Ty::Option(Box::new(t.clone()))),
                    "and" => Some(Ty::Option(Box::new(self.infer.new_var()))),
                    "zip" => Some(Ty::Option(Box::new(Ty::Tuple(vec![
                        t.clone(),
                        self.infer.new_var(),
                    ])))),
                    "ok_or" | "ok_or_else" => Some(Ty::Result(
                        Box::new(t.clone()),
                        Box::new(self.infer.new_var()),
                    )),
                    "clone" => Some(Ty::Option(Box::new(t.clone()))),
                    "take" => Some(Ty::Option(Box::new(t.clone()))),
                    "replace" => Some(Ty::Option(Box::new(t.clone()))),
                    "inspect" => Some(Ty::Option(Box::new(t.clone()))),
                    _ => None,
                }
            }

            // ── result[T, E] ───────────────────────────────────────────────
            Ty::Result(ok_ty, err_ty) => {
                let t = *ok_ty.clone();
                let e = *err_ty.clone();
                match method {
                    "is_ok" | "is_err" => Some(Ty::Bool),
                    "unwrap" | "unwrap_or" | "unwrap_or_default" | "unwrap_or_else" => {
                        Some(t.clone())
                    }
                    "unwrap_err" => Some(e.clone()),
                    "expect" => Some(t.clone()),
                    "expect_err" => Some(e.clone()),
                    "ok" => Some(Ty::Option(Box::new(t.clone()))),
                    "err" => Some(Ty::Option(Box::new(e.clone()))),
                    "map" => Some(Ty::Result(
                        Box::new(self.infer.new_var()),
                        Box::new(e.clone()),
                    )),
                    "map_err" => Some(Ty::Result(
                        Box::new(t.clone()),
                        Box::new(self.infer.new_var()),
                    )),
                    "and_then" | "flat_map" => Some(Ty::Result(
                        Box::new(self.infer.new_var()),
                        Box::new(e.clone()),
                    )),
                    "or_else" | "or" => Some(Ty::Result(
                        Box::new(t.clone()),
                        Box::new(self.infer.new_var()),
                    )),
                    "clone" => Some(Ty::Result(Box::new(t), Box::new(e))),
                    "inspect" | "inspect_err" => {
                        Some(Ty::Result(Box::new(t.clone()), Box::new(e.clone())))
                    }
                    _ => None,
                }
            }

            // ── Numeric types ──────────────────────────────────────────────
            ty if ty.is_numeric() => match method {
                "abs" | "min" | "max" | "clamp" | "pow" => Some(ty.clone()),
                "to_string" => Some(Ty::Str),
                "clone" => Some(ty.clone()),
                "checked_add" | "checked_sub" | "checked_mul" | "checked_div" => {
                    Some(Ty::Option(Box::new(ty.clone())))
                }
                "wrapping_add" | "wrapping_sub" | "wrapping_mul" => Some(ty.clone()),
                "saturating_add" | "saturating_sub" => Some(ty.clone()),
                "sqrt" | "sin" | "cos" | "tan" | "ln" | "log2" | "log10" | "floor" | "ceil"
                | "round" | "exp" => Some(Ty::Float),
                "to_radians" | "to_degrees" => Some(Ty::Float),
                "is_nan" | "is_infinite" | "is_finite" => Some(Ty::Bool),
                "leading_zeros" | "trailing_zeros" | "count_ones" => Some(Ty::U32),
                _ => None,
            },

            // ── bool ───────────────────────────────────────────────────────
            Ty::Bool => match method {
                "to_string" => Some(Ty::Str),
                "clone" => Some(Ty::Bool),
                _ => None,
            },

            // ── char ───────────────────────────────────────────────────────
            Ty::Char => match method {
                "to_string" | "to_uppercase" | "to_lowercase" => Some(Ty::Str),
                "is_alphabetic" | "is_numeric" | "is_alphanumeric" | "is_whitespace"
                | "is_uppercase" | "is_lowercase" | "is_ascii" => Some(Ty::Bool),
                "to_digit" => Some(Ty::Option(Box::new(Ty::U32))),
                "as_u32" | "as_usize" => Some(Ty::U32),
                "clone" => Some(Ty::Char),
                _ => None,
            },

            // ── bytes ──────────────────────────────────────────────────────
            Ty::Bytes => match method {
                "len" => Some(Ty::Uint),
                "is_empty" => Some(Ty::Bool),
                "get" => Some(Ty::Option(Box::new(Ty::U8))),
                "to_string" | "as_str" => Some(Ty::Str),
                "clone" => Some(Ty::Bytes),
                _ => None,
            },

            // ── Tensor ─────────────────────────────────────────────────────
            Ty::Tensor => match method {
                "shape" | "size" => Some(Ty::Vec(Box::new(Ty::Uint))),
                "dtype" => Some(Ty::Str),
                "reshape" => Some(Ty::Tensor),
                "dot" => Some(Ty::Float),
                "matmul" | "transpose" | "flatten" | "expand_dims" | "squeeze" => Some(Ty::Tensor),
                "to_gpu" | "to_cpu" | "clone" => Some(Ty::Tensor),
                "sum" | "mean" | "min" | "max" | "std" | "var" => Some(Ty::Float),
                _ => None,
            },

            _ => None,
        }
    }

    // -----------------------------------------------------------------------
    // Statement checking
    // -----------------------------------------------------------------------

    pub fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let {
                pattern, ty, value, ..
            } => {
                let val_ty = self.infer_expr(value);
                let declared_ty = if let Some(ty_expr) = ty {
                    let declared = self.resolve_type_expr(ty_expr);
                    let mut errors = std::mem::take(&mut self.model.errors);
                    let unified =
                        self.infer
                            .unify(val_ty, declared.clone(), value.span(), &mut errors);
                    self.model.errors = errors;
                    unified
                } else {
                    val_ty
                };
                self.bind_pattern_types(pattern, declared_ty, false);
            }

            Stmt::LetMut {
                name, ty, value, ..
            } => {
                let declared_ty = self.resolve_type_expr(ty);
                self.check_expr(value, declared_ty.clone());
                if let Some(def_id) = self.lookup_def(name) {
                    self.model.type_env.insert(def_id, declared_ty);
                }
            }

            Stmt::Const {
                name, ty, value, ..
            } => {
                let declared_ty = self.resolve_type_expr(ty);
                self.check_expr(value, declared_ty.clone());
                if let Some(def_id) = self.lookup_def(name) {
                    self.model.type_env.insert(def_id, declared_ty);
                }
            }

            Stmt::Shared {
                name, ty, value, ..
            } => {
                let val_ty = self.infer_expr(value);
                let inner_ty = if let Some(ty_expr) = ty {
                    let declared = self.resolve_type_expr(ty_expr);
                    let mut errors = std::mem::take(&mut self.model.errors);
                    let unified =
                        self.infer
                            .unify(val_ty, declared.clone(), value.span(), &mut errors);
                    self.model.errors = errors;
                    unified
                } else {
                    val_ty
                };
                let shared_ty = Ty::Shared(Box::new(inner_ty));
                if let Some(def_id) = self.lookup_def(name) {
                    self.model.type_env.insert(def_id, shared_ty);
                }
            }

            Stmt::Expr { expr, .. } => {
                self.infer_expr(expr);
            }

            Stmt::Return { value, .. } => {
                if let Some(v) = value {
                    if let Some(ret_ty) = self.current_fn_ret.clone() {
                        self.check_expr(v, ret_ty);
                    } else {
                        self.infer_expr(v);
                    }
                }
            }

            Stmt::Break { value, .. } => {
                if let Some(v) = value {
                    self.infer_expr(v);
                }
            }

            Stmt::Next { .. } => {}

            Stmt::Defer { body, .. } => {
                self.infer_expr(body);
            }

            Stmt::Guard {
                condition,
                else_body,
                ..
            } => {
                self.check_expr(condition, Ty::Bool);
                self.infer_expr(else_body);
            }

            Stmt::Assign { target, value, .. } => {
                let target_ty = self.infer_expr(target);
                self.check_expr(value, target_ty);
            }

            Stmt::CompoundAssign { target, value, .. } => {
                let target_ty = self.infer_expr(target);
                self.check_expr(value, target_ty);
            }

            Stmt::Item { item, .. } => {
                self.collect_definitions(item);
                self.check_item(item);
            }
        }
    }

    // -----------------------------------------------------------------------
    // TypeExpr resolution
    // -----------------------------------------------------------------------

    pub fn resolve_type_expr(&mut self, ty: &TypeExpr) -> Ty {
        match ty {
            TypeExpr::Named { name, .. } => self.resolve_named_type(&name.name),

            TypeExpr::Generic { name, args, .. } => {
                let resolved: Vec<Ty> = args.iter().map(|a| self.resolve_type_expr(a)).collect();
                match name.name.as_str() {
                    "vec" => Ty::Vec(Box::new(
                        resolved
                            .into_iter()
                            .next()
                            .unwrap_or_else(|| self.infer.new_var()),
                    )),
                    "map" => {
                        let mut it = resolved.into_iter();
                        let k = it.next().unwrap_or_else(|| self.infer.new_var());
                        let v = it.next().unwrap_or_else(|| self.infer.new_var());
                        Ty::Map(Box::new(k), Box::new(v))
                    }
                    "set" => Ty::Set(Box::new(
                        resolved
                            .into_iter()
                            .next()
                            .unwrap_or_else(|| self.infer.new_var()),
                    )),
                    "option" => Ty::Option(Box::new(
                        resolved
                            .into_iter()
                            .next()
                            .unwrap_or_else(|| self.infer.new_var()),
                    )),
                    "result" => {
                        let mut it = resolved.into_iter();
                        let t = it.next().unwrap_or_else(|| self.infer.new_var());
                        let e = it.next().unwrap_or(Ty::Str);
                        Ty::Result(Box::new(t), Box::new(e))
                    }
                    _ => {
                        let def_id = self
                            .model
                            .environment
                            .resolve(&name.name)
                            .unwrap_or(crate::symbol::DefId(usize::MAX - 20));
                        Ty::Named {
                            def_id,
                            name: name.name.clone(),
                            generics: resolved,
                        }
                    }
                }
            }

            TypeExpr::Tuple { elements, .. } => {
                Ty::Tuple(elements.iter().map(|e| self.resolve_type_expr(e)).collect())
            }

            TypeExpr::Array { element, size, .. } => {
                let elem_ty = self.resolve_type_expr(element);
                let sz = self.eval_const_size(size).unwrap_or(0);
                Ty::Array {
                    element: Box::new(elem_ty),
                    size: sz,
                }
            }

            TypeExpr::Closure { params, ret, .. } => {
                let param_tys = params.iter().map(|p| self.resolve_type_expr(p)).collect();
                let ret_ty = self.resolve_type_expr(ret);
                Ty::Fn {
                    params: param_tys,
                    ret: Box::new(ret_ty),
                    is_async: false,
                }
            }

            TypeExpr::Void { .. } => Ty::Void,
            TypeExpr::Never { .. } => Ty::Never,
            TypeExpr::SelfType { .. } => self.current_self_ty.clone().unwrap_or(Ty::SelfTy),
            TypeExpr::Ref { inner, .. } => self.resolve_type_expr(inner),
            TypeExpr::Inferred { .. } => self.infer.new_var(),
        }
    }

    fn resolve_named_type(&self, name: &str) -> Ty {
        match name {
            "bool" => Ty::Bool,
            "int" => Ty::Int,
            "uint" => Ty::Uint,
            "float" => Ty::Float,
            "i8" => Ty::I8,
            "i16" => Ty::I16,
            "i32" => Ty::I32,
            "i64" => Ty::I64,
            "i128" => Ty::I128,
            "isize" => Ty::Isize,
            "u8" => Ty::U8,
            "u16" => Ty::U16,
            "u32" => Ty::U32,
            "u64" => Ty::U64,
            "u128" => Ty::U128,
            "usize" => Ty::Usize,
            "f32" => Ty::F32,
            "f64" => Ty::F64,
            "char" => Ty::Char,
            "str" => Ty::Str,
            "bytes" => Ty::Bytes,
            "void" => Ty::Void,
            "never" => Ty::Never,
            "tensor" => Ty::Tensor,
            _ => {
                if let Some(def_id) = self.model.environment.resolve(name) {
                    // If this name is a type alias, return the aliased type
                    // directly (transparent aliases: `alias UserId = int`
                    // means `UserId` and `int` are interchangeable).
                    if let Some(sym) = self.model.symbol_table.get(def_id) {
                        if matches!(sym.kind, crate::symbol::SymbolKind::TypeAlias) {
                            if let Some(aliased) = self.model.type_env.get(&def_id) {
                                // Only unwrap if the aliased type is not itself
                                // a Named type (avoid infinite recursion for
                                // aliases of user-defined types).
                                match aliased {
                                    Ty::Named { .. } => {}
                                    other => return other.clone(),
                                }
                            }
                        }
                    }
                    Ty::Named {
                        def_id,
                        name: name.to_string(),
                        generics: vec![],
                    }
                } else {
                    Ty::Error
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Field lookup
    // -----------------------------------------------------------------------

    fn lookup_field(&mut self, ty: &Ty, field: &str, span: Span) -> Ty {
        let base = ty.peel_shared().clone();
        match &base {
            Ty::Named { def_id, .. } => {
                if let Some(fields) = self.struct_fields.get(def_id).cloned() {
                    if let Some((_, fty)) = fields.iter().find(|(n, _)| n == field) {
                        return fty.clone();
                    }
                }
                // Unknown field — return a fresh inference var so we don't cascade.
                self.infer.new_var()
            }
            Ty::Tuple(elements) => {
                if let Ok(idx) = field.parse::<usize>() {
                    elements.get(idx).cloned().unwrap_or(Ty::Error)
                } else {
                    Ty::Error
                }
            }
            Ty::Str => match field {
                "len" => Ty::Uint,
                _ => self.infer.new_var(),
            },
            Ty::Infer(_) | Ty::Error => Ty::Error,
            _ => self.infer.new_var(),
        }
    }

    // -----------------------------------------------------------------------
    // Pattern binding
    // -----------------------------------------------------------------------

    fn bind_pattern_types(&mut self, pat: &Pattern, ty: Ty, is_mut: bool) {
        match pat {
            Pattern::Binding { name, span } => {
                if let Some(def_id) = self.lookup_span_def(*span) {
                    self.model.type_env.insert(def_id, ty);
                }
            }

            Pattern::Wildcard { .. } => {}

            Pattern::Literal { .. } => {}

            Pattern::Tuple { elements, .. } => {
                if let Ty::Tuple(types) = &ty {
                    for (elem_pat, elem_ty) in elements.iter().zip(types.iter()) {
                        self.bind_pattern_types(elem_pat, elem_ty.clone(), is_mut);
                    }
                } else {
                    for elem_pat in elements {
                        let fresh = self.infer.new_var();
                        self.bind_pattern_types(elem_pat, fresh, is_mut);
                    }
                }
            }

            Pattern::Some { inner, .. } => {
                let inner_ty = match ty {
                    Ty::Option(t) => *t,
                    _ => self.infer.new_var(),
                };
                self.bind_pattern_types(inner, inner_ty, is_mut);
            }

            Pattern::None { .. } => {}

            Pattern::Ok { inner, .. } => {
                let inner_ty = match ty {
                    Ty::Result(t, _) => *t,
                    _ => self.infer.new_var(),
                };
                self.bind_pattern_types(inner, inner_ty, is_mut);
            }

            Pattern::Err { inner, .. } => {
                let inner_ty = match ty {
                    Ty::Result(_, e) => *e,
                    _ => self.infer.new_var(),
                };
                self.bind_pattern_types(inner, inner_ty, is_mut);
            }

            Pattern::Struct { fields, .. } => {
                for field in fields {
                    let fty = self.lookup_field(&ty, &field.name.name, field.span);
                    let bind_ident = field.rename.as_ref().unwrap_or(&field.name);
                    if let Some(def_id) = self.lookup_span_def(bind_ident.span) {
                        self.model.type_env.insert(def_id, fty);
                    }
                }
            }

            Pattern::EnumPositional { args, .. } => {
                for arg_pat in args {
                    let fresh = self.infer.new_var();
                    self.bind_pattern_types(arg_pat, fresh, is_mut);
                }
            }

            Pattern::EnumNamed { fields, .. } => {
                for field in fields {
                    let fty = self.lookup_field(&ty, &field.name.name, field.span);
                    let bind_ident = field.rename.as_ref().unwrap_or(&field.name);
                    if let Some(def_id) = self.lookup_span_def(bind_ident.span) {
                        self.model.type_env.insert(def_id, fty);
                    }
                }
            }

            Pattern::EnumUnit { .. } => {}

            Pattern::TupleStruct { fields, .. } => {
                for field_pat in fields {
                    let fresh = self.infer.new_var();
                    self.bind_pattern_types(field_pat, fresh, is_mut);
                }
            }

            Pattern::Or { patterns, .. } => {
                for p in patterns {
                    self.bind_pattern_types(p, ty.clone(), is_mut);
                }
            }

            Pattern::Range { .. } => {}
        }
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Look up the `DefId` of a named identifier via the environment.
    fn lookup_def(&self, ident: &Ident) -> Option<DefId> {
        self.model.environment.resolve(&ident.name)
    }

    /// Look up the `DefId` of an identifier span via resolutions.
    fn lookup_span_def(&self, span: Span) -> Option<DefId> {
        self.model.resolutions.get(&span).copied()
    }

    /// Resolve a function's full type signature as `Ty::Fn`.
    fn resolve_fn_sig(&mut self, fndef: &FnDef) -> Ty {
        let param_tys: Vec<Ty> = fndef
            .params
            .iter()
            .map(|p| {
                p.ty.as_ref()
                    .map(|t| self.resolve_type_expr(t))
                    .unwrap_or_else(|| self.infer.new_var())
            })
            .collect();
        let ret_ty = fndef
            .return_type
            .as_ref()
            .map(|t| self.resolve_type_expr(t))
            .unwrap_or(Ty::Void);
        Ty::Fn {
            params: param_tys,
            ret: Box::new(ret_ty),
            is_async: fndef.is_async,
        }
    }

    /// Given an iterable type, return the element type.
    fn elem_type_of(&mut self, ty: &Ty, span: Span) -> Ty {
        match ty {
            Ty::Vec(t) | Ty::Set(t) | Ty::Array { element: t, .. } => *t.clone(),
            Ty::Map(k, v) => Ty::Tuple(vec![*k.clone(), *v.clone()]),
            Ty::Str => Ty::Char,
            Ty::Tensor => Ty::Float,
            Ty::Named {
                def_id,
                name,
                generics,
            } => {
                // Range type — element is the first generic arg.
                if name == "Range" {
                    generics.first().cloned().unwrap_or(Ty::Int)
                } else {
                    self.infer.new_var()
                }
            }
            Ty::Infer(_) | Ty::Error => self.infer.new_var(),
            _ => self.infer.new_var(),
        }
    }

    /// Try to evaluate a constant integer expression (for array sizes).
    fn eval_const_size(&self, expr: &Expr) -> Option<u64> {
        match expr {
            Expr::Literal {
                lit: Literal::Int { raw, .. },
                ..
            } => {
                let clean = raw.trim_end_matches(|c: char| c.is_alphabetic());
                if clean.starts_with("0x") || clean.starts_with("0X") {
                    u64::from_str_radix(&clean[2..], 16).ok()
                } else if clean.starts_with("0b") || clean.starts_with("0B") {
                    u64::from_str_radix(&clean[2..], 2).ok()
                } else if clean.starts_with("0o") || clean.starts_with("0O") {
                    u64::from_str_radix(&clean[2..], 8).ok()
                } else {
                    clean.replace('_', "").parse::<u64>().ok()
                }
            }
            _ => None,
        }
    }

    /// Push a custom semantic error.
    fn push_error(&mut self, message: String, span: Span) {
        self.model
            .errors
            .push(SemanticError::Custom { message, span });
    }

    // -----------------------------------------------------------------------
    // Finalization
    // -----------------------------------------------------------------------

    /// Apply all remaining substitutions to every recorded type, and replace
    /// any unsolved inference variables with their default concrete types.
    pub fn finalize(&mut self) {
        // Apply to expr_types.
        let spans: Vec<Span> = self.model.expr_types.keys().cloned().collect();
        for span in spans {
            let ty = self.model.expr_types.remove(&span).unwrap();
            let applied = self.infer.apply(ty);
            let final_ty = Self::default_infer_vars(applied);
            self.model.expr_types.insert(span, final_ty);
        }

        // Apply to type_env.
        let ids: Vec<DefId> = self.model.type_env.keys().cloned().collect();
        for id in ids {
            let ty = self.model.type_env.remove(&id).unwrap();
            let applied = self.infer.apply(ty);
            let final_ty = Self::default_infer_vars(applied);
            self.model.type_env.insert(id, final_ty);
        }
    }

    /// Replace any remaining `Ty::Infer` variables with `Ty::Int` (the
    /// integer-defaulting rule, like Rust defaulting unconstrained integer
    /// literals to `i32`).
    fn default_infer_vars(ty: Ty) -> Ty {
        match ty {
            Ty::Infer(_) => Ty::Int,
            Ty::Vec(t) => Ty::Vec(Box::new(Self::default_infer_vars(*t))),
            Ty::Map(k, v) => Ty::Map(
                Box::new(Self::default_infer_vars(*k)),
                Box::new(Self::default_infer_vars(*v)),
            ),
            Ty::Set(t) => Ty::Set(Box::new(Self::default_infer_vars(*t))),
            Ty::Option(t) => Ty::Option(Box::new(Self::default_infer_vars(*t))),
            Ty::Result(t, e) => Ty::Result(
                Box::new(Self::default_infer_vars(*t)),
                Box::new(Self::default_infer_vars(*e)),
            ),
            Ty::Tuple(elements) => {
                Ty::Tuple(elements.into_iter().map(Self::default_infer_vars).collect())
            }
            Ty::Array { element, size } => Ty::Array {
                element: Box::new(Self::default_infer_vars(*element)),
                size,
            },
            Ty::Named {
                def_id,
                name,
                generics,
            } => Ty::Named {
                def_id,
                name,
                generics: generics.into_iter().map(Self::default_infer_vars).collect(),
            },
            Ty::Fn {
                params,
                ret,
                is_async,
            } => Ty::Fn {
                params: params.into_iter().map(Self::default_infer_vars).collect(),
                ret: Box::new(Self::default_infer_vars(*ret)),
                is_async,
            },
            Ty::Shared(t) => Ty::Shared(Box::new(Self::default_infer_vars(*t))),
            other => other,
        }
    }
}
