use mlua::Lua;
use worker::LuaWorker;

use crate::{include_lua, lua_string_as_path};

use super::os::LuaFile;

mod worker;

pub fn load_proc(lua: &Lua) -> mlua::Result<mlua::Table> {
    lua.load_from_function(
        "proc",
        lua.create_function(|lua, ()| {
            let proc = lua.create_table()?;

            let worker_constructors = lua.create_table()?;
            proc.set("Worker", worker_constructors)?;

            lua.load(include_lua!("./proc.lua"))
                .eval::<mlua::Function>()?
                .call::<()>((
                    proc.clone(),
                    lua.create_function(|lua, path: mlua::String| {
                        lua_string_as_path!(path = path);
                        LuaWorker::new(lua, path)
                    })?,
                    lua.named_registry_value::<bool>("worker").unwrap(),
                    LuaFile::stdin(false)?,
                    LuaFile::stderr()?,
                ))?;

            Ok(proc)
        })?,
    )
}
