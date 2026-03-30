//! Input bridge between razen_lexer tokens and winnow combinators.
//!
//! This module provides helper functions that pattern-match on `TokenKind`
//! and produce winnow-compatible parsers operating on `&[Token]`.

use razen_lexer::{Token, TokenKind, Span};
use crate::error::ParseError;

/// The parser state — a simple index into a token slice.
///
/// We use a hand-rolled recursive-descent parser that consumes from this state,
/// rather than winnow's `TokenSlice`, because our `TokenKind` contains non-Copy
/// variants (String). This approach gives us full control over error recovery
/// and span tracking while still being production-quality.
#[derive(Debug, Clone)]
pub struct TokenStream<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl<'a> TokenStream<'a> {
    /// Create a new token stream from a slice of tokens.
    pub fn new(tokens: &'a [Token]) -> Self {
        Self { tokens, pos: 0 }
    }

    /// Peek at the current token without consuming it.
    pub fn peek(&self) -> &Token {
        &self.tokens[self.pos.min(self.tokens.len() - 1)]
    }

    /// Peek at the token `offset` positions ahead.
    pub fn peek_ahead(&self, offset: usize) -> &Token {
        let idx = (self.pos + offset).min(self.tokens.len() - 1);
        &self.tokens[idx]
    }

    /// Peek at the kind of the current token.
    pub fn peek_kind(&self) -> &TokenKind {
        &self.peek().kind
    }

    /// Check if the current token matches a given kind.
    pub fn check(&self, kind: &TokenKind) -> bool {
        self.peek_kind() == kind
    }

    /// Check if we are at end of input.
    pub fn is_eof(&self) -> bool {
        matches!(self.peek_kind(), TokenKind::Eof)
    }

    /// Consume the current token and advance.
    pub fn advance(&mut self) -> &Token {
        let tok = &self.tokens[self.pos.min(self.tokens.len() - 1)];
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    /// Consume a token of the expected kind, or return an error.
    pub fn expect(&mut self, expected: &TokenKind) -> Result<&Token, ParseError> {
        let tok = self.peek();
        if &tok.kind == expected {
            Ok(self.advance())
        } else {
            Err(ParseError::expected(
                format!("expected {:?}, found {:?}", expected, tok.kind),
                tok.span,
                vec![format!("{:?}", expected)],
            ))
        }
    }

    /// Consume a token if it matches the expected kind.
    /// Returns `true` if consumed, `false` otherwise.
    pub fn eat(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// Consume an identifier token and return its name and span.
    pub fn expect_ident(&mut self) -> Result<(String, Span), ParseError> {
        let tok = self.peek().clone();
        match &tok.kind {
            TokenKind::Ident(name) => {
                let name = name.clone();
                let span = tok.span;
                self.advance();
                Ok((name, span))
            }
            _ => Err(ParseError::expected(
                format!("expected identifier, found {:?}", tok.kind),
                tok.span,
                vec!["identifier".to_string()],
            )),
        }
    }

    /// Get the span of the current token.
    pub fn current_span(&self) -> Span {
        self.peek().span
    }

    /// Get the span of the previously consumed token.
    pub fn prev_span(&self) -> Span {
        if self.pos > 0 {
            self.tokens[self.pos - 1].span
        } else {
            Span::default()
        }
    }

    /// Create a span from `start` to the end of the previously consumed token.
    pub fn span_from(&self, start: Span) -> Span {
        let end = self.prev_span();
        Span::new(start.start, end.end)
    }

    /// Save the current position for backtracking.
    pub fn save(&self) -> usize {
        self.pos
    }

    /// Restore a previously saved position.
    pub fn restore(&mut self, pos: usize) {
        self.pos = pos;
    }

    /// Check if the current token is a specific keyword identifier.
    pub fn check_ident(&self, name: &str) -> bool {
        matches!(self.peek_kind(), TokenKind::Ident(n) if n == name)
    }

    /// Consume a comma-separated list of items, stopping at the given
    /// closing delimiter. Allows optional trailing comma.
    pub fn parse_comma_separated<T>(
        &mut self,
        close: &TokenKind,
        mut parse_item: impl FnMut(&mut Self) -> Result<T, ParseError>,
    ) -> Result<Vec<T>, ParseError> {
        let mut items = Vec::new();
        while !self.check(close) && !self.is_eof() {
            items.push(parse_item(self)?);
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        Ok(items)
    }
}
