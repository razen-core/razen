//! AST → MIR Lowering Pass.
//!
//! The `Lowerer` walks a parsed + type-checked Razen module and produces a
//! complete `MirProgram`.  Every expression is lowered into a sequence of
//! MIR instructions, with the result placed in a fresh local-variable slot.
//!
//! # Algorithm
//!
//! 1. **First pass** — lower all struct / enum type definitions.
//! 2. **Second pass** — lower all function / impl bodies.
//!
//! Each function is handled by a `FnBuilder` which maintains:
//!   - The `MirFn` under construction.
//!   - A `local_map: HashMap<DefId, LocalId>` mapping resolved source
//!     bindings to their MIR local slots.
//!   - A `current_block: BlockId` pointing to the block currently being
//!     appended to.
//!   - A `loop_stack` for `break` / `next` resolution.
//!   - A `terminated: bool` flag so we never append to a sealed block.

use std::collections::HashMap;

use razen_ast::expr::{Expr, ForkKind, LoopKind};
use razen_ast::ident::Ident;
use razen_ast::item::{EnumVariantKind, FnBody, FnDef, ImplBlock, Item, StructKind, Visibility};
use razen_ast::lit::Literal;
use razen_ast::module::Module;
use razen_ast::ops::{BinOp, CompoundOp};
use razen_ast::pat::Pattern;
use razen_ast::stmt::Stmt as AstStmt;
use razen_ast::types::TypeExpr;
use razen_sema::{DefId, SemanticModel, Ty};

use crate::func::{LocalDecl, MirFn};
use crate::inst::{Inst, RValue, Terminator};
use crate::program::{MirEnum, MirProgram, MirStruct, MirVariant, MirVariantKind};
use crate::ty::MirTy;
use crate::value::{BlockId, Const, LocalId, Operand};

// ---------------------------------------------------------------------------
// Loop context (for break / next)
// ---------------------------------------------------------------------------

struct LoopCtx {
    /// Block to jump to when `break` is executed.
    break_block: BlockId,
    /// Block to jump to when `next` (continue) is executed.
    continue_block: BlockId,
    /// If the loop is used as an expression, this local accumulates the value
    /// written by `break value`.
    break_value_local: Option<LocalId>,
}

// ---------------------------------------------------------------------------
// FnBuilder — per-function lowering state
// ---------------------------------------------------------------------------

struct FnBuilder<'a> {
    func: MirFn,
    sema: &'a SemanticModel,
    /// Maps a binding `DefId` to the `LocalId` that holds its value.
    local_map: HashMap<DefId, LocalId>,
    /// The basic block currently being appended to.
    current_block: BlockId,
    /// Stack of loop contexts (innermost last).
    loop_stack: Vec<LoopCtx>,
    /// Whether the current block has already been terminated.
    terminated: bool,
}

impl<'a> FnBuilder<'a> {
    fn new(
        sema: &'a SemanticModel,
        name: String,
        ret_ty: MirTy,
        is_async: bool,
        is_pub: bool,
    ) -> Self {
        let func = MirFn::new(name, ret_ty, is_async, is_pub);
        let entry = func.entry();
        Self {
            func,
            sema,
            local_map: HashMap::new(),
            current_block: entry,
            loop_stack: Vec::new(),
            terminated: false,
        }
    }

    // ── Block management ──────────────────────────────────────────────────

    fn new_block(&mut self) -> BlockId {
        self.func.new_block()
    }

    fn switch_to(&mut self, block: BlockId) {
        self.current_block = block;
        self.terminated = false;
    }

    fn is_terminated(&self) -> bool {
        self.terminated
    }

    // ── Instruction emission ──────────────────────────────────────────────

    fn emit(&mut self, inst: Inst) {
        if !self.terminated {
            self.func.emit_to(self.current_block, inst);
        }
    }

    fn emit_assign(&mut self, dest: LocalId, value: RValue) {
        self.emit(Inst::assign(dest, value));
    }

    fn terminate(&mut self, term: Terminator) {
        if !self.terminated {
            self.func.terminate(self.current_block, term);
            self.terminated = true;
        }
    }

    // ── Local allocation ──────────────────────────────────────────────────

    fn alloc_tmp(&mut self, ty: MirTy) -> LocalId {
        self.func.new_tmp(ty)
    }

    fn alloc_named(&mut self, name: &str, ty: MirTy, is_mut: bool) -> LocalId {
        self.func.new_named(name, ty, is_mut)
    }

    // ── Type helpers ──────────────────────────────────────────────────────

    /// Get the MIR type of an expression from the semantic model.
    fn expr_ty(&self, expr: &Expr) -> MirTy {
        self.sema
            .expr_types
            .get(&expr.span())
            .map(MirTy::from_sema)
            .unwrap_or(MirTy::Opaque)
    }

    /// Get the MIR type of a binding from the semantic model.
    fn def_ty(&self, id: DefId) -> MirTy {
        self.sema
            .type_env
            .get(&id)
            .map(MirTy::from_sema)
            .unwrap_or(MirTy::Opaque)
    }

    /// Look up the `DefId` for an identifier span.
    fn resolve_ident_span(&self, span: razen_lexer::Span) -> Option<DefId> {
        self.sema.resolutions.get(&span).copied()
    }

    // ── Main expression lowering ──────────────────────────────────────────

    /// Lower `expr` and return the `LocalId` that holds the result.
    ///
    /// Every path through this function **must** return a valid `LocalId`
    /// pointing to the result (even for diverging expressions — in that case
    /// we still allocate a `Never`-typed slot).
    fn lower_expr(&mut self, expr: &Expr) -> LocalId {
        let ty = self.expr_ty(expr);
        match expr {
            // ── Literals ─────────────────────────────────────────────────
            Expr::Literal { lit, .. } => {
                let c = lower_literal(lit);
                let tmp = self.alloc_tmp(ty.clone());
                self.emit_assign(tmp, RValue::Use(Operand::Const(c)));
                tmp
            }

            // ── Identifier ───────────────────────────────────────────────
            Expr::Ident { ident, span } => {
                // Special handling for `none` — it's a value, not a binding.
                if ident.name == "none" {
                    let tmp = self.alloc_tmp(ty.clone());
                    self.emit_assign(tmp, RValue::None { ty: ty.clone() });
                    return tmp;
                }

                if let Some(def_id) = self.resolve_ident_span(*span) {
                    if let Some(&local_id) = self.local_map.get(&def_id) {
                        return local_id;
                    }
                }

                // Unknown identifier — allocate an opaque placeholder so
                // lowering can continue.
                let tmp = self.alloc_tmp(MirTy::Opaque);
                self.emit_assign(tmp, RValue::Use(Operand::Const(Const::Null)));
                tmp
            }

            // ── Paren ────────────────────────────────────────────────────
            Expr::Paren { inner, .. } => self.lower_expr(inner),

            // ── Unsafe ───────────────────────────────────────────────────
            Expr::Unsafe { body, .. } => self.lower_expr(body),

            // ── Placeholder ──────────────────────────────────────────────
            Expr::Placeholder { .. } => {
                let tmp = self.alloc_tmp(MirTy::Opaque);
                self.emit_assign(tmp, RValue::Use(Operand::Const(Const::Null)));
                tmp
            }

            // ── Binary operation ─────────────────────────────────────────
            Expr::Binary {
                left, op, right, ..
            } => {
                let l = self.lower_expr(left);
                let r = self.lower_expr(right);
                let tmp = self.alloc_tmp(ty.clone());
                self.emit_assign(
                    tmp,
                    RValue::BinOp {
                        op: *op,
                        left: Operand::Local(l),
                        right: Operand::Local(r),
                    },
                );
                tmp
            }

            // ── Unary operation ──────────────────────────────────────────
            Expr::Unary { op, operand, .. } => {
                let o = self.lower_expr(operand);
                let tmp = self.alloc_tmp(ty.clone());
                self.emit_assign(
                    tmp,
                    RValue::UnaryOp {
                        op: *op,
                        operand: Operand::Local(o),
                    },
                );
                tmp
            }

            // ── Field access ─────────────────────────────────────────────
            Expr::Field { object, field, .. } => {
                let obj = self.lower_expr(object);
                let tmp = self.alloc_tmp(ty.clone());
                self.emit_assign(
                    tmp,
                    RValue::Field {
                        base: Operand::Local(obj),
                        field: field.name.clone(),
                    },
                );
                tmp
            }

            // ── Index access ─────────────────────────────────────────────
            Expr::Index { object, index, .. } => {
                let obj = self.lower_expr(object);
                let idx = self.lower_expr(index);
                let tmp = self.alloc_tmp(ty.clone());
                self.emit_assign(
                    tmp,
                    RValue::Index {
                        base: Operand::Local(obj),
                        index: Operand::Local(idx),
                    },
                );
                tmp
            }

            // ── Method call ──────────────────────────────────────────────
            Expr::MethodCall {
                object,
                method,
                args,
                ..
            } => {
                let obj = self.lower_expr(object);
                let mut arg_ops = vec![Operand::Local(obj)];
                for a in args.iter() {
                    let la = self.lower_expr(a);
                    arg_ops.push(Operand::Local(la));
                }
                let result = self.alloc_tmp(ty.clone());
                let cont = self.new_block();
                // Encode the method name in the callee operand as a Str const
                // so codegen can emit the right call.
                let method_callee =
                    Operand::Const(Const::Str(format!("__method__{}", method.name)));
                self.terminate(Terminator::Call {
                    callee: method_callee,
                    args: arg_ops,
                    dest: Some(result),
                    target: cont,
                });
                self.switch_to(cont);
                result
            }

            // ── Function call ────────────────────────────────────────────
            Expr::Call { callee, args, .. } => {
                let callee_local = self.lower_expr(callee);
                let mut arg_ops = Vec::new();
                for a in args.iter() {
                    let la = self.lower_expr(a);
                    arg_ops.push(Operand::Local(la));
                }
                let result = self.alloc_tmp(ty.clone());
                let cont = self.new_block();
                self.terminate(Terminator::Call {
                    callee: Operand::Local(callee_local),
                    args: arg_ops,
                    dest: Some(result),
                    target: cont,
                });
                self.switch_to(cont);
                result
            }

            // ── Block ────────────────────────────────────────────────────
            Expr::Block { stmts, tail, .. } => {
                for stmt in stmts.iter() {
                    self.lower_stmt(stmt);
                    if self.is_terminated() {
                        break;
                    }
                }
                if let Some(tail_expr) = tail {
                    self.lower_expr(tail_expr)
                } else {
                    let tmp = self.alloc_tmp(MirTy::Void);
                    self.emit_assign(tmp, RValue::Use(Operand::unit()));
                    tmp
                }
            }

            // ── If / else ────────────────────────────────────────────────
            Expr::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                let cond = self.lower_expr(condition);
                let then_bb = self.new_block();
                let else_bb = self.new_block();
                let join_bb = self.new_block();
                let result = self.alloc_tmp(ty.clone());

                self.terminate(Terminator::Branch {
                    cond: Operand::Local(cond),
                    then_block: then_bb,
                    else_block: else_bb,
                });

                // Then branch
                self.switch_to(then_bb);
                let then_val = self.lower_expr(then_block);
                if !self.is_terminated() {
                    self.emit_assign(result, RValue::Use(Operand::Local(then_val)));
                    self.terminate(Terminator::Goto(join_bb));
                }

                // Else branch
                self.switch_to(else_bb);
                if let Some(else_expr) = else_block {
                    let else_val = self.lower_expr(else_expr);
                    if !self.is_terminated() {
                        self.emit_assign(result, RValue::Use(Operand::Local(else_val)));
                        self.terminate(Terminator::Goto(join_bb));
                    }
                } else {
                    self.emit_assign(result, RValue::Use(Operand::unit()));
                    self.terminate(Terminator::Goto(join_bb));
                }

                self.switch_to(join_bb);
                result
            }

            // ── If let ───────────────────────────────────────────────────
            Expr::IfLet {
                pattern,
                value,
                then_block,
                else_block,
                ..
            } => {
                let val = self.lower_expr(value);
                let then_bb = self.new_block();
                let else_bb = self.new_block();
                let join_bb = self.new_block();
                let result = self.alloc_tmp(ty.clone());

                // Use the discriminant to branch (1 = some/ok, 0 = none/err)
                let disc = self.alloc_tmp(MirTy::I64);
                self.emit_assign(disc, RValue::Discriminant(Operand::Local(val)));
                self.terminate(Terminator::Branch {
                    cond: Operand::Local(disc),
                    then_block: then_bb,
                    else_block: else_bb,
                });

                self.switch_to(then_bb);
                self.bind_pattern(pattern, val);
                let then_val = self.lower_expr(then_block);
                if !self.is_terminated() {
                    self.emit_assign(result, RValue::Use(Operand::Local(then_val)));
                    self.terminate(Terminator::Goto(join_bb));
                }

                self.switch_to(else_bb);
                if let Some(else_expr) = else_block {
                    let else_val = self.lower_expr(else_expr);
                    if !self.is_terminated() {
                        self.emit_assign(result, RValue::Use(Operand::Local(else_val)));
                        self.terminate(Terminator::Goto(join_bb));
                    }
                } else {
                    self.emit_assign(result, RValue::Use(Operand::unit()));
                    self.terminate(Terminator::Goto(join_bb));
                }

                self.switch_to(join_bb);
                result
            }

            // ── Match ────────────────────────────────────────────────────
            Expr::Match { subject, arms, .. } => {
                let subj = self.lower_expr(subject);
                let disc = self.alloc_tmp(MirTy::I64);
                self.emit_assign(disc, RValue::Discriminant(Operand::Local(subj)));

                let result = self.alloc_tmp(ty.clone());
                let join_bb = self.new_block();

                // Allocate a block per arm.
                let arm_blocks: Vec<BlockId> = (0..arms.len()).map(|_| self.new_block()).collect();

                let default_bb = arm_blocks.last().copied().unwrap_or(join_bb);

                // Build switch arms for literal / enum-unit patterns.
                let switch_arms: Vec<(i64, BlockId)> = arms
                    .iter()
                    .zip(arm_blocks.iter())
                    .filter_map(|(arm, &blk)| pattern_discriminant(&arm.pattern).map(|d| (d, blk)))
                    .collect();

                self.terminate(Terminator::Switch {
                    value: Operand::Local(disc),
                    arms: switch_arms,
                    otherwise: default_bb,
                });

                // Lower each arm body.
                for (arm, &arm_bb) in arms.iter().zip(arm_blocks.iter()) {
                    self.switch_to(arm_bb);
                    self.bind_pattern(&arm.pattern, subj);
                    if let Some(guard_expr) = &arm.guard {
                        // Guard: if it fails we fall through to next arm
                        // (simplified: we just evaluate it for side effects).
                        self.lower_expr(guard_expr);
                    }
                    let arm_val = self.lower_expr(&arm.body);
                    if !self.is_terminated() {
                        self.emit_assign(result, RValue::Use(Operand::Local(arm_val)));
                        self.terminate(Terminator::Goto(join_bb));
                    }
                }

                self.switch_to(join_bb);
                result
            }

            // ── Loop ─────────────────────────────────────────────────────
            Expr::Loop { kind, body, .. } => {
                let loop_header = self.new_block();
                let loop_body = self.new_block();
                let loop_exit = self.new_block();
                let result = self.alloc_tmp(ty.clone());

                // Jump into the loop header
                self.terminate(Terminator::Goto(loop_header));
                self.switch_to(loop_header);

                match kind {
                    LoopKind::Infinite => {
                        self.terminate(Terminator::Goto(loop_body));
                    }
                    LoopKind::While { condition } => {
                        let cond = self.lower_expr(condition);
                        self.terminate(Terminator::Branch {
                            cond: Operand::Local(cond),
                            then_block: loop_body,
                            else_block: loop_exit,
                        });
                    }
                    LoopKind::ForIn { binding, iterable } => {
                        let iter = self.lower_expr(iterable);
                        // Call __iter_next(iter) → option[T]
                        let next_opt = self.alloc_tmp(MirTy::Opaque);
                        let check_bb = self.new_block();
                        self.terminate(Terminator::Call {
                            callee: Operand::Const(Const::Str("__iter_next".into())),
                            args: vec![Operand::Local(iter)],
                            dest: Some(next_opt),
                            target: check_bb,
                        });
                        self.switch_to(check_bb);
                        let disc = self.alloc_tmp(MirTy::I64);
                        self.emit_assign(disc, RValue::Discriminant(Operand::Local(next_opt)));
                        self.terminate(Terminator::Branch {
                            cond: Operand::Local(disc),
                            then_block: loop_body,
                            else_block: loop_exit,
                        });
                        // In loop_body: unwrap the option and bind the pattern
                        self.switch_to(loop_body);
                        let elem = self.alloc_tmp(MirTy::Opaque);
                        self.emit_assign(
                            elem,
                            RValue::Field {
                                base: Operand::Local(next_opt),
                                field: "0".to_string(),
                            },
                        );
                        self.bind_pattern(binding, elem);
                        // Fall through to the loop body start
                        let body_start = self.new_block();
                        self.terminate(Terminator::Goto(body_start));
                        self.switch_to(body_start);
                    }
                }

                self.loop_stack.push(LoopCtx {
                    break_block: loop_exit,
                    continue_block: loop_header,
                    break_value_local: Some(result),
                });

                if !matches!(kind, LoopKind::ForIn { .. }) {
                    // For ForIn we're already positioned in body_start
                    self.switch_to(loop_body);
                }

                self.lower_expr(body);

                if !self.is_terminated() {
                    self.terminate(Terminator::Goto(loop_header));
                }

                self.loop_stack.pop();
                self.switch_to(loop_exit);
                result
            }

            // ── Loop let ─────────────────────────────────────────────────
            Expr::LoopLet {
                pattern,
                value,
                body,
                ..
            } => {
                let loop_header = self.new_block();
                let loop_body = self.new_block();
                let loop_exit = self.new_block();
                let result = self.alloc_tmp(ty.clone());

                self.terminate(Terminator::Goto(loop_header));
                self.switch_to(loop_header);

                let val = self.lower_expr(value);
                let disc = self.alloc_tmp(MirTy::I64);
                self.emit_assign(disc, RValue::Discriminant(Operand::Local(val)));
                self.terminate(Terminator::Branch {
                    cond: Operand::Local(disc),
                    then_block: loop_body,
                    else_block: loop_exit,
                });

                self.loop_stack.push(LoopCtx {
                    break_block: loop_exit,
                    continue_block: loop_header,
                    break_value_local: Some(result),
                });

                self.switch_to(loop_body);
                self.bind_pattern(pattern, val);
                self.lower_expr(body);
                if !self.is_terminated() {
                    self.terminate(Terminator::Goto(loop_header));
                }

                self.loop_stack.pop();
                self.switch_to(loop_exit);
                result
            }

            // ── Closure ──────────────────────────────────────────────────
            Expr::Closure {
                params, body: _, ..
            } => {
                // Simplified: represent as a closure constant.
                // Full closure conversion (capture analysis) is deferred to
                // a separate pass.
                let closure_name =
                    format!("__closure_{}_{}", self.func.name, self.func.block_count());
                let tmp = self.alloc_tmp(ty.clone());
                self.emit_assign(
                    tmp,
                    RValue::Closure {
                        func_name: closure_name,
                        captures: vec![],
                    },
                );
                tmp
            }

            // ── Tuple ────────────────────────────────────────────────────
            Expr::Tuple { elements, .. } => {
                let ops: Vec<Operand> = elements
                    .iter()
                    .map(|e| Operand::Local(self.lower_expr(e)))
                    .collect();
                let tmp = self.alloc_tmp(ty.clone());
                self.emit_assign(tmp, RValue::Tuple(ops));
                tmp
            }

            // ── Vec literal ───────────────────────────────────────────────
            Expr::Vec { elements, .. } => {
                let elem_ty = match &ty {
                    MirTy::Vec(t) => *t.clone(),
                    _ => MirTy::Opaque,
                };
                let ops: Vec<Operand> = elements
                    .iter()
                    .map(|e| Operand::Local(self.lower_expr(e)))
                    .collect();
                let tmp = self.alloc_tmp(ty.clone());
                self.emit_assign(
                    tmp,
                    RValue::Vec {
                        elem_ty,
                        elements: ops,
                    },
                );
                tmp
            }

            // ── Map literal ───────────────────────────────────────────────
            Expr::Map { entries, .. } => {
                let (key_ty, val_ty) = match &ty {
                    MirTy::Map(k, v) => (*k.clone(), *v.clone()),
                    _ => (MirTy::Opaque, MirTy::Opaque),
                };
                let kvs: Vec<(Operand, Operand)> = entries
                    .iter()
                    .map(|(k, v)| {
                        (
                            Operand::Local(self.lower_expr(k)),
                            Operand::Local(self.lower_expr(v)),
                        )
                    })
                    .collect();
                let tmp = self.alloc_tmp(ty.clone());
                self.emit_assign(
                    tmp,
                    RValue::Map {
                        key_ty,
                        val_ty,
                        entries: kvs,
                    },
                );
                tmp
            }

            // ── Set literal ───────────────────────────────────────────────
            Expr::Set { elements, .. } => {
                let elem_ty = match &ty {
                    MirTy::Set(t) => *t.clone(),
                    _ => MirTy::Opaque,
                };
                let ops: Vec<Operand> = elements
                    .iter()
                    .map(|e| Operand::Local(self.lower_expr(e)))
                    .collect();
                let tmp = self.alloc_tmp(ty.clone());
                self.emit_assign(
                    tmp,
                    RValue::Set {
                        elem_ty,
                        elements: ops,
                    },
                );
                tmp
            }

            // ── Array literal ─────────────────────────────────────────────
            Expr::Array { elements, .. } => {
                let elem_ty = match &ty {
                    MirTy::Array { element, .. } => *element.clone(),
                    _ => MirTy::Opaque,
                };
                let ops: Vec<Operand> = elements
                    .iter()
                    .map(|e| Operand::Local(self.lower_expr(e)))
                    .collect();
                let tmp = self.alloc_tmp(ty.clone());
                self.emit_assign(
                    tmp,
                    RValue::Array {
                        elem_ty,
                        elements: ops,
                    },
                );
                tmp
            }

            // ── Tensor literal ────────────────────────────────────────────
            Expr::Tensor { elements, .. } => {
                let ops: Vec<Operand> = elements
                    .iter()
                    .map(|e| Operand::Local(self.lower_expr(e)))
                    .collect();
                let tmp = self.alloc_tmp(MirTy::Tensor);
                self.emit_assign(tmp, RValue::Tensor(ops));
                tmp
            }

            // ── Struct literal ────────────────────────────────────────────
            Expr::StructLiteral { name, fields, .. } => {
                let struct_name = match name.as_ref() {
                    Expr::Ident { ident, .. } => ident.name.clone(),
                    Expr::Path { segments, .. } => {
                        segments.last().map(|s| s.name.clone()).unwrap_or_default()
                    }
                    _ => "Unknown".to_string(),
                };
                let field_ops: Vec<(String, Operand)> = fields
                    .iter()
                    .map(|f| {
                        let v = self.lower_expr(&f.value);
                        (f.name.name.clone(), Operand::Local(v))
                    })
                    .collect();
                let tmp = self.alloc_tmp(ty.clone());
                self.emit_assign(
                    tmp,
                    RValue::Struct {
                        name: struct_name,
                        fields: field_ops,
                    },
                );
                tmp
            }

            // ── Cast (`as`) ───────────────────────────────────────────────
            Expr::Cast {
                expr, ty: cast_ty, ..
            } => {
                let val = self.lower_expr(expr);
                let target_mir_ty = resolve_ast_type(cast_ty);
                let tmp = self.alloc_tmp(target_mir_ty.clone());
                self.emit_assign(
                    tmp,
                    RValue::Cast {
                        operand: Operand::Local(val),
                        ty: target_mir_ty,
                    },
                );
                tmp
            }

            // ── Type check (`is`) ─────────────────────────────────────────
            Expr::TypeCheck { expr, .. } => {
                // Runtime type check: emit as always-true for now (RTTI is
                // deferred to codegen).
                self.lower_expr(expr); // evaluate for side effects
                let tmp = self.alloc_tmp(MirTy::Bool);
                self.emit_assign(tmp, RValue::Use(Operand::bool_const(true)));
                tmp
            }

            // ── Try (`?`) ─────────────────────────────────────────────────
            Expr::Try { expr, .. } => {
                let val = self.lower_expr(expr);
                // Simplified: extract the inner value (.0 field).
                // Full `?` desugaring (early return on Err) is handled by codegen.
                let inner = self.alloc_tmp(ty.clone());
                self.emit_assign(
                    inner,
                    RValue::Field {
                        base: Operand::Local(val),
                        field: "0".to_string(),
                    },
                );
                inner
            }

            // ── Await ─────────────────────────────────────────────────────
            Expr::Await { expr, .. } => {
                let fut = self.lower_expr(expr);
                let result = self.alloc_tmp(ty.clone());
                let cont = self.new_block();
                self.terminate(Terminator::Call {
                    callee: Operand::Const(Const::Str("__await".into())),
                    args: vec![Operand::Local(fut)],
                    dest: Some(result),
                    target: cont,
                });
                self.switch_to(cont);
                result
            }

            // ── Assign ───────────────────────────────────────────────────
            Expr::Assign { target, value, .. } => {
                let rhs = self.lower_expr(value);
                let lhs = self.lower_lvalue(target);
                self.emit_assign(lhs, RValue::Use(Operand::Local(rhs)));
                let tmp = self.alloc_tmp(MirTy::Void);
                self.emit_assign(tmp, RValue::Use(Operand::unit()));
                tmp
            }

            // ── Compound assign ───────────────────────────────────────────
            Expr::CompoundAssign {
                target, op, value, ..
            } => {
                let rhs = self.lower_expr(value);
                let lhs = self.lower_lvalue(target);
                let bin_op = compound_to_bin(*op);
                let old_val = self.alloc_tmp(MirTy::Opaque);
                self.emit_assign(old_val, RValue::Use(Operand::Local(lhs)));
                let new_val = self.alloc_tmp(MirTy::Opaque);
                self.emit_assign(
                    new_val,
                    RValue::BinOp {
                        op: bin_op,
                        left: Operand::Local(old_val),
                        right: Operand::Local(rhs),
                    },
                );
                self.emit_assign(lhs, RValue::Use(Operand::Local(new_val)));
                let tmp = self.alloc_tmp(MirTy::Void);
                self.emit_assign(tmp, RValue::Use(Operand::unit()));
                tmp
            }

            // ── Break ────────────────────────────────────────────────────
            Expr::Break { value, .. } => {
                let break_block = self
                    .loop_stack
                    .last()
                    .map(|c| c.break_block)
                    .unwrap_or(BlockId(0));
                let break_val_local = self.loop_stack.last().and_then(|c| c.break_value_local);

                if let (Some(v_expr), Some(bvl)) = (value.as_deref(), break_val_local) {
                    let v = self.lower_expr(v_expr);
                    self.emit_assign(bvl, RValue::Use(Operand::Local(v)));
                }
                self.terminate(Terminator::Goto(break_block));

                // Allocate a Never local so the function always returns a LocalId.
                let tmp = self.alloc_tmp(MirTy::Never);
                tmp
            }

            // ── Next (continue) ───────────────────────────────────────────
            Expr::Next { .. } => {
                let cont = self
                    .loop_stack
                    .last()
                    .map(|c| c.continue_block)
                    .unwrap_or(BlockId(0));
                self.terminate(Terminator::Goto(cont));
                self.alloc_tmp(MirTy::Never)
            }

            // ── Return ───────────────────────────────────────────────────
            Expr::Return { value, .. } => {
                if let Some(v) = value {
                    let val = self.lower_expr(v);
                    self.terminate(Terminator::Return(Some(Operand::Local(val))));
                } else {
                    self.terminate(Terminator::Return(None));
                }
                self.alloc_tmp(MirTy::Never)
            }

            // ── Fork ─────────────────────────────────────────────────────
            Expr::Fork { kind, .. } => match kind {
                ForkKind::Block { tasks } => {
                    let mut results = Vec::new();
                    for task in tasks.iter() {
                        let r = self.lower_expr(&task.expr);
                        if let Some(binding) = &task.binding {
                            // Named fork result — bind it
                            let named = self.alloc_named(&binding.name, MirTy::Opaque, false);
                            self.emit_assign(named, RValue::Use(Operand::Local(r)));
                            if let Some(def_id) = self.resolve_ident_span(binding.span) {
                                self.local_map.insert(def_id, named);
                            }
                            results.push(Operand::Local(named));
                        } else {
                            results.push(Operand::Local(r));
                        }
                    }
                    let tmp = self.alloc_tmp(ty.clone());
                    self.emit_assign(tmp, RValue::Tuple(results));
                    tmp
                }
                ForkKind::Loop {
                    binding,
                    iterable,
                    body,
                } => {
                    // Lower as a sequential for-loop collecting results into a vec.
                    let iter = self.lower_expr(iterable);
                    let results_local = self.alloc_tmp(MirTy::Vec(Box::new(MirTy::Opaque)));
                    self.emit_assign(
                        results_local,
                        RValue::Vec {
                            elem_ty: MirTy::Opaque,
                            elements: vec![],
                        },
                    );
                    let loop_header = self.new_block();
                    let loop_body_bb = self.new_block();
                    let loop_exit = self.new_block();

                    self.terminate(Terminator::Goto(loop_header));
                    self.switch_to(loop_header);

                    let next_opt = self.alloc_tmp(MirTy::Opaque);
                    let check_bb = self.new_block();
                    self.terminate(Terminator::Call {
                        callee: Operand::Const(Const::Str("__iter_next".into())),
                        args: vec![Operand::Local(iter)],
                        dest: Some(next_opt),
                        target: check_bb,
                    });
                    self.switch_to(check_bb);
                    let disc = self.alloc_tmp(MirTy::I64);
                    self.emit_assign(disc, RValue::Discriminant(Operand::Local(next_opt)));
                    self.terminate(Terminator::Branch {
                        cond: Operand::Local(disc),
                        then_block: loop_body_bb,
                        else_block: loop_exit,
                    });

                    self.switch_to(loop_body_bb);
                    self.bind_pattern(binding, next_opt);
                    let body_val = self.lower_expr(body);
                    // Push body_val into results_local
                    let push_cont = self.new_block();
                    self.terminate(Terminator::Call {
                        callee: Operand::Const(Const::Str("__vec_push".into())),
                        args: vec![Operand::Local(results_local), Operand::Local(body_val)],
                        dest: None,
                        target: push_cont,
                    });
                    self.switch_to(push_cont);
                    if !self.is_terminated() {
                        self.terminate(Terminator::Goto(loop_header));
                    }

                    self.switch_to(loop_exit);
                    results_local
                }
            },

            // ── Path (Enum.Variant / module.item) ─────────────────────────
            Expr::Path { segments, span } => {
                // Try to look up as a binding first.
                if let Some(def_id) = self.resolve_ident_span(*span) {
                    if let Some(&local_id) = self.local_map.get(&def_id) {
                        return local_id;
                    }
                }

                // Treat as enum unit variant access.
                let enum_name = segments.first().map(|s| s.name.clone()).unwrap_or_default();
                let variant_name = if segments.len() > 1 {
                    segments.last().map(|s| s.name.clone()).unwrap_or_default()
                } else {
                    enum_name.clone()
                };
                let tmp = self.alloc_tmp(ty.clone());
                self.emit_assign(
                    tmp,
                    RValue::EnumVariant {
                        enum_name,
                        variant: variant_name,
                        payload: vec![],
                    },
                );
                tmp
            }
        }
    }

    // ── L-value lowering ──────────────────────────────────────────────────

    /// Lower an assignment target expression, returning the `LocalId` of the
    /// slot to be written.
    fn lower_lvalue(&mut self, expr: &Expr) -> LocalId {
        match expr {
            Expr::Ident { span, .. } => {
                if let Some(def_id) = self.resolve_ident_span(*span) {
                    if let Some(&local_id) = self.local_map.get(&def_id) {
                        return local_id;
                    }
                }
                // Allocate a fresh slot so we don't crash.
                let ty = self.expr_ty(expr);
                let tmp = self.alloc_tmp(ty.clone());
                tmp
            }
            Expr::Field { object, field, .. } => {
                let obj = self.lower_lvalue(object);
                let tmp = self.alloc_tmp(MirTy::Opaque);
                self.emit_assign(
                    tmp,
                    RValue::Field {
                        base: Operand::Local(obj),
                        field: field.name.clone(),
                    },
                );
                tmp
            }
            Expr::Index { object, index, .. } => {
                let obj = self.lower_lvalue(object);
                let idx = self.lower_expr(index);
                let tmp = self.alloc_tmp(MirTy::Opaque);
                self.emit_assign(
                    tmp,
                    RValue::Index {
                        base: Operand::Local(obj),
                        index: Operand::Local(idx),
                    },
                );
                tmp
            }
            Expr::Paren { inner, .. } => self.lower_lvalue(inner),
            _ => self.lower_expr(expr),
        }
    }

    // ── Statement lowering ────────────────────────────────────────────────

    fn lower_stmt(&mut self, stmt: &AstStmt) {
        if self.is_terminated() {
            return;
        }
        match stmt {
            AstStmt::Let { pattern, value, .. } => {
                let val = self.lower_expr(value);
                self.bind_pattern(pattern, val);
            }

            AstStmt::LetMut { name, value, .. } => {
                let val = self.lower_expr(value);
                let val_ty = self.func.local_ty(val).clone();
                let local = self.alloc_named(&name.name, val_ty, true);
                self.emit_assign(local, RValue::Use(Operand::Local(val)));
                if let Some(def_id) = self.resolve_ident_span(name.span) {
                    self.local_map.insert(def_id, local);
                }
            }

            AstStmt::Const { name, value, .. } => {
                let val = self.lower_expr(value);
                // Consts are stored as immutable locals at the MIR level.
                if let Some(def_id) = self.resolve_ident_span(name.span) {
                    self.local_map.insert(def_id, val);
                }
            }

            AstStmt::Shared { name, value, .. } => {
                let val = self.lower_expr(value);
                let val_ty = self.func.local_ty(val).clone();
                let local = self.alloc_named(&name.name, MirTy::Shared(Box::new(val_ty)), true);
                self.emit_assign(local, RValue::Use(Operand::Local(val)));
                if let Some(def_id) = self.resolve_ident_span(name.span) {
                    self.local_map.insert(def_id, local);
                }
            }

            AstStmt::Expr { expr, .. } => {
                self.lower_expr(expr);
            }

            AstStmt::Return { value, .. } => {
                if let Some(v) = value {
                    let val = self.lower_expr(v);
                    self.terminate(Terminator::Return(Some(Operand::Local(val))));
                } else {
                    self.terminate(Terminator::Return(None));
                }
            }

            AstStmt::Break { value, .. } => {
                let break_block = self
                    .loop_stack
                    .last()
                    .map(|c| c.break_block)
                    .unwrap_or(BlockId(0));
                let break_val_local = self.loop_stack.last().and_then(|c| c.break_value_local);
                if let (Some(v), Some(bvl)) = (value.as_ref(), break_val_local) {
                    let val = self.lower_expr(v);
                    self.emit_assign(bvl, RValue::Use(Operand::Local(val)));
                }
                self.terminate(Terminator::Goto(break_block));
            }

            AstStmt::Next { .. } => {
                let cont = self
                    .loop_stack
                    .last()
                    .map(|c| c.continue_block)
                    .unwrap_or(BlockId(0));
                self.terminate(Terminator::Goto(cont));
            }

            AstStmt::Defer { body, .. } => {
                // Simplified: execute immediately (proper defer requires scope tracking).
                self.lower_expr(body);
            }

            AstStmt::Guard {
                condition,
                else_body,
                ..
            } => {
                let cond = self.lower_expr(condition);
                let else_bb = self.new_block();
                let cont_bb = self.new_block();
                self.terminate(Terminator::Branch {
                    cond: Operand::Local(cond),
                    then_block: cont_bb,
                    else_block: else_bb,
                });
                self.switch_to(else_bb);
                self.lower_expr(else_body);
                if !self.is_terminated() {
                    self.terminate(Terminator::Goto(cont_bb));
                }
                self.switch_to(cont_bb);
            }

            AstStmt::Assign { target, value, .. } => {
                let rhs = self.lower_expr(value);
                let lhs = self.lower_lvalue(target);
                self.emit_assign(lhs, RValue::Use(Operand::Local(rhs)));
            }

            AstStmt::CompoundAssign {
                target, op, value, ..
            } => {
                let rhs = self.lower_expr(value);
                let lhs = self.lower_lvalue(target);
                let old = self.alloc_tmp(MirTy::Opaque);
                self.emit_assign(old, RValue::Use(Operand::Local(lhs)));
                let new_val = self.alloc_tmp(MirTy::Opaque);
                self.emit_assign(
                    new_val,
                    RValue::BinOp {
                        op: compound_to_bin(*op),
                        left: Operand::Local(old),
                        right: Operand::Local(rhs),
                    },
                );
                self.emit_assign(lhs, RValue::Use(Operand::Local(new_val)));
            }

            AstStmt::Item { item, .. } => {
                // Nested items (local functions) are skipped at this stage.
                // Proper handling requires closure conversion.
                let _ = item;
            }
        }
    }

    // ── Pattern binding ───────────────────────────────────────────────────

    /// Emit instructions to destructure `source` according to `pat` and
    /// register each bound name in `local_map`.
    fn bind_pattern(&mut self, pat: &Pattern, source: LocalId) {
        match pat {
            Pattern::Binding { name, span } => {
                let ty = self.func.local_ty(source).clone();
                let local = self.alloc_named(&name.name, ty, false);
                self.emit_assign(local, RValue::Use(Operand::Local(source)));
                if let Some(def_id) = self.resolve_ident_span(*span) {
                    self.local_map.insert(def_id, local);
                }
            }

            Pattern::Wildcard { .. }
            | Pattern::Literal { .. }
            | Pattern::None { .. }
            | Pattern::EnumUnit { .. }
            | Pattern::Range { .. } => {}

            Pattern::Tuple { elements, .. } => {
                for (i, elem_pat) in elements.iter().enumerate() {
                    let field_local = self.alloc_tmp(MirTy::Opaque);
                    self.emit_assign(
                        field_local,
                        RValue::FieldIdx {
                            base: Operand::Local(source),
                            index: i,
                        },
                    );
                    self.bind_pattern(elem_pat, field_local);
                }
            }

            Pattern::Struct { fields, .. } => {
                for field in fields.iter() {
                    let field_local = self.alloc_tmp(MirTy::Opaque);
                    self.emit_assign(
                        field_local,
                        RValue::Field {
                            base: Operand::Local(source),
                            field: field.name.name.clone(),
                        },
                    );
                    let bind_ident = field.rename.as_ref().unwrap_or(&field.name);
                    let named = self.alloc_named(&bind_ident.name, MirTy::Opaque, false);
                    self.emit_assign(named, RValue::Use(Operand::Local(field_local)));
                    if let Some(def_id) = self.resolve_ident_span(bind_ident.span) {
                        self.local_map.insert(def_id, named);
                    }
                }
            }

            Pattern::Some { inner, .. }
            | Pattern::Ok { inner, .. }
            | Pattern::Err { inner, .. } => {
                let inner_local = self.alloc_tmp(MirTy::Opaque);
                self.emit_assign(
                    inner_local,
                    RValue::Field {
                        base: Operand::Local(source),
                        field: "0".to_string(),
                    },
                );
                self.bind_pattern(inner, inner_local);
            }

            Pattern::EnumPositional { args, .. } => {
                for (i, arg_pat) in args.iter().enumerate() {
                    let field_local = self.alloc_tmp(MirTy::Opaque);
                    self.emit_assign(
                        field_local,
                        RValue::FieldIdx {
                            base: Operand::Local(source),
                            index: i,
                        },
                    );
                    self.bind_pattern(arg_pat, field_local);
                }
            }

            Pattern::EnumNamed { fields, .. } => {
                for field in fields.iter() {
                    let field_local = self.alloc_tmp(MirTy::Opaque);
                    self.emit_assign(
                        field_local,
                        RValue::Field {
                            base: Operand::Local(source),
                            field: field.name.name.clone(),
                        },
                    );
                    let bind_ident = field.rename.as_ref().unwrap_or(&field.name);
                    let named = self.alloc_named(&bind_ident.name, MirTy::Opaque, false);
                    self.emit_assign(named, RValue::Use(Operand::Local(field_local)));
                    if let Some(def_id) = self.resolve_ident_span(bind_ident.span) {
                        self.local_map.insert(def_id, named);
                    }
                }
            }

            Pattern::TupleStruct { fields, .. } => {
                for (i, field_pat) in fields.iter().enumerate() {
                    let field_local = self.alloc_tmp(MirTy::Opaque);
                    self.emit_assign(
                        field_local,
                        RValue::FieldIdx {
                            base: Operand::Local(source),
                            index: i,
                        },
                    );
                    self.bind_pattern(field_pat, field_local);
                }
            }

            Pattern::Or { patterns, .. } => {
                // Lower the first alternative (simplified).
                if let Some(first) = patterns.first() {
                    self.bind_pattern(first, source);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Top-level Lowerer
// ---------------------------------------------------------------------------

pub struct Lowerer<'a> {
    sema: &'a SemanticModel,
    program: MirProgram,
}

impl<'a> Lowerer<'a> {
    pub fn new(sema: &'a SemanticModel) -> Self {
        Self {
            sema,
            program: MirProgram::new(),
        }
    }

    /// Lower a complete Razen module into a `MirProgram`.
    pub fn lower_module(mut self, module: &Module) -> MirProgram {
        // Pass 1: type definitions (structs, enums).
        for item in &module.items {
            self.lower_type_def(item);
        }
        // Pass 2: function definitions and impl blocks.
        for item in &module.items {
            self.lower_item(item);
        }
        self.program
    }

    // ── Type definitions ──────────────────────────────────────────────────

    fn lower_type_def(&mut self, item: &Item) {
        match item {
            Item::Struct(sdef) => {
                let mir_struct = lower_struct_def(sdef);
                self.program.add_struct(mir_struct);
            }
            Item::Enum(edef) => {
                let mir_enum = lower_enum_def(edef);
                self.program.add_enum(mir_enum);
            }
            _ => {}
        }
    }

    // ── Item lowering ─────────────────────────────────────────────────────

    fn lower_item(&mut self, item: &Item) {
        match item {
            Item::Function(fndef) => {
                if let Some(mir_fn) = self.lower_fn(fndef, None) {
                    self.program.add_fn(mir_fn);
                }
            }
            Item::Impl(iblock) => {
                self.lower_impl(iblock);
            }
            Item::Const(_)
            | Item::Shared(_)
            | Item::Struct(_)
            | Item::Enum(_)
            | Item::TypeAlias(_)
            | Item::Use(_)
            | Item::Trait(_) => {}
        }
    }

    fn lower_impl(&mut self, iblock: &ImplBlock) {
        let self_name = match &iblock.target {
            TypeExpr::Named { name, .. } => Some(name.name.clone()),
            TypeExpr::Generic { name, .. } => Some(name.name.clone()),
            _ => None,
        };

        for method in &iblock.methods {
            // Mangle: TypeName_methodName
            let mangled = if let Some(ref sn) = self_name {
                format!("{}_{}", sn, method.name.name)
            } else {
                method.name.name.clone()
            };

            // Clone and rename for lowering
            let mut renamed = method.clone();
            renamed.name = Ident::new(mangled, method.name.span);

            if let Some(mir_fn) = self.lower_fn(&renamed, self_name.clone()) {
                self.program.add_fn(mir_fn);
            }
        }
    }

    fn lower_fn(&self, fndef: &FnDef, _self_type_name: Option<String>) -> Option<MirFn> {
        // Skip bodyless functions (trait signatures, externs).
        if matches!(fndef.body, FnBody::None) {
            return None;
        }

        let ret_ty = fndef
            .return_type
            .as_ref()
            .map(|t| resolve_ast_type(t))
            .unwrap_or(MirTy::Void);

        let is_pub = matches!(fndef.vis, Visibility::Public);
        let mut builder = FnBuilder::new(
            self.sema,
            fndef.name.name.clone(),
            ret_ty,
            fndef.is_async,
            is_pub,
        );

        // Register parameters.
        for param in &fndef.params {
            let param_ty = param
                .ty
                .as_ref()
                .map(|t| resolve_ast_type(t))
                .unwrap_or(MirTy::Opaque);

            let (param_name, param_def_id) = extract_pattern_binding(&param.pattern, self.sema);
            let local_id = builder.func.new_param(param_ty, param_name);
            if let Some(def_id) = param_def_id {
                builder.local_map.insert(def_id, local_id);
            }
        }

        // Lower the body.
        match &fndef.body {
            FnBody::Block { stmts, tail, .. } => {
                for stmt in stmts.iter() {
                    builder.lower_stmt(stmt);
                    if builder.is_terminated() {
                        break;
                    }
                }
                if let Some(tail_expr) = tail {
                    if !builder.is_terminated() {
                        let result = builder.lower_expr(tail_expr);
                        if !builder.is_terminated() {
                            builder.terminate(Terminator::Return(Some(Operand::Local(result))));
                        }
                    }
                } else if !builder.is_terminated() {
                    builder.terminate(Terminator::Return(None));
                }
            }
            FnBody::Expr(expr) => {
                let result = builder.lower_expr(expr);
                if !builder.is_terminated() {
                    builder.terminate(Terminator::Return(Some(Operand::Local(result))));
                }
            }
            FnBody::None => unreachable!("handled above"),
        }

        Some(builder.func)
    }
}

// ---------------------------------------------------------------------------
// Free helper functions
// ---------------------------------------------------------------------------

/// Lower a struct definition to MIR.
fn lower_struct_def(sdef: &razen_ast::item::StructDef) -> MirStruct {
    let fields: Vec<(String, MirTy)> = match &sdef.kind {
        StructKind::Named { fields } => fields
            .iter()
            .map(|f| (f.name.name.clone(), resolve_ast_type(&f.ty)))
            .collect(),
        StructKind::Tuple { fields } => fields
            .iter()
            .enumerate()
            .map(|(i, ty)| (i.to_string(), resolve_ast_type(ty)))
            .collect(),
        StructKind::Unit => vec![],
    };
    MirStruct {
        name: sdef.name.name.clone(),
        fields,
        is_pub: matches!(sdef.vis, Visibility::Public),
    }
}

/// Lower an enum definition to MIR.
fn lower_enum_def(edef: &razen_ast::item::EnumDef) -> MirEnum {
    let variants: Vec<MirVariant> = edef
        .variants
        .iter()
        .enumerate()
        .map(|(disc, v)| {
            let kind = match &v.kind {
                EnumVariantKind::Unit => MirVariantKind::Unit,
                EnumVariantKind::Positional { fields } => {
                    MirVariantKind::Positional(fields.iter().map(|t| resolve_ast_type(t)).collect())
                }
                EnumVariantKind::Named { fields } => MirVariantKind::Named(
                    fields
                        .iter()
                        .map(|f| (f.name.name.clone(), resolve_ast_type(&f.ty)))
                        .collect(),
                ),
            };
            MirVariant {
                name: v.name.name.clone(),
                kind,
                discriminant: disc as i64,
            }
        })
        .collect();

    MirEnum {
        name: edef.name.name.clone(),
        variants,
        is_pub: matches!(edef.vis, Visibility::Public),
    }
}

/// Resolve a `TypeExpr` (from the AST) to a `MirTy` without consulting the
/// semantic model — this is used for struct/enum field types and function
/// parameter types where we just do a structural mapping.
pub fn resolve_ast_type(ty: &TypeExpr) -> MirTy {
    match ty {
        TypeExpr::Named { name, .. } => match name.name.as_str() {
            "bool" => MirTy::Bool,
            "int" => MirTy::Int,
            "uint" => MirTy::Uint,
            "float" => MirTy::Float,
            "i8" => MirTy::I8,
            "i16" => MirTy::I16,
            "i32" => MirTy::I32,
            "i64" => MirTy::I64,
            "i128" => MirTy::I128,
            "isize" => MirTy::Isize,
            "u8" => MirTy::U8,
            "u16" => MirTy::U16,
            "u32" => MirTy::U32,
            "u64" => MirTy::U64,
            "u128" => MirTy::U128,
            "usize" => MirTy::Usize,
            "f32" => MirTy::F32,
            "f64" => MirTy::F64,
            "char" => MirTy::Char,
            "str" => MirTy::Str,
            "bytes" => MirTy::Bytes,
            "void" => MirTy::Void,
            "never" => MirTy::Never,
            "tensor" => MirTy::Tensor,
            other => MirTy::Struct(other.to_string()),
        },
        TypeExpr::Generic { name, args, .. } => {
            let arg_tys: Vec<MirTy> = args.iter().map(|a| resolve_ast_type(a)).collect();
            match name.name.as_str() {
                "vec" => MirTy::Vec(Box::new(
                    arg_tys.into_iter().next().unwrap_or(MirTy::Opaque),
                )),
                "map" => {
                    let mut it = arg_tys.into_iter();
                    let k = it.next().unwrap_or(MirTy::Opaque);
                    let v = it.next().unwrap_or(MirTy::Opaque);
                    MirTy::Map(Box::new(k), Box::new(v))
                }
                "set" => MirTy::Set(Box::new(
                    arg_tys.into_iter().next().unwrap_or(MirTy::Opaque),
                )),
                "option" => MirTy::Option(Box::new(
                    arg_tys.into_iter().next().unwrap_or(MirTy::Opaque),
                )),
                "result" => {
                    let mut it = arg_tys.into_iter();
                    let t = it.next().unwrap_or(MirTy::Opaque);
                    let e = it.next().unwrap_or(MirTy::Str);
                    MirTy::Result(Box::new(t), Box::new(e))
                }
                other => MirTy::Struct(other.to_string()),
            }
        }
        TypeExpr::Tuple { elements, .. } => {
            MirTy::Tuple(elements.iter().map(|e| resolve_ast_type(e)).collect())
        }
        TypeExpr::Array { element, .. } => MirTy::Array {
            element: Box::new(resolve_ast_type(element)),
            size: 0,
        },
        TypeExpr::Closure { params, ret, .. } => MirTy::Fn {
            params: params.iter().map(|p| resolve_ast_type(p)).collect(),
            ret: Box::new(resolve_ast_type(ret)),
        },
        TypeExpr::Void { .. } => MirTy::Void,
        TypeExpr::Never { .. } => MirTy::Never,
        TypeExpr::SelfType { .. } | TypeExpr::Inferred { .. } => MirTy::Opaque,
        TypeExpr::Ref { inner, .. } => resolve_ast_type(inner),
    }
}

/// Convert a literal value to a `Const`.
fn lower_literal(lit: &Literal) -> Const {
    match lit {
        Literal::Int { raw, .. } => {
            let clean = raw.trim_end_matches(|c: char| c.is_alphabetic() || c == '_');
            let parsed = if clean.starts_with("0x") || clean.starts_with("0X") {
                i64::from_str_radix(&clean[2..].replace('_', ""), 16).unwrap_or(0)
            } else if clean.starts_with("0b") || clean.starts_with("0B") {
                i64::from_str_radix(&clean[2..].replace('_', ""), 2).unwrap_or(0)
            } else if clean.starts_with("0o") || clean.starts_with("0O") {
                i64::from_str_radix(&clean[2..].replace('_', ""), 8).unwrap_or(0)
            } else {
                clean.replace('_', "").parse::<i64>().unwrap_or(0)
            };
            Const::Int(parsed)
        }
        Literal::Float { raw, .. } => {
            let clean = raw.trim_end_matches(|c: char| c.is_alphabetic());
            Const::Float(clean.replace('_', "").parse::<f64>().unwrap_or(0.0))
        }
        Literal::Str { value, .. } => Const::Str(value.clone()),
        Literal::Char { value, .. } => Const::Char(*value),
        Literal::Bool { value, .. } => Const::Bool(*value),
    }
}

/// Convert a `CompoundOp` to the corresponding `BinOp`.
fn compound_to_bin(op: CompoundOp) -> BinOp {
    match op {
        CompoundOp::AddAssign => BinOp::Add,
        CompoundOp::SubAssign => BinOp::Sub,
        CompoundOp::MulAssign => BinOp::Mul,
        CompoundOp::DivAssign => BinOp::Div,
        CompoundOp::ModAssign => BinOp::Mod,
        CompoundOp::PowAssign => BinOp::Pow,
    }
}

/// Try to extract an integer discriminant from a match pattern for use in
/// a `Terminator::Switch`.  Returns `None` for patterns that need a full
/// comparison (struct patterns, etc.).
fn pattern_discriminant(pat: &Pattern) -> Option<i64> {
    match pat {
        Pattern::Literal { lit, .. } => match lit {
            Literal::Int { raw, .. } => {
                let clean = raw.trim_end_matches(|c: char| c.is_alphabetic());
                clean.replace('_', "").parse::<i64>().ok()
            }
            Literal::Bool { value, .. } => Some(if *value { 1 } else { 0 }),
            _ => None,
        },
        Pattern::None { .. } => Some(0),
        Pattern::Some { .. } | Pattern::Ok { .. } => Some(1),
        Pattern::Err { .. } => Some(0),
        _ => None,
    }
}

/// Extract the binding name and `DefId` from a `Pattern::Binding`.
fn extract_pattern_binding(pat: &Pattern, sema: &SemanticModel) -> (Option<String>, Option<DefId>) {
    match pat {
        Pattern::Binding { name, span } => {
            let def_id = sema.resolutions.get(span).copied();
            (Some(name.name.clone()), def_id)
        }
        _ => (None, None),
    }
}
