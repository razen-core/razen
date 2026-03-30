//! Statement parser.
//!
//! Parses variable declarations (let, mut, const, shared), expression statements,
//! control flow (ret, break, next), defer, and guard.

use razen_ast::expr::Expr;
use razen_ast::ident::Ident;

use razen_ast::pat::Pattern;
use razen_ast::stmt::Stmt;

use razen_lexer::TokenKind;

use crate::error::ParseError;
use crate::expr::{parse_block_expr, parse_expr};
use crate::input::TokenStream;
use crate::item;
use crate::types::{parse_optional_type, parse_type_annotation};

/// Parse a single statement.
pub fn parse_stmt(s: &mut TokenStream) -> Result<Stmt, ParseError> {
    let start = s.current_span();

    match s.peek_kind().clone() {
        // Mutable variable: `mut name: Type = value`
        TokenKind::Mut => {
            s.advance();
            let (name, name_span) = s.expect_ident()?;
            let ident = Ident::new(name, name_span);
            let ty = parse_type_annotation(s)?;
            s.expect(&TokenKind::Eq)?;
            let value = parse_expr(s)?;
            let span = s.span_from(start);
            Ok(Stmt::LetMut {
                name: ident,
                ty,
                value,
                span,
            })
        }

        // Constant: `const NAME: Type = value`
        TokenKind::Const => {
            s.advance();
            let (name, name_span) = s.expect_ident()?;
            let ident = Ident::new(name, name_span);
            let ty = parse_type_annotation(s)?;
            s.expect(&TokenKind::Eq)?;
            let value = parse_expr(s)?;
            let span = s.span_from(start);
            Ok(Stmt::Const {
                name: ident,
                ty,
                value,
                span,
            })
        }

        // Shared: `shared name: Type = value` or `shared name = value`
        TokenKind::Shared => {
            s.advance();
            let (name, name_span) = s.expect_ident()?;
            let ident = Ident::new(name, name_span);
            let ty = parse_optional_type(s)?;
            s.expect(&TokenKind::Eq)?;
            let value = parse_expr(s)?;
            let span = s.span_from(start);
            Ok(Stmt::Shared {
                name: ident,
                ty,
                value,
                span,
            })
        }

        // Return: `ret expr`
        TokenKind::Ret => {
            s.advance();
            let value = if !s.is_eof()
                && !s.check(&TokenKind::RBrace)
            {
                Some(parse_expr(s)?)
            } else {
                None
            };
            let span = s.span_from(start);
            Ok(Stmt::Return { value, span })
        }

        // Defer: `defer expr`
        TokenKind::Defer => {
            s.advance();
            let body = parse_expr(s)?;
            let span = s.span_from(start);
            Ok(Stmt::Defer { body, span })
        }

        // Guard: `guard condition else { ... }`
        TokenKind::Guard => {
            s.advance();
            let condition = parse_expr(s)?;
            s.expect(&TokenKind::Else)?;
            let else_body = parse_block_expr(s)?;
            let span = s.span_from(start);
            Ok(Stmt::Guard {
                condition,
                else_body,
                span,
            })
        }

        // Items that can appear at statement level
        TokenKind::Act | TokenKind::Struct | TokenKind::Enum | TokenKind::Trait
        | TokenKind::Impl | TokenKind::Alias | TokenKind::Use | TokenKind::Pub
        | TokenKind::Async | TokenKind::At => {
            let item_node = item::parse_item(s)?;
            let span = item_node.span();
            Ok(Stmt::Item {
                item: Box::new(item_node),
                span,
            })
        }

        // Everything else is an expression statement or a `:=` binding
        _ => parse_expr_or_let_stmt(s),
    }
}

/// Parse either an expression statement or a `:=` binding.
///
/// This handles the ambiguity between:
/// - `expr` (expression statement)
/// - `name := value` (immutable binding)
/// - `name: Type := value` (explicitly typed immutable binding)
/// - `(a, b) := tuple` (destructuring)
fn parse_expr_or_let_stmt(s: &mut TokenStream) -> Result<Stmt, ParseError> {
    let start = s.current_span();

    // Try destructuring tuple pattern: (a, b, c) := expr
    if s.check(&TokenKind::LParen) {
        let save = s.save();
        if let Ok(pattern) = crate::pat::parse_pattern(s) {
            if s.eat(&TokenKind::ColonEq) {
                let value = parse_expr(s)?;
                let span = s.span_from(start);
                return Ok(Stmt::Let {
                    pattern,
                    ty: None,
                    value,
                    span,
                });
            }
        }
        s.restore(save);
    }

    // Try struct destructuring: { name, age } := expr
    if s.check(&TokenKind::LBrace) {
        let save = s.save();
        if let Ok(pattern) = crate::pat::parse_pattern(s) {
            if s.eat(&TokenKind::ColonEq) {
                let value = parse_expr(s)?;
                let span = s.span_from(start);
                return Ok(Stmt::Let {
                    pattern,
                    ty: None,
                    value,
                    span,
                });
            }
        }
        s.restore(save);
    }

    // Parse expression
    let expr = parse_expr(s)?;

    // Check for `:=` (immutable binding)
    if s.check(&TokenKind::ColonEq) {
        s.advance();
        let value = parse_expr(s)?;
        let span = s.span_from(start);
        // The `expr` should be either an identifier or a typed identifier (`name: Type`)
        let (pattern, ty) = expr_to_let_pattern(expr)?;
        return Ok(Stmt::Let {
            pattern,
            ty,
            value,
            span,
        });
    }

    // Check for `: Type :=` (explicitly typed immutable binding)
    // This is already handled because the expression parser would have parsed
    // `name: Type` and then we hit `:=` above. But we handle the case where
    // the colon was consumed as part of a type annotation.

    let span = s.span_from(start);
    Ok(Stmt::Expr { expr, span })
}

/// Convert an expression (from the LHS of `:=`) into a binding pattern.
fn expr_to_let_pattern(expr: Expr) -> Result<(Pattern, Option<razen_ast::TypeExpr>), ParseError> {
    match expr {
        Expr::Ident { ident, span } => {
            if ident.name == "_" {
                Ok((Pattern::Wildcard { span }, None))
            } else {
                Ok((Pattern::Binding { name: ident, span }, None))
            }
        }
        _ => {
            let span = expr.span();
            Ok((
                Pattern::Binding {
                    name: Ident::new("_", span),
                    span,
                },
                None,
            ))
        }
    }
}

/// Parse statements inside a block `{ ... }`, returning (statements, optional tail expression).
pub fn parse_block_stmts(
    s: &mut TokenStream,
) -> Result<(Vec<Stmt>, Option<Box<Expr>>), ParseError> {
    let mut stmts = Vec::new();

    while !s.check(&TokenKind::RBrace) && !s.is_eof() {
        let stmt = parse_stmt(s)?;
        stmts.push(stmt);
    }

    // The last expression statement may be a tail expression (implicit return).
    // If the last statement is an Expr without a side-effect keyword, treat it as tail.
    let tail = if let Some(last) = stmts.last() {
        if let Stmt::Expr { .. } = last {
            if let Some(Stmt::Expr { expr, .. }) = stmts.pop() {
                Some(Box::new(expr))
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    Ok((stmts, tail))
}
