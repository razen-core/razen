#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    // Keywords
    Mut, Const, Shared, Struct, Enum, Trait, Impl, Alias, Act, Ret, Use, Pub,
    If, Else, Loop, Break, Next, Match, Guard, In, As, Is, SelfKw, SelfType,
    Defer, Async, Await, Fork, Unsafe, Where, True, False,

    // Identifiers and Literals
    Ident(String),
    Int(String),       
    Float(String),     
    Char(char),
    String(String),    // Placeholder for string interpolation future enhancement

    // Variables & Assignment
    ColonEq,        // :=
    Eq,             // =
    Colon,          // :

    // Compound Assignment
    PlusEq,         // +=
    MinusEq,        // -=
    StarEq,         // *=
    SlashEq,        // /=
    PercentEq,      // %=
    StarStarEq,     // **=

    // Math
    Plus,           // +
    Minus,          // -
    Star,           // *
    Slash,          // /
    Percent,        // %
    StarStar,       // **

    // Logic & Bitwise
    EqEq,           // ==
    NotEq,          // !=
    Lt,             // <
    Gt,             // >
    LtEq,           // <=
    GtEq,           // >=
    AndAnd,         // &&
    OrOr,           // ||
    And,            // &
    Or,             // |
    Caret,          // ^
    Tilde,          // ~
    Shl,            // <<
    Shr,            // >>
    Bang,           // !

    // Structure
    Dot,            // .
    Arrow,          // ->
    AsyncPipe,      // ~>
    Question,       // ?
    DotDot,         // ..
    DotDotEq,       // ..=
    Underscore,     // _
    QuoteLabel(String), // 'label

    // Punctuation
    LBrace,         // {
    RBrace,         // }
    LBracket,       // [
    RBracket,       // ]
    LParen,         // (
    RParen,         // )
    Comma,          // ,
    Semi,           // ;
    At,             // @
    DocComment(String), // /// ...
    
    // Control
    Eof,
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}
