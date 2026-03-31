//! MIR Instructions, RValues, and Terminators.
//!
//! Every basic block in a MIR function consists of:
//!   • A (possibly empty) sequence of `Inst` values — the straight-line work.
//!   • Exactly one `Terminator` — the control-flow transfer at the end.
//!
//! `RValue` represents the right-hand side of an assignment: a computation
//! that produces a value without changing control flow.

use razen_ast::ops::{BinOp, UnaryOp};

use crate::ty::MirTy;
use crate::value::{BlockId, Const, LocalId, Operand};

// ---------------------------------------------------------------------------
// RValue — the right-hand side of an assignment
// ---------------------------------------------------------------------------

/// A computation that produces a value and can be assigned to a local.
///
/// RValues are always side-effect-free with respect to control flow.
/// Operations that may change control flow (e.g. function calls that can
/// panic) use `Terminator::Call` instead.
#[derive(Debug, Clone, PartialEq)]
pub enum RValue {
    // ── Identity / use ───────────────────────────────────────────────────────
    /// Copy / use an operand verbatim: `_dest = _src` or `_dest = const`.
    Use(Operand),

    // ── Arithmetic & logic ───────────────────────────────────────────────────
    /// Binary operation: `_dest = _lhs op _rhs`.
    BinOp {
        op: BinOp,
        left: Operand,
        right: Operand,
    },

    /// Unary (prefix) operation: `_dest = op _operand`.
    UnaryOp { op: UnaryOp, operand: Operand },

    // ── Aggregate constructors ───────────────────────────────────────────────
    /// Construct a named struct: `User { id: _1, name: _2 }`.
    Struct {
        name: String,
        fields: Vec<(String, Operand)>,
    },

    /// Construct a tuple: `(_0, _1, _2)`.
    Tuple(Vec<Operand>),

    /// Construct a vec literal: `vec[_0, _1, _2]`.
    Vec {
        elem_ty: MirTy,
        elements: Vec<Operand>,
    },

    /// Construct a map literal: `map[k0: v0, k1: v1]`.
    Map {
        key_ty: MirTy,
        val_ty: MirTy,
        entries: Vec<(Operand, Operand)>,
    },

    /// Construct a set literal: `set[_0, _1]`.
    Set {
        elem_ty: MirTy,
        elements: Vec<Operand>,
    },

    /// Construct a fixed array: `[_0, _1, _2, _3, _4]`.
    Array {
        elem_ty: MirTy,
        elements: Vec<Operand>,
    },

    // ── Option / Result ──────────────────────────────────────────────────────
    /// Wrap a value in `some(...)`.
    SomeWrap(Operand),

    /// The `none` value (an empty option).
    None { ty: MirTy },

    /// Wrap a value in `ok(...)`.
    OkWrap(Operand),

    /// Wrap a value in `err(...)`.
    ErrWrap(Operand),

    // ── Enum ─────────────────────────────────────────────────────────────────
    /// Construct an enum variant: `Direction.North` or `Shape.Circle(_r)`.
    EnumVariant {
        enum_name: String,
        variant: String,
        payload: Vec<Operand>,
    },

    // ── Field / index access ─────────────────────────────────────────────────
    /// Read a named struct field: `_obj.field_name`.
    Field { base: Operand, field: String },

    /// Read a positional field / tuple element: `_obj.0`.
    FieldIdx { base: Operand, index: usize },

    /// Get the integer discriminant of an enum or option/result value.
    /// Used to drive `Terminator::Switch`.
    Discriminant(Operand),

    /// Index into a collection: `_obj[_idx]`.
    Index { base: Operand, index: Operand },

    // ── Type operations ──────────────────────────────────────────────────────
    /// Type cast: `_val as int`.
    Cast { operand: Operand, ty: MirTy },

    // ── Closures ─────────────────────────────────────────────────────────────
    /// Create a closure object (function pointer + captured environment).
    Closure {
        /// Mangled name of the generated closure function.
        func_name: String,
        /// Captured variables: (name, value).
        captures: Vec<(String, Operand)>,
    },

    // ── AI / ML ──────────────────────────────────────────────────────────────
    /// Tensor literal: `tensor[_0, _1, _2]`.
    Tensor(Vec<Operand>),

    // ── Inline call (no continuation) ────────────────────────────────────────
    /// A function call whose result is needed as an rvalue.
    /// Use this for simple built-in calls that cannot panic and do not need
    /// their own continuation block.  For all other calls, use
    /// `Terminator::Call`.
    Call { callee: Operand, args: Vec<Operand> },
}

// ---------------------------------------------------------------------------
// Instruction — an assignment statement
// ---------------------------------------------------------------------------

/// A single non-terminating MIR instruction.
///
/// Every instruction assigns the result of an `RValue` computation to a
/// `LocalId` destination slot, or is a no-op placeholder.
#[derive(Debug, Clone, PartialEq)]
pub enum Inst {
    /// `_dest = rvalue`
    Assign { dest: LocalId, value: RValue },

    /// No-op placeholder (eliminated in later passes).
    Nop,
}

impl Inst {
    /// Convenience: create an assign instruction.
    pub fn assign(dest: LocalId, value: RValue) -> Self {
        Inst::Assign { dest, value }
    }

    /// Returns the destination local, if this is an `Assign`.
    pub fn dest(&self) -> Option<LocalId> {
        match self {
            Inst::Assign { dest, .. } => Some(*dest),
            Inst::Nop => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Terminator — the control-flow transfer at the end of a basic block
// ---------------------------------------------------------------------------

/// The terminator of a basic block.
///
/// Every basic block ends with exactly one `Terminator` that transfers
/// control to one or more successor blocks, or exits the function.
#[derive(Debug, Clone, PartialEq)]
pub enum Terminator {
    // ── Return ───────────────────────────────────────────────────────────────
    /// Return from the function, optionally with a value.
    Return(Option<Operand>),

    // ── Unconditional jump ───────────────────────────────────────────────────
    /// Unconditional branch to a block: `goto bb3`.
    Goto(BlockId),

    // ── Conditional branch ───────────────────────────────────────────────────
    /// Conditional branch: `if _cond { goto then } else { goto else }`.
    Branch {
        cond: Operand,
        then_block: BlockId,
        else_block: BlockId,
    },

    // ── Multi-way branch ─────────────────────────────────────────────────────
    /// Multi-way branch driven by an integer discriminant value.
    ///
    /// `arms` maps each discriminant value to a target block.
    /// `otherwise` is taken if none of the arms match.
    Switch {
        value: Operand,
        arms: Vec<(i64, BlockId)>,
        otherwise: BlockId,
    },

    // ── Function call with continuation ──────────────────────────────────────
    /// A function call.
    ///
    /// After the call returns, execution resumes at `target`.
    /// The return value (if any) is stored in `dest`.
    Call {
        callee: Operand,
        args: Vec<Operand>,
        dest: Option<LocalId>,
        target: BlockId,
    },

    // ── Diverging ────────────────────────────────────────────────────────────
    /// Marks a block that can never be reached (after `panic`, `unreachable`,
    /// or a `never`-typed expression).
    Unreachable,
}

impl Terminator {
    /// Returns all successor block IDs (blocks that this terminator can
    /// transfer control to).
    pub fn successors(&self) -> Vec<BlockId> {
        match self {
            Terminator::Return(_) | Terminator::Unreachable => vec![],
            Terminator::Goto(b) => vec![*b],
            Terminator::Branch {
                then_block,
                else_block,
                ..
            } => vec![*then_block, *else_block],
            Terminator::Switch {
                arms, otherwise, ..
            } => {
                let mut targets: Vec<BlockId> = arms.iter().map(|(_, b)| *b).collect();
                targets.push(*otherwise);
                targets.dedup();
                targets
            }
            Terminator::Call { target, .. } => vec![*target],
        }
    }

    /// Returns `true` if this terminator exits the function.
    pub fn is_return(&self) -> bool {
        matches!(self, Terminator::Return(_))
    }

    /// Returns `true` if this is the `Unreachable` sentinel.
    pub fn is_unreachable(&self) -> bool {
        matches!(self, Terminator::Unreachable)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::{BlockId, Const, LocalId, Operand};

    #[test]
    fn test_terminator_successors_return() {
        let t = Terminator::Return(None);
        assert!(t.successors().is_empty());
    }

    #[test]
    fn test_terminator_successors_goto() {
        let t = Terminator::Goto(BlockId(3));
        assert_eq!(t.successors(), vec![BlockId(3)]);
    }

    #[test]
    fn test_terminator_successors_branch() {
        let t = Terminator::Branch {
            cond: Operand::bool_const(true),
            then_block: BlockId(1),
            else_block: BlockId(2),
        };
        assert_eq!(t.successors(), vec![BlockId(1), BlockId(2)]);
    }

    #[test]
    fn test_terminator_successors_switch() {
        let t = Terminator::Switch {
            value: Operand::int_const(0),
            arms: vec![(0, BlockId(1)), (1, BlockId(2))],
            otherwise: BlockId(3),
        };
        let succs = t.successors();
        assert!(succs.contains(&BlockId(1)));
        assert!(succs.contains(&BlockId(2)));
        assert!(succs.contains(&BlockId(3)));
    }

    #[test]
    fn test_inst_assign_dest() {
        let inst = Inst::assign(LocalId(5), RValue::Use(Operand::unit()));
        assert_eq!(inst.dest(), Some(LocalId(5)));
    }

    #[test]
    fn test_inst_nop_has_no_dest() {
        assert_eq!(Inst::Nop.dest(), None);
    }

    #[test]
    fn test_terminator_display_return_none() {
        assert_eq!(format!("{}", Terminator::Return(None)), "return");
    }

    #[test]
    fn test_terminator_display_goto() {
        assert_eq!(format!("{}", Terminator::Goto(BlockId(4))), "goto bb4");
    }

    #[test]
    fn test_terminator_is_return() {
        assert!(Terminator::Return(None).is_return());
        assert!(!Terminator::Goto(BlockId(0)).is_return());
    }
}
