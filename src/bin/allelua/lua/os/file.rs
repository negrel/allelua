use std::io::SeekFrom;
use std::os::fd::AsRawFd;

use mlua::{FromLua, MetaMethod, UserData};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

use crate::lua::error::LuaError;
use crate::lua::io;

#[derive(Debug)]
pub(super) struct LuaFile(pub File);

impl UserData for LuaFile {
    fn add_fields<'lua, F: mlua::UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field("__type", "File")
    }

    fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, f, ()| {
            let address = f as *const _ as usize;
            let fd = f.0.as_raw_fd();
            Ok(format!("File(fd={fd}) 0x{address:x}"))
        });

        methods.add_async_method_mut("write", |_, f, str: mlua::String| async move {
            f.0.write_all(str.as_bytes())
                .await
                .map_err(io::LuaError::from)
                .map_err(LuaError::from)
                .map_err(mlua::Error::external)?;
            Ok(())
        });

        methods.add_async_method_mut("read_to_end", |lua, f, ()| async move {
            let mut buf = Vec::new();
            f.0.read_to_end(&mut buf)
                .await
                .map_err(io::LuaError::from)
                .map_err(LuaError::from)
                .map_err(mlua::Error::external)?;
            Ok(lua.create_string(buf))
        });

        methods.add_async_method_mut("read_exact", |lua, f, n: usize| async move {
            let mut buf = vec![0; n];
            f.0.read_exact(&mut buf)
                .await
                .map_err(io::LuaError::from)
                .map_err(LuaError::from)
                .map_err(mlua::Error::external)?;
            Ok(lua.create_string(buf))
        });

        methods.add_async_method_mut("seek", |_, f, seek_from: LuaSeekFrom| async move {
            f.0.seek(seek_from.0)
                .await
                .map_err(io::LuaError::from)
                .map_err(LuaError::from)
                .map_err(mlua::Error::external)?;
            Ok(())
        });

        methods.add_async_method_mut("flush", |_, f, ()| async {
            f.0.flush()
                .await
                .map_err(io::LuaError::from)
                .map_err(LuaError::from)
                .map_err(mlua::Error::external)?;
            Ok(())
        });
    }
}

#[derive(Debug, Clone, Copy, FromLua)]
struct LuaSeekFrom(SeekFrom);

impl UserData for LuaSeekFrom {
    fn add_fields<'lua, F: mlua::UserDataFields<'lua, Self>>(_fields: &mut F) {}

    fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, f, ()| {
            let address = f as *const _ as usize;
            let str = match f.0 {
                SeekFrom::Start(offset) => format!("SeekFrom(start={offset}) 0x{address:x}"),
                SeekFrom::End(offset) => format!("SeekFrom(end={offset}) 0x{address:x}"),
                SeekFrom::Current(offset) => format!("SeekFrom(current={offset}) 0x{address:x}"),
            };
            Ok(str)
        });
    }
}
