use std::slice;

use mlua::ObjectLike;
use tokio::io::{AsyncBufReadExt, AsyncReadExt};

use super::{LuaError, MaybeClosed};

macro_rules! lua_buffer_from_userdata {
    ($udata:ident, $n:ident) => {{
        let (ptr, len) = $udata
            .get::<mlua::Function>("reserve")?
            .call::<(mlua::Value, usize)>(($udata.to_owned(), $n))?;

        if len == 0 || ptr.is_null() {
            (std::ptr::null_mut(), 0)
        } else {
            let ptr = unsafe { *(ptr.to_pointer() as *const *mut u8) };
            if ptr.is_null() {
                (ptr, 0)
            } else {
                (ptr, len)
            }
        }
    }};
}

macro_rules! slice_from_userdata {
    ($udata:ident, $n:ident) => {{
        let (ptr, len) = lua_buffer_from_userdata!($udata, $n);
        if len == 0 {
            return Ok(0);
        }
        let len = if len < $n { len } else { $n };

        unsafe { slice::from_raw_parts_mut(ptr, len) }
    }};
}

pub fn add_io_reader_methods<
    T: AsyncReadExt + Unpin,
    C: MaybeClosed<T>,
    R: AsMut<C> + 'static,
    M: mlua::UserDataMethods<R>,
>(
    methods: &mut M,
) {
    methods.add_async_method_mut(
        "read",
        |_lua, mut reader, (udata, n): (mlua::AnyUserData, usize)| async move {
            let reader = reader.as_mut().ok_or_broken_pipe()?;

            let buf = slice_from_userdata!(udata, n);

            let read = AsyncReadExt::read(reader, buf)
                .await
                .map_err(super::LuaError::from)
                .map_err(LuaError::from)
                .map_err(mlua::Error::external)?;

            if read > 0 {
                udata
                    .get::<mlua::Function>("commit")?
                    .call::<()>((udata.to_owned(), read))?;
            }

            Ok(read)
        },
    );

    methods.add_async_method_mut("read_to_end", |lua, mut reader, ()| async move {
        let reader = reader.as_mut().ok_or_broken_pipe()?;

        let mut buf = Vec::with_capacity(4096);

        AsyncReadExt::read_to_end(reader, &mut buf)
            .await
            .map_err(super::LuaError::from)
            .map_err(LuaError::from)
            .map_err(mlua::Error::external)?;

        lua.create_string(buf)
    });
}

pub fn add_io_buf_reader_methods<
    T: AsyncBufReadExt + Unpin + 'static,
    C: MaybeClosed<T>,
    R: AsMut<C> + 'static,
    M: mlua::UserDataMethods<R>,
>(
    methods: &mut M,
) {
    add_io_reader_methods(methods);

    methods.add_async_method_mut("read_line", |lua, mut reader, ()| async move {
        let reader = reader.as_mut().ok_or_broken_pipe()?;
        let mut buf = Vec::with_capacity(4096);
        let read = reader
            .read_until(b'\n', &mut buf)
            .await
            .map_err(super::LuaError::from)
            .map_err(LuaError::from)
            .map_err(mlua::Error::external)?;

        let mut slice = &buf[..];

        // Remove LF (\n).
        if buf.last().is_some() {
            slice = &slice[..slice.len() - 1];
        }

        // Remove CR from CRLF.
        if let Some(b'\r') = buf.last() {
            slice = &slice[..slice.len() - 1];
        }

        if read == 0 {
            Ok(mlua::Value::Nil)
        } else {
            Ok(mlua::Value::String(lua.create_string(slice)?))
        }
    });
}
