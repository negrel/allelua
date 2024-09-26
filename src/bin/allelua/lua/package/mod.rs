use std::{os::unix::ffi::OsStrExt, path::Path};

use mlua::Lua;

pub fn load_package(lua: Lua, fpath: &Path) -> mlua::Result<()> {
    let fpath = lua.create_string(fpath.as_os_str().as_bytes())?;

    // Delete coroutine library.
    let coroutine = lua.globals().get::<mlua::Table>("coroutine")?;
    let patched_coroutine = lua.create_table()?;
    patched_coroutine.set("yield", coroutine.get::<mlua::Function>("yield")?)?;
    lua.globals().set("coroutine", patched_coroutine)?;

    lua.load(include_str!("./package.lua"))
        .eval::<mlua::Function>()?
        .call::<()>(fpath)
}
