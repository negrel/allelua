use std::os::fd::AsRawFd;
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt},
};

use mlua::{ExternalError, ExternalResult, IntoLuaMulti, Lua, MetaMethod, UserData};

use crate::lua::io;

#[derive(Debug)]
pub struct LuaFile(pub File);

impl UserData for LuaFile {
    fn add_fields<'lua, F: mlua::UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field("__type", "File");
    }

    fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, f, ()| {
            let address = f as *const _ as usize;
            let fd = f.0.as_raw_fd();
            Ok(format!("File(fd={fd}) 0x{address:x}"))
        });

        methods.add_async_method_mut("write", |lua, f, str: mlua::String| async move {
            f.0.write_all(str.as_bytes())
                .await
                .map(|()| 2)
                .map_err(mlua::Error::external)
                .into_lua_multi(lua)
        });

        methods.add_async_method_mut("read_to_end", |lua, f, ()| async move {
            let mut buf = Vec::new();
            f.0.read_to_end(&mut buf).await.into_lua_err()?;
            Ok(lua.create_string(buf))
        });

        let read_exact = |lua: &'lua Lua, f: &'lua mut LuaFile, n: usize| async move {
            let mut buf = vec![0; n];
            f.0.read_exact(&mut buf).await?;
            Ok(lua.create_string(buf))
        };
        methods.add_async_method_mut("read_exact", read_exact);
        // methods.add_async_method_mut("pread_exact", move |lua, f, n: usize| async move {
        //     read_exact(lua, f, n).await.into_lua_multi(lua)
        // });

        methods.add_async_method_mut("seek", |_, f, seek_from: io::LuaSeekFrom| async move {
            f.0.seek(seek_from.0).await.into_lua_err()?;
            Ok(())
        });

        methods.add_async_method_mut("flush", |_, f, ()| async {
            f.0.flush().await.into_lua_err()?;
            Ok(())
        });
    }
}
