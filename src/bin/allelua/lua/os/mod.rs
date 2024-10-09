use std::{
    env,
    ffi::{OsStr, OsString},
    os::unix::ffi::OsStrExt,
    path::Path,
    process::exit,
};

use mlua::Lua;
use tokio::fs::{self, OpenOptions};

mod args;
mod child;
mod env_vars;
mod file;

use args::*;
use child::*;
use env_vars::*;
use file::*;

use super::{error::LuaError, io};

pub fn load_os(lua: &Lua, args: Vec<OsString>) -> mlua::Result<mlua::Table> {
    lua.load_from_function(
        "os",
        lua.create_function(move |lua, ()| {
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
            file_constructors.set(
                "read",
                lua.create_async_function(|lua, path: mlua::String| async move {
                    let path = path.as_bytes();
                    let path = Path::new(OsStr::from_bytes(&path));
                    let content = fs::read(path)
                        .await
                        .map_err(io::LuaError::from)
                        .map_err(LuaError::from)
                        .map_err(mlua::Error::external)?;

                    lua.create_string(content)
                })?,
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

            os.set(
                "temp_dir",
                lua.create_function(|lua, ()| {
                    lua.create_string(env::temp_dir().as_os_str().as_bytes())
                })?,
            )?;

            os.set(
                "current_dir",
                lua.create_function(|lua, ()| {
                    lua.create_string(env::current_dir()?.as_os_str().as_bytes())
                })?,
            )?;

            // Process environment.
            os.set("env_vars", EnvVars::default())?;
            os.set("args", Args::new(lua, args.clone())?)?;

            // Constants.
            os.set("family", lua.create_string(std::env::consts::FAMILY)?)?;
            os.set("arch", lua.create_string(std::env::consts::ARCH)?)?;
            os.set("os_name", lua.create_string(std::env::consts::OS)?)?;

            // Exec a child process.
            os.set("exec", lua.create_function(exec)?)?;

            Ok(os)
        })?,
    )
}
