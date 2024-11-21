use std::fmt;

use super::{
    FunctionType, GenericType, IfaceType, Type, TypeEnvironment, TypeId, TypeParameter,
    TypeParameterConstraint, UnionType,
};

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
                if self.can_assign_functions(&source, &target, &mut reasons) {
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
                if self.can_assign_iface(&source, &target, &mut reasons) {
                    return Ok(());
                }
            }
            (Type::Parameter(source), Type::Parameter(target)) => {
                match (source.constraint, target.constraint) {
                    (
                        TypeParameterConstraint::Extends(source),
                        TypeParameterConstraint::Extends(target),
                    ) => match self.can_assign(source, target) {
                        Ok(_) => return Ok(()),
                        Err(err) => reasons.push(err),
                    },
                }
            }
            // TypeParameter is assignable if it's constraint satisfy target type.
            (Type::Parameter(param), _) => match param.constraint {
                TypeParameterConstraint::Extends(id) => match self.can_assign(id, target_id) {
                    Ok(_) => return Ok(()),
                    Err(err) => reasons.push(err),
                },
            },
            (Type::Generic(source), Type::Generic(target)) => {
                if self.can_assign_generic(source, target, &mut reasons) {
                    return Ok(());
                }
            }
            (_, Type::Generic(generic)) => match self.can_assign(source_id, generic.on) {
                Ok(_) => return Ok(()),
                Err(err) => reasons.push(err),
            },
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
    ) -> bool {
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

        initial_reasons_len == reasons.len()
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
    ) -> bool {
        let initial_reasons_len = reasons.len();

        // Source is assignable if all fields of target are assignable from
        // source to target. If field is missing in source, nil must be
        // assignable in target.
        for (f_name, f_type_id) in target.fields.iter() {
            match self.get_field_type(source, *f_name) {
                Some(source_f_type_id) => {
                    if let Err(reason) = self.can_assign(source_f_type_id, *f_type_id) {
                        reasons.push(TypeCheckError::IncompatibleFieldType {
                            field_name: *f_name,
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

        initial_reasons_len == reasons.len()
    }

    pub fn can_assign_generic<'a>(
        &self,
        source: &'a GenericType,
        target: &'a GenericType,
    ) -> Result<(), TypeCheckError> {
        // if !source.on.is_function() || target.on.is_function() {
        //     return Err(TypeCheckError::TypeParameterInstantiationMismatch {
        //         source: (),
        //         target: (),
        //     });
        // }

        Ok(())
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
            Type::Parameter(p) => Some(self.normalize_type_parameter(p)),
            Type::Generic(g) => Some(self.normalize_generic(g)),
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

    fn normalize_union_types(&mut self, ids: &Vec<TypeId>, mut result: Vec<TypeId>) -> Vec<TypeId> {
        // Any as priority over unknown so we check it first.
        if ids.contains(&TypeId::ANY) {
            return vec![TypeId::ANY];
        }

        'type_ids_loop: for type_id in ids {
            // Filter duplicate.
            if result.contains(type_id) {
                continue;
            }

            let mut type_id = *type_id;

            let t = self.env.lookup(type_id).unwrap().to_owned();
            match t {
                Type::Never => continue,
                Type::Literal { kind, .. } => {
                    // Skip literal if primitive already present.
                    for id in &result {
                        if *id == kind.into() {
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
                            if *kind == primitive_kind {
                                return false;
                            }
                        }
                        true
                    });
                }
                Type::Function(function) => type_id = self.normalize_function(&function),
                Type::Union(union) => {
                    result = self.normalize_union_types(&union.types, result);
                    continue;
                }
                Type::Iface(iface) => type_id = self.normalize_iface(&iface),
                Type::Any => unreachable!(),
                Type::Unknown => return vec![TypeId::UNKNOWN],
                Type::Parameter(p) => type_id = self.normalize_type_parameter(&p),
                Type::Generic(g) => type_id = self.normalize_generic(&g),
            }

            // Filter duplicate.
            if result.contains(&type_id) {
                continue;
            }

            result.push(type_id);
        }

        result
    }

    fn normalize_type_parameter(&mut self, tparam: &TypeParameter) -> TypeId {
        let mut normalized = tparam.to_owned();
        match normalized.constraint {
            TypeParameterConstraint::Extends(id) => {
                let t = self.lookup_type(id).unwrap();
                if let Some(id) = self.normalize(&t.to_owned()) {
                    normalized.constraint = TypeParameterConstraint::Extends(id);
                }
            }
        }
        self.register_type(normalized.into())
    }

    fn normalize_generic(&mut self, generic: &GenericType) -> TypeId {
        let mut normalized = generic.to_owned();
        normalized.on = self
            .normalize(&self.lookup_type(normalized.on).unwrap().to_owned())
            .unwrap();
        normalized.params = normalized
            .params
            .iter()
            .map(|p| {
                let t = self.lookup_type(*p).unwrap();
                self.normalize(&t.to_owned()).unwrap_or(*p)
            })
            .collect();

        self.register_type(normalized.into())
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
            Type::Parameter(p) => match p.constraint {
                TypeParameterConstraint::Extends(id) => {
                    let t = self.lookup_type(id).unwrap().to_owned();
                    self.get_field_type(&t, field)
                }
            },
            Type::Generic(g) => {
                let t = self.lookup_type(g.on).unwrap().to_owned();
                self.get_field_type(&t, field)
            }
        }
    }

    fn is_generic(&self, id: TypeId) -> bool {
        if id.is_generic() || id.is_type_parameter() {
            return true;
        }
        if !id.is_composite() {
            return false;
        }

        let t = self.lookup_type(id).unwrap();

        match t {
            Type::Function(function) => {
                function.params.iter().any(|p| self.is_generic(*p))
                    || function.results.iter().any(|r| self.is_generic(*r))
            }
            Type::Union(u) => u.types.iter().any(|i| self.is_generic(*i)),
            Type::Iface(f) => f
                .fields
                .iter()
                .any(|(k, v)| self.is_generic(*k) || self.is_generic(*v)),
            _ => unreachable!(),
        }
    }

    fn substitute_type_params(
        &mut self,
        generic: &GenericType,
        type_args: Vec<TypeId>,
    ) -> Result<TypeId, TypeCheckError> {
        if type_args.len() != generic.params.len() {
            panic!("number of type arguments and type parameters doesn't match");
        }

        let mut concrete = self.lookup_type(generic.on).unwrap().to_owned();

        match concrete {
            Type::Never
            | Type::Literal { .. }
            | Type::Primitive { .. }
            | Type::Any
            | Type::Unknown
            | Type::Generic(_)
            | Type::Parameter(_) => unreachable!(),
            Type::Function(ref mut function) => {
                for param in &mut function.params {
                    if param.is_type_parameter() {
                        match generic.params.iter().position(|p| p == param) {
                            Some(i) => *param = type_args[i],
                            None => panic!("{function} contains unknown type parameter"),
                        }
                    }
                }
            }
            Type::Union(ref mut u) => {
                for t in &mut u.types {
                    if t.is_type_parameter() {
                        match generic.params.iter().position(|p| p == t) {
                            Some(i) => *t = type_args[i],
                            None => panic!("{u} contains unknown type parameter"),
                        }
                    }
                }
            }
            Type::Iface(ref mut iface) => {
                iface.fields = iface
                    .fields
                    .iter()
                    .map(|(mut k, mut v)| {
                        if k.is_type_parameter() {
                            match generic.params.iter().position(|p| p == k) {
                                Some(i) => k = &type_args[i],
                                None => {
                                    panic!("{iface} field key {k} is an unknown type parameter")
                                }
                            }
                        }
                        if v.is_type_parameter() {
                            match generic.params.iter().position(|p| p == v) {
                                Some(i) => v = &type_args[i],
                                None => {
                                    panic!(
                                        "{iface} field {k} value {v} is an unknown type parameter"
                                    )
                                }
                            }
                        }

                        (*k, *v)
                    })
                    .collect();
            }
        }

        Ok(self.register_type(concrete))
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
        field_name: TypeId,
        source: Box<TypeCheckError>,
    },
    RequiredFieldMissing {
        field_name: TypeId,
        field_type: TypeId,
    },
    TypeParameterInstantiationMismatch {
        source: TypeId,
        target: TypeId,
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
            TypeCheckError::RequiredFieldMissing { field_name, field_type } => write!(f, r#"Mandatory field {field_name:?} of type "{field_type:?}" is missing"#),
            TypeCheckError::TypeParameterInstantiationMismatch { source, target } => write!(f, r#""{source}" is assignable to the constraint of type '{target:#}', but '{target}' could be instantiated with a different subtype of constraint."#),
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

    macro_rules! type_id_of {
        ($checker:ident, $t:expr) => {
            $checker.register_type($t)
        };
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
    fn generic_iface_foo_t_extends_number_is_assignable_to_iface_foo_number() {
        let mut checker = TypeChecker::new();

        let lit_foo_string = Type::string(r#""foo""#.to_owned());
        let lit_foo_string_id = type_id_of!(checker, lit_foo_string);

        let t = TypeParameter {
            name: "T".to_owned(),
            constraint: TypeParameterConstraint::Extends(TypeId::ANY),
        }
        .into();
        let t_id = type_id_of!(checker, t);

        let generic_iface_foo = IfaceType::new(vec![(lit_foo_string_id, TypeId::NUMBER)]).into();
        let generic_iface_foo_id = type_id_of!(checker, generic_iface_foo);

        let generic = GenericType {
            on: generic_iface_foo_id,
            params: vec![t_id],
        }
        .into();
        let generic_id = type_id_of!(checker, generic);

        let iface_foo = IfaceType::new(vec![(lit_foo_string_id, TypeId::NUMBER)]).into();
        let iface_foo_id = type_id_of!(checker, iface_foo);

        assert!(checker.can_assign(generic_id, iface_foo_id).is_ok());
    }

    #[test]
    fn generic_function_with_param_t_extends_any_and_return_t_is_assignable_to_function_with_param_any_and_return_any(
    ) {
        let mut checker = TypeChecker::new();
        let t = TypeParameter {
            name: "T".to_owned(),
            constraint: TypeParameterConstraint::Extends(TypeId::ANY),
        }
        .into();
        let t_id = type_id_of!(checker, t);

        let generic_function = FunctionType {
            params: vec![t_id],
            results: vec![t_id],
        }
        .into();
        let generic_function_id = type_id_of!(checker, generic_function);

        let function = FunctionType {
            params: vec![TypeId::ANY],
            results: vec![TypeId::ANY],
        }
        .into();
        let function_id = type_id_of!(checker, function);

        assert!(checker.can_assign(generic_function_id, function_id).is_ok())
    }

    #[test]
    fn function_with_param_any_and_return_any_is_assignable_to_generic_function_with_param_t_extends_any_and_return_t(
    ) {
        let mut checker = TypeChecker::new();
        let t = TypeParameter {
            name: "T".to_owned(),
            constraint: TypeParameterConstraint::Extends(TypeId::ANY),
        }
        .into();
        let t_id = type_id_of!(checker, t);

        let generic_function = FunctionType {
            params: vec![t_id],
            results: vec![t_id],
        }
        .into();
        let generic_function_id = type_id_of!(checker, generic_function);

        let function = FunctionType {
            params: vec![TypeId::ANY],
            results: vec![TypeId::ANY],
        }
        .into();
        let function_id = type_id_of!(checker, function);

        assert!(checker.can_assign(function_id, generic_function_id).is_ok())
    }

    #[test]
    fn generic_function_with_param_t_extends_any_and_return_t_is_assignable_to_function_with_param_number_and_return_number(
    ) {
        let mut checker = TypeChecker::new();
        let t = TypeParameter {
            name: "T".to_owned(),
            constraint: TypeParameterConstraint::Extends(TypeId::ANY),
        }
        .into();
        let t_id = type_id_of!(checker, t);

        let generic_function = FunctionType {
            params: vec![t_id],
            results: vec![t_id],
        }
        .into();
        let generic_function_id = type_id_of!(checker, generic_function);

        let function = FunctionType {
            params: vec![TypeId::NUMBER],
            results: vec![TypeId::NUMBER],
        }
        .into();
        let function_id = type_id_of!(checker, function);

        println!("{}", {
            let err = checker
                .can_assign(generic_function_id, function_id)
                .unwrap_err();
            checker.environment().replace_type_ids(err.to_string())
        });

        assert!(checker.can_assign(generic_function_id, function_id).is_ok())
    }

    #[test]
    fn function_with_param_number_and_return_number_is_assignable_to_generic_function_with_param_t_extends_any_and_return_t(
    ) {
        let mut checker = TypeChecker::new();
        let t = TypeParameter {
            name: "T".to_owned(),
            constraint: TypeParameterConstraint::Extends(TypeId::ANY),
        }
        .into();
        let t_id = type_id_of!(checker, t);

        let generic_function = FunctionType {
            params: vec![t_id],
            results: vec![t_id],
        }
        .into();
        let generic_function_id = type_id_of!(checker, generic_function);

        let function = FunctionType {
            params: vec![TypeId::NUMBER],
            results: vec![TypeId::NUMBER],
        }
        .into();
        let function_id = type_id_of!(checker, function);

        assert!(checker.can_assign(function_id, generic_function_id).is_ok())
    }

    #[test]
    fn generic_function_with_param_t_extends_string_and_return_t_is_assignable_to_function_with_param_string_and_return_string(
    ) {
        let mut checker = TypeChecker::new();
        let t = TypeParameter {
            name: "T".to_owned(),
            constraint: TypeParameterConstraint::Extends(TypeId::STRING),
        };
        let t_id = type_id_of!(checker, t.clone().into());

        let generic_function = FunctionType {
            params: vec![t_id],
            results: vec![t_id],
        }
        .into();
        let generic_function_id = type_id_of!(checker, generic_function);
        let generic = GenericType {
            on: generic_function_id,
            params: vec![t_id],
        }
        .into();
        let generic_id = type_id_of!(checker, generic);

        let function = FunctionType {
            params: vec![TypeId::STRING],
            results: vec![TypeId::STRING],
        }
        .into();
        let function_id = type_id_of!(checker, function);

        assert!(checker.can_assign(generic_id, function_id).is_ok())
    }

    #[test]
    fn function_with_param_string_and_return_string_is_assignable_to_generic_function_with_param_t_extends_string_and_return_t(
    ) {
        let mut checker = TypeChecker::new();
        let t = TypeParameter {
            name: "T".to_owned(),
            constraint: TypeParameterConstraint::Extends(TypeId::STRING),
        }
        .into();
        let t_id = type_id_of!(checker, t);

        let generic_function = FunctionType {
            params: vec![t_id],
            results: vec![t_id],
        }
        .into();
        let generic_function_id = type_id_of!(checker, generic_function);

        let generic = GenericType {
            on: generic_function_id,
            params: vec![t_id],
        }
        .into();
        let generic_id = type_id_of!(checker, generic);

        let function = FunctionType {
            params: vec![TypeId::STRING],
            results: vec![TypeId::STRING],
        }
        .into();
        let function_id = type_id_of!(checker, function);

        assert!(checker.can_assign(function_id, generic_id).is_ok())
    }

    #[test]
    fn generic_function_with_param_t_extends_string_and_return_t_is_not_assignable_to_function_with_param_number_and_return_number(
    ) {
        let mut checker = TypeChecker::new();
        let t = TypeParameter {
            name: "T".to_owned(),
            constraint: TypeParameterConstraint::Extends(TypeId::STRING),
        }
        .into();
        let t_id = type_id_of!(checker, t);

        let generic_function = FunctionType {
            params: vec![t_id],
            results: vec![t_id],
        }
        .into();
        let generic_function_id = type_id_of!(checker, generic_function);
        let generic = GenericType {
            on: generic_function_id,
            params: vec![t_id],
        }
        .into();
        let generic_id = type_id_of!(checker, generic);

        let function = FunctionType {
            params: vec![TypeId::NUMBER],
            results: vec![TypeId::NUMBER],
        }
        .into();
        let function_id = type_id_of!(checker, function);

        assert_eq!(
            checker.can_assign(generic_id, function_id),
            Err(TypeCheckError::IncompatibleType(IncompatibleTypeError {
                source_type: generic_id,
                target_type: function_id,
                reasons: vec![TypeCheckError::IncompatibleType(IncompatibleTypeError {
                    source_type: generic_function_id,
                    target_type: function_id,
                    reasons: vec![
                        TypeCheckError::IncompatibleParameterType {
                            nth: 0,
                            source: Box::new(TypeCheckError::IncompatibleType(
                                IncompatibleTypeError {
                                    source_type: TypeId::NUMBER,
                                    target_type: t_id,
                                    reasons: vec![TypeCheckError::IncompatibleType(
                                        IncompatibleTypeError {
                                            source_type: TypeId::NUMBER,
                                            target_type: TypeId::STRING,
                                            reasons: vec![]
                                        }
                                    )]
                                }
                            ))
                        },
                        TypeCheckError::IncompatibleReturnType {
                            nth: 0,
                            source: Box::new(TypeCheckError::IncompatibleType(
                                IncompatibleTypeError {
                                    source_type: t_id,
                                    target_type: TypeId::NUMBER,
                                    reasons: vec![TypeCheckError::IncompatibleType(
                                        IncompatibleTypeError {
                                            source_type: TypeId::STRING,
                                            target_type: TypeId::NUMBER,
                                            reasons: vec![]
                                        }
                                    )]
                                }
                            ))
                        }
                    ]
                })],
            }))
        )
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
