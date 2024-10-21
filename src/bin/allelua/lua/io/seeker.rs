use std::io::SeekFrom;

use mlua::{FromLua, IntoLua, MetaMethod, UserData};
use tokio::io::AsyncSeekExt;

use super::Closable;

#[derive(Debug, Clone, Copy, FromLua)]
pub(super) struct LuaSeekFrom(SeekFrom);

impl LuaSeekFrom {
    pub(super) fn start(n: u64) -> Self {
        LuaSeekFrom(SeekFrom::Start(n))
    }

    pub(super) fn current(n: i64) -> Self {
        LuaSeekFrom(SeekFrom::Current(n))
    }

    pub(super) fn end(n: i64) -> Self {
        LuaSeekFrom(SeekFrom::End(n))
    }
}

impl UserData for LuaSeekFrom {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("mode", |_, seek| match seek.0 {
            SeekFrom::Start(_) => Ok("start"),
            SeekFrom::End(_) => Ok("end"),
            SeekFrom::Current(_) => Ok("current"),
        });

        fields.add_field_method_get("offset", |lua, seek| match seek.0 {
            SeekFrom::Start(offset) => offset.into_lua(lua),
            SeekFrom::End(offset) => offset.into_lua(lua),
            SeekFrom::Current(offset) => offset.into_lua(lua),
        })
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, seek, ()| {
            let address = seek as *const _ as usize;
            let str = match seek.0 {
                SeekFrom::Start(offset) => {
                    format!("SeekFrom(mode=start, offset={offset}) 0x{address:x}")
                }
                SeekFrom::End(offset) => {
                    format!("SeekFrom(mode=end, offset={offset}) 0x{address:x}")
                }
                SeekFrom::Current(offset) => {
                    format!("SeekFrom(mode=current, offset={offset}) 0x{address:x}")
                }
            };
            Ok(str)
        });
    }
}

pub fn add_io_seek_methods<
    T: AsyncSeekExt + Unpin + 'static,
    R: AsRef<Closable<T>> + 'static,
    M: mlua::UserDataMethods<R>,
>(
    methods: &mut M,
) {
    methods.add_async_method("seek", |_lua, seeker, seek_from: LuaSeekFrom| async move {
        let mut seeker = seeker.as_ref().get().await?;
        let pos = seeker.seek(seek_from.0).await?;
        Ok(pos)
    });

    methods.add_async_method("rewind", |_lua, seeker, ()| async move {
        let mut seeker = seeker.as_ref().get().await?;
        let pos = seeker.rewind().await?;
        Ok(pos)
    });
}
