use mlua::{FromLua, IntoLua, LuaOptions, StdLib};

use crate::{include_lua, IncludeChunk};

pub const MOD: IncludeChunk = include_lua!("./init.lua");

/// VM define a type expression virtual machine handle.
#[derive(Debug, Clone)]
pub struct VM {
    #[allow(dead_code)]
    lua: mlua::Lua,
    g: mlua::Table,
}

impl VM {
    pub fn new() -> mlua::Result<Self> {
        let vm = mlua::Lua::new_with(StdLib::NONE, LuaOptions::default())?;
        let types = vm.load(MOD).eval::<mlua::Table>()?;
        let g = types.clone();
        Ok(Self { lua: vm, g })
    }

    /// Retrieves name of [Type].
    pub fn type_string(&self, t: Type) -> String {
        self.g
            .get::<mlua::Function>("to_type_string")
            .unwrap()
            .call::<String>(t)
            .unwrap()
    }

    /// Assigns `rhs` [Type] to `lhs` [Type]. This function returns an error
    /// if assignment is not possible.
    pub fn assign(&self, lhs: Type, rhs: Type) -> Result<(), String> {
        let reason = self
            .g
            .get::<mlua::Function>("_assign")
            .unwrap()
            .call::<Option<String>>((lhs, rhs))
            .unwrap();

        match reason {
            Some(err) => {
                // Remove lua_file:line in error message.
                match err.find(": ") {
                    Some(i) => Err(err[i + ": ".len()..].to_string()),
                    None => Err(err),
                }
            }
            None => Ok(()),
        }
    }

    /// Returns current lexical scope.
    pub fn scope(&self) -> Scope {
        self.g
            .get::<mlua::Table>("scopes")
            .unwrap()
            .get::<Scope>("current")
            .unwrap()
    }

    /// Evaluates provided expression within current scope and returns it's [Type]
    /// and [mlua::Value] if possible.
    pub fn eval_in_scope(&self, expr: &str) -> mlua::Result<(Type, Option<mlua::Value>)> {
        let (t, literal): (Type, mlua::Value) = self
            .g
            .get::<mlua::Function>("eval_in_scope")
            .unwrap()
            .call(expr)?;

        if t != Type(mlua::Value::Nil) && literal == mlua::Value::Nil {
            Ok((t, None))
        } else {
            Ok((t, Some(literal)))
        }
    }

    /// Evaluates expression within current scope and returns it's [Type]
    /// and [mlua::Value].
    pub fn eval_expr_in_scope(&self, expr: &str) -> mlua::Result<(Type, Option<mlua::Value>)> {
        self.eval_in_scope(&("return ".to_owned() + expr))
    }

    /// Evaluates type expression (e.g. `fn(number, number) (number)`) and
    /// returns it's [Type].
    pub fn eval_type(&self, expr: &str) -> mlua::Result<Type> {
        self.g
            .get::<mlua::Function>("eval_type")
            .unwrap()
            .call("return ".to_owned() + expr)
    }

    /// Pushes a new scope on stack of scopes and returns it.
    pub fn push_scope(&self) -> Scope {
        self.g
            .get::<mlua::Table>("scopes")
            .unwrap()
            .get::<mlua::Function>("push")
            .unwrap()
            .call::<Scope>(())
            .unwrap()
    }

    /// Pops scope from stack of scopes and returns new lexical scope.
    pub fn pop_scope(&self) -> Scope {
        self.g
            .get::<mlua::Table>("scopes")
            .unwrap()
            .get::<mlua::Function>("pop")
            .unwrap()
            .call::<Scope>(())
            .unwrap()
    }

    /// Returns any type singleton.
    pub fn any(&self) -> Type {
        self.g.get::<Type>("any").unwrap()
    }

    /// Returns never type singleton.
    pub fn never(&self) -> Type {
        self.g.get::<Type>("never").unwrap()
    }

    /// Returns string type singleton.
    pub fn string(&self) -> Type {
        self.g.get::<Type>("string").unwrap()
    }

    /// Returns number type singleton.
    pub fn number(&self) -> Type {
        self.g.get::<Type>("number").unwrap()
    }

    /// Returns boolean type singleton.
    pub fn boolean(&self) -> Type {
        self.g.get::<Type>("boolean").unwrap()
    }

    /// Returns nil type singleton.
    pub fn nil(&self) -> Type {
        Type(mlua::Nil)
    }

    /// Returns a function [Type] with provided parameters and returns [Type].
    pub fn function(&self, params: &[Type], results: &[Type]) -> Type {
        self.g
            .get::<mlua::Function>("_fn")
            .unwrap()
            .call((params, results))
            .unwrap()
    }
}

/// Type define a Lua type in our type system.
#[derive(Debug, Clone, PartialEq)]
pub struct Type(mlua::Value);

impl From<mlua::Value> for Type {
    fn from(value: mlua::Value) -> Self {
        Self(value)
    }
}

impl FromLua for Type {
    fn from_lua(value: mlua::Value, _lua: &mlua::Lua) -> mlua::Result<Self> {
        Ok(Self(value))
    }
}

impl IntoLua for Type {
    fn into_lua(self, _lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        Ok(self.0)
    }
}

/// Scope define a Lua lexical scope.
#[derive(Debug, PartialEq)]
pub struct Scope {
    is_nil: mlua::Table,
    values: mlua::Table,
    types: mlua::Table,
}

impl FromLua for Scope {
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
        let tab = mlua::Table::from_lua(value, lua)?;
        let is_nil = tab.get::<mlua::Table>("is_nil")?;
        let values = tab.get::<mlua::Table>("values")?;
        let types = tab.get::<mlua::Table>("types")?;

        Ok(Self {
            is_nil,
            values,
            types,
        })
    }
}

impl Scope {
    pub fn set(&self, name: &str, t: Type, value: Option<mlua::Value>) {
        if let Some(value) = value {
            self.values.set(name, value).unwrap();
        }
        if t == Type(mlua::Value::Nil) {
            self.is_nil.set(name, true).unwrap();
        }
        self.types.set(name, t).unwrap();
    }

    pub fn get(&self, name: &str) -> Type {
        self.values.get::<Type>(name).unwrap()
    }

    pub fn get_type(&self, name: &str) -> Type {
        self.types.get::<Type>(name).unwrap()
    }

    pub fn get_local(&self, name: &str) -> Type {
        let v = self.values.raw_get::<Type>(name).unwrap();
        if v == Type(mlua::Value::Nil) && self.is_nil.get::<bool>(name).unwrap() {
            return v;
        }

        self.types.raw_get::<Type>(name).unwrap()
    }

    pub fn get_local_type(&self, name: &str) -> Type {
        self.types.raw_get::<Type>(name).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_lua() {
        VM::new().unwrap();
    }

    #[test]
    fn primitive_singletons() {
        let vm = VM::new().unwrap();

        let string = vm.string();
        assert_eq!(string, vm.string());

        let boolean = vm.boolean();
        assert_eq!(boolean, vm.boolean());

        let number = vm.number();
        assert_eq!(number, vm.number());

        assert_ne!(string, boolean);
        assert_ne!(boolean, number);
        assert_ne!(number, string);
    }

    #[test]
    fn scope_get_nil() {
        let vm = VM::new().unwrap();
        assert_eq!(vm.scope().get("foo"), Type(mlua::Value::Nil));
    }

    #[test]
    fn scope_get_local() {
        let vm = VM::new().unwrap();
        let scope = vm.scope();

        scope.set("foo", vm.string(), None);

        assert_eq!(scope.get("foo"), vm.string());
    }

    #[test]
    fn scope_get_shadowed() {
        let vm = VM::new().unwrap();
        // Setup initial scope.
        {
            let scope = vm.scope();
            scope.set("foo", vm.string(), None);
        }

        // Create a new scope.
        let scope = vm.push_scope();
        assert_eq!(scope, vm.scope());
        assert_eq!(scope.get("foo"), vm.string());

        // Shadow the variable.
        scope.set("foo", vm.number(), None);
        assert_eq!(scope.get("foo"), vm.number());

        // Restore initial scope.
        let scope = vm.pop_scope();
        assert_eq!(scope, vm.scope());
        assert_eq!(scope.get("foo"), vm.string());
    }

    #[test]
    fn eval_expr_in_scope() {
        let vm = VM::new().unwrap();
        {
            let scope = vm.scope();
            scope.set("foo", vm.string(), None);
        }

        let scope = vm.push_scope();
        scope.set("foo", vm.number(), None);

        assert_eq!(vm.eval_expr_in_scope("foo").unwrap(), (vm.number(), None));
    }
}
