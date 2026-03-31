//! MIR Pretty Printer.
//!
//! Implements `std::fmt::Display` for `MirProgram`, `MirFn`, and all
//! supporting types so the entire MIR can be dumped as human-readable text
//! for debugging, testing, and diagnostic output.
//!
//! # Format overview
//!
//! ```text
//! struct Point {
//!     x: float,
//!     y: float,
//! }
//!
//! enum Direction { North(0), South(1), East(2), West(3) }
//!
//! fn add(_1: int, _2: int) -> int {
//!     let _0: int;          // return slot
//!     let _3: int;          // tmp
//!
//!     bb0:
//!         _3 = BinOp(Add, _1, _2)
//!         return _3
//! }
//! ```

use std::fmt;

use crate::func::{LocalDecl, MirFn};
use crate::inst::{Inst, RValue, Terminator};
use crate::program::{MirEnum, MirProgram, MirStruct, MirVariant, MirVariantKind};
use crate::ty::MirTy;
use crate::value::{BlockId, Const, LocalId, Operand};

// ---------------------------------------------------------------------------
// MirProgram
// ---------------------------------------------------------------------------

impl fmt::Display for MirProgram {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Struct definitions
        for s in &self.structs {
            write!(f, "{}", s)?;
            writeln!(f)?;
        }

        // Enum definitions
        for e in &self.enums {
            write!(f, "{}", e)?;
            writeln!(f)?;
        }

        // Function definitions
        for func in &self.functions {
            write!(f, "{}", func)?;
            writeln!(f)?;
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// MirStruct
// ---------------------------------------------------------------------------

impl fmt::Display for MirStruct {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pub_kw = if self.is_pub { "pub " } else { "" };
        writeln!(f, "{}struct {} {{", pub_kw, self.name)?;
        for (name, ty) in &self.fields {
            writeln!(f, "    {}: {},", name, ty)?;
        }
        write!(f, "}}")
    }
}

// ---------------------------------------------------------------------------
// MirEnum
// ---------------------------------------------------------------------------

impl fmt::Display for MirEnum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pub_kw = if self.is_pub { "pub " } else { "" };
        write!(f, "{}enum {} {{", pub_kw, self.name)?;
        for (i, v) in self.variants.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, " {}", v)?;
        }
        write!(f, " }}")
    }
}

impl fmt::Display for MirVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({})", self.name, self.discriminant)?;
        match &self.kind {
            MirVariantKind::Unit => Ok(()),
            MirVariantKind::Positional(fields) => {
                write!(f, "(")?;
                for (i, ty) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", ty)?;
                }
                write!(f, ")")
            }
            MirVariantKind::Named(fields) => {
                write!(f, " {{ ")?;
                for (i, (name, ty)) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", name, ty)?;
                }
                write!(f, " }}")
            }
        }
    }
}

// ---------------------------------------------------------------------------
// MirFn
// ---------------------------------------------------------------------------

impl fmt::Display for MirFn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Function signature line
        let pub_kw = if self.is_pub { "pub " } else { "" };
        let async_kw = if self.is_async { "async " } else { "" };

        write!(f, "{}{}fn {}(", pub_kw, async_kw, self.name)?;

        // Parameters (locals 1..=param_count)
        let params: Vec<&LocalDecl> = self.locals.iter().filter(|l| l.is_param).collect();
        for (i, param) in params.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            let name = param.name.as_deref().unwrap_or("_");
            write!(f, "{}: {}", name, param.ty)?;
        }

        writeln!(f, ") -> {} {{", self.ret_ty)?;

        // Local declarations (return slot + non-param locals)
        // Return slot
        if let Some(ret_slot) = self.locals.first() {
            writeln!(f, "    // return slot")?;
            writeln!(f, "    let mut _{}: {};", ret_slot.id.0, ret_slot.ty)?;
        }

        // Named bindings and temporaries
        let non_params: Vec<&LocalDecl> = self
            .locals
            .iter()
            .filter(|l| !l.is_param && !l.is_return_slot())
            .collect();

        if !non_params.is_empty() {
            writeln!(f)?;
            for local in &non_params {
                let mut_kw = if local.is_mut { "mut " } else { "" };
                let comment = match &local.name {
                    Some(name) if !name.starts_with("_ret") && !name.starts_with("_tmp") => {
                        format!(" // {}", name)
                    }
                    _ => String::new(),
                };
                writeln!(
                    f,
                    "    let {}_{}: {};{}",
                    mut_kw, local.id.0, local.ty, comment
                )?;
            }
        }

        writeln!(f)?;

        // Basic blocks
        for block in &self.blocks {
            writeln!(f, "  bb{}:", block.id.0)?;
            for inst in &block.insts {
                writeln!(f, "      {}", inst)?;
            }
            writeln!(f, "      {}", block.term)?;
        }

        write!(f, "}}")
    }
}

// ---------------------------------------------------------------------------
// Instruction display
// ---------------------------------------------------------------------------

impl fmt::Display for Inst {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Inst::Assign { dest, value } => write!(f, "{} = {}", dest, value),
            Inst::Nop => write!(f, "nop"),
        }
    }
}

// ---------------------------------------------------------------------------
// RValue display
// ---------------------------------------------------------------------------

impl fmt::Display for RValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RValue::Use(op) => write!(f, "{}", op),

            RValue::BinOp { op, left, right } => {
                write!(f, "BinOp({:?}, {}, {})", op, left, right)
            }

            RValue::UnaryOp { op, operand } => write!(f, "UnaryOp({:?}, {})", op, operand),

            RValue::Call { callee, args } => {
                write!(f, "call {}(", callee)?;
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", a)?;
                }
                write!(f, ")")
            }

            RValue::Struct { name, fields } => {
                write!(f, "{} {{", name)?;
                for (i, (fname, val)) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, " {}: {}", fname, val)?;
                }
                write!(f, " }}")
            }

            RValue::Tuple(ops) => {
                write!(f, "(")?;
                for (i, op) in ops.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", op)?;
                }
                write!(f, ")")
            }

            RValue::Vec { elem_ty, elements } => {
                write!(f, "Vec<{}>[ ", elem_ty)?;
                for (i, op) in elements.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", op)?;
                }
                write!(f, "]")
            }

            RValue::Map {
                key_ty,
                val_ty,
                entries,
            } => {
                write!(f, "Map<{}, {}>[", key_ty, val_ty)?;
                for (i, (k, v)) in entries.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "]")
            }

            RValue::Set { elem_ty, elements } => {
                write!(f, "Set<{}>{{", elem_ty)?;
                for (i, op) in elements.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", op)?;
                }
                write!(f, "}}")
            }

            RValue::Array { elem_ty, elements } => {
                write!(f, "[{}; {}](", elem_ty, elements.len())?;
                for (i, op) in elements.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", op)?;
                }
                write!(f, ")")
            }

            RValue::SomeWrap(op) => write!(f, "some({})", op),
            RValue::None { ty } => write!(f, "none<{}>", ty),
            RValue::OkWrap(op) => write!(f, "ok({})", op),
            RValue::ErrWrap(op) => write!(f, "err({})", op),

            RValue::EnumVariant {
                enum_name,
                variant,
                payload,
            } => {
                write!(f, "{}.{}", enum_name, variant)?;
                if !payload.is_empty() {
                    write!(f, "(")?;
                    for (i, p) in payload.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", p)?;
                    }
                    write!(f, ")")?;
                }
                Ok(())
            }

            RValue::Field { base, field } => write!(f, "{}.{}", base, field),
            RValue::FieldIdx { base, index } => write!(f, "{}.{}", base, index),
            RValue::Discriminant(op) => write!(f, "discriminant({})", op),
            RValue::Index { base, index } => write!(f, "{}[{}]", base, index),

            RValue::Cast { operand, ty } => write!(f, "{} as {}", operand, ty),

            RValue::Closure {
                func_name,
                captures,
            } => {
                write!(f, "closure<{}>", func_name)?;
                if !captures.is_empty() {
                    write!(f, "[")?;
                    for (i, (name, val)) in captures.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}={}", name, val)?;
                    }
                    write!(f, "]")?;
                }
                Ok(())
            }

            RValue::Tensor(ops) => {
                write!(f, "tensor[")?;
                for (i, op) in ops.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", op)?;
                }
                write!(f, "]")
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Terminator display
// ---------------------------------------------------------------------------

impl fmt::Display for Terminator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Terminator::Return(None) => write!(f, "return"),
            Terminator::Return(Some(v)) => write!(f, "return {}", v),
            Terminator::Goto(b) => write!(f, "goto {}", b),
            Terminator::Branch {
                cond,
                then_block,
                else_block,
            } => write!(f, "branch {} → {} | {}", cond, then_block, else_block),
            Terminator::Switch {
                value,
                arms,
                otherwise,
            } => {
                write!(f, "switch {} [", value)?;
                for (disc, blk) in arms {
                    write!(f, "{} → {}, ", disc, blk)?;
                }
                write!(f, "_ → {}]", otherwise)
            }
            Terminator::Call {
                callee,
                args,
                dest,
                target,
            } => {
                if let Some(d) = dest {
                    write!(f, "{} = ", d)?;
                }
                write!(f, "call {}(", callee)?;
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", a)?;
                }
                write!(f, ") → {}", target)
            }
            Terminator::Unreachable => write!(f, "unreachable"),
        }
    }
}

// ---------------------------------------------------------------------------
// Value display helpers (for Operand, Const, LocalId, BlockId)
// ---------------------------------------------------------------------------

impl fmt::Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Operand::Local(id) => write!(f, "{}", id),
            Operand::Const(c) => write!(f, "{}", c),
        }
    }
}

impl fmt::Display for Const {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Const::Bool(b) => write!(f, "{}", b),
            Const::Int(n) => write!(f, "{}", n),
            Const::Uint(n) => write!(f, "{}u", n),
            Const::Float(v) => {
                // Always show a decimal point so it's clearly a float.
                if v.fract() == 0.0 {
                    write!(f, "{:.1}", v)
                } else {
                    write!(f, "{}", v)
                }
            }
            Const::Str(s) => write!(f, "{:?}", s),
            Const::Char(c) => write!(f, "'{}'", c),
            Const::Unit => write!(f, "()"),
            Const::Null => write!(f, "null"),
        }
    }
}

impl fmt::Display for LocalId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "_{}", self.0)
    }
}

impl fmt::Display for BlockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "bb{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::func::MirFn;
    use crate::inst::{Inst, RValue, Terminator};
    use crate::program::{MirEnum, MirProgram, MirStruct, MirVariant};
    use crate::ty::MirTy;
    use crate::value::{BlockId, Const, LocalId, Operand};

    fn simple_fn(name: &str) -> MirFn {
        let mut f = MirFn::new(name.to_string(), MirTy::Int, false, true);
        let p = f.new_param(MirTy::Int, Some("n".to_string()));
        let entry = f.entry();
        f.terminate(entry, Terminator::Return(Some(Operand::Local(p))));
        f
    }

    #[test]
    fn test_display_const_int() {
        assert_eq!(format!("{}", Const::Int(42)), "42");
        assert_eq!(format!("{}", Const::Int(-1)), "-1");
    }

    #[test]
    fn test_display_const_float() {
        assert_eq!(format!("{}", Const::Float(1.0)), "1.0");
        assert_eq!(format!("{}", Const::Float(3.14)), "3.14");
    }

    #[test]
    fn test_display_const_str() {
        assert_eq!(format!("{}", Const::Str("hello".into())), "\"hello\"");
    }

    #[test]
    fn test_display_const_unit() {
        assert_eq!(format!("{}", Const::Unit), "()");
    }

    #[test]
    fn test_display_local_id() {
        assert_eq!(format!("{}", LocalId(5)), "_5");
    }

    #[test]
    fn test_display_block_id() {
        assert_eq!(format!("{}", BlockId(3)), "bb3");
    }

    #[test]
    fn test_display_operand_local() {
        let op = Operand::Local(LocalId(2));
        assert_eq!(format!("{}", op), "_2");
    }

    #[test]
    fn test_display_operand_const() {
        let op = Operand::Const(Const::Bool(true));
        assert_eq!(format!("{}", op), "true");
    }

    #[test]
    fn test_display_terminator_return_none() {
        assert_eq!(format!("{}", Terminator::Return(None)), "return");
    }

    #[test]
    fn test_display_terminator_return_value() {
        let t = Terminator::Return(Some(Operand::Local(LocalId(1))));
        assert_eq!(format!("{}", t), "return _1");
    }

    #[test]
    fn test_display_terminator_goto() {
        assert_eq!(format!("{}", Terminator::Goto(BlockId(2))), "goto bb2");
    }

    #[test]
    fn test_display_terminator_branch() {
        let t = Terminator::Branch {
            cond: Operand::Local(LocalId(0)),
            then_block: BlockId(1),
            else_block: BlockId(2),
        };
        let s = format!("{}", t);
        assert!(s.contains("_0"), "should contain cond local");
        assert!(s.contains("bb1"), "should contain then block");
        assert!(s.contains("bb2"), "should contain else block");
    }

    #[test]
    fn test_display_rvalue_use() {
        let rv = RValue::Use(Operand::Const(Const::Int(99)));
        assert_eq!(format!("{}", rv), "99");
    }

    #[test]
    fn test_display_rvalue_binop() {
        let rv = RValue::BinOp {
            op: razen_ast::ops::BinOp::Add,
            left: Operand::Local(LocalId(1)),
            right: Operand::Local(LocalId(2)),
        };
        let s = format!("{}", rv);
        assert!(s.contains("Add"), "should name the op");
        assert!(s.contains("_1"), "should reference left");
        assert!(s.contains("_2"), "should reference right");
    }

    #[test]
    fn test_display_rvalue_struct() {
        let rv = RValue::Struct {
            name: "Point".to_string(),
            fields: vec![
                ("x".to_string(), Operand::Const(Const::Float(1.0))),
                ("y".to_string(), Operand::Const(Const::Float(2.0))),
            ],
        };
        let s = format!("{}", rv);
        assert!(s.contains("Point"), "should mention struct name");
        assert!(s.contains("x"), "should mention field x");
        assert!(s.contains("y"), "should mention field y");
    }

    #[test]
    fn test_display_rvalue_none() {
        let rv = RValue::None { ty: MirTy::Int };
        let s = format!("{}", rv);
        assert!(s.contains("none"), "should say none");
    }

    #[test]
    fn test_display_rvalue_some_wrap() {
        let rv = RValue::SomeWrap(Operand::Local(LocalId(3)));
        assert_eq!(format!("{}", rv), "some(_3)");
    }

    #[test]
    fn test_display_inst_assign() {
        let inst = Inst::assign(LocalId(4), RValue::Use(Operand::Const(Const::Int(7))));
        let s = format!("{}", inst);
        assert!(s.contains("_4"), "should show dest");
        assert!(s.contains("7"), "should show value");
    }

    #[test]
    fn test_display_inst_nop() {
        assert_eq!(format!("{}", Inst::Nop), "nop");
    }

    #[test]
    fn test_display_mir_struct() {
        let s = MirStruct {
            name: "Point".to_string(),
            fields: vec![
                ("x".to_string(), MirTy::Float),
                ("y".to_string(), MirTy::Float),
            ],
            is_pub: false,
        };
        let text = format!("{}", s);
        assert!(text.contains("struct Point"), "should have struct header");
        assert!(text.contains("x: float"), "should have field x");
        assert!(text.contains("y: float"), "should have field y");
    }

    #[test]
    fn test_display_mir_enum() {
        let e = MirEnum {
            name: "Direction".to_string(),
            variants: vec![
                MirVariant::unit("North".to_string(), 0),
                MirVariant::unit("South".to_string(), 1),
            ],
            is_pub: true,
        };
        let text = format!("{}", e);
        assert!(text.contains("enum Direction"), "should name the enum");
        assert!(text.contains("North"), "should list variants");
        assert!(text.contains("South"), "should list variants");
    }

    #[test]
    fn test_display_mir_fn_contains_name() {
        let f = simple_fn("add");
        let text = format!("{}", f);
        assert!(text.contains("fn add"), "should show function name");
    }

    #[test]
    fn test_display_mir_fn_shows_param() {
        let f = simple_fn("add");
        let text = format!("{}", f);
        assert!(text.contains("int"), "should show param type");
    }

    #[test]
    fn test_display_mir_fn_shows_return() {
        let f = simple_fn("add");
        let text = format!("{}", f);
        assert!(text.contains("return"), "should show return");
    }

    #[test]
    fn test_display_mir_program_empty() {
        let prog = MirProgram::new();
        let text = format!("{}", prog);
        // Empty program should produce no output (or only whitespace).
        assert!(text.trim().is_empty());
    }

    #[test]
    fn test_display_mir_program_with_fn() {
        let mut prog = MirProgram::new();
        prog.add_fn(simple_fn("main"));
        let text = format!("{}", prog);
        assert!(text.contains("fn main"), "should contain function name");
    }

    #[test]
    fn test_display_rvalue_vec() {
        let rv = RValue::Vec {
            elem_ty: MirTy::Int,
            elements: vec![Operand::Const(Const::Int(1)), Operand::Const(Const::Int(2))],
        };
        let s = format!("{}", rv);
        assert!(s.contains("Vec"), "should say Vec");
        assert!(s.contains("int"), "should mention element type");
    }

    #[test]
    fn test_display_rvalue_tuple() {
        let rv = RValue::Tuple(vec![
            Operand::Const(Const::Int(1)),
            Operand::Const(Const::Bool(true)),
        ]);
        let s = format!("{}", rv);
        assert!(s.contains("1"), "should show first element");
        assert!(s.contains("true"), "should show second element");
    }

    #[test]
    fn test_display_rvalue_cast() {
        let rv = RValue::Cast {
            operand: Operand::Local(LocalId(1)),
            ty: MirTy::Float,
        };
        let s = format!("{}", rv);
        assert!(s.contains("_1"), "should show source");
        assert!(s.contains("float"), "should show target type");
        assert!(s.contains("as"), "should use 'as' keyword");
    }

    #[test]
    fn test_display_rvalue_field() {
        let rv = RValue::Field {
            base: Operand::Local(LocalId(2)),
            field: "name".to_string(),
        };
        assert_eq!(format!("{}", rv), "_2.name");
    }

    #[test]
    fn test_display_rvalue_index() {
        let rv = RValue::Index {
            base: Operand::Local(LocalId(0)),
            index: Operand::Local(LocalId(1)),
        };
        assert_eq!(format!("{}", rv), "_0[_1]");
    }

    #[test]
    fn test_display_rvalue_discriminant() {
        let rv = RValue::Discriminant(Operand::Local(LocalId(5)));
        assert_eq!(format!("{}", rv), "discriminant(_5)");
    }

    #[test]
    fn test_display_rvalue_enum_variant_unit() {
        let rv = RValue::EnumVariant {
            enum_name: "Direction".to_string(),
            variant: "North".to_string(),
            payload: vec![],
        };
        assert_eq!(format!("{}", rv), "Direction.North");
    }

    #[test]
    fn test_display_rvalue_enum_variant_with_payload() {
        let rv = RValue::EnumVariant {
            enum_name: "Shape".to_string(),
            variant: "Circle".to_string(),
            payload: vec![Operand::Const(Const::Float(5.0))],
        };
        let s = format!("{}", rv);
        assert!(s.contains("Shape.Circle"), "should name the variant");
        assert!(s.contains("5"), "should show payload");
    }

    #[test]
    fn test_display_terminator_switch() {
        let t = Terminator::Switch {
            value: Operand::Local(LocalId(0)),
            arms: vec![(0, BlockId(1)), (1, BlockId(2))],
            otherwise: BlockId(3),
        };
        let s = format!("{}", t);
        assert!(s.contains("switch"), "should say switch");
        assert!(s.contains("bb1"), "should list arm target bb1");
        assert!(s.contains("bb3"), "should list default target bb3");
    }

    #[test]
    fn test_display_terminator_call() {
        let t = Terminator::Call {
            callee: Operand::Const(Const::Str("println".into())),
            args: vec![Operand::Local(LocalId(1))],
            dest: Some(LocalId(2)),
            target: BlockId(5),
        };
        let s = format!("{}", t);
        assert!(s.contains("call"), "should say call");
        assert!(s.contains("println"), "should name the callee");
        assert!(s.contains("bb5"), "should show continuation block");
    }
}
