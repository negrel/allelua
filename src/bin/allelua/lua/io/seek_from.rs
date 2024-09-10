use std::io::SeekFrom;

use mlua::{FromLua, MetaMethod, UserData};

#[derive(Debug, Clone, Copy, FromLua)]
pub struct LuaSeekFrom(pub SeekFrom);

impl UserData for LuaSeekFrom {
    fn add_fields<'lua, F: mlua::UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field("__type", "SeekFrom")
    }

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
