use std::{
    ffi::OsStr,
    fs::Metadata,
    ops::Deref,
    os::unix::{ffi::OsStrExt, fs::FileTypeExt},
    path::{self, Path},
};

use mlua::{FromLua, Lua, MetaMethod, UserData};
use tokio::fs;

use crate::lua::io;

#[macro_export]
macro_rules! lua_string_as_path {
    ($path:ident = $str:ident) => {
        let str = $str.to_str()?.to_owned();
        let $path = ::std::path::Path::new(&str);
    };
}

pub fn load_path(lua: &Lua) -> mlua::Result<mlua::Table> {
    lua.load_from_function(
        "path",
        lua.create_function(|lua, ()| {
            let path = lua.create_table()?;
            lua.globals().set("path", path.clone())?;

            path.set(
                "canonicalize",
                lua.create_async_function(|lua, str: mlua::String| async move {
                    lua_string_as_path!(path = str);
                    let path = fs::canonicalize(path).await.map_err(io::LuaError::from)?;
                    lua.create_string(path.as_os_str().as_bytes())
                })?,
            )?;

            path.set(
                "exists",
                lua.create_async_function(|_lua, str: mlua::String| async move {
                    lua_string_as_path!(path = str);
                    fs::try_exists(path).await.map_err(io::LuaError::from)?;
                    Ok(())
                })?,
            )?;

            path.set(
                "metadata",
                lua.create_async_function(|_lua, str: mlua::String| async move {
                    lua_string_as_path!(path = str);
                    let metadata = metadata(path).await?;

                    Ok(LuaMetadata(metadata))
                })?,
            )?;

            path.set(
                "is_file",
                lua.create_async_function(|_lua, str: mlua::String| async move {
                    lua_string_as_path!(path = str);
                    let metadata = metadata(path).await?;

                    Ok(metadata.file_type().is_file())
                })?,
            )?;

            path.set(
                "is_dir",
                lua.create_async_function(|_lua, str: mlua::String| async move {
                    lua_string_as_path!(path = str);
                    let metadata = metadata(path).await?;

                    Ok(metadata.file_type().is_dir())
                })?,
            )?;

            path.set(
                "is_symlink",
                lua.create_async_function(|_lua, str: mlua::String| async move {
                    lua_string_as_path!(path = str);
                    let metadata = fs::symlink_metadata(path)
                        .await
                        .map_err(io::LuaError::from)?;

                    Ok(metadata.file_type().is_symlink())
                })?,
            )?;

            path.set(
                "len",
                lua.create_async_function(|_lua, str: mlua::String| async move {
                    lua_string_as_path!(path = str);
                    let metadata = fs::symlink_metadata(path)
                        .await
                        .map_err(io::LuaError::from)?;

                    Ok(metadata.len())
                })?,
            )?;

            if cfg!(unix) {
                path.set(
                    "is_block_device",
                    lua.create_async_function(|_lua, str: mlua::String| async move {
                        lua_string_as_path!(path = str);
                        let metadata = metadata(path).await?;

                        Ok(metadata.file_type().is_block_device())
                    })?,
                )?;
                path.set(
                    "is_char_device",
                    lua.create_async_function(|_lua, str: mlua::String| async move {
                        lua_string_as_path!(path = str);
                        let metadata = metadata(path).await?;

                        Ok(metadata.file_type().is_char_device())
                    })?,
                )?;
                path.set(
                    "is_socket",
                    lua.create_async_function(|_lua, str: mlua::String| async move {
                        lua_string_as_path!(path = str);
                        let metadata = metadata(path).await?;

                        Ok(metadata.file_type().is_socket())
                    })?,
                )?;
                path.set(
                    "is_fifo",
                    lua.create_async_function(|_lua, str: mlua::String| async move {
                        lua_string_as_path!(path = str);
                        let metadata = metadata(path).await?;

                        Ok(metadata.file_type().is_fifo())
                    })?,
                )?;
            }

            path.set(
                "is_absolute",
                lua.create_function(|_lua, str: mlua::String| {
                    lua_string_as_path!(path = str);
                    Ok(path.is_absolute())
                })?,
            )?;

            path.set(
                "is_relative",
                lua.create_function(|_lua, str: mlua::String| {
                    lua_string_as_path!(path = str);
                    Ok(path.is_relative())
                })?,
            )?;

            path.set(
                "file_name",
                lua.create_function(|lua, str: mlua::String| {
                    lua_string_as_path!(path = str);
                    match path.file_name() {
                        Some(fname) => Ok(Some(lua.create_string(fname.as_bytes())?)),
                        None => Ok(None),
                    }
                })?,
            )?;

            path.set(
                "file_stem",
                lua.create_function(|lua, str: mlua::String| {
                    lua_string_as_path!(path = str);
                    match path.file_stem() {
                        Some(fname) => Ok(Some(lua.create_string(fname.as_bytes())?)),
                        None => Ok(None),
                    }
                })?,
            )?;

            path.set(
                "extension",
                lua.create_function(|lua, str: mlua::String| {
                    lua_string_as_path!(path = str);
                    match path.extension() {
                        Some(fname) => Ok(Some(lua.create_string(fname.as_bytes())?)),
                        None => Ok(None),
                    }
                })?,
            )?;

            path.set(
                "parent",
                lua.create_function(|lua, str: mlua::String| {
                    lua_string_as_path!(path = str);
                    match path.parent() {
                        Some(parent) => Ok(Some(lua.create_string(parent.as_os_str().as_bytes())?)),
                        None => Ok(None),
                    }
                })?,
            )?;

            path.set(
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

            path.set(
                "with_file_name",
                lua.create_function(|lua, (path, fname): (mlua::String, mlua::String)| {
                    lua_string_as_path!(path = path);
                    lua_string_as_path!(fname = fname);
                    lua.create_string(path.to_owned().with_file_name(fname).as_os_str().as_bytes())
                })?,
            )?;

            path.set(
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

            Ok(path)
        })?,
    )
}

async fn metadata(path: &Path) -> Result<Metadata, io::LuaError> {
    fs::metadata(path).await.map_err(io::LuaError::from)
}

#[derive(Debug)]
pub struct LuaMetadata(pub Metadata);

impl Deref for LuaMetadata {
    type Target = Metadata;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl UserData for LuaMetadata {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("len", |_, metadata| Ok(metadata.len()));
        fields.add_field_method_get("is_file", |_, metadata| Ok(metadata.is_file()));
        fields.add_field_method_get("is_dir", |_, metadata| Ok(metadata.is_dir()));
        fields.add_field_method_get("is_symlink", |_, metadata| Ok(metadata.is_symlink()));

        if cfg!(unix) {
            fields.add_field_method_get("is_block_device", |_, metadata| {
                Ok(metadata.file_type().is_block_device())
            });
            fields.add_field_method_get("is_char_device", |_, metadata| {
                Ok(metadata.file_type().is_char_device())
            });
            fields.add_field_method_get("is_socket", |_, metadata| {
                Ok(metadata.file_type().is_socket())
            });
            fields
                .add_field_method_get("is_fifo", |_, metadata| Ok(metadata.file_type().is_fifo()));
        }

        fields.add_field_method_get("file_type", |_, metadata| {
            let ft = metadata.file_type();
            if ft.is_file() {
                Ok("file")
            } else if ft.is_dir() {
                Ok("dir")
            } else if cfg!(unix) {
                if ft.is_symlink() {
                    Ok("symlink")
                } else if ft.is_block_device() {
                    Ok("block_device")
                } else if ft.is_char_device() {
                    Ok("char_device")
                } else if ft.is_socket() {
                    Ok("socket")
                } else if ft.is_fifo() {
                    Ok("fifo")
                } else {
                    Ok("unknown")
                }
            } else {
                Ok("unknown")
            }
        });
    }

    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, metadata, ()| {
            let address = metadata as *const _ as usize;
            Ok(format!("Metadata 0x{address:x}"))
        });
    }
}
