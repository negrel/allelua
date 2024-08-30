use std::io::SeekFrom;
use std::os::fd::AsRawFd;
use std::{ffi::OsStr, os::unix::ffi::OsStrExt, path::Path};

use mlua::{FromLua, Lua, MetaMethod, UserData};
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

#[derive(Debug)]
struct LuaFile(File);

impl UserData for LuaFile {
    fn add_fields<'lua, F: mlua::prelude::LuaUserDataFields<'lua, Self>>(_fields: &mut F) {}

    fn add_methods<'lua, M: mlua::prelude::LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, f, ()| {
            let address = f as *const _ as usize;
            let fd = f.0.as_raw_fd();
            Ok(format!("File(fd={fd}) 0x{address:x}"))
        });

        methods.add_async_method_mut("write", |_, f, str: mlua::String| async move {
            f.0.write_all(str.as_bytes())
                .await
                .map_err(mlua::Error::external)?;
            Ok(())
        });

        methods.add_async_method_mut("read_to_end", |lua, f, ()| async move {
            let mut buf = Vec::new();
            f.0.read_to_end(&mut buf).await?;
            Ok(lua.create_string(buf))
        });

        methods.add_async_method_mut("read_exact", |lua, f, n: usize| async move {
            let mut buf = vec![0; n];
            f.0.read_exact(&mut buf).await?;
            Ok(lua.create_string(buf))
        });

        methods.add_async_method_mut("seek", |_, f, seek_from: LuaSeekFrom| async move {
            f.0.seek(seek_from.0).await.map_err(mlua::Error::external)?;
            Ok(())
        });

        methods.add_async_method_mut("flush", |_, f, ()| async {
            f.0.flush().await.map_err(mlua::Error::external)?;
            Ok(())
        });
    }
}

#[derive(Debug, Clone, Copy, FromLua)]
struct LuaSeekFrom(SeekFrom);

impl UserData for LuaSeekFrom {
    fn add_fields<'lua, F: mlua::prelude::LuaUserDataFields<'lua, Self>>(_fields: &mut F) {}

    fn add_methods<'lua, M: mlua::prelude::LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
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

async fn file_open<'lua>(
    _: &'lua Lua,
    (path, mode): (mlua::String<'lua>, mlua::String<'lua>),
) -> mlua::Result<LuaFile> {
    let path = Path::new(OsStr::from_bytes(path.as_bytes()));
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

    let file = options.open(path).await.map_err(mlua::Error::external)?;
    Ok(LuaFile(file))
}

pub fn load_fs(lua: &'static Lua) -> mlua::Result<mlua::Table<'static>> {
    lua.load_from_function(
        "fs",
        lua.create_function(|_, ()| {
            let fs = lua.create_table()?;

            let file = lua.create_table()?;
            file.set("open", lua.create_async_function(file_open)?)?;
            fs.set("File", file)?;

            let seek_from = lua.create_table()?;
            seek_from.set(
                "start",
                lua.create_function(|_, offset: u64| Ok(LuaSeekFrom(SeekFrom::Start(offset))))?,
            )?;
            seek_from.set(
                "end",
                lua.create_function(|_, offset: i64| Ok(LuaSeekFrom(SeekFrom::End(offset))))?,
            )?;
            seek_from.set(
                "current",
                lua.create_function(|_, offset: i64| Ok(LuaSeekFrom(SeekFrom::Current(offset))))?,
            )?;
            fs.set("SeekFrom", seek_from)?;

            Ok(fs)
        })?,
    )
}
