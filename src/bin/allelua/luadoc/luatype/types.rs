use std::{
    collections::{BTreeMap, HashMap},
    fmt::{self},
    hash::Hash,
    rc::{Rc, Weak},
};

/// TypeRef define either a strong or weak [Type] reference. This is needed
/// to enable self-referential composite types.
#[derive(Clone)]
pub enum TypeRef {
    Strong(Rc<Type>),
    Weak(Weak<Type>),
}

impl fmt::Debug for TypeRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Strong(rc) => f.debug_tuple("Strong").field(rc).finish(),
            Self::Weak(w) => f.debug_tuple("Weak").field(&w.as_ptr()).finish(),
        }
    }
}

impl fmt::Display for TypeRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(t) = self.try_get() {
            fmt::Display::fmt(&t, f)
        } else {
            write!(f, "<self> {:?}", self.as_ptr())
        }
    }
}

impl<T: Into<Type>> From<T> for TypeRef {
    fn from(value: T) -> Self {
        Self::Strong(Rc::new(value.into()))
    }
}

impl From<Rc<Type>> for TypeRef {
    fn from(value: Rc<Type>) -> Self {
        Self::Strong(value)
    }
}

impl From<Weak<Type>> for TypeRef {
    fn from(value: Weak<Type>) -> Self {
        Self::Weak(value)
    }
}

impl Hash for TypeRef {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Self::Strong(rc) => rc.hash(state),
            // We can't upgrade as weak ref means type is recursive and thus it
            // would lead to infinite recursion.
            Self::Weak(w) => w.as_ptr().hash(state),
        }
    }
}

impl PartialEq for TypeRef {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Strong(s), Self::Strong(other)) => s == other,
            _ => self.as_ptr() == other.as_ptr(),
        }
    }
}

impl Eq for TypeRef {}

impl PartialOrd for TypeRef {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TypeRef {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self.try_get(), other.try_get()) {
            (Some(s), Some(other)) => s.cmp(&other),
            _ => self.as_ptr().cmp(&other.as_ptr()),
        }
    }
}

impl TypeRef {
    fn try_get(&self) -> Option<Rc<Type>> {
        match self {
            TypeRef::Strong(rc) => Some(rc.to_owned()),
            TypeRef::Weak(w) => w.upgrade(),
        }
    }

    pub fn get(&self) -> Rc<Type> {
        match self {
            Self::Strong(rc) => rc.to_owned(),
            Self::Weak(w) => w
                .upgrade()
                .expect("Failed to upgrade weak reference, this is likely a bug, please report it at https://github.com/negrel/allelua/issues"),
        }
    }

    pub fn get_noalias(&self) -> TypeRef {
        Type::noalias(self.get())
    }

    pub fn as_ptr(&self) -> *const Type {
        match self {
            TypeRef::Strong(rc) => std::rc::Rc::<Type>::as_ptr(rc),
            TypeRef::Weak(w) => w.as_ptr(),
        }
    }
}

type NormalizeContext = HashMap<TypeRef, TypeRef>;

/// Type define a Lua type in our type system.
#[derive(Debug, PartialEq, Eq, Clone, Hash, PartialOrd, Ord)]
pub enum Type {
    Never,
    Literal { value: String, kind: PrimitiveKind },
    Primitive { kind: PrimitiveKind },
    Function(FunctionType),
    Union(UnionType),
    Iface(IfaceType),
    Alias(AliasType),
    Any,
    Unknown,
}

impl Type {
    pub const BOOLEAN: Type = Type::Primitive {
        kind: PrimitiveKind::Boolean,
    };
    pub const NUMBER: Type = Type::Primitive {
        kind: PrimitiveKind::Number,
    };
    pub const STRING: Type = Type::Primitive {
        kind: PrimitiveKind::String,
    };
    pub const NIL: Type = Type::Primitive {
        kind: PrimitiveKind::Nil,
    };

    pub fn string(value: String) -> Self {
        if value.len() < 2
            || value.as_bytes().first() != Some(&b'"')
            || value.as_bytes().last() != Some(&b'"')
        {
            panic!("literal string is unquoted");
        }

        Self::Literal {
            value,
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

    /// Unwrap aliased type until type is a primitive or a composite type.
    pub fn noalias(itref: impl Into<TypeRef>) -> TypeRef {
        let tref = itref.into();
        match tref.get().as_ref() {
            Self::Alias(a) => a.alias.get_noalias(),
            _ => tref,
        }
    }

    /// Normalizes given type.
    pub fn normalize(itref: impl Into<TypeRef>) -> TypeRef {
        let mut ctx = HashMap::new();
        Self::normalize_with_ctx(itref, &mut ctx)
    }

    fn normalize_with_ctx(itref: impl Into<TypeRef>, ctx: &mut NormalizeContext) -> TypeRef {
        let tref = itref.into();
        if let Some(tref) = ctx.get(&tref) {
            return tref.to_owned();
        }

        let trc = tref.get();

        let normalized = match trc.as_ref() {
            Type::Never
            | Type::Any
            | Type::Unknown
            | Type::Literal { .. }
            | Type::Primitive { .. } => tref.to_owned(),
            Type::Function(_) => FunctionType::normalize_with_ctx(tref.to_owned(), ctx),
            Type::Union(_) => UnionType::normalize_with_ctx(tref.to_owned(), ctx),
            Type::Iface(_) => IfaceType::normalize_with_ctx(tref.to_owned(), ctx),
            Type::Alias(_) => AliasType::normalize_with_ctx(tref.to_owned(), ctx),
        };

        if normalized != tref {
            ctx.insert(tref.to_owned(), normalized.to_owned());
            normalized
        } else {
            ctx.insert(tref.to_owned(), tref.to_owned());
            tref
        }
    }

    /// Returns true if tref contains x.
    pub fn contains(itref: impl Into<TypeRef>, x: TypeRef) -> bool {
        let tref = itref.into();
        match tref.get().as_ref() {
            Type::Never
            | Type::Any
            | Type::Unknown
            | Type::Literal { .. }
            | Type::Primitive { .. } => false,
            Type::Function(f) => f.contains(x),
            Type::Union(u) => u.contains(x),
            Type::Iface(i) => i.contains(x),
            Type::Alias(a) => {
                if a.alias == x {
                    true
                } else {
                    Type::contains(a.alias.clone(), x)
                }
            }
        }
    }

    /// Returns true if type is self-referential.
    pub fn is_cyclic(itref: impl Into<TypeRef>) -> bool {
        let tref = itref.into();
        Self::contains(tref.clone(), tref)
    }
}

impl From<&'static str> for Type {
    fn from(value: &'static str) -> Self {
        Self::string(format!("{:?}", value))
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Never => write!(f, "never"),
            Type::Literal { value, .. } => write!(f, "{}", value),
            Type::Primitive { kind } => fmt::Display::fmt(kind, f),
            Type::Function(function) => fmt::Display::fmt(function, f),
            Type::Union(union) => fmt::Display::fmt(union, f),
            Type::Iface(iface) => fmt::Display::fmt(iface, f),
            Type::Alias(alias) => fmt::Display::fmt(alias, f),
            Type::Any => write!(f, "any"),
            Type::Unknown => write!(f, "unknown"),
        }
    }
}

/// PrimitiveKind enumerates all Lua primitive types.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, PartialOrd, Ord)]
pub enum PrimitiveKind {
    Nil,
    Boolean,
    Number,
    String,
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
#[derive(Debug, PartialEq, Eq, Clone, Hash, PartialOrd, Ord)]
pub struct FunctionType {
    pub params: Vec<TypeRef>,
    pub results: Vec<TypeRef>,
}

impl fmt::Display for FunctionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let params = self
            .params
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(", ");

        if self.results.len() == 1 {
            write!(f, "({params}) -> {}", self.results[0])
        } else {
            write!(
                f,
                "({params}) -> ({})",
                self.results
                    .iter()
                    .map(|p| p.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    }
}

impl From<FunctionType> for Type {
    fn from(value: FunctionType) -> Self {
        Type::Function(value)
    }
}

impl<'a> TryFrom<&'a Type> for &'a FunctionType {
    type Error = &'static str;

    fn try_from(value: &'a Type) -> Result<Self, Self::Error> {
        match value {
            Type::Function(function) => Ok(function),
            _ => Err("Type is not Type::Function"),
        }
    }
}

impl FunctionType {
    pub fn new(
        params: impl IntoIterator<Item = impl Into<TypeRef>>,
        results: impl IntoIterator<Item = impl Into<TypeRef>>,
    ) -> Self {
        Self {
            params: Vec::from_iter(params.into_iter().map(Into::into)),
            results: Vec::from_iter(results.into_iter().map(Into::into)),
        }
    }

    fn contains(&self, x: TypeRef) -> bool {
        if self
            .params
            .iter()
            .any(|param| param == &x || Type::contains(param.to_owned(), x.clone()))
        {
            true
        } else {
            self.results
                .iter()
                .any(|result| result == &x || Type::contains(result.to_owned(), x.clone()))
        }
    }

    fn normalize_with_ctx(function_ref: TypeRef, ctx: &mut NormalizeContext) -> TypeRef {
        let function_rc = function_ref.get();
        let function: &FunctionType = function_rc.as_ref().try_into().unwrap();

        Self::new(
            function
                .params
                .iter()
                .map(|p| Type::normalize_with_ctx(p.to_owned(), ctx))
                .collect::<Vec<_>>(),
            function
                .results
                .iter()
                .map(|r| Type::normalize_with_ctx(r.to_owned(), ctx)),
        )
        .into()
    }
}

/// A union type represents a type that can be one of several variants.
#[derive(Debug, PartialEq, Eq, Clone, Hash, PartialOrd, Ord)]
pub struct UnionType {
    pub variants: Vec<TypeRef>,
}

impl fmt::Display for UnionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            self.variants
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(" | ")
        )
    }
}

impl From<UnionType> for Type {
    fn from(value: UnionType) -> Self {
        Self::Union(value)
    }
}

impl<'a> TryFrom<&'a Type> for &'a UnionType {
    type Error = &'static str;

    fn try_from(value: &'a Type) -> Result<Self, Self::Error> {
        match value {
            Type::Union(u) => Ok(u),
            _ => Err("Type is not Type::Union"),
        }
    }
}

impl UnionType {
    /// Creates a new union type from the given iterator. This function panics
    /// if iterator contains 0 types.
    pub fn new(types: impl IntoIterator<Item = impl Into<TypeRef>>) -> Self {
        let types = Vec::from_iter(types.into_iter().map(Into::into));

        if types.is_empty() {
            panic!("union must contain at least one type")
        }

        Self { variants: types }
    }

    /// Returns true if given type is contained within this type.
    pub fn contains(&self, x: TypeRef) -> bool {
        for trc in &self.variants {
            let tref = trc.to_owned();
            if tref == x || Type::contains(tref, x.clone()) {
                return true;
            }
        }

        false
    }

    /// Simplifies union type by removing duplicates, merging inner unions and
    /// more.
    fn normalize_with_ctx(union_ref: TypeRef, ctx: &mut NormalizeContext) -> TypeRef {
        let union_rc = union_ref.get();
        let union: &UnionType = union_rc.as_ref().try_into().unwrap();

        let mut types: Vec<TypeRef> = Vec::new();

        for trc in &union.variants {
            let normalized = Type::normalize_with_ctx(trc.to_owned(), ctx);

            if let Some(tref) = Self::normalize_insert_variant_type(&mut types, normalized) {
                return tref.into();
            }
        }

        // Remove duplicates (vector is sorted).
        types.dedup();

        if types.len() == 1 {
            return types[0].to_owned();
        }

        Self::new(types).into()
    }

    fn normalize_insert_variant_type(types: &mut Vec<TypeRef>, tref: TypeRef) -> Option<Type> {
        match types.binary_search(&tref) {
            Ok(_) => None,
            Err(pos) => {
                if let Some(rc) = tref.try_get().as_ref() {
                    match rc.as_ref() {
                        Type::Never => return None,
                        Type::Literal { kind: lit_kind, .. } => {
                            if types.iter().any(|v| {
                                if let Some(Type::Primitive { kind }) = v.try_get().as_deref() {
                                    kind == lit_kind
                                } else {
                                    false
                                }
                            }) {
                                return None;
                            }
                        }
                        Type::Primitive { kind } => types.retain(|v| {
                            if let Some(Type::Literal { kind: lit_kind, .. }) =
                                v.try_get().as_deref()
                            {
                                lit_kind != kind
                            } else {
                                true
                            }
                        }),
                        Type::Function(_) => {}
                        Type::Union(u) => {
                            for var in &u.variants {
                                if let Some(t) =
                                    Self::normalize_insert_variant_type(types, var.to_owned())
                                {
                                    return Some(t);
                                }
                            }
                            return None;
                        }
                        Type::Iface(_) => {}
                        Type::Alias(a) => {
                            return Self::normalize_insert_variant_type(types, a.alias.to_owned())
                        }
                        Type::Any => return Some(Type::Any),
                        Type::Unknown => return Some(Type::Any),
                    }
                }

                types.insert(pos, tref);
                None
            }
        }
    }
}

/// Field define an [IfaceType] field. Fields contains a key, a value and an
/// optional comment.
#[derive(Debug, Clone)]
pub(super) struct Field {
    pub comment: String,
    pub key: TypeRef,
    pub value: TypeRef,
}

impl fmt::Display for Field {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(t) = self.key.try_get() {
            if let Type::Literal { value: key, kind } = t.as_ref() {
                let char_count = key.chars().count();

                if kind == &PrimitiveKind::String
                    && key.chars().enumerate().all(|(i, c)| {
                        if c == '"' && (i == 0 || i == char_count - 1) {
                            return true;
                        }

                        !char::is_whitespace(c) && c != '"'
                    })
                {
                    let key = key.strip_prefix('"').unwrap().strip_suffix('"').unwrap();
                    if f.alternate() {
                        return write!(f, "{}: {:#}", key, self.value);
                    } else {
                        return write!(f, "{}: {}", key, self.value);
                    }
                }
            }
        }

        if f.alternate() {
            write!(f, "[{}]: {:#}", self.key, self.value)
        } else {
            write!(f, "[{}]: {}", self.key, self.value)
        }
    }
}

impl PartialEq for Field {
    fn eq(&self, other: &Field) -> bool {
        self.key == other.key && self.value == other.value
    }
}

impl Eq for Field {}

impl PartialOrd for Field {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Field {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.key.cmp(&other.key) {
            core::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        self.value.cmp(&other.value)
    }
}

impl Hash for Field {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.key.hash(state);
        self.value.hash(state)
    }
}

impl<K: Into<TypeRef>, V: Into<TypeRef>> From<(K, V)> for Field {
    fn from((key, value): (K, V)) -> Self {
        Self {
            comment: "".to_owned(),
            key: key.into(),
            value: value.into(),
        }
    }
}

/// IfaceType define a Lua table with keys and values of specified types.
/// Every type that contains those fields is assignable to an interface.
#[derive(Debug, PartialEq, Eq, Clone, Hash, PartialOrd, Ord)]
pub struct IfaceType {
    pub fields: BTreeMap<TypeRef, Field>,
}

impl fmt::Display for IfaceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let fields = self
            .fields
            .values()
            .map(|field| {
                if f.alternate() {
                    format!("\n {field:#}")
                } else {
                    format!(" {field}")
                }
            })
            .collect::<Vec<_>>()
            .join(",");

        write!(f, "{{{fields}{}}}", if f.alternate() { "\n" } else { " " })
    }
}

impl From<IfaceType> for Type {
    fn from(value: IfaceType) -> Self {
        Self::Iface(value)
    }
}

impl<'a> TryFrom<&'a Type> for &'a IfaceType {
    type Error = &'static str;

    fn try_from(value: &'a Type) -> Result<Self, Self::Error> {
        match value {
            Type::Iface(iface) => Ok(iface),
            _ => Err("Type is not Type::Iface"),
        }
    }
}

impl IfaceType {
    fn new(fields: impl IntoIterator<Item = impl Into<Field>>) -> Self {
        Self {
            fields: BTreeMap::from_iter(
                fields
                    .into_iter()
                    .map(Into::into)
                    .map(|f| (f.key.to_owned(), f)),
            ),
        }
    }

    fn contains(&self, x: TypeRef) -> bool {
        self.fields.iter().any(|(_, f)| {
            f.key == x
                || f.value == x
                || Type::contains(f.key.to_owned(), x.clone())
                || Type::contains(f.value.to_owned(), x.clone())
        })
    }

    fn normalize_with_ctx(iface_ref: TypeRef, ctx: &mut NormalizeContext) -> TypeRef {
        let iface_rc = iface_ref.get();
        let iface: &IfaceType = iface_rc.as_ref().try_into().unwrap();

        let mut fields: Vec<Field> = Vec::new();

        for f in iface.fields.values() {
            let mut field = f.to_owned();
            field.key = Type::normalize_with_ctx(f.key.to_owned(), ctx);
            field.value = Type::normalize_with_ctx(f.value.to_owned(), ctx);

            match field.key.get().as_ref() {
                Type::Union(union) => {
                    for tref in &union.variants {
                        field.key = tref.to_owned();
                        fields.push(field.to_owned())
                    }
                }
                _ => fields.push(field),
            }
        }

        Self::new(fields).into()
    }
}

/// AliasType define a named alias for another type.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash)]
pub struct AliasType {
    name: String,
    alias: TypeRef,
    comment: String,
}

impl fmt::Display for AliasType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.name, f)
    }
}

impl From<AliasType> for Type {
    fn from(value: AliasType) -> Self {
        Self::Alias(value)
    }
}

impl<'a> TryFrom<&'a Type> for &'a AliasType {
    type Error = &'static str;

    fn try_from(value: &'a Type) -> Result<Self, Self::Error> {
        match value {
            Type::Alias(alias) => Ok(alias),
            _ => Err("Type is not Type::Alias"),
        }
    }
}

impl AliasType {
    pub fn new(name: String, alias: TypeRef) -> Self {
        Self::new_with_comment(name, alias, "".to_owned())
    }

    pub fn new_with_comment(name: String, alias: TypeRef, comment: String) -> Self {
        if !name.chars().enumerate().all(|(i, c)| {
            if i == 0 {
                char::is_uppercase(c) && char::is_ascii_alphabetic(&c)
            } else {
                char::is_ascii_alphanumeric(&c)
            }
        }) {
            panic!("type alias must start with uppercase ascii alphabetic char and contains only ascii alphanumeric chars")
        }

        Self {
            name,
            alias,
            comment,
        }
    }

    pub fn new_recursive(
        name: String,
        data_fn: impl FnOnce(std::rc::Weak<Type>) -> TypeRef,
    ) -> TypeRef {
        Self::new_recursive_with_comment(name, data_fn, "".to_owned())
    }

    pub fn new_recursive_with_comment(
        name: String,
        data_fn: impl FnOnce(std::rc::Weak<Type>) -> TypeRef,
        comment: String,
    ) -> TypeRef {
        Rc::new_cyclic(|w| Self::new_with_comment(name, data_fn(w.to_owned()), comment).into())
            .into()
    }

    fn normalize_with_ctx(alias_ref: TypeRef, ctx: &mut HashMap<TypeRef, TypeRef>) -> TypeRef {
        let alias_rc = alias_ref.get();
        let alias: &AliasType = alias_rc.as_ref().try_into().unwrap();

        let normalized = AliasType::new_recursive_with_comment(
            alias.name.clone(),
            |w| {
                ctx.insert(Rc::downgrade(&alias_rc).into(), w.to_owned().into());
                let normalized = Type::normalize_with_ctx(alias.alias.to_owned(), ctx);
                ctx.remove(&alias_ref);
                normalized
            },
            alias.comment.clone(),
        );

        let normalized_rc = normalized.get();
        let normalized_alias: &AliasType = normalized_rc.as_ref().try_into().unwrap();
        if normalized_alias.alias == alias.alias {
            alias_ref
        } else {
            normalized
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_bool() {
        assert_eq!(Type::boolean(true).to_string().as_str(), "true");
    }

    #[test]
    fn literal_number() {
        assert_eq!(
            Type::number(std::f64::consts::PI).to_string().as_str(),
            std::f64::consts::PI.to_string(),
        );
    }

    #[test]
    fn literal_negative_number() {
        assert_eq!(Type::number(-1.234).to_string().as_str(), "-1.234");
    }

    #[test]
    #[should_panic(expected = "literal string is unquoted")]
    fn literal_string_panics_if_unquoted() {
        Type::string("foo".to_string());
    }

    #[test]
    fn literal_string() {
        assert_eq!(
            Type::string(r#""foo""#.to_string()).to_string().as_str(),
            r#""foo""#
        );
    }

    #[test]
    fn literal_string_with_inner_quote() {
        assert_eq!(
            Type::string(r#""foo \" foo ""#.to_string())
                .to_string()
                .as_str(),
            r#""foo \" foo ""#
        );
    }

    #[test]
    #[should_panic(expected = "union must contain at least one type")]
    fn empty_union_panics() {
        UnionType::new(Vec::<TypeRef>::new());
    }

    #[test]
    fn union_new_single_type() {
        let any: TypeRef = Rc::new(Type::Any).into();
        let union = UnionType::new(vec![any.to_owned()]);

        assert_eq!(union.variants, vec![any])
    }

    #[test]
    fn union_new_multiple_types() {
        let any: TypeRef = Rc::new(Type::Any).into();
        let number: TypeRef = Rc::new(Type::Primitive {
            kind: PrimitiveKind::Number,
        })
        .into();
        let union = UnionType::new(vec![any.to_owned(), number.to_owned()]);

        assert_eq!(union.variants, vec![any, number])
    }

    #[test]
    fn union_normalize_removes_duplicate() {
        let string: TypeRef = Type::STRING.into();
        let number: TypeRef = Type::NUMBER.into();
        let union = UnionType::new(vec![number.clone(), number.clone(), string.clone()]);
        let normalized = Type::normalize(union);
        let normalized_rc = normalized.get();
        let normalized_union = match normalized_rc.as_ref() {
            Type::Union(u) => u,
            _ => unreachable!(),
        };

        assert_eq!(normalized_union.variants, vec![number, string])
    }

    #[test]
    fn union_normalize_returns_type_if_single_type() {
        let number: TypeRef = Type::NUMBER.into();
        let union = UnionType::new(vec![number.clone(), number.clone()]);
        let normalized = Type::normalize(union);

        assert_eq!(normalized, number)
    }

    #[test]
    fn nested_union_normalize() {
        let number: TypeRef = Type::NUMBER.into();
        let string: TypeRef = Type::STRING.into();
        let nested = UnionType::new(vec![number.to_owned(), string.to_owned()]);
        let union: TypeRef =
            UnionType::new(vec![number.to_owned(), string.to_owned(), nested.into()]).into();
        let normalized = Type::normalize(union);
        let normalized_rc = normalized.get();
        let normalized_union = match normalized_rc.as_ref() {
            Type::Union(u) => u,
            _ => unreachable!(),
        };

        assert_eq!(normalized_union.variants, vec![number, string])
    }

    #[test]
    fn union_display() {
        let number: TypeRef = Type::NUMBER.into();
        let string: TypeRef = Type::STRING.into();
        let union = UnionType::new(vec![number.to_owned(), string.to_owned()]);

        assert_eq!(union.to_string().as_str(), "number | string")
    }

    #[test]
    fn function_no_param_no_result_display() {
        assert_eq!(
            FunctionType::new(Vec::<TypeRef>::new(), Vec::<TypeRef>::new())
                .to_string()
                .as_str(),
            "() -> ()"
        )
    }

    #[test]
    fn function_no_param_single_result_display() {
        assert_eq!(
            FunctionType::new(Vec::<TypeRef>::new(), vec![Type::NUMBER])
                .to_string()
                .as_str(),
            "() -> number"
        )
    }

    #[test]
    fn function_no_param_multiple_results_display() {
        assert_eq!(
            FunctionType::new(Vec::<TypeRef>::new(), vec![Type::NUMBER, Type::NUMBER])
                .to_string()
                .as_str(),
            "() -> (number, number)"
        )
    }

    #[test]
    fn function_multiple_params_multiple_results_display() {
        assert_eq!(
            FunctionType::new(
                vec![Type::NUMBER, Type::NUMBER],
                vec![Type::NUMBER, Type::NUMBER]
            )
            .to_string()
            .as_str(),
            "(number, number) -> (number, number)"
        )
    }

    #[test]
    fn function_normalize() {
        let union = UnionType::new(vec![Type::NUMBER, Type::NUMBER]);
        let function: TypeRef = FunctionType::new(vec![union.clone()], vec![union.clone()]).into();
        let normalize = Type::normalize(function);

        assert_eq!(
            normalize,
            FunctionType::new(vec![Type::NUMBER], vec![Type::NUMBER]).into()
        )
    }

    #[test]
    fn type_ref_hash() {
        let mut hmap = HashMap::<TypeRef, TypeRef>::new();
        let number: TypeRef = Type::NUMBER.into();
        hmap.insert(number.to_owned(), number.to_owned());

        assert!(hmap.contains_key(&Type::NUMBER.into()));
    }

    #[test]
    fn recursive_alias_normalize() {
        let alias_ref = AliasType::new_recursive("Foo".to_owned(), |w| w.to_owned().into());
        let alias_rc = alias_ref.get();
        let alias: &AliasType = alias_rc.as_ref().try_into().unwrap();

        let normalized = Type::normalize(alias_ref.to_owned());
        let normalized_rc = normalized.get();
        let normalized_alias: &AliasType = normalized_rc.as_ref().try_into().unwrap();

        assert_ne!(alias_ref, normalized);

        assert_eq!(alias.alias, Rc::downgrade(&alias_rc).into());
        assert_eq!(normalized_alias.alias, Rc::downgrade(&normalized_rc).into());
    }

    #[test]
    fn iface_normalize() {
        let foo_bar = UnionType::new(vec!["foo", "bar"]);
        let number_nil = UnionType::new(vec![Type::NIL, Type::NUMBER]);

        let iface: TypeRef = IfaceType::new(vec![(foo_bar, number_nil.to_owned())]).into();
        let normalized = Type::normalize(iface.to_owned());

        assert_eq!(
            normalized,
            IfaceType::new(vec![("bar", number_nil.to_owned()), ("foo", number_nil)]).into()
        );
    }

    #[test]
    fn alias_normalize() {
        let alias: TypeRef = AliasType::new("Foo".to_string(), Type::NUMBER.into()).into();
        let normalized = Type::normalize(alias.to_owned());

        assert_eq!(normalized, alias);
        assert_eq!(normalized.as_ptr(), alias.as_ptr());
    }

    #[test]
    fn alias_cyclic_normalize() {
        let alias: TypeRef = AliasType::new("Foo".to_string(), Type::NUMBER.into()).into();
        let normalized = Type::normalize(alias.to_owned());

        assert_eq!(normalized, alias);
        assert_eq!(normalized.as_ptr(), alias.as_ptr());
    }

    #[test]
    fn cyclic_alias_linked_list_normalize() {
        let alias: TypeRef = AliasType::new_recursive("Node".to_string(), |w| {
            IfaceType::new(vec![("next", w.to_owned()), ("prev", w.to_owned())]).into()
        });
        let normalized = Type::normalize(alias.to_owned());

        assert_eq!(
            normalized.clone(),
            AliasType::new(
                "Node".to_string(),
                IfaceType::new(vec![("next", normalized.clone()), ("prev", normalized)]).into()
            )
            .into()
        )
    }
}
