use core::fmt;
use std::{
    collections::{BTreeMap, HashMap},
    str::FromStr,
};

use similar::DiffableStr;

use super::{FunctionType, IfaceType, PrimitiveKind, Type, TypeId, UnionType};

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

        // require()
        env.register(Type::Require(FunctionType {
            params: vec![TypeId::STRING],
            results: vec![],
        }));
        // _G
        env.register(Type::Global(IfaceType {
            fields: BTreeMap::new(),
        }));

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

    /// Returns [Type] associated to given [TypeId] within the environment.
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

    /// Returns [TypeId] associated to given [Type] within the environment.
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
        let id = self.next_id();
        self.reverse_lookup.insert(t, id);
        id
    }

    /// Next id returns next [TypeId]. This allows creating self-referential
    /// composite types.
    pub fn next_id(&self) -> TypeId {
        TypeId(self.offset + self.types.len() - 1)
    }

    /// Replace TypeId(n) in a string with actual types. This is needed as
    /// we can't access environment in fmt::Display implementation of Type.
    pub fn fmt(&self, d: impl fmt::Display, alternate: bool) -> String {
        let str = if alternate {
            format!("{d:#}")
        } else {
            format!("{d}")
        };
        let mut str = str.as_str();

        let mut result = "".to_owned();
        while let Some(i) = str.find("TypeId(") {
            result += str.slice(0..i);
            str = &str[i + "TypeId(".len()..str.len()];
            if let Some(mut j) = str.find(")") {
                let type_id = TypeId::from_str(&str[0..j]).unwrap();
                if let Some(t) = self.lookup(type_id) {
                    if let Some(b'#') = str.as_bytes().get(j + 1) {
                        result += &self.fmt(format!("{t:#}"), alternate);
                        j += 1; // Skip #
                    } else {
                        result += &self.fmt(t.to_string(), alternate);
                    }
                }
                str = str.slice(j + 1..str.len());
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
            Type::Function(function) | Type::Require(function) => {
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
            Type::Iface(iface) | Type::Global(iface) => result.extend(
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

    /// Creates a new boolean literal type in the environment.
    pub fn boolean(&mut self, value: bool) -> TypeId {
        self.register(Type::Literal {
            value: value.to_string(),
            kind: PrimitiveKind::Boolean,
        })
    }

    /// Creates a new string literal type in the environment.
    pub fn string(&mut self, lit: String) -> TypeId {
        if lit.len() < 2 || lit.as_bytes()[0] != b'"' || lit.as_bytes()[lit.len() - 1] != b'"' {
            panic!("literal string is unquoted");
        }

        self.register(Type::Literal {
            value: lit,
            kind: PrimitiveKind::String,
        })
    }

    /// Creates a new number literal type in the environment.
    pub fn number(&mut self, value: f64) -> TypeId {
        self.register(Type::Literal {
            value: value.to_string(),
            kind: PrimitiveKind::Number,
        })
    }

    /// Creates a new [FunctionType] type in the environment. This function panics
    /// if one of the params or results [TypeId] isn't part of the environment.
    pub fn function(&mut self, params: Vec<TypeId>, results: Vec<TypeId>) -> TypeId {
        let id = self.next_id();

        params.iter().for_each(|param| {
            if *param != id {
                self.lookup(*param).expect("unknown type id");
            }
        });

        self.register(FunctionType { params, results }.into())
    }

    /// Creates a new [UnionType] type in the environment. This function panics
    /// if one of the [TypeId] isn't part of the environment.
    pub fn union(&mut self, types: Vec<TypeId>) -> TypeId {
        let id = self.next_id();

        let types: Vec<_> = types
            .iter()
            .flat_map(|field| {
                if *field != id {
                    let t = self.lookup(*field).expect("unknown type id");
                    if let Type::Union(u) = t {
                        return u.types.clone();
                    }
                }
                vec![*field]
            })
            .collect();

        if types.len() == 1 {
            return types[0];
        }

        self.register(UnionType { types }.into())
    }

    /// Creates a new [IfaceType] type in the environment. This function panics
    /// if one of the field's key or value [TypeId] isn't part of the environment.
    pub fn iface(&mut self, fields: impl IntoIterator<Item = (TypeId, TypeId)>) -> TypeId {
        let id = self.next_id();

        let fields = BTreeMap::from_iter(fields);

        fields.iter().for_each(|(k, v)| {
            if *k != id {
                let k = self.lookup(*k).expect("unknown iface field key type id");
                match k {
                    Type::Literal { .. } => {}
                    Type::Never
                    | Type::Primitive { .. }
                    | Type::Function(_)
                    | Type::Require(_)
                    | Type::Union(_)
                    | Type::Iface(_)
                    | Type::Global(_)
                    | Type::Any
                    | Type::Unknown => panic!("iface field key must be a literal"),
                }
            }

            if *v != id {
                self.lookup(*v).expect("unknown field iface value type id");
            }
        });

        self.register(IfaceType { fields }.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_environment_register_is_idempotent() {
        let mut env = TypeEnvironment::new();
        assert_eq!(env.register(Type::NIL), TypeId::NIL)
    }

    #[test]
    fn literal_bool() {
        let mut env = TypeEnvironment::new();
        let true_id = env.boolean(true);
        let false_id = env.boolean(false);

        assert!(true_id != false_id);
        assert_eq!(
            env.lookup(true_id),
            Some(&Type::Literal {
                value: "true".to_owned(),
                kind: PrimitiveKind::Boolean
            })
        );
        assert_eq!(
            env.lookup(false_id),
            Some(&Type::Literal {
                value: "false".to_owned(),
                kind: PrimitiveKind::Boolean
            })
        )
    }

    #[test]
    #[should_panic(expected = "literal string is unquoted")]
    fn literal_string_panics_if_unquoted() {
        let mut env = TypeEnvironment::new();
        env.string("foo".to_owned());
    }

    #[test]
    fn literal_string() {
        let mut env = TypeEnvironment::new();
        let foo_id = env.string(r#""FOO BAR BAZ""#.to_owned());

        assert_eq!(
            env.lookup(foo_id),
            Some(&Type::Literal {
                value: r#""FOO BAR BAZ""#.to_owned(),
                kind: PrimitiveKind::String
            })
        )
    }

    #[test]
    fn literal_number() {
        let mut env = TypeEnvironment::new();
        let num_id = env.number(0.05);

        assert_eq!(
            env.lookup(num_id),
            Some(&Type::Literal {
                value: r#"0.05"#.to_owned(),
                kind: PrimitiveKind::Number
            })
        )
    }

    #[test]
    #[should_panic(expected = "unknown type id")]
    fn function_with_unknown_type_id_panics() {
        let mut env = TypeEnvironment::new();
        let mut env2 = TypeEnvironment::new();

        let true_id = env2.boolean(true);

        env.function(vec![true_id], vec![]);
    }

    #[test]
    fn function() {
        let mut env = TypeEnvironment::new();

        let hundred_id = env.number(100.0);
        let hundred_one_id = env.number(101.0);

        let fun_id = env.function(vec![hundred_id], vec![hundred_one_id]);

        assert_eq!(
            env.lookup(fun_id),
            Some(&Type::Function(FunctionType {
                params: vec![hundred_id],
                results: vec![hundred_one_id]
            }))
        )
    }

    #[test]
    #[should_panic(expected = "unknown type id")]
    fn union_with_unknown_type_id_panics() {
        let mut env = TypeEnvironment::new();
        let mut env2 = TypeEnvironment::new();

        let true_id = env2.boolean(true);

        let _ = env.union(vec![true_id]);
    }

    #[test]
    fn union_with_single_type_returns_it() {
        let mut env = TypeEnvironment::new();

        let true_id = env.boolean(true);
        let union_id = env.union(vec![true_id]);

        assert_eq!(true_id, union_id);
    }

    #[test]
    fn union() {
        let mut env = TypeEnvironment::new();

        let true_id = env.boolean(true);
        let false_id = env.boolean(false);
        let union_id = env.union(vec![true_id, false_id]);

        assert_eq!(
            env.lookup(union_id),
            Some(
                &UnionType {
                    types: vec![true_id, false_id]
                }
                .into()
            )
        )
    }

    #[test]
    #[should_panic(expected = "unknown iface field key type id")]
    fn iface_with_unknown_field_key_type_panics() {
        let mut env = TypeEnvironment::new();
        let mut env2 = TypeEnvironment::new();

        let true_id = env2.boolean(true);

        let _ = env.iface(vec![(true_id, TypeId::BOOLEAN)]);
    }

    #[test]
    #[should_panic(expected = "unknown field iface value type id")]
    fn iface_with_unknown_field_value_type_panics() {
        let mut env = TypeEnvironment::new();
        let mut env2 = TypeEnvironment::new();

        let true_id = env.boolean(true);

        // We must create two IDs or foo_id will collide with true_id.
        env2.string(r#""bar""#.to_owned());
        let foo_id = env2.string(r#""foo""#.to_owned());

        let _ = env.iface(vec![(true_id, foo_id)]);
    }

    #[test]
    #[should_panic(expected = "iface field key must be a literal")]
    fn iface_with_non_literal_field_key_panics() {
        let mut env = TypeEnvironment::new();

        let _ = env.iface(vec![(TypeId::NUMBER, TypeId::STRING)]);
    }

    #[test]
    fn iface() {
        let mut env = TypeEnvironment::new();

        let foo_id = env.string(r#""foo""#.to_owned());

        let iface_id = env.iface(vec![(foo_id, TypeId::STRING)]);

        assert_eq!(
            env.lookup(iface_id),
            Some(
                &IfaceType {
                    fields: BTreeMap::from_iter(vec![(foo_id, TypeId::STRING)])
                }
                .into()
            )
        )
    }
}
