use std::io::SeekFrom;

use mlua::{FromLua, MetaMethod, UserData};
use tokio::io::AsyncSeekExt;

use crate::lua::error::LuaError;

use super::closer::MaybeClosed;

#[derive(Debug, Clone, Copy, FromLua)]
struct LuaSeekFrom(SeekFrom);

impl UserData for LuaSeekFrom {
    fn add_fields<F: mlua::UserDataFields<Self>>(_fields: &mut F) {}

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
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

pub fn add_io_seeker_methods<
    T: AsyncSeekExt + Unpin,
    C: MaybeClosed<T>,
    R: AsMut<C> + 'static,
    M: mlua::UserDataMethods<R>,
>(
    methods: &mut M,
) {
    methods.add_async_method_mut(
        "seek",
        |_lua, mut seeker, seek_from: LuaSeekFrom| async move {
            let seeker = seeker.as_mut().ok_or_broken_pipe()?;
            let pos = seeker
                .seek(seek_from.0)
                .await
                .map_err(super::LuaError::from)
                .map_err(LuaError::from)
                .map_err(mlua::Error::external)?;

            Ok(pos)
        },
    );

    methods.add_async_method_mut("rewind", |_lua, mut seeker, ()| async move {
        let seeker = seeker.as_mut().ok_or_broken_pipe()?;
        let pos = seeker
            .rewind()
            .await
            .map_err(super::LuaError::from)
            .map_err(LuaError::from)
            .map_err(mlua::Error::external)?;

        Ok(pos)
    });
}
