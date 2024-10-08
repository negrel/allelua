use std::{
    ffi::OsStr,
    os::unix::{ffi::OsStrExt, fs::FileTypeExt},
    path,
};

use mlua::{FromLua, Lua};
use tokio::fs;

#[macro_export]
macro_rules! lua_string_as_path {
    ($path:ident = $str:ident) => {
        let str = $str.to_str()?.to_owned();
        let $path = ::std::path::Path::new(&str);
    };
}

pub fn load_path(lua: Lua) -> mlua::Result<mlua::Table> {
    lua.load_from_function(
        "path",
        lua.create_function(|lua, ()| {
            let table = lua.create_table()?;

            table.set(
                "canonicalize",
                lua.create_async_function(|lua, str: mlua::String| async move {
                    lua_string_as_path!(path = str);
                    lua.create_string(path.as_os_str().as_bytes())
                })?,
            )?;

            table.set(
                "exists",
                lua.create_async_function(|_lua, str: mlua::String| async move {
                    lua_string_as_path!(path = str);
                    fs::try_exists(path).await?;
                    Ok(())
                })?,
            )?;

            table.set(
                "is_file",
                lua.create_async_function(|_lua, str: mlua::String| async move {
                    lua_string_as_path!(path = str);
                    let metadata = fs::metadata(path).await?;

                    Ok(metadata.file_type().is_file())
                })?,
            )?;

            table.set(
                "is_dir",
                lua.create_async_function(|_lua, str: mlua::String| async move {
                    lua_string_as_path!(path = str);
                    let metadata = fs::metadata(path).await?;

                    Ok(metadata.file_type().is_dir())
                })?,
            )?;

            table.set(
                "is_symlink",
                lua.create_async_function(|_lua, str: mlua::String| async move {
                    lua_string_as_path!(path = str);
                    let metadata = fs::symlink_metadata(path).await?;

                    Ok(metadata.file_type().is_symlink())
                })?,
            )?;

            table.set(
                "len",
                lua.create_async_function(|_lua, str: mlua::String| async move {
                    lua_string_as_path!(path = str);
                    let metadata = fs::symlink_metadata(path).await?;

                    Ok(metadata.len())
                })?,
            )?;

            if cfg!(unix) {
                table.set(
                    "is_block_device",
                    lua.create_async_function(|_lua, str: mlua::String| async move {
                        lua_string_as_path!(path = str);
                        let metadata = fs::metadata(path).await?;

                        Ok(metadata.file_type().is_block_device())
                    })?,
                )?;
                table.set(
                    "is_char_device",
                    lua.create_async_function(|_lua, str: mlua::String| async move {
                        lua_string_as_path!(path = str);
                        let metadata = fs::metadata(path).await?;

                        Ok(metadata.file_type().is_char_device())
                    })?,
                )?;
                table.set(
                    "is_socket",
                    lua.create_async_function(|_lua, str: mlua::String| async move {
                        lua_string_as_path!(path = str);
                        let metadata = fs::metadata(path).await?;

                        Ok(metadata.file_type().is_socket())
                    })?,
                )?;
                table.set(
                    "is_fifo",
                    lua.create_async_function(|_lua, str: mlua::String| async move {
                        lua_string_as_path!(path = str);
                        let metadata = fs::metadata(path).await?;

                        Ok(metadata.file_type().is_fifo())
                    })?,
                )?;
            }

            table.set(
                "is_absolute",
                lua.create_function(|_lua, str: mlua::String| {
                    lua_string_as_path!(path = str);
                    Ok(path.is_absolute())
                })?,
            )?;

            table.set(
                "is_relative",
                lua.create_function(|_lua, str: mlua::String| {
                    lua_string_as_path!(path = str);
                    Ok(path.is_relative())
                })?,
            )?;

            table.set(
                "file_name",
                lua.create_function(|lua, str: mlua::String| {
                    lua_string_as_path!(path = str);
                    match path.file_name() {
                        Some(fname) => Ok(Some(lua.create_string(fname.as_bytes())?)),
                        None => Ok(None),
                    }
                })?,
            )?;

            table.set(
                "file_stem",
                lua.create_function(|lua, str: mlua::String| {
                    lua_string_as_path!(path = str);
                    match path.file_stem() {
                        Some(fname) => Ok(Some(lua.create_string(fname.as_bytes())?)),
                        None => Ok(None),
                    }
                })?,
            )?;

            table.set(
                "extension",
                lua.create_function(|lua, str: mlua::String| {
                    lua_string_as_path!(path = str);
                    match path.extension() {
                        Some(fname) => Ok(Some(lua.create_string(fname.as_bytes())?)),
                        None => Ok(None),
                    }
                })?,
            )?;

            table.set(
                "parent",
                lua.create_function(|lua, str: mlua::String| {
                    lua_string_as_path!(path = str);
                    match path.parent() {
                        Some(parent) => Ok(Some(lua.create_string(parent.as_os_str().as_bytes())?)),
                        None => Ok(None),
                    }
                })?,
            )?;

            table.set(
                "join",
                lua.create_function(|lua, (base, join): (mlua::String, mlua::MultiValue)| {
                    lua_string_as_path!(base = base);
                    let mut base = base.to_owned();

                    for component in join {
                        if component.is_nil() || component.is_null() {
                            continue;
                        }
                        let component = mlua::String::from_lua(component, lua)?;
                        let component = component.as_bytes();
                        let component = path::Path::new(OsStr::from_bytes(&component));
                        base = base.join(component);
                    }

                    lua.create_string(base.as_os_str().as_bytes())
                })?,
            )?;

            table.set(
                "with_file_name",
                lua.create_function(|lua, (path, fname): (mlua::String, mlua::String)| {
                    lua_string_as_path!(path = path);
                    lua_string_as_path!(fname = fname);
                    lua.create_string(path.to_owned().with_file_name(fname).as_os_str().as_bytes())
                })?,
            )?;

            table.set(
                "with_extension",
                lua.create_function(|lua, (path, extension): (mlua::String, mlua::String)| {
                    lua_string_as_path!(path = path);
                    lua_string_as_path!(extension = extension);
                    lua.create_string(
                        path.to_owned()
                            .with_extension(extension)
                            .as_os_str()
                            .as_bytes(),
                    )
                })?,
            )?;

            Ok(table)
        })?,
    )
}
