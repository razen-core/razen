//! MIR Type Representation.
//!
//! `MirTy` is the fully resolved type used throughout MIR.  Unlike
//! `razen_sema::Ty`, it carries no inference variables and no generic
//! type parameters — all types are monomorphised before lowering.

use std::fmt;

/// The fully resolved, monomorphised type used in MIR.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MirTy {
    // ── Scalar primitives ────────────────────────────────────────────────────
    Bool,
    /// Default signed integer (i64).
    Int,
    /// Default unsigned integer (u64).
    Uint,
    /// Default float (f64).
    Float,
    I8,
    I16,
    I32,
    I64,
    I128,
    Isize,
    U8,
    U16,
    U32,
    U64,
    U128,
    Usize,
    F32,
    F64,
    Char,
    Str,
    Bytes,
    Void,
    Never,

    // ── Built-in collections ─────────────────────────────────────────────────
    Vec(Box<MirTy>),
    Map(Box<MirTy>, Box<MirTy>),
    Set(Box<MirTy>),

    // ── Structural ───────────────────────────────────────────────────────────
    Array {
        element: Box<MirTy>,
        size: u64,
    },
    Tuple(Vec<MirTy>),

    // ── Option / Result ──────────────────────────────────────────────────────
    Option(Box<MirTy>),
    Result(Box<MirTy>, Box<MirTy>),

    // ── AI / ML ──────────────────────────────────────────────────────────────
    Tensor,

    // ── User-defined types (by name after monomorphisation) ──────────────────
    /// A struct type, identified by name.
    Struct(String),
    /// An enum type, identified by name.
    Enum(String),

    // ── Function pointer ─────────────────────────────────────────────────────
    Fn {
        params: Vec<MirTy>,
        ret: Box<MirTy>,
    },

    // ── DASO shared ownership ────────────────────────────────────────────────
    Shared(Box<MirTy>),

    // ── Opaque / unknown ─────────────────────────────────────────────────────
    /// Used when the type is unknown, unsupported, or intentionally erased.
    Opaque,
}

// ---------------------------------------------------------------------------
// Conversion from sema Ty
// ---------------------------------------------------------------------------

impl MirTy {
    /// Convert a `razen_sema::Ty` into a `MirTy`.
    ///
    /// Inference variables, generic parameters, and error sentinels all map to
    /// `MirTy::Opaque` so that the lowerer can continue without crashing.
    pub fn from_sema(ty: &razen_sema::Ty) -> Self {
        use razen_sema::Ty;
        match ty {
            Ty::Bool => MirTy::Bool,
            Ty::Int => MirTy::Int,
            Ty::Uint => MirTy::Uint,
            Ty::Float => MirTy::Float,
            Ty::I8 => MirTy::I8,
            Ty::I16 => MirTy::I16,
            Ty::I32 => MirTy::I32,
            Ty::I64 => MirTy::I64,
            Ty::I128 => MirTy::I128,
            Ty::Isize => MirTy::Isize,
            Ty::U8 => MirTy::U8,
            Ty::U16 => MirTy::U16,
            Ty::U32 => MirTy::U32,
            Ty::U64 => MirTy::U64,
            Ty::U128 => MirTy::U128,
            Ty::Usize => MirTy::Usize,
            Ty::F32 => MirTy::F32,
            Ty::F64 => MirTy::F64,
            Ty::Char => MirTy::Char,
            Ty::Str => MirTy::Str,
            Ty::Bytes => MirTy::Bytes,
            Ty::Void => MirTy::Void,
            Ty::Never => MirTy::Never,
            Ty::Tensor => MirTy::Tensor,

            Ty::Vec(t) => MirTy::Vec(Box::new(Self::from_sema(t))),
            Ty::Map(k, v) => MirTy::Map(Box::new(Self::from_sema(k)), Box::new(Self::from_sema(v))),
            Ty::Set(t) => MirTy::Set(Box::new(Self::from_sema(t))),

            Ty::Array { element, size } => MirTy::Array {
                element: Box::new(Self::from_sema(element)),
                size: *size,
            },
            Ty::Tuple(elements) => MirTy::Tuple(elements.iter().map(Self::from_sema).collect()),

            Ty::Option(t) => MirTy::Option(Box::new(Self::from_sema(t))),
            Ty::Result(t, e) => {
                MirTy::Result(Box::new(Self::from_sema(t)), Box::new(Self::from_sema(e)))
            }

            // User-defined types: conservative mapping — named types default
            // to Struct; the codegen layer can distinguish via program defs.
            Ty::Named { name, .. } => MirTy::Struct(name.clone()),

            Ty::Fn { params, ret, .. } => MirTy::Fn {
                params: params.iter().map(Self::from_sema).collect(),
                ret: Box::new(Self::from_sema(ret)),
            },

            Ty::Shared(t) => MirTy::Shared(Box::new(Self::from_sema(t))),

            // Unresolvable at MIR level — map to Opaque.
            Ty::Param(_) | Ty::Infer(_) | Ty::SelfTy | Ty::Error => MirTy::Opaque,
        }
    }
}

// ---------------------------------------------------------------------------
// Helper predicates
// ---------------------------------------------------------------------------

impl MirTy {
    /// Returns `true` for any numeric type.
    pub fn is_numeric(&self) -> bool {
        self.is_integral() || self.is_float_ty()
    }

    /// Returns `true` for any integer-family type.
    pub fn is_integral(&self) -> bool {
        matches!(
            self,
            MirTy::Int
                | MirTy::Uint
                | MirTy::I8
                | MirTy::I16
                | MirTy::I32
                | MirTy::I64
                | MirTy::I128
                | MirTy::Isize
                | MirTy::U8
                | MirTy::U16
                | MirTy::U32
                | MirTy::U64
                | MirTy::U128
                | MirTy::Usize
        )
    }

    /// Returns `true` for floating-point types.
    pub fn is_float_ty(&self) -> bool {
        matches!(self, MirTy::Float | MirTy::F32 | MirTy::F64)
    }

    /// Returns `true` for the `void` / unit type.
    pub fn is_void(&self) -> bool {
        matches!(self, MirTy::Void)
    }

    /// Returns `true` for the `never` diverging type.
    pub fn is_never(&self) -> bool {
        matches!(self, MirTy::Never)
    }

    /// Returns `true` when the type is meaningfully opaque (unknown).
    pub fn is_opaque(&self) -> bool {
        matches!(self, MirTy::Opaque)
    }
}

// ---------------------------------------------------------------------------
// Display
// ---------------------------------------------------------------------------

impl fmt::Display for MirTy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MirTy::Bool => write!(f, "bool"),
            MirTy::Int => write!(f, "int"),
            MirTy::Uint => write!(f, "uint"),
            MirTy::Float => write!(f, "float"),
            MirTy::I8 => write!(f, "i8"),
            MirTy::I16 => write!(f, "i16"),
            MirTy::I32 => write!(f, "i32"),
            MirTy::I64 => write!(f, "i64"),
            MirTy::I128 => write!(f, "i128"),
            MirTy::Isize => write!(f, "isize"),
            MirTy::U8 => write!(f, "u8"),
            MirTy::U16 => write!(f, "u16"),
            MirTy::U32 => write!(f, "u32"),
            MirTy::U64 => write!(f, "u64"),
            MirTy::U128 => write!(f, "u128"),
            MirTy::Usize => write!(f, "usize"),
            MirTy::F32 => write!(f, "f32"),
            MirTy::F64 => write!(f, "f64"),
            MirTy::Char => write!(f, "char"),
            MirTy::Str => write!(f, "str"),
            MirTy::Bytes => write!(f, "bytes"),
            MirTy::Void => write!(f, "void"),
            MirTy::Never => write!(f, "never"),
            MirTy::Tensor => write!(f, "tensor"),
            MirTy::Opaque => write!(f, "<opaque>"),

            MirTy::Vec(t) => write!(f, "vec[{}]", t),
            MirTy::Map(k, v) => write!(f, "map[{}, {}]", k, v),
            MirTy::Set(t) => write!(f, "set[{}]", t),

            MirTy::Array { element, size } => write!(f, "[{}; {}]", element, size),

            MirTy::Tuple(elems) => {
                write!(f, "(")?;
                for (i, e) in elems.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", e)?;
                }
                write!(f, ")")
            }

            MirTy::Option(t) => write!(f, "option[{}]", t),
            MirTy::Result(t, e) => write!(f, "result[{}, {}]", t, e),

            MirTy::Struct(name) => write!(f, "{}", name),
            MirTy::Enum(name) => write!(f, "{}", name),

            MirTy::Fn { params, ret } => {
                write!(f, "|")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", p)?;
                }
                write!(f, "| -> {}", ret)
            }

            MirTy::Shared(t) => write!(f, "shared {}", t),
        }
    }
}
