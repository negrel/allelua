use std::{ffi::OsStr, os::unix::ffi::OsStrExt, path};

use mlua::{FromLua, Lua};

pub fn load_path(lua: &Lua) -> mlua::Result<mlua::Table> {
    lua.load_from_function(
        "path",
        lua.create_function(|lua, ()| {
            let table = lua.create_table()?;

            table.set(
                "absolute",
                lua.create_function(|lua, str: mlua::String| {
                    let path = path::Path::new(OsStr::from_bytes(str.as_bytes()));
                    let path = path::absolute(path).map_err(mlua::Error::runtime)?;
                    lua.create_string(path.as_os_str().as_bytes())
                })?,
            )?;

            table.set(
                "exists",
                lua.create_function(|_lua, str: mlua::String| {
                    match path::Path::new(OsStr::from_bytes(str.as_bytes())).try_exists() {
                        Ok(exists) => Ok(exists),
                        Err(err) => Err(mlua::Error::runtime(err)),
                    }
                })?,
            )?;

            table.set(
                "is_file",
                lua.create_function(|_lua, str: mlua::String| {
                    Ok(path::Path::new(OsStr::from_bytes(str.as_bytes())).is_file())
                })?,
            )?;

            table.set(
                "is_dir",
                lua.create_function(|_lua, str: mlua::String| {
                    Ok(path::Path::new(OsStr::from_bytes(str.as_bytes())).is_dir())
                })?,
            )?;

            table.set(
                "is_symlink",
                lua.create_function(|_lua, str: mlua::String| {
                    Ok(path::Path::new(OsStr::from_bytes(str.as_bytes())).is_symlink())
                })?,
            )?;

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
