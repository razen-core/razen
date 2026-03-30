//! Pattern parser.
//!
//! Parses patterns for `match` arms, `if let`, `loop let`, and destructuring.

use razen_ast::ident::Ident;
use razen_ast::lit::Literal;
use razen_ast::pat::{Pattern, StructPatternField};
use razen_lexer::TokenKind;

use crate::error::ParseError;
use crate::input::TokenStream;

/// Parse a pattern, including or-patterns.
pub fn parse_pattern(s: &mut TokenStream) -> Result<Pattern, ParseError> {
    let start = s.current_span();
    let first = parse_single_pattern(s)?;

    // Check for or-pattern: `pattern | pattern | ...`
    if s.check(&TokenKind::Or) {
        let mut patterns = vec![first];
        while s.eat(&TokenKind::Or) {
            patterns.push(parse_single_pattern(s)?);
        }
        let span = s.span_from(start);
        Ok(Pattern::Or { patterns, span })
    } else {
        Ok(first)
    }
}

/// Parse a single (non-or) pattern.
fn parse_single_pattern(s: &mut TokenStream) -> Result<Pattern, ParseError> {
    let start = s.current_span();

    match s.peek_kind().clone() {
        // Wildcard: _
        TokenKind::Underscore => {
            s.advance();
            Ok(Pattern::Wildcard { span: start })
        }

        // Boolean literal patterns
        TokenKind::True => {
            s.advance();
            Ok(Pattern::Literal {
                lit: Literal::Bool {
                    value: true,
                    span: start,
                },
                span: start,
            })
        }
        TokenKind::False => {
            s.advance();
            Ok(Pattern::Literal {
                lit: Literal::Bool {
                    value: false,
                    span: start,
                },
                span: start,
            })
        }

        // Integer literal pattern
        TokenKind::Int(ref raw) => {
            let raw = raw.clone();
            s.advance();
            let lit = Literal::Int {
                raw,
                span: start,
            };
            // Check for range pattern: `0..=9`
            maybe_range_pattern(s, lit, start)
        }

        // Float literal pattern
        TokenKind::Float(ref raw) => {
            let raw = raw.clone();
            s.advance();
            let lit = Literal::Float {
                raw,
                span: start,
            };
            Ok(Pattern::Literal { lit, span: start })
        }

        // String literal pattern
        TokenKind::String(ref val) => {
            let val = val.clone();
            s.advance();
            Ok(Pattern::Literal {
                lit: Literal::Str {
                    value: val,
                    span: start,
                },
                span: start,
            })
        }

        // Char literal pattern
        TokenKind::Char(ch) => {
            s.advance();
            let lit = Literal::Char {
                value: ch,
                span: start,
            };
            // Check for range pattern: `'a'..='z'`
            maybe_range_pattern(s, lit, start)
        }

        // Tuple pattern: (a, b, c)
        TokenKind::LParen => {
            s.advance();
            let elements =
                s.parse_comma_separated(&TokenKind::RParen, |s| parse_pattern(s))?;
            s.expect(&TokenKind::RParen)?;
            let span = s.span_from(start);
            Ok(Pattern::Tuple { elements, span })
        }

        // Struct destructuring pattern: { name, age, _ }
        TokenKind::LBrace => {
            s.advance();
            let mut fields = Vec::new();
            let mut has_rest = false;
            while !s.check(&TokenKind::RBrace) && !s.is_eof() {
                if s.check(&TokenKind::Underscore) {
                    s.advance();
                    has_rest = true;
                    // Allow trailing comma
                    s.eat(&TokenKind::Comma);
                    break;
                }
                let (field_name, field_span) = s.expect_ident()?;
                let ident = Ident::new(field_name, field_span);
                // Check for rename: `name: alias` or nested pattern: `name: pattern`
                let (rename, pattern) = if s.check(&TokenKind::Colon) {
                    s.advance();
                    // Try parsing as an identifier rename
                    if let TokenKind::Ident(ref _n) = s.peek_kind().clone() {
                        let (alias_name, alias_span) = s.expect_ident()?;
                        (Some(Ident::new(alias_name, alias_span)), None)
                    } else {
                        (None, None)
                    }
                } else {
                    (None, None)
                };

                let fspan = s.span_from(field_span);
                fields.push(StructPatternField {
                    name: ident,
                    rename,
                    pattern,
                    span: fspan,
                });
                if !s.eat(&TokenKind::Comma) {
                    break;
                }
            }
            s.expect(&TokenKind::RBrace)?;
            let span = s.span_from(start);
            Ok(Pattern::Struct {
                fields,
                has_rest,
                span,
            })
        }

        // Identifier-based patterns: binding, enum variant, some/none/ok/err
        TokenKind::Ident(ref name) => {
            let name_clone = name.clone();
            match name_clone.as_str() {
                "some" => {
                    s.advance();
                    s.expect(&TokenKind::LParen)?;
                    let inner = parse_pattern(s)?;
                    s.expect(&TokenKind::RParen)?;
                    let span = s.span_from(start);
                    Ok(Pattern::Some {
                        inner: Box::new(inner),
                        span,
                    })
                }
                "none" => {
                    s.advance();
                    Ok(Pattern::None { span: start })
                }
                "ok" => {
                    s.advance();
                    s.expect(&TokenKind::LParen)?;
                    let inner = parse_pattern(s)?;
                    s.expect(&TokenKind::RParen)?;
                    let span = s.span_from(start);
                    Ok(Pattern::Ok {
                        inner: Box::new(inner),
                        span,
                    })
                }
                "err" => {
                    s.advance();
                    s.expect(&TokenKind::LParen)?;
                    let inner = parse_pattern(s)?;
                    s.expect(&TokenKind::RParen)?;
                    let span = s.span_from(start);
                    Ok(Pattern::Err {
                        inner: Box::new(inner),
                        span,
                    })
                }
                _ => {
                    // Could be a simple binding or an enum path pattern
                    let ident = Ident::new(name_clone, start);
                    s.advance();

                    // Check for path: Ident.Variant or Ident.Variant(...)
                    if s.check(&TokenKind::Dot) {
                        let mut path = vec![ident];
                        while s.eat(&TokenKind::Dot) {
                            let (seg_name, seg_span) = s.expect_ident()?;
                            path.push(Ident::new(seg_name, seg_span));
                        }

                        // Named fields: Event.Click { x, y }
                        if s.check(&TokenKind::LBrace) {
                            s.advance();
                            let mut fields = Vec::new();
                            let mut has_rest = false;
                            while !s.check(&TokenKind::RBrace) && !s.is_eof() {
                                if s.check(&TokenKind::Underscore) {
                                    s.advance();
                                    has_rest = true;
                                    s.eat(&TokenKind::Comma);
                                    break;
                                }
                                let (fname, fspan) = s.expect_ident()?;
                                let field_ident = Ident::new(fname, fspan);

                                let (rename, pattern) = if s.check(&TokenKind::Colon) {
                                    s.advance();
                                    // Check if it's an identifier (rename) or a pattern
                                    if let TokenKind::Ident(ref _n) = s.peek_kind().clone() {
                                        let _save = s.save();
                                        let (rname, rspan) = s.expect_ident()?;
                                        // Simple rename
                                        (Some(Ident::new(rname, rspan)), None)
                                    } else {
                                        // Value pattern (like `shift: false`)
                                        let pat = parse_single_pattern(s)?;
                                        (None, Some(pat))
                                    }
                                } else {
                                    (None, None)
                                };

                                let field_span = s.span_from(fspan);
                                fields.push(StructPatternField {
                                    name: field_ident,
                                    rename,
                                    pattern,
                                    span: field_span,
                                });
                                if !s.eat(&TokenKind::Comma) {
                                    break;
                                }
                            }
                            s.expect(&TokenKind::RBrace)?;
                            let span = s.span_from(start);
                            Ok(Pattern::EnumNamed {
                                path,
                                fields,
                                has_rest,
                                span,
                            })
                        }
                        // Positional args: Shape.Circle(r)
                        else if s.check(&TokenKind::LParen) {
                            s.advance();
                            let args = s.parse_comma_separated(&TokenKind::RParen, |s| {
                                parse_pattern(s)
                            })?;
                            s.expect(&TokenKind::RParen)?;
                            let span = s.span_from(start);
                            Ok(Pattern::EnumPositional { path, args, span })
                        }
                        // Unit variant: Direction.North
                        else {
                            let span = s.span_from(start);
                            Ok(Pattern::EnumUnit { path, span })
                        }
                    }
                    // Tuple struct pattern: UserId(n)
                    else if s.check(&TokenKind::LParen) {
                        s.advance();
                        let fields = s.parse_comma_separated(&TokenKind::RParen, |s| {
                            parse_pattern(s)
                        })?;
                        s.expect(&TokenKind::RParen)?;
                        let span = s.span_from(start);
                        Ok(Pattern::TupleStruct {
                            name: ident,
                            fields,
                            span,
                        })
                    }
                    // Simple binding
                    else {
                        Ok(Pattern::Binding {
                            name: ident,
                            span: start,
                        })
                    }
                }
            }
        }

        _ => Err(ParseError::expected(
            format!("expected pattern, found {:?}", s.peek_kind()),
            start,
            vec!["pattern".to_string()],
        )),
    }
}

/// If the next token is `..` or `..=`, turn the literal into a range pattern.
fn maybe_range_pattern(
    s: &mut TokenStream,
    start_lit: Literal,
    start_span: razen_lexer::Span,
) -> Result<Pattern, ParseError> {
    if s.check(&TokenKind::DotDotEq) || s.check(&TokenKind::DotDot) {
        let inclusive = s.check(&TokenKind::DotDotEq);
        s.advance();
        let end_expr = crate::expr::parse_primary(s)?;
        let start_expr = crate::expr::lit_to_expr(start_lit.clone());
        let span = s.span_from(start_span);
        Ok(Pattern::Range {
            start: Box::new(start_expr),
            end: Box::new(end_expr),
            inclusive,
            span,
        })
    } else {
        Ok(Pattern::Literal {
            lit: start_lit,
            span: start_span,
        })
    }
}
