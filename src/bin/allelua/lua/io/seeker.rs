use std::io::SeekFrom;

use mlua::{FromLua, MetaMethod, UserData};
use tokio::io::AsyncSeekExt;

use super::Closable;

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
