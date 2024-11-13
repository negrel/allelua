use std::{
    ffi::OsStr,
    os::unix::ffi::OsStrExt,
    path::{self, Path},
};

use mlua::Lua;

use crate::include_lua;

use super::io;

pub fn load_package(lua: Lua, fpath: &Path) -> mlua::Result<()> {
    let fpath = lua.create_string(fpath.as_os_str().as_bytes())?;

    // Sync version of "path.canonicalize".
    let path_canonicalize = lua.create_function(|lua, str: mlua::String| {
        let str = str.as_bytes();
        let path = path::Path::new(OsStr::from_bytes(&str));
        let path = std::fs::canonicalize(path).map_err(io::LuaError::from)?;
        lua.create_string(path.as_os_str().as_bytes())
    })?;

    // Returns source path of the caller.
    let caller_source =
        lua.create_function(|lua, lvl: usize| match lua.inspect_stack(lvl + 1) {
            Some(debug) => match debug.source() {
                mlua::DebugSource {
                    source: Some(src), ..
                } => Ok(mlua::Value::String(
                    lua.create_string(src.trim_start_matches('@').as_bytes())?,
                )),
                _ => Ok(mlua::Value::Nil),
            },
            _ => Ok(mlua::Value::Nil),
        })?;

    lua.load(include_lua!("./package.lua"))
        .eval::<mlua::Function>()?
        .call((fpath, path_canonicalize, caller_source))
}
