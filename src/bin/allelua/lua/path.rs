use std::{
    ffi::OsStr,
    os::unix::{ffi::OsStrExt, fs::FileTypeExt},
    path,
};

use mlua::{FromLua, Lua};
use tokio::fs;

pub fn load_path(lua: &Lua) -> mlua::Result<mlua::Table> {
    lua.load_from_function(
        "path",
        lua.create_function(|lua, ()| {
            let table = lua.create_table()?;

            table.set(
                "canonicalize",
                lua.create_async_function(|lua, str: mlua::String| async move {
                    let path = path::Path::new(OsStr::from_bytes(str.as_bytes()));
                    let path = fs::canonicalize(path).await.map_err(mlua::Error::runtime)?;
                    lua.create_string(path.as_os_str().as_bytes())
                })?,
            )?;

            table.set(
                "exists",
                lua.create_async_function(|_lua, str: mlua::String| async move {
                    let path = path::Path::new(OsStr::from_bytes(str.as_bytes()));
                    fs::try_exists(path).await.map_err(mlua::Error::runtime)
                })?,
            )?;

            table.set(
                "is_file",
                lua.create_async_function(|_lua, str: mlua::String| async move {
                    let path = path::Path::new(OsStr::from_bytes(str.as_bytes()));
                    let metadata = fs::metadata(path).await.map_err(mlua::Error::runtime)?;

                    Ok(metadata.file_type().is_file())
                })?,
            )?;

            table.set(
                "is_dir",
                lua.create_async_function(|_lua, str: mlua::String| async move {
                    let path = path::Path::new(OsStr::from_bytes(str.as_bytes()));
                    let metadata = fs::metadata(path).await.map_err(mlua::Error::runtime)?;

                    Ok(metadata.file_type().is_dir())
                })?,
            )?;

            table.set(
                "is_symlink",
                lua.create_async_function(|_lua, str: mlua::String| async move {
                    let path = path::Path::new(OsStr::from_bytes(str.as_bytes()));
                    let metadata = fs::symlink_metadata(path)
                        .await
                        .map_err(mlua::Error::runtime)?;

                    Ok(metadata.file_type().is_symlink())
                })?,
            )?;

            table.set(
                "len",
                lua.create_async_function(|_lua, str: mlua::String| async move {
                    let path = path::Path::new(OsStr::from_bytes(str.as_bytes()));
                    let metadata = fs::symlink_metadata(path)
                        .await
                        .map_err(mlua::Error::runtime)?;

                    Ok(metadata.len())
                })?,
            )?;

            if cfg!(unix) {
                table.set(
                    "is_block_device",
                    lua.create_async_function(|_lua, str: mlua::String| async move {
                        let path = path::Path::new(OsStr::from_bytes(str.as_bytes()));
                        let metadata = fs::metadata(path).await.map_err(mlua::Error::runtime)?;

                        Ok(metadata.file_type().is_block_device())
                    })?,
                )?;
                table.set(
                    "is_char_device",
                    lua.create_async_function(|_lua, str: mlua::String| async move {
                        let path = path::Path::new(OsStr::from_bytes(str.as_bytes()));
                        let metadata = fs::metadata(path).await.map_err(mlua::Error::runtime)?;

                        Ok(metadata.file_type().is_char_device())
                    })?,
                )?;
                table.set(
                    "is_socket",
                    lua.create_async_function(|_lua, str: mlua::String| async move {
                        let path = path::Path::new(OsStr::from_bytes(str.as_bytes()));
                        let metadata = fs::metadata(path).await.map_err(mlua::Error::runtime)?;

                        Ok(metadata.file_type().is_socket())
                    })?,
                )?;
                table.set(
                    "is_fifo",
                    lua.create_async_function(|_lua, str: mlua::String| async move {
                        let path = path::Path::new(OsStr::from_bytes(str.as_bytes()));
                        let metadata = fs::metadata(path).await.map_err(mlua::Error::runtime)?;

                        Ok(metadata.file_type().is_fifo())
                    })?,
                )?;
            }

            table.set(
                "is_absolute",
                lua.create_function(|_lua, str: mlua::String| {
                    Ok(path::Path::new(OsStr::from_bytes(str.as_bytes())).is_absolute())
                })?,
            )?;

            table.set(
                "is_relative",
                lua.create_function(|_lua, str: mlua::String| {
                    Ok(path::Path::new(OsStr::from_bytes(str.as_bytes())).is_relative())
                })?,
            )?;

            table.set(
                "file_name",
                lua.create_function(|lua, str: mlua::String| {
                    match path::Path::new(OsStr::from_bytes(str.as_bytes())).file_name() {
                        Some(fname) => Ok(Some(lua.create_string(fname.as_bytes())?)),
                        None => Ok(None),
                    }
                })?,
            )?;

            table.set(
                "file_stem",
                lua.create_function(|lua, str: mlua::String| {
                    match path::Path::new(OsStr::from_bytes(str.as_bytes())).file_stem() {
                        Some(fname) => Ok(Some(lua.create_string(fname.as_bytes())?)),
                        None => Ok(None),
                    }
                })?,
            )?;

            table.set(
                "extension",
                lua.create_function(|lua, str: mlua::String| {
                    match path::Path::new(OsStr::from_bytes(str.as_bytes())).extension() {
                        Some(fname) => Ok(Some(lua.create_string(fname.as_bytes())?)),
                        None => Ok(None),
                    }
                })?,
            )?;

            table.set(
                "parent",
                lua.create_function(|lua, str: mlua::String| {
                    match path::Path::new(OsStr::from_bytes(str.as_bytes())).parent() {
                        Some(parent) => Ok(Some(lua.create_string(parent.as_os_str().as_bytes())?)),
                        None => Ok(None),
                    }
                })?,
            )?;

            table.set(
                "join",
                lua.create_function(|lua, (base, join): (mlua::String, mlua::MultiValue)| {
                    let mut base = path::PathBuf::from(OsStr::from_bytes(base.as_bytes()));
                    for component in join {
                        let component = mlua::String::from_lua(component, lua)?;
                        let component = path::Path::new(OsStr::from_bytes(component.as_bytes()));
                        base = base.join(component);
                    }

                    lua.create_string(base.as_os_str().as_bytes())
                })?,
            )?;

            table.set(
                "with_file_name",
                lua.create_function(|lua, (path, fname): (mlua::String, mlua::String)| {
                    lua.create_string(
                        path::Path::new(OsStr::from_bytes(path.as_bytes()))
                            .with_file_name(path::Path::new(OsStr::from_bytes(fname.as_bytes())))
                            .as_os_str()
                            .as_bytes(),
                    )
                })?,
            )?;

            table.set(
                "with_extension",
                lua.create_function(|lua, (path, fname): (mlua::String, mlua::String)| {
                    lua.create_string(
                        path::Path::new(OsStr::from_bytes(path.as_bytes()))
                            .with_extension(path::Path::new(OsStr::from_bytes(fname.as_bytes())))
                            .as_os_str()
                            .as_bytes(),
                    )
                })?,
            )?;

            Ok(table)
        })?,
    )
}
