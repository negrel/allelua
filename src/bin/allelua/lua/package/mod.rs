use std::{
    ffi::OsStr,
    fs,
    os::unix::ffi::OsStrExt,
    path::{self, Path},
};

use mlua::Lua;

use crate::include_lua;

use super::{error::LuaError, io};

pub fn load_package(lua: Lua, fpath: &Path) -> mlua::Result<()> {
    let fpath = lua.create_string(fpath.as_os_str().as_bytes())?;

    // Delete coroutine library.
    let coroutine = lua.globals().get::<mlua::Table>("coroutine")?;
    let patched_coroutine = lua.create_table()?;
    patched_coroutine.set("yield", coroutine.get::<mlua::Function>("yield")?)?;
    lua.globals().set("coroutine", patched_coroutine)?;

    // Sync version of "path.canonicalize".
    let path_canonicalize = lua.create_function(|lua, str: mlua::String| {
        let str = str.as_bytes();
        let path = path::Path::new(OsStr::from_bytes(&str));
        let path = fs::canonicalize(path)
            .map_err(io::LuaError::from)
            .map_err(LuaError::from)
            .map_err(mlua::Error::external)?;
        lua.create_string(path.as_os_str().as_bytes())
    })?;

    lua.load(include_lua!("./package.lua"))
        .eval::<mlua::Function>()?
        .call((fpath, path_canonicalize))
}
