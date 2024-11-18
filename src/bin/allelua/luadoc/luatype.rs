use std::{borrow::Cow, collections::HashMap, fmt};

use similar::DiffableStr;

/// TypeEnvironment define an environment in our type system.
pub struct TypeEnvironment {
    parent: Option<Box<TypeEnvironment>>,
    types: Vec<Type>,
    offset: usize,
}

impl TypeEnvironment {
    pub fn new() -> Self {
        let mut env = Self {
            parent: None,
            types: Vec::new(),
            offset: 0,
        };

        // Keep same order as TypeId::XXX values.
        env.register(Type::Never);
        env.register(Type::Any);
        env.register(Type::Unknown);
        env.register(Type::NIL);
        env.register(Type::BOOLEAN);
        env.register(Type::NUMBER);
        env.register(Type::STRING);

        env
    }

    pub fn child_env(self) -> TypeEnvironment {
        let offset = self.types.len();
        TypeEnvironment {
            parent: Some(Box::new(self)),
            types: Vec::new(),
            offset,
        }
    }

    /// Returns parent env along a boolean flag which is true if returned env
    /// is the root env. Calling this function on root environment always returns
    /// `(self, true)`.
    pub fn parent_env(self) -> (TypeEnvironment, bool) {
        match self.parent {
            Some(p) => {
                let is_root = p.parent.is_none();
                (*p, is_root)
            }
            None => (self, true),
        }
    }

    pub fn lookup(&self, id: TypeId) -> Option<&Type> {
        if id.0 < self.offset {
            if let Some(parent) = &self.parent {
                parent.lookup(id)
            } else {
                None
            }
        } else if id.0 < (self.types.len() + self.offset) {
            Some(&self.types[id.0 - self.offset])
        } else {
            None
        }
    }

    pub fn register(&mut self, t: Type) -> TypeId {
        // TODO: return existing type id when already registered.
        self.types.push(t);
        TypeId(self.types.len() - 1)
    }

    /// Replace TypeId(n) in a string with actual types. This is needed as
    /// we can't access environment in fmt::Display implementation of Type.
    pub fn replace_type_ids(&self, s: impl AsRef<str>) -> String {
        let mut str = s.as_ref();

        let mut result = "".to_owned();
        while let Some(i) = str.find("TypeId(") {
            result += str.slice(0..i);
            str = str.slice(i + "TypeId(".len()..str.len());
            if let Some(i) = str.find(")") {
                let type_id: usize = str.slice(0..i).parse().unwrap();
                if let Some(t) = self.lookup(TypeId(type_id)) {
                    result += &self.replace_type_ids(t.to_string());
                }
                str = str.slice(i + 1..str.len());
            }
        }

        result + str
    }
}

/// TypeChecker is a Lua type checker. All logic related to type checking is
/// implemented in this type.
pub struct TypeChecker {
    env: TypeEnvironment,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            env: TypeEnvironment::new(),
        }
    }

    pub fn environment(&self) -> &TypeEnvironment {
        &self.env
    }

    pub fn environment_mut(&mut self) -> &mut TypeEnvironment {
        &mut self.env
    }

    /// Search type with given [TypeId] in the current environment and returns it.
    /// A [TypeCheckError::InvalidTypeId] is returned if associated type is not found.
    fn lookup_type(&self, id: TypeId) -> Result<&Type, TypeCheckError> {
        match self.env.lookup(id) {
            Some(t) => Ok(t),
            None => Err(TypeCheckError::InvalidTypeId(id)),
        }
    }

    fn lookup_type_string(&self, id: TypeId) -> Result<String, TypeCheckError> {
        self.lookup_type(id).map(|t| t.to_string())
    }

    /// Transform given type to it's formatted string representation.
    pub fn fmt(&self, t: &Type) -> String {
        self.environment().replace_type_ids(t.to_string())
    }

    /// Checks whether source [Type] is assignable to target [Type].
    pub fn can_assign<'a>(
        &'a self,
        source: &'a Type,
        target: &'a Type,
    ) -> Result<(), TypeCheckError<'a>> {
        // Handles special case: never, any and unknown.
        {
            // Never is assignable to everything.
            if *source == Type::Never {
                return Ok(());
            }
            // Everything is assignable to any and unknown.
            if *target == Type::Any || *target == Type::Unknown {
                return Ok(());
            }
            // Any is assignable to everything except never.
            if *source == Type::Any && *target != Type::Never {
                return Ok(());
            }
        }

        let mut reasons = Vec::new();

        match (source, target) {
            // Literal can be assigned to literal if they are equal.
            (Type::Literal { value: src, .. }, Type::Literal { value: trg, .. }) => {
                if src == trg {
                    return Ok(());
                }
            }
            // Literal can be assigned to primitive if they share the same kind.
            (Type::Literal { kind: lit_kind, .. }, Type::Primitive { kind, .. }) => {
                if lit_kind == kind {
                    return Ok(());
                }
            }
            // Primitive of same kind are assignable to each other.
            (Type::Primitive { kind: src_kind, .. }, Type::Primitive { kind, .. }) => {
                if src_kind == kind {
                    return Ok(());
                }
            }
            (Type::Function(source), Type::Function(target)) => {
                if self.can_assign_functions(source, target, &mut reasons)? {
                    return Ok(());
                }
            }
            // Source is assignable if it is assignable to one of union's types.
            (source, Type::Union(target_union)) => {
                for target_type_id in &target_union.types {
                    let target = self.lookup_type(*target_type_id)?;

                    match self.can_assign(source, target) {
                        Ok(_) => return Ok(()),
                        Err(err) => reasons.push(err),
                    }
                }
            }
            // Union is assignable if all type of source is assignable to
            // target.
            (Type::Union(source_union), target) => {
                for source_type_id in &source_union.types {
                    let source = self.lookup_type(*source_type_id)?;

                    match self.can_assign(source, target) {
                        Ok(_) => return Ok(()),
                        Err(err) => reasons.push(err),
                    }
                }
            }
            (Type::Iface(source), Type::Iface(target)) => {
                if self.can_assign_iface(source, target, &mut reasons)? {
                    return Ok(());
                }
            }
            (Type::Never, _)
            | (_, Type::Never)
            | (Type::Any, _)
            | (_, Type::Any)
            | (Type::Unknown, _)
            | (_, Type::Unknown) => {}
            _ => {}
        }

        Err(TypeCheckError::IncompatibleType(IncompatibleTypeError {
            source_type: source,
            target_type: target,
            reasons,
        }))
    }

    fn can_assign_functions<'a>(
        &'a self,
        source: &'a FunctionType,
        target: &'a FunctionType,
        reasons: &mut Vec<TypeCheckError<'a>>,
    ) -> Result<bool, TypeCheckError<'a>> {
        let initial_reasons_len = reasons.len();

        // Check that target params are assignable to source params (contravariant).
        match self.can_assign_tuple(&target.params, &source.params) {
            Ok(_) => {}
            Err((i, reason)) => reasons.push(TypeCheckError::IncompatibleParameterType {
                nth: i,
                source: Box::new(reason),
            }),
        }

        // Check that source results are assignable to target results (covariant).
        match self.can_assign_tuple(&source.results, &target.results) {
            Ok(_) => {}
            Err((i, reason)) => reasons.push(TypeCheckError::IncompatibleReturnType {
                nth: i,
                source: Box::new(reason),
            }),
        }

        Ok(initial_reasons_len == reasons.len())
    }

    fn can_assign_tuple<'a>(
        &'a self,
        source: &'a [TypeId],
        target: &'a [TypeId],
    ) -> Result<(), (usize, TypeCheckError<'a>)> {
        if source.len() < target.len() {
            // There is more entry in source than target.
            // We check that all source is assignable to target and for
            // i in source.len()..target.len() we check that nil can be assigned to
            // target[i].
            for (i, target_type_id) in target.iter().enumerate() {
                let source_type_id = source.get(i).unwrap_or(&TypeId::NIL);

                let source_type = self.lookup_type(*source_type_id).map_err(|e| (i, e))?;
                let target_type = self.lookup_type(*target_type_id).map_err(|e| (i, e))?;

                self.can_assign(source_type, target_type)
                    .map_err(|e| (i, e))?;
            }
        } else {
            // Source size is larger or equal to target size.
            // We check subset of source is assignable to target type.
            for (i, target_type_id) in target.iter().enumerate() {
                let source_type_id = source[i];

                let source_type = self.lookup_type(source_type_id).map_err(|e| (i, e))?;
                let target_type = self.lookup_type(*target_type_id).map_err(|e| (i, e))?;

                self.can_assign(source_type, target_type)
                    .map_err(|e| (i, e))?;
            }
        }

        Ok(())
    }

    fn can_assign_iface<'a>(
        &'a self,
        source: &'a IfaceType,
        target: &'a IfaceType,
        reasons: &mut Vec<TypeCheckError<'a>>,
    ) -> Result<bool, TypeCheckError<'a>> {
        let initial_reasons_len = reasons.len();

        // Source is assignable if all fields of target are assignable from
        // source to target. If field is missing in source, nil must be
        // assignable in target.
        for (f_name, f_type_id) in target.fields.iter() {
            let f_type = self.lookup_type(*f_type_id)?;

            match source.fields.get(f_name) {
                Some(source_f_type_id) => {
                    let source_f_type = self.lookup_type(*source_f_type_id)?;

                    if let Err(reason) = self.can_assign(source_f_type, f_type) {
                        reasons.push(TypeCheckError::IncompatibleFieldType {
                            field_name: self.lookup_type_string(*f_name)?,
                            source: Box::new(reason),
                        })
                    }
                }
                None => {
                    // Field is required but missing in source.
                    if self.can_assign(&Type::NIL, f_type).is_err() {
                        reasons.push(TypeCheckError::RequiredFieldMissing {
                            field_name: self.lookup_type_string(*f_name)?,
                            field_type: f_type,
                        })
                    }
                }
            }
        }

        Ok(initial_reasons_len == reasons.len())
    }

    pub fn normalize_then_lookup<'a>(&'a mut self, t: &'a Type) -> &'a Type {
        match self.normalize(t) {
            Some(id) => self.lookup_type(id).unwrap(),
            None => t,
        }
    }

    /// Normalize the given type, adds it to then environment and returns a
    /// reference to it.
    pub fn normalize<'a>(&'a mut self, t: &'a Type) -> Option<TypeId> {
        match t {
            Type::Never
            | Type::Any
            | Type::Unknown
            | Type::Literal { .. }
            | Type::Primitive { .. } => None,
            Type::Function(f) => Some(self.normalize_function(f)),
            Type::Iface(i) => Some(self.normalize_iface(i)),
            Type::Union(u) => Some(self.normalize_union(u)),
        }
    }

    fn normalize_iface(&mut self, iface: &IfaceType) -> TypeId {
        let mut normalized = iface.clone();

        normalized.fields = normalized
            .fields
            .into_iter()
            .map(|(field_name_id, field_type_id)| {
                let normalized_field_name = {
                    let field_name = self
                        .lookup_type(field_name_id)
                        .map(|p| p.to_owned())
                        .unwrap();
                    self.normalize(&field_name).unwrap_or(field_name_id)
                };

                let normalized_field_type = {
                    let field_type = self
                        .lookup_type(field_type_id)
                        .map(|p| p.to_owned())
                        .unwrap();
                    self.normalize(&field_type).unwrap_or(field_type_id)
                };

                (normalized_field_name, normalized_field_type)
            })
            .collect();

        self.env.register(Type::Iface(normalized))
    }

    fn normalize_function(&mut self, function: &FunctionType) -> TypeId {
        let mut normalized = function.clone();

        // Normalize parameters.
        normalized.params = normalized
            .params
            .into_iter()
            .map(|id| {
                let param = self.lookup_type(id).map(|p| p.to_owned()).unwrap();
                self.normalize(&param).unwrap_or(id)
            })
            .collect();

        // Normalize results.
        normalized.results = normalized
            .results
            .into_iter()
            .map(|id| {
                let result = self.lookup_type(id).map(|p| p.to_owned()).unwrap();
                self.normalize(&result).unwrap_or(id)
            })
            .collect();

        self.env.register(Type::Function(normalized))
    }

    fn normalize_union<'a>(&'a mut self, u: &'a UnionType) -> TypeId {
        let mut type_ids = Vec::new();

        type_ids = self.normalize_union_types(&u.types, type_ids);
        match type_ids.len() {
            0 => TypeId::NEVER,
            1 => type_ids[0],
            _ => self.env.register(UnionType::new(type_ids).into()),
        }
    }

    fn normalize_union_types(&self, ids: &Vec<TypeId>, mut result: Vec<TypeId>) -> Vec<TypeId> {
        // Any as priority over unknown so we check it first.
        if ids.contains(&TypeId::ANY) {
            return vec![TypeId::ANY];
        }

        'type_ids_loop: for type_id in ids {
            // Filter duplicate.
            if result.contains(type_id) {
                continue;
            }

            let t = self.env.lookup(*type_id).unwrap();
            match t {
                Type::Never => continue,
                Type::Literal { kind, .. } => {
                    // Skip literal if primitive already present.
                    for id in &result {
                        if *id == (*kind).into() {
                            continue 'type_ids_loop;
                        }
                    }
                }
                Type::Primitive {
                    kind: primitive_kind,
                    ..
                } => {
                    Self::filter_types(&self.env, &mut result, |t| {
                        if let Type::Literal { kind, .. } = t {
                            if kind == primitive_kind {
                                return false;
                            }
                        }
                        true
                    });
                }
                Type::Function(_function) => {
                    // Nothing to do.
                }
                Type::Union(union) => {
                    result = self.normalize_union_types(&union.types, result);
                    continue;
                }
                Type::Iface(_) => {}
                Type::Any => unreachable!(),
                Type::Unknown => return vec![TypeId::UNKNOWN],
            }

            result.push(*type_id);
        }

        result
    }

    fn filter_types(
        env: &TypeEnvironment,
        vec: &mut Vec<TypeId>,
        predicate: impl Fn(&Type) -> bool,
    ) {
        let mut i = 0;
        while i < vec.len() {
            let id = vec[i];
            let t = env.lookup(id).unwrap();

            if !predicate(t) {
                vec.remove(i);
                continue;
            }

            i += 1;
        }
    }
}

/// TypeCheckError enumerates errors returned by [TypeChecker] during type
/// checking.
#[derive(Debug, PartialEq)]
pub enum TypeCheckError<'a> {
    IncompatibleType(IncompatibleTypeError<'a>),
    InvalidTypeId(TypeId),
    TargetSignatureTooFewParams {
        expected: usize,
        got: usize,
    },
    SourceSignatureTooFewResults {
        expected: usize,
        got: usize,
    },
    IncompatibleParameterType {
        nth: usize,
        source: Box<TypeCheckError<'a>>,
    },
    IncompatibleReturnType {
        nth: usize,
        source: Box<TypeCheckError<'a>>,
    },
    IncompatibleFieldType {
        field_name: String,
        source: Box<TypeCheckError<'a>>,
    },
    RequiredFieldMissing {
        field_name: String,
        field_type: &'a Type,
    },
}

impl<'a> std::error::Error for TypeCheckError<'a> {}

impl<'a> fmt::Display for TypeCheckError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeCheckError::IncompatibleType(err) => fmt::Display::fmt(err, f),
            TypeCheckError::InvalidTypeId(id) =>write!(f, "Invalid type id {id:?}, this is likely a bug, please report it at https://github.com/negrel/allelua/issues") ,
            TypeCheckError::TargetSignatureTooFewParams { expected, got } => write!(f, "Target signature provides too few parameters. Expected {expected} or more, but got {got}."),
            TypeCheckError::SourceSignatureTooFewResults { expected, got } => write!(f, "Source signature provides too few results. Expected {expected} or more, but got {got}."),
            TypeCheckError::IncompatibleParameterType { nth, source } => write!(f, "Type of parameters {nth} are incompatible.\n{}", space_indent_by(&source.to_string(), 2)),
            TypeCheckError::IncompatibleReturnType { nth, source } => write!(f, "Type of return values {nth} are incompatible.\n{}", space_indent_by(&source.to_string(), 2)),
            TypeCheckError::IncompatibleFieldType { field_name, source } => write!(f, "Type of fields {field_name:?} are incompatible.\n{}", space_indent_by(&source.to_string(), 2)),
            TypeCheckError::RequiredFieldMissing { field_name, field_type } => write!(f,r#"Mandatory field {field_name:?} of type "{field_type}" is missing"#)
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct IncompatibleTypeError<'a> {
    source_type: &'a Type,
    target_type: &'a Type,
    reasons: Vec<TypeCheckError<'a>>,
}

impl<'a> From<IncompatibleTypeError<'a>> for TypeCheckError<'a> {
    fn from(value: IncompatibleTypeError<'a>) -> Self {
        TypeCheckError::IncompatibleType(value)
    }
}

impl<'a> std::error::Error for IncompatibleTypeError<'a> {}

impl<'a> fmt::Display for IncompatibleTypeError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let reasons = self
            .reasons
            .iter()
            .map(|r| space_indent_by(&r.to_string(), 2))
            .collect::<Vec<_>>()
            .join("\n");

        let sep = if reasons.is_empty() { "" } else { "\n" };

        write!(
            f,
            r#"Type "{}" is not assignable to type "{}".{sep}{reasons}"#,
            self.source_type, self.target_type
        )
    }
}

/// TypeId define a unique type identifier in a [Context].
#[derive(PartialEq, Eq, Clone, Copy, Hash)]
pub struct TypeId(usize);

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
#[derive(Debug, PartialEq, Eq, Clone)]
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
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
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
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct FunctionType {
    params: Vec<TypeId>,
    results: Vec<TypeId>,
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

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct UnionType {
    types: Vec<TypeId>,
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
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct IfaceType {
    fields: HashMap<TypeId, TypeId>,
}

impl IfaceType {
    pub fn new(fields: impl IntoIterator<Item = (TypeId, TypeId)>) -> Self {
        Self {
            fields: HashMap::from_iter(fields),
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

fn space_indent_by(str: &str, n: usize) -> String {
    str.split('\n')
        .map(|line| " ".repeat(n) + line)
        .collect::<Vec<_>>()
        .join("\n")
}

mod tests {
    use super::*;

    macro_rules! type_id_of {
        ($checker:ident, $t:expr) => {
            $checker.environment_mut().register($t)
        };
    }

    #[test]
    fn any_is_assignable_to_any() {
        let checker = TypeChecker::new();
        let any = Type::Any;
        assert!(checker.can_assign(&any, &any).is_ok());
    }

    #[test]
    fn never_is_assignable_to_any() {
        let checker = TypeChecker::new();
        let never = Type::Never;
        let any = Type::Any;
        assert!(checker.can_assign(&never, &any).is_ok());
    }

    #[test]
    fn any_is_not_assignable_to_never() {
        let checker = TypeChecker::new();
        let never = Type::Never;
        let any = Type::Any;
        assert_eq!(
            checker.can_assign(&any, &never),
            Err(TypeCheckError::IncompatibleType(IncompatibleTypeError {
                source_type: &any,
                target_type: &never,
                reasons: Vec::new(),
            }))
        );
    }

    #[test]
    fn any_is_assignable_to_unknown() {
        let checker = TypeChecker::new();
        let any = Type::Any;
        let unknown = Type::Unknown;
        assert!(checker.can_assign(&any, &unknown).is_ok());
    }

    #[test]
    fn unknown_is_assignable_to_any() {
        let checker = TypeChecker::new();
        let any = Type::Any;
        let unknown = Type::Unknown;
        assert!(checker.can_assign(&unknown, &any).is_ok());
    }

    #[test]
    fn unknown_is_not_assignable_to_never() {
        let checker = TypeChecker::new();
        let never = Type::Never;
        let unknown = Type::Unknown;
        assert_eq!(
            checker.can_assign(&unknown, &never),
            Err(TypeCheckError::IncompatibleType(IncompatibleTypeError {
                source_type: &unknown,
                target_type: &never,
                reasons: Vec::new(),
            }))
        );
    }

    #[test]
    fn literal_is_assignable_to_itself() {
        let checker = TypeChecker::new();
        let lit = Type::number(1.0);

        assert!(checker.can_assign(&lit, &lit).is_ok())
    }

    #[test]
    fn literal_is_assignable_to_primitive_of_same_kind() {
        let checker = TypeChecker::new();
        let lit = Type::number(1.0);

        assert!(checker.can_assign(&lit, &Type::NUMBER).is_ok())
    }

    #[test]
    fn primitive_is_assignable_to_itself() {
        let checker = TypeChecker::new();

        assert!(checker.can_assign(&Type::NUMBER, &Type::NUMBER).is_ok())
    }

    #[test]
    fn function_without_params_and_results_is_assignable_to_itself() {
        let checker = TypeChecker::new();
        let function = FunctionType {
            params: vec![],
            results: vec![],
        };

        assert!(checker
            .can_assign(&Type::Function(function.clone()), &Type::Function(function))
            .is_ok())
    }

    #[test]
    fn function_with_same_params_and_returns_is_assignable_to_itself() {
        let checker = TypeChecker::new();
        let function = FunctionType {
            params: vec![TypeId::NUMBER],
            results: vec![TypeId::STRING],
        };

        assert!(checker
            .can_assign(&Type::Function(function.clone()), &Type::Function(function))
            .is_ok())
    }

    #[test]
    fn function_with_number_param_is_assignable_to_function_with_literal_number_param() {
        let mut checker = TypeChecker::new();
        let lit = Type::number(1.0);
        let lit_id = checker.environment_mut().register(lit);

        let function1 = FunctionType {
            params: vec![lit_id],
            results: vec![],
        };
        let function2 = FunctionType {
            params: vec![TypeId::NUMBER],
            results: vec![],
        };

        assert!(checker
            .can_assign(&function2.into(), &function1.into())
            .is_ok())
    }

    #[test]
    fn function_with_literal_number_param_is_not_assignable_to_function_with_number_param() {
        let mut checker = TypeChecker::new();
        let lit = Type::number(1.0);
        let lit_id = checker.environment_mut().register(lit.clone());

        let function1: Type = FunctionType {
            params: vec![lit_id],
            results: vec![],
        }
        .into();
        let function2: Type = FunctionType {
            params: vec![TypeId::NUMBER],
            results: vec![],
        }
        .into();

        assert_eq!(
            checker.can_assign(&function1, &function2),
            Err(TypeCheckError::IncompatibleType(IncompatibleTypeError {
                source_type: &function1,
                target_type: &function2,
                reasons: vec![TypeCheckError::IncompatibleParameterType {
                    nth: 0,
                    source: Box::new(TypeCheckError::IncompatibleType(IncompatibleTypeError {
                        source_type: &Type::NUMBER,
                        target_type: &lit,
                        reasons: vec![]
                    }))
                }]
            }))
        )
    }

    #[test]
    fn function_with_literal_number_return_is_assignable_to_function_with_number_return() {
        let mut checker = TypeChecker::new();
        let lit = Type::number(1.0);
        let lit_id = checker.environment_mut().register(lit.clone());

        let function1: Type = FunctionType {
            params: vec![],
            results: vec![lit_id],
        }
        .into();
        let function2: Type = FunctionType {
            params: vec![],
            results: vec![TypeId::NUMBER],
        }
        .into();

        assert!(checker.can_assign(&function1, &function2).is_ok())
    }

    #[test]
    fn function_with_number_return_is_not_assignable_to_function_with_literal_number_return() {
        let mut checker = TypeChecker::new();
        let lit = Type::number(1.0);
        let lit_id = checker.environment_mut().register(lit.clone());

        let function1: Type = FunctionType {
            params: vec![],
            results: vec![lit_id],
        }
        .into();
        let function2: Type = FunctionType {
            params: vec![],
            results: vec![TypeId::NUMBER],
        }
        .into();

        assert_eq!(
            checker.can_assign(&function2, &function1),
            Err(TypeCheckError::IncompatibleType(IncompatibleTypeError {
                source_type: &function2,
                target_type: &function1,
                reasons: vec![TypeCheckError::IncompatibleReturnType {
                    nth: 0,
                    source: Box::new(TypeCheckError::IncompatibleType(IncompatibleTypeError {
                        source_type: &Type::NUMBER,
                        target_type: &lit,
                        reasons: vec![]
                    }))
                }]
            }))
        )
    }

    #[test]
    fn function_with_1_number_params_is_assignable_to_function_with_2_number_params() {
        let checker = TypeChecker::new();
        let function1: Type = FunctionType {
            params: vec![TypeId::NUMBER],
            results: vec![],
        }
        .into();
        let function2: Type = FunctionType {
            params: vec![TypeId::NUMBER, TypeId::NUMBER],
            results: vec![],
        }
        .into();

        assert!(checker.can_assign(&function1, &function2).is_ok())
    }

    #[test]
    fn function_with_2_number_params_is_not_assignable_to_function_with_1_number_params() {
        let checker = TypeChecker::new();
        let function1: Type = FunctionType {
            params: vec![TypeId::NUMBER],
            results: vec![],
        }
        .into();
        let function2: Type = FunctionType {
            params: vec![TypeId::NUMBER, TypeId::NUMBER],
            results: vec![],
        }
        .into();

        assert_eq!(
            checker.can_assign(&function2, &function1),
            Err(TypeCheckError::IncompatibleType(IncompatibleTypeError {
                source_type: &function2,
                target_type: &function1,
                reasons: vec![TypeCheckError::IncompatibleParameterType {
                    nth: 1,
                    source: Box::new(TypeCheckError::IncompatibleType(IncompatibleTypeError {
                        source_type: &Type::NIL,
                        target_type: &Type::NUMBER,
                        reasons: vec![]
                    }))
                }]
            }))
        )
    }

    #[test]
    fn function_with_2_number_returns_is_assignable_to_function_with_1_number_returns() {
        let checker = TypeChecker::new();
        let function1: Type = FunctionType {
            params: vec![],
            results: vec![TypeId::NUMBER],
        }
        .into();
        let function2: Type = FunctionType {
            params: vec![],
            results: vec![TypeId::NUMBER, TypeId::NUMBER],
        }
        .into();

        assert!(checker.can_assign(&function2, &function1).is_ok(),)
    }

    #[test]
    fn function_with_1_number_returns_is_not_assignable_to_function_with_2_number_returns() {
        let checker = TypeChecker::new();
        let function1: Type = FunctionType {
            params: vec![],
            results: vec![TypeId::NUMBER],
        }
        .into();
        let function2: Type = FunctionType {
            params: vec![],
            results: vec![TypeId::NUMBER, TypeId::NUMBER],
        }
        .into();

        assert_eq!(
            checker.can_assign(&function1, &function2),
            Err(TypeCheckError::IncompatibleType(IncompatibleTypeError {
                source_type: &function1,
                target_type: &function2,
                reasons: vec![TypeCheckError::IncompatibleReturnType {
                    nth: 1,
                    source: Box::new(TypeCheckError::IncompatibleType(IncompatibleTypeError {
                        source_type: &Type::NIL,
                        target_type: &Type::NUMBER,
                        reasons: vec![]
                    }))
                }]
            }))
        )
    }

    #[test]
    fn union_of_literal_is_assignable_to_itself() {
        let checker = TypeChecker::new();
        let union_type = UnionType::new(vec![TypeId::NUMBER, TypeId::STRING]).into();

        assert!(checker.can_assign(&union_type, &union_type).is_ok());
    }

    #[test]
    fn union_of_string_nil_is_not_assignable_to_number() {
        let checker = TypeChecker::new();
        let union_type = UnionType::new(vec![TypeId::STRING, TypeId::NIL]).into();

        assert_eq!(
            checker.can_assign(&union_type, &Type::NUMBER),
            Err(TypeCheckError::IncompatibleType(IncompatibleTypeError {
                source_type: &union_type,
                target_type: &Type::NUMBER,
                reasons: vec![
                    TypeCheckError::IncompatibleType(IncompatibleTypeError {
                        source_type: &Type::STRING,
                        target_type: &Type::NUMBER,
                        reasons: vec![],
                    }),
                    TypeCheckError::IncompatibleType(IncompatibleTypeError {
                        source_type: &Type::NIL,
                        target_type: &Type::NUMBER,
                        reasons: vec![],
                    }),
                ]
            }))
        );
    }

    #[test]
    fn iface_is_assignable_to_itself() {
        let mut checker = TypeChecker::new();
        let iface_type = IfaceType::new(vec![
            (
                type_id_of!(checker, Type::string("foo".to_owned())),
                TypeId::NUMBER,
            ),
            (
                type_id_of!(checker, Type::string("bar".to_owned())),
                TypeId::STRING,
            ),
            (
                type_id_of!(checker, Type::string("baz".to_owned())),
                TypeId::BOOLEAN,
            ),
            (
                type_id_of!(checker, Type::string("qux".to_owned())),
                TypeId::NIL,
            ),
            (
                type_id_of!(checker, Type::string("quz".to_owned())),
                TypeId::ANY,
            ),
        ])
        .into();

        assert!(checker.can_assign(&iface_type, &iface_type).is_ok())
    }

    #[test]
    fn iface_is_assignable_to_empty_iface() {
        let mut checker = TypeChecker::new();
        let empty_iface_type = IfaceType::new(vec![]).into();
        let iface_type = IfaceType::new(vec![
            (
                type_id_of!(checker, Type::string("foo".to_owned())),
                TypeId::NUMBER,
            ),
            (
                type_id_of!(checker, Type::string("bar".to_owned())),
                TypeId::STRING,
            ),
            (
                type_id_of!(checker, Type::string("baz".to_owned())),
                TypeId::BOOLEAN,
            ),
            (
                type_id_of!(checker, Type::string("qux".to_owned())),
                TypeId::NIL,
            ),
            (
                type_id_of!(checker, Type::string("quz".to_owned())),
                TypeId::ANY,
            ),
        ])
        .into();

        assert!(checker.can_assign(&iface_type, &empty_iface_type).is_ok())
    }

    #[test]
    fn empty_iface_is_assignable_to_iface_with_field_union_of_number_nil() {
        let mut checker = TypeChecker::new();
        let empty_iface_type = IfaceType::new(vec![]).into();

        let number_or_nil = UnionType::new(vec![TypeId::NUMBER, TypeId::NIL]).into();
        let number_or_nil_id = checker.environment_mut().register(number_or_nil);

        let iface_type = IfaceType::new(vec![(
            type_id_of!(checker, Type::string("foo".to_owned())),
            number_or_nil_id,
        )])
        .into();

        assert!(checker.can_assign(&empty_iface_type, &iface_type).is_ok())
    }

    #[test]
    fn empty_iface_is_not_assignable_to_iface_with_field_number() {
        let mut checker = TypeChecker::new();
        let empty_iface_type = IfaceType::new(vec![]).into();
        let iface_type = IfaceType::new(vec![(
            type_id_of!(checker, Type::string("foo".to_owned())),
            TypeId::NUMBER,
        )])
        .into();

        assert_eq!(
            checker.can_assign(&empty_iface_type, &iface_type),
            Err(TypeCheckError::IncompatibleType(IncompatibleTypeError {
                source_type: &empty_iface_type,
                target_type: &iface_type,
                reasons: vec![TypeCheckError::RequiredFieldMissing {
                    field_name: r#""foo""#.to_owned(),
                    field_type: &Type::NUMBER,
                }]
            }))
        )
    }

    #[test]
    fn normalize_union_of_number_number_returns_number() {
        let mut checker = TypeChecker::new();
        let union_type = UnionType::new(vec![TypeId::NUMBER, TypeId::NUMBER]);
        assert_eq!(
            checker.normalize_then_lookup(&union_type.into()),
            &Type::NUMBER
        );
    }

    #[test]
    fn normalize_union_of_number_never_returns_number() {
        let mut checker = TypeChecker::new();
        let union_type = UnionType::new(vec![TypeId::NUMBER, TypeId::NEVER]);
        assert_eq!(
            checker.normalize_then_lookup(&union_type.into()),
            &Type::NUMBER
        );
    }

    #[test]
    fn normalize_union_of_number_any_returns_any() {
        let mut checker = TypeChecker::new();
        let union_type = UnionType::new(vec![TypeId::NUMBER, TypeId::ANY]);
        assert_eq!(
            checker.normalize_then_lookup(&union_type.into()),
            &Type::Any
        );
    }

    #[test]
    fn normalize_union_of_number_unknown_returns_unknown() {
        let mut checker = TypeChecker::new();
        let union_type = UnionType::new(vec![TypeId::NUMBER, TypeId::UNKNOWN]);
        assert_eq!(
            checker.normalize_then_lookup(&union_type.into()),
            &Type::Unknown
        );
    }

    #[test]
    fn normalize_union_of_any_unknown_returns_any() {
        let mut checker = TypeChecker::new();
        let union_type = UnionType::new(vec![TypeId::ANY, TypeId::UNKNOWN]);
        assert_eq!(
            checker.normalize_then_lookup(&union_type.into()),
            &Type::Any
        );
    }

    #[test]
    fn normalize_union_of_unknown_any_returns_any() {
        let mut checker = TypeChecker::new();
        let union_type = UnionType::new(vec![TypeId::UNKNOWN, TypeId::ANY]);
        assert_eq!(
            checker.normalize_then_lookup(&union_type.into()),
            &Type::Any
        );
    }

    #[test]
    fn normalize_union_of_number_number_literal_returns_number() {
        let mut checker = TypeChecker::new();
        let env = checker.environment_mut();

        let lit = Type::number(1.0);
        let lit_id = env.register(lit.clone());
        let union_type = UnionType::new(vec![TypeId::NUMBER, lit_id]).into();

        assert_eq!(checker.normalize_then_lookup(&union_type), &Type::NUMBER);
    }

    #[test]
    fn normalize_union_of_number_literal_number_returns_number() {
        let mut checker = TypeChecker::new();
        let env = checker.environment_mut();

        let lit = Type::number(1.0);
        let lit_id = env.register(lit.clone());
        let union_type = UnionType::new(vec![lit_id, TypeId::NUMBER]).into();

        assert_eq!(checker.normalize_then_lookup(&union_type), &Type::NUMBER);
    }

    #[test]
    fn normalize_union_of_number_union_of_number_number_returns_number() {
        let mut checker = TypeChecker::new();
        let env = checker.environment_mut();

        let union_type1: Type = UnionType::new(vec![TypeId::NUMBER, TypeId::NUMBER]).into();
        let union_type1_id = env.register(union_type1.clone());
        let union_type2 = UnionType::new(vec![TypeId::NUMBER, union_type1_id]).into();

        assert_eq!(checker.normalize_then_lookup(&union_type2), &Type::NUMBER);
    }

    #[test]
    fn normalize_union_of_number_union_of_nil_string_returns_union_of_number_nil_string() {
        let mut checker = TypeChecker::new();
        let env = checker.environment_mut();

        let union_type1: Type = UnionType::new(vec![TypeId::STRING, TypeId::NIL]).into();
        let union_type1_id = env.register(union_type1.clone());
        let union_type2 = UnionType::new(vec![TypeId::NUMBER, union_type1_id]).into();

        let union_type3 = UnionType::new(vec![TypeId::NUMBER, TypeId::STRING, TypeId::NIL]).into();
        assert_eq!(checker.normalize_then_lookup(&union_type2), &union_type3);
    }

    #[test]
    fn normalize_empty_union_returns_never() {
        let mut checker = TypeChecker::new();

        let union_type1: Type = UnionType::new(vec![]).into();
        assert_eq!(checker.normalize_then_lookup(&union_type1), &Type::Never);
    }

    #[test]
    fn normalize_union_with_1_type_returns_it() {
        let mut checker = TypeChecker::new();

        let union_type1: Type = UnionType::new(vec![TypeId::NIL]).into();
        assert_eq!(checker.normalize_then_lookup(&union_type1), &Type::NIL);
    }

    #[test]
    fn normalize_function_with_arg_union_of_number_any() {
        let mut checker = TypeChecker::new();
        let number_any = UnionType::new(vec![TypeId::NUMBER, TypeId::ANY]).into();
        let number_any_id = checker.environment_mut().register(number_any);
        let function: Type = FunctionType {
            params: vec![number_any_id],
            results: vec![],
        }
        .into();

        let normalized_function: Type = FunctionType {
            params: vec![TypeId::ANY],
            results: vec![],
        }
        .into();

        assert_eq!(
            checker.normalize_then_lookup(&function),
            &normalized_function
        );
    }

    #[test]
    fn normalize_function_with_result_union_of_number_any() {
        let mut checker = TypeChecker::new();
        let number_any = UnionType::new(vec![TypeId::NUMBER, TypeId::ANY]).into();
        let number_any_id = checker.environment_mut().register(number_any);
        let function: Type = FunctionType {
            params: vec![],
            results: vec![number_any_id],
        }
        .into();

        let normalized_function: Type = FunctionType {
            params: vec![],
            results: vec![TypeId::ANY],
        }
        .into();

        assert_eq!(
            checker.normalize_then_lookup(&function),
            &normalized_function
        );
    }

    #[test]
    fn normalize_iface_with_field_union_of_number_any() {
        let mut checker = TypeChecker::new();
        let number_any = UnionType::new(vec![TypeId::NUMBER, TypeId::ANY]).into();
        let number_any_id = checker.environment_mut().register(number_any);
        let lit_foo_type_id = type_id_of!(checker, Type::string("foo".to_owned()));
        let iface: Type = IfaceType::new(vec![(lit_foo_type_id, number_any_id)]).into();

        let normalized_iface: Type = IfaceType::new(vec![(lit_foo_type_id, TypeId::ANY)]).into();

        assert_eq!(checker.normalize_then_lookup(&iface), &normalized_iface);
    }

    #[test]
    fn normalize_iface_with_field_name_union_of_literal_foo_string_or_string() {
        let mut checker = TypeChecker::new();
        let lit_foo_type_id = type_id_of!(checker, Type::string("foo".to_owned()));
        let literal_foo_string_string =
            UnionType::new(vec![lit_foo_type_id, TypeId::STRING]).into();
        let literal_foo_string_string_id = checker
            .environment_mut()
            .register(literal_foo_string_string);
        let iface: Type =
            IfaceType::new(vec![(literal_foo_string_string_id, TypeId::NUMBER)]).into();

        let normalized_iface: Type = IfaceType::new(vec![(TypeId::STRING, TypeId::NUMBER)]).into();

        assert_eq!(checker.normalize_then_lookup(&iface), &normalized_iface);
    }
}
