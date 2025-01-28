use std::{
    ffi::OsStr,
    os::unix::ffi::OsStrExt,
    path::{self, Path, PathBuf},
};

use mlua::Lua;

use super::io;
use crate::{include_lua, lua_string_as_path};

pub fn load_package(lua: Lua, fpath: &Path) -> mlua::Result<()> {
    let fpath = lua.create_string(fpath.as_os_str().as_bytes())?;

    // Sync version of "path.canonicalize".
    let resolve_path = lua.create_function(|lua, str: mlua::String| {
        let str = str.as_bytes();
        let path = path::Path::new(OsStr::from_bytes(&str));
        let path = resolve_path(path)?;
        lua.create_string(path.as_os_str().as_bytes())
    })?;

    let list_files = lua.create_function(|lua, path: mlua::String| {
        lua_string_as_path!(path = path);

        let rd = std::fs::read_dir(path).map_err(io::LuaError::from)?;
        let files = lua.create_table()?;
        for entry in rd {
            let path = entry.map_err(io::LuaError::from)?.path();
            files.push(path)?;
        }

        Ok(files)
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
        .call((fpath, resolve_path, list_files, caller_source))
}

// Resolves and canonicalize path relative to current working directory
// (@/my/path), path relative to current file (./my/path) or absolute path.
pub fn resolve_path(path: &Path) -> mlua::Result<PathBuf> {
    if path.starts_with("@/") {
        let cwd = std::env::current_dir().map_err(io::LuaError::from)?;
        Ok(cwd.join(path.strip_prefix("@/").unwrap()))
    } else if path.is_relative() {
        Ok(path.canonicalize().map_err(io::LuaError::from)?)
    } else {
        Ok(path.to_owned())
    }
}
