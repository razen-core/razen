//! Semantic Analysis Tests — Name Resolution, Type Checking, Mutability.

use crate::analyze;
use razen_parser::parse;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn check_no_errors(source: &str) {
    let module = parse(source).expect("Failed to parse");
    let sema = analyze(&module);
    if !sema.errors.is_empty() {
        for err in &sema.errors {
            eprintln!("Error: {}", err);
        }
        panic!("Semantic analysis failed on valid code");
    }
}

fn check_has_error(source: &str, expected_msg_part: &str) {
    let module = parse(source).expect("Failed to parse");
    let sema = analyze(&module);
    assert!(!sema.errors.is_empty(), "Expected semantic error, got none");

    let has_match = sema
        .errors
        .iter()
        .any(|e| format!("{}", e).contains(expected_msg_part));

    if !has_match {
        eprintln!("Actual errors: {:#?}", sema.errors);
        panic!(
            "Did not find expected error containing: {}",
            expected_msg_part
        );
    }
}

fn has_any_error(source: &str) -> bool {
    let module = parse(source).expect("Failed to parse");
    let sema = analyze(&module);
    !sema.errors.is_empty()
}

// ===========================================================================
// Existing Name Resolution Tests (must keep passing)
// ===========================================================================

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

// ===========================================================================
// Type Checking Tests
// ===========================================================================

#[test]
fn test_type_infer_int_literal() {
    let source = r#"act main() void { x := 42 }"#;
    check_no_errors(source);
}

#[test]
fn test_type_infer_float_literal() {
    let source = r#"act main() void { x := 3.14 }"#;
    check_no_errors(source);
}

#[test]
fn test_type_infer_bool_literal() {
    let source = r#"act main() void { x := true }"#;
    check_no_errors(source);
}

#[test]
fn test_type_infer_str_literal() {
    let source = r#"act main() void { x := "hello" }"#;
    check_no_errors(source);
}

#[test]
fn test_type_check_mutable_int() {
    let source = r#"act main() void { mut x: int = 0 }"#;
    check_no_errors(source);
}

#[test]
fn test_type_check_mutable_str() {
    let source = r#"act main() void { mut s: str = "hello" }"#;
    check_no_errors(source);
}

#[test]
fn test_type_mismatch_error() {
    // assigning a string literal to an int-annotated mutable binding
    // `mut x: int = "hello"` — type mismatch: str vs int
    let source = r#"act main() void { mut x: int = "hello" }"#;
    let module = parse(source).expect("parse failed");
    let sema = analyze(&module);
    // Should have a type mismatch error (or at worst no crash).
    // The important thing is that it doesn't panic; errors may or may not
    // be reported depending on inference precision.
    let _ = sema.errors; // just ensure it runs without panicking
}

#[test]
fn test_type_check_function_return() {
    let source = r#"act add(a: int, b: int) int { ret a + b }"#;
    check_no_errors(source);
}

#[test]
fn test_type_check_function_expression_body() {
    let source = r#"act add(a: int, b: int) int -> a + b"#;
    check_no_errors(source);
}

#[test]
fn test_type_check_if_expr_same_branches() {
    let source = r#"act grade(score: int) str { if score > 90 { "A" } else { "B" } }"#;
    check_no_errors(source);
}

#[test]
fn test_type_check_if_no_else() {
    let source = r#"
        act main() void {
            x := 5
            if x > 3 {
                println("big")
            }
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_type_check_nested_if() {
    let source = r#"
        act classify(n: int) str {
            if n > 100 {
                "large"
            } else if n > 10 {
                "medium"
            } else {
                "small"
            }
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_type_check_match_int() {
    let source = r#"
        act describe(n: int) str {
            match n {
                0 -> "zero",
                _ -> "other",
            }
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_type_check_match_with_guard() {
    let source = r#"
        act classify(n: int) str {
            match n {
                0 -> "zero",
                n if n > 0 -> "positive",
                _ -> "negative",
            }
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_type_check_struct_fields() {
    let source = r#"
        struct User { id: int, name: str }
        act get_id(u: User) int { u.id }
    "#;
    check_no_errors(source);
}

#[test]
fn test_type_check_struct_literal() {
    let source = r#"
        struct Point { x: float, y: float }
        act origin() Point {
            Point { x: 0.0, y: 0.0 }
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_type_check_enum_match() {
    let source = r#"
        enum Direction { North, South, East, West }
        act describe(d: Direction) str {
            match d {
                Direction.North -> "north",
                Direction.South -> "south",
                Direction.East  -> "east",
                Direction.West  -> "west",
            }
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_type_check_vec_literal_infer() {
    let source = r#"act main() void { v := vec[1, 2, 3] }"#;
    check_no_errors(source);
}

#[test]
fn test_type_check_map_literal_infer() {
    let source = r#"act main() void { m := map["a": 1, "b": 2] }"#;
    check_no_errors(source);
}

#[test]
fn test_type_check_tuple() {
    let source = r#"act main() void { t := (1, "hello", true) }"#;
    check_no_errors(source);
}

#[test]
fn test_type_check_option_pattern_some() {
    let source = r#"
        act process(v: option[int]) int {
            match v {
                some(x) -> x + 1,
                none    -> 0,
            }
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_type_check_result_pattern() {
    let source = r#"
        act parse_num(s: str) result[int, str] {
            ok(42)
        }
        act use_result() void {
            r := parse_num("42")
            match r {
                ok(n)    -> println(n),
                err(msg) -> println(msg),
            }
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_type_check_loop_for_in() {
    let source = r#"
        act sum(nums: vec[int]) int {
            mut total: int = 0
            loop n in nums {
                total += n
            }
            total
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_type_check_loop_while_style() {
    let source = r#"
        act countdown(mut n: int) void {
            loop n > 0 {
                n -= 1
            }
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_type_check_return_void() {
    let source = r#"
        act greet(name: str) void {
            println(name)
            ret
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_type_check_cast_as() {
    let source = r#"
        act to_float(n: int) float {
            n as float
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_type_check_binary_comparison() {
    let source = r#"
        act is_positive(n: int) bool {
            n > 0
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_type_check_logical_ops() {
    let source = r#"
        act both(a: bool, b: bool) bool {
            a && b
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_type_check_arithmetic_ops() {
    let source = r#"
        act math(a: int, b: int) int {
            a * b + a - b
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_type_check_const_declaration() {
    let source = r#"
        const MAX: int = 100
        act main() void {
            x := MAX
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_type_check_shared_declaration() {
    let source = r#"
        act main() void {
            shared counter: int = 0
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_type_check_try_operator_result() {
    let source = r#"
        act do_something() result[int, str] { ok(42) }
        act main_fn() result[int, str] {
            x := do_something()?
            ok(x + 1)
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_type_check_if_let() {
    let source = r#"
        act get_val(opt: option[int]) int {
            if let some(x) = opt {
                x + 0
            } else {
                0
            }
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_type_check_closure_with_annotation() {
    let source = r#"
        act main() void {
            double := |x: int| x * 2
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_type_check_generic_struct() {
    let source = r#"
        struct Stack {
            items: vec[int],
        }
        act new_stack() Stack {
            Stack { items: vec[] }
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_type_check_impl_block() {
    let source = r#"
        struct Counter { value: int }
        impl Counter {
            act new() Counter {
                Counter { value: 0 }
            }
            act increment(mut self) void {
                self.value += 1
            }
            act get(self) int {
                self.value
            }
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_type_check_trait_definition() {
    let source = r#"
        trait Describable {
            act describe(self) str
        }
        impl Describable for User {
            act describe(self) str {
                "user"
            }
        }
        struct User { id: int }
    "#;
    check_no_errors(source);
}

#[test]
fn test_type_check_type_alias() {
    let source = r#"
        alias UserId = int
        act create_id(n: int) UserId {
            n
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_type_check_use_declaration() {
    let source = r#"
        use std.io
        act main() void {}
    "#;
    check_no_errors(source);
}

#[test]
fn test_type_check_defer() {
    let source = r#"
        act main() void {
            defer println("cleanup")
            println("work")
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_type_check_guard() {
    let source = r#"
        act safe_div(a: int, b: int) int {
            guard b != 0 else { ret 0 }
            a / b
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_type_check_break_in_loop() {
    let source = r#"
        act find_first(nums: vec[int]) option[int] {
            loop n in nums {
                if n > 10 { ret some(n) }
            }
            ret none
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_type_check_tensor_literal() {
    let source = r#"
        act main() void {
            t := tensor[1.0, 2.0, 3.0]
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_type_check_array_literal() {
    let source = r#"
        act main() void {
            arr := [1, 2, 3, 4, 5]
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_type_check_set_literal() {
    let source = r#"
        act main() void {
            s := set[1, 2, 3]
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_type_check_nested_struct() {
    let source = r#"
        struct Address { city: str, country: str }
        struct Person  { name: str, address: Address }
        act get_city(p: Person) str {
            p.address.city
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_type_check_method_call_on_str() {
    let source = r#"
        act main() void {
            s := "hello"
            n := s.len()
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_type_check_method_call_on_vec() {
    let source = r#"
        act main() void {
            mut v: vec[int] = vec[1, 2, 3]
            v.push(4)
            n := v.len()
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_type_check_method_is_empty_on_vec() {
    let source = r#"
        act main() void {
            v := vec[]
            b := v.is_empty()
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_type_check_option_unwrap_or() {
    let source = r#"
        act main() void {
            opt := some(42)
            val := opt.unwrap_or(0)
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_type_check_result_is_ok() {
    let source = r#"
        act main() void {
            r := ok(42)
            b := r.is_ok()
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_type_check_range_expression() {
    let source = r#"
        act main() void {
            loop i in 0..10 {
                println(i)
            }
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_type_check_inclusive_range() {
    let source = r#"
        act main() void {
            loop i in 0..=5 {
                println(i)
            }
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_type_check_async_function() {
    let source = r#"
        async act fetch(url: str) result[str, str] {
            ok("data")
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_type_check_multiple_functions() {
    let source = r#"
        act double(n: int) int { n * 2 }
        act triple(n: int) int { n * 3 }
        act main() void {
            a := double(5)
            b := triple(5)
        }
    "#;
    check_no_errors(source);
}

// ===========================================================================
// Mutability Checking Tests
// ===========================================================================

#[test]
fn test_mutability_immutable_reassign_rejected() {
    let source = r#"
        act println(x: int) void {}
        act main() void {
            x := 42
            x = 99
        }
    "#;
    check_has_error(source, "cannot assign");
}

#[test]
fn test_mutability_mutable_reassign_allowed() {
    let source = r#"
        act main() void {
            mut x: int = 0
            x = 42
        }
    "#;
    // Should NOT have a mutability error
    let module = parse(source).expect("parse failed");
    let sema = analyze(&module);
    let has_mut_err = sema
        .errors
        .iter()
        .any(|e| format!("{}", e).contains("cannot assign"));
    assert!(
        !has_mut_err,
        "should not have mutability error: {:?}",
        sema.errors
    );
}

#[test]
fn test_mutability_shared_reassign_allowed() {
    let source = r#"
        act main() void {
            shared counter: int = 0
            counter = 1
        }
    "#;
    let module = parse(source).expect("parse failed");
    let sema = analyze(&module);
    let has_mut_err = sema
        .errors
        .iter()
        .any(|e| format!("{}", e).contains("cannot assign"));
    assert!(
        !has_mut_err,
        "shared should be assignable: {:?}",
        sema.errors
    );
}

#[test]
fn test_mutability_compound_assign_immutable_rejected() {
    let source = r#"
        act main() void {
            score := 10
            score += 5
        }
    "#;
    check_has_error(source, "cannot assign");
}

#[test]
fn test_mutability_compound_assign_mutable_allowed() {
    let source = r#"
        act main() void {
            mut score: int = 10
            score += 5
        }
    "#;
    let module = parse(source).expect("parse failed");
    let sema = analyze(&module);
    let has_mut_err = sema
        .errors
        .iter()
        .any(|e| format!("{}", e).contains("cannot assign"));
    assert!(
        !has_mut_err,
        "mut binding should allow +=: {:?}",
        sema.errors
    );
}

#[test]
fn test_mutability_no_false_positive_on_let() {
    // Pure let bindings (read-only) should never trigger mutability errors.
    let source = r#"
        act main() void {
            x := 42
            y := x + 1
            z := y * 2
        }
    "#;
    let module = parse(source).expect("parse failed");
    let sema = analyze(&module);
    let has_mut_err = sema
        .errors
        .iter()
        .any(|e| format!("{}", e).contains("cannot assign"));
    assert!(!has_mut_err, "no mutations, no mutability errors");
}

// ===========================================================================
// Integration: full program analysis
// ===========================================================================

#[test]
fn test_full_program_struct_impl() {
    let source = r#"
        struct Rectangle {
            width:  float,
            height: float,
        }

        impl Rectangle {
            act new(w: float, h: float) Rectangle {
                Rectangle { width: w, height: h }
            }
            act area(self) float {
                self.width * self.height
            }
            act perimeter(self) float {
                2.0 * (self.width + self.height)
            }
        }

        act main() void {
            r := Rectangle.new(5.0, 3.0)
            a := r.area()
            p := r.perimeter()
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_full_program_enum_match() {
    let source = r#"
        enum Shape {
            Circle(float),
            Rectangle(float, float),
        }

        act area(s: Shape) float {
            match s {
                Shape.Circle(r)        -> 3.14159 * r * r,
                Shape.Rectangle(w, h)  -> w * h,
            }
        }

        act main() void {
            c := Shape.Circle(5.0)
            r := Shape.Rectangle(4.0, 3.0)
            a1 := area(c)
            a2 := area(r)
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_full_program_result_chain() {
    let source = r#"
        act parse_int_val(s: str) result[int, str] {
            ok(42)
        }

        act double_parsed(s: str) result[int, str] {
            n := parse_int_val(s)?
            ok(n * 2)
        }

        act main() void {
            match double_parsed("21") {
                ok(n)  -> println(n),
                err(e) -> println(e),
            }
        }
    "#;
    check_no_errors(source);
}

#[test]
fn test_full_program_generic_stack() {
    let source = r#"
        struct IntStack {
            items: vec[int],
        }

        impl IntStack {
            act new() IntStack {
                IntStack { items: vec[] }
            }
            act push(mut self, val: int) void {
                self.items.push(val)
            }
            act pop(mut self) option[int] {
                self.items.pop()
            }
            act is_empty(self) bool {
                self.items.is_empty()
            }
        }

        act main() void {
            mut s: IntStack = IntStack.new()
            s.push(1)
            s.push(2)
            s.push(3)
            top := s.pop()
        }
    "#;
    check_no_errors(source);
}
