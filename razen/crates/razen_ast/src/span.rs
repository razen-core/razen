//! Span re-export and helpers.

/// A byte-offset range in source code.
///
/// Re-exported from `razen_lexer` so that every AST node can carry location
/// information without an additional dependency.
pub use razen_lexer::Span;

/// Helper to merge two spans into one that covers both.
pub fn merge_spans(a: Span, b: Span) -> Span {
    Span::new(a.start.min(b.start), a.end.max(b.end))
}
