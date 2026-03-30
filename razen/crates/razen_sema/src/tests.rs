//! Semantic Analysis Tests

use crate::{analyze, error::SemanticError};
use razen_parser::parse;

fn check_no_errors(source: &str) {
    let module = parse(source).expect("Failed to parse");
    let semantics = analyze(&module);
    if !semantics.errors.is_empty() {
        for err in &semantics.errors {
            eprintln!("Error: {}", err);
        }
        panic!("Semantic analysis failed on valid code");
    }
}

fn check_has_error(source: &str, expected_msg_part: &str) {
    let module = parse(source).expect("Failed to parse");
    let semantics = analyze(&module);
    assert!(!semantics.errors.is_empty(), "Expected semantic error, got none");
    
    let has_match = semantics.errors.iter().any(|e| {
        format!("{}", e).contains(expected_msg_part)
    });

    if !has_match {
        eprintln!("Actual errors: {:#?}", semantics.errors);
        panic!("Did not find expected error containing: {}", expected_msg_part);
    }
}

#[test]
fn test_resolve_local_variable() {
    let source = r#"
        act println(x: int) void {}
        act main() void {
            x := 42
            println(x) // x is resolved
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_undefined_variable() {
    let source = r#"
        act println(x: int) void {}
        act main() void {
            println(unknown_var)
        }
    "#;
    check_has_error(source, "Cannot find value, function, or type `unknown_var`");
}

#[test]
fn test_shadowing_same_scope() {
    let source = r#"
        act println(x: str) void {}
        act main() void {
            x := 10
            x := "shadowed" 
            println(x)
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_shadowing_nested_scope() {
    let source = r#"
        act println(x: int) void {}
        act main() void {
            x := 10
            if true {
                x := 20
                println(x)
            }
            println(x) // outer x
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_function_parameters_in_scope() {
    let source = r#"
        act add(a: int, b: int) int {
            ret a + b
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_struct_fields_and_scope() {
    let source = r#"
        struct User {
            id: int,
            name: str,
        }
        act create(id: int, name: str) User {
            ret User { id: id, name: name }
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_closure_parameters() {
    let source = r#"
        act test() void {
            f := |x, y| {
                ret x + y
            }
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_closure_captures_outer_scope() {
    let source = r#"
        act test() void {
            outer := 100
            f := |x| x + outer
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_match_arm_bindings() {
    let source = r#"
        act process(v: option[int]) int {
            match v {
                some(x) -> x + 1,
                none -> 0,
            }
        }
    "#;
    check_no_errors(source);
}
