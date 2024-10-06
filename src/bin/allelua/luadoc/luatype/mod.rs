mod env;

use std::path::PathBuf;

use crate::{include_lua, lua};

/// Checker define a type checker. It is responsible of checking if a type is
/// assignable to another type. It is NOT responsible of type inference.
pub struct Checker {
    vm: lua::Runtime,
    module: mlua::Table,
}

impl Checker {
    pub fn new() -> Self {
        let vm = lua::Runtime::new(
            &PathBuf::from("luatype"),
            vec![],
            lua::RuntimeSafetyLevel::Safe,
        );
        let module = vm
            .load(include_lua!("./luatype.lua"))
            .eval::<mlua::Table>()
            .unwrap();

        Self { vm, module }
    }
}
