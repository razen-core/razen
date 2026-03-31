//! Resolved Type Representation for Razen.
//!
//! `Ty` is the compiler's internal representation of types AFTER semantic analysis.
//! Unlike `razen_ast::TypeExpr` (which represents types as written in source code),
//! `Ty` is fully resolved and may contain inference variables during type inference.
//!
//! After `TypeChecker::finalize()` runs, all `Ty::Infer` variants are replaced
//! with concrete types.

use crate::symbol::DefId;
use std::fmt;

/// A unique identifier for a type-inference variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InferVarId(pub u32);

/// The compiler's internal resolved type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Ty {
    // -----------------------------------------------------------------------
    // Primitive scalars
    // -----------------------------------------------------------------------
    /// `bool`
    Bool,

    /// Default signed integer — `int` (64-bit)
    Int,
    /// Default unsigned integer — `uint` (64-bit)
    Uint,
    /// Default float — `float` (64-bit)
    Float,

    /// Sized signed integers
    I8,
    I16,
    I32,
    I64,
    I128,
    Isize,

    /// Sized unsigned integers
    U8,
    U16,
    U32,
    U64,
    U128,
    Usize,

    /// Sized floats
    F32,
    F64,

    /// Unicode code point — `char`
    Char,
    /// UTF-8 string — `str`
    Str,
    /// Raw byte buffer — `bytes`
    Bytes,

    // -----------------------------------------------------------------------
    // Unit / diverging
    // -----------------------------------------------------------------------
    /// No return value — `void`
    Void,
    /// Diverging computation — `never`
    Never,

    // -----------------------------------------------------------------------
    // Built-in generic collections
    // -----------------------------------------------------------------------
    /// Dynamic array — `vec[T]`
    Vec(Box<Ty>),
    /// Hash map — `map[K, V]`
    Map(Box<Ty>, Box<Ty>),
    /// Hash set — `set[T]`
    Set(Box<Ty>),

    // -----------------------------------------------------------------------
    // Structural types
    // -----------------------------------------------------------------------
    /// Fixed-size array — `[T; N]`
    Array {
        element: Box<Ty>,
        size: u64,
    },
    /// Tuple — `(T1, T2, ...)`
    Tuple(Vec<Ty>),

    // -----------------------------------------------------------------------
    // Option / Result  (special-cased for ergonomics)
    // -----------------------------------------------------------------------
    /// `option[T]`
    Option(Box<Ty>),
    /// `result[T, E]`
    Result(Box<Ty>, Box<Ty>),

    // -----------------------------------------------------------------------
    // AI / ML
    // -----------------------------------------------------------------------
    /// N-dimensional array — `tensor`
    Tensor,

    // -----------------------------------------------------------------------
    // User-defined types
    // -----------------------------------------------------------------------
    /// A user-defined struct or enum referenced by its `DefId`.
    /// `generics` holds the instantiated type arguments.
    Named {
        def_id: DefId,
        name: String,
        generics: Vec<Ty>,
    },

    // -----------------------------------------------------------------------
    // Function types
    // -----------------------------------------------------------------------
    /// A function or method type.
    Fn {
        params: Vec<Ty>,
        ret: Box<Ty>,
        is_async: bool,
    },

    // -----------------------------------------------------------------------
    // Generic type parameters
    // -----------------------------------------------------------------------
    /// A generic type parameter, e.g. `T` in `act identity[T](x: T) T`.
    Param(String),

    // -----------------------------------------------------------------------
    // DASO shared ownership
    // -----------------------------------------------------------------------
    /// A shared reference-counted value — `shared T`.
    Shared(Box<Ty>),

    // -----------------------------------------------------------------------
    // Type inference
    // -----------------------------------------------------------------------
    /// An unresolved inference variable.
    /// Should not appear after `TypeChecker::finalize()`.
    Infer(InferVarId),

    // -----------------------------------------------------------------------
    // Special
    // -----------------------------------------------------------------------
    /// The `Self` type inside an `impl` or `trait` block.
    SelfTy,

    /// Error sentinel — used to continue type-checking after a type error
    /// without cascading false positives.  Any operation on `Error` also
    /// produces `Error`, keeping error counts meaningful.
    Error,
}

// ---------------------------------------------------------------------------
// Helper predicates & utilities
// ---------------------------------------------------------------------------

impl Ty {
    /// Returns `true` for any numeric type (integer or float).
    pub fn is_numeric(&self) -> bool {
        self.is_integral() || self.is_float_ty()
    }

    /// Returns `true` for any integer-family type.
    pub fn is_integral(&self) -> bool {
        matches!(
            self,
            Ty::Int
                | Ty::Uint
                | Ty::I8
                | Ty::I16
                | Ty::I32
                | Ty::I64
                | Ty::I128
                | Ty::Isize
                | Ty::U8
                | Ty::U16
                | Ty::U32
                | Ty::U64
                | Ty::U128
                | Ty::Usize
        )
    }

    /// Returns `true` for floating-point types.
    pub fn is_float_ty(&self) -> bool {
        matches!(self, Ty::Float | Ty::F32 | Ty::F64)
    }

    /// Returns `true` if this is the `bool` type.
    pub fn is_bool(&self) -> bool {
        matches!(self, Ty::Bool)
    }

    /// Returns `true` if this is the `never` type.
    pub fn is_never(&self) -> bool {
        matches!(self, Ty::Never)
    }

    /// Returns `true` if this is the `void` type.
    pub fn is_void(&self) -> bool {
        matches!(self, Ty::Void)
    }

    /// Returns `true` if this is the error sentinel type.
    pub fn is_error(&self) -> bool {
        matches!(self, Ty::Error)
    }

    /// Returns `true` if this is an unresolved inference variable.
    pub fn is_infer(&self) -> bool {
        matches!(self, Ty::Infer(_))
    }

    /// If this is `Ty::Option(t)`, return `Some(*t)`.  Otherwise `None`.
    pub fn unwrap_option(self) -> std::option::Option<Ty> {
        match self {
            Ty::Option(t) => Some(*t),
            _ => None,
        }
    }

    /// If this is `Ty::Result(t, _)`, return `Some(*t)`.  Otherwise `None`.
    pub fn unwrap_result_ok(self) -> std::option::Option<Ty> {
        match self {
            Ty::Result(t, _) => Some(*t),
            _ => None,
        }
    }

    /// If this is `Ty::Result(_, e)`, return `Some(*e)`.  Otherwise `None`.
    pub fn unwrap_result_err(self) -> std::option::Option<Ty> {
        match self {
            Ty::Result(_, e) => Some(*e),
            _ => None,
        }
    }

    /// The default integer type used when an integer literal remains
    /// unconstrained (analogous to Rust's integer defaulting to `i32`).
    pub fn integer_default() -> Ty {
        Ty::Int
    }

    /// The default float type.
    pub fn float_default() -> Ty {
        Ty::Float
    }

    /// Returns `true` if this type is transparent to ownership (i.e. it is
    /// a `shared` wrapper and the inner type is what matters for method
    /// resolution).
    pub fn is_shared(&self) -> bool {
        matches!(self, Ty::Shared(_))
    }

    /// Peel a `Shared` wrapper, if present.
    pub fn peel_shared(&self) -> &Ty {
        match self {
            Ty::Shared(inner) => inner.as_ref(),
            other => other,
        }
    }
}

// ---------------------------------------------------------------------------
// Display
// ---------------------------------------------------------------------------

impl fmt::Display for Ty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Ty::Bool => write!(f, "bool"),
            Ty::Int => write!(f, "int"),
            Ty::Uint => write!(f, "uint"),
            Ty::Float => write!(f, "float"),
            Ty::I8 => write!(f, "i8"),
            Ty::I16 => write!(f, "i16"),
            Ty::I32 => write!(f, "i32"),
            Ty::I64 => write!(f, "i64"),
            Ty::I128 => write!(f, "i128"),
            Ty::Isize => write!(f, "isize"),
            Ty::U8 => write!(f, "u8"),
            Ty::U16 => write!(f, "u16"),
            Ty::U32 => write!(f, "u32"),
            Ty::U64 => write!(f, "u64"),
            Ty::U128 => write!(f, "u128"),
            Ty::Usize => write!(f, "usize"),
            Ty::F32 => write!(f, "f32"),
            Ty::F64 => write!(f, "f64"),
            Ty::Char => write!(f, "char"),
            Ty::Str => write!(f, "str"),
            Ty::Bytes => write!(f, "bytes"),
            Ty::Void => write!(f, "void"),
            Ty::Never => write!(f, "never"),
            Ty::Tensor => write!(f, "tensor"),
            Ty::SelfTy => write!(f, "Self"),
            Ty::Error => write!(f, "<error>"),

            Ty::Vec(t) => write!(f, "vec[{}]", t),
            Ty::Map(k, v) => write!(f, "map[{}, {}]", k, v),
            Ty::Set(t) => write!(f, "set[{}]", t),

            Ty::Array { element, size } => write!(f, "[{}; {}]", element, size),

            Ty::Tuple(elements) => {
                write!(f, "(")?;
                for (i, t) in elements.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", t)?;
                }
                write!(f, ")")
            }

            Ty::Option(t) => write!(f, "option[{}]", t),
            Ty::Result(t, e) => write!(f, "result[{}, {}]", t, e),

            Ty::Named { name, generics, .. } => {
                write!(f, "{}", name)?;
                if !generics.is_empty() {
                    write!(f, "[")?;
                    for (i, g) in generics.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", g)?;
                    }
                    write!(f, "]")?;
                }
                Ok(())
            }

            Ty::Fn {
                params,
                ret,
                is_async,
            } => {
                if *is_async {
                    write!(f, "async ")?;
                }
                write!(f, "|")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", p)?;
                }
                write!(f, "| -> {}", ret)
            }

            Ty::Param(name) => write!(f, "{}", name),
            Ty::Shared(t) => write!(f, "shared {}", t),
            Ty::Infer(id) => write!(f, "?{}", id.0),
        }
    }
}
