//! MIR Program — the top-level IR unit produced by the lowering pass.
//!
//! A `MirProgram` holds all struct definitions, enum definitions, and
//! function definitions that result from lowering a single Razen source
//! module.  It is the primary input to every downstream compiler phase
//! (optimisation, C-codegen, etc.).

use crate::func::MirFn;
use crate::ty::MirTy;

// ---------------------------------------------------------------------------
// Struct definition
// ---------------------------------------------------------------------------

/// A struct definition in MIR form.
///
/// Fields are stored in declaration order; the name is used by codegen to
/// emit the correct struct type and to resolve field accesses at compile time.
#[derive(Debug, Clone)]
pub struct MirStruct {
    /// The struct's unmangled source name (e.g. `"User"`).
    pub name: String,

    /// Ordered list of `(field_name, field_type)` pairs.
    pub fields: Vec<(String, MirTy)>,

    /// Whether the struct was declared `pub`.
    pub is_pub: bool,
}

impl MirStruct {
    /// Create a new struct definition.
    pub fn new(name: String, fields: Vec<(String, MirTy)>, is_pub: bool) -> Self {
        Self {
            name,
            fields,
            is_pub,
        }
    }

    /// Look up the index and type of a field by name.
    ///
    /// Returns `Some((index, &MirTy))` if the field exists, `None` otherwise.
    pub fn field(&self, name: &str) -> Option<(usize, &MirTy)> {
        self.fields
            .iter()
            .enumerate()
            .find_map(|(i, (n, ty))| if n == name { Some((i, ty)) } else { None })
    }

    /// Returns the number of fields in this struct.
    pub fn field_count(&self) -> usize {
        self.fields.len()
    }

    /// Returns `true` if this is a unit struct (no fields).
    pub fn is_unit(&self) -> bool {
        self.fields.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Enum definition
// ---------------------------------------------------------------------------

/// The shape of data carried by an enum variant.
#[derive(Debug, Clone, PartialEq)]
pub enum MirVariantKind {
    /// No associated data: `North`, `None`, …
    Unit,

    /// Positional (unnamed) fields: `Circle(float)`, `Ok(T)`, …
    Positional(Vec<MirTy>),

    /// Named fields: `Click { x: float, y: float }`, …
    Named(Vec<(String, MirTy)>),
}

impl MirVariantKind {
    /// Returns `true` if this variant carries no data.
    pub fn is_unit(&self) -> bool {
        matches!(self, MirVariantKind::Unit)
    }

    /// Returns the number of fields (0 for unit variants).
    pub fn field_count(&self) -> usize {
        match self {
            MirVariantKind::Unit => 0,
            MirVariantKind::Positional(fields) => fields.len(),
            MirVariantKind::Named(fields) => fields.len(),
        }
    }
}

/// A single variant within a MIR enum definition.
#[derive(Debug, Clone)]
pub struct MirVariant {
    /// The variant's unmangled source name (e.g. `"North"`, `"Circle"`).
    pub name: String,

    /// The data carried by this variant.
    pub kind: MirVariantKind,

    /// The integer discriminant assigned to this variant.
    /// Discriminants are assigned in declaration order starting at 0.
    pub discriminant: i64,
}

impl MirVariant {
    /// Create a unit variant with the given discriminant.
    pub fn unit(name: String, discriminant: i64) -> Self {
        Self {
            name,
            kind: MirVariantKind::Unit,
            discriminant,
        }
    }

    /// Create a positional variant.
    pub fn positional(name: String, fields: Vec<MirTy>, discriminant: i64) -> Self {
        Self {
            name,
            kind: MirVariantKind::Positional(fields),
            discriminant,
        }
    }

    /// Create a named-field variant.
    pub fn named(name: String, fields: Vec<(String, MirTy)>, discriminant: i64) -> Self {
        Self {
            name,
            kind: MirVariantKind::Named(fields),
            discriminant,
        }
    }
}

/// An enum definition in MIR form.
#[derive(Debug, Clone)]
pub struct MirEnum {
    /// The enum's unmangled source name (e.g. `"Direction"`, `"Shape"`).
    pub name: String,

    /// All variants in declaration order.
    pub variants: Vec<MirVariant>,

    /// Whether the enum was declared `pub`.
    pub is_pub: bool,
}

impl MirEnum {
    /// Create a new enum definition.
    pub fn new(name: String, variants: Vec<MirVariant>, is_pub: bool) -> Self {
        Self {
            name,
            variants,
            is_pub,
        }
    }

    /// Look up a variant by name.
    ///
    /// Returns `Some(&MirVariant)` if found, `None` otherwise.
    pub fn variant(&self, name: &str) -> Option<&MirVariant> {
        self.variants.iter().find(|v| v.name == name)
    }

    /// Returns the discriminant for a variant name, or `None` if not found.
    pub fn discriminant_of(&self, variant_name: &str) -> Option<i64> {
        self.variant(variant_name).map(|v| v.discriminant)
    }

    /// Returns the number of variants.
    pub fn variant_count(&self) -> usize {
        self.variants.len()
    }
}

// ---------------------------------------------------------------------------
// MirProgram
// ---------------------------------------------------------------------------

/// The complete MIR program produced by lowering a single Razen module.
///
/// A `MirProgram` is the output of `razen_mir::lower()` and the input to
/// every downstream compiler phase.
#[derive(Debug, Clone)]
pub struct MirProgram {
    /// All struct type definitions, in declaration order.
    pub structs: Vec<MirStruct>,

    /// All enum type definitions, in declaration order.
    pub enums: Vec<MirEnum>,

    /// All function definitions (including `main` and impl methods),
    /// in declaration order.
    pub functions: Vec<MirFn>,
}

impl MirProgram {
    /// Create an empty `MirProgram`.
    pub fn new() -> Self {
        Self {
            structs: Vec::new(),
            enums: Vec::new(),
            functions: Vec::new(),
        }
    }

    // -----------------------------------------------------------------------
    // Builders
    // -----------------------------------------------------------------------

    /// Add a struct definition.
    pub fn add_struct(&mut self, s: MirStruct) {
        self.structs.push(s);
    }

    /// Add an enum definition.
    pub fn add_enum(&mut self, e: MirEnum) {
        self.enums.push(e);
    }

    /// Add a function definition.
    pub fn add_fn(&mut self, f: MirFn) {
        self.functions.push(f);
    }

    // -----------------------------------------------------------------------
    // Lookups
    // -----------------------------------------------------------------------

    /// Find a function by name.
    ///
    /// Returns `Some(&MirFn)` if a function with the given name exists.
    pub fn get_fn(&self, name: &str) -> Option<&MirFn> {
        self.functions.iter().find(|f| f.name == name)
    }

    /// Find a function by name (mutable).
    pub fn get_fn_mut(&mut self, name: &str) -> Option<&mut MirFn> {
        self.functions.iter_mut().find(|f| f.name == name)
    }

    /// Find a struct definition by name.
    pub fn get_struct(&self, name: &str) -> Option<&MirStruct> {
        self.structs.iter().find(|s| s.name == name)
    }

    /// Find an enum definition by name.
    pub fn get_enum(&self, name: &str) -> Option<&MirEnum> {
        self.enums.iter().find(|e| e.name == name)
    }

    // -----------------------------------------------------------------------
    // Queries
    // -----------------------------------------------------------------------

    /// Returns `true` if the program has no functions.
    pub fn is_empty(&self) -> bool {
        self.functions.is_empty()
    }

    /// Returns the total number of functions.
    pub fn fn_count(&self) -> usize {
        self.functions.len()
    }

    /// Returns the total number of struct definitions.
    pub fn struct_count(&self) -> usize {
        self.structs.len()
    }

    /// Returns the total number of enum definitions.
    pub fn enum_count(&self) -> usize {
        self.enums.len()
    }

    /// Returns an iterator over all function names.
    pub fn fn_names(&self) -> impl Iterator<Item = &str> {
        self.functions.iter().map(|f| f.name.as_str())
    }
}

impl Default for MirProgram {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::func::MirFn;
    use crate::inst::Terminator;
    use crate::ty::MirTy;
    use crate::value::BlockId;

    fn make_fn(name: &str) -> MirFn {
        let mut f = MirFn::new(name.to_string(), MirTy::Void, false, false);
        f.terminate(BlockId(0), Terminator::Return(None));
        f
    }

    #[test]
    fn test_empty_program() {
        let prog = MirProgram::new();
        assert!(prog.is_empty());
        assert_eq!(prog.fn_count(), 0);
        assert_eq!(prog.struct_count(), 0);
        assert_eq!(prog.enum_count(), 0);
    }

    #[test]
    fn test_add_and_lookup_fn() {
        let mut prog = MirProgram::new();
        prog.add_fn(make_fn("main"));
        prog.add_fn(make_fn("helper"));
        assert_eq!(prog.fn_count(), 2);
        assert!(prog.get_fn("main").is_some());
        assert!(prog.get_fn("helper").is_some());
        assert!(prog.get_fn("missing").is_none());
    }

    #[test]
    fn test_add_and_lookup_struct() {
        let mut prog = MirProgram::new();
        let s = MirStruct::new(
            "Point".to_string(),
            vec![
                ("x".to_string(), MirTy::Float),
                ("y".to_string(), MirTy::Float),
            ],
            true,
        );
        prog.add_struct(s);
        assert_eq!(prog.struct_count(), 1);
        let found = prog.get_struct("Point").expect("Point struct");
        assert_eq!(found.field_count(), 2);
        let (idx, ty) = found.field("x").expect("field x");
        assert_eq!(idx, 0);
        assert_eq!(*ty, MirTy::Float);
    }

    #[test]
    fn test_add_and_lookup_enum() {
        let mut prog = MirProgram::new();
        let e = MirEnum::new(
            "Direction".to_string(),
            vec![
                MirVariant::unit("North".to_string(), 0),
                MirVariant::unit("South".to_string(), 1),
                MirVariant::unit("East".to_string(), 2),
                MirVariant::unit("West".to_string(), 3),
            ],
            true,
        );
        prog.add_enum(e);
        assert_eq!(prog.enum_count(), 1);
        let found = prog.get_enum("Direction").expect("Direction enum");
        assert_eq!(found.variant_count(), 4);
        assert_eq!(found.discriminant_of("North"), Some(0));
        assert_eq!(found.discriminant_of("West"), Some(3));
        assert_eq!(found.discriminant_of("Up"), None);
    }

    #[test]
    fn test_fn_names_iterator() {
        let mut prog = MirProgram::new();
        prog.add_fn(make_fn("alpha"));
        prog.add_fn(make_fn("beta"));
        prog.add_fn(make_fn("gamma"));
        let names: Vec<&str> = prog.fn_names().collect();
        assert_eq!(names, vec!["alpha", "beta", "gamma"]);
    }

    #[test]
    fn test_mir_struct_unit() {
        let s = MirStruct::new("Marker".to_string(), vec![], false);
        assert!(s.is_unit());
        assert_eq!(s.field_count(), 0);
    }

    #[test]
    fn test_mir_variant_kind_field_count() {
        assert_eq!(MirVariantKind::Unit.field_count(), 0);
        assert_eq!(
            MirVariantKind::Positional(vec![MirTy::Int, MirTy::Float]).field_count(),
            2
        );
        assert_eq!(
            MirVariantKind::Named(vec![("x".to_string(), MirTy::Float)]).field_count(),
            1
        );
    }

    #[test]
    fn test_default_is_empty() {
        let prog = MirProgram::default();
        assert!(prog.is_empty());
    }
}
