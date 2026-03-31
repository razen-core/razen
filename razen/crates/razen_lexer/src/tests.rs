//! Comprehensive Lexer Tests for Razen.
//!
//! Covers every TokenKind variant, all operators, all numeric literal forms,
//! string/char escapes, labels, comments, spans, and edge cases.
//!
//! Structure:
//!   - Keywords (all 32)
//!   - Operators & Compound Assignment
//!   - Punctuation
//!   - Numeric Literals (decimal, hex, binary, octal, suffixes, floats)
//!   - String Literals (escapes, interpolation syntax)
//!   - Char Literals (ASCII, escapes)
//!   - Identifiers (various forms)
//!   - Labels
//!   - Comments (line, block, doc)
//!   - Span correctness
//!   - Edge cases (empty, whitespace, error recovery, consecutive tokens)

use crate::{Lexer, Token, TokenKind, tokenize};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Lex a source string and return all token *kinds*, including Eof.
fn lex_kinds(src: &str) -> Vec<TokenKind> {
    tokenize(src).into_iter().map(|t| t.kind).collect()
}

/// Lex a source string and return all token *kinds* **excluding** Eof.
fn lex_no_eof(src: &str) -> Vec<TokenKind> {
    let mut v = lex_kinds(src);
    if v.last() == Some(&TokenKind::Eof) {
        v.pop();
    }
    v
}

/// Lex a source string and return all tokens including span, excluding Eof.
fn lex_tokens(src: &str) -> Vec<Token> {
    tokenize(src)
        .into_iter()
        .filter(|t| t.kind != TokenKind::Eof)
        .collect()
}

/// Assert that lexing `src` produces exactly `expected` kinds (no Eof in expected).
fn assert_kinds(src: &str, expected: &[TokenKind]) {
    let actual = lex_no_eof(src);
    assert_eq!(
        actual, expected,
        "\nsource: {:?}\nexpected: {:?}\nactual:   {:?}",
        src, expected, actual
    );
}

// ===========================================================================
// 1. KEYWORDS
// ===========================================================================

#[test]
fn test_kw_mut() {
    assert_kinds("mut", &[TokenKind::Mut]);
}

#[test]
fn test_kw_const() {
    assert_kinds("const", &[TokenKind::Const]);
}

#[test]
fn test_kw_shared() {
    assert_kinds("shared", &[TokenKind::Shared]);
}

#[test]
fn test_kw_struct() {
    assert_kinds("struct", &[TokenKind::Struct]);
}

#[test]
fn test_kw_enum() {
    assert_kinds("enum", &[TokenKind::Enum]);
}

#[test]
fn test_kw_trait() {
    assert_kinds("trait", &[TokenKind::Trait]);
}

#[test]
fn test_kw_impl() {
    assert_kinds("impl", &[TokenKind::Impl]);
}

#[test]
fn test_kw_alias() {
    assert_kinds("alias", &[TokenKind::Alias]);
}

#[test]
fn test_kw_act() {
    assert_kinds("act", &[TokenKind::Act]);
}

#[test]
fn test_kw_ret() {
    assert_kinds("ret", &[TokenKind::Ret]);
}

#[test]
fn test_kw_use() {
    assert_kinds("use", &[TokenKind::Use]);
}

#[test]
fn test_kw_pub() {
    assert_kinds("pub", &[TokenKind::Pub]);
}

#[test]
fn test_kw_if() {
    assert_kinds("if", &[TokenKind::If]);
}

#[test]
fn test_kw_else() {
    assert_kinds("else", &[TokenKind::Else]);
}

#[test]
fn test_kw_loop() {
    assert_kinds("loop", &[TokenKind::Loop]);
}

#[test]
fn test_kw_break() {
    assert_kinds("break", &[TokenKind::Break]);
}

#[test]
fn test_kw_next() {
    assert_kinds("next", &[TokenKind::Next]);
}

#[test]
fn test_kw_match() {
    assert_kinds("match", &[TokenKind::Match]);
}

#[test]
fn test_kw_guard() {
    assert_kinds("guard", &[TokenKind::Guard]);
}

#[test]
fn test_kw_in() {
    assert_kinds("in", &[TokenKind::In]);
}

#[test]
fn test_kw_as() {
    assert_kinds("as", &[TokenKind::As]);
}

#[test]
fn test_kw_is() {
    assert_kinds("is", &[TokenKind::Is]);
}

#[test]
fn test_kw_self_lowercase() {
    assert_kinds("self", &[TokenKind::SelfKw]);
}

#[test]
fn test_kw_self_type() {
    assert_kinds("Self", &[TokenKind::SelfType]);
}

#[test]
fn test_kw_defer() {
    assert_kinds("defer", &[TokenKind::Defer]);
}

#[test]
fn test_kw_async() {
    assert_kinds("async", &[TokenKind::Async]);
}

#[test]
fn test_kw_await() {
    assert_kinds("await", &[TokenKind::Await]);
}

#[test]
fn test_kw_fork() {
    assert_kinds("fork", &[TokenKind::Fork]);
}

#[test]
fn test_kw_unsafe() {
    assert_kinds("unsafe", &[TokenKind::Unsafe]);
}

#[test]
fn test_kw_where() {
    assert_kinds("where", &[TokenKind::Where]);
}

#[test]
fn test_kw_true() {
    assert_kinds("true", &[TokenKind::True]);
}

#[test]
fn test_kw_false() {
    assert_kinds("false", &[TokenKind::False]);
}

/// All keywords in a single source, verifying none bleed into identifiers.
#[test]
fn test_all_keywords_together() {
    let src = "mut const shared struct enum trait impl alias act ret use pub \
               if else loop break next match guard in as is self Self \
               defer async await fork unsafe where true false";
    let kinds = lex_no_eof(src);
    assert_eq!(
        kinds,
        vec![
            TokenKind::Mut,
            TokenKind::Const,
            TokenKind::Shared,
            TokenKind::Struct,
            TokenKind::Enum,
            TokenKind::Trait,
            TokenKind::Impl,
            TokenKind::Alias,
            TokenKind::Act,
            TokenKind::Ret,
            TokenKind::Use,
            TokenKind::Pub,
            TokenKind::If,
            TokenKind::Else,
            TokenKind::Loop,
            TokenKind::Break,
            TokenKind::Next,
            TokenKind::Match,
            TokenKind::Guard,
            TokenKind::In,
            TokenKind::As,
            TokenKind::Is,
            TokenKind::SelfKw,
            TokenKind::SelfType,
            TokenKind::Defer,
            TokenKind::Async,
            TokenKind::Await,
            TokenKind::Fork,
            TokenKind::Unsafe,
            TokenKind::Where,
            TokenKind::True,
            TokenKind::False,
        ]
    );
}

/// Identifiers that START with a keyword prefix are NOT keywords.
#[test]
fn test_keyword_prefix_is_ident() {
    assert_kinds("mutate", &[TokenKind::Ident("mutate".into())]);
    assert_kinds("truely", &[TokenKind::Ident("truely".into())]);
    assert_kinds("looping", &[TokenKind::Ident("looping".into())]);
    assert_kinds("matchbox", &[TokenKind::Ident("matchbox".into())]);
    assert_kinds("iffy", &[TokenKind::Ident("iffy".into())]);
    assert_kinds("retrace", &[TokenKind::Ident("retrace".into())]);
    assert_kinds("usage", &[TokenKind::Ident("usage".into())]);
    assert_kinds("constrain", &[TokenKind::Ident("constrain".into())]);
}

/// A keyword immediately followed by non-alphanumeric is still a keyword.
#[test]
fn test_keyword_followed_by_punctuation() {
    assert_kinds("mut:", &[TokenKind::Mut, TokenKind::Colon]);
    assert_kinds("if(", &[TokenKind::If, TokenKind::LParen]);
    assert_kinds("true,", &[TokenKind::True, TokenKind::Comma]);
    assert_kinds("ret;", &[TokenKind::Ret, TokenKind::Semi]);
}

// ===========================================================================
// 2. OPERATORS & SYMBOLS
// ===========================================================================

#[test]
fn test_op_colon_eq() {
    assert_kinds(":=", &[TokenKind::ColonEq]);
}

#[test]
fn test_op_eq() {
    assert_kinds("=", &[TokenKind::Eq]);
}

#[test]
fn test_op_colon() {
    assert_kinds(":", &[TokenKind::Colon]);
}

#[test]
fn test_op_colon_vs_colon_eq() {
    // `:` followed by `=` forms `:=`, not Colon then Eq
    assert_kinds(":=", &[TokenKind::ColonEq]);
    // `: =` (with space) is separate
    assert_kinds(": =", &[TokenKind::Colon, TokenKind::Eq]);
}

#[test]
fn test_op_plus_eq() {
    assert_kinds("+=", &[TokenKind::PlusEq]);
}

#[test]
fn test_op_minus_eq() {
    assert_kinds("-=", &[TokenKind::MinusEq]);
}

#[test]
fn test_op_star_eq() {
    assert_kinds("*=", &[TokenKind::StarEq]);
}

#[test]
fn test_op_slash_eq() {
    assert_kinds("/=", &[TokenKind::SlashEq]);
}

#[test]
fn test_op_percent_eq() {
    assert_kinds("%=", &[TokenKind::PercentEq]);
}

#[test]
fn test_op_star_star_eq() {
    assert_kinds("**=", &[TokenKind::StarStarEq]);
}

#[test]
fn test_op_star_star() {
    assert_kinds("**", &[TokenKind::StarStar]);
}

#[test]
fn test_op_star_star_vs_star() {
    // `**=` should be a single token, not `**` + `=`
    assert_kinds("**=", &[TokenKind::StarStarEq]);
    // `** =` (with space) is StarStar then Eq
    assert_kinds("** =", &[TokenKind::StarStar, TokenKind::Eq]);
}

#[test]
fn test_op_plus() {
    assert_kinds("+", &[TokenKind::Plus]);
}

#[test]
fn test_op_minus() {
    assert_kinds("-", &[TokenKind::Minus]);
}

#[test]
fn test_op_star() {
    assert_kinds("*", &[TokenKind::Star]);
}

#[test]
fn test_op_slash() {
    assert_kinds("/", &[TokenKind::Slash]);
}

#[test]
fn test_op_percent() {
    assert_kinds("%", &[TokenKind::Percent]);
}

#[test]
fn test_op_eq_eq() {
    assert_kinds("==", &[TokenKind::EqEq]);
}

#[test]
fn test_op_not_eq() {
    assert_kinds("!=", &[TokenKind::NotEq]);
}

#[test]
fn test_op_lt() {
    assert_kinds("<", &[TokenKind::Lt]);
}

#[test]
fn test_op_gt() {
    assert_kinds(">", &[TokenKind::Gt]);
}

#[test]
fn test_op_lt_eq() {
    assert_kinds("<=", &[TokenKind::LtEq]);
}

#[test]
fn test_op_gt_eq() {
    assert_kinds(">=", &[TokenKind::GtEq]);
}

#[test]
fn test_op_and_and() {
    assert_kinds("&&", &[TokenKind::AndAnd]);
}

#[test]
fn test_op_or_or() {
    assert_kinds("||", &[TokenKind::OrOr]);
}

#[test]
fn test_op_and() {
    assert_kinds("&", &[TokenKind::And]);
}

#[test]
fn test_op_or() {
    assert_kinds("|", &[TokenKind::Or]);
}

#[test]
fn test_op_caret() {
    assert_kinds("^", &[TokenKind::Caret]);
}

#[test]
fn test_op_tilde_standalone() {
    assert_kinds("~", &[TokenKind::Tilde]);
}

#[test]
fn test_op_shl() {
    assert_kinds("<<", &[TokenKind::Shl]);
}

#[test]
fn test_op_shr() {
    assert_kinds(">>", &[TokenKind::Shr]);
}

#[test]
fn test_op_bang() {
    assert_kinds("!", &[TokenKind::Bang]);
}

#[test]
fn test_op_dot() {
    assert_kinds(".", &[TokenKind::Dot]);
}

#[test]
fn test_op_arrow() {
    assert_kinds("->", &[TokenKind::Arrow]);
}

#[test]
fn test_op_async_pipe() {
    assert_kinds("~>", &[TokenKind::AsyncPipe]);
}

#[test]
fn test_op_question() {
    assert_kinds("?", &[TokenKind::Question]);
}

#[test]
fn test_op_dot_dot() {
    assert_kinds("..", &[TokenKind::DotDot]);
}

#[test]
fn test_op_dot_dot_eq() {
    assert_kinds("..=", &[TokenKind::DotDotEq]);
}

#[test]
fn test_op_underscore_standalone() {
    assert_kinds("_", &[TokenKind::Underscore]);
}

#[test]
fn test_op_at() {
    assert_kinds("@", &[TokenKind::At]);
}

/// Compound operator disambiguation: `<<` should be Shl, not two `<`.
#[test]
fn test_op_shl_vs_lt_lt() {
    assert_kinds("<<", &[TokenKind::Shl]);
    assert_kinds("< <", &[TokenKind::Lt, TokenKind::Lt]);
}

/// `>>` should be Shr, not two `>`.
#[test]
fn test_op_shr_vs_gt_gt() {
    assert_kinds(">>", &[TokenKind::Shr]);
    assert_kinds("> >", &[TokenKind::Gt, TokenKind::Gt]);
}

/// `&&` vs `& &`.
#[test]
fn test_op_and_and_vs_single_and() {
    assert_kinds("&&", &[TokenKind::AndAnd]);
    assert_kinds("& &", &[TokenKind::And, TokenKind::And]);
}

/// `||` vs `| |`.
#[test]
fn test_op_or_or_vs_single_or() {
    assert_kinds("||", &[TokenKind::OrOr]);
    assert_kinds("| |", &[TokenKind::Or, TokenKind::Or]);
}

/// `->`  vs `-` `>`.
#[test]
fn test_arrow_vs_minus_gt() {
    assert_kinds("->", &[TokenKind::Arrow]);
    assert_kinds("- >", &[TokenKind::Minus, TokenKind::Gt]);
}

/// `~>` vs `~` `>`.
#[test]
fn test_async_pipe_vs_tilde_gt() {
    assert_kinds("~>", &[TokenKind::AsyncPipe]);
    assert_kinds("~ >", &[TokenKind::Tilde, TokenKind::Gt]);
}

/// `==` vs `=` then `=`.
#[test]
fn test_eq_eq_vs_two_eq() {
    assert_kinds("==", &[TokenKind::EqEq]);
    assert_kinds("= =", &[TokenKind::Eq, TokenKind::Eq]);
}

/// All compound assignments in sequence.
#[test]
fn test_all_compound_assignments() {
    assert_kinds(
        "+= -= *= /= %= **=",
        &[
            TokenKind::PlusEq,
            TokenKind::MinusEq,
            TokenKind::StarEq,
            TokenKind::SlashEq,
            TokenKind::PercentEq,
            TokenKind::StarStarEq,
        ],
    );
}

/// Range operators.
#[test]
fn test_range_operators() {
    assert_kinds(
        "0..10",
        &[
            TokenKind::Int("0".into()),
            TokenKind::DotDot,
            TokenKind::Int("10".into()),
        ],
    );
    assert_kinds(
        "0..=10",
        &[
            TokenKind::Int("0".into()),
            TokenKind::DotDotEq,
            TokenKind::Int("10".into()),
        ],
    );
}

/// Dot-dot after a method call chain (not a range).
#[test]
fn test_dot_dot_struct_update() {
    // `..other` — two tokens
    assert_kinds(
        "..other",
        &[TokenKind::DotDot, TokenKind::Ident("other".into())],
    );
}

/// All comparison operators together.
#[test]
fn test_all_comparison_ops() {
    assert_kinds(
        "== != < > <= >=",
        &[
            TokenKind::EqEq,
            TokenKind::NotEq,
            TokenKind::Lt,
            TokenKind::Gt,
            TokenKind::LtEq,
            TokenKind::GtEq,
        ],
    );
}

/// All bitwise operators together.
#[test]
fn test_all_bitwise_ops() {
    assert_kinds(
        "& | ^ ~ << >>",
        &[
            TokenKind::And,
            TokenKind::Or,
            TokenKind::Caret,
            TokenKind::Tilde,
            TokenKind::Shl,
            TokenKind::Shr,
        ],
    );
}

// ===========================================================================
// 3. PUNCTUATION
// ===========================================================================

#[test]
fn test_punct_braces() {
    assert_kinds("{}", &[TokenKind::LBrace, TokenKind::RBrace]);
}

#[test]
fn test_punct_brackets() {
    assert_kinds("[]", &[TokenKind::LBracket, TokenKind::RBracket]);
}

#[test]
fn test_punct_parens() {
    assert_kinds("()", &[TokenKind::LParen, TokenKind::RParen]);
}

#[test]
fn test_punct_comma() {
    assert_kinds(",", &[TokenKind::Comma]);
}

#[test]
fn test_punct_semicolon() {
    assert_kinds(";", &[TokenKind::Semi]);
}

#[test]
fn test_punct_all_delimiters() {
    assert_kinds(
        "{}[]()",
        &[
            TokenKind::LBrace,
            TokenKind::RBrace,
            TokenKind::LBracket,
            TokenKind::RBracket,
            TokenKind::LParen,
            TokenKind::RParen,
        ],
    );
}

#[test]
fn test_punct_nested_delimiters() {
    assert_kinds(
        "([{  }])",
        &[
            TokenKind::LParen,
            TokenKind::LBracket,
            TokenKind::LBrace,
            TokenKind::RBrace,
            TokenKind::RBracket,
            TokenKind::RParen,
        ],
    );
}

#[test]
fn test_punct_comma_separated_list() {
    assert_kinds(
        "a, b, c",
        &[
            TokenKind::Ident("a".into()),
            TokenKind::Comma,
            TokenKind::Ident("b".into()),
            TokenKind::Comma,
            TokenKind::Ident("c".into()),
        ],
    );
}

#[test]
fn test_punct_at_ident() {
    assert_kinds(
        "@derive",
        &[TokenKind::At, TokenKind::Ident("derive".into())],
    );
}

#[test]
fn test_punct_at_with_brackets() {
    assert_kinds(
        "@test[arg]",
        &[
            TokenKind::At,
            TokenKind::Ident("test".into()),
            TokenKind::LBracket,
            TokenKind::Ident("arg".into()),
            TokenKind::RBracket,
        ],
    );
}

// ===========================================================================
// 4. NUMERIC LITERALS
// ===========================================================================

#[test]
fn test_int_decimal_zero() {
    assert_kinds("0", &[TokenKind::Int("0".into())]);
}

#[test]
fn test_int_decimal_positive() {
    assert_kinds("42", &[TokenKind::Int("42".into())]);
    assert_kinds("100", &[TokenKind::Int("100".into())]);
    assert_kinds("999999", &[TokenKind::Int("999999".into())]);
}

#[test]
fn test_int_decimal_large() {
    assert_kinds("1000000000", &[TokenKind::Int("1000000000".into())]);
}

#[test]
fn test_int_with_underscores() {
    assert_kinds("1_000_000", &[TokenKind::Int("1_000_000".into())]);
    assert_kinds("1_0", &[TokenKind::Int("1_0".into())]);
}

#[test]
fn test_int_hex_lowercase() {
    assert_kinds("0xff", &[TokenKind::Int("0xff".into())]);
    assert_kinds("0xdead", &[TokenKind::Int("0xdead".into())]);
}

#[test]
fn test_int_hex_uppercase() {
    assert_kinds("0xFF", &[TokenKind::Int("0xFF".into())]);
    assert_kinds("0xDEAD", &[TokenKind::Int("0xDEAD".into())]);
    assert_kinds("0xABCDEF", &[TokenKind::Int("0xABCDEF".into())]);
}

#[test]
fn test_int_hex_mixed_case() {
    assert_kinds("0xDEadBEef", &[TokenKind::Int("0xDEadBEef".into())]);
}

#[test]
fn test_int_binary() {
    assert_kinds("0b0", &[TokenKind::Int("0b0".into())]);
    assert_kinds("0b1", &[TokenKind::Int("0b1".into())]);
    assert_kinds("0b1010", &[TokenKind::Int("0b1010".into())]);
    assert_kinds("0b1111_0000", &[TokenKind::Int("0b1111_0000".into())]);
}

#[test]
fn test_int_octal() {
    assert_kinds("0o0", &[TokenKind::Int("0o0".into())]);
    assert_kinds("0o7", &[TokenKind::Int("0o7".into())]);
    assert_kinds("0o755", &[TokenKind::Int("0o755".into())]);
    assert_kinds("0o777", &[TokenKind::Int("0o777".into())]);
}

#[test]
fn test_int_with_type_suffix_u8() {
    assert_kinds("255u8", &[TokenKind::Int("255u8".into())]);
    assert_kinds("0u8", &[TokenKind::Int("0u8".into())]);
}

#[test]
fn test_int_with_type_suffix_i32() {
    assert_kinds("100i32", &[TokenKind::Int("100i32".into())]);
    assert_kinds("-1i32", &[TokenKind::Minus, TokenKind::Int("1i32".into())]);
}

#[test]
fn test_int_with_type_suffix_u64() {
    assert_kinds("1024u64", &[TokenKind::Int("1024u64".into())]);
}

#[test]
fn test_int_with_uint_suffix() {
    assert_kinds("42u", &[TokenKind::Int("42u".into())]);
    assert_kinds("1024u", &[TokenKind::Int("1024u".into())]);
}

#[test]
fn test_int_hex_with_suffix() {
    assert_kinds("0xFFi64", &[TokenKind::Int("0xFFi64".into())]);
    assert_kinds("0xABu8", &[TokenKind::Int("0xABu8".into())]);
}

#[test]
fn test_float_basic() {
    assert_kinds("3.14", &[TokenKind::Float("3.14".into())]);
    assert_kinds("0.5", &[TokenKind::Float("0.5".into())]);
    assert_kinds("99.99", &[TokenKind::Float("99.99".into())]);
}

#[test]
fn test_float_zero_dot() {
    assert_kinds("0.0", &[TokenKind::Float("0.0".into())]);
}

#[test]
fn test_float_suffix_f32() {
    assert_kinds("0.5f32", &[TokenKind::Float("0.5f32".into())]);
    assert_kinds("3.14f32", &[TokenKind::Float("3.14f32".into())]);
}

#[test]
fn test_float_suffix_f64() {
    assert_kinds("1.0f64", &[TokenKind::Float("1.0f64".into())]);
    assert_kinds("9.81f64", &[TokenKind::Float("9.81f64".into())]);
}

/// An integer followed by `..` should produce Int + DotDot, NOT a float.
#[test]
fn test_int_followed_by_dot_dot() {
    assert_kinds(
        "3..10",
        &[
            TokenKind::Int("3".into()),
            TokenKind::DotDot,
            TokenKind::Int("10".into()),
        ],
    );
}

/// An integer followed by `..=` should produce Int + DotDotEq.
#[test]
fn test_int_followed_by_dot_dot_eq() {
    assert_kinds(
        "1..=5",
        &[
            TokenKind::Int("1".into()),
            TokenKind::DotDotEq,
            TokenKind::Int("5".into()),
        ],
    );
}

/// An integer followed by `.method` should be Int + Dot + Ident (not a float).
#[test]
fn test_int_followed_by_dot_method() {
    assert_kinds(
        "42.to_string()",
        &[
            TokenKind::Int("42".into()),
            TokenKind::Dot,
            TokenKind::Ident("to_string".into()),
            TokenKind::LParen,
            TokenKind::RParen,
        ],
    );
}

/// Float with method call (chaining).
#[test]
fn test_float_followed_by_dot() {
    assert_kinds(
        "3.14.floor()",
        &[
            TokenKind::Float("3.14".into()),
            TokenKind::Dot,
            TokenKind::Ident("floor".into()),
            TokenKind::LParen,
            TokenKind::RParen,
        ],
    );
}

#[test]
fn test_multiple_numbers() {
    assert_kinds(
        "1 2 3",
        &[
            TokenKind::Int("1".into()),
            TokenKind::Int("2".into()),
            TokenKind::Int("3".into()),
        ],
    );
}

// ===========================================================================
// 5. STRING LITERALS
// ===========================================================================

#[test]
fn test_string_empty() {
    assert_kinds(r#""""#, &[TokenKind::String("".into())]);
}

#[test]
fn test_string_simple() {
    assert_kinds(r#""hello""#, &[TokenKind::String("hello".into())]);
}

#[test]
fn test_string_with_spaces() {
    assert_kinds(
        r#""hello world""#,
        &[TokenKind::String("hello world".into())],
    );
}

#[test]
fn test_string_with_numbers() {
    assert_kinds(r#""abc123""#, &[TokenKind::String("abc123".into())]);
}

/// String containing interpolation syntax `{name}` is stored as-is (raw text).
#[test]
fn test_string_interpolation_syntax_preserved() {
    assert_kinds(
        r#""Hello, {name}!""#,
        &[TokenKind::String("Hello, {name}!".into())],
    );
}

#[test]
fn test_string_nested_braces() {
    assert_kinds(
        r#""result: {a + b}""#,
        &[TokenKind::String("result: {a + b}".into())],
    );
}

#[test]
fn test_string_escape_newline() {
    let toks = lex_tokens(r#""\n""#);
    assert_eq!(toks.len(), 1);
    assert_eq!(toks[0].kind, TokenKind::String("\n".into()));
}

#[test]
fn test_string_escape_tab() {
    let toks = lex_tokens(r#""\t""#);
    assert_eq!(toks.len(), 1);
    assert_eq!(toks[0].kind, TokenKind::String("\t".into()));
}

#[test]
fn test_string_escape_carriage_return() {
    let toks = lex_tokens(r#""\r""#);
    assert_eq!(toks.len(), 1);
    assert_eq!(toks[0].kind, TokenKind::String("\r".into()));
}

#[test]
fn test_string_escape_backslash() {
    let toks = lex_tokens(r#""\\""#);
    assert_eq!(toks.len(), 1);
    assert_eq!(toks[0].kind, TokenKind::String("\\".into()));
}

#[test]
fn test_string_escape_quote() {
    let toks = lex_tokens(r#""\"""#);
    assert_eq!(toks.len(), 1);
    assert_eq!(toks[0].kind, TokenKind::String("\"".into()));
}

#[test]
fn test_string_escape_null() {
    let toks = lex_tokens(r#""\0""#);
    assert_eq!(toks.len(), 1);
    assert_eq!(toks[0].kind, TokenKind::String("\0".into()));
}

#[test]
fn test_string_multiple_escapes() {
    let toks = lex_tokens(r#""\n\t\r""#);
    assert_eq!(toks.len(), 1);
    assert_eq!(toks[0].kind, TokenKind::String("\n\t\r".into()));
}

#[test]
fn test_string_followed_by_more_tokens() {
    assert_kinds(
        r#""hello" + "world""#,
        &[
            TokenKind::String("hello".into()),
            TokenKind::Plus,
            TokenKind::String("world".into()),
        ],
    );
}

#[test]
fn test_string_with_special_chars() {
    assert_kinds(r#""!@#$%^&*()""#, &[TokenKind::String("!@#$%^&*()".into())]);
}

#[test]
fn test_unterminated_string_produces_error() {
    // An unterminated string should produce an Error token, not panic.
    let kinds = lex_kinds(r#""hello"#);
    let has_error = kinds.iter().any(|k| matches!(k, TokenKind::Error(_)));
    assert!(has_error, "unterminated string should produce Error token");
}

// ===========================================================================
// 6. CHAR LITERALS
// ===========================================================================

#[test]
fn test_char_letter() {
    assert_kinds("'a'", &[TokenKind::Char('a')]);
    assert_kinds("'z'", &[TokenKind::Char('z')]);
    assert_kinds("'A'", &[TokenKind::Char('A')]);
    assert_kinds("'Z'", &[TokenKind::Char('Z')]);
}

#[test]
fn test_char_digit() {
    assert_kinds("'0'", &[TokenKind::Char('0')]);
    assert_kinds("'9'", &[TokenKind::Char('9')]);
}

#[test]
fn test_char_punctuation() {
    assert_kinds("'!'", &[TokenKind::Char('!')]);
    assert_kinds("'+'", &[TokenKind::Char('+')]);
    assert_kinds("'.'", &[TokenKind::Char('.')]);
    assert_kinds("','", &[TokenKind::Char(',')]);
}

#[test]
fn test_char_space() {
    assert_kinds("' '", &[TokenKind::Char(' ')]);
}

#[test]
fn test_char_escape_newline() {
    assert_kinds(r"'\n'", &[TokenKind::Char('\n')]);
}

#[test]
fn test_char_escape_tab() {
    assert_kinds(r"'\t'", &[TokenKind::Char('\t')]);
}

#[test]
fn test_char_escape_carriage_return() {
    assert_kinds(r"'\r'", &[TokenKind::Char('\r')]);
}

#[test]
fn test_char_escape_backslash() {
    assert_kinds(r"'\\'", &[TokenKind::Char('\\')]);
}

#[test]
fn test_char_escape_single_quote() {
    assert_kinds(r"'\''", &[TokenKind::Char('\'')]);
}

#[test]
fn test_char_escape_null() {
    assert_kinds(r"'\0'", &[TokenKind::Char('\0')]);
}

#[test]
fn test_char_in_expression() {
    assert_kinds(
        "'a' == 'b'",
        &[TokenKind::Char('a'), TokenKind::EqEq, TokenKind::Char('b')],
    );
}

// ===========================================================================
// 7. IDENTIFIERS
// ===========================================================================

#[test]
fn test_ident_simple_lowercase() {
    assert_kinds("foo", &[TokenKind::Ident("foo".into())]);
    assert_kinds("bar", &[TokenKind::Ident("bar".into())]);
    assert_kinds("hello", &[TokenKind::Ident("hello".into())]);
}

#[test]
fn test_ident_simple_uppercase() {
    assert_kinds("Foo", &[TokenKind::Ident("Foo".into())]);
    assert_kinds("Bar", &[TokenKind::Ident("Bar".into())]);
    assert_kinds("User", &[TokenKind::Ident("User".into())]);
}

#[test]
fn test_ident_mixed_case() {
    assert_kinds("myVariable", &[TokenKind::Ident("myVariable".into())]);
    assert_kinds("MyStruct", &[TokenKind::Ident("MyStruct".into())]);
    assert_kinds("camelCase", &[TokenKind::Ident("camelCase".into())]);
    assert_kinds("PascalCase", &[TokenKind::Ident("PascalCase".into())]);
}

#[test]
fn test_ident_with_underscore() {
    assert_kinds("my_var", &[TokenKind::Ident("my_var".into())]);
    assert_kinds("snake_case", &[TokenKind::Ident("snake_case".into())]);
    assert_kinds(
        "long_name_here",
        &[TokenKind::Ident("long_name_here".into())],
    );
}

#[test]
fn test_ident_starts_with_underscore() {
    assert_kinds("_private", &[TokenKind::Ident("_private".into())]);
    assert_kinds("__internal", &[TokenKind::Ident("__internal".into())]);
}

#[test]
fn test_ident_with_digits() {
    assert_kinds("var1", &[TokenKind::Ident("var1".into())]);
    assert_kinds("x2", &[TokenKind::Ident("x2".into())]);
    assert_kinds("test123", &[TokenKind::Ident("test123".into())]);
}

#[test]
fn test_ident_single_letter() {
    assert_kinds("x", &[TokenKind::Ident("x".into())]);
    assert_kinds("a", &[TokenKind::Ident("a".into())]);
    assert_kinds("T", &[TokenKind::Ident("T".into())]);
}

#[test]
fn test_ident_all_caps_constant() {
    assert_kinds("MAX_SIZE", &[TokenKind::Ident("MAX_SIZE".into())]);
    assert_kinds("PI", &[TokenKind::Ident("PI".into())]);
    assert_kinds("MAX_SCORE", &[TokenKind::Ident("MAX_SCORE".into())]);
}

#[test]
fn test_ident_spine_case() {
    // Razen's "Spine Case" — significant word capitalized
    assert_kinds("get_User", &[TokenKind::Ident("get_User".into())]);
    assert_kinds("user_Account", &[TokenKind::Ident("user_Account".into())]);
    assert_kinds(
        "process_Payment",
        &[TokenKind::Ident("process_Payment".into())],
    );
}

/// Underscore alone is the Underscore token, not an identifier.
#[test]
fn test_underscore_standalone_is_not_ident() {
    assert_kinds("_", &[TokenKind::Underscore]);
}

/// Underscore followed by letters is an identifier starting with `_`.
#[test]
fn test_underscore_prefix_is_ident() {
    assert_kinds("_x", &[TokenKind::Ident("_x".into())]);
    assert_kinds("_foo", &[TokenKind::Ident("_foo".into())]);
}

/// Multiple identifiers separated by spaces.
#[test]
fn test_multiple_idents() {
    assert_kinds(
        "a b c",
        &[
            TokenKind::Ident("a".into()),
            TokenKind::Ident("b".into()),
            TokenKind::Ident("c".into()),
        ],
    );
}

// ===========================================================================
// 8. LABELS
// ===========================================================================

#[test]
fn test_label_simple() {
    assert_kinds("'outer", &[TokenKind::QuoteLabel("'outer".into())]);
}

#[test]
fn test_label_inner() {
    assert_kinds("'inner", &[TokenKind::QuoteLabel("'inner".into())]);
}

#[test]
fn test_label_loop_colon() {
    // 'outer: loop ...
    assert_kinds(
        "'outer:",
        &[TokenKind::QuoteLabel("'outer".into()), TokenKind::Colon],
    );
}

#[test]
fn test_label_in_break() {
    assert_kinds(
        "break 'outer",
        &[TokenKind::Break, TokenKind::QuoteLabel("'outer".into())],
    );
}

#[test]
fn test_label_in_next() {
    assert_kinds(
        "next 'loop1",
        &[TokenKind::Next, TokenKind::QuoteLabel("'loop1".into())],
    );
}

#[test]
fn test_label_full_loop_syntax() {
    // 'outer: loop i in 0..5 { break 'outer }
    let kinds = lex_no_eof("'outer: loop i in 0..5 { break 'outer }");
    assert_eq!(kinds[0], TokenKind::QuoteLabel("'outer".into()));
    assert_eq!(kinds[1], TokenKind::Colon);
    assert_eq!(kinds[2], TokenKind::Loop);
    assert_eq!(kinds[3], TokenKind::Ident("i".into()));
    assert_eq!(kinds[4], TokenKind::In);
    assert_eq!(kinds[9], TokenKind::Break);
    assert_eq!(kinds[10], TokenKind::QuoteLabel("'outer".into()));
}

/// A char literal `'a'` should NOT be treated as a label.
#[test]
fn test_char_not_label() {
    // 'a' is a char literal, not a label
    assert_kinds("'a'", &[TokenKind::Char('a')]);
}

/// A label has no closing quote, so 'outer is a label token.
#[test]
fn test_label_vs_char() {
    // 'a' → Char('a')   (has closing quote)
    // 'ab → label (no closing quote, more than one char after ')
    assert_kinds("'a'", &[TokenKind::Char('a')]);
    assert_kinds("'ab", &[TokenKind::QuoteLabel("'ab".into())]);
}

// ===========================================================================
// 9. COMMENTS
// ===========================================================================

#[test]
fn test_line_comment_skipped() {
    // Line comments are skipped — no tokens produced except Eof.
    let kinds = lex_no_eof("// this is a comment");
    assert!(
        kinds.is_empty(),
        "line comment should produce no tokens: {:?}",
        kinds
    );
}

#[test]
fn test_line_comment_tokens_before_and_after() {
    assert_kinds(
        "x // comment\ny",
        &[TokenKind::Ident("x".into()), TokenKind::Ident("y".into())],
    );
}

#[test]
fn test_multiple_line_comments() {
    assert_kinds("// first\n// second\nx", &[TokenKind::Ident("x".into())]);
}

#[test]
fn test_block_comment_skipped() {
    let kinds = lex_no_eof("/* block comment */");
    assert!(
        kinds.is_empty(),
        "block comment should produce no tokens: {:?}",
        kinds
    );
}

#[test]
fn test_block_comment_tokens_around() {
    assert_kinds(
        "a /* comment */ b",
        &[TokenKind::Ident("a".into()), TokenKind::Ident("b".into())],
    );
}

#[test]
fn test_multiline_block_comment() {
    assert_kinds(
        "a /* line1\nline2\nline3 */ b",
        &[TokenKind::Ident("a".into()), TokenKind::Ident("b".into())],
    );
}

#[test]
fn test_doc_comment_preserved() {
    let kinds = lex_no_eof("/// doc comment");
    assert_eq!(kinds.len(), 1);
    assert!(
        matches!(&kinds[0], TokenKind::DocComment(_)),
        "doc comment should produce DocComment token, got: {:?}",
        kinds[0]
    );
}

#[test]
fn test_doc_comment_content() {
    let kinds = lex_no_eof("/// This is the description");
    if let TokenKind::DocComment(content) = &kinds[0] {
        assert!(
            content.contains("This is the description"),
            "DocComment should contain the full text: {:?}",
            content
        );
    } else {
        panic!("expected DocComment token");
    }
}

#[test]
fn test_doc_comment_before_function() {
    let kinds = lex_no_eof("/// My function\nact foo() void {}");
    assert!(
        matches!(&kinds[0], TokenKind::DocComment(_)),
        "first token should be DocComment"
    );
    assert_eq!(kinds[1], TokenKind::Act);
    assert_eq!(kinds[2], TokenKind::Ident("foo".into()));
}

#[test]
fn test_multiple_doc_comments() {
    let kinds = lex_no_eof("/// First\n/// Second\nact foo() {}");
    assert!(matches!(&kinds[0], TokenKind::DocComment(_)));
    assert!(matches!(&kinds[1], TokenKind::DocComment(_)));
    assert_eq!(kinds[2], TokenKind::Act);
}

#[test]
fn test_double_slash_not_doc() {
    // `//` is a regular comment, `///` is a doc comment
    let kinds = lex_no_eof("// regular\n/// doc");
    assert_eq!(kinds.len(), 1, "only doc comment should appear");
    assert!(matches!(&kinds[0], TokenKind::DocComment(_)));
}

#[test]
fn test_comment_at_end_of_line_after_token() {
    assert_kinds(
        "x := 42 // assign x",
        &[
            TokenKind::Ident("x".into()),
            TokenKind::ColonEq,
            TokenKind::Int("42".into()),
        ],
    );
}

// ===========================================================================
// 10. SPAN CORRECTNESS
// ===========================================================================

#[test]
fn test_span_single_token_int() {
    let toks = lex_tokens("42");
    assert_eq!(toks.len(), 1);
    let span = toks[0].span;
    assert_eq!(span.start, 0);
    assert_eq!(span.end, 2);
}

#[test]
fn test_span_keyword() {
    let toks = lex_tokens("mut");
    assert_eq!(toks.len(), 1);
    let span = toks[0].span;
    assert_eq!(span.start, 0);
    assert_eq!(span.end, 3);
}

#[test]
fn test_span_ident_after_space() {
    let toks = lex_tokens("  hello");
    assert_eq!(toks.len(), 1);
    let span = toks[0].span;
    assert_eq!(span.start, 2, "ident should start after 2 spaces");
    assert_eq!(span.end, 7, "ident 'hello' is 5 chars: 2+5=7");
}

#[test]
fn test_span_two_tokens() {
    let toks = lex_tokens("ab cd");
    assert_eq!(toks.len(), 2);
    assert_eq!(toks[0].span.start, 0);
    assert_eq!(toks[0].span.end, 2);
    assert_eq!(toks[1].span.start, 3);
    assert_eq!(toks[1].span.end, 5);
}

#[test]
fn test_span_operator() {
    let toks = lex_tokens(":=");
    assert_eq!(toks.len(), 1);
    assert_eq!(toks[0].span.start, 0);
    assert_eq!(toks[0].span.end, 2);
}

#[test]
fn test_span_increases_monotonically() {
    let toks = lex_tokens("a + b * c");
    let mut prev_end = 0;
    for tok in &toks {
        assert!(
            tok.span.start >= prev_end,
            "span start {} is before previous end {}",
            tok.span.start,
            prev_end
        );
        assert!(
            tok.span.end > tok.span.start || tok.kind == TokenKind::Eof,
            "span end should be > start for non-eof token {:?}",
            tok.kind
        );
        prev_end = tok.span.end;
    }
}

#[test]
fn test_span_start_le_end() {
    // Every token should have start <= end.
    let src = "act main() void { x := 42 + 3 }";
    for tok in tokenize(src) {
        assert!(
            tok.span.start <= tok.span.end,
            "invalid span {:?} for token {:?}",
            tok.span,
            tok.kind
        );
    }
}

// ===========================================================================
// 11. EDGE CASES
// ===========================================================================

#[test]
fn test_empty_source() {
    let kinds = lex_kinds("");
    assert_eq!(kinds, vec![TokenKind::Eof]);
}

#[test]
fn test_whitespace_only_spaces() {
    let kinds = lex_kinds("   ");
    assert_eq!(kinds, vec![TokenKind::Eof]);
}

#[test]
fn test_whitespace_only_newlines() {
    let kinds = lex_kinds("\n\n\n");
    assert_eq!(kinds, vec![TokenKind::Eof]);
}

#[test]
fn test_whitespace_only_tabs() {
    let kinds = lex_kinds("\t\t\t");
    assert_eq!(kinds, vec![TokenKind::Eof]);
}

#[test]
fn test_whitespace_mixed() {
    let kinds = lex_kinds("  \t \n \r\n  ");
    assert_eq!(kinds, vec![TokenKind::Eof]);
}

#[test]
fn test_eof_is_always_last() {
    let kinds = lex_kinds("x");
    assert_eq!(*kinds.last().unwrap(), TokenKind::Eof);
}

#[test]
fn test_eof_at_exactly_end() {
    let kinds = lex_kinds("x := 1");
    assert_eq!(kinds.last(), Some(&TokenKind::Eof));
    // Eof appears exactly once.
    let eof_count = kinds.iter().filter(|k| **k == TokenKind::Eof).count();
    assert_eq!(eof_count, 1, "Eof should appear exactly once");
}

#[test]
fn test_only_comment_no_tokens() {
    let kinds = lex_no_eof("// entire line is a comment");
    assert!(kinds.is_empty());
}

#[test]
fn test_error_unexpected_char() {
    // A character not in Razen's alphabet should produce an Error token.
    let kinds = lex_kinds("$");
    let has_error = kinds.iter().any(|k| matches!(k, TokenKind::Error(_)));
    assert!(
        has_error,
        "unexpected char should produce Error token, got: {:?}",
        kinds
    );
}

#[test]
fn test_tokens_no_spaces() {
    // Tokens that can be adjacent without ambiguity.
    assert_kinds(
        "a+b",
        &[
            TokenKind::Ident("a".into()),
            TokenKind::Plus,
            TokenKind::Ident("b".into()),
        ],
    );
}

#[test]
fn test_consecutive_operators() {
    assert_kinds(
        "+-*/",
        &[
            TokenKind::Plus,
            TokenKind::Minus,
            TokenKind::Star,
            TokenKind::Slash,
        ],
    );
}

#[test]
fn test_tokenize_returns_vec() {
    // `tokenize` is the public API — verify it works end-to-end.
    let tokens = tokenize("mut x: int = 42");
    assert!(!tokens.is_empty());
    assert_eq!(tokens[0].kind, TokenKind::Mut);
    assert_eq!(
        *tokens.last().unwrap(),
        crate::Token {
            kind: TokenKind::Eof,
            span: tokens.last().unwrap().span
        }
    );
}

// ===========================================================================
// 12. REAL-CODE SEQUENCES
// ===========================================================================

#[test]
fn test_variable_declaration_immutable() {
    // x := 42
    assert_kinds(
        "x := 42",
        &[
            TokenKind::Ident("x".into()),
            TokenKind::ColonEq,
            TokenKind::Int("42".into()),
        ],
    );
}

#[test]
fn test_variable_declaration_mutable_explicit_type() {
    // mut score: int = 0
    assert_kinds(
        "mut score: int = 0",
        &[
            TokenKind::Mut,
            TokenKind::Ident("score".into()),
            TokenKind::Colon,
            TokenKind::Ident("int".into()),
            TokenKind::Eq,
            TokenKind::Int("0".into()),
        ],
    );
}

#[test]
fn test_function_header() {
    // act add(a: int, b: int) int
    assert_kinds(
        "act add(a: int, b: int) int",
        &[
            TokenKind::Act,
            TokenKind::Ident("add".into()),
            TokenKind::LParen,
            TokenKind::Ident("a".into()),
            TokenKind::Colon,
            TokenKind::Ident("int".into()),
            TokenKind::Comma,
            TokenKind::Ident("b".into()),
            TokenKind::Colon,
            TokenKind::Ident("int".into()),
            TokenKind::RParen,
            TokenKind::Ident("int".into()),
        ],
    );
}

#[test]
fn test_struct_definition() {
    // struct User { id: int, name: str }
    assert_kinds(
        "struct User { id: int, name: str }",
        &[
            TokenKind::Struct,
            TokenKind::Ident("User".into()),
            TokenKind::LBrace,
            TokenKind::Ident("id".into()),
            TokenKind::Colon,
            TokenKind::Ident("int".into()),
            TokenKind::Comma,
            TokenKind::Ident("name".into()),
            TokenKind::Colon,
            TokenKind::Ident("str".into()),
            TokenKind::RBrace,
        ],
    );
}

#[test]
fn test_if_else_tokens() {
    // if x > 0 { true } else { false }
    assert_kinds(
        "if x > 0 { true } else { false }",
        &[
            TokenKind::If,
            TokenKind::Ident("x".into()),
            TokenKind::Gt,
            TokenKind::Int("0".into()),
            TokenKind::LBrace,
            TokenKind::True,
            TokenKind::RBrace,
            TokenKind::Else,
            TokenKind::LBrace,
            TokenKind::False,
            TokenKind::RBrace,
        ],
    );
}

#[test]
fn test_match_tokens() {
    // match val { 0 -> "zero", _ -> "other" }
    assert_kinds(
        r#"match val { 0 -> "zero", _ -> "other" }"#,
        &[
            TokenKind::Match,
            TokenKind::Ident("val".into()),
            TokenKind::LBrace,
            TokenKind::Int("0".into()),
            TokenKind::Arrow,
            TokenKind::String("zero".into()),
            TokenKind::Comma,
            TokenKind::Underscore,
            TokenKind::Arrow,
            TokenKind::String("other".into()),
            TokenKind::RBrace,
        ],
    );
}

#[test]
fn test_loop_for_in_tokens() {
    // loop i in 0..10 { println(i) }
    assert_kinds(
        "loop i in 0..10 { println(i) }",
        &[
            TokenKind::Loop,
            TokenKind::Ident("i".into()),
            TokenKind::In,
            TokenKind::Int("0".into()),
            TokenKind::DotDot,
            TokenKind::Int("10".into()),
            TokenKind::LBrace,
            TokenKind::Ident("println".into()),
            TokenKind::LParen,
            TokenKind::Ident("i".into()),
            TokenKind::RParen,
            TokenKind::RBrace,
        ],
    );
}

#[test]
fn test_async_await_tokens() {
    // async act fetch(url: str) result[str, str]
    assert_kinds(
        "async act fetch(url: str) result[str, str]",
        &[
            TokenKind::Async,
            TokenKind::Act,
            TokenKind::Ident("fetch".into()),
            TokenKind::LParen,
            TokenKind::Ident("url".into()),
            TokenKind::Colon,
            TokenKind::Ident("str".into()),
            TokenKind::RParen,
            TokenKind::Ident("result".into()),
            TokenKind::LBracket,
            TokenKind::Ident("str".into()),
            TokenKind::Comma,
            TokenKind::Ident("str".into()),
            TokenKind::RBracket,
        ],
    );
}

#[test]
fn test_try_operator_token() {
    // val := parse(s)?
    assert_kinds(
        "val := parse(s)?",
        &[
            TokenKind::Ident("val".into()),
            TokenKind::ColonEq,
            TokenKind::Ident("parse".into()),
            TokenKind::LParen,
            TokenKind::Ident("s".into()),
            TokenKind::RParen,
            TokenKind::Question,
        ],
    );
}

#[test]
fn test_impl_block_header() {
    // impl MyTrait for MyStruct
    assert_kinds(
        "impl MyTrait for MyStruct",
        &[
            TokenKind::Impl,
            TokenKind::Ident("MyTrait".into()),
            TokenKind::Ident("for".into()), // "for" is not a keyword in razen_lexer
            TokenKind::Ident("MyStruct".into()),
        ],
    );
}

#[test]
fn test_use_declaration_tokens() {
    // use std.io
    assert_kinds(
        "use std.io",
        &[
            TokenKind::Use,
            TokenKind::Ident("std".into()),
            TokenKind::Dot,
            TokenKind::Ident("io".into()),
        ],
    );
}

#[test]
fn test_use_with_braces() {
    // use models { User, Role }
    assert_kinds(
        "use models { User, Role }",
        &[
            TokenKind::Use,
            TokenKind::Ident("models".into()),
            TokenKind::LBrace,
            TokenKind::Ident("User".into()),
            TokenKind::Comma,
            TokenKind::Ident("Role".into()),
            TokenKind::RBrace,
        ],
    );
}

#[test]
fn test_attribute_with_args() {
    // @derive[Debug, Clone]
    assert_kinds(
        "@derive[Debug, Clone]",
        &[
            TokenKind::At,
            TokenKind::Ident("derive".into()),
            TokenKind::LBracket,
            TokenKind::Ident("Debug".into()),
            TokenKind::Comma,
            TokenKind::Ident("Clone".into()),
            TokenKind::RBracket,
        ],
    );
}

#[test]
fn test_closure_tokens() {
    // |x: int| x * 2
    assert_kinds(
        "|x: int| x * 2",
        &[
            TokenKind::Or,
            TokenKind::Ident("x".into()),
            TokenKind::Colon,
            TokenKind::Ident("int".into()),
            TokenKind::Or,
            TokenKind::Ident("x".into()),
            TokenKind::Star,
            TokenKind::Int("2".into()),
        ],
    );
}

#[test]
fn test_range_in_loop() {
    // 'a'..='z'
    assert_kinds(
        "'a'..='z'",
        &[
            TokenKind::Char('a'),
            TokenKind::DotDotEq,
            TokenKind::Char('z'),
        ],
    );
}

#[test]
fn test_ret_expression() {
    // ret ok(value)
    assert_kinds(
        "ret ok(value)",
        &[
            TokenKind::Ret,
            TokenKind::Ident("ok".into()),
            TokenKind::LParen,
            TokenKind::Ident("value".into()),
            TokenKind::RParen,
        ],
    );
}

#[test]
fn test_defer_tokens() {
    // defer f.close()
    assert_kinds(
        "defer f.close()",
        &[
            TokenKind::Defer,
            TokenKind::Ident("f".into()),
            TokenKind::Dot,
            TokenKind::Ident("close".into()),
            TokenKind::LParen,
            TokenKind::RParen,
        ],
    );
}

#[test]
fn test_guard_tokens() {
    // guard x > 0 else { ret }
    assert_kinds(
        "guard x > 0 else { ret }",
        &[
            TokenKind::Guard,
            TokenKind::Ident("x".into()),
            TokenKind::Gt,
            TokenKind::Int("0".into()),
            TokenKind::Else,
            TokenKind::LBrace,
            TokenKind::Ret,
            TokenKind::RBrace,
        ],
    );
}

#[test]
fn test_where_clause_tokens() {
    // where T: Clone + Display
    assert_kinds(
        "where T: Clone + Display",
        &[
            TokenKind::Where,
            TokenKind::Ident("T".into()),
            TokenKind::Colon,
            TokenKind::Ident("Clone".into()),
            TokenKind::Plus,
            TokenKind::Ident("Display".into()),
        ],
    );
}

#[test]
fn test_pub_visibility() {
    // pub act foo() void
    assert_kinds(
        "pub act foo() void",
        &[
            TokenKind::Pub,
            TokenKind::Act,
            TokenKind::Ident("foo".into()),
            TokenKind::LParen,
            TokenKind::RParen,
            TokenKind::Ident("void".into()),
        ],
    );
}

#[test]
fn test_fork_tokens() {
    // fork { task1, task2 }
    assert_kinds(
        "fork { task1, task2 }",
        &[
            TokenKind::Fork,
            TokenKind::LBrace,
            TokenKind::Ident("task1".into()),
            TokenKind::Comma,
            TokenKind::Ident("task2".into()),
            TokenKind::RBrace,
        ],
    );
}

#[test]
fn test_unsafe_block_tokens() {
    // unsafe { let x = *ptr }
    assert_kinds(
        "unsafe { let x = *ptr }",
        &[
            TokenKind::Unsafe,
            TokenKind::LBrace,
            TokenKind::Ident("let".into()),
            TokenKind::Ident("x".into()),
            TokenKind::Eq,
            TokenKind::Star,
            TokenKind::Ident("ptr".into()),
            TokenKind::RBrace,
        ],
    );
}

#[test]
fn test_bitwise_ops_in_expression() {
    // a & b | c ^ d
    assert_kinds(
        "a & b | c ^ d",
        &[
            TokenKind::Ident("a".into()),
            TokenKind::And,
            TokenKind::Ident("b".into()),
            TokenKind::Or,
            TokenKind::Ident("c".into()),
            TokenKind::Caret,
            TokenKind::Ident("d".into()),
        ],
    );
}

#[test]
fn test_shift_ops() {
    // x << 2 >> 1
    assert_kinds(
        "x << 2 >> 1",
        &[
            TokenKind::Ident("x".into()),
            TokenKind::Shl,
            TokenKind::Int("2".into()),
            TokenKind::Shr,
            TokenKind::Int("1".into()),
        ],
    );
}

#[test]
fn test_full_function_body_tokens() {
    let src = "act add(a: int, b: int) int { ret a + b }";
    let kinds = lex_no_eof(src);
    assert_eq!(
        kinds,
        vec![
            TokenKind::Act,
            TokenKind::Ident("add".into()),
            TokenKind::LParen,
            TokenKind::Ident("a".into()),
            TokenKind::Colon,
            TokenKind::Ident("int".into()),
            TokenKind::Comma,
            TokenKind::Ident("b".into()),
            TokenKind::Colon,
            TokenKind::Ident("int".into()),
            TokenKind::RParen,
            TokenKind::Ident("int".into()),
            TokenKind::LBrace,
            TokenKind::Ret,
            TokenKind::Ident("a".into()),
            TokenKind::Plus,
            TokenKind::Ident("b".into()),
            TokenKind::RBrace,
        ]
    );
}

#[test]
fn test_complex_expression_tokens() {
    // a + b * c - d / e % f ** g
    let kinds = lex_no_eof("a + b * c - d / e % f ** g");
    assert_eq!(
        kinds,
        vec![
            TokenKind::Ident("a".into()),
            TokenKind::Plus,
            TokenKind::Ident("b".into()),
            TokenKind::Star,
            TokenKind::Ident("c".into()),
            TokenKind::Minus,
            TokenKind::Ident("d".into()),
            TokenKind::Slash,
            TokenKind::Ident("e".into()),
            TokenKind::Percent,
            TokenKind::Ident("f".into()),
            TokenKind::StarStar,
            TokenKind::Ident("g".into()),
        ]
    );
}

#[test]
fn test_newline_does_not_affect_tokens() {
    // Tokens across newlines should be identical to tokens on one line.
    let single_line = lex_no_eof("x := 1 + 2");
    let multi_line = lex_no_eof("x\n:=\n1\n+\n2");
    assert_eq!(single_line, multi_line);
}

#[test]
fn test_vec_literal_tokens() {
    // vec[1, 2, 3]
    assert_kinds(
        "vec[1, 2, 3]",
        &[
            TokenKind::Ident("vec".into()),
            TokenKind::LBracket,
            TokenKind::Int("1".into()),
            TokenKind::Comma,
            TokenKind::Int("2".into()),
            TokenKind::Comma,
            TokenKind::Int("3".into()),
            TokenKind::RBracket,
        ],
    );
}

#[test]
fn test_map_literal_tokens() {
    // map["key": 1]
    assert_kinds(
        r#"map["key": 1]"#,
        &[
            TokenKind::Ident("map".into()),
            TokenKind::LBracket,
            TokenKind::String("key".into()),
            TokenKind::Colon,
            TokenKind::Int("1".into()),
            TokenKind::RBracket,
        ],
    );
}

#[test]
fn test_tensor_literal_tokens() {
    // tensor[1.0, 2.0]
    assert_kinds(
        "tensor[1.0, 2.0]",
        &[
            TokenKind::Ident("tensor".into()),
            TokenKind::LBracket,
            TokenKind::Float("1.0".into()),
            TokenKind::Comma,
            TokenKind::Float("2.0".into()),
            TokenKind::RBracket,
        ],
    );
}

#[test]
fn test_set_literal_tokens() {
    // set[1, 2, 3]
    assert_kinds(
        "set[1, 2, 3]",
        &[
            TokenKind::Ident("set".into()),
            TokenKind::LBracket,
            TokenKind::Int("1".into()),
            TokenKind::Comma,
            TokenKind::Int("2".into()),
            TokenKind::Comma,
            TokenKind::Int("3".into()),
            TokenKind::RBracket,
        ],
    );
}
