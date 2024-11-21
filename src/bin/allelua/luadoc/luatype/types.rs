use std::{collections::BTreeMap, fmt};

/// TypeId define a unique type identifier in a [Context].
#[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord, Hash)]
pub struct TypeId(u32);

impl Into<u32> for TypeId {
    fn into(self) -> u32 {
        // 4 bits are used to tag the type id.
        self.0 & ((u32::MAX << 4) >> 4)
    }
}

impl Into<usize> for TypeId {
    fn into(self) -> usize {
        Into::<u32>::into(self) as usize
    }
}

impl TypeId {
    pub const TAG_NEVER: u32 = 0;
    pub const TAG_LITERAL: u32 = 1 << 28;
    pub const TAG_PRIMITIVE: u32 = 2 << 28;
    pub const TAG_FUNCTION: u32 = 3 << 28;
    pub const TAG_UNION: u32 = 4 << 28;
    pub const TAG_IFACE: u32 = 5 << 28;
    pub const TAG_PARAMETER: u32 = 6 << 28;
    pub const TAG_GENERIC: u32 = 7 << 28;
    pub const TAG_ANY: u32 = 8 << 28;
    pub const TAG_UNKNOWN: u32 = 9 << 28;

    pub const NEVER: TypeId = TypeId(Self::TAG_NEVER);
    pub const ANY: TypeId = TypeId(Self::TAG_ANY | 1);
    pub const UNKNOWN: TypeId = TypeId(Self::TAG_UNKNOWN | 2);
    pub const NIL: TypeId = TypeId(Self::TAG_PRIMITIVE | 3);
    pub const BOOLEAN: TypeId = TypeId(Self::TAG_PRIMITIVE | 4);
    pub const NUMBER: TypeId = TypeId(Self::TAG_PRIMITIVE | 5);
    pub const STRING: TypeId = TypeId(Self::TAG_PRIMITIVE | 6);

    pub fn new(t: &Type, id: u32) -> Self {
        if id & (0b1111 << 28) != 0 {
            panic!("id must be comprised between 0 and 268435456 (2 ^ 28)")
        }

        let tag = match t {
            Type::Never | Type::Any | Type::Unknown | Type::Primitive { .. } => {
                panic!("can't create new TypeId for never, any, unknown and primitive types")
            }
            Type::Literal { .. } => Self::TAG_LITERAL,
            Type::Function(_) => Self::TAG_FUNCTION,
            Type::Union(_) => Self::TAG_UNION,
            Type::Iface(_) => Self::TAG_IFACE,
            Type::Parameter(_) => Self::TAG_PARAMETER,
            Type::Generic(_) => Self::TAG_GENERIC,
        };

        Self(tag | id)
    }

    pub fn tag(&self) -> u32 {
        (self.0 >> 28) << 28
    }

    pub fn is_never(&self) -> bool {
        self.tag() == Self::TAG_NEVER
    }

    pub fn is_literal(&self) -> bool {
        self.tag() == Self::TAG_LITERAL
    }

    pub fn is_primitive(&self) -> bool {
        self.tag() == Self::TAG_PRIMITIVE
    }

    pub fn is_function(&self) -> bool {
        self.tag() == Self::TAG_FUNCTION
    }
    pub fn is_union(&self) -> bool {
        self.tag() == Self::TAG_UNION
    }

    pub fn is_iface(&self) -> bool {
        self.tag() == Self::TAG_IFACE
    }

    pub fn is_type_parameter(&self) -> bool {
        self.tag() == Self::TAG_PARAMETER
    }

    pub fn is_generic(&self) -> bool {
        self.tag() == Self::TAG_GENERIC
    }

    pub fn is_any(&self) -> bool {
        self.tag() == Self::TAG_ANY
    }

    pub fn is_unknown(&self) -> bool {
        self.tag() == Self::TAG_UNKNOWN
    }

    pub fn is_composite(&self) -> bool {
        self.tag() == Self::TAG_FUNCTION
            || self.tag() == Self::TAG_UNION
            || self.tag() == Self::TAG_IFACE
    }
}

impl std::str::FromStr for TypeId {
    type Err = std::num::ParseIntError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let u = u32::from_str(value)?;
        Ok(Self(u))
    }
}

impl fmt::Display for TypeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::NEVER => write!(f, "never"),
            Self::ANY => write!(f, "any"),
            Self::UNKNOWN => write!(f, "unknown"),
            Self::NIL => write!(f, "nil"),
            Self::BOOLEAN => write!(f, "boolean"),
            Self::NUMBER => write!(f, "number"),
            Self::STRING => write!(f, "string"),
            _ => {
                write!(f, "TypeId({})", self.0)?;
                if f.alternate() {
                    write!(f, "#")
                } else {
                    Ok(())
                }
            }
        }
    }
}

/// Type define a Lua type in our type system.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum Type {
    Never,
    Literal {
        value: String,
        kind: PrimitiveKind,
    },
    Primitive {
        kind: PrimitiveKind,
        metatable: TypeId,
    },
    Function(FunctionType),
    Union(UnionType),
    Iface(IfaceType),
    Parameter(TypeParameter),
    Generic(GenericType),
    Any,
    Unknown,
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str = match self {
            Self::Never => "never",
            Self::Any => "any",
            Self::Unknown => "unknown",
            Self::Literal { value, .. } => value,
            Self::Primitive { kind, .. } => &kind.to_string(),
            Type::Function(function) => return fmt::Display::fmt(function, f),
            Type::Union(union) => return fmt::Display::fmt(union, f),
            Type::Iface(s) => return fmt::Display::fmt(s, f),
            Type::Parameter(p) => return fmt::Display::fmt(p, f),
            Type::Generic(g) => return fmt::Display::fmt(g, f),
        };

        write!(f, "{str}")
    }
}

impl Type {
    pub const NIL: Type = Type::Primitive {
        kind: PrimitiveKind::Nil,
        metatable: TypeId::NIL,
    };
    pub const BOOLEAN: Type = Type::Primitive {
        kind: PrimitiveKind::Boolean,
        metatable: TypeId::NIL,
    };
    pub const NUMBER: Type = Type::Primitive {
        kind: PrimitiveKind::Number,
        metatable: TypeId::NIL,
    };
    // Todo: add string metatable.
    pub const STRING: Type = Type::Primitive {
        kind: PrimitiveKind::String,
        metatable: TypeId::NIL,
    };

    /// Create a string literal with the given value.
    pub fn string(value: String) -> Self {
        Self::Literal {
            value: format!("{value:?}"),
            kind: PrimitiveKind::String,
        }
    }

    /// Create a number literal with the given value.
    pub fn number(value: f64) -> Self {
        Self::Literal {
            value: value.to_string(),
            kind: PrimitiveKind::Number,
        }
    }

    /// Create a boolean literal with the given value.
    pub fn boolean(value: bool) -> Self {
        Self::Literal {
            value: value.to_string(),
            kind: PrimitiveKind::Boolean,
        }
    }
}

/// PrimitiveKind enumerates all Lua primitive types.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum PrimitiveKind {
    Nil,
    Boolean,
    Number,
    String,
}

impl From<PrimitiveKind> for TypeId {
    fn from(val: PrimitiveKind) -> Self {
        match val {
            PrimitiveKind::Nil => TypeId::NIL,
            PrimitiveKind::Boolean => TypeId::BOOLEAN,
            PrimitiveKind::Number => TypeId::NUMBER,
            PrimitiveKind::String => TypeId::STRING,
        }
    }
}

impl fmt::Display for PrimitiveKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str = match self {
            PrimitiveKind::Nil => "nil",
            PrimitiveKind::Boolean => "boolean",
            PrimitiveKind::Number => "number",
            PrimitiveKind::String => "string",
        };

        write!(f, "{str}")
    }
}

/// FunctionType define a Lua function type.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct FunctionType {
    pub(super) params: Vec<TypeId>,
    pub(super) results: Vec<TypeId>,
}

impl From<FunctionType> for Type {
    fn from(value: FunctionType) -> Self {
        Type::Function(value)
    }
}

impl fmt::Display for FunctionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let params = self
            .params
            .iter()
            .map(|id| {
                if f.alternate() {
                    format!("{:#}", id)
                } else {
                    format!("{id}")
                }
            })
            .collect::<Vec<_>>()
            .join(", ");

        let results = self
            .results
            .iter()
            .map(|id| {
                if f.alternate() {
                    format!("{:#}", id)
                } else {
                    format!("{id}")
                }
            })
            .collect::<Vec<_>>()
            .join(", ");

        if self.results.len() == 1 {
            write!(f, "({params}) -> {results}")
        } else {
            write!(f, "({params}) -> ({results})")
        }
    }
}

/// UnionType define a union of Lua types. A type is assignable to a union
/// if it can be assigned to one of union's type.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct UnionType {
    pub(super) types: Vec<TypeId>,
}

impl UnionType {
    pub fn new(types: impl IntoIterator<Item = TypeId>) -> Self {
        Self {
            types: Vec::from_iter(types),
        }
    }
}

impl From<UnionType> for Type {
    fn from(value: UnionType) -> Self {
        Self::Union(value)
    }
}

impl fmt::Display for UnionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.types.is_empty() {
            return write!(f, "<empty union>");
        }

        let str = self
            .types
            .iter()
            .map(|id| {
                if f.alternate() {
                    format!("{:#}", id)
                } else {
                    format!("{id}")
                }
            })
            .collect::<Vec<_>>()
            .join(" | ");

        write!(f, "{str}")
    }
}

/// IfaceType define a Lua table with keys and values of specified types.
/// Every type that contains those fields is assignable to an interface.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct IfaceType {
    pub(super) fields: BTreeMap<TypeId, TypeId>,
    pub(super) kv: Option<(TypeId, TypeId)>,
}

impl IfaceType {
    pub fn new(
        fields: impl IntoIterator<Item = (TypeId, TypeId)>,
        kv: Option<(TypeId, TypeId)>,
    ) -> Self {
        Self {
            fields: BTreeMap::from_iter(fields),
            kv,
        }
    }
}

impl fmt::Display for IfaceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let fields = self
            .fields
            .iter()
            .map(|(k, v)| {
                if f.alternate() {
                    format!("\n {k}: {v:#},")
                } else {
                    format!(" {k}: {v:#},")
                }
            })
            .collect::<Vec<_>>()
            .join("");

        write!(f, "{{{fields}{}}}", if f.alternate() { "\n" } else { "" })
    }
}

impl From<IfaceType> for Type {
    fn from(value: IfaceType) -> Self {
        Self::Iface(value)
    }
}

/// GenericType define a type constructor in our type system.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct GenericType {
    pub(super) on: TypeId,
    pub(super) params: Vec<TypeId>,
}

impl GenericType {
    pub fn new(on: TypeId, params: Vec<TypeId>) -> Self {
        if !on.is_composite() {
            panic!("only composite types can be generic")
        }

        if params.iter().any(|p| p.tag() != TypeId::TAG_PARAMETER) {
            panic!("generic type params contains non TypeParameter type id")
        }

        Self { on, params }
    }
}

impl From<GenericType> for Type {
    fn from(value: GenericType) -> Self {
        Self::Generic(value)
    }
}

impl fmt::Display for GenericType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let params = self
            .params
            .iter()
            .map(|id| format!("{id:#}"))
            .collect::<Vec<_>>()
            .join(", ");

        write!(f, "<{params}>")?;

        if f.alternate() {
            write!(f, "{:#}", self.on)
        } else {
            write!(f, "{}", self.on)
        }
    }
}

/// TypeParameter define a parameter of a generic type.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct TypeParameter {
    pub(super) name: String,
    pub(super) constraint: TypeParameterConstraint,
}

impl From<TypeParameter> for Type {
    fn from(value: TypeParameter) -> Self {
        Self::Parameter(value)
    }
}

impl fmt::Display for TypeParameter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "{} {:#}", self.name, self.constraint)
        } else {
            write!(f, "{}", self.name)
        }
    }
}

/// TypeParameterConstraint enumerates possible constraint of a type parameter.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum TypeParameterConstraint {
    Extends(TypeId),
}

impl fmt::Display for TypeParameterConstraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Extends(id) => {
                if f.alternate() {
                    write!(f, "extends {id:#}")
                } else {
                    write!(f, "extends {id}")
                }
            }
        }
    }
}
