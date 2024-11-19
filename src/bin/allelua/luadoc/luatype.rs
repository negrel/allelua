use std::{
    collections::{BTreeMap, HashMap},
    fmt,
    hash::Hash,
};

use similar::DiffableStr;

/// TypeEnvironment define an environment in our type system.
#[derive(Debug)]
pub struct TypeEnvironment {
    parent: Option<Box<TypeEnvironment>>,
    types: Vec<Type>,
    reverse_lookup: HashMap<Type, TypeId>,
    offset: usize,
}

impl TypeEnvironment {
    /// Creates a new root type environment.
    pub fn new() -> Self {
        let mut env = Self {
            parent: None,
            types: Vec::new(),
            reverse_lookup: HashMap::new(),
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

    /// Creates a new [TypeEnvironment] with [self] as parent environment.
    pub fn child_env(self) -> TypeEnvironment {
        let offset = self.types.len();
        TypeEnvironment {
            parent: Some(Box::new(self)),
            types: Vec::new(),
            reverse_lookup: HashMap::new(),
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

    /// Returns type associated to given id within the environment.
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

    /// Returns type id associated to given type within the environment.
    pub fn reverse_lookup(&self, t: &Type) -> Option<TypeId> {
        if let Some(id) = self.reverse_lookup.get(t) {
            Some(*id)
        } else if let Some(parent) = &self.parent {
            parent.reverse_lookup(t)
        } else {
            None
        }
    }

    /// Registers given type in the environment. If type is already registered,
    /// associated type id is returned.
    pub fn register(&mut self, t: Type) -> TypeId {
        if let Some(id) = self.reverse_lookup(&t) {
            return id;
        }

        self.types.push(t.clone());
        let id = TypeId(self.offset + self.types.len() - 1);
        self.reverse_lookup.insert(t, id);
        id
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

    /// Finds all types contained within type with the given [TypeId] and returns
    /// them.
    pub fn find_associated(&self, id: TypeId) -> Option<Vec<TypeId>> {
        let mut result = Vec::new();

        match self.lookup(id)? {
            Type::Never => {}
            Type::Literal { kind, .. } | Type::Primitive { kind, .. } => {
                result.push((*kind).into())
            }
            Type::Function(function) => {
                result.extend(
                    function
                        .params
                        .iter()
                        .flat_map(|id| self.find_associated(*id))
                        .flatten(),
                );
                result.extend(
                    function
                        .results
                        .iter()
                        .flat_map(|id| self.find_associated(*id))
                        .flatten(),
                )
            }
            Type::Union(u) => result.extend_from_slice(&u.types),
            Type::Iface(iface) => result.extend(
                iface
                    .fields
                    .iter()
                    .flat_map(|(k, v)| vec![self.find_associated(*k), self.find_associated(*v)])
                    .flatten()
                    .flatten(),
            ),
            Type::Any => {}
            Type::Unknown => {}
        }

        Some(result)
    }
}

/// TypeChecker is a Lua type checker. All logic related to type checking is
/// implemented in this type.
#[derive(Debug)]
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

    pub fn register_type(&mut self, t: Type) -> TypeId {
        self.env.register(t)
    }

    /// Search [Type] with given [TypeId] in the current environment and returns it.
    /// A [TypeCheckError::InvalidTypeId] is returned if associated type is not found.
    fn lookup_type(&self, id: TypeId) -> Result<&Type, TypeCheckError> {
        match self.env.lookup(id) {
            Some(t) => Ok(t),
            None => Err(TypeCheckError::InvalidTypeId(id)),
        }
    }

    /// Search [TypeId] of the given [Type] in the current environment and returns it.
    fn lookup_type_id<'a>(&'a self, id: &'a Type) -> Option<TypeId> {
        self.env.reverse_lookup(id)
    }

    fn lookup_type_string(&self, id: TypeId) -> Result<String, TypeCheckError> {
        self.lookup_type(id).map(|t| t.to_string())
    }

    /// Transform given type to it's formatted string representation.
    pub fn fmt(&self, t: &Type) -> String {
        self.environment().replace_type_ids(t.to_string())
    }

    /// Checks whether source [Type] is assignable to target [Type].
    pub fn can_assign(
        &mut self,
        source_id: TypeId,
        target_id: TypeId,
    ) -> Result<(), TypeCheckError> {
        if source_id == target_id {
            return Ok(());
        }

        let source = self.lookup_type(source_id)?.to_owned();
        let source = self.normalize_then_lookup(&source).to_owned();
        let target = self.lookup_type(target_id)?.to_owned();
        let target = self.normalize_then_lookup(&target).to_owned();

        if source == target {
            return Ok(());
        }

        // Handles special case: never, any and unknown.
        {
            // Never is assignable to everything.
            if source == Type::Never {
                return Ok(());
            }
            // Everything is assignable to any and unknown.
            if target == Type::Any || target == Type::Unknown {
                return Ok(());
            }
            // Any is assignable to everything except never.
            if source == Type::Any && target != Type::Never {
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
                if self.can_assign_functions(&source, &target, &mut reasons)? {
                    return Ok(());
                }
            }
            // Source is assignable if it is assignable to one of union's types.
            (_, Type::Union(target_union)) => {
                for target_type_id in target_union.types {
                    match self.can_assign(source_id, target_type_id) {
                        Ok(_) => return Ok(()),
                        Err(err) => reasons.push(err),
                    }
                }
            }
            // Union is assignable if all type of source is assignable to
            // target.
            (Type::Union(source_union), _) => {
                for source_type_id in &source_union.types {
                    match self.can_assign(*source_type_id, target_id) {
                        Ok(_) => return Ok(()),
                        Err(err) => reasons.push(err),
                    }
                }
            }
            (source, Type::Iface(target)) => {
                if self.can_assign_iface(&source, &target, &mut reasons)? {
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
            source_type: source_id,
            target_type: target_id,
            reasons,
        }))
    }

    fn can_assign_functions<'a>(
        &'a mut self,
        source: &'a FunctionType,
        target: &'a FunctionType,
        reasons: &mut Vec<TypeCheckError>,
    ) -> Result<bool, TypeCheckError> {
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
        &'a mut self,
        source: &'a [TypeId],
        target: &'a [TypeId],
    ) -> Result<(), (usize, TypeCheckError)> {
        if source.len() < target.len() {
            // There is more entry in source than target.
            // We check that all source is assignable to target and for
            // i in source.len()..target.len() we check that nil can be assigned to
            // target[i].
            for (i, target_type_id) in target.iter().enumerate() {
                let source_type_id = source.get(i).unwrap_or(&TypeId::NIL);

                self.can_assign(*source_type_id, *target_type_id)
                    .map_err(|e| (i, e))?;
            }
        } else {
            // Source size is larger or equal to target size.
            // We check subset of source is assignable to target type.
            for (i, target_type_id) in target.iter().enumerate() {
                let source_type_id = source[i];

                self.can_assign(source_type_id, *target_type_id)
                    .map_err(|e| (i, e))?;
            }
        }

        Ok(())
    }

    fn can_assign_iface<'a>(
        &mut self,
        source: &'a Type,
        target: &'a IfaceType,
        reasons: &mut Vec<TypeCheckError>,
    ) -> Result<bool, TypeCheckError> {
        let initial_reasons_len = reasons.len();

        // Source is assignable if all fields of target are assignable from
        // source to target. If field is missing in source, nil must be
        // assignable in target.
        for (f_name, f_type_id) in target.fields.iter() {
            match self.get_field_type(source, *f_name) {
                Some(source_f_type_id) => {
                    if let Err(reason) = self.can_assign(source_f_type_id, *f_type_id) {
                        reasons.push(TypeCheckError::IncompatibleFieldType {
                            field_name: self.lookup_type_string(*f_name)?,
                            source: Box::new(reason),
                        })
                    }
                }
                None => {
                    // Field is required but missing in source.
                    if self.can_assign(TypeId::NIL, *f_type_id).is_err() {
                        reasons.push(TypeCheckError::RequiredFieldMissing {
                            field_name: *f_name,
                            field_type: *f_type_id,
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

    pub fn get_field_type<'a>(&'a mut self, t: &'a Type, field: TypeId) -> Option<TypeId> {
        match t {
            Type::Never => Some(TypeId::NEVER),
            Type::Literal { .. } => None,
            // TODO: support metatable __index lookup.
            Type::Primitive { .. } => None,
            Type::Function(_) => None,
            Type::Union(u) => {
                let types = u
                    .types
                    .iter()
                    .filter_map(|id| {
                        let t = self.lookup_type(*id).map(ToOwned::to_owned).unwrap();
                        self.get_field_type(&t, field)
                    })
                    .collect::<Vec<_>>();
                if types.is_empty() {
                    None
                } else {
                    self.normalize(&UnionType::new(types).into())
                }
            }
            Type::Iface(i) => i.fields.get(&field).map(ToOwned::to_owned),
            Type::Any => Some(TypeId::ANY),
            Type::Unknown => None,
        }
    }
}

/// TypeCheckError enumerates errors returned by [TypeChecker] during type
/// checking.
#[derive(Debug, PartialEq)]
pub enum TypeCheckError {
    IncompatibleType(IncompatibleTypeError),
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
        source: Box<TypeCheckError>,
    },
    IncompatibleReturnType {
        nth: usize,
        source: Box<TypeCheckError>,
    },
    IncompatibleFieldType {
        field_name: String,
        source: Box<TypeCheckError>,
    },
    RequiredFieldMissing {
        field_name: TypeId,
        field_type: TypeId,
    },
}

impl std::error::Error for TypeCheckError {}

impl fmt::Display for TypeCheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeCheckError::IncompatibleType(err) => fmt::Display::fmt(err, f),
            TypeCheckError::InvalidTypeId(id) =>write!(f, "Invalid type id {id:?}, this is likely a bug, please report it at https://github.com/negrel/allelua/issues") ,
            TypeCheckError::TargetSignatureTooFewParams { expected, got } => write!(f, "Target signature provides too few parameters. Expected {expected} or more, but got {got}."),
            TypeCheckError::SourceSignatureTooFewResults { expected, got } => write!(f, "Source signature provides too few results. Expected {expected} or more, but got {got}."),
            TypeCheckError::IncompatibleParameterType { nth, source } => write!(f, "Type of parameters {nth} are incompatible.\n{}", space_indent_by(&source.to_string(), 2)),
            TypeCheckError::IncompatibleReturnType { nth, source } => write!(f, "Type of return values {nth} are incompatible.\n{}", space_indent_by(&source.to_string(), 2)),
            TypeCheckError::IncompatibleFieldType { field_name, source } => write!(f, "Type of fields {field_name:?} are incompatible.\n{}", space_indent_by(&source.to_string(), 2)),
            TypeCheckError::RequiredFieldMissing { field_name, field_type } => write!(f,r#"Mandatory field {field_name:?} of type "{field_type:?}" is missing"#)
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct IncompatibleTypeError {
    source_type: TypeId,
    target_type: TypeId,
    reasons: Vec<TypeCheckError>,
}

impl<'a> From<IncompatibleTypeError> for TypeCheckError {
    fn from(value: IncompatibleTypeError) -> Self {
        TypeCheckError::IncompatibleType(value)
    }
}

impl std::error::Error for IncompatibleTypeError {}

impl fmt::Display for IncompatibleTypeError {
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
            r#"Type "{:?}" is not assignable to type "{:?}".{sep}{reasons}"#,
            self.source_type, self.target_type
        )
    }
}

/// TypeId define a unique type identifier in a [Context].
#[derive(PartialEq, Eq, Clone, Copy, PartialOrd, Ord, Hash)]
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

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
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
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct IfaceType {
    fields: BTreeMap<TypeId, TypeId>,
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
            $checker.register_type($t)
        };
    }

    #[test]
    fn type_environment_register_is_idempotent() {
        let mut env = TypeEnvironment::new();
        assert_eq!(env.register(Type::NIL), TypeId::NIL)
    }

    #[test]
    fn any_is_assignable_to_any() {
        let mut checker = TypeChecker::new();
        assert!(checker.can_assign(TypeId::ANY, TypeId::ANY).is_ok());
    }

    #[test]
    fn never_is_assignable_to_any() {
        let mut checker = TypeChecker::new();
        assert!(checker.can_assign(TypeId::NEVER, TypeId::ANY).is_ok());
    }

    #[test]
    fn any_is_not_assignable_to_never() {
        let mut checker = TypeChecker::new();
        assert_eq!(
            checker.can_assign(TypeId::ANY, TypeId::NEVER),
            Err(TypeCheckError::IncompatibleType(IncompatibleTypeError {
                source_type: TypeId::ANY,
                target_type: TypeId::NEVER,
                reasons: Vec::new(),
            }))
        );
    }

    #[test]
    fn any_is_assignable_to_unknown() {
        let mut checker = TypeChecker::new();
        assert!(checker.can_assign(TypeId::ANY, TypeId::UNKNOWN).is_ok());
    }

    #[test]
    fn unknown_is_assignable_to_any() {
        let mut checker = TypeChecker::new();
        assert!(checker.can_assign(TypeId::UNKNOWN, TypeId::ANY).is_ok());
    }

    #[test]
    fn unknown_is_not_assignable_to_never() {
        let mut checker = TypeChecker::new();
        assert_eq!(
            checker.can_assign(TypeId::UNKNOWN, TypeId::NEVER),
            Err(TypeCheckError::IncompatibleType(IncompatibleTypeError {
                source_type: TypeId::UNKNOWN,
                target_type: TypeId::NEVER,
                reasons: Vec::new(),
            }))
        );
    }

    #[test]
    fn literal_is_assignable_to_itself() {
        let mut checker = TypeChecker::new();
        let lit = Type::number(1.0);
        let lit_id = type_id_of!(checker, lit);

        assert!(checker.can_assign(lit_id, lit_id).is_ok())
    }

    #[test]
    fn literal_is_assignable_to_primitive_of_same_kind() {
        let mut checker = TypeChecker::new();
        let lit = Type::number(1.0);
        let lit_id = type_id_of!(checker, lit);

        assert!(checker.can_assign(lit_id, TypeId::NUMBER).is_ok())
    }

    #[test]
    fn primitive_is_assignable_to_itself() {
        let mut checker = TypeChecker::new();

        assert!(checker.can_assign(TypeId::NUMBER, TypeId::NUMBER).is_ok())
    }

    #[test]
    fn function_without_params_and_results_is_assignable_to_itself() {
        let mut checker = TypeChecker::new();
        let function = FunctionType {
            params: vec![],
            results: vec![],
        }
        .into();
        let function_id = type_id_of!(checker, function);

        assert!(checker.can_assign(function_id, function_id).is_ok())
    }

    #[test]
    fn function_with_same_params_and_returns_is_assignable_to_itself() {
        let mut checker = TypeChecker::new();
        let function = FunctionType {
            params: vec![TypeId::NUMBER],
            results: vec![TypeId::STRING],
        }
        .into();
        let function_id = type_id_of!(checker, function);

        assert!(checker.can_assign(function_id, function_id).is_ok())
    }

    #[test]
    fn function_with_number_param_is_assignable_to_function_with_literal_number_param() {
        let mut checker = TypeChecker::new();
        let lit = Type::number(1.0);
        let lit_id = type_id_of!(checker, lit);

        let function1 = FunctionType {
            params: vec![lit_id],
            results: vec![],
        }
        .into();
        let function1_id = type_id_of!(checker, function1);

        let function2 = FunctionType {
            params: vec![TypeId::NUMBER],
            results: vec![],
        }
        .into();
        let function2_id = type_id_of!(checker, function2);

        assert!(checker.can_assign(function2_id, function1_id).is_ok())
    }

    #[test]
    fn function_with_literal_number_param_is_not_assignable_to_function_with_number_param() {
        let mut checker = TypeChecker::new();
        let lit = Type::number(1.0);
        let lit_id = type_id_of!(checker, lit);

        let function1 = FunctionType {
            params: vec![lit_id],
            results: vec![],
        }
        .into();
        let function1_id = type_id_of!(checker, function1);

        let function2 = FunctionType {
            params: vec![TypeId::NUMBER],
            results: vec![],
        }
        .into();
        let function2_id = type_id_of!(checker, function2);

        assert_eq!(
            checker.can_assign(function1_id, function2_id),
            Err(TypeCheckError::IncompatibleType(IncompatibleTypeError {
                source_type: function1_id,
                target_type: function2_id,
                reasons: vec![TypeCheckError::IncompatibleParameterType {
                    nth: 0,
                    source: Box::new(TypeCheckError::IncompatibleType(IncompatibleTypeError {
                        source_type: TypeId::NUMBER,
                        target_type: lit_id,
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
        let lit_id = type_id_of!(checker, lit);

        let function1: Type = FunctionType {
            params: vec![],
            results: vec![lit_id],
        }
        .into();
        let function1_id = type_id_of!(checker, function1);

        let function2: Type = FunctionType {
            params: vec![],
            results: vec![TypeId::NUMBER],
        }
        .into();
        let function2_id = type_id_of!(checker, function2);

        assert!(checker.can_assign(function1_id, function2_id).is_ok())
    }

    #[test]
    fn function_with_number_return_is_not_assignable_to_function_with_literal_number_return() {
        let mut checker = TypeChecker::new();
        let lit = Type::number(1.0);
        let lit_id = type_id_of!(checker, lit);

        let function1: Type = FunctionType {
            params: vec![],
            results: vec![lit_id],
        }
        .into();
        let function1_id = type_id_of!(checker, function1);

        let function2: Type = FunctionType {
            params: vec![],
            results: vec![TypeId::NUMBER],
        }
        .into();
        let function2_id = type_id_of!(checker, function2);

        assert_eq!(
            checker.can_assign(function2_id, function1_id),
            Err(TypeCheckError::IncompatibleType(IncompatibleTypeError {
                source_type: function2_id,
                target_type: function1_id,
                reasons: vec![TypeCheckError::IncompatibleReturnType {
                    nth: 0,
                    source: Box::new(TypeCheckError::IncompatibleType(IncompatibleTypeError {
                        source_type: TypeId::NUMBER,
                        target_type: lit_id,
                        reasons: vec![]
                    }))
                }]
            }))
        )
    }

    #[test]
    fn function_with_1_number_params_is_assignable_to_function_with_2_number_params() {
        let mut checker = TypeChecker::new();
        let function1: Type = FunctionType {
            params: vec![TypeId::NUMBER],
            results: vec![],
        }
        .into();
        let function1_id = type_id_of!(checker, function1);

        let function2: Type = FunctionType {
            params: vec![TypeId::NUMBER, TypeId::NUMBER],
            results: vec![],
        }
        .into();
        let function2_id = type_id_of!(checker, function2);

        assert!(checker.can_assign(function1_id, function2_id).is_ok())
    }

    #[test]
    fn function_with_2_number_params_is_not_assignable_to_function_with_1_number_params() {
        let mut checker = TypeChecker::new();
        let function1: Type = FunctionType {
            params: vec![TypeId::NUMBER],
            results: vec![],
        }
        .into();
        let function1_id = type_id_of!(checker, function1);

        let function2: Type = FunctionType {
            params: vec![TypeId::NUMBER, TypeId::NUMBER],
            results: vec![],
        }
        .into();
        let function2_id = type_id_of!(checker, function2);

        assert_eq!(
            checker.can_assign(function2_id, function1_id),
            Err(TypeCheckError::IncompatibleType(IncompatibleTypeError {
                source_type: function2_id,
                target_type: function1_id,
                reasons: vec![TypeCheckError::IncompatibleParameterType {
                    nth: 1,
                    source: Box::new(TypeCheckError::IncompatibleType(IncompatibleTypeError {
                        source_type: TypeId::NIL,
                        target_type: TypeId::NUMBER,
                        reasons: vec![]
                    }))
                }]
            }))
        )
    }

    #[test]
    fn function_with_2_number_returns_is_assignable_to_function_with_1_number_returns() {
        let mut checker = TypeChecker::new();
        let function1: Type = FunctionType {
            params: vec![],
            results: vec![TypeId::NUMBER],
        }
        .into();
        let function1_id = type_id_of!(checker, function1);

        let function2: Type = FunctionType {
            params: vec![],
            results: vec![TypeId::NUMBER, TypeId::NUMBER],
        }
        .into();
        let function2_id = type_id_of!(checker, function2);

        assert!(checker.can_assign(function2_id, function1_id).is_ok(),)
    }

    #[test]
    fn function_with_1_number_returns_is_not_assignable_to_function_with_2_number_returns() {
        let mut checker = TypeChecker::new();
        let function1: Type = FunctionType {
            params: vec![],
            results: vec![TypeId::NUMBER],
        }
        .into();
        let function1_id = type_id_of!(checker, function1);

        let function2: Type = FunctionType {
            params: vec![],
            results: vec![TypeId::NUMBER, TypeId::NUMBER],
        }
        .into();
        let function2_id = type_id_of!(checker, function2);

        assert_eq!(
            checker.can_assign(function1_id, function2_id),
            Err(TypeCheckError::IncompatibleType(IncompatibleTypeError {
                source_type: function1_id,
                target_type: function2_id,
                reasons: vec![TypeCheckError::IncompatibleReturnType {
                    nth: 1,
                    source: Box::new(TypeCheckError::IncompatibleType(IncompatibleTypeError {
                        source_type: TypeId::NIL,
                        target_type: TypeId::NUMBER,
                        reasons: vec![]
                    }))
                }]
            }))
        )
    }

    #[test]
    fn union_of_literal_is_assignable_to_itself() {
        let mut checker = TypeChecker::new();
        let union_type = UnionType::new(vec![TypeId::NUMBER, TypeId::STRING]).into();
        let union_type_id = type_id_of!(checker, union_type);

        assert!(checker.can_assign(union_type_id, union_type_id).is_ok());
    }

    #[test]
    fn union_of_string_nil_is_not_assignable_to_number() {
        let mut checker = TypeChecker::new();
        let union_type = UnionType::new(vec![TypeId::STRING, TypeId::NIL]).into();
        let union_type_id = type_id_of!(checker, union_type);

        assert_eq!(
            checker.can_assign(union_type_id, TypeId::NUMBER),
            Err(TypeCheckError::IncompatibleType(IncompatibleTypeError {
                source_type: union_type_id,
                target_type: TypeId::NUMBER,
                reasons: vec![
                    TypeCheckError::IncompatibleType(IncompatibleTypeError {
                        source_type: TypeId::STRING,
                        target_type: TypeId::NUMBER,
                        reasons: vec![],
                    }),
                    TypeCheckError::IncompatibleType(IncompatibleTypeError {
                        source_type: TypeId::NIL,
                        target_type: TypeId::NUMBER,
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
        let iface_type_id = type_id_of!(checker, iface_type);

        assert!(checker.can_assign(iface_type_id, iface_type_id).is_ok())
    }

    #[test]
    fn iface_is_assignable_to_empty_iface() {
        let mut checker = TypeChecker::new();
        let empty_iface_type = IfaceType::new(vec![]).into();
        let empty_iface_type_id = type_id_of!(checker, empty_iface_type);
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
        let iface_type_id = type_id_of!(checker, iface_type);

        assert!(checker
            .can_assign(iface_type_id, empty_iface_type_id)
            .is_ok())
    }

    #[test]
    fn empty_iface_is_assignable_to_iface_with_field_union_of_number_nil() {
        let mut checker = TypeChecker::new();
        let empty_iface_type = IfaceType::new(vec![]).into();
        let empty_iface_type_id = type_id_of!(checker, empty_iface_type);

        let number_or_nil = UnionType::new(vec![TypeId::NUMBER, TypeId::NIL]).into();
        let number_or_nil_id = type_id_of!(checker, number_or_nil);

        let iface_type = IfaceType::new(vec![(
            type_id_of!(checker, Type::string("foo".to_owned())),
            number_or_nil_id,
        )])
        .into();
        let iface_type_id = type_id_of!(checker, iface_type);

        assert!(checker
            .can_assign(empty_iface_type_id, iface_type_id)
            .is_ok())
    }

    #[test]
    fn empty_iface_is_not_assignable_to_iface_with_field_number() {
        let mut checker = TypeChecker::new();

        let empty_iface_type = IfaceType::new(vec![]).into();
        let empty_iface_type_id = type_id_of!(checker, empty_iface_type);

        let lit_foo_string = Type::string(r#""foo""#.to_string());
        let lit_foo_string_id = type_id_of!(checker, lit_foo_string);

        let iface_type = IfaceType::new(vec![(lit_foo_string_id, TypeId::NUMBER)]).into();
        let iface_type_id = type_id_of!(checker, iface_type);

        assert_eq!(
            checker.can_assign(empty_iface_type_id, iface_type_id),
            Err(TypeCheckError::IncompatibleType(IncompatibleTypeError {
                source_type: empty_iface_type_id,
                target_type: iface_type_id,
                reasons: vec![TypeCheckError::RequiredFieldMissing {
                    field_name: lit_foo_string_id,
                    field_type: TypeId::NUMBER,
                }]
            }))
        )
    }

    #[test]
    fn union_of_iface_foo_number_iface_bar_number_is_assignable_to_iface_foo_bar_numbers() {
        let mut checker = TypeChecker::new();

        let lit_foo_string = Type::string(r#""foo""#.to_owned());
        let lit_foo_string_id = type_id_of!(checker, lit_foo_string);
        let lit_bar_string = Type::string(r#""foo""#.to_owned());
        let lit_bar_string_id = type_id_of!(checker, lit_bar_string);

        let iface_foo = IfaceType::new(vec![(lit_foo_string_id, TypeId::NUMBER)]).into();
        let iface_foo_id = type_id_of!(checker, iface_foo);
        let iface_bar = IfaceType::new(vec![(lit_bar_string_id, TypeId::NUMBER)]).into();
        let iface_bar_id = type_id_of!(checker, iface_bar);

        let union_iface_foo_iface_bar = UnionType::new(vec![iface_foo_id, iface_bar_id]).into();
        let union_iface_foo_iface_bar_id = type_id_of!(checker, union_iface_foo_iface_bar);

        let iface_foo_bar = IfaceType::new(vec![
            (lit_foo_string_id, TypeId::NUMBER),
            (lit_bar_string_id, TypeId::NUMBER),
        ])
        .into();
        let iface_foo_bar_id = type_id_of!(checker, iface_foo_bar);

        assert!(checker
            .can_assign(union_iface_foo_iface_bar_id, iface_foo_bar_id)
            .is_ok());
    }

    #[test]
    fn iface_foo_bar_numbers_is_assignable_to_union_of_iface_foo_number_iface_bar_number() {
        let mut checker = TypeChecker::new();

        let lit_foo_string = Type::string(r#""foo""#.to_owned());
        let lit_foo_string_id = type_id_of!(checker, lit_foo_string);
        let lit_bar_string = Type::string(r#""foo""#.to_owned());
        let lit_bar_string_id = type_id_of!(checker, lit_bar_string);

        let iface_foo = IfaceType::new(vec![(lit_foo_string_id, TypeId::NUMBER)]).into();
        let iface_foo_id = type_id_of!(checker, iface_foo);
        let iface_bar = IfaceType::new(vec![(lit_bar_string_id, TypeId::NUMBER)]).into();
        let iface_bar_id = type_id_of!(checker, iface_bar);

        let union_iface_foo_iface_bar = UnionType::new(vec![iface_foo_id, iface_bar_id]).into();
        let union_iface_foo_iface_bar_id = type_id_of!(checker, union_iface_foo_iface_bar);

        let iface_foo_bar = IfaceType::new(vec![
            (lit_foo_string_id, TypeId::NUMBER),
            (lit_bar_string_id, TypeId::NUMBER),
        ])
        .into();
        let iface_foo_bar_id = type_id_of!(checker, iface_foo_bar);

        assert!(checker
            .can_assign(iface_foo_bar_id, union_iface_foo_iface_bar_id)
            .is_ok());
    }

    #[test]
    fn normalize_union_of_number_number_returns_number() {
        let mut checker = TypeChecker::new();
        let union_type: Type = UnionType::new(vec![TypeId::NUMBER, TypeId::NUMBER]).into();

        assert_eq!(checker.normalize(&union_type), Some(TypeId::NUMBER));
    }

    #[test]
    fn normalize_union_of_number_never_returns_number() {
        let mut checker = TypeChecker::new();
        let union_type: Type = UnionType::new(vec![TypeId::NUMBER, TypeId::NEVER]).into();
        assert_eq!(checker.normalize(&union_type), Some(TypeId::NUMBER));
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

        let lit = Type::number(1.0);
        let lit_id = type_id_of!(checker, lit);
        let union_type = UnionType::new(vec![TypeId::NUMBER, lit_id]).into();

        assert_eq!(checker.normalize(&union_type), Some(TypeId::NUMBER));
    }

    #[test]
    fn normalize_union_of_number_literal_number_returns_number() {
        let mut checker = TypeChecker::new();

        let lit = Type::number(1.0);
        let lit_id = type_id_of!(checker, lit);
        let union_type = UnionType::new(vec![lit_id, TypeId::NUMBER]).into();

        assert_eq!(checker.normalize(&union_type), Some(TypeId::NUMBER));
    }

    #[test]
    fn normalize_union_of_number_union_of_number_number_returns_number() {
        let mut checker = TypeChecker::new();

        let union_type1: Type = UnionType::new(vec![TypeId::NUMBER, TypeId::NUMBER]).into();
        let union_type1_id = type_id_of!(checker, union_type1);
        let union_type2 = UnionType::new(vec![TypeId::NUMBER, union_type1_id]).into();

        assert_eq!(checker.normalize(&union_type2), Some(TypeId::NUMBER));
    }

    #[test]
    fn normalize_union_of_number_union_of_nil_string_returns_union_of_number_nil_string() {
        let mut checker = TypeChecker::new();

        let union_type1: Type = UnionType::new(vec![TypeId::STRING, TypeId::NIL]).into();
        let union_type1_id = type_id_of!(checker, union_type1);
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
        let number_any_id = type_id_of!(checker, number_any);
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
        let number_any_id = type_id_of!(checker, number_any);
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
        let number_any_id = type_id_of!(checker, number_any);
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
        let literal_foo_string_string_id = type_id_of!(checker, literal_foo_string_string);
        let iface: Type =
            IfaceType::new(vec![(literal_foo_string_string_id, TypeId::NUMBER)]).into();

        let normalized_iface: Type = IfaceType::new(vec![(TypeId::STRING, TypeId::NUMBER)]).into();

        assert_eq!(checker.normalize_then_lookup(&iface), &normalized_iface);
    }
}
