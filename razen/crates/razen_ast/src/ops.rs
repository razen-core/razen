//! Operator enumerations.

/// Binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    // Arithmetic
    Add,        // +
    Sub,        // -
    Mul,        // *
    Div,        // /
    Mod,        // %
    Pow,        // **

    // Comparison
    Eq,         // ==
    NotEq,      // !=
    Lt,         // <
    Gt,         // >
    LtEq,       // <=
    GtEq,       // >=

    // Logical
    And,        // &&
    Or,         // ||

    // Bitwise
    BitAnd,     // &
    BitOr,      // |
    BitXor,     // ^
    Shl,        // <<
    Shr,        // >>

    // Range
    Range,          // ..
    RangeInclusive, // ..=

    // Pipeline
    AsyncPipe,  // ~>
}

/// Unary (prefix) operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    /// Arithmetic negation `-x`
    Neg,
    /// Logical NOT `!x`
    Not,
    /// Bitwise NOT `~x`
    BitNot,
    /// Address-of `&x` (unsafe context)
    Ref,
    /// Dereference `*x` (unsafe context)
    Deref,
}

/// Compound assignment operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompoundOp {
    AddAssign,  // +=
    SubAssign,  // -=
    MulAssign,  // *=
    DivAssign,  // /=
    ModAssign,  // %=
    PowAssign,  // **=
}
