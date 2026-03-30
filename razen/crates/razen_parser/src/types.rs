//! Type expression parser.
//!
//! Parses type annotations like `int`, `vec[T]`, `map[str, int]`,
//! `(int, str)`, `|int| -> int`, `[float; 3]`, etc.

use razen_ast::ident::Ident;
use razen_ast::types::TypeExpr;
use razen_lexer::TokenKind;

use crate::error::ParseError;
use crate::input::TokenStream;

/// Parse a type expression.
pub fn parse_type(s: &mut TokenStream) -> Result<TypeExpr, ParseError> {
    let start = s.current_span();

    match s.peek_kind().clone() {
        // Closure type: |ParamType, ...| -> ReturnType
        TokenKind::Or => {
            s.advance(); // consume |
            let params = s.parse_comma_separated(&TokenKind::Or, |s| parse_type(s))?;
            s.expect(&TokenKind::Or)?;
            s.expect(&TokenKind::Arrow)?;
            let ret = parse_type(s)?;
            let span = s.span_from(start);
            Ok(TypeExpr::Closure {
                params,
                ret: Box::new(ret),
                span,
            })
        }

        // Tuple type: (Type, Type, ...)
        TokenKind::LParen => {
            s.advance();
            let elements = s.parse_comma_separated(&TokenKind::RParen, |s| parse_type(s))?;
            s.expect(&TokenKind::RParen)?;
            let span = s.span_from(start);
            // Single element in parens is just grouping, not a tuple
            if elements.len() == 1 {
                Ok(elements.into_iter().next().unwrap())
            } else {
                Ok(TypeExpr::Tuple { elements, span })
            }
        }

        // Array type: [Type; Size]
        TokenKind::LBracket => {
            s.advance(); // [
            let element = parse_type(s)?;
            s.expect(&TokenKind::Semi)?;
            let size = crate::expr::parse_expr(s)?;
            s.expect(&TokenKind::RBracket)?;
            let span = s.span_from(start);
            Ok(TypeExpr::Array {
                element: Box::new(element),
                size: Box::new(size),
                span,
            })
        }

        // Named / generic type
        TokenKind::Ident(ref name) => {
            let name_str = name.clone();
            let name_span = s.peek().span;

            // Check for built-in special types
            match name_str.as_str() {
                "void" => {
                    s.advance();
                    Ok(TypeExpr::Void { span: name_span })
                }
                "never" => {
                    s.advance();
                    Ok(TypeExpr::Never { span: name_span })
                }
                _ => {
                    let ident = Ident::new(name_str, name_span);
                    s.advance();

                    // Check for generic arguments: name[T, U]
                    if s.check(&TokenKind::LBracket) {
                        s.advance(); // [
                        let args =
                            s.parse_comma_separated(&TokenKind::RBracket, |s| parse_type(s))?;
                        s.expect(&TokenKind::RBracket)?;
                        let span = s.span_from(start);
                        Ok(TypeExpr::Generic {
                            name: ident,
                            args,
                            span,
                        })
                    } else {
                        Ok(TypeExpr::Named {
                            name: ident,
                            span: name_span,
                        })
                    }
                }
            }
        }

        // Self type
        TokenKind::SelfType => {
            s.advance();
            Ok(TypeExpr::SelfType { span: start })
        }

        // Reference type (unsafe context): &Type
        TokenKind::And => {
            s.advance();
            let inner = parse_type(s)?;
            let span = s.span_from(start);
            Ok(TypeExpr::Ref {
                inner: Box::new(inner),
                span,
            })
        }

        _ => Err(ParseError::expected(
            format!("expected type, found {:?}", s.peek_kind()),
            start,
            vec!["type".to_string()],
        )),
    }
}

/// Parse an optional type annotation: `: Type`.
/// Returns `None` if no colon is present.
pub fn parse_optional_type(s: &mut TokenStream) -> Result<Option<TypeExpr>, ParseError> {
    if s.check(&TokenKind::Colon) {
        s.advance(); // :
        Ok(Some(parse_type(s)?))
    } else {
        Ok(None)
    }
}

/// Parse a required type annotation: `: Type`.
pub fn parse_type_annotation(s: &mut TokenStream) -> Result<TypeExpr, ParseError> {
    s.expect(&TokenKind::Colon)?;
    parse_type(s)
}
