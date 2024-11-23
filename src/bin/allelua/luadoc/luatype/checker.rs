use std::fmt;

use super::{FunctionType, IfaceType, Type, TypeEnvironment, TypeId, UnionType};

/// TypeChecker is a Lua type checker. All logic related to type checking is
/// implemented in this type.
#[derive(Debug)]
pub struct TypeChecker {
    env: TypeEnvironment,
}

macro_rules! literal_constructor {
    ($name:ident, $name_then_lookup:ident, $type:ty) => {
        pub fn $name(&mut self, arg: $type) -> TypeId {
            self.env.$name(arg)
        }
        pub fn $name_then_lookup(&mut self, arg: $type) -> &Type {
            let id = self.$name(arg);
            self.lookup_type(id).unwrap()
        }
    };
}

macro_rules! forward_composite_constructor {
    ($name:ident, $name_then_lookup:ident, $type:ty) => {
        pub fn $name(&mut self, arg: $type) -> TypeId {
            self.env.$name(arg)
        }
        pub fn $name_then_lookup(&mut self, arg: $type) -> &Type {
            let id = self.$name(arg);
            self.lookup_type(id).unwrap()
        }
    };
    ($name:ident, $name_then_lookup:ident, $type1:ty, $type2:ty) => {
        pub fn $name(&mut self, arg1: $type1, arg2: $type2) -> TypeId {
            self.env.$name(arg1, arg2)
        }
        pub fn $name_then_lookup(&mut self, arg1: $type1, arg2: $type2) -> &Type {
            let id = self.$name(arg1, arg2);
            self.lookup_type(id).unwrap()
        }
    };
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            env: TypeEnvironment::new(),
        }
    }

    literal_constructor!(boolean, boolean_then_lookup, bool);
    literal_constructor!(number, number_then_lookup, f64);
    literal_constructor!(string, string_then_lookup, String);
    forward_composite_constructor!(function, function_then_lookup, Vec<TypeId>, Vec<TypeId>);
    forward_composite_constructor!(union, union_then_lookup, Vec<TypeId>);
    forward_composite_constructor!(
        iface,
        iface_then_lookup,
        impl IntoIterator<Item = (TypeId, TypeId)>
    );

    /// Returns current type environment.
    pub fn environment(&self) -> &TypeEnvironment {
        &self.env
    }

    /// Registers a type in current type environment.
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
    fn lookup_type_id<'a>(&'a self, t: &'a Type) -> Option<TypeId> {
        self.env.reverse_lookup(t)
    }

    /// Transform given type to it's formatted string representation.
    pub fn fmt(&self, t: &Type, alternate: bool) -> String {
        self.environment().fmt(t.to_string(), alternate)
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
            (source, Type::Iface(target)) => {
                if self.can_assign_iface(&source, &target, &mut reasons)? {
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
                            field_name: self.lookup_type(*f_name).map(|t| t.to_string())?,
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
            Type::Function(f) | Type::Require(f) => Some(self.normalize_function(f)),
            Type::Iface(i) | Type::Global(i) => Some(self.normalize_iface(i)),
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
            _ => self.env.union(type_ids),
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
                Type::Function(_) | Type::Require(_) => {
                    // Nothing to do.
                }
                Type::Union(union) => {
                    result = self.normalize_union_types(&union.types, result);
                    continue;
                }
                Type::Iface(_) | Type::Global(_) => {}
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
            Type::Function(_) | Type::Require(_) => None,
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
                    let id = self.env.union(types);
                    let u = self.lookup_type(id).unwrap();
                    self.normalize(&u.to_owned())
                }
            }
            Type::Iface(i) | Type::Global(i) => i.fields.get(&field).map(ToOwned::to_owned),
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

fn space_indent_by(str: &str, n: usize) -> String {
    str.split('\n')
        .map(|line| " ".repeat(n) + line)
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let lit_id = checker.number(1.0);

        assert!(checker.can_assign(lit_id, lit_id).is_ok())
    }

    #[test]
    fn literal_is_assignable_to_primitive_of_same_kind() {
        let mut checker = TypeChecker::new();
        let lit_id = checker.number(1.0);

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
        let function_id = checker.function(vec![], vec![]);

        assert!(checker.can_assign(function_id, function_id).is_ok())
    }

    #[test]
    fn function_with_same_params_and_returns_is_assignable_to_itself() {
        let mut checker = TypeChecker::new();
        let function_id = checker.function(vec![TypeId::NUMBER], vec![TypeId::STRING]);

        assert!(checker.can_assign(function_id, function_id).is_ok())
    }

    #[test]
    fn function_with_number_param_is_assignable_to_function_with_literal_number_param() {
        let mut checker = TypeChecker::new();
        let lit_id = checker.number(1.0);
        let function1_id = checker.function(vec![lit_id], vec![]);
        let function2_id = checker.function(vec![TypeId::NUMBER], vec![]);

        assert!(checker.can_assign(function2_id, function1_id).is_ok())
    }

    #[test]
    fn function_with_literal_number_param_is_not_assignable_to_function_with_number_param() {
        let mut checker = TypeChecker::new();
        let lit_id = checker.number(1.0);

        let function1_id = checker.function(vec![lit_id], vec![]);
        let function2_id = checker.function(vec![TypeId::NUMBER], vec![]);

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
        let lit_id = checker.number(1.0);
        let function1_id = checker.function(vec![], vec![lit_id]);
        let function2_id = checker.function(vec![], vec![TypeId::NUMBER]);

        assert!(checker.can_assign(function1_id, function2_id).is_ok())
    }

    #[test]
    fn function_with_number_return_is_not_assignable_to_function_with_literal_number_return() {
        let mut checker = TypeChecker::new();
        let lit_id = checker.number(1.0);
        let function1_id = checker.function(vec![], vec![lit_id]);
        let function2_id = checker.function(vec![], vec![TypeId::NUMBER]);

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
        let function1_id = checker.function(vec![TypeId::NUMBER], vec![]);
        let function2_id = checker.function(vec![TypeId::NUMBER, TypeId::NUMBER], vec![]);

        assert!(checker.can_assign(function1_id, function2_id).is_ok())
    }

    #[test]
    fn function_with_2_number_params_is_not_assignable_to_function_with_1_number_params() {
        let mut checker = TypeChecker::new();
        let function1_id = checker.function(vec![TypeId::NUMBER], vec![]);
        let function2_id = checker.function(vec![TypeId::NUMBER, TypeId::NUMBER], vec![]);

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
        let function1_id = checker.function(vec![], vec![TypeId::NUMBER]);
        let function2_id = checker.function(vec![], vec![TypeId::NUMBER, TypeId::NUMBER]);

        assert!(checker.can_assign(function2_id, function1_id).is_ok(),)
    }

    #[test]
    fn function_with_1_number_returns_is_not_assignable_to_function_with_2_number_returns() {
        let mut checker = TypeChecker::new();
        let function1_id = checker.function(vec![], vec![TypeId::NUMBER]);
        let function2_id = checker.function(vec![], vec![TypeId::NUMBER, TypeId::NUMBER]);

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
        let union_type_id = checker.union(vec![TypeId::NUMBER, TypeId::STRING]);

        assert!(checker.can_assign(union_type_id, union_type_id).is_ok());
    }

    #[test]
    fn union_of_string_nil_is_not_assignable_to_number() {
        let mut checker = TypeChecker::new();
        let union_type_id = checker.union(vec![TypeId::STRING, TypeId::NIL]);

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
        let (foo_id, bar_id, baz_id, qux_id, quz_id) = (
            checker.string(r#""foo""#.to_string()),
            checker.string(r#""bar""#.to_string()),
            checker.string(r#""baz""#.to_string()),
            checker.string(r#""qux""#.to_string()),
            checker.string(r#""quz""#.to_string()),
        );
        let iface_type_id = checker.iface(vec![
            (foo_id, TypeId::NUMBER),
            (bar_id, TypeId::STRING),
            (baz_id, TypeId::BOOLEAN),
            (qux_id, TypeId::NIL),
            (quz_id, TypeId::ANY),
        ]);

        assert!(checker.can_assign(iface_type_id, iface_type_id).is_ok())
    }

    #[test]
    fn iface_is_assignable_to_empty_iface() {
        let mut checker = TypeChecker::new();
        let empty_iface_type_id = checker.iface(vec![]).to_owned();

        let (foo_id, bar_id, baz_id, qux_id, quz_id) = (
            checker.string(r#""foo""#.to_string()),
            checker.string(r#""bar""#.to_string()),
            checker.string(r#""baz""#.to_string()),
            checker.string(r#""qux""#.to_string()),
            checker.string(r#""quz""#.to_string()),
        );
        let iface_type_id = checker.iface(vec![
            (foo_id, TypeId::NUMBER),
            (bar_id, TypeId::STRING),
            (baz_id, TypeId::BOOLEAN),
            (qux_id, TypeId::NIL),
            (quz_id, TypeId::ANY),
        ]);

        assert!(checker
            .can_assign(iface_type_id, empty_iface_type_id)
            .is_ok())
    }

    #[test]
    fn empty_iface_is_assignable_to_iface_with_field_union_of_number_nil() {
        let mut checker = TypeChecker::new();
        let empty_iface_type_id = checker.iface(vec![]).to_owned();
        let number_or_nil_id = checker.union(vec![TypeId::NUMBER, TypeId::NIL]);
        let foo_id = checker.string(r#""foo""#.to_owned());

        let iface_type_id = checker.iface(vec![(foo_id, number_or_nil_id)]);

        assert!(checker
            .can_assign(empty_iface_type_id, iface_type_id)
            .is_ok())
    }

    #[test]
    fn empty_iface_is_not_assignable_to_iface_with_field_number() {
        let mut checker = TypeChecker::new();
        let empty_iface_type_id = checker.iface(vec![]);
        let lit_foo_string_id = checker.string(r#""foo""#.to_string());
        let iface_type_id = checker.iface(vec![(lit_foo_string_id, TypeId::NUMBER)]);

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
    fn union_of_iface_foo_number_iface_bar_number_is_not_assignable_to_iface_foo_bar_numbers() {
        let mut checker = TypeChecker::new();

        let lit_foo_string_id = checker.string(r#""foo""#.to_owned());
        let lit_bar_string_id = checker.string(r#""bar""#.to_owned());

        let iface_foo_id = checker.iface(vec![(lit_foo_string_id, TypeId::NUMBER)]);
        let iface_bar_id = checker.iface(vec![(lit_bar_string_id, TypeId::NUMBER)]);

        let union_iface_foo_iface_bar_id = checker.union(vec![iface_foo_id, iface_bar_id]);

        let iface_foo_bar_id = checker.iface(vec![
            (lit_foo_string_id, TypeId::NUMBER),
            (lit_bar_string_id, TypeId::NUMBER),
        ]);

        assert_eq!(
            checker.can_assign(union_iface_foo_iface_bar_id, iface_foo_bar_id),
            Err(TypeCheckError::IncompatibleType(IncompatibleTypeError {
                source_type: union_iface_foo_iface_bar_id,
                target_type: iface_foo_bar_id,
                reasons: vec![
                    TypeCheckError::RequiredFieldMissing {
                        field_name: lit_foo_string_id,
                        field_type: TypeId::NUMBER
                    },
                    TypeCheckError::RequiredFieldMissing {
                        field_name: lit_bar_string_id,
                        field_type: TypeId::NUMBER
                    },
                ],
            }))
        );
    }

    #[test]
    fn iface_foo_bar_numbers_is_assignable_to_union_of_iface_foo_number_iface_bar_number() {
        let mut checker = TypeChecker::new();

        let lit_foo_string_id = checker.string(r#""foo""#.to_owned());
        let lit_bar_string_id = checker.string(r#""bar""#.to_owned());

        let iface_foo_id = checker.iface(vec![(lit_foo_string_id, TypeId::NUMBER)]);
        let iface_bar_id = checker.iface(vec![(lit_bar_string_id, TypeId::NUMBER)]);

        let union_iface_foo_iface_bar_id = checker.union(vec![iface_foo_id, iface_bar_id]);

        let iface_foo_bar_id = checker.iface(vec![
            (lit_foo_string_id, TypeId::NUMBER),
            (lit_bar_string_id, TypeId::NUMBER),
        ]);

        assert!(checker
            .can_assign(iface_foo_bar_id, union_iface_foo_iface_bar_id)
            .is_ok());
    }

    #[test]
    fn normalize_union_of_number_number_returns_number() {
        let mut checker = TypeChecker::new();
        let union_type = checker
            .union_then_lookup(vec![TypeId::NUMBER, TypeId::NUMBER])
            .to_owned();

        assert_eq!(checker.normalize(&union_type), Some(TypeId::NUMBER));
    }

    #[test]
    fn normalize_union_of_number_never_returns_number() {
        let mut checker = TypeChecker::new();
        let union_type = checker
            .union_then_lookup(vec![TypeId::NUMBER, TypeId::NEVER])
            .to_owned();
        assert_eq!(checker.normalize(&union_type), Some(TypeId::NUMBER));
    }

    #[test]
    fn normalize_union_of_number_any_returns_any() {
        let mut checker = TypeChecker::new();
        let union_type = checker
            .union_then_lookup(vec![TypeId::NUMBER, TypeId::ANY])
            .to_owned();
        assert_eq!(checker.normalize_then_lookup(&union_type), &Type::Any);
    }

    #[test]
    fn normalize_union_of_number_unknown_returns_unknown() {
        let mut checker = TypeChecker::new();
        let union_type = checker
            .union_then_lookup(vec![TypeId::NUMBER, TypeId::UNKNOWN])
            .to_owned();
        assert_eq!(checker.normalize_then_lookup(&union_type), &Type::Unknown);
    }

    #[test]
    fn normalize_union_of_any_unknown_returns_any() {
        let mut checker = TypeChecker::new();
        let union_type = checker
            .union_then_lookup(vec![TypeId::ANY, TypeId::UNKNOWN])
            .to_owned();
        assert_eq!(checker.normalize_then_lookup(&union_type), &Type::Any);
    }

    #[test]
    fn normalize_union_of_unknown_any_returns_any() {
        let mut checker = TypeChecker::new();
        let union_type = checker
            .union_then_lookup(vec![TypeId::UNKNOWN, TypeId::ANY])
            .to_owned();
        assert_eq!(checker.normalize_then_lookup(&union_type), &Type::Any);
    }

    #[test]
    fn normalize_union_of_number_number_literal_returns_number() {
        let mut checker = TypeChecker::new();

        let lit_id = checker.number(1.0);
        let union_type = checker
            .union_then_lookup(vec![TypeId::NUMBER, lit_id])
            .to_owned();

        assert_eq!(checker.normalize(&union_type), Some(TypeId::NUMBER));
    }

    #[test]
    fn normalize_union_of_number_literal_number_returns_number() {
        let mut checker = TypeChecker::new();

        let lit_id = checker.number(1.0);
        let union_type = checker
            .union_then_lookup(vec![lit_id, TypeId::NUMBER])
            .to_owned();

        assert_eq!(checker.normalize(&union_type), Some(TypeId::NUMBER));
    }

    #[test]
    fn normalize_union_of_number_union_of_number_number_returns_number() {
        let mut checker = TypeChecker::new();

        let union_type1 = checker
            .union_then_lookup(vec![TypeId::NUMBER, TypeId::NUMBER])
            .to_owned();
        let union_type1_id = checker.lookup_type_id(&union_type1).unwrap();
        let union_type2 = checker
            .union_then_lookup(vec![TypeId::NUMBER, union_type1_id])
            .to_owned();

        assert_eq!(checker.normalize(&union_type2), Some(TypeId::NUMBER));
    }

    #[test]
    fn normalize_union_of_number_union_of_nil_string_returns_union_of_number_nil_string() {
        let mut checker = TypeChecker::new();

        let union_type1 = checker
            .union_then_lookup(vec![TypeId::STRING, TypeId::NIL])
            .to_owned();
        let union_type1_id = checker.lookup_type_id(&union_type1).unwrap();
        let union_type2 = checker
            .union_then_lookup(vec![TypeId::NUMBER, union_type1_id])
            .to_owned();

        let union_type3 = checker
            .union_then_lookup(vec![TypeId::NUMBER, TypeId::STRING, TypeId::NIL])
            .to_owned();
        assert_eq!(checker.normalize_then_lookup(&union_type2), &union_type3);
    }

    #[test]
    fn normalize_empty_union_returns_never() {
        let mut checker = TypeChecker::new();

        let union_type1 = checker.union_then_lookup(vec![]).to_owned();
        assert_eq!(checker.normalize_then_lookup(&union_type1), &Type::Never);
    }

    #[test]
    fn normalize_union_with_1_type_returns_it() {
        let mut checker = TypeChecker::new();

        let union_type1 = checker.union_then_lookup(vec![TypeId::NIL]).to_owned();
        assert_eq!(checker.normalize_then_lookup(&union_type1), &Type::NIL);
    }

    #[test]
    fn normalize_function_with_arg_union_of_number_any() {
        let mut checker = TypeChecker::new();
        let number_any_id = checker.union(vec![TypeId::NUMBER, TypeId::ANY]);
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
        let number_any_id = checker.union(vec![TypeId::NUMBER, TypeId::ANY]);
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
        let number_any_id = checker.union(vec![TypeId::NUMBER, TypeId::ANY]);
        let lit_foo_id = checker.string(r#""foo""#.to_owned());
        let iface = checker
            .iface_then_lookup(vec![(lit_foo_id, number_any_id)])
            .to_owned();

        let normalized_iface = checker
            .iface_then_lookup(vec![(lit_foo_id, TypeId::ANY)])
            .to_owned();

        assert_eq!(checker.normalize_then_lookup(&iface), &normalized_iface);
    }
}
