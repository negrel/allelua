use std::collections::HashMap;
use std::str::FromStr;

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
        env.register_new(Type::Never, TypeId::NEVER);
        env.register_new(Type::Any, TypeId::ANY);
        env.register_new(Type::Unknown, TypeId::UNKNOWN);
        env.register_new(Type::NIL, TypeId::NIL);
        env.register_new(Type::BOOLEAN, TypeId::BOOLEAN);
        env.register_new(Type::NUMBER, TypeId::NUMBER);
        env.register_new(Type::STRING, TypeId::STRING);

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
        let i = Into::<usize>::into(id);
        if i < self.offset {
            if let Some(parent) = &self.parent {
                parent.lookup(id)
            } else {
                None
            }
        } else if (self.offset..self.types.len()).contains(&i) {
            Some(&self.types[i - self.offset])
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

        let i = (self.offset + self.types.len()).try_into().unwrap();
        let id = TypeId::new(&t, i);
        self.register_new(t, id);
        id
    }

    fn register_new(&mut self, t: Type, id: TypeId) {
        self.types.push(t.clone());
        self.reverse_lookup.insert(t, id);
    }

    /// Replace TypeId(n) in a string with actual types. This is needed as
    /// we can't access environment in [std::fmt::Display] implementation of Type.
    pub fn replace_type_ids(&self, s: impl AsRef<str>) -> String {
        let mut str = s.as_ref();

        let mut result = "".to_owned();
        while let Some(i) = str.find("TypeId(") {
            result += str.slice(0..i);
            str = &str[i + "TypeId(".len()..str.len()];
            if let Some(mut i) = str.find(")") {
                let type_id = TypeId::from_str(&str[0..i]).unwrap();
                if let Some(t) = self.lookup(type_id) {
                    if i + 1 < str.len() && str.as_bytes()[i + 1] == b'#' {
                        result += &self.replace_type_ids(format!("{t:#}"));
                        i += 1;
                    } else {
                        result += &self.replace_type_ids(t.to_string());
                    }
                }
                str = str.slice(i + 1..str.len());
            }
        }

        result + str
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn type_environment_register_is_idempotent() {
        let mut env = TypeEnvironment::new();
        assert_eq!(env.register(Type::NIL), TypeId::NIL)
    }
}
