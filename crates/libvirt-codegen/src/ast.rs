//! AST definitions for XDR protocol.

/// Represents a complete XDR protocol definition.
#[derive(Debug, Clone)]
pub struct Protocol {
    pub name: String,
    pub constants: Vec<Constant>,
    pub types: Vec<TypeDef>,
    pub procedures: Vec<Procedure>,
}

impl Protocol {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            constants: Vec::new(),
            types: Vec::new(),
            procedures: Vec::new(),
        }
    }
}

/// A constant definition.
#[derive(Debug, Clone)]
pub struct Constant {
    pub name: String,
    pub value: ConstValue,
}

/// Constant value.
#[derive(Debug, Clone)]
pub enum ConstValue {
    Int(i64),
    Ident(String),
}

/// Type definition.
#[derive(Debug, Clone)]
pub enum TypeDef {
    Struct(StructDef),
    Enum(EnumDef),
    Union(UnionDef),
    Typedef(TypedefDef),
}

/// Struct definition.
#[derive(Debug, Clone)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<Field>,
}

/// Struct field.
#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub ty: Type,
}

/// Enum definition.
#[derive(Debug, Clone)]
pub struct EnumDef {
    pub name: String,
    pub variants: Vec<EnumVariant>,
}

/// Enum variant.
#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: String,
    pub value: Option<ConstValue>,
}

/// Union definition (discriminated union).
#[derive(Debug, Clone)]
pub struct UnionDef {
    pub name: String,
    pub discriminant: Field,
    pub cases: Vec<UnionCase>,
    pub default: Option<Box<Type>>,
}

/// Union case.
#[derive(Debug, Clone)]
pub struct UnionCase {
    pub values: Vec<ConstValue>,
    pub field: Option<Field>,
}

/// Typedef definition.
#[derive(Debug, Clone)]
pub struct TypedefDef {
    pub name: String,
    pub target: Type,
}

/// Type representation.
#[derive(Debug, Clone)]
pub enum Type {
    /// void
    Void,
    /// int
    Int,
    /// unsigned int
    UInt,
    /// hyper
    Hyper,
    /// unsigned hyper
    UHyper,
    /// float
    Float,
    /// double
    Double,
    /// bool
    Bool,
    /// string<N> or string<>
    String { max_len: Option<u32> },
    /// opaque<N> or opaque[N]
    Opaque { len: LengthSpec },
    /// T<N> or T[N] (array)
    Array {
        elem: Box<Type>,
        len: LengthSpec,
    },
    /// T * (optional)
    Optional(Box<Type>),
    /// Named type reference
    Named(String),
}

/// Length specification for arrays and opaque data.
#[derive(Debug, Clone)]
pub enum LengthSpec {
    /// Fixed length [N]
    Fixed(u32),
    /// Variable length <N> or <>
    Variable { max: Option<u32> },
}

/// RPC procedure definition.
#[derive(Debug, Clone)]
pub struct Procedure {
    pub name: String,
    pub number: u32,
    pub args: Option<String>,
    pub ret: Option<String>,
    pub priority: Priority,
}

/// Procedure priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Priority {
    #[default]
    Low,
    High,
}
