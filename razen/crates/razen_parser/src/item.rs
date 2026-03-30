//! Item / declaration parser.
//!
//! Parses top-level declarations: functions (act), structs, enums, traits,
//! impl blocks, type aliases, use declarations, attributes, and visibility.


use razen_ast::ident::Ident;
use razen_ast::item::*;
use razen_ast::lit::Literal;
use razen_ast::pat::Pattern;

use razen_lexer::TokenKind;

use crate::error::ParseError;
use crate::expr::parse_expr;
use crate::input::TokenStream;
use crate::stmt::parse_block_stmts;
use crate::types::{parse_optional_type, parse_type};

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Parse a single top-level item.
pub fn parse_item(s: &mut TokenStream) -> Result<Item, ParseError> {
    // Collect attributes
    let attrs = parse_attributes(s)?;

    // Parse visibility
    let vis = parse_visibility(s)?;

    let start = s.current_span();

    match s.peek_kind().clone() {
        TokenKind::Async => {
            s.advance();
            // async act ...
            if s.check(&TokenKind::Act) {
                let mut fndef = parse_fn_def(s, attrs, vis)?;
                fndef.is_async = true;
                Ok(Item::Function(fndef))
            } else {
                Err(ParseError::expected(
                    "expected `act` after `async`",
                    start,
                    vec!["act".to_string()],
                ))
            }
        }

        TokenKind::Act => {
            let fndef = parse_fn_def(s, attrs, vis)?;
            Ok(Item::Function(fndef))
        }

        TokenKind::Struct => {
            let sdef = parse_struct_def(s, attrs, vis)?;
            Ok(Item::Struct(sdef))
        }

        TokenKind::Enum => {
            let edef = parse_enum_def(s, attrs, vis)?;
            Ok(Item::Enum(edef))
        }

        TokenKind::Trait => {
            let tdef = parse_trait_def(s, attrs, vis)?;
            Ok(Item::Trait(tdef))
        }

        TokenKind::Impl => {
            let iblock = parse_impl_block(s, attrs)?;
            Ok(Item::Impl(iblock))
        }

        TokenKind::Alias => {
            let alias = parse_type_alias(s, attrs, vis)?;
            Ok(Item::TypeAlias(alias))
        }

        TokenKind::Use => {
            let udef = parse_use_decl(s, vis)?;
            Ok(Item::Use(udef))
        }

        TokenKind::Const => {
            s.advance();
            let (name, name_span) = s.expect_ident()?;
            let ident = Ident::new(name, name_span);
            s.expect(&TokenKind::Colon)?;
            let ty = parse_type(s)?;
            s.expect(&TokenKind::Eq)?;
            let value = parse_expr(s)?;
            let span = s.span_from(start);
            Ok(Item::Const(ConstDef {
                attrs,
                vis,
                name: ident,
                ty,
                value,
                span,
            }))
        }

        TokenKind::Shared => {
            s.advance();
            let (name, name_span) = s.expect_ident()?;
            let ident = Ident::new(name, name_span);
            let ty = parse_optional_type(s)?;
            s.expect(&TokenKind::Eq)?;
            let value = parse_expr(s)?;
            let span = s.span_from(start);
            Ok(Item::Shared(SharedDef {
                attrs,
                vis,
                name: ident,
                ty,
                value,
                span,
            }))
        }

        _ => Err(ParseError::expected(
            format!("expected item declaration, found {:?}", s.peek_kind()),
            start,
            vec![
                "act", "struct", "enum", "trait", "impl", "alias", "use", "const",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
        )),
    }
}

// ---------------------------------------------------------------------------
// Attributes
// ---------------------------------------------------------------------------

/// Parse zero or more `@attribute` annotations.
fn parse_attributes(s: &mut TokenStream) -> Result<Vec<Attribute>, ParseError> {
    let mut attrs = Vec::new();
    while s.check(&TokenKind::At) {
        let start = s.current_span();
        s.advance(); // @
        let (name, name_span) = s.expect_ident()?;
        let name = Ident::new(name, name_span);

        // Optional arguments: @name[arg1, arg2, ...]
        let args = if s.check(&TokenKind::LBracket) {
            s.advance();
            let args = s.parse_comma_separated(&TokenKind::RBracket, |s| {
                parse_attribute_arg(s)
            })?;
            s.expect(&TokenKind::RBracket)?;
            args
        } else {
            vec![]
        };

        let span = s.span_from(start);
        attrs.push(Attribute { name, args, span });
    }
    Ok(attrs)
}

fn parse_attribute_arg(s: &mut TokenStream) -> Result<AttributeArg, ParseError> {
    let start = s.current_span();

    // String literal argument
    if let TokenKind::String(ref val) = s.peek_kind().clone() {
        let val = val.clone();
        s.advance();
        return Ok(AttributeArg::Literal(Literal::Str {
            value: val,
            span: start,
        }));
    }

    // Identifier — possibly key: value
    let (name, name_span) = s.expect_ident()?;
    let ident = Ident::new(name, name_span);

    if s.check(&TokenKind::Colon) {
        s.advance();
        let value = parse_expr(s)?;
        let span = s.span_from(start);
        Ok(AttributeArg::KeyValue {
            key: ident,
            value,
            span,
        })
    } else {
        Ok(AttributeArg::Ident(ident))
    }
}

// ---------------------------------------------------------------------------
// Visibility
// ---------------------------------------------------------------------------

fn parse_visibility(s: &mut TokenStream) -> Result<Visibility, ParseError> {
    if !s.check(&TokenKind::Pub) {
        return Ok(Visibility::Private);
    }

    s.advance(); // consume `pub`

    // Check for pub(pkg) or pub(mod)
    if s.check(&TokenKind::LParen) {
        s.advance();
        let (kind_name, _) = s.expect_ident()?;
        s.expect(&TokenKind::RParen)?;
        match kind_name.as_str() {
            "pkg" => Ok(Visibility::PublicPkg),
            "mod" => Ok(Visibility::PublicMod),
            _ => Ok(Visibility::Public), // Fallback
        }
    } else {
        Ok(Visibility::Public)
    }
}

// ---------------------------------------------------------------------------
// Function definition
// ---------------------------------------------------------------------------

fn parse_fn_def(
    s: &mut TokenStream,
    attrs: Vec<Attribute>,
    vis: Visibility,
) -> Result<FnDef, ParseError> {
    let start = s.current_span();
    s.expect(&TokenKind::Act)?;
    let (name, name_span) = s.expect_ident()?;
    let name = Ident::new(name, name_span);

    // Optional generic params: [T, U: Trait]
    let generic_params = parse_generic_params(s)?;

    // Parameters: (param: Type, ...)
    s.expect(&TokenKind::LParen)?;
    let params = s.parse_comma_separated(&TokenKind::RParen, |s| parse_fn_param(s))?;
    s.expect(&TokenKind::RParen)?;

    // Optional return type
    let return_type = if !s.check(&TokenKind::LBrace)
        && !s.check(&TokenKind::Arrow)
        && !s.check(&TokenKind::Where)
        && !s.is_eof()
    {
        // It's a type if next token starts a type expression
        match s.peek_kind() {
            TokenKind::Ident(_) | TokenKind::LParen | TokenKind::LBracket
            | TokenKind::SelfType | TokenKind::Or => Some(parse_type(s)?),
            _ => None,
        }
    } else {
        None
    };

    // Optional where clause
    let where_clause = parse_where_clause(s)?;

    // Body
    let body = if s.check(&TokenKind::Arrow) {
        // Expression body: -> expr
        s.advance();
        let expr = parse_expr(s)?;
        FnBody::Expr(Box::new(expr))
    } else if s.check(&TokenKind::LBrace) {
        // Block body
        let block_start = s.current_span();
        s.expect(&TokenKind::LBrace)?;
        let (stmts, tail) = parse_block_stmts(s)?;
        s.expect(&TokenKind::RBrace)?;
        let span = s.span_from(block_start);
        FnBody::Block { stmts, tail, span }
    } else {
        // No body (trait method signature)
        FnBody::None
    };

    let span = s.span_from(start);
    Ok(FnDef {
        attrs,
        vis,
        is_async: false,
        name,
        generic_params,
        params,
        return_type,
        where_clause,
        body,
        span,
    })
}

fn parse_fn_param(s: &mut TokenStream) -> Result<FnParam, ParseError> {
    let start = s.current_span();

    // Check for `mut` modifier
    let is_mut = s.eat(&TokenKind::Mut);

    // `self` parameter
    if s.check(&TokenKind::SelfKw) {
        let self_span = s.current_span();
        s.advance();
        return Ok(FnParam {
            is_mut,
            pattern: Pattern::Binding {
                name: Ident::new("self", self_span),
                span: self_span,
            },
            ty: None,
            span: s.span_from(start),
        });
    }

    // `_` parameter (ignored)
    if s.check(&TokenKind::Underscore) {
        let us_span = s.current_span();
        s.advance();
        let ty = if s.check(&TokenKind::Colon) {
            s.advance();
            Some(parse_type(s)?)
        } else {
            None
        };
        return Ok(FnParam {
            is_mut,
            pattern: Pattern::Wildcard { span: us_span },
            ty,
            span: s.span_from(start),
        });
    }

    // Normal parameter: name: Type
    let (name, name_span) = s.expect_ident()?;
    let ident = Ident::new(name, name_span);

    let ty = if s.check(&TokenKind::Colon) {
        s.advance();
        Some(parse_type(s)?)
    } else {
        None
    };

    let span = s.span_from(start);
    Ok(FnParam {
        is_mut,
        pattern: Pattern::Binding {
            name: ident,
            span: name_span,
        },
        ty,
        span,
    })
}

// ---------------------------------------------------------------------------
// Generics and where clause
// ---------------------------------------------------------------------------

fn parse_generic_params(s: &mut TokenStream) -> Result<Vec<GenericParam>, ParseError> {
    if !s.check(&TokenKind::LBracket) {
        return Ok(vec![]);
    }
    s.advance();
    let params = s.parse_comma_separated(&TokenKind::RBracket, |s| {
        let start = s.current_span();
        let (name, name_span) = s.expect_ident()?;
        let ident = Ident::new(name, name_span);

        let mut bounds = Vec::new();
        if s.check(&TokenKind::Colon) {
            s.advance();
            // Parse bounds: Trait + Trait + ...
            bounds.push(parse_type(s)?);
            while s.eat(&TokenKind::Plus) {
                bounds.push(parse_type(s)?);
            }
        }
        let span = s.span_from(start);
        Ok(GenericParam {
            name: ident,
            bounds,
            span,
        })
    })?;
    s.expect(&TokenKind::RBracket)?;
    Ok(params)
}

fn parse_where_clause(s: &mut TokenStream) -> Result<Vec<WhereBound>, ParseError> {
    if !s.check(&TokenKind::Where) {
        return Ok(vec![]);
    }
    s.advance();

    let mut bounds = Vec::new();
    loop {
        let start = s.current_span();
        if s.check(&TokenKind::LBrace) || s.check(&TokenKind::Arrow) || s.is_eof() {
            break;
        }
        let (name, name_span) = s.expect_ident()?;
        let ident = Ident::new(name, name_span);
        s.expect(&TokenKind::Colon)?;

        let mut type_bounds = vec![parse_type(s)?];
        while s.eat(&TokenKind::Plus) {
            type_bounds.push(parse_type(s)?);
        }

        let span = s.span_from(start);
        bounds.push(WhereBound {
            param: ident,
            bounds: type_bounds,
            span,
        });

        if !s.eat(&TokenKind::Comma) {
            break;
        }
    }
    Ok(bounds)
}

// ---------------------------------------------------------------------------
// Struct definition
// ---------------------------------------------------------------------------

fn parse_struct_def(
    s: &mut TokenStream,
    attrs: Vec<Attribute>,
    vis: Visibility,
) -> Result<StructDef, ParseError> {
    let start = s.current_span();
    s.expect(&TokenKind::Struct)?;
    let (name, name_span) = s.expect_ident()?;
    let name = Ident::new(name, name_span);

    let generic_params = parse_generic_params(s)?;
    let where_clause = parse_where_clause(s)?;

    let kind = if s.check(&TokenKind::LParen) {
        // Tuple struct: struct Name(Type, Type)
        s.advance();
        let fields = s.parse_comma_separated(&TokenKind::RParen, |s| parse_type(s))?;
        s.expect(&TokenKind::RParen)?;
        StructKind::Tuple { fields }
    } else if s.check(&TokenKind::LBrace) {
        // Named struct: struct Name { field: Type, ... }
        s.advance();
        let fields = s.parse_comma_separated(&TokenKind::RBrace, |s| {
            let field_start = s.current_span();
            let field_vis = parse_visibility(s)?;
            let (fname, fspan) = s.expect_ident()?;
            let field_name = Ident::new(fname, fspan);
            s.expect(&TokenKind::Colon)?;
            let fty = parse_type(s)?;
            let field_span = s.span_from(field_start);
            Ok(StructField {
                vis: field_vis,
                name: field_name,
                ty: fty,
                span: field_span,
            })
        })?;
        s.expect(&TokenKind::RBrace)?;
        StructKind::Named { fields }
    } else {
        // Unit struct
        StructKind::Unit
    };

    let span = s.span_from(start);
    Ok(StructDef {
        attrs,
        vis,
        name,
        generic_params,
        where_clause,
        kind,
        span,
    })
}

// ---------------------------------------------------------------------------
// Enum definition
// ---------------------------------------------------------------------------

fn parse_enum_def(
    s: &mut TokenStream,
    attrs: Vec<Attribute>,
    vis: Visibility,
) -> Result<EnumDef, ParseError> {
    let start = s.current_span();
    s.expect(&TokenKind::Enum)?;
    let (name, name_span) = s.expect_ident()?;
    let name = Ident::new(name, name_span);

    let generic_params = parse_generic_params(s)?;
    let where_clause = parse_where_clause(s)?;

    s.expect(&TokenKind::LBrace)?;
    let variants = s.parse_comma_separated(&TokenKind::RBrace, |s| {
        let v_start = s.current_span();
        let v_attrs = parse_attributes(s)?;
        let (v_name, v_span) = s.expect_ident()?;
        let v_ident = Ident::new(v_name, v_span);

        let kind = if s.check(&TokenKind::LParen) {
            // Positional: Variant(Type, Type)
            s.advance();
            let fields = s.parse_comma_separated(&TokenKind::RParen, |s| parse_type(s))?;
            s.expect(&TokenKind::RParen)?;
            EnumVariantKind::Positional { fields }
        } else if s.check(&TokenKind::LBrace) {
            // Named: Variant { field: Type, ... }
            s.advance();
            let fields = s.parse_comma_separated(&TokenKind::RBrace, |s| {
                let fs = s.current_span();
                let (fname, fspan) = s.expect_ident()?;
                let field_name = Ident::new(fname, fspan);
                s.expect(&TokenKind::Colon)?;
                let fty = parse_type(s)?;
                let fsp = s.span_from(fs);
                Ok(StructField {
                    vis: Visibility::Private,
                    name: field_name,
                    ty: fty,
                    span: fsp,
                })
            })?;
            s.expect(&TokenKind::RBrace)?;
            EnumVariantKind::Named { fields }
        } else {
            EnumVariantKind::Unit
        };

        let vspan = s.span_from(v_start);
        Ok(EnumVariant {
            attrs: v_attrs,
            name: v_ident,
            kind,
            span: vspan,
        })
    })?;
    s.expect(&TokenKind::RBrace)?;

    let span = s.span_from(start);
    Ok(EnumDef {
        attrs,
        vis,
        name,
        generic_params,
        where_clause,
        variants,
        span,
    })
}

// ---------------------------------------------------------------------------
// Trait definition
// ---------------------------------------------------------------------------

fn parse_trait_def(
    s: &mut TokenStream,
    attrs: Vec<Attribute>,
    vis: Visibility,
) -> Result<TraitDef, ParseError> {
    let start = s.current_span();
    s.expect(&TokenKind::Trait)?;
    let (name, name_span) = s.expect_ident()?;
    let name = Ident::new(name, name_span);

    let generic_params = parse_generic_params(s)?;

    // Supertraits: trait Ord: Eq
    let supertraits = if s.check(&TokenKind::Colon) {
        s.advance();
        let mut traits = vec![parse_type(s)?];
        while s.eat(&TokenKind::Plus) {
            traits.push(parse_type(s)?);
        }
        traits
    } else {
        vec![]
    };

    let where_clause = parse_where_clause(s)?;

    s.expect(&TokenKind::LBrace)?;
    let mut methods = Vec::new();
    while !s.check(&TokenKind::RBrace) && !s.is_eof() {
        let method_attrs = parse_attributes(s)?;
        let method_vis = parse_visibility(s)?;
        let fndef = parse_fn_def(s, method_attrs, method_vis)?;
        methods.push(fndef);
    }
    s.expect(&TokenKind::RBrace)?;

    let span = s.span_from(start);
    Ok(TraitDef {
        attrs,
        vis,
        name,
        generic_params,
        supertraits,
        where_clause,
        methods,
        span,
    })
}

// ---------------------------------------------------------------------------
// Impl block
// ---------------------------------------------------------------------------

fn parse_impl_block(
    s: &mut TokenStream,
    attrs: Vec<Attribute>,
) -> Result<ImplBlock, ParseError> {
    let start = s.current_span();
    s.expect(&TokenKind::Impl)?;

    let generic_params = parse_generic_params(s)?;

    // Parse the first type
    let first_type = parse_type(s)?;

    // Check for `for Type` (trait impl)
    let (trait_name, target) = if matches!(s.peek_kind(), TokenKind::Ident(n) if n == "for") {
        s.advance(); // consume `for`
        let target = parse_type(s)?;
        (Some(first_type), target)
    } else {
        (None, first_type)
    };

    let where_clause = parse_where_clause(s)?;

    s.expect(&TokenKind::LBrace)?;
    let mut methods = Vec::new();
    while !s.check(&TokenKind::RBrace) && !s.is_eof() {
        let method_attrs = parse_attributes(s)?;
        let method_vis = parse_visibility(s)?;
        let fndef = parse_fn_def(s, method_attrs, method_vis)?;
        methods.push(fndef);
    }
    s.expect(&TokenKind::RBrace)?;

    let span = s.span_from(start);
    Ok(ImplBlock {
        attrs,
        generic_params,
        trait_name,
        target,
        where_clause,
        methods,
        span,
    })
}

// ---------------------------------------------------------------------------
// Type alias
// ---------------------------------------------------------------------------

fn parse_type_alias(
    s: &mut TokenStream,
    attrs: Vec<Attribute>,
    vis: Visibility,
) -> Result<TypeAliasDef, ParseError> {
    let start = s.current_span();
    s.expect(&TokenKind::Alias)?;
    let (name, name_span) = s.expect_ident()?;
    let name = Ident::new(name, name_span);

    let generic_params = parse_generic_params(s)?;

    s.expect(&TokenKind::Eq)?;
    let ty = parse_type(s)?;

    let span = s.span_from(start);
    Ok(TypeAliasDef {
        attrs,
        vis,
        name,
        generic_params,
        ty,
        span,
    })
}

// ---------------------------------------------------------------------------
// Use declaration
// ---------------------------------------------------------------------------

fn parse_use_decl(s: &mut TokenStream, vis: Visibility) -> Result<UseDef, ParseError> {
    let start = s.current_span();
    s.expect(&TokenKind::Use)?;

    // Parse module path: name.name.name
    let mut path = Vec::new();

    // Handle root import: ~.path
    if s.check(&TokenKind::Tilde) {
        s.advance();
        path.push(Ident::new("~", s.prev_span()));
        s.expect(&TokenKind::Dot)?;
    }

    let (first_name, first_span) = s.expect_ident()?;
    path.push(Ident::new(first_name, first_span));

    while s.check(&TokenKind::Dot) {
        s.advance();
        let (seg_name, seg_span) = s.expect_ident()?;
        path.push(Ident::new(seg_name, seg_span));
    }

    // Determine kind
    let kind = if s.check(&TokenKind::LBrace) {
        // use module { Item1, Item2 as Alias }
        s.advance();
        let items = s.parse_comma_separated(&TokenKind::RBrace, |s| {
            let item_start = s.current_span();
            let (item_name, item_span) = s.expect_ident()?;
            let item_ident = Ident::new(item_name, item_span);
            let alias = if s.eat(&TokenKind::As) {
                let (alias_name, alias_span) = s.expect_ident()?;
                Some(Ident::new(alias_name, alias_span))
            } else {
                None
            };
            let span = s.span_from(item_start);
            Ok(UseItem {
                name: item_ident,
                alias,
                span,
            })
        })?;
        s.expect(&TokenKind::RBrace)?;
        UseKind::Items(items)
    } else if s.eat(&TokenKind::As) {
        // use module as alias
        let (alias_name, alias_span) = s.expect_ident()?;
        UseKind::Alias(Ident::new(alias_name, alias_span))
    } else {
        UseKind::Module
    };

    let span = s.span_from(start);
    Ok(UseDef {
        vis,
        path,
        kind,
        span,
    })
}
