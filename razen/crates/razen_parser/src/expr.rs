//! Expression parser (Pratt / precedence climbing).
//!
//! This module implements a production-quality expression parser using
//! precedence climbing (Pratt parsing). It handles all Razen expression
//! forms with correct associativity and precedence.

use razen_ast::expr::*;
use razen_ast::ident::Ident;
use razen_ast::item::MatchArm;
use razen_ast::lit::Literal;
use razen_ast::ops::{BinOp, CompoundOp, UnaryOp};

use razen_ast::Span;
use razen_lexer::TokenKind;

use crate::error::ParseError;
use crate::input::TokenStream;
use crate::pat::parse_pattern;
use crate::stmt::parse_block_stmts;
use crate::types::parse_type;

// ---------------------------------------------------------------------------
// Precedence levels (higher = binds tighter)
// ---------------------------------------------------------------------------

fn prefix_binding_power(op: &UnaryOp) -> u8 {
    match op {
        UnaryOp::Neg | UnaryOp::Not | UnaryOp::BitNot | UnaryOp::Ref | UnaryOp::Deref => 27,
    }
}

fn infix_binding_power(op: &BinOp) -> (u8, u8) {
    match op {
        BinOp::AsyncPipe => (2, 3),
        BinOp::Or => (4, 5),
        BinOp::And => (6, 7),
        BinOp::Eq | BinOp::NotEq => (8, 9),
        BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq => (10, 11),
        BinOp::Range | BinOp::RangeInclusive => (12, 13),
        BinOp::BitOr => (14, 15),
        BinOp::BitXor => (16, 17),
        BinOp::BitAnd => (18, 19),
        BinOp::Shl | BinOp::Shr => (20, 21),
        BinOp::Add | BinOp::Sub => (22, 23),
        BinOp::Mul | BinOp::Div | BinOp::Mod => (24, 25),
        BinOp::Pow => (29, 28), // right-associative
    }
}

fn token_to_binop(kind: &TokenKind) -> Option<BinOp> {
    match kind {
        TokenKind::Plus => Some(BinOp::Add),
        TokenKind::Minus => Some(BinOp::Sub),
        TokenKind::Star => Some(BinOp::Mul),
        TokenKind::Slash => Some(BinOp::Div),
        TokenKind::Percent => Some(BinOp::Mod),
        TokenKind::StarStar => Some(BinOp::Pow),
        TokenKind::EqEq => Some(BinOp::Eq),
        TokenKind::NotEq => Some(BinOp::NotEq),
        TokenKind::Lt => Some(BinOp::Lt),
        TokenKind::Gt => Some(BinOp::Gt),
        TokenKind::LtEq => Some(BinOp::LtEq),
        TokenKind::GtEq => Some(BinOp::GtEq),
        TokenKind::AndAnd => Some(BinOp::And),
        TokenKind::OrOr => Some(BinOp::Or),
        TokenKind::And => Some(BinOp::BitAnd),
        TokenKind::Or => Some(BinOp::BitOr),
        TokenKind::Caret => Some(BinOp::BitXor),
        TokenKind::Shl => Some(BinOp::Shl),
        TokenKind::Shr => Some(BinOp::Shr),
        TokenKind::DotDot => Some(BinOp::Range),
        TokenKind::DotDotEq => Some(BinOp::RangeInclusive),
        TokenKind::AsyncPipe => Some(BinOp::AsyncPipe),
        _ => None,
    }
}

fn token_to_compound_op(kind: &TokenKind) -> Option<CompoundOp> {
    match kind {
        TokenKind::PlusEq => Some(CompoundOp::AddAssign),
        TokenKind::MinusEq => Some(CompoundOp::SubAssign),
        TokenKind::StarEq => Some(CompoundOp::MulAssign),
        TokenKind::SlashEq => Some(CompoundOp::DivAssign),
        TokenKind::PercentEq => Some(CompoundOp::ModAssign),
        TokenKind::StarStarEq => Some(CompoundOp::PowAssign),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parse an expression.
pub fn parse_expr(s: &mut TokenStream) -> Result<Expr, ParseError> {
    parse_expr_bp(s, 0)
}

/// Convert a Literal to an Expr (used by pattern parser for range patterns).
pub fn lit_to_expr(lit: Literal) -> Expr {
    let span = lit.span();
    Expr::Literal { lit, span }
}

// ---------------------------------------------------------------------------
// Pratt parser core
// ---------------------------------------------------------------------------

fn parse_expr_bp(s: &mut TokenStream, min_bp: u8) -> Result<Expr, ParseError> {
    let mut lhs = parse_unary(s)?;

    loop {
        // Check for assignment: =
        if min_bp == 0 && s.check(&TokenKind::Eq) {
            let start = lhs.span();
            s.advance();
            let rhs = parse_expr_bp(s, 0)?;
            let span = Span::new(start.start, rhs.span().end);
            lhs = Expr::Assign {
                target: Box::new(lhs),
                value: Box::new(rhs),
                span,
            };
            continue;
        }

        // Check for compound assignment: +=, -=, etc.
        if min_bp == 0 {
            if let Some(cop) = token_to_compound_op(s.peek_kind()) {
                let start = lhs.span();
                s.advance();
                let rhs = parse_expr_bp(s, 0)?;
                let span = Span::new(start.start, rhs.span().end);
                lhs = Expr::CompoundAssign {
                    target: Box::new(lhs),
                    op: cop,
                    value: Box::new(rhs),
                    span,
                };
                continue;
            }
        }

        // Check for `as` (type cast)
        if s.check(&TokenKind::As) {
            let start = lhs.span();
            s.advance();
            let ty = parse_type(s)?;
            let span = Span::new(start.start, ty.span().end);
            lhs = Expr::Cast {
                expr: Box::new(lhs),
                ty,
                span,
            };
            continue;
        }

        // Check for `is` (type check)
        if s.check(&TokenKind::Is) {
            let start = lhs.span();
            s.advance();
            let ty = parse_type(s)?;
            let span = Span::new(start.start, ty.span().end);
            lhs = Expr::TypeCheck {
                expr: Box::new(lhs),
                ty,
                span,
            };
            continue;
        }

        // Binary operators
        if let Some(op) = token_to_binop(s.peek_kind()) {
            let (l_bp, r_bp) = infix_binding_power(&op);
            if l_bp < min_bp {
                break;
            }
            s.advance();
            let rhs = parse_expr_bp(s, r_bp)?;
            let span = Span::new(lhs.span().start, rhs.span().end);
            lhs = Expr::Binary {
                left: Box::new(lhs),
                op,
                right: Box::new(rhs),
                span,
            };
            continue;
        }

        // Postfix operations
        lhs = match parse_postfix(s, lhs)? {
            PostfixResult::Consumed(new_lhs) => new_lhs,
            PostfixResult::Done(final_lhs) => {
                lhs = final_lhs;
                break;
            }
        };
    }

    Ok(lhs)
}

enum PostfixResult {
    Consumed(Expr),
    Done(Expr),
}

fn parse_postfix(s: &mut TokenStream, lhs: Expr) -> Result<PostfixResult, ParseError> {
    match s.peek_kind().clone() {
        // Field access / method call / .await: expr.ident, expr.method(), expr.await
        TokenKind::Dot => {
            let start = lhs.span();
            s.advance();

            // .await
            if s.check(&TokenKind::Await) {
                s.advance();
                let span = s.span_from(start);
                return Ok(PostfixResult::Consumed(Expr::Await {
                    expr: Box::new(lhs),
                    span,
                }));
            }

            let (field_name, field_span) = s.expect_ident()?;
            let field = Ident::new(field_name, field_span);

            // Method call: expr.method(args)
            if s.check(&TokenKind::LParen) {
                s.advance();
                let args =
                    s.parse_comma_separated(&TokenKind::RParen, |s| parse_expr(s))?;
                s.expect(&TokenKind::RParen)?;
                let span = s.span_from(start);
                Ok(PostfixResult::Consumed(Expr::MethodCall {
                    object: Box::new(lhs),
                    method: field,
                    args,
                    span,
                }))
            }
            // Field access
            else {
                let span = s.span_from(start);
                Ok(PostfixResult::Consumed(Expr::Field {
                    object: Box::new(lhs),
                    field,
                    span,
                }))
            }
        }

        // Index: expr[index]
        TokenKind::LBracket => {
            let start = lhs.span();
            s.advance();
            let index = parse_expr(s)?;
            s.expect(&TokenKind::RBracket)?;
            let span = s.span_from(start);
            Ok(PostfixResult::Consumed(Expr::Index {
                object: Box::new(lhs),
                index: Box::new(index),
                span,
            }))
        }

        // Function call: expr(args)
        TokenKind::LParen => {
            let start = lhs.span();
            s.advance();
            let args = s.parse_comma_separated(&TokenKind::RParen, |s| parse_expr(s))?;
            s.expect(&TokenKind::RParen)?;
            let span = s.span_from(start);
            Ok(PostfixResult::Consumed(Expr::Call {
                callee: Box::new(lhs),
                args,
                span,
            }))
        }

        // Error propagation: expr?
        TokenKind::Question => {
            let start = lhs.span();
            s.advance();
            let span = s.span_from(start);
            Ok(PostfixResult::Consumed(Expr::Try {
                expr: Box::new(lhs),
                span,
            }))
        }

        _ => Ok(PostfixResult::Done(lhs)),
    }
}

// ---------------------------------------------------------------------------
// Unary prefix parsing
// ---------------------------------------------------------------------------

fn parse_unary(s: &mut TokenStream) -> Result<Expr, ParseError> {
    let start = s.current_span();

    match s.peek_kind().clone() {
        TokenKind::Minus => {
            s.advance();
            let op = UnaryOp::Neg;
            let bp = prefix_binding_power(&op);
            let operand = parse_expr_bp(s, bp)?;
            let span = s.span_from(start);
            Ok(Expr::Unary {
                op,
                operand: Box::new(operand),
                span,
            })
        }
        TokenKind::Bang => {
            s.advance();
            let op = UnaryOp::Not;
            let bp = prefix_binding_power(&op);
            let operand = parse_expr_bp(s, bp)?;
            let span = s.span_from(start);
            Ok(Expr::Unary {
                op,
                operand: Box::new(operand),
                span,
            })
        }
        TokenKind::Tilde => {
            s.advance();
            let op = UnaryOp::BitNot;
            let bp = prefix_binding_power(&op);
            let operand = parse_expr_bp(s, bp)?;
            let span = s.span_from(start);
            Ok(Expr::Unary {
                op,
                operand: Box::new(operand),
                span,
            })
        }
        TokenKind::And => {
            s.advance();
            let op = UnaryOp::Ref;
            let bp = prefix_binding_power(&op);
            let operand = parse_expr_bp(s, bp)?;
            let span = s.span_from(start);
            Ok(Expr::Unary {
                op,
                operand: Box::new(operand),
                span,
            })
        }
        TokenKind::Star => {
            s.advance();
            let op = UnaryOp::Deref;
            let bp = prefix_binding_power(&op);
            let operand = parse_expr_bp(s, bp)?;
            let span = s.span_from(start);
            Ok(Expr::Unary {
                op,
                operand: Box::new(operand),
                span,
            })
        }
        _ => parse_primary(s),
    }
}

// ---------------------------------------------------------------------------
// Primary expressions
// ---------------------------------------------------------------------------

/// Parse a primary (atomic) expression.
pub fn parse_primary(s: &mut TokenStream) -> Result<Expr, ParseError> {
    let start = s.current_span();

    match s.peek_kind().clone() {
        // Integer literal
        TokenKind::Int(ref raw) => {
            let raw = raw.clone();
            s.advance();
            Ok(Expr::Literal {
                lit: Literal::Int {
                    raw,
                    span: start,
                },
                span: start,
            })
        }

        // Float literal
        TokenKind::Float(ref raw) => {
            let raw = raw.clone();
            s.advance();
            Ok(Expr::Literal {
                lit: Literal::Float {
                    raw,
                    span: start,
                },
                span: start,
            })
        }

        // String literal
        TokenKind::String(ref val) => {
            let val = val.clone();
            s.advance();
            Ok(Expr::Literal {
                lit: Literal::Str {
                    value: val,
                    span: start,
                },
                span: start,
            })
        }

        // Char literal
        TokenKind::Char(ch) => {
            s.advance();
            Ok(Expr::Literal {
                lit: Literal::Char {
                    value: ch,
                    span: start,
                },
                span: start,
            })
        }

        // Boolean: true
        TokenKind::True => {
            s.advance();
            Ok(Expr::Literal {
                lit: Literal::Bool {
                    value: true,
                    span: start,
                },
                span: start,
            })
        }

        // Boolean: false
        TokenKind::False => {
            s.advance();
            Ok(Expr::Literal {
                lit: Literal::Bool {
                    value: false,
                    span: start,
                },
                span: start,
            })
        }

        // self
        TokenKind::SelfKw => {
            s.advance();
            Ok(Expr::Ident {
                ident: Ident::new("self", start),
                span: start,
            })
        }

        // Identifier (may start a path, call, struct literal, etc.)
        TokenKind::Ident(ref name) => {
            let name = name.clone();
            parse_ident_or_path(s, name, start)
        }

        // Parenthesized expression or tuple
        TokenKind::LParen => {
            s.advance();
            if s.check(&TokenKind::RParen) {
                s.advance();
                let span = s.span_from(start);
                return Ok(Expr::Tuple {
                    elements: vec![],
                    span,
                });
            }
            let first = parse_expr(s)?;
            if s.check(&TokenKind::Comma) {
                // It's a tuple
                s.advance();
                let mut elements = vec![first];
                if !s.check(&TokenKind::RParen) {
                    let rest = s.parse_comma_separated(&TokenKind::RParen, |s| {
                        parse_expr(s)
                    })?;
                    elements.extend(rest);
                }
                s.expect(&TokenKind::RParen)?;
                let span = s.span_from(start);
                Ok(Expr::Tuple { elements, span })
            } else {
                s.expect(&TokenKind::RParen)?;
                let span = s.span_from(start);
                Ok(Expr::Paren {
                    inner: Box::new(first),
                    span,
                })
            }
        }

        // Block expression: { ... }
        TokenKind::LBrace => parse_block_expr(s),

        // Array literal: [1, 2, 3]
        TokenKind::LBracket => {
            s.advance();
            let elements =
                s.parse_comma_separated(&TokenKind::RBracket, |s| parse_expr(s))?;
            s.expect(&TokenKind::RBracket)?;
            let span = s.span_from(start);
            Ok(Expr::Array { elements, span })
        }

        // If expression
        TokenKind::If => parse_if_expr(s),

        // Match expression
        TokenKind::Match => parse_match_expr(s),

        // Loop expression
        TokenKind::Loop => parse_loop_expr(s, None),

        // Closure: |params| body
        TokenKind::Or => parse_closure(s),

        // Return
        TokenKind::Ret => {
            s.advance();
            let value = if !s.is_eof()
                && !s.check(&TokenKind::RBrace)
                && !s.check(&TokenKind::Comma)
            {
                Some(Box::new(parse_expr(s)?))
            } else {
                None
            };
            let span = s.span_from(start);
            Ok(Expr::Return { value, span })
        }

        // Break
        TokenKind::Break => {
            s.advance();
            let label = if let TokenKind::QuoteLabel(ref l) = s.peek_kind().clone() {
                let l = l.clone();
                s.advance();
                Some(l)
            } else {
                None
            };
            let value = if !s.is_eof()
                && !s.check(&TokenKind::RBrace)
                && !s.check(&TokenKind::Comma)
                && !s.check(&TokenKind::Semi)
            {
                Some(Box::new(parse_expr(s)?))
            } else {
                None
            };
            let span = s.span_from(start);
            Ok(Expr::Break { label, value, span })
        }

        // Next (continue)
        TokenKind::Next => {
            s.advance();
            let label = if let TokenKind::QuoteLabel(ref l) = s.peek_kind().clone() {
                let l = l.clone();
                s.advance();
                Some(l)
            } else {
                None
            };
            let span = s.span_from(start);
            Ok(Expr::Next { label, span })
        }

        // Fork
        TokenKind::Fork => parse_fork_expr(s),

        // Unsafe block
        TokenKind::Unsafe => {
            s.advance();
            let body = parse_block_expr(s)?;
            let span = s.span_from(start);
            Ok(Expr::Unsafe {
                body: Box::new(body),
                span,
            })
        }

        // Labeled loop: 'label: loop ...
        TokenKind::QuoteLabel(ref label) => {
            let label = label.clone();
            s.advance();
            s.expect(&TokenKind::Colon)?;
            if s.check(&TokenKind::Loop) {
                parse_loop_expr(s, Some(label))
            } else {
                Err(ParseError::expected(
                    "expected `loop` after label",
                    start,
                    vec!["loop".to_string()],
                ))
            }
        }

        // Underscore (wildcard)
        TokenKind::Underscore => {
            s.advance();
            Ok(Expr::Ident {
                ident: Ident::new("_", start),
                span: start,
            })
        }

        _ => Err(ParseError::unexpected_token(s.peek_kind(), start)),
    }
}

// ---------------------------------------------------------------------------
// Identifier / Path / Struct literal / collection literal dispatch
// ---------------------------------------------------------------------------

fn parse_ident_or_path(
    s: &mut TokenStream,
    name: String,
    start: Span,
) -> Result<Expr, ParseError> {
    // Check for collection constructors: vec[...], map[...], set[...], tensor[...]
    match name.as_str() {
        "vec" if s.check(&TokenKind::LBracket) => {
            s.advance(); // consume ident
            s.advance(); // consume [
            let elements =
                s.parse_comma_separated(&TokenKind::RBracket, |s| parse_expr(s))?;
            s.expect(&TokenKind::RBracket)?;
            let span = s.span_from(start);
            return Ok(Expr::Vec { elements, span });
        }
        "set" if s.check(&TokenKind::LBracket) => {
            s.advance();
            s.advance();
            let elements =
                s.parse_comma_separated(&TokenKind::RBracket, |s| parse_expr(s))?;
            s.expect(&TokenKind::RBracket)?;
            let span = s.span_from(start);
            return Ok(Expr::Set { elements, span });
        }
        "map" if s.check(&TokenKind::LBracket) => {
            s.advance();
            s.advance();
            let entries = s.parse_comma_separated(&TokenKind::RBracket, |s| {
                let key = parse_expr(s)?;
                s.expect(&TokenKind::Colon)?;
                let value = parse_expr(s)?;
                Ok((key, value))
            })?;
            s.expect(&TokenKind::RBracket)?;
            let span = s.span_from(start);
            return Ok(Expr::Map { entries, span });
        }
        "tensor" if s.check(&TokenKind::LBracket) => {
            s.advance();
            s.advance();
            let elements =
                s.parse_comma_separated(&TokenKind::RBracket, |s| parse_expr(s))?;
            s.expect(&TokenKind::RBracket)?;
            let span = s.span_from(start);
            return Ok(Expr::Tensor { elements, span });
        }
        _ => {}
    }

    let ident = Ident::new(name, start);
    s.advance();

    // Check for struct literal: Name { field: value, ... }
    // We look for `Ident {` but must distinguish from block expressions.
    // A struct literal starts with `Name {` where Name starts with uppercase
    // or is followed by a field pattern like `ident:`.
    if s.check(&TokenKind::LBrace) {
        // Heuristic: if next token after `{` is `ident:` or `..`, it's a struct literal
        let save = s.save();
        if is_struct_literal_start(s) {
            return parse_struct_literal_body(s, Expr::Ident { ident, span: start }, start);
        }
        s.restore(save);
    }

    Ok(Expr::Ident {
        ident,
        span: start,
    })
}

/// Heuristic to determine if `{` starts a struct literal vs a block.
fn is_struct_literal_start(s: &TokenStream) -> bool {
    // Look at token after `{`
    let after_brace = &s.peek_ahead(1).kind;
    match after_brace {
        // { field_name: ... } — likely struct
        TokenKind::Ident(_) => {
            let after_ident = &s.peek_ahead(2).kind;
            matches!(after_ident, TokenKind::Colon | TokenKind::Comma | TokenKind::RBrace)
        }
        // { ..source } — struct update
        TokenKind::DotDot => true,
        // { } — could be either, treat as empty struct
        TokenKind::RBrace => true,
        _ => false,
    }
}

fn parse_struct_literal_body(
    s: &mut TokenStream,
    name_expr: Expr,
    start: Span,
) -> Result<Expr, ParseError> {
    s.expect(&TokenKind::LBrace)?;

    let mut fields = Vec::new();
    let mut spread = None;

    while !s.check(&TokenKind::RBrace) && !s.is_eof() {
        // Check for spread: ..source
        if s.check(&TokenKind::DotDot) {
            s.advance();
            spread = Some(Box::new(parse_expr(s)?));
            s.eat(&TokenKind::Comma);
            break;
        }

        let (field_name, field_span) = s.expect_ident()?;
        let field_ident = Ident::new(field_name, field_span);
        s.expect(&TokenKind::Colon)?;
        let value = parse_expr(s)?;
        let fspan = s.span_from(field_span);
        fields.push(StructLiteralField {
            name: field_ident,
            value,
            span: fspan,
        });

        if !s.eat(&TokenKind::Comma) {
            break;
        }
    }

    s.expect(&TokenKind::RBrace)?;
    let span = s.span_from(start);

    Ok(Expr::StructLiteral {
        name: Box::new(name_expr),
        fields,
        spread,
        span,
    })
}

// ---------------------------------------------------------------------------
// Block expression
// ---------------------------------------------------------------------------

pub fn parse_block_expr(s: &mut TokenStream) -> Result<Expr, ParseError> {
    let start = s.current_span();
    s.expect(&TokenKind::LBrace)?;
    let (stmts, tail) = parse_block_stmts(s)?;
    s.expect(&TokenKind::RBrace)?;
    let span = s.span_from(start);
    Ok(Expr::Block { stmts, tail, span })
}

// ---------------------------------------------------------------------------
// If expression
// ---------------------------------------------------------------------------

fn parse_if_expr(s: &mut TokenStream) -> Result<Expr, ParseError> {
    let start = s.current_span();
    s.expect(&TokenKind::If)?;

    // Check for `if let`
    if s.check(&TokenKind::Ident(String::new())) || {
        // Check specifically for the `let` keyword (it's parsed as an identifier by the lexer)
        matches!(s.peek_kind(), TokenKind::Ident(n) if n == "let")
    } {
        if let TokenKind::Ident(ref n) = s.peek_kind().clone() {
            if n == "let" {
                s.advance(); // consume `let`
                let pattern = parse_pattern(s)?;
                s.expect(&TokenKind::Eq)?;
                let value = parse_expr(s)?;
                let then_block = parse_block_expr(s)?;
                let else_block = if s.eat(&TokenKind::Else) {
                    Some(Box::new(if s.check(&TokenKind::If) {
                        parse_if_expr(s)?
                    } else {
                        parse_block_expr(s)?
                    }))
                } else {
                    None
                };
                let span = s.span_from(start);
                return Ok(Expr::IfLet {
                    pattern,
                    value: Box::new(value),
                    then_block: Box::new(then_block),
                    else_block,
                    span,
                });
            }
        }
    }

    let condition = parse_expr(s)?;
    let then_block = parse_block_expr(s)?;
    let else_block = if s.eat(&TokenKind::Else) {
        Some(Box::new(if s.check(&TokenKind::If) {
            parse_if_expr(s)?
        } else {
            parse_block_expr(s)?
        }))
    } else {
        None
    };
    let span = s.span_from(start);
    Ok(Expr::If {
        condition: Box::new(condition),
        then_block: Box::new(then_block),
        else_block,
        span,
    })
}

// ---------------------------------------------------------------------------
// Match expression
// ---------------------------------------------------------------------------

fn parse_match_expr(s: &mut TokenStream) -> Result<Expr, ParseError> {
    let start = s.current_span();
    s.expect(&TokenKind::Match)?;
    let subject = parse_expr(s)?;
    s.expect(&TokenKind::LBrace)?;

    let mut arms = Vec::new();
    while !s.check(&TokenKind::RBrace) && !s.is_eof() {
        let arm_start = s.current_span();
        let pattern = parse_pattern(s)?;

        // Optional guard: `if condition`
        let guard = if s.eat(&TokenKind::If) {
            Some(Box::new(parse_expr(s)?))
        } else {
            None
        };

        s.expect(&TokenKind::Arrow)?;
        let body = parse_expr(s)?;
        let arm_span = s.span_from(arm_start);

        arms.push(MatchArm {
            pattern,
            guard,
            body: Box::new(body),
            span: arm_span,
        });

        // Arms are comma-separated
        if !s.eat(&TokenKind::Comma) {
            break;
        }
    }

    s.expect(&TokenKind::RBrace)?;
    let span = s.span_from(start);
    Ok(Expr::Match {
        subject: Box::new(subject),
        arms,
        span,
    })
}

// ---------------------------------------------------------------------------
// Loop expression
// ---------------------------------------------------------------------------

fn parse_loop_expr(s: &mut TokenStream, label: Option<String>) -> Result<Expr, ParseError> {
    let start = s.current_span();
    s.expect(&TokenKind::Loop)?;

    // Check for `loop let` pattern
    if matches!(s.peek_kind(), TokenKind::Ident(n) if n == "let") {
        s.advance(); // consume `let`
        let pattern = parse_pattern(s)?;
        s.expect(&TokenKind::Eq)?;
        let value = parse_expr(s)?;
        let body = parse_block_expr(s)?;
        let span = s.span_from(start);
        return Ok(Expr::LoopLet {
            label,
            pattern,
            value: Box::new(value),
            body: Box::new(body),
            span,
        });
    }

    // Check for `loop { ... }` (infinite)
    if s.check(&TokenKind::LBrace) {
        let body = parse_block_expr(s)?;
        let else_block = if s.eat(&TokenKind::Else) {
            Some(Box::new(parse_block_expr(s)?))
        } else {
            None
        };
        let span = s.span_from(start);
        return Ok(Expr::Loop {
            label,
            kind: LoopKind::Infinite,
            body: Box::new(body),
            else_block,
            span,
        });
    }

    // Try to detect `loop var in iterable` vs `loop condition`
    // If we see `ident in` or `(pattern) in`, it's a for-in loop.
    let save = s.save();

    // Try pattern + `in`
    if let Ok(binding) = parse_pattern(s) {
        if s.eat(&TokenKind::In) {
            let iterable = parse_expr(s)?;
            let body = parse_block_expr(s)?;
            let else_block = if s.eat(&TokenKind::Else) {
                Some(Box::new(parse_block_expr(s)?))
            } else {
                None
            };
            let span = s.span_from(start);
            return Ok(Expr::Loop {
                label,
                kind: LoopKind::ForIn {
                    binding,
                    iterable: Box::new(iterable),
                },
                body: Box::new(body),
                else_block,
                span,
            });
        }
    }

    // Not a for-in — restore and parse as while-style
    s.restore(save);
    let condition = parse_expr(s)?;
    let body = parse_block_expr(s)?;
    let else_block = if s.eat(&TokenKind::Else) {
        Some(Box::new(parse_block_expr(s)?))
    } else {
        None
    };
    let span = s.span_from(start);
    Ok(Expr::Loop {
        label,
        kind: LoopKind::While {
            condition: Box::new(condition),
        },
        body: Box::new(body),
        else_block,
        span,
    })
}

// ---------------------------------------------------------------------------
// Closure
// ---------------------------------------------------------------------------

fn parse_closure(s: &mut TokenStream) -> Result<Expr, ParseError> {
    let start = s.current_span();
    s.expect(&TokenKind::Or)?;

    let params = if s.check(&TokenKind::Or) {
        vec![]
    } else {
        s.parse_comma_separated(&TokenKind::Or, |s| {
            let (pname, pspan) = s.expect_ident()?;
            let ty = if s.check(&TokenKind::Colon) {
                s.advance();
                Some(parse_type(s)?)
            } else {
                None
            };
            let span = s.span_from(pspan);
            Ok(ClosureParam {
                name: Ident::new(pname, pspan),
                ty,
                span,
            })
        })?
    };

    s.expect(&TokenKind::Or)?;

    // Body: either a block `{ ... }` or a single expression
    let body = if s.check(&TokenKind::LBrace) {
        parse_block_expr(s)?
    } else {
        parse_expr(s)?
    };

    let span = s.span_from(start);
    Ok(Expr::Closure {
        params,
        body: Box::new(body),
        span,
    })
}

// ---------------------------------------------------------------------------
// Fork expression
// ---------------------------------------------------------------------------

fn parse_fork_expr(s: &mut TokenStream) -> Result<Expr, ParseError> {
    let start = s.current_span();
    s.expect(&TokenKind::Fork)?;

    // fork loop ...
    if s.check(&TokenKind::Loop) {
        s.advance();
        let binding = parse_pattern(s)?;
        s.expect(&TokenKind::In)?;
        let iterable = parse_expr(s)?;
        let body = parse_block_expr(s)?;
        let span = s.span_from(start);
        return Ok(Expr::Fork {
            kind: ForkKind::Loop {
                binding,
                iterable: Box::new(iterable),
                body: Box::new(body),
            },
            span,
        });
    }

    // fork { task1, task2, ... }
    s.expect(&TokenKind::LBrace)?;
    let mut tasks = Vec::new();

    while !s.check(&TokenKind::RBrace) && !s.is_eof() {
        let task_start = s.current_span();

        // Check for named binding: `name <- expr`
        let save = s.save();
        let binding = if let Ok((name, span)) = s.expect_ident() {
            if s.check(&TokenKind::Lt) {
                // Check for `<-`
                s.advance();
                if s.eat(&TokenKind::Minus) {
                    Some(Ident::new(name, span))
                } else {
                    s.restore(save);
                    None
                }
            } else {
                s.restore(save);
                None
            }
        } else {
            s.restore(save);
            None
        };

        let expr = parse_expr(s)?;
        let task_span = s.span_from(task_start);
        tasks.push(ForkTask {
            binding,
            expr,
            span: task_span,
        });

        if !s.eat(&TokenKind::Comma) {
            break;
        }
    }

    s.expect(&TokenKind::RBrace)?;
    let span = s.span_from(start);
    Ok(Expr::Fork {
        kind: ForkKind::Block { tasks },
        span,
    })
}
