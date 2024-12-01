use std::{env, ffi::OsString, os::unix::ffi::OsStrExt, process::exit};

use mlua::{IntoLua, Lua};
use tokio::fs;

mod args;
mod child;
mod env_vars;
mod file;
mod pipe;
mod stdio;

use args::*;
use child::*;
use env_vars::*;
use file::*;
use pipe::*;
use stdio::*;

use crate::lua_string_as_path;

pub fn load_os(lua: &Lua, args: Vec<OsString>) -> mlua::Result<mlua::Table> {
    lua.load_from_function(
        "os",
        lua.create_function(move |lua, ()| {
            let os = lua.create_table()?;
            lua.globals().set("os", os.clone())?;

            let os_mt = lua.create_table()?;
            os_mt.set(
                "__index",
                lua.create_function(|lua, (_, key): (mlua::Value, mlua::String)| {
                    match key.as_bytes().as_ref() {
                        b"stdin" => LuaFile::stdin().map(|f| f.into_lua(lua))?,
                        b"stdout" => LuaFile::stdout().map(|f| f.into_lua(lua))?,
                        b"stderr" => LuaFile::stderr().map(|f| f.into_lua(lua))?,
                        _ => Ok(mlua::Value::Nil),
                    }
                })?,
            )?;
            os.set_metatable(Some(os_mt));

            let file_constructors = lua.create_table()?;
            file_constructors.set("open", lua.create_async_function(open_file)?)?;
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
            os.set(
                "exec",
                lua.create_async_function(|lua, args| async move { exec(&lua, args).await })?,
            )?;

            // Create a pipe.
            os.set("pipe", lua.create_async_function(pipe)?)?;

            Ok(os)
        })?,
    )
}
