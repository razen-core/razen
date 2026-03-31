//! MIR Function Representation.
//!
//! A `MirFn` is a complete function in MIR form, consisting of:
//!   • A list of `LocalDecl`s — every named binding and temporary slot.
//!   • A list of `BasicBlock`s — the control-flow graph of the function.
//!
//! # Local variable layout convention
//!
//! | Index | Role                                            |
//! |-------|-------------------------------------------------|
//! | 0     | Return-value slot (may be `MirTy::Void`)        |
//! | 1..P  | Function parameters (P = `param_count`)         |
//! | P+1.. | Compiler-generated temporaries & named bindings |
//!
//! The entry block is always `BlockId(0)`.

use crate::block::BasicBlock;
use crate::inst::{Inst, RValue, Terminator};
use crate::ty::MirTy;
use crate::value::{BlockId, LocalId, Operand};

// ---------------------------------------------------------------------------
// Local variable declaration
// ---------------------------------------------------------------------------

/// A declared local variable slot inside a `MirFn`.
#[derive(Debug, Clone)]
pub struct LocalDecl {
    /// The unique identifier for this local within the function.
    pub id: LocalId,

    /// The resolved MIR type of this local.
    pub ty: MirTy,

    /// Optional source-level name, kept for debug output.
    /// `None` for anonymous compiler temporaries.
    pub name: Option<String>,

    /// Whether the binding was declared `mut` or `shared`.
    pub is_mut: bool,

    /// Whether this local is a function parameter (including `self`).
    pub is_param: bool,

    /// 0-based parameter index (only meaningful when `is_param = true`).
    pub param_index: Option<usize>,
}

impl LocalDecl {
    /// Returns `true` if this local is the return-value slot (index 0).
    pub fn is_return_slot(&self) -> bool {
        self.id.0 == 0
    }

    /// Returns `true` if this is an anonymous temporary.
    pub fn is_temp(&self) -> bool {
        self.name.is_none() && !self.is_param
    }

    /// Returns the display name for debug output.
    pub fn display_name(&self) -> String {
        match &self.name {
            Some(n) => n.clone(),
            None => format!("_tmp{}", self.id.0),
        }
    }
}

// ---------------------------------------------------------------------------
// MirFn
// ---------------------------------------------------------------------------

/// A complete MIR function.
#[derive(Debug, Clone)]
pub struct MirFn {
    /// The (possibly mangled) function name.
    pub name: String,

    /// Whether this is an `async act`.
    pub is_async: bool,

    /// Whether this function is `pub`.
    pub is_pub: bool,

    /// The declared return type.
    pub ret_ty: MirTy,

    /// All local variable declarations.
    ///
    /// Local 0 is the return-value slot.
    /// Locals 1..=param_count are parameters (in declaration order).
    /// The rest are temporaries and named bindings.
    pub locals: Vec<LocalDecl>,

    /// Number of parameters (excluding the return-value slot at index 0).
    pub param_count: usize,

    /// All basic blocks.  `blocks[0]` is always the entry block.
    pub blocks: Vec<BasicBlock>,
}

impl MirFn {
    // -----------------------------------------------------------------------
    // Construction
    // -----------------------------------------------------------------------

    /// Create a new `MirFn` with an empty entry block already allocated.
    pub fn new(name: String, ret_ty: MirTy, is_async: bool, is_pub: bool) -> Self {
        let mut func = Self {
            name,
            is_async,
            is_pub,
            ret_ty: ret_ty.clone(),
            locals: Vec::new(),
            param_count: 0,
            blocks: Vec::new(),
        };

        // Local 0 — return-value slot.
        func.locals.push(LocalDecl {
            id: LocalId(0),
            ty: ret_ty,
            name: Some("_ret".to_string()),
            is_mut: true,
            is_param: false,
            param_index: None,
        });

        // Create the entry block (block 0).
        func.blocks.push(BasicBlock::new(BlockId(0)));

        func
    }

    // -----------------------------------------------------------------------
    // Local allocation
    // -----------------------------------------------------------------------

    /// Allocate a new anonymous temporary local and return its `LocalId`.
    pub fn new_local(&mut self, ty: MirTy, name: Option<String>, is_mut: bool) -> LocalId {
        let id = LocalId(self.locals.len() as u32);
        self.locals.push(LocalDecl {
            id,
            ty,
            name,
            is_mut,
            is_param: false,
            param_index: None,
        });
        id
    }

    /// Allocate a new anonymous temporary local (shorthand).
    pub fn new_tmp(&mut self, ty: MirTy) -> LocalId {
        self.new_local(ty, None, false)
    }

    /// Allocate a named local (for variable bindings).
    pub fn new_named(&mut self, name: &str, ty: MirTy, is_mut: bool) -> LocalId {
        self.new_local(ty, Some(name.to_string()), is_mut)
    }

    /// Allocate a parameter slot and return its `LocalId`.
    ///
    /// Parameters must be allocated in order, immediately after construction.
    pub fn new_param(&mut self, ty: MirTy, name: Option<String>) -> LocalId {
        let id = LocalId(self.locals.len() as u32);
        let idx = self.param_count;
        self.locals.push(LocalDecl {
            id,
            ty,
            name,
            is_mut: false,
            is_param: true,
            param_index: Some(idx),
        });
        self.param_count += 1;
        id
    }

    // -----------------------------------------------------------------------
    // Block management
    // -----------------------------------------------------------------------

    /// Create a new (empty) basic block and return its `BlockId`.
    pub fn new_block(&mut self) -> BlockId {
        let id = BlockId(self.blocks.len() as u32);
        self.blocks.push(BasicBlock::new(id));
        id
    }

    /// Get an immutable reference to a basic block by `BlockId`.
    ///
    /// # Panics
    ///
    /// Panics if `id` is out of bounds.
    pub fn block(&self, id: BlockId) -> &BasicBlock {
        &self.blocks[id.0 as usize]
    }

    /// Get a mutable reference to a basic block by `BlockId`.
    ///
    /// # Panics
    ///
    /// Panics if `id` is out of bounds.
    pub fn block_mut(&mut self, id: BlockId) -> &mut BasicBlock {
        &mut self.blocks[id.0 as usize]
    }

    /// The entry block ID (always `BlockId(0)`).
    pub fn entry(&self) -> BlockId {
        BlockId(0)
    }

    // -----------------------------------------------------------------------
    // Convenience emitters
    // -----------------------------------------------------------------------

    /// Append an instruction to a specific block.
    pub fn emit_to(&mut self, block: BlockId, inst: Inst) {
        self.block_mut(block).push(inst);
    }

    /// Set the terminator of a specific block.
    pub fn terminate(&mut self, block: BlockId, term: Terminator) {
        self.block_mut(block).set_term(term);
    }

    // -----------------------------------------------------------------------
    // Queries
    // -----------------------------------------------------------------------

    /// Returns the `LocalDecl` for a given `LocalId`.
    ///
    /// # Panics
    ///
    /// Panics if `id` is out of bounds.
    pub fn local(&self, id: LocalId) -> &LocalDecl {
        &self.locals[id.0 as usize]
    }

    /// Returns the MIR type of a local variable.
    pub fn local_ty(&self, id: LocalId) -> &MirTy {
        &self.locals[id.0 as usize].ty
    }

    /// Returns the total number of locals (including the return slot and params).
    pub fn local_count(&self) -> usize {
        self.locals.len()
    }

    /// Returns the number of basic blocks.
    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }

    /// Returns an iterator over all parameter `LocalDecl`s (in order).
    pub fn params(&self) -> impl Iterator<Item = &LocalDecl> {
        self.locals
            .iter()
            .filter(|l| l.is_param)
            .take(self.param_count)
    }

    /// Returns `true` if every basic block has been explicitly terminated
    /// (i.e. none of them still hold the default `Unreachable` sentinel).
    pub fn is_fully_terminated(&self) -> bool {
        self.blocks.iter().all(|b| b.is_terminated())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inst::{RValue, Terminator};
    use crate::value::Operand;

    fn make_fn() -> MirFn {
        MirFn::new("test_fn".to_string(), MirTy::Int, false, false)
    }

    #[test]
    fn test_new_fn_has_entry_block() {
        let f = make_fn();
        assert_eq!(f.block_count(), 1);
        assert_eq!(f.entry(), BlockId(0));
    }

    #[test]
    fn test_new_fn_has_return_slot() {
        let f = make_fn();
        // Local 0 = return slot
        assert_eq!(f.local_count(), 1);
        let ret = f.local(LocalId(0));
        assert!(ret.is_return_slot());
        assert_eq!(ret.ty, MirTy::Int);
    }

    #[test]
    fn test_new_param_increments_count() {
        let mut f = make_fn();
        let p0 = f.new_param(MirTy::Int, Some("a".to_string()));
        let p1 = f.new_param(MirTy::Str, Some("b".to_string()));
        assert_eq!(f.param_count, 2);
        assert_eq!(f.local(p0).is_param, true);
        assert_eq!(f.local(p1).is_param, true);
        assert_eq!(f.local(p0).param_index, Some(0));
        assert_eq!(f.local(p1).param_index, Some(1));
    }

    #[test]
    fn test_new_tmp_is_anonymous() {
        let mut f = make_fn();
        let t = f.new_tmp(MirTy::Bool);
        let decl = f.local(t);
        assert!(decl.name.is_none());
        assert!(decl.is_temp());
        assert!(!decl.is_param);
    }

    #[test]
    fn test_new_named_local() {
        let mut f = make_fn();
        let id = f.new_named("counter", MirTy::Int, true);
        let decl = f.local(id);
        assert_eq!(decl.name.as_deref(), Some("counter"));
        assert!(decl.is_mut);
        assert!(!decl.is_param);
    }

    #[test]
    fn test_new_block_increments_count() {
        let mut f = make_fn();
        assert_eq!(f.block_count(), 1);
        let bb1 = f.new_block();
        let bb2 = f.new_block();
        assert_eq!(f.block_count(), 3);
        assert_eq!(bb1, BlockId(1));
        assert_eq!(bb2, BlockId(2));
    }

    #[test]
    fn test_emit_and_terminate() {
        let mut f = make_fn();
        let entry = f.entry();
        let t = f.new_tmp(MirTy::Int);
        f.emit_to(entry, Inst::assign(t, RValue::Use(Operand::int_const(42))));
        f.terminate(entry, Terminator::Return(Some(Operand::local(t))));
        let bb = f.block(entry);
        assert_eq!(bb.inst_count(), 1);
        assert!(bb.is_terminated());
    }

    #[test]
    fn test_is_fully_terminated() {
        let mut f = make_fn();
        assert!(!f.is_fully_terminated()); // entry not terminated yet
        f.terminate(BlockId(0), Terminator::Return(None));
        assert!(f.is_fully_terminated());
    }

    #[test]
    fn test_params_iterator() {
        let mut f = make_fn();
        f.new_param(MirTy::Int, Some("x".to_string()));
        f.new_param(MirTy::Float, Some("y".to_string()));
        f.new_tmp(MirTy::Bool); // not a param
        let params: Vec<_> = f.params().collect();
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].name.as_deref(), Some("x"));
        assert_eq!(params[1].name.as_deref(), Some("y"));
    }

    #[test]
    fn test_local_ty() {
        let mut f = make_fn();
        let id = f.new_tmp(MirTy::Str);
        assert_eq!(f.local_ty(id), &MirTy::Str);
    }

    #[test]
    fn test_display_name_for_temp() {
        let mut f = make_fn();
        let id = f.new_tmp(MirTy::Int);
        // Temp locals have generated names like "_tmpN"
        assert!(f.local(id).display_name().starts_with("_tmp"));
    }

    #[test]
    fn test_display_name_for_named() {
        let mut f = make_fn();
        let id = f.new_named("score", MirTy::Int, false);
        assert_eq!(f.local(id).display_name(), "score");
    }
}
