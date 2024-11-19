use std::collections::HashMap;

use similar::DiffableStr;

use super::{Type, TypeId};

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_environment_register_is_idempotent() {
        let mut env = TypeEnvironment::new();
        assert_eq!(env.register(Type::NIL), TypeId::NIL)
    }
}
