//! Parser tests.
//!
//! These tests verify that the parser correctly transforms token streams
//! into AST nodes for all major Razen language constructs.

use crate::parse;
use razen_ast::expr::*;
use razen_ast::item::*;
use razen_ast::lit::Literal;
use razen_ast::ops::*;
use razen_ast::pat::Pattern;
use razen_ast::stmt::Stmt;

/// Helper: parse source and return the module, panicking on error.
fn parse_ok(source: &str) -> razen_ast::Module {
    match parse(source) {
        Ok(m) => m,
        Err(errors) => {
            for e in &errors {
                eprintln!("  ERROR: {}", e);
            }
            panic!("parse failed with {} error(s)", errors.len());
        }
    }
}

/// Helper: parse source and assert it produces errors.
fn parse_err(source: &str) -> Vec<crate::ParseError> {
    match parse(source) {
        Ok(_) => panic!("expected parse error, but succeeded"),
        Err(errors) => errors,
    }
}

// ============================================================================
// Test 1: Immutable variable declaration (`:=`)
// ============================================================================
#[test]
fn test_immutable_variable_declaration() {
    let module = parse_ok("act main() void { x := 42 }");
    assert_eq!(module.items.len(), 1);

    if let Item::Function(f) = &module.items[0] {
        assert_eq!(f.name.name, "main");
        if let FnBody::Block { stmts, tail, .. } = &f.body {
            // The `x := 42` may be parsed as tail or as a statement
            let total = stmts.len() + if tail.is_some() { 1 } else { 0 };
            assert!(total >= 1, "expected at least 1 statement/tail in block");
        } else {
            panic!("expected block body");
        }
    } else {
        panic!("expected function");
    }
}

// ============================================================================
// Test 2: Mutable variable declaration (`mut`)
// ============================================================================
#[test]
fn test_mutable_variable_declaration() {
    let module = parse_ok("act test() void { mut score: int = 0 }");
    assert_eq!(module.items.len(), 1);

    if let Item::Function(f) = &module.items[0] {
        if let FnBody::Block { stmts, tail, .. } = &f.body {
            let total = stmts.len() + if tail.is_some() { 1 } else { 0 };
            assert!(total >= 1);
            // Find the LetMut statement
            let has_let_mut = stmts.iter().any(|s| matches!(s, Stmt::LetMut { .. }));
            assert!(has_let_mut, "expected a LetMut statement");
        } else {
            panic!("expected block body");
        }
    } else {
        panic!("expected function");
    }
}

// ============================================================================
// Test 3: Constant declaration
// ============================================================================
#[test]
fn test_const_declaration() {
    let module = parse_ok("const MAX_SCORE: int = 100");
    assert_eq!(module.items.len(), 1);

    if let Item::Const(c) = &module.items[0] {
        assert_eq!(c.name.name, "MAX_SCORE");
    } else {
        panic!("expected const item, got {:?}", module.items[0]);
    }
}

// ============================================================================
// Test 4: Function definition with parameters and return type
// ============================================================================
#[test]
fn test_function_definition() {
    let module = parse_ok("act add(a: int, b: int) int -> a + b");
    assert_eq!(module.items.len(), 1);

    if let Item::Function(f) = &module.items[0] {
        assert_eq!(f.name.name, "add");
        assert_eq!(f.params.len(), 2);
        assert!(f.return_type.is_some());
        assert!(matches!(f.body, FnBody::Expr(_)));
    } else {
        panic!("expected function");
    }
}

// ============================================================================
// Test 5: Struct definition with fields
// ============================================================================
#[test]
fn test_struct_definition() {
    let source = r#"
        struct User {
            id: int,
            name: str,
            is_active: bool,
        }
    "#;
    let module = parse_ok(source);
    assert_eq!(module.items.len(), 1);

    if let Item::Struct(s) = &module.items[0] {
        assert_eq!(s.name.name, "User");
        if let StructKind::Named { fields } = &s.kind {
            assert_eq!(fields.len(), 3);
            assert_eq!(fields[0].name.name, "id");
            assert_eq!(fields[1].name.name, "name");
            assert_eq!(fields[2].name.name, "is_active");
        } else {
            panic!("expected named struct");
        }
    } else {
        panic!("expected struct");
    }
}

// ============================================================================
// Test 6: Enum with mixed variants
// ============================================================================
#[test]
fn test_enum_definition() {
    let source = r#"
        enum Shape {
            Circle(float),
            Rectangle(float, float),
            Point,
        }
    "#;
    let module = parse_ok(source);
    assert_eq!(module.items.len(), 1);

    if let Item::Enum(e) = &module.items[0] {
        assert_eq!(e.name.name, "Shape");
        assert_eq!(e.variants.len(), 3);
        assert_eq!(e.variants[0].name.name, "Circle");
        assert!(matches!(e.variants[0].kind, EnumVariantKind::Positional { .. }));
        assert_eq!(e.variants[1].name.name, "Rectangle");
        assert_eq!(e.variants[2].name.name, "Point");
        assert!(matches!(e.variants[2].kind, EnumVariantKind::Unit));
    } else {
        panic!("expected enum");
    }
}

// ============================================================================
// Test 7: If/else expression
// ============================================================================
#[test]
fn test_if_else_expression() {
    let source = r#"
        act grade(score: int) str {
            if score > 90 {
                "A"
            } else {
                "B"
            }
        }
    "#;
    let module = parse_ok(source);
    assert_eq!(module.items.len(), 1);

    if let Item::Function(f) = &module.items[0] {
        assert_eq!(f.name.name, "grade");
        if let FnBody::Block { tail, .. } = &f.body {
            assert!(tail.is_some(), "expected tail expression (if/else)");
            if let Some(tail_expr) = tail {
                assert!(
                    matches!(tail_expr.as_ref(), Expr::If { else_block: Some(_), .. }),
                    "expected if/else expression as tail"
                );
            }
        } else {
            panic!("expected block body");
        }
    } else {
        panic!("expected function");
    }
}

// ============================================================================
// Test 8: Match expression with guards
// ============================================================================
#[test]
fn test_match_expression() {
    let source = r#"
        act classify(n: int) str {
            match n {
                0 -> "zero",
                n if n > 0 -> "positive",
                _ -> "negative",
            }
        }
    "#;
    let module = parse_ok(source);
    assert_eq!(module.items.len(), 1);

    if let Item::Function(f) = &module.items[0] {
        if let FnBody::Block { tail, .. } = &f.body {
            assert!(tail.is_some());
            if let Some(tail_expr) = tail {
                if let Expr::Match { arms, .. } = tail_expr.as_ref() {
                    assert_eq!(arms.len(), 3, "expected 3 match arms");
                    // Second arm has a guard
                    assert!(arms[1].guard.is_some(), "expected guard on second arm");
                    // Third arm is wildcard
                    assert!(
                        matches!(arms[2].pattern, Pattern::Wildcard { .. }),
                        "expected wildcard pattern"
                    );
                } else {
                    panic!("expected match expression");
                }
            }
        } else {
            panic!("expected block body");
        }
    } else {
        panic!("expected function");
    }
}

// ============================================================================
// Test 9: Loop with range
// ============================================================================
#[test]
fn test_loop_with_range() {
    let source = r#"
        act count() void {
            loop i in 0..10 {
                println(i)
            }
        }
    "#;
    let module = parse_ok(source);
    assert_eq!(module.items.len(), 1);

    if let Item::Function(f) = &module.items[0] {
        if let FnBody::Block { stmts, tail, .. } = &f.body {
            let total = stmts.len() + if tail.is_some() { 1 } else { 0 };
            assert!(total >= 1, "expected loop statement");
        } else {
            panic!("expected block body");
        }
    } else {
        panic!("expected function");
    }
}

// ============================================================================
// Test 10: Complex expression with operator precedence
// ============================================================================
#[test]
fn test_operator_precedence() {
    // 2 + 3 * 4 should parse as 2 + (3 * 4)
    let source = "act test() int -> 2 + 3 * 4";
    let module = parse_ok(source);

    if let Item::Function(f) = &module.items[0] {
        if let FnBody::Expr(body) = &f.body {
            // Should be Binary(Add, 2, Binary(Mul, 3, 4))
            if let Expr::Binary { op, left, right, .. } = body.as_ref() {
                assert_eq!(*op, BinOp::Add, "top-level should be Add");
                // Right side should be Mul
                if let Expr::Binary { op: inner_op, .. } = right.as_ref() {
                    assert_eq!(*inner_op, BinOp::Mul, "right side should be Mul");
                } else {
                    panic!("expected binary mul on right side");
                }
            } else {
                panic!("expected binary expression");
            }
        } else {
            panic!("expected expression body");
        }
    } else {
        panic!("expected function");
    }
}

// ============================================================================
// Test 11: Use/import statement
// ============================================================================
#[test]
fn test_use_declaration() {
    let source = "use std.io";
    let module = parse_ok(source);
    assert_eq!(module.items.len(), 1);

    if let Item::Use(u) = &module.items[0] {
        assert_eq!(u.path.len(), 2);
        assert_eq!(u.path[0].name, "std");
        assert_eq!(u.path[1].name, "io");
        assert!(matches!(u.kind, UseKind::Module));
    } else {
        panic!("expected use declaration");
    }
}

// ============================================================================
// Test 12: Use with specific items
// ============================================================================
#[test]
fn test_use_specific_items() {
    let source = "use models { User, Role }";
    let module = parse_ok(source);
    assert_eq!(module.items.len(), 1);

    if let Item::Use(u) = &module.items[0] {
        assert_eq!(u.path.len(), 1);
        assert_eq!(u.path[0].name, "models");
        if let UseKind::Items(items) = &u.kind {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].name.name, "User");
            assert_eq!(items[1].name.name, "Role");
        } else {
            panic!("expected use items");
        }
    } else {
        panic!("expected use declaration");
    }
}

// ============================================================================
// Test 13: Trait and impl blocks
// ============================================================================
#[test]
fn test_trait_and_impl() {
    let source = r#"
        trait Describable {
            act describe(self) str
        }

        impl Describable for User {
            act describe(self) str {
                "user"
            }
        }
    "#;
    let module = parse_ok(source);
    assert_eq!(module.items.len(), 2);

    if let Item::Trait(t) = &module.items[0] {
        assert_eq!(t.name.name, "Describable");
        assert_eq!(t.methods.len(), 1);
        assert_eq!(t.methods[0].name.name, "describe");
    } else {
        panic!("expected trait");
    }

    if let Item::Impl(i) = &module.items[1] {
        assert!(i.trait_name.is_some());
        assert_eq!(i.methods.len(), 1);
    } else {
        panic!("expected impl");
    }
}

// ============================================================================
// Test 14: Type alias
// ============================================================================
#[test]
fn test_type_alias() {
    let source = "alias UserId = int";
    let module = parse_ok(source);
    assert_eq!(module.items.len(), 1);

    if let Item::TypeAlias(a) = &module.items[0] {
        assert_eq!(a.name.name, "UserId");
    } else {
        panic!("expected type alias");
    }
}

// ============================================================================
// Test 15: Attributes
// ============================================================================
#[test]
fn test_attributes() {
    let source = r#"
        @derive[Debug, Clone]
        struct Point {
            x: float,
            y: float,
        }
    "#;
    let module = parse_ok(source);
    assert_eq!(module.items.len(), 1);

    if let Item::Struct(s) = &module.items[0] {
        assert_eq!(s.name.name, "Point");
        assert_eq!(s.attrs.len(), 1);
        assert_eq!(s.attrs[0].name.name, "derive");
        assert_eq!(s.attrs[0].args.len(), 2);
    } else {
        panic!("expected struct");
    }
}
