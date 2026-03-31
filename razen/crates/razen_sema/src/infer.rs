//! Type Inference Engine for Razen.
//!
//! This module implements a constraint-based type inference system using a
//! simplified union-find approach. During type-checking, fresh inference
//! variables (`Ty::Infer`) are created as placeholders for unknown types.
//! The `InferCtx` unifies these variables with concrete types as constraints
//! are discovered, and `apply()` substitutes all resolved variables into a
//! type once inference is complete.

use crate::error::SemanticError;
use crate::ty::{InferVarId, Ty};
use razen_lexer::Span;
use std::collections::HashMap;

/// Type inference context.
///
/// Maintains a substitution map from inference variable IDs to their resolved
/// types.  Unification walks the type structure recursively, binding variables
/// when a concrete type is found.
pub struct InferCtx {
    /// Counter for generating fresh inference variable IDs.
    next_id: u32,
    /// Substitution map: inference variable id → resolved type.
    /// Entries may point to other `Ty::Infer` variables (chains are followed
    /// by `apply`).
    subst: HashMap<u32, Ty>,
}

impl InferCtx {
    /// Create a new, empty inference context.
    pub fn new() -> Self {
        Self {
            next_id: 0,
            subst: HashMap::new(),
        }
    }

    // -----------------------------------------------------------------------
    // Variable creation
    // -----------------------------------------------------------------------

    /// Allocate a fresh inference variable and return it as `Ty::Infer(id)`.
    pub fn new_var(&mut self) -> Ty {
        let id = InferVarId(self.next_id);
        self.next_id += 1;
        Ty::Infer(id)
    }

    // -----------------------------------------------------------------------
    // Substitution
    // -----------------------------------------------------------------------

    /// Follow the substitution chain for a variable, returning the deepest
    /// known type (which may still be an unresolved `Ty::Infer`).
    fn walk(&self, id: u32) -> Ty {
        match self.subst.get(&id) {
            Some(Ty::Infer(next)) => self.walk(next.0),
            Some(other) => other.clone(),
            None => Ty::Infer(InferVarId(id)),
        }
    }

    /// Bind an inference variable to a type.
    /// Performs a shallow occurs-check to prevent trivial cycles:
    /// if `ty` is the same variable as `id`, the bind is a no-op.
    fn bind(&mut self, id: u32, ty: Ty) {
        if let Ty::Infer(other_id) = &ty {
            if other_id.0 == id {
                return; // trivial cycle — skip
            }
        }
        self.subst.insert(id, ty);
    }

    /// Deeply apply all current substitutions to `ty`, replacing every
    /// resolved `Ty::Infer` variable with its concrete type.
    pub fn apply(&self, ty: Ty) -> Ty {
        match ty {
            Ty::Infer(id) => {
                let walked = self.walk(id.0);
                match walked {
                    Ty::Infer(_) => walked, // still unresolved
                    other => self.apply(other),
                }
            }

            Ty::Vec(t) => Ty::Vec(Box::new(self.apply(*t))),
            Ty::Map(k, v) => Ty::Map(Box::new(self.apply(*k)), Box::new(self.apply(*v))),
            Ty::Set(t) => Ty::Set(Box::new(self.apply(*t))),

            Ty::Array { element, size } => Ty::Array {
                element: Box::new(self.apply(*element)),
                size,
            },

            Ty::Tuple(elements) => Ty::Tuple(elements.into_iter().map(|t| self.apply(t)).collect()),

            Ty::Option(t) => Ty::Option(Box::new(self.apply(*t))),
            Ty::Result(t, e) => Ty::Result(Box::new(self.apply(*t)), Box::new(self.apply(*e))),

            Ty::Named {
                def_id,
                name,
                generics,
            } => Ty::Named {
                def_id,
                name,
                generics: generics.into_iter().map(|g| self.apply(g)).collect(),
            },

            Ty::Fn {
                params,
                ret,
                is_async,
            } => Ty::Fn {
                params: params.into_iter().map(|p| self.apply(p)).collect(),
                ret: Box::new(self.apply(*ret)),
                is_async,
            },

            Ty::Shared(t) => Ty::Shared(Box::new(self.apply(*t))),

            // Leaf types — nothing to substitute
            other => other,
        }
    }

    // -----------------------------------------------------------------------
    // Unification
    // -----------------------------------------------------------------------

    /// Unify two types, binding inference variables as needed.
    ///
    /// Returns the "unified" type (the more concrete of the two).  On a
    /// structural mismatch a `SemanticError::Custom` is pushed and
    /// `Ty::Error` is returned so the caller can continue.
    pub fn unify(&mut self, a: Ty, b: Ty, span: Span, errors: &mut Vec<SemanticError>) -> Ty {
        // Apply current substitutions first so we work with the most concrete
        // types available.
        let a = self.apply(a);
        let b = self.apply(b);

        match (a, b) {
            // ── Error sentinel propagates silently ──────────────────────────
            (Ty::Error, _) | (_, Ty::Error) => Ty::Error,

            // ── Identical types ─────────────────────────────────────────────
            (ref a, ref b) if a == b => a.clone(),

            // ── Bind inference variable ──────────────────────────────────────
            (Ty::Infer(id), other) | (other, Ty::Infer(id)) => {
                self.bind(id.0, other.clone());
                other
            }

            // ── Never is the bottom type — unifies with anything ─────────────
            (Ty::Never, other) | (other, Ty::Never) => other,

            // ── Void unifies only with Void (already covered above) ──────────

            // ── Numeric widening rules ───────────────────────────────────────
            // `int` (the default) widens to any concrete integer type.
            (Ty::Int, t) | (t, Ty::Int) if t.is_integral() => t,
            // `uint` widens to any unsigned integer.
            (Ty::Uint, t) | (t, Ty::Uint)
                if matches!(
                    t,
                    Ty::U8 | Ty::U16 | Ty::U32 | Ty::U64 | Ty::U128 | Ty::Usize
                ) =>
            {
                t
            }
            // `float` widens to any float type.
            (Ty::Float, t) | (t, Ty::Float) if t.is_float_ty() => t,

            // ── Shared is transparent for unification ────────────────────────
            (Ty::Shared(a), Ty::Shared(b)) => {
                let inner = self.unify(*a, *b, span, errors);
                Ty::Shared(Box::new(inner))
            }
            (Ty::Shared(inner), other) | (other, Ty::Shared(inner)) => {
                self.unify(*inner, other, span, errors)
            }

            // ── Structural recursion ─────────────────────────────────────────
            (Ty::Vec(a), Ty::Vec(b)) => {
                let inner = self.unify(*a, *b, span, errors);
                Ty::Vec(Box::new(inner))
            }

            (Ty::Map(k1, v1), Ty::Map(k2, v2)) => {
                let k = self.unify(*k1, *k2, span, errors);
                let v = self.unify(*v1, *v2, span, errors);
                Ty::Map(Box::new(k), Box::new(v))
            }

            (Ty::Set(a), Ty::Set(b)) => {
                let inner = self.unify(*a, *b, span, errors);
                Ty::Set(Box::new(inner))
            }

            (Ty::Option(a), Ty::Option(b)) => {
                let inner = self.unify(*a, *b, span, errors);
                Ty::Option(Box::new(inner))
            }

            (Ty::Result(t1, e1), Ty::Result(t2, e2)) => {
                let t = self.unify(*t1, *t2, span, errors);
                let e = self.unify(*e1, *e2, span, errors);
                Ty::Result(Box::new(t), Box::new(e))
            }

            (Ty::Tuple(as_), Ty::Tuple(bs)) if as_.len() == bs.len() => {
                let elements = as_
                    .into_iter()
                    .zip(bs)
                    .map(|(a, b)| self.unify(a, b, span, errors))
                    .collect();
                Ty::Tuple(elements)
            }

            (
                Ty::Array {
                    element: ea,
                    size: sa,
                },
                Ty::Array {
                    element: eb,
                    size: sb,
                },
            ) => {
                let element = self.unify(*ea, *eb, span, errors);
                // Sizes must match; if one is 0 (unknown), adopt the other.
                let size = match (sa, sb) {
                    (0, s) | (s, 0) => s,
                    (s1, s2) if s1 == s2 => s1,
                    (s1, s2) => {
                        errors.push(SemanticError::Custom {
                            message: format!("array size mismatch: {} vs {}", s1, s2),
                            span,
                        });
                        s1
                    }
                };
                Ty::Array {
                    element: Box::new(element),
                    size,
                }
            }

            (
                Ty::Named {
                    def_id: id1,
                    name: n1,
                    generics: g1,
                },
                Ty::Named {
                    def_id: id2,
                    name: n2,
                    generics: g2,
                },
            ) if id1 == id2 => {
                let generics = g1
                    .into_iter()
                    .zip(g2)
                    .map(|(a, b)| self.unify(a, b, span, errors))
                    .collect();
                Ty::Named {
                    def_id: id1,
                    name: n1,
                    generics,
                }
            }

            (
                Ty::Fn {
                    params: p1,
                    ret: r1,
                    is_async: a1,
                },
                Ty::Fn {
                    params: p2,
                    ret: r2,
                    is_async: a2,
                },
            ) if p1.len() == p2.len() && a1 == a2 => {
                let params = p1
                    .into_iter()
                    .zip(p2)
                    .map(|(a, b)| self.unify(a, b, span, errors))
                    .collect();
                let ret = Box::new(self.unify(*r1, *r2, span, errors));
                Ty::Fn {
                    params,
                    ret,
                    is_async: a1,
                }
            }

            // ── Param widens to anything (polymorphic built-in) ──────────────
            (Ty::Param(_), other) | (other, Ty::Param(_)) => other,

            // ── Mismatch ─────────────────────────────────────────────────────
            (a, b) => {
                errors.push(SemanticError::Custom {
                    message: format!("type mismatch: expected `{}`, found `{}`", b, a),
                    span,
                });
                Ty::Error
            }
        }
    }
}

impl Default for InferCtx {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use razen_lexer::Span;

    fn span() -> Span {
        Span::default()
    }

    #[test]
    fn test_new_var_unique() {
        let mut ctx = InferCtx::new();
        let a = ctx.new_var();
        let b = ctx.new_var();
        assert_ne!(a, b);
    }

    #[test]
    fn test_unify_identical() {
        let mut ctx = InferCtx::new();
        let mut errs = Vec::new();
        let result = ctx.unify(Ty::Int, Ty::Int, span(), &mut errs);
        assert_eq!(result, Ty::Int);
        assert!(errs.is_empty());
    }

    #[test]
    fn test_unify_infer_with_concrete() {
        let mut ctx = InferCtx::new();
        let mut errs = Vec::new();
        let var = ctx.new_var();
        let result = ctx.unify(var.clone(), Ty::Str, span(), &mut errs);
        assert_eq!(result, Ty::Str);
        assert!(errs.is_empty());
        // applying the var should now return Str
        assert_eq!(ctx.apply(var), Ty::Str);
    }

    #[test]
    fn test_unify_mismatch_produces_error() {
        let mut ctx = InferCtx::new();
        let mut errs = Vec::new();
        let result = ctx.unify(Ty::Int, Ty::Str, span(), &mut errs);
        assert_eq!(result, Ty::Error);
        assert_eq!(errs.len(), 1);
    }

    #[test]
    fn test_unify_never_is_bottom() {
        let mut ctx = InferCtx::new();
        let mut errs = Vec::new();
        let result = ctx.unify(Ty::Never, Ty::Int, span(), &mut errs);
        assert_eq!(result, Ty::Int);
        assert!(errs.is_empty());
    }

    #[test]
    fn test_unify_vec() {
        let mut ctx = InferCtx::new();
        let mut errs = Vec::new();
        let var = ctx.new_var();
        let a = Ty::Vec(Box::new(var));
        let b = Ty::Vec(Box::new(Ty::Int));
        let result = ctx.unify(a, b, span(), &mut errs);
        assert_eq!(result, Ty::Vec(Box::new(Ty::Int)));
        assert!(errs.is_empty());
    }

    #[test]
    fn test_unify_option() {
        let mut ctx = InferCtx::new();
        let mut errs = Vec::new();
        let result = ctx.unify(
            Ty::Option(Box::new(Ty::Int)),
            Ty::Option(Box::new(Ty::Int)),
            span(),
            &mut errs,
        );
        assert_eq!(result, Ty::Option(Box::new(Ty::Int)));
        assert!(errs.is_empty());
    }

    #[test]
    fn test_apply_chain() {
        let mut ctx = InferCtx::new();
        let mut errs = Vec::new();
        let v1 = ctx.new_var();
        let v2 = ctx.new_var();
        // Unify v1 with v2, then v2 with Int
        ctx.unify(v1.clone(), v2.clone(), span(), &mut errs);
        ctx.unify(v2.clone(), Ty::Int, span(), &mut errs);
        assert_eq!(ctx.apply(v1), Ty::Int);
        assert!(errs.is_empty());
    }

    #[test]
    fn test_int_widens_to_i32() {
        let mut ctx = InferCtx::new();
        let mut errs = Vec::new();
        let result = ctx.unify(Ty::Int, Ty::I32, span(), &mut errs);
        assert_eq!(result, Ty::I32);
        assert!(errs.is_empty());
    }

    #[test]
    fn test_float_widens_to_f64() {
        let mut ctx = InferCtx::new();
        let mut errs = Vec::new();
        let result = ctx.unify(Ty::Float, Ty::F64, span(), &mut errs);
        assert_eq!(result, Ty::F64);
        assert!(errs.is_empty());
    }

    #[test]
    fn test_error_propagates_silently() {
        let mut ctx = InferCtx::new();
        let mut errs = Vec::new();
        let result = ctx.unify(Ty::Error, Ty::Int, span(), &mut errs);
        assert_eq!(result, Ty::Error);
        assert!(
            errs.is_empty(),
            "Error should not produce additional errors"
        );
    }
}
