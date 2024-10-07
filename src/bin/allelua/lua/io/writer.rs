use std::slice;

use mlua::ObjectLike;
use tokio::io::AsyncWriteExt;

use super::{add_io_closer_methods, Close, LuaBuffer, LuaError, MaybeClosed};

macro_rules! lua_buffer_from_userdata {
    ($udata:ident) => {{
        let (ptr, len) = $udata
            .get::<mlua::Function>("ref")?
            .call::<(mlua::Value, usize)>($udata.to_owned())?;

        if len == 0 || ptr.is_null() {
            (std::ptr::null(), 0)
        } else {
            let ptr = unsafe { *(ptr.to_pointer() as *const *const u8) };
            if ptr.is_null() {
                (ptr, 0)
            } else {
                (ptr, len)
            }
        }
    }};
}

macro_rules! slice_from_userdata {
    ($udata:ident) => {{
        let (ptr, len) = lua_buffer_from_userdata!($udata);
        if len == 0 {
            return Ok(0);
        }
        unsafe { slice::from_raw_parts(ptr, len) }
    }};
}

pub fn add_io_writer_methods<
    T: AsyncWriteExt + Unpin,
    C: MaybeClosed<T>,
    R: AsMut<C> + 'static,
    M: mlua::UserDataMethods<R>,
>(
    methods: &mut M,
) {
    methods.add_async_method_mut(
        "write",
        |_, mut writer, udata: mlua::AnyUserData| async move {
            let writer = writer.as_mut().ok_or_broken_pipe()?;
            let buf = slice_from_userdata!(udata);

            let write = writer
                .write(buf)
                .await
                .map_err(super::LuaError::from)
                .map_err(LuaError::from)
                .map_err(mlua::Error::external)?;

            if write > 0 {
                udata
                    .get::<mlua::Function>("skip")?
                    .call::<()>((udata.to_owned(), write))?;
            }

            Ok(write)
        },
    );

    methods.add_async_method_mut(
        "write_buf",
        |_, mut writer, buf: LuaBuffer<'static>| async move {
            let writer = writer.as_mut().ok_or_broken_pipe()?;

            writer
                .write_all(buf.0)
                .await
                .map_err(super::LuaError::from)
                .map_err(LuaError::from)
                .map_err(mlua::Error::external)?;

            Ok(buf.0.len())
        },
    );

    methods.add_async_method_mut(
        "write_all",
        |_, mut writer, udata: mlua::AnyUserData| async move {
            let writer = writer.as_mut().ok_or_broken_pipe()?;
            let buf = slice_from_userdata!(udata);

            writer
                .write_all(buf)
                .await
                .map_err(super::LuaError::from)
                .map_err(LuaError::from)
                .map_err(mlua::Error::external)?;

            udata
                .get::<mlua::Function>("skip")?
                .call::<()>((udata.to_owned(), buf.len()))?;

            Ok(buf.len())
        },
    );

    methods.add_async_method_mut("flush", |_, mut writer, ()| async move {
        let writer = writer.as_mut().ok_or_broken_pipe()?;
        writer
            .flush()
            .await
            .map_err(super::LuaError::from)
            .map_err(LuaError::from)
            .map_err(mlua::Error::external)
    });
}

pub fn add_io_write_closer_methods<
    T: AsyncWriteExt + Unpin,
    C: MaybeClosed<T> + Close,
    R: AsMut<C> + 'static,
    M: mlua::UserDataMethods<R>,
>(
    methods: &mut M,
) {
    add_io_closer_methods(methods);
    add_io_writer_methods(methods);

    methods.add_async_method_mut("close", |_, mut writer, ()| async move {
        let writer_closer = writer.as_mut();
        let writer = writer_closer.ok_or_broken_pipe()?;
        writer
            .shutdown()
            .await
            .map_err(super::LuaError::from)
            .map_err(LuaError::from)
            .map_err(mlua::Error::external)?;

        writer_closer.close()?;
        Ok(())
    });
}
