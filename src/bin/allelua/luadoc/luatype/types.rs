use std::{collections::BTreeMap, fmt};

/// TypeId define a unique type identifier in a [Context].
#[derive(PartialEq, Eq, Clone, Copy, PartialOrd, Ord, Hash)]
pub struct TypeId(pub usize);

impl TypeId {
    pub const NEVER: TypeId = TypeId(0);
    pub const ANY: TypeId = TypeId(1);
    pub const UNKNOWN: TypeId = TypeId(2);
    pub const NIL: TypeId = TypeId(3);
    pub const BOOLEAN: TypeId = TypeId(4);
    pub const NUMBER: TypeId = TypeId(5);
    pub const STRING: TypeId = TypeId(6);
}

impl fmt::Debug for TypeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::NEVER => write!(f, "never"),
            Self::ANY => write!(f, "any"),
            Self::UNKNOWN => write!(f, "unknown"),
            Self::NIL => write!(f, "nil"),
            Self::BOOLEAN => write!(f, "boolean"),
            Self::NUMBER => write!(f, "number"),
            Self::STRING => write!(f, "string"),
            _ => f.debug_tuple("TypeId").field(&self.0).finish(),
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

    pub fn string(value: String) -> Self {
        Self::Literal {
            value: format!("{value:?}"),
            kind: PrimitiveKind::String,
        }
    }

    pub fn number(value: f64) -> Self {
        Self::Literal {
            value: value.to_string(),
            kind: PrimitiveKind::Number,
        }
    }

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
    pub params: Vec<TypeId>,
    pub results: Vec<TypeId>,
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
            .map(|id| format!("{:?}", id))
            .collect::<Vec<_>>()
            .join(", ");

        let results = self
            .results
            .iter()
            .map(|id| format!("{:?}", id))
            .collect::<Vec<_>>()
            .join(", ");

        write!(f, "({params}) -> ({results})")
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct UnionType {
    pub types: Vec<TypeId>,
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
            .map(|id| format!("{id:?}"))
            .collect::<Vec<_>>()
            .join(" | ");

        write!(f, "{str}")
    }
}

/// IfaceType define a Lua table with keys and values of specified types.
/// Every type that contains those fields is assignable to an interface.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct IfaceType {
    pub fields: BTreeMap<TypeId, TypeId>,
}

impl IfaceType {
    pub fn new(fields: impl IntoIterator<Item = (TypeId, TypeId)>) -> Self {
        Self {
            fields: BTreeMap::from_iter(fields),
        }
    }
}

impl fmt::Display for IfaceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let fields = self
            .fields
            .iter()
            .map(|(k, v)| format!(" {k:?}: {v:?};"))
            .collect::<Vec<_>>()
            .join("");

        write!(f, "{{{fields} }}")
    }
}

impl From<IfaceType> for Type {
    fn from(value: IfaceType) -> Self {
        Self::Iface(value)
    }
}
