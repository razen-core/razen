pub mod token;
pub mod lexer;

pub use token::{Token, TokenKind, Span};
pub use lexer::Lexer;

/// Tokenize the entire source string into a vector of tokens.
///
/// The returned vector always ends with a single `TokenKind::Eof` token.
/// This is the primary entry point for feeding tokens into the parser.
pub fn tokenize(source: &str) -> Vec<Token> {
    let mut lexer = Lexer::new(source);
    let mut tokens = Vec::new();
    loop {
        let tok = lexer.next_token();
        let is_eof = tok.kind == TokenKind::Eof;
        tokens.push(tok);
        if is_eof {
            break;
        }
    }
    tokens
}

#[cfg(test)]
mod tests;
