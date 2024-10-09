use std::{
    env,
    ffi::{OsStr, OsString},
    os::unix::ffi::OsStrExt,
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

use crate::lua_string_as_path;

pub fn load_os(lua: &Lua, args: Vec<OsString>) -> mlua::Result<mlua::Table> {
    lua.load_from_function(
        "os",
        lua.create_function(move |lua, ()| {
            let os = lua.create_table()?;

            let file_constructors = lua.create_table()?;
            file_constructors.set(
                "open",
                lua.create_async_function(
                    |_lua, (path, opt_table): (mlua::String, Option<mlua::Table>)| async move {
                        lua_string_as_path!(path = path);
                        let mut options = OpenOptions::new();
                        let mut buffer_size = None;

                        if let Some(opt_table) = opt_table {
                            if let Some(true) = opt_table.get::<Option<bool>>("create")? {
                                options.create(true);
                            }

                            if let Some(true) = opt_table.get::<Option<bool>>("create_new")? {
                                options.create_new(true);
                            }

                            if let Some(true) = opt_table.get::<Option<bool>>("read")? {
                                options.read(true);
                            }

                            if let Some(true) = opt_table.get::<Option<bool>>("write")? {
                                options.write(true);
                            }

                            if let Some(true) = opt_table.get::<Option<bool>>("append")? {
                                options.append(true);
                            }

                            buffer_size = opt_table.get::<Option<usize>>("buffer_size")?;
                        }

                        let file = options.open(path).await?;

                        Ok(LuaFile::new(file, buffer_size))
                    },
                )?,
            )?;
            file_constructors.set(
                "read",
                lua.create_async_function(|lua, path: mlua::String| async move {
                    lua_string_as_path!(path = path);
                    let content = fs::read(path).await?;
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

            os.set(
                "hard_link",
                lua.create_async_function(
                    |_lua, (src_str, dst_str): (mlua::String, mlua::String)| async move {
                        lua_string_as_path!(src = src_str);
                        lua_string_as_path!(dst = dst_str);
                        fs::hard_link(src, dst).await?;
                        Ok(())
                    },
                )?,
            )?;

            #[cfg(unix)]
            os.set(
                "symlink",
                lua.create_async_function(
                    |_lua, (src_str, dst_str): (mlua::String, mlua::String)| async move {
                        lua_string_as_path!(src = src_str);
                        lua_string_as_path!(dst = dst_str);
                        fs::symlink(src, dst).await?;
                        Ok(())
                    },
                )?,
            )?;

            os.set(
                "rename",
                lua.create_async_function(
                    |_lua, (src_str, dst_str): (mlua::String, mlua::String)| async move {
                        lua_string_as_path!(src = src_str);
                        lua_string_as_path!(dst = dst_str);
                        fs::rename(src, dst).await?;
                        Ok(())
                    },
                )?,
            )?;

            os.set(
                "create_dir",
                lua.create_async_function(|_lua, str: mlua::String| async move {
                    lua_string_as_path!(path = str);
                    fs::create_dir(path).await?;
                    Ok(())
                })?,
            )?;

            os.set(
                "create_dir_all",
                lua.create_async_function(|_lua, str: mlua::String| async move {
                    lua_string_as_path!(path = str);
                    fs::create_dir_all(path).await?;
                    Ok(())
                })?,
            )?;

            os.set(
                "remove_dir",
                lua.create_async_function(|_lua, str: mlua::String| async move {
                    lua_string_as_path!(path = str);
                    fs::remove_dir(path).await?;
                    Ok(())
                })?,
            )?;

            os.set(
                "remove_dir_all",
                lua.create_async_function(|_lua, str: mlua::String| async move {
                    lua_string_as_path!(path = str);
                    fs::remove_dir_all(path).await?;
                    Ok(())
                })?,
            )?;

            os.set(
                "remove_file",
                lua.create_async_function(|_lua, str: mlua::String| async move {
                    lua_string_as_path!(path = str);
                    fs::remove_file(path).await?;
                    Ok(())
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
