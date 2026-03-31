//! Razen Prelude — Built-in Names Injected into Every Module.
//!
//! Before name resolution runs, `inject_prelude` populates the module scope
//! with all built-in types, functions, and constructors so that user code can
//! reference them without explicit `use` statements.
//!
//! Every name registered here gets:
//!   • A `DefId` in the `SymbolTable`
//!   • A `Ty` entry in `type_env` (the type of the binding)
//!   • A binding in the current `Environment` scope

use std::collections::HashMap;

use razen_ast::ident::Ident;
use razen_lexer::Span;

use crate::scope::Environment;
use crate::symbol::{DefId, SymbolKind, SymbolTable};
use crate::ty::Ty;

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Inject all Razen prelude names into the provided module scope.
///
/// Call this ONCE before the name resolver runs so that built-in identifiers
/// (e.g. `println`, `some`, `ok`, `vec`, primitive type names) are visible to
/// user code.
pub fn inject_prelude(
    env: &mut Environment,
    table: &mut SymbolTable,
    type_env: &mut HashMap<DefId, Ty>,
) {
    let mut ctx = PreludeCtx {
        env,
        table,
        type_env,
    };

    // ── I/O ─────────────────────────────────────────────────────────────────
    ctx.reg_fn("println", vec![Ty::Param("T".into())], Ty::Void, false);
    ctx.reg_fn("print", vec![Ty::Param("T".into())], Ty::Void, false);
    ctx.reg_fn("eprintln", vec![Ty::Param("T".into())], Ty::Void, false);
    ctx.reg_fn("eprint", vec![Ty::Param("T".into())], Ty::Void, false);

    // ── Debug ────────────────────────────────────────────────────────────────
    ctx.reg_fn(
        "dbg",
        vec![Ty::Param("T".into())],
        Ty::Param("T".into()),
        false,
    );

    // ── Panic / assertions ───────────────────────────────────────────────────
    ctx.reg_fn("panic", vec![Ty::Str], Ty::Never, false);
    ctx.reg_fn("unreachable", vec![Ty::Param("T".into())], Ty::Never, false);
    ctx.reg_fn("assert", vec![Ty::Bool], Ty::Void, false);
    ctx.reg_fn(
        "assert_eq",
        vec![Ty::Param("T".into()), Ty::Param("T".into())],
        Ty::Void,
        false,
    );
    ctx.reg_fn(
        "assert_ne",
        vec![Ty::Param("T".into()), Ty::Param("T".into())],
        Ty::Void,
        false,
    );

    // ── Formatting ───────────────────────────────────────────────────────────
    ctx.reg_fn("format", vec![Ty::Param("T".into())], Ty::Str, false);

    // ── Input ────────────────────────────────────────────────────────────────
    ctx.reg_fn(
        "read_line",
        vec![],
        Ty::Result(Box::new(Ty::Str), Box::new(Ty::Str)),
        false,
    );

    // ── String building ──────────────────────────────────────────────────────
    // StringBuilder is a named opaque type for now; codegen will handle it.
    ctx.reg_fn(
        "string_builder",
        vec![],
        Ty::Named {
            def_id: DefId(usize::MAX - 1),
            name: "StringBuilder".into(),
            generics: vec![],
        },
        false,
    );
    ctx.reg_fn(
        "string_builder_with_capacity",
        vec![Ty::Uint],
        Ty::Named {
            def_id: DefId(usize::MAX - 1),
            name: "StringBuilder".into(),
            generics: vec![],
        },
        false,
    );

    // ── Option constructors ──────────────────────────────────────────────────
    ctx.reg_fn(
        "some",
        vec![Ty::Param("T".into())],
        Ty::Option(Box::new(Ty::Param("T".into()))),
        false,
    );
    // `none` is a bare value (not a function call) — `ret none`, `opt = none`.
    // Register it as a variable with type `option[T]` so the type checker
    // finds it as a concrete option value, not as a function pointer.
    ctx.reg_var("none", Ty::Option(Box::new(Ty::Param("T".into()))));

    // ── Result constructors ──────────────────────────────────────────────────
    ctx.reg_fn(
        "ok",
        vec![Ty::Param("T".into())],
        Ty::Result(
            Box::new(Ty::Param("T".into())),
            Box::new(Ty::Param("E".into())),
        ),
        false,
    );
    ctx.reg_fn(
        "err",
        vec![Ty::Param("E".into())],
        Ty::Result(
            Box::new(Ty::Param("T".into())),
            Box::new(Ty::Param("E".into())),
        ),
        false,
    );

    // ── Cloning ──────────────────────────────────────────────────────────────
    ctx.reg_fn(
        "clone",
        vec![Ty::Param("T".into())],
        Ty::Param("T".into()),
        false,
    );

    // ── JSON (commonly used in examples) ────────────────────────────────────
    ctx.reg_fn(
        "parse_json",
        vec![Ty::Str],
        Ty::Result(Box::new(Ty::Param("T".into())), Box::new(Ty::Str)),
        false,
    );

    // ── Type-conversion helpers ──────────────────────────────────────────────
    ctx.reg_fn("to_string", vec![Ty::Param("T".into())], Ty::Str, false);
    ctx.reg_fn("from_str", vec![Ty::Str], Ty::Param("T".into()), false);

    // ── Async built-ins ──────────────────────────────────────────────────────
    // `http` is registered as an opaque module-level name so `http.get(url)`
    // resolves without an "undefined" error.
    ctx.reg_var(
        "http",
        Ty::Named {
            def_id: DefId(usize::MAX - 2),
            name: "Http".into(),
            generics: vec![],
        },
    );

    // ── Primitive type name aliases ──────────────────────────────────────────
    // Registering type names lets the resolver find them when they appear as
    // identifiers (e.g. `println(int)` would be odd, but struct field types
    // referencing `int` do go through name resolution in some contexts).
    ctx.reg_type_alias("bool", Ty::Bool);
    ctx.reg_type_alias("int", Ty::Int);
    ctx.reg_type_alias("uint", Ty::Uint);
    ctx.reg_type_alias("float", Ty::Float);
    ctx.reg_type_alias("i8", Ty::I8);
    ctx.reg_type_alias("i16", Ty::I16);
    ctx.reg_type_alias("i32", Ty::I32);
    ctx.reg_type_alias("i64", Ty::I64);
    ctx.reg_type_alias("i128", Ty::I128);
    ctx.reg_type_alias("isize", Ty::Isize);
    ctx.reg_type_alias("u8", Ty::U8);
    ctx.reg_type_alias("u16", Ty::U16);
    ctx.reg_type_alias("u32", Ty::U32);
    ctx.reg_type_alias("u64", Ty::U64);
    ctx.reg_type_alias("u128", Ty::U128);
    ctx.reg_type_alias("usize", Ty::Usize);
    ctx.reg_type_alias("f32", Ty::F32);
    ctx.reg_type_alias("f64", Ty::F64);
    ctx.reg_type_alias("char", Ty::Char);
    ctx.reg_type_alias("str", Ty::Str);
    ctx.reg_type_alias("bytes", Ty::Bytes);
    ctx.reg_type_alias("void", Ty::Void);
    ctx.reg_type_alias("never", Ty::Never);
    ctx.reg_type_alias("tensor", Ty::Tensor);

    // ── Collection constructors / type names ─────────────────────────────────
    // `vec`, `map`, `set` appear as identifier-like constructors in source.
    ctx.reg_fn(
        "vec",
        vec![Ty::Param("T".into())],
        Ty::Vec(Box::new(Ty::Param("T".into()))),
        false,
    );
    ctx.reg_fn(
        "map",
        vec![],
        Ty::Map(
            Box::new(Ty::Param("K".into())),
            Box::new(Ty::Param("V".into())),
        ),
        false,
    );
    ctx.reg_fn(
        "set",
        vec![Ty::Param("T".into())],
        Ty::Set(Box::new(Ty::Param("T".into()))),
        false,
    );
    ctx.reg_fn("tensor", vec![Ty::Param("T".into())], Ty::Tensor, false);

    // ── Ordering enum (used by Ord trait) ────────────────────────────────────
    ctx.reg_type_alias(
        "Ordering",
        Ty::Named {
            def_id: DefId(usize::MAX - 3),
            name: "Ordering".into(),
            generics: vec![],
        },
    );
}

// ---------------------------------------------------------------------------
// Internal helper — bundles the three mutable references together so methods
// don't need to repeat all three parameters each time.
// ---------------------------------------------------------------------------

struct PreludeCtx<'a> {
    env: &'a mut Environment,
    table: &'a mut SymbolTable,
    type_env: &'a mut HashMap<DefId, Ty>,
}

impl<'a> PreludeCtx<'a> {
    /// Register a built-in function with the given parameter and return types.
    fn reg_fn(&mut self, name: &str, params: Vec<Ty>, ret: Ty, is_async: bool) {
        let ty = Ty::Fn {
            params,
            ret: Box::new(ret),
            is_async,
        };
        let id = self.add(name, SymbolKind::Function, ty);
        let _ = id;
    }

    /// Register a built-in variable / constant value (not a function).
    fn reg_var(&mut self, name: &str, ty: Ty) {
        self.add(name, SymbolKind::Variable { is_mut: false }, ty);
    }

    /// Register a primitive or built-in type alias.
    /// We use `SymbolKind::TypeAlias` so the resolver treats it as a type.
    fn reg_type_alias(&mut self, name: &str, ty: Ty) {
        self.add(name, SymbolKind::TypeAlias, ty);
    }

    /// Core registration: add a name to the symbol table and current scope.
    fn add(&mut self, name: &str, kind: SymbolKind, ty: Ty) -> DefId {
        let ident = Ident::new(name, Span::default());
        let id = self.table.add(&ident, kind);
        self.env.define(name.to_string(), id);
        self.type_env.insert(id, ty);
        id
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scope::{Environment, ScopeKind};
    use crate::symbol::SymbolTable;

    fn make_prelude() -> (Environment, SymbolTable, HashMap<DefId, Ty>) {
        let mut env = Environment::new();
        let mut table = SymbolTable::new();
        let mut type_env = HashMap::new();
        env.enter_scope(ScopeKind::Module);
        inject_prelude(&mut env, &mut table, &mut type_env);
        (env, table, type_env)
    }

    #[test]
    fn test_println_is_in_scope() {
        let (env, _table, type_env) = make_prelude();
        let id = env.resolve("println");
        assert!(id.is_some(), "println should be in scope");
        let ty = type_env.get(&id.unwrap());
        assert!(matches!(ty, Some(Ty::Fn { .. })));
    }

    #[test]
    fn test_some_is_in_scope() {
        let (env, _table, type_env) = make_prelude();
        let id = env.resolve("some").expect("some should be in scope");
        let ty = type_env.get(&id).expect("some should have a type");
        assert!(matches!(ty, Ty::Fn { .. }));
    }

    #[test]
    fn test_ok_is_in_scope() {
        let (env, _table, _type_env) = make_prelude();
        assert!(env.resolve("ok").is_some(), "ok should be in scope");
    }

    #[test]
    fn test_err_is_in_scope() {
        let (env, _table, _type_env) = make_prelude();
        assert!(env.resolve("err").is_some(), "err should be in scope");
    }

    #[test]
    fn test_none_is_in_scope() {
        let (env, _table, _type_env) = make_prelude();
        assert!(env.resolve("none").is_some(), "none should be in scope");
    }

    #[test]
    fn test_panic_returns_never() {
        let (env, _table, type_env) = make_prelude();
        let id = env.resolve("panic").expect("panic should be in scope");
        let ty = type_env.get(&id).expect("panic should have a type");
        if let Ty::Fn { ret, .. } = ty {
            assert_eq!(**ret, Ty::Never);
        } else {
            panic!("panic should be Ty::Fn");
        }
    }

    #[test]
    fn test_assert_in_scope() {
        let (env, _table, _type_env) = make_prelude();
        assert!(env.resolve("assert").is_some());
        assert!(env.resolve("assert_eq").is_some());
        assert!(env.resolve("assert_ne").is_some());
    }

    #[test]
    fn test_primitive_type_names_in_scope() {
        let (env, _table, type_env) = make_prelude();
        for name in &[
            "bool", "int", "uint", "float", "str", "char", "i32", "u64", "f32", "f64",
        ] {
            let id = env
                .resolve(name)
                .unwrap_or_else(|| panic!("{name} should be in scope"));
            assert!(
                type_env.contains_key(&id),
                "{name} should have a type entry"
            );
        }
    }

    #[test]
    fn test_read_line_returns_result() {
        let (env, _table, type_env) = make_prelude();
        let id = env.resolve("read_line").expect("read_line in scope");
        let ty = type_env.get(&id).expect("read_line has type");
        if let Ty::Fn { ret, .. } = ty {
            assert!(matches!(**ret, Ty::Result(_, _)));
        } else {
            panic!("read_line should be Ty::Fn");
        }
    }

    #[test]
    fn test_dbg_returns_same_type() {
        let (env, _table, type_env) = make_prelude();
        let id = env.resolve("dbg").expect("dbg in scope");
        let ty = type_env.get(&id).expect("dbg has type");
        if let Ty::Fn { ret, .. } = ty {
            assert!(matches!(**ret, Ty::Param(_)));
        } else {
            panic!("dbg should be Ty::Fn");
        }
    }
}
