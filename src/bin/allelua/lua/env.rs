use std::{env, ffi::OsString, os::unix::ffi::OsStrExt};

use mlua::{FromLua, Lua, UserData};

use crate::LuaModule;

#[derive(Clone, FromLua)]
struct LuaEnvVarsTable;

impl UserData for LuaEnvVarsTable {
    fn add_fields<'lua, F: mlua::UserDataFields<'lua, Self>>(_fields: &mut F) {}

    fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(mlua::MetaMethod::ToString, |_, _, ()| Ok("EnvVarsTable"));
        methods.add_meta_method(
            mlua::MetaMethod::NewIndex,
            |_, _, (key, val): (String, String)| {
                env::set_var(key, val);
                Ok(())
            },
        );
        methods.add_meta_method(mlua::MetaMethod::Index, |lua, _, key: String| {
            let str = env::var(key).map_err(mlua::Error::runtime)?;
            lua.create_string(str)
        });
    }
}

LuaModule!(LuaEnvModule, fields {
    var = LuaEnvVarsTable
}, functions {
    args(lua) {
        lua.named_registry_value::<mlua::Table>("env_args")
    },
    vars(lua) {
        let table = lua.create_table()?;
        for (key,val) in env::vars_os() {
            let key = lua.create_string(key.as_bytes())?;
            table.set(key, lua.create_string(val.as_bytes())?)?;
        }
        Ok(table)
    },
    current_dir(lua) {
        let path = env::current_dir().map_err(mlua::Error::runtime)?;
        lua.create_string(path.as_os_str().as_bytes())
    }
}, async functions {});

pub fn load_env(lua: &'static Lua, run_args: Vec<OsString>) -> mlua::Result<LuaEnvModule> {
    let args = lua.create_table()?;
    for arg in run_args {
        args.push(lua.create_string(arg.as_bytes())?)?;
    }
    lua.set_named_registry_value("env_args", args)?;
    lua.load_from_function("env", lua.create_function(|_, ()| Ok(LuaEnvModule))?)
}
