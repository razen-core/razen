//! MIR Values: local variable IDs, block IDs, constants, and operands.
//!
//! These are the fundamental "leaves" of the MIR — every RValue and
//! Terminator ultimately bottoms out into `Operand`s which are either
//! references to `Local`s or compile-time `Const`s.

// ---------------------------------------------------------------------------
// Identifiers
// ---------------------------------------------------------------------------

/// Identifies a local variable slot inside a `MirFn`.
///
/// Local 0 is conventionally the return-value slot.
/// Locals 1..param_count are parameters.
/// Locals beyond that are compiler-generated temporaries and named bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LocalId(pub u32);

/// Identifies a basic block inside a `MirFn`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BlockId(pub u32);

// ---------------------------------------------------------------------------
// Compile-time constants
// ---------------------------------------------------------------------------

/// A compile-time constant value embedded directly in MIR.
#[derive(Debug, Clone, PartialEq)]
pub enum Const {
    /// Boolean constant: `true` or `false`.
    Bool(bool),
    /// Signed integer constant (covers i8–i128, isize, and the default `int`).
    Int(i64),
    /// Unsigned integer constant (covers u8–u128, usize, and the default `uint`).
    Uint(u64),
    /// Floating-point constant (covers f32, f64, and the default `float`).
    Float(f64),
    /// String literal.
    Str(String),
    /// Character literal.
    Char(char),
    /// The unit / void value `()`.
    Unit,
    /// The `none` / null value (empty option).
    Null,
}

impl Const {
    /// Returns `true` if this constant is numerically zero / false.
    pub fn is_zero(&self) -> bool {
        match self {
            Const::Bool(false) | Const::Int(0) | Const::Uint(0) => true,
            Const::Float(f) => *f == 0.0,
            _ => false,
        }
    }

    /// Returns `true` if this is the unit value.
    pub fn is_unit(&self) -> bool {
        matches!(self, Const::Unit)
    }

    /// Returns `true` if this is the null / none value.
    pub fn is_null(&self) -> bool {
        matches!(self, Const::Null)
    }
}

// ---------------------------------------------------------------------------
// Operands
// ---------------------------------------------------------------------------

/// An *operand* is the read side of any instruction — either a reference to
/// a local variable or an inline compile-time constant.
///
/// We use a single `Operand` type rather than separate Copy/Move variants
/// because move semantics are handled at a higher level (DASO analysis).
#[derive(Debug, Clone, PartialEq)]
pub enum Operand {
    /// Read the value of a local variable.
    Local(LocalId),
    /// An inline compile-time constant.
    Const(Const),
}

impl Operand {
    /// Convenience constructor for a local operand.
    pub fn local(id: LocalId) -> Self {
        Operand::Local(id)
    }

    /// Convenience constructor for a boolean constant operand.
    pub fn bool_const(v: bool) -> Self {
        Operand::Const(Const::Bool(v))
    }

    /// Convenience constructor for an integer constant operand.
    pub fn int_const(n: i64) -> Self {
        Operand::Const(Const::Int(n))
    }

    /// Convenience constructor for the unit value operand.
    pub fn unit() -> Self {
        Operand::Const(Const::Unit)
    }

    /// Convenience constructor for the null / none operand.
    pub fn null() -> Self {
        Operand::Const(Const::Null)
    }

    /// Returns the `LocalId` if this is a `Local` operand, otherwise `None`.
    pub fn as_local(&self) -> Option<LocalId> {
        match self {
            Operand::Local(id) => Some(*id),
            _ => None,
        }
    }

    /// Returns the `Const` if this is a `Const` operand, otherwise `None`.
    pub fn as_const(&self) -> Option<&Const> {
        match self {
            Operand::Const(c) => Some(c),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_id_display() {
        assert_eq!(format!("{}", LocalId(0)), "_0");
        assert_eq!(format!("{}", LocalId(42)), "_42");
    }

    #[test]
    fn test_block_id_display() {
        assert_eq!(format!("{}", BlockId(0)), "bb0");
        assert_eq!(format!("{}", BlockId(5)), "bb5");
    }

    #[test]
    fn test_const_display() {
        assert_eq!(format!("{}", Const::Bool(true)), "true");
        assert_eq!(format!("{}", Const::Int(-42)), "-42");
        assert_eq!(format!("{}", Const::Uint(100)), "100u");
        assert_eq!(format!("{}", Const::Str("hello".into())), "\"hello\"");
        assert_eq!(format!("{}", Const::Char('a')), "'a'");
        assert_eq!(format!("{}", Const::Unit), "()");
        assert_eq!(format!("{}", Const::Null), "null");
    }

    #[test]
    fn test_operand_as_local() {
        let op = Operand::local(LocalId(3));
        assert_eq!(op.as_local(), Some(LocalId(3)));
        let op2 = Operand::unit();
        assert_eq!(op2.as_local(), None);
    }

    #[test]
    fn test_operand_display() {
        assert_eq!(format!("{}", Operand::Local(LocalId(2))), "_2");
        assert_eq!(format!("{}", Operand::Const(Const::Int(7))), "7");
    }
}
