use std::ops::Deref;

use mlua::{AnyUserData, FromLua, Lua, MetaMethod, UserData};

struct LuaByteBuffer(Vec<u8>);

impl Deref for LuaByteBuffer {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

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
        // methods.add_meta_method(MetaMethod::Index, |lua, b, v: mlua::Value| {
        //     if let Ok(i) = usize::from_lua(v, lua) {
        //         Ok(b.0.get(i - 1).map(ToOwned::to_owned))
        //     } else {
        //         Ok(Some(0u8))
        //     }
        // });
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

        methods.add_method_mut("push", |lua, b, values: mlua::MultiValue| {
            for v in values {
                let byte = u8::from_lua(v, lua)?;
                b.0.push(byte);
            }
            Ok(b.0.len())
        });
        methods.add_method_mut("unshift", |lua, b, values: mlua::MultiValue| {
            for v in values {
                let byte = u8::from_lua(v, lua)?;
                b.0.insert(0, byte);
            }

            Ok((b.0.pop(), b.0.len()))
        });

        methods.add_method_mut("pop", |_lua, b, ()| Ok((b.0.pop(), b.0.len())));
        methods.add_method_mut("shift", |_lua, b, ()| Ok((b.0.pop(), b.0.len())));

        methods.add_method_mut("reserve", |_lua, b, additional: usize| {
            b.0.reserve(additional);
            Ok(())
        });

        methods.add_method_mut("resize", |_lua, b, (len, byte): (usize, Option<u8>)| {
            b.0.resize(len, byte.unwrap_or(0));
            Ok(())
        });

        methods.add_method_mut("fill", |_lua, b, byte: u8| {
            b.0.fill(byte);
            Ok(())
        });

        methods.add_method_mut("truncate", |_lua, b, len: usize| {
            b.0.truncate(len);
            Ok(())
        });

        methods.add_method("__clone", |_lua, b, ()| Ok(LuaByteBuffer(b.0.clone())));
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
                "fromstring",
                lua.create_function(|_lua, str: mlua::String| {
                    Ok(LuaByteBuffer(str.as_bytes().to_owned()))
                })?,
            )?;
            byte.set("Buffer", buf_constructors)?;

            Ok(byte)
        })?,
    )
}
