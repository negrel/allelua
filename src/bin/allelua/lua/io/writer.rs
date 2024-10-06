use std::slice;

use mlua::ObjectLike;
use tokio::io::AsyncWriteExt;

use super::LuaError;

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
    R: AsMut<T> + 'static,
    M: mlua::UserDataMethods<R>,
>(
    methods: &mut M,
) {
    methods.add_async_method_mut(
        "write",
        |_, mut writer, udata: mlua::AnyUserData| async move {
            let writer = writer.as_mut();
            let buf = slice_from_userdata!(udata);

            let write = writer
                .write(buf)
                .await
                .map_err(super::LuaError::from)
                .map_err(LuaError::from)
                .map_err(mlua::Error::external)?;

            Ok(write)
        },
    );

    methods.add_async_method_mut(
        "write_all",
        |_, mut writer, udata: mlua::AnyUserData| async move {
            let writer = writer.as_mut();
            let buf = slice_from_userdata!(udata);

            writer
                .write_all(buf)
                .await
                .map_err(super::LuaError::from)
                .map_err(LuaError::from)
                .map_err(mlua::Error::external)?;

            Ok(buf.len())
        },
    );
}
