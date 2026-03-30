//! # Razen Parser
//!
//! This crate implements a production-quality recursive-descent parser for the
//! Razen programming language. It consumes a token stream from `razen_lexer` and
//! produces a complete AST defined in `razen_ast`.
//!
//! ## Architecture
//!
//! The parser uses Pratt parsing (precedence climbing) for expressions and
//! recursive descent for statements and declarations. It operates on a
//! `TokenStream` wrapper around `&[Token]` for clean error recovery and
//! span tracking.
//!
//! ## Usage
//!
//! ```rust
//! use razen_parser::parse;
//!
//! let source = r#"act main() void { println("hello") }"#;
//! match parse(source) {
//!     Ok(module) => println!("Parsed {} items", module.items.len()),
//!     Err(errors) => {
//!         for e in &errors {
//!             eprintln!("{}", e);
//!         }
//!     }
//! }
//! ```

pub mod error;
pub mod input;
pub mod types;
pub mod pat;
pub mod expr;
pub mod stmt;
pub mod item;

#[cfg(test)]
mod tests;

pub use error::ParseError;
pub use razen_ast::Module;

use input::TokenStream;

/// Parse a Razen source string into a `Module` AST.
///
/// This is the primary public API. It tokenizes the source, then parses
/// the token stream into a complete AST.
///
/// Returns `Ok(Module)` on success, or `Err(Vec<ParseError>)` with all
/// errors encountered.
pub fn parse(source: &str) -> Result<Module, Vec<ParseError>> {
    let tokens = razen_lexer::tokenize(source);
    parse_tokens(&tokens)
}

/// Parse a pre-tokenized slice into a `Module` AST.
pub fn parse_tokens(tokens: &[razen_lexer::Token]) -> Result<Module, Vec<ParseError>> {
    let mut stream = TokenStream::new(tokens);
    let mut items = Vec::new();
    let mut errors = Vec::new();

    let start = stream.current_span();

    while !stream.is_eof() {
        // Skip doc comments at the top level
        if matches!(stream.peek_kind(), razen_lexer::TokenKind::DocComment(_)) {
            stream.advance();
            continue;
        }

        match item::parse_item(&mut stream) {
            Ok(item_node) => items.push(item_node),
            Err(e) => {
                errors.push(e);
                // Error recovery: skip to the next plausible item boundary
                recover_to_next_item(&mut stream);
            }
        }
    }

    if errors.is_empty() {
        let span = stream.span_from(start);
        Ok(Module::new(items, span))
    } else {
        Err(errors)
    }
}

/// Panic recovery: skip tokens until we find something that looks like
/// the start of a new top-level item.
fn recover_to_next_item(s: &mut TokenStream) {
    loop {
        match s.peek_kind() {
            razen_lexer::TokenKind::Eof => break,
            razen_lexer::TokenKind::Act
            | razen_lexer::TokenKind::Struct
            | razen_lexer::TokenKind::Enum
            | razen_lexer::TokenKind::Trait
            | razen_lexer::TokenKind::Impl
            | razen_lexer::TokenKind::Alias
            | razen_lexer::TokenKind::Use
            | razen_lexer::TokenKind::Pub
            | razen_lexer::TokenKind::Const
            | razen_lexer::TokenKind::Shared
            | razen_lexer::TokenKind::Async
            | razen_lexer::TokenKind::At => break,
            _ => {
                s.advance();
            }
        }
    }
}
