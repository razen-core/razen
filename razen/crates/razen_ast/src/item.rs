//! Top-level item / declaration AST nodes.
//!
//! Items are the declarations that appear at the top level of a module:
//! functions, structs, enums, traits, impls, type aliases, and use declarations.

use crate::expr::Expr;
use crate::ident::Ident;
use crate::pat::Pattern;
use crate::span::Span;
use crate::stmt::Stmt;
use crate::types::TypeExpr;

// ---------------------------------------------------------------------------
// Top-level item
// ---------------------------------------------------------------------------

/// A top-level declaration.
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    /// A function definition: `act name(...) ReturnType { ... }`.
    Function(FnDef),

    /// A struct definition.
    Struct(StructDef),

    /// An enum definition.
    Enum(EnumDef),

    /// A trait definition.
    Trait(TraitDef),

    /// An `impl` block.
    Impl(ImplBlock),

    /// A type alias: `alias Name = Type`.
    TypeAlias(TypeAliasDef),

    /// A use / import declaration.
    Use(UseDef),

    /// A constant at module level.
    Const(ConstDef),

    /// A shared binding at module level.
    Shared(SharedDef),
}

impl Item {
    pub fn span(&self) -> Span {
        match self {
            Item::Function(f) => f.span,
            Item::Struct(s) => s.span,
            Item::Enum(e) => e.span,
            Item::Trait(t) => t.span,
            Item::Impl(i) => i.span,
            Item::TypeAlias(a) => a.span,
            Item::Use(u) => u.span,
            Item::Const(c) => c.span,
            Item::Shared(s) => s.span,
        }
    }
}

// ---------------------------------------------------------------------------
// Visibility
// ---------------------------------------------------------------------------

/// Visibility of a declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Visibility {
    /// Private (default) — visible only in the current file.
    Private,
    /// `pub` — visible everywhere.
    Public,
    /// `pub(pkg)` — visible within the package.
    PublicPkg,
    /// `pub(mod)` — visible within the module (sibling files).
    PublicMod,
}

impl Default for Visibility {
    fn default() -> Self {
        Visibility::Private
    }
}

// ---------------------------------------------------------------------------
// Attributes
// ---------------------------------------------------------------------------

/// An attribute, e.g. `@test`, `@derive[Debug, Clone]`.
#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    pub name: Ident,
    /// Arguments inside `[...]`, if present.
    pub args: Vec<AttributeArg>,
    pub span: Span,
}

/// An argument inside an attribute's `[...]`.
#[derive(Debug, Clone, PartialEq)]
pub enum AttributeArg {
    /// A simple identifier: `Debug`, `Clone`.
    Ident(Ident),
    /// A key-value pair: `rename_all: "camelCase"`.
    KeyValue {
        key: Ident,
        value: Expr,
        span: Span,
    },
    /// A string literal argument: `"use new_parse instead"`.
    Literal(crate::lit::Literal),
}

// ---------------------------------------------------------------------------
// Function definition
// ---------------------------------------------------------------------------

/// A function definition: `act name(...) ReturnType { body }`.
#[derive(Debug, Clone, PartialEq)]
pub struct FnDef {
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub is_async: bool,
    pub name: Ident,
    pub generic_params: Vec<GenericParam>,
    pub params: Vec<FnParam>,
    pub return_type: Option<TypeExpr>,
    pub where_clause: Vec<WhereBound>,
    pub body: FnBody,
    pub span: Span,
}

/// A function parameter.
#[derive(Debug, Clone, PartialEq)]
pub struct FnParam {
    /// `mut` modifier on `self`.
    pub is_mut: bool,
    pub pattern: Pattern,
    pub ty: Option<TypeExpr>,
    pub span: Span,
}

/// The body of a function — block or expression body (`-> expr`).
#[derive(Debug, Clone, PartialEq)]
pub enum FnBody {
    /// A block body: `{ stmts }`.
    Block {
        stmts: Vec<Stmt>,
        tail: Option<Box<Expr>>,
        span: Span,
    },
    /// An expression body: `-> expr`.
    Expr(Box<Expr>),
    /// No body (trait method signature without default).
    None,
}

// ---------------------------------------------------------------------------
// Generic parameters and where clauses
// ---------------------------------------------------------------------------

/// A generic type parameter, e.g. `T`, `T: Shape`, `T: Shape + Display`.
#[derive(Debug, Clone, PartialEq)]
pub struct GenericParam {
    pub name: Ident,
    pub bounds: Vec<TypeExpr>,
    pub span: Span,
}

/// A where-clause bound, e.g. `T: Clone + Display`.
#[derive(Debug, Clone, PartialEq)]
pub struct WhereBound {
    pub param: Ident,
    pub bounds: Vec<TypeExpr>,
    pub span: Span,
}

// ---------------------------------------------------------------------------
// Struct definition
// ---------------------------------------------------------------------------

/// A struct definition.
#[derive(Debug, Clone, PartialEq)]
pub struct StructDef {
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub name: Ident,
    pub generic_params: Vec<GenericParam>,
    pub where_clause: Vec<WhereBound>,
    pub kind: StructKind,
    pub span: Span,
}

/// The kind of struct.
#[derive(Debug, Clone, PartialEq)]
pub enum StructKind {
    /// Named fields: `struct User { id: int, name: str }`.
    Named { fields: Vec<StructField> },
    /// Tuple struct (newtype): `struct UserId(int)`.
    Tuple { fields: Vec<TypeExpr> },
    /// Unit struct: `struct Marker`.
    Unit,
}

/// A named struct field.
#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
    pub vis: Visibility,
    pub name: Ident,
    pub ty: TypeExpr,
    pub span: Span,
}

// ---------------------------------------------------------------------------
// Enum definition
// ---------------------------------------------------------------------------

/// An enum definition.
#[derive(Debug, Clone, PartialEq)]
pub struct EnumDef {
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub name: Ident,
    pub generic_params: Vec<GenericParam>,
    pub where_clause: Vec<WhereBound>,
    pub variants: Vec<EnumVariant>,
    pub span: Span,
}

/// A single enum variant.
#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariant {
    pub attrs: Vec<Attribute>,
    pub name: Ident,
    pub kind: EnumVariantKind,
    pub span: Span,
}

/// The data carried by an enum variant.
#[derive(Debug, Clone, PartialEq)]
pub enum EnumVariantKind {
    /// Unit variant: `North`.
    Unit,
    /// Positional data: `Circle(float)`.
    Positional { fields: Vec<TypeExpr> },
    /// Named fields: `Click { x: float, y: float }`.
    Named { fields: Vec<StructField> },
}

// ---------------------------------------------------------------------------
// Trait definition
// ---------------------------------------------------------------------------

/// A trait definition.
#[derive(Debug, Clone, PartialEq)]
pub struct TraitDef {
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub name: Ident,
    pub generic_params: Vec<GenericParam>,
    /// Supertrait bounds, e.g. `trait Ord: Eq`.
    pub supertraits: Vec<TypeExpr>,
    pub where_clause: Vec<WhereBound>,
    pub methods: Vec<FnDef>,
    pub span: Span,
}

// ---------------------------------------------------------------------------
// Impl block
// ---------------------------------------------------------------------------

/// An `impl` block.
#[derive(Debug, Clone, PartialEq)]
pub struct ImplBlock {
    pub attrs: Vec<Attribute>,
    pub generic_params: Vec<GenericParam>,
    /// The trait being implemented, if any: `impl Trait for Type`.
    pub trait_name: Option<TypeExpr>,
    /// The type being implemented.
    pub target: TypeExpr,
    pub where_clause: Vec<WhereBound>,
    pub methods: Vec<FnDef>,
    pub span: Span,
}

// ---------------------------------------------------------------------------
// Type alias
// ---------------------------------------------------------------------------

/// A type alias: `alias Name = Type`.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeAliasDef {
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub name: Ident,
    pub generic_params: Vec<GenericParam>,
    pub ty: TypeExpr,
    pub span: Span,
}

// ---------------------------------------------------------------------------
// Use declaration
// ---------------------------------------------------------------------------

/// A use / import declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct UseDef {
    pub vis: Visibility,
    pub path: Vec<Ident>,
    pub kind: UseKind,
    pub span: Span,
}

/// What is being imported.
#[derive(Debug, Clone, PartialEq)]
pub enum UseKind {
    /// Import the whole module: `use math`.
    Module,
    /// Import specific items: `use math { add, Vector }`.
    Items(Vec<UseItem>),
    /// Import with rename: `use utils.network as net`.
    Alias(Ident),
}

/// A single item in a `use ... { item1, item2 }` import.
#[derive(Debug, Clone, PartialEq)]
pub struct UseItem {
    pub name: Ident,
    /// Optional rename: `User as AppUser`.
    pub alias: Option<Ident>,
    pub span: Span,
}

// ---------------------------------------------------------------------------
// Match arm
// ---------------------------------------------------------------------------

/// A single arm in a `match` expression.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    /// Optional guard: `if condition`.
    pub guard: Option<Box<Expr>>,
    pub body: Box<Expr>,
    pub span: Span,
}

// ---------------------------------------------------------------------------
// Module-level const and shared
// ---------------------------------------------------------------------------

/// A constant at module level.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstDef {
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub name: Ident,
    pub ty: TypeExpr,
    pub value: Expr,
    pub span: Span,
}

/// A shared binding at module level.
#[derive(Debug, Clone, PartialEq)]
pub struct SharedDef {
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub name: Ident,
    pub ty: Option<TypeExpr>,
    pub value: Expr,
    pub span: Span,
}
