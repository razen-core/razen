//! # Razen MIR — Mid-level Intermediate Representation
//!
//! This crate defines and implements the MIR for the Razen programming language
//! compiler.  MIR is a 3-address code flat representation based on basic blocks
//! with explicit control flow.  It sits between the type-checked AST (produced
//! by `razen_sema`) and the C code generator (`razen_codegen_c`).
//!
//! ## Pipeline position
//!
//! ```text
//! Source  ──►  razen_lexer
//!         ──►  razen_parser   (AST)
//!         ──►  razen_sema     (SemanticModel: name resolution + types)
//!         ──►  razen_mir      (MirProgram)      ← this crate
//!         ──►  razen_codegen_c (C source)
//!         ──►  GCC / TCC      (native binary)
//! ```
//!
//! ## Primary entry point
//!
//! ```rust,ignore
//! use razen_mir::lower;
//! use razen_sema::analyze;
//!
//! // Assume `module` and `sema` are already produced by earlier phases.
//! let mir = lower(&module, &sema);
//! println!("{}", mir);  // pretty-print the MIR
//! ```
//!
//! ## Crate structure
//!
//! | Module      | Contents                                               |
//! |-------------|--------------------------------------------------------|
//! | `ty`        | `MirTy` — fully resolved, monomorphised types          |
//! | `value`     | `LocalId`, `BlockId`, `Const`, `Operand`               |
//! | `inst`      | `RValue`, `Inst`, `Terminator`                         |
//! | `block`     | `BasicBlock`                                           |
//! | `func`      | `MirFn`, `LocalDecl`                                   |
//! | `program`   | `MirProgram`, `MirStruct`, `MirEnum`, `MirVariant`     |
//! | `lower`     | `Lowerer` — AST + SemanticModel → MirProgram           |
//! | `display`   | `Display` implementations for all MIR types            |

// ---------------------------------------------------------------------------
// Sub-modules
// ---------------------------------------------------------------------------

pub mod block;
pub mod display;
pub mod func;
pub mod inst;
pub mod lower;
pub mod program;
pub mod ty;
pub mod value;

// ---------------------------------------------------------------------------
// Re-exports (flat public API)
// ---------------------------------------------------------------------------

pub use block::BasicBlock;
pub use func::{LocalDecl, MirFn};
pub use inst::{Inst, RValue, Terminator};
pub use lower::{Lowerer, resolve_ast_type};
pub use program::{MirEnum, MirProgram, MirStruct, MirVariant, MirVariantKind};
pub use ty::MirTy;
pub use value::{BlockId, Const, LocalId, Operand};

use razen_ast::Module;
use razen_sema::SemanticModel;

// ---------------------------------------------------------------------------
// Primary entry point
// ---------------------------------------------------------------------------

/// Lower a type-checked Razen module into a `MirProgram`.
///
/// # Arguments
///
/// * `module` — The parsed AST module (from `razen_parser::parse`).
/// * `sema`   — The semantic model (from `razen_sema::analyze`).
///
/// # Returns
///
/// A complete [`MirProgram`] suitable for code generation.
///
/// # Example
///
/// ```rust,ignore
/// let tokens = razen_lexer::tokenize(source);
/// let module = razen_parser::parse_tokens(&tokens).unwrap();
/// let sema   = razen_sema::analyze(&module);
/// let mir    = razen_mir::lower(&module, &sema);
/// println!("{}", mir);
/// ```
pub fn lower(module: &Module, sema: &SemanticModel) -> MirProgram {
    Lowerer::new(sema).lower_module(module)
}

// ---------------------------------------------------------------------------
// Integration tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use razen_sema::analyze;

    /// Compile Razen source all the way to MIR and return the program.
    fn compile(source: &str) -> MirProgram {
        let tokens = razen_lexer::tokenize(source);
        let module = razen_parser::parse_tokens(&tokens).expect("parse failed");
        let sema = analyze(&module);
        lower(&module, &sema)
    }

    // ── Basic function lowering ───────────────────────────────────────────

    #[test]
    fn test_lower_simple_function() {
        let prog = compile(r#"act add(a: int, b: int) int -> a + b"#);
        assert_eq!(prog.fn_count(), 1);
        let f = &prog.functions[0];
        assert_eq!(f.name, "add");
        assert!(!f.blocks.is_empty(), "expected at least one basic block");
    }

    #[test]
    fn test_lower_void_function() {
        let prog = compile(r#"act greet() void { println("hello") }"#);
        assert_eq!(prog.fn_count(), 1);
        let f = &prog.functions[0];
        assert_eq!(f.name, "greet");
        assert_eq!(f.ret_ty, MirTy::Void);
    }

    #[test]
    fn test_lower_function_params_registered() {
        let prog = compile(r#"act sum(a: int, b: int, c: int) int -> a + b + c"#);
        let f = &prog.functions[0];
        // 3 named params
        assert_eq!(f.param_count, 3);
    }

    // ── If / else ─────────────────────────────────────────────────────────

    #[test]
    fn test_lower_if_else_creates_multiple_blocks() {
        let prog = compile(
            r#"act max(a: int, b: int) int {
                if a > b { a } else { b }
            }"#,
        );
        let f = &prog.functions[0];
        // if/else produces: entry, then_bb, else_bb, join_bb = at least 4 blocks
        assert!(
            f.block_count() >= 4,
            "if/else needs ≥ 4 blocks, got {}",
            f.block_count()
        );
    }

    #[test]
    fn test_lower_if_without_else() {
        let prog = compile(
            r#"act maybe_print(flag: bool) void {
                if flag { println("yes") }
            }"#,
        );
        let f = &prog.functions[0];
        // if without else still creates then/else/join blocks
        assert!(f.block_count() >= 3);
    }

    // ── Struct definitions ─────────────────────────────────────────────────

    #[test]
    fn test_lower_struct_definition() {
        let prog = compile(
            r#"struct Point { x: float, y: float }
               act origin() Point { Point { x: 0.0, y: 0.0 } }"#,
        );
        assert_eq!(prog.struct_count(), 1);
        let s = prog.get_struct("Point").expect("Point struct");
        assert_eq!(s.field_count(), 2);
        assert!(s.field("x").is_some());
        assert!(s.field("y").is_some());
    }

    #[test]
    fn test_lower_unit_struct() {
        let prog = compile(r#"struct Marker  act test() void {}"#);
        assert_eq!(prog.struct_count(), 1);
        let s = prog.get_struct("Marker").expect("Marker struct");
        assert!(s.is_unit());
    }

    #[test]
    fn test_lower_struct_literal_in_fn() {
        let prog = compile(
            r#"struct User { id: int, name: str }
               act make() User { User { id: 1, name: "alice" } }"#,
        );
        let f = prog.get_fn("make").expect("make fn");
        // Should have some instructions in entry block
        assert!(!f.blocks.is_empty());
    }

    // ── Enum definitions ──────────────────────────────────────────────────

    #[test]
    fn test_lower_enum_unit_variants() {
        let prog = compile(
            r#"enum Direction { North, South, East, West }
               act go(d: Direction) int { 0 }"#,
        );
        assert_eq!(prog.enum_count(), 1);
        let e = prog.get_enum("Direction").expect("Direction enum");
        assert_eq!(e.variant_count(), 4);
        assert_eq!(e.discriminant_of("North"), Some(0));
        assert_eq!(e.discriminant_of("West"), Some(3));
    }

    #[test]
    fn test_lower_enum_positional_variants() {
        let prog = compile(
            r#"enum Shape { Circle(float), Rect(float, float), Point }
               act area(s: Shape) float { 0.0 }"#,
        );
        let e = prog.get_enum("Shape").expect("Shape enum");
        assert_eq!(e.variant_count(), 3);
        // Circle has 1 positional field
        let circle = e.variant("Circle").expect("Circle variant");
        assert!(matches!(circle.kind, MirVariantKind::Positional(ref f) if f.len() == 1));
        // Point is unit
        let point = e.variant("Point").expect("Point variant");
        assert!(matches!(point.kind, MirVariantKind::Unit));
    }

    // ── Loop lowering ─────────────────────────────────────────────────────

    #[test]
    fn test_lower_infinite_loop() {
        let prog = compile(
            r#"act spin() void {
                loop { println("tick") }
            }"#,
        );
        let f = &prog.functions[0];
        // loop: entry → header → body (≥ 3 blocks)
        assert!(f.block_count() >= 3, "loop needs ≥ 3 blocks");
    }

    #[test]
    fn test_lower_while_loop() {
        let prog = compile(
            r#"act countdown(mut n: int) void {
                loop n > 0 { n -= 1 }
            }"#,
        );
        let f = &prog.functions[0];
        assert!(f.block_count() >= 3);
    }

    #[test]
    fn test_lower_for_in_loop() {
        let prog = compile(
            r#"act sum_range() int {
                mut total: int = 0
                loop i in 0..10 { total += i }
                total
            }"#,
        );
        let f = &prog.functions[0];
        // for-in creates more blocks: entry, header, check, body, exit
        assert!(f.block_count() >= 4);
    }

    // ── Match lowering ────────────────────────────────────────────────────

    #[test]
    fn test_lower_match_int() {
        let prog = compile(
            r#"act describe(n: int) str {
                match n {
                    0 -> "zero",
                    _ -> "other",
                }
            }"#,
        );
        let f = &prog.functions[0];
        assert!(f.block_count() >= 3);
    }

    #[test]
    fn test_lower_match_option() {
        let prog = compile(
            r#"act unwrap_or_zero(opt: option[int]) int {
                match opt {
                    some(x) -> x,
                    none    -> 0,
                }
            }"#,
        );
        let f = &prog.functions[0];
        assert!(!f.blocks.is_empty());
    }

    // ── Impl block lowering ───────────────────────────────────────────────

    #[test]
    fn test_lower_impl_block_methods_mangled() {
        let prog = compile(
            r#"struct Counter { value: int }
               impl Counter {
                   act new() Counter { Counter { value: 0 } }
                   act get(self) int { self.value }
               }"#,
        );
        // Methods should be mangled as "Counter_new" and "Counter_get"
        assert!(prog.get_fn("Counter_new").is_some(), "Counter_new missing");
        assert!(prog.get_fn("Counter_get").is_some(), "Counter_get missing");
    }

    // ── Collections ───────────────────────────────────────────────────────

    #[test]
    fn test_lower_vec_literal() {
        let prog = compile(r#"act nums() vec[int] { vec[1, 2, 3] }"#);
        let f = prog.get_fn("nums").expect("nums fn");
        assert!(!f.blocks.is_empty());
    }

    #[test]
    fn test_lower_map_literal() {
        let prog = compile(r#"act scores() map[str, int] { map["a": 1, "b": 2] }"#);
        let f = prog.get_fn("scores").expect("scores fn");
        assert!(!f.blocks.is_empty());
    }

    #[test]
    fn test_lower_empty_vec() {
        let prog = compile(r#"act empty() vec[int] { vec[] }"#);
        let f = prog.get_fn("empty").expect("empty fn");
        assert!(!f.blocks.is_empty());
    }

    // ── Bindings and assignments ──────────────────────────────────────────

    #[test]
    fn test_lower_let_binding() {
        let prog = compile(
            r#"act test() int {
                x := 42
                x
            }"#,
        );
        let f = prog.get_fn("test").expect("test fn");
        assert!(!f.blocks.is_empty());
    }

    #[test]
    fn test_lower_mut_binding_and_assign() {
        let prog = compile(
            r#"act test() int {
                mut n: int = 0
                n = 99
                n
            }"#,
        );
        let f = prog.get_fn("test").expect("test fn");
        // Should have a mutable local
        let has_mut = f.locals.iter().any(|l| l.is_mut && !l.is_return_slot());
        assert!(has_mut, "expected a mutable local");
    }

    #[test]
    fn test_lower_compound_assign() {
        let prog = compile(
            r#"act test() int {
                mut x: int = 10
                x += 5
                x
            }"#,
        );
        let f = prog.get_fn("test").expect("test fn");
        assert!(!f.blocks.is_empty());
    }

    // ── Tensor ───────────────────────────────────────────────────────────

    #[test]
    fn test_lower_tensor_literal() {
        let prog = compile(r#"act weights() tensor { tensor[0.1, 0.4, 0.2, 0.3] }"#);
        let f = prog.get_fn("weights").expect("weights fn");
        assert!(!f.blocks.is_empty());
        assert_eq!(f.ret_ty, MirTy::Tensor);
    }

    // ── Return ────────────────────────────────────────────────────────────

    #[test]
    fn test_lower_early_return() {
        let prog = compile(
            r#"act abs_val(n: int) int {
                if n < 0 { ret n * -1 }
                n
            }"#,
        );
        let f = prog.get_fn("abs_val").expect("abs_val fn");
        // At least one block should have a Return terminator
        let has_return = f.blocks.iter().any(|b| b.term.is_return());
        assert!(has_return, "expected a return terminator");
    }

    // ── Async ─────────────────────────────────────────────────────────────

    #[test]
    fn test_lower_async_function() {
        let prog = compile(
            r#"async act fetch(url: str) result[str, str] {
                ok("data")
            }"#,
        );
        let f = prog.get_fn("fetch").expect("fetch fn");
        assert!(f.is_async, "should be marked async");
    }

    // ── MIR display ───────────────────────────────────────────────────────

    #[test]
    fn test_mir_display_contains_fn_name() {
        let prog = compile(r#"act hello() void {}"#);
        let text = format!("{}", prog);
        assert!(text.contains("fn hello"), "display should show 'fn hello'");
    }

    #[test]
    fn test_mir_display_contains_struct_name() {
        let prog = compile(
            r#"struct Rect { w: float, h: float }
               act dummy() void {}"#,
        );
        let text = format!("{}", prog);
        assert!(text.contains("struct Rect"), "display should show struct");
    }

    #[test]
    fn test_mir_display_contains_enum_name() {
        let prog = compile(
            r#"enum Color { Red, Green, Blue }
               act dummy() void {}"#,
        );
        let text = format!("{}", prog);
        assert!(text.contains("enum Color"), "display should show enum");
    }

    // ── Multiple items ────────────────────────────────────────────────────

    #[test]
    fn test_lower_multiple_functions() {
        let prog = compile(
            r#"act double(n: int) int { n * 2 }
               act triple(n: int) int { n * 3 }
               act main() void {
                   a := double(5)
                   b := triple(5)
               }"#,
        );
        assert_eq!(prog.fn_count(), 3);
        assert!(prog.get_fn("double").is_some());
        assert!(prog.get_fn("triple").is_some());
        assert!(prog.get_fn("main").is_some());
    }

    #[test]
    fn test_lower_struct_and_impl_and_fn() {
        let prog = compile(
            r#"struct Vec2 { x: float, y: float }
               impl Vec2 {
                   act zero() Vec2 { Vec2 { x: 0.0, y: 0.0 } }
                   act length(self) float { self.x * self.x + self.y * self.y }
               }
               act main() void {
                   v := Vec2.zero()
                   l := v.length()
               }"#,
        );
        assert_eq!(prog.struct_count(), 1);
        assert!(prog.get_fn("Vec2_zero").is_some());
        assert!(prog.get_fn("Vec2_length").is_some());
        assert!(prog.get_fn("main").is_some());
    }

    // ── Guard ─────────────────────────────────────────────────────────────

    #[test]
    fn test_lower_guard_statement() {
        let prog = compile(
            r#"act safe_div(a: int, b: int) int {
                guard b != 0 else { ret 0 }
                a / b
            }"#,
        );
        let f = prog.get_fn("safe_div").expect("safe_div fn");
        // guard creates a branch
        assert!(f.block_count() >= 2);
    }

    // ── Defer ─────────────────────────────────────────────────────────────

    #[test]
    fn test_lower_defer() {
        let prog = compile(
            r#"act with_cleanup() void {
                defer println("cleanup")
                println("work")
            }"#,
        );
        let f = prog.get_fn("with_cleanup").expect("fn");
        assert!(!f.blocks.is_empty());
    }

    // ── Type alias ────────────────────────────────────────────────────────

    #[test]
    fn test_lower_type_alias_fn() {
        let prog = compile(
            r#"alias Score = int
               act top_score() Score { 100 }"#,
        );
        // No struct/enum for type aliases — just the function
        let f = prog.get_fn("top_score").expect("fn");
        assert!(!f.blocks.is_empty());
    }
}
