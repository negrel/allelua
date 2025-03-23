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
            .get::<mlua::Table>("current")
            .unwrap()
            .into()
    }

    /// Evaluates provided expression within current scope.
    pub fn eval_in_scope(&self, expr: &str) -> mlua::Result<Type> {
        self.g
            .get::<mlua::Function>("eval_in_scope")
            .unwrap()
            .call(expr)
    }

    /// Evaluates variable expression (e.g. `foo.bar.bar`) and returns it's [Type].
    pub fn eval_expr_in_scope(&self, expr: &str) -> mlua::Result<Type> {
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
            .call::<mlua::Table>(())
            .unwrap()
            .into()
    }

    /// Pops scope from stack of scopes and returns new lexical scope.
    pub fn pop_scope(&self) -> Scope {
        self.g
            .get::<mlua::Table>("scopes")
            .unwrap()
            .get::<mlua::Function>("pop")
            .unwrap()
            .call::<mlua::Table>(())
            .unwrap()
            .into()
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
pub struct Scope(mlua::Table);

impl From<mlua::Table> for Scope {
    fn from(value: mlua::Table) -> Self {
        Self(value)
    }
}

impl Scope {
    pub fn set(&self, name: &str, t: Type) {
        self.0.set(name, t).unwrap()
    }

    pub fn get(&self, name: &str) -> Type {
        self.0.get(name).unwrap()
    }

    pub fn get_local(&self, name: &str) -> Type {
        self.0.raw_get(name).unwrap()
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

        scope.set("foo", vm.string());

        assert_eq!(scope.get("foo"), vm.string());
    }

    #[test]
    fn scope_get_shadowed() {
        let vm = VM::new().unwrap();
        // Setup initial scope.
        {
            let scope = vm.scope();
            scope.set("foo", vm.string());
        }

        // Create a new scope.
        let scope = vm.push_scope();
        assert_eq!(scope, vm.scope());
        assert_eq!(scope.get("foo"), vm.string());

        // Shadow the variable.
        scope.set("foo", vm.number());
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
            scope.set("foo", vm.string());
        }

        let scope = vm.push_scope();
        scope.set("foo", vm.number());

        assert_eq!(vm.eval_expr_in_scope("foo").unwrap(), vm.number());
    }
}
