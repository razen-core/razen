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
mod tests {
    use super::*;

    fn lex_all(source: &str) -> Vec<TokenKind> {
        let mut lexer = Lexer::new(source);
        let mut kinds = Vec::new();
        loop {
            let t = lexer.next_token();
            kinds.push(t.kind.clone());
            if t.kind == TokenKind::Eof {
                break;
            }
        }
        kinds
    }

    #[test]
    fn test_keywords_and_identifiers() {
        let tokens = lex_all("mut score: int = 42\nshared cache: map[str, str]");
        assert_eq!(
            tokens,
            vec![
                TokenKind::Mut,
                TokenKind::Ident("score".to_string()),
                TokenKind::Colon,
                TokenKind::Ident("int".to_string()),
                TokenKind::Eq,
                TokenKind::Int("42".to_string()),
                TokenKind::Shared,
                TokenKind::Ident("cache".to_string()),
                TokenKind::Colon,
                TokenKind::Ident("map".to_string()),
                TokenKind::LBracket,
                TokenKind::Ident("str".to_string()),
                TokenKind::Comma,
                TokenKind::Ident("str".to_string()),
                TokenKind::RBracket,
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn test_compound_assignments_and_punctuation() {
        let tokens = lex_all("score **= 2\nratio += 0.5 ~> p()");
        assert_eq!(
            tokens,
            vec![
                TokenKind::Ident("score".to_string()),
                TokenKind::StarStarEq,
                TokenKind::Int("2".to_string()),
                TokenKind::Ident("ratio".to_string()),
                TokenKind::PlusEq,
                TokenKind::Float("0.5".to_string()),
                TokenKind::AsyncPipe,
                TokenKind::Ident("p".to_string()),
                TokenKind::LParen,
                TokenKind::RParen,
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn test_numbers_with_suffixes() {
        let tokens = lex_all("255u8 0.5f32 0b1111_0000 0xFFi64");
        assert_eq!(
            tokens,
            vec![
                TokenKind::Int("255u8".to_string()),
                TokenKind::Float("0.5f32".to_string()),
                TokenKind::Int("0b1111_0000".to_string()),
                TokenKind::Int("0xFFi64".to_string()),
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn test_strings_and_chars() {
        // String contains an unparsed placeholder interpolation variable according to placeholder setup
        let tokens = lex_all(r#" "Hello, {name}!" 'a' '\n' "#);
        assert_eq!(
            tokens[0], TokenKind::String("Hello, {name}!".to_string())
        );
        assert_eq!(tokens[1], TokenKind::Char('a'));
        assert_eq!(tokens[2], TokenKind::Char('\n'));
        assert_eq!(tokens[3], TokenKind::Eof);
    }

    #[test]
    fn test_labels_and_arrows() {
        let tokens = lex_all("'outer: loop i in 0..10 { break 'outer }");
        assert_eq!(
            tokens,
            vec![
                TokenKind::QuoteLabel("'outer".to_string()),
                TokenKind::Colon,
                TokenKind::Loop,
                TokenKind::Ident("i".to_string()),
                TokenKind::In,
                TokenKind::Int("0".to_string()),
                TokenKind::DotDot,
                TokenKind::Int("10".to_string()),
                TokenKind::LBrace,
                TokenKind::Break,
                TokenKind::QuoteLabel("'outer".to_string()),
                TokenKind::RBrace,
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn test_comments() {
        let text = "// Single comment\n/* Block\nComment */ /// Doc Comment\nx";
        let tokens = lex_all(text);
        assert_eq!(
            tokens,
            vec![
                TokenKind::DocComment("/// Doc Comment".to_string()),
                TokenKind::Ident("x".to_string()),
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn test_range_and_attributes() {
        let tokens = lex_all("@test act foo() -> 0..=5");
        assert_eq!(
            tokens,
            vec![
                TokenKind::At,
                TokenKind::Ident("test".to_string()),
                TokenKind::Act,
                TokenKind::Ident("foo".to_string()),
                TokenKind::LParen,
                TokenKind::RParen,
                TokenKind::Arrow,
                TokenKind::Int("0".to_string()),
                TokenKind::DotDotEq,
                TokenKind::Int("5".to_string()),
                TokenKind::Eof
            ]
        );
    }
}
