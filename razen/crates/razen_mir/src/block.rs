//! Basic Block.
//!
//! A `BasicBlock` is a maximal straight-line sequence of instructions
//! that ends with exactly one `Terminator`.  Control only enters a basic
//! block at its first instruction and only leaves through its terminator.
//!
//! # Invariants
//!
//! * `term` is set to `Terminator::Unreachable` by default; callers **must**
//!   call `set_term` before the block is considered complete.
//! * Instructions are appended with `push`; the terminator is set separately
//!   with `set_term`.

use crate::inst::{Inst, Terminator};
use crate::value::BlockId;

// ---------------------------------------------------------------------------
// BasicBlock
// ---------------------------------------------------------------------------

/// A basic block inside a `MirFn`.
#[derive(Debug, Clone)]
pub struct BasicBlock {
    /// The unique identifier of this block within its function.
    pub id: BlockId,

    /// The straight-line instructions.  May be empty.
    pub insts: Vec<Inst>,

    /// The control-flow transfer at the end of the block.
    /// Initially `Terminator::Unreachable` (a placeholder); must be set
    /// before MIR is considered well-formed.
    pub term: Terminator,
}

impl BasicBlock {
    /// Create a new, empty basic block with id `id`.
    ///
    /// The terminator is initialised to `Terminator::Unreachable` as a
    /// sentinel — callers should always call `set_term` before the block
    /// is emitted to codegen.
    pub fn new(id: BlockId) -> Self {
        Self {
            id,
            insts: Vec::new(),
            term: Terminator::Unreachable,
        }
    }

    /// Append an instruction to the end of this block.
    ///
    /// `Inst::Nop` instructions are silently dropped to keep the IR clean.
    pub fn push(&mut self, inst: Inst) {
        if !matches!(inst, Inst::Nop) {
            self.insts.push(inst);
        }
    }

    /// Overwrite the block's terminator.
    ///
    /// Calling this more than once on the same block is allowed during
    /// construction (the last call wins), but the final MIR should have
    /// each block terminated exactly once.
    pub fn set_term(&mut self, term: Terminator) {
        self.term = term;
    }

    /// Returns `true` if the terminator has been explicitly set
    /// (i.e. it is not the default `Unreachable` placeholder).
    pub fn is_terminated(&self) -> bool {
        !matches!(self.term, Terminator::Unreachable)
    }

    /// Returns the number of instructions in this block (excluding the
    /// terminator).
    pub fn inst_count(&self) -> usize {
        self.insts.len()
    }

    /// Returns `true` if this block has no instructions (only a terminator).
    pub fn is_empty(&self) -> bool {
        self.insts.is_empty()
    }

    /// Returns a slice over the instructions in this block.
    pub fn instructions(&self) -> &[Inst] {
        &self.insts
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inst::{Inst, RValue, Terminator};
    use crate::value::{BlockId, LocalId, Operand};

    #[test]
    fn test_new_block_is_unreachable() {
        let bb = BasicBlock::new(BlockId(0));
        assert!(matches!(bb.term, Terminator::Unreachable));
        assert!(!bb.is_terminated());
    }

    #[test]
    fn test_push_and_count() {
        let mut bb = BasicBlock::new(BlockId(1));
        assert_eq!(bb.inst_count(), 0);
        assert!(bb.is_empty());

        bb.push(Inst::assign(LocalId(0), RValue::Use(Operand::unit())));
        assert_eq!(bb.inst_count(), 1);
        assert!(!bb.is_empty());
    }

    #[test]
    fn test_push_nop_is_ignored() {
        let mut bb = BasicBlock::new(BlockId(2));
        bb.push(Inst::Nop);
        assert_eq!(bb.inst_count(), 0, "Nop should be dropped");
    }

    #[test]
    fn test_set_term_marks_terminated() {
        let mut bb = BasicBlock::new(BlockId(3));
        assert!(!bb.is_terminated());
        bb.set_term(Terminator::Return(None));
        assert!(bb.is_terminated());
    }

    #[test]
    fn test_set_term_can_be_overwritten() {
        let mut bb = BasicBlock::new(BlockId(4));
        bb.set_term(Terminator::Goto(BlockId(1)));
        bb.set_term(Terminator::Return(None));
        assert!(matches!(bb.term, Terminator::Return(None)));
    }

    #[test]
    fn test_instructions_slice() {
        let mut bb = BasicBlock::new(BlockId(5));
        bb.push(Inst::assign(
            LocalId(1),
            RValue::Use(Operand::int_const(42)),
        ));
        bb.push(Inst::assign(
            LocalId(2),
            RValue::Use(Operand::bool_const(true)),
        ));
        assert_eq!(bb.instructions().len(), 2);
    }
}
