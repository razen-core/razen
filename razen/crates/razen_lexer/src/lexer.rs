use crate::token::{Token, TokenKind, Span};

pub struct Lexer<'a> {
    source: &'a str,
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            bytes: source.as_bytes(),
            pos: 0,
        }
    }

    fn advance(&mut self) -> Option<u8> {
        if self.is_eof() {
            None
        } else {
            let b = self.bytes[self.pos];
            self.pos += 1;
            Some(b)
        }
    }

    fn peek(&self) -> Option<u8> {
        if self.is_eof() {
            None
        } else {
            Some(self.bytes[self.pos])
        }
    }

    fn peek_next(&self) -> Option<u8> {
        if self.pos + 1 >= self.bytes.len() {
            None
        } else {
            Some(self.bytes[self.pos + 1])
        }
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.bytes.len()
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_ascii_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn skip_comments(&mut self) {
        while let Some(c) = self.peek() {
            if c == b'/' {
                if self.peek_next() == Some(b'/') && self.peek_nth(2) != Some(b'/') {
                    // Regular comment
                    self.advance();
                    self.advance();
                    while let Some(ch) = self.peek() {
                        if ch == b'\n' {
                            break;
                        }
                        self.advance();
                    }
                } else if self.peek_next() == Some(b'*') {
                    // Block comment
                    self.advance();
                    self.advance();
                    while let Some(ch) = self.peek() {
                        if ch == b'*' && self.peek_next() == Some(b'/') {
                            self.advance();
                            self.advance();
                            break;
                        }
                        self.advance();
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }

    fn peek_nth(&self, offset: usize) -> Option<u8> {
        if self.pos + offset >= self.bytes.len() {
            None
        } else {
            Some(self.bytes[self.pos + offset])
        }
    }

    pub fn next_token(&mut self) -> Token {
        loop {
            self.skip_whitespace();
            let start_pos = self.pos;

            // Check for potential comments to skip
            if self.peek() == Some(b'/') {
                let next = self.peek_next();
                if next == Some(b'/') {
                    // Doc comment Check
                    if self.peek_nth(2) == Some(b'/') {
                        // It's a doc comment!
                        self.advance();
                        self.advance();
                        self.advance();
                        while self.peek().map(|c| c != b'\n').unwrap_or(false) {
                            self.advance();
                        }
                        return Token::new(
                            TokenKind::DocComment(self.source[start_pos..self.pos].to_string()),
                            Span::new(start_pos, self.pos),
                        );
                    } else {
                        self.skip_comments();
                        continue;
                    }
                } else if next == Some(b'*') {
                    self.skip_comments();
                    continue;
                }
            }
            break;
        }

        if self.is_eof() {
            return Token::new(TokenKind::Eof, Span::new(self.pos, self.pos));
        }

        let start = self.pos;
        let c = self.advance().unwrap();

        let kind = match c {
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                if c == b'_' && !self.peek().map_or(false, |p| p.is_ascii_alphanumeric() || p == b'_') {
                    TokenKind::Underscore
                } else {
                    self.lex_identifier(start)
                }
            }
            b'0'..=b'9' => self.lex_number(start),
            b'"' => self.lex_string(start),
            b'\'' => self.lex_char_or_label(start),
            b':' => {
                if self.peek() == Some(b'=') {
                    self.advance();
                    TokenKind::ColonEq
                } else {
                    TokenKind::Colon
                }
            }
            b'=' => {
                if self.peek() == Some(b'=') {
                    self.advance();
                    TokenKind::EqEq
                } else {
                    TokenKind::Eq
                }
            }
            b'+' => {
                if self.peek() == Some(b'=') {
                    self.advance();
                    TokenKind::PlusEq
                } else {
                    TokenKind::Plus
                }
            }
            b'-' => {
                let p = self.peek();
                if p == Some(b'=') {
                    self.advance();
                    TokenKind::MinusEq
                } else if p == Some(b'>') {
                    self.advance();
                    TokenKind::Arrow
                } else {
                    TokenKind::Minus
                }
            }
            b'*' => {
                let p1 = self.peek();
                let p2 = self.peek_next();
                if p1 == Some(b'*') && p2 == Some(b'=') {
                    self.advance();
                    self.advance();
                    TokenKind::StarStarEq
                } else if p1 == Some(b'*') {
                    self.advance();
                    TokenKind::StarStar
                } else if p1 == Some(b'=') {
                    self.advance();
                    TokenKind::StarEq
                } else {
                    TokenKind::Star
                }
            }
            b'/' => {
                if self.peek() == Some(b'=') {
                    self.advance();
                    TokenKind::SlashEq
                } else {
                    TokenKind::Slash
                }
            }
            b'%' => {
                if self.peek() == Some(b'=') {
                    self.advance();
                    TokenKind::PercentEq
                } else {
                    TokenKind::Percent
                }
            }
            b'.' => {
                if self.peek() == Some(b'.') {
                    self.advance();
                    if self.peek() == Some(b'=') {
                        self.advance();
                        TokenKind::DotDotEq
                    } else {
                        TokenKind::DotDot
                    }
                } else {
                    TokenKind::Dot
                }
            }
            b'~' => {
                if self.peek() == Some(b'>') {
                    self.advance();
                    TokenKind::AsyncPipe
                } else {
                    TokenKind::Tilde
                }
            }
            b'<' => {
                let p = self.peek();
                if p == Some(b'=') {
                    self.advance();
                    TokenKind::LtEq
                } else if p == Some(b'<') {
                    self.advance();
                    TokenKind::Shl
                } else {
                    TokenKind::Lt
                }
            }
            b'>' => {
                let p = self.peek();
                if p == Some(b'=') {
                    self.advance();
                    TokenKind::GtEq
                } else if p == Some(b'>') {
                    self.advance();
                    TokenKind::Shr
                } else {
                    TokenKind::Gt
                }
            }
            b'!' => {
                if self.peek() == Some(b'=') {
                    self.advance();
                    TokenKind::NotEq
                } else {
                    TokenKind::Bang
                }
            }
            b'&' => {
                if self.peek() == Some(b'&') {
                    self.advance();
                    TokenKind::AndAnd
                } else {
                    TokenKind::And
                }
            }
            b'|' => {
                if self.peek() == Some(b'|') {
                    self.advance();
                    TokenKind::OrOr
                } else {
                    TokenKind::Or
                }
            }
            b'?' => TokenKind::Question,
            b'^' => TokenKind::Caret,
            b'{' => TokenKind::LBrace,
            b'}' => TokenKind::RBrace,
            b'[' => TokenKind::LBracket,
            b']' => TokenKind::RBracket,
            b'(' => TokenKind::LParen,
            b')' => TokenKind::RParen,
            b',' => TokenKind::Comma,
            b';' => TokenKind::Semi,
            b'@' => TokenKind::At,
            _ => TokenKind::Error(format!("Unexpected character: {}", c as char)),
        };

        Token::new(kind, Span::new(start, self.pos))
    }

    fn lex_identifier(&mut self, start: usize) -> TokenKind {
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == b'_' {
                self.advance();
            } else {
                break;
            }
        }

        let text = &self.source[start..self.pos];
        match text {
            "mut" => TokenKind::Mut,
            "const" => TokenKind::Const,
            "shared" => TokenKind::Shared,
            "struct" => TokenKind::Struct,
            "enum" => TokenKind::Enum,
            "trait" => TokenKind::Trait,
            "impl" => TokenKind::Impl,
            "alias" => TokenKind::Alias,
            "act" => TokenKind::Act,
            "ret" => TokenKind::Ret,
            "use" => TokenKind::Use,
            "pub" => TokenKind::Pub,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "loop" => TokenKind::Loop,
            "break" => TokenKind::Break,
            "next" => TokenKind::Next,
            "match" => TokenKind::Match,
            "guard" => TokenKind::Guard,
            "in" => TokenKind::In,
            "as" => TokenKind::As,
            "is" => TokenKind::Is,
            "self" => TokenKind::SelfKw,
            "Self" => TokenKind::SelfType,
            "defer" => TokenKind::Defer,
            "async" => TokenKind::Async,
            "await" => TokenKind::Await,
            "fork" => TokenKind::Fork,
            "unsafe" => TokenKind::Unsafe,
            "where" => TokenKind::Where,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            _ => TokenKind::Ident(text.to_string()),
        }
    }

    fn lex_number(&mut self, start: usize) -> TokenKind {
        // Hex, binary, floats, suffixes
        let mut is_float = false;
        
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == b'_' {
                self.advance();
            } else if c == b'.' {
                // Check if it's double dot `..` or method call `.`
                // Only treat as float if next char is ascii digit
                if let Some(next) = self.peek_next() {
                    if next.is_ascii_digit() {
                        self.advance();
                        is_float = true;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        
        let text = self.source[start..self.pos].to_string();
        if is_float || text.contains("f32") || text.contains("f64") {
            TokenKind::Float(text)
        } else {
            TokenKind::Int(text)
        }
    }

    fn lex_string(&mut self, _start: usize) -> TokenKind {
        // We already consumed `"`
        let mut content = String::new();
        while let Some(c) = self.peek() {
            if c == b'"' {
                self.advance(); // consume it
                return TokenKind::String(content);
            }
            let ch = self.advance().unwrap();
            if ch == b'\\' {
                // simple escape handling
                if let Some(esc) = self.advance() {
                    content.push(match esc {
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'\\' => '\\',
                        b'"' => '"',
                        _ => esc as char,
                    });
                }
            } else {
                content.push(ch as char);
            }
        }
        TokenKind::Error("Unterminated string literal".to_string())
    }

    fn lex_char_or_label(&mut self, start: usize) -> TokenKind {
        // already consumed `'`
        /*
          Cases:
          1. 'a' -> Char
          2. '\n' -> Char
          3. 'label -> Label
          4. 'label: -> the colon is separate, label part is just 'label
        */
        
        let p1 = self.peek();
        let p2 = self.peek_next();
        
        let mut is_char = false;
        if p1 == Some(b'\\') {
            // escape seq, assume char
            is_char = true;
        } else if p1.is_some() && p2 == Some(b'\'') {
            is_char = true; // normal char
        }

        if is_char {
            let ch = self.advance().unwrap();
            let actual_char = if ch == b'\\' {
                match self.advance() {
                    Some(b'n') => '\n',
                    Some(b't') => '\t',
                    Some(b'r') => '\r',
                    Some(b'\\') => '\\',
                    Some(b'\'') => '\'',
                    Some(b'0') => '\0',
                    Some(other) => other as char,
                    None => return TokenKind::Error("Unterminated char escape".to_string()),
                }
            } else {
                ch as char
            };

            if self.peek() == Some(b'\'') {
                self.advance();
                TokenKind::Char(actual_char)
            } else {
                TokenKind::Error("Unterminated char literal".to_string())
            }
        } else {
            // Label
            while let Some(c) = self.peek() {
                if c.is_ascii_alphanumeric() || c == b'_' {
                    self.advance();
                } else {
                    break;
                }
            }
            let text = &self.source[start..self.pos];
            if text.len() == 1 {
                // Just `'`
                TokenKind::Error("Invalid label or unclosed character".to_string())
            } else {
                TokenKind::QuoteLabel(text.to_string())
            }
        }
    }
}
