use mlua::{AnyUserData, Lua, MetaMethod, UserData};

struct LuaByteBuffer(Vec<u8>);

impl UserData for LuaByteBuffer {
    fn add_fields<'lua, F: mlua::UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field("__type", "ByteBuffer");
    }

    fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::Len, |_, b, ()| Ok(b.0.len()));
        methods.add_meta_method(MetaMethod::ToString, |lua, b, ()| {
            Ok(lua.create_string(&*b.0))
        });
        methods.add_meta_method(MetaMethod::Eq, |_, b, other: AnyUserData| {
            let other = other.borrow::<LuaByteBuffer>()?;
            Ok(b.0 == other.0)
        });
        methods.add_meta_method(MetaMethod::Index, |_, b, i: usize| {
            Ok(b.0.get(i - 1).map(ToOwned::to_owned))
        });
        methods.add_meta_method_mut(MetaMethod::NewIndex, |_, b, (i, byte): (usize, u8)| {
            let i = i - 1;
            if i >= b.0.len() {
                return Err(mlua::Error::runtime(format!(
                    "index out of bound: the len is {} but the index is {i}",
                    b.0.len(),
                )));
            }

            b.0[i] = byte;
            Ok(b.0.len())
        });
    }
}

pub fn load_byte(lua: &'static Lua) -> mlua::Result<mlua::Table> {
    lua.load_from_function(
        "byte",
        lua.create_function(|_, ()| {
            let byte = lua.create_table()?;

            let buf_constructors = lua.create_table()?;
            buf_constructors.set(
                "new",
                lua.create_function(|_lua, (len, fill): (Option<usize>, Option<u8>)| {
                    let vec = match len {
                        Some(len) => vec![fill.unwrap_or(0); len],
                        None => Vec::new(),
                    };
                    Ok(LuaByteBuffer(vec))
                })?,
            )?;
            buf_constructors.set(
                "from_string",
                lua.create_function(|_lua, str: mlua::String| {
                    Ok(LuaByteBuffer(str.as_bytes().to_owned()))
                })?,
            )?;
            byte.set("Buffer", buf_constructors)?;

            Ok(byte)
        })?,
    )
}
