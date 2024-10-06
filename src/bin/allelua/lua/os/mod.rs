use std::{ffi::OsStr, os::unix::ffi::OsStrExt, path::Path, process::exit};

use mlua::Lua;
use tokio::fs::OpenOptions;

mod child;
mod file;

use child::*;
use file::*;

use super::{error::LuaError, io};

pub fn load_os(lua: Lua) -> mlua::Result<mlua::Table> {
    lua.load_from_function(
        "os",
        lua.create_function(|lua, ()| {
            let os = lua.create_table()?;

            let file_constructors = lua.create_table()?;
            file_constructors.set(
                "open",
                lua.create_async_function(
                    |_lua, (path, mode): (mlua::String, mlua::String)| async move {
                        let path = path.as_bytes();
                        let path = Path::new(OsStr::from_bytes(&path));
                        let mut options = OpenOptions::new();
                        let mode = mode.as_bytes();
                        if mode.contains(&b'c') {
                            options.create(true);
                        }
                        if mode.contains(&b'C') {
                            options.create_new(true);
                        }
                        if mode.contains(&b'r') {
                            options.read(true);
                        }
                        if mode.contains(&b'w') {
                            options.write(true);
                        }
                        if mode.contains(&b'a') {
                            options.write(true).append(true);
                        }

                        let file = options
                            .open(path)
                            .await
                            .map_err(io::LuaError::from)
                            .map_err(LuaError::from)
                            .map_err(mlua::Error::external)?;
                        Ok(LuaFile::new(file))
                    },
                )?,
            )?;
            os.set("File", file_constructors)?;

            os.set(
                "exit",
                lua.create_function(|_, code: i32| {
                    exit(code);
                    #[allow(unreachable_code)]
                    Ok(())
                })?,
            )?;

            os.set("exec", lua.create_function(exec)?)?;

            Ok(os)
        })?,
    )
}
