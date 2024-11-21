use std::{collections::HashMap, ops::Range};

use full_moon::tokenizer::Position;

use super::luatype;

/// ScopeManager define a Lua scope manager.
#[derive(Debug, Default)]
pub struct ScopeManager {
    scopes: Vec<Scope>,
}

impl ScopeManager {
    /// Pushes a new empty scope onto the scope stack.
    pub fn push_scope(&mut self) {
        self.scopes.push(Scope::default())
    }

    /// Pops scope on top of the stack. If there is no more scope, a new one is
    /// added.
    pub fn pop_scope(&mut self) -> Scope {
        if self.scopes.len() == 1 {
            panic!("can't pop root scope");
        }

        self.scopes.pop().unwrap()
    }

    /// Retrieves a variable by looking in current scope and ancestors otherwise.
    pub fn get_variable(&self, ident: &str) -> Option<&Variable> {
        for s in self.scopes.iter().rev() {
            if let Some(var) = s.get(ident) {
                return Some(var);
            }
        }

        None
    }

    /// Sets a variable in current scope.
    pub fn set_local(&mut self, var: Variable) -> Option<Variable> {
        self.scopes.last_mut().unwrap().set(var)
    }

    /// Sets a variable in root scope.
    pub fn set_global(&mut self, var: Variable) -> Option<Variable> {
        self.scopes.first_mut().unwrap().set(var)
    }

    /// Depth returns current scope depth.
    pub fn depth(&self) -> usize {
        self.scopes.len()
    }
}

/// Scope define a Lua scope.
#[derive(Debug, Default)]
pub struct Scope {
    variables: HashMap<String, Variable>,
}

impl Scope {
    pub fn get(&self, ident: &str) -> Option<&Variable> {
        self.variables.get(ident)
    }

    pub fn set(&mut self, var: Variable) -> Option<Variable> {
        self.variables.insert(var.identifier.to_string(), var)
    }
}

/// Variable define a Lua variable.
#[derive(Debug)]
pub struct Variable {
    pub definition: Range<Position>,
    pub identifier: String,
    pub type_id: luatype::TypeId,
}
