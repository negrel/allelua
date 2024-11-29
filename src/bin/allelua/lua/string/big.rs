use std::rc::Rc;

use mlua::{FromLua, UserData};

use super::{LuaString, MaybeBig};

/// LuaBigString define a rust allocated string that can exceed Lua string size
/// limits.
#[derive(Debug, Clone, FromLua, PartialEq, Eq)]
pub struct LuaBigString {
    rc: Rc<[u8]>,
    data: &'static [u8],
}

impl<'a> LuaString<'a> for LuaBigString {
    type Bytes = &'a [u8];

    fn as_bytes(&'a self) -> Self::Bytes {
        self.data
    }

    fn create_string(_: &mlua::prelude::Lua, bytes: &[u8]) -> mlua::Result<Self> {
        Ok(Self::from(bytes))
    }
}

impl UserData for LuaBigString {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "string.Big");
    }

    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        // Native lua string methods.
        methods.add_method("rep", |_, str, n: usize| Ok(Self::from(str.data.repeat(n))));
        // TODO: use locale to change case.
        methods.add_method("lower", |_, str, ()| {
            Ok(Self::from(str.data.to_ascii_lowercase()))
        });
        // TODO: use locale to change case.
        methods.add_method("upper", |_, str, ()| {
            Ok(Self::from(str.data.to_ascii_uppercase()))
        });
        methods.add_method("len", |_, str, ()| Ok(str.data.len()));
        methods.add_method("reverse", |_, str, ()| {
            let mut data = str.data.to_owned();
            data.reverse();
            Ok(Self::from(data))
        });
        methods.add_method("slice", |_, str, (mut i, j): (isize, Option<isize>)| {
            let ilen: isize = str.data.len().try_into().map_err(mlua::Error::runtime)?;
            let ilen = ilen + 1;
            let mut j = j.unwrap_or(-1);

            // Translate negative indices.
            if i < 0 {
                i += ilen;
            }
            if j < 0 {
                j += ilen;
            }

            // i is less than 1, correct it to 1 (beginning of string).
            let start = if i < 1 { 1 } else { i.unsigned_abs() };
            // j is greater than string length, correct it to string length.
            let end = if j >= ilen {
                str.data.len()
            } else {
                j.unsigned_abs()
            };

            // Empty slice.
            if start > end {
                return Ok(Self::from(""));
            }

            Ok(Self {
                rc: str.rc.to_owned(),
                data: &str.data[(start - 1)..end],
            })
        });
        methods.add_method("eq", |_, str, rhs: MaybeBig| {
            Ok(*str.as_bytes() == *rhs.as_bytes())
        });

        methods.add_meta_method(mlua::MetaMethod::ToString, |lua, str, ()| {
            // Truncate string if it exceed max lua string size.
            if str.data.len() > 2147483391 {
                lua.create_string(&str.data[..2147483391])
            } else {
                lua.create_string(str.data)
            }
        });

        methods.add_meta_method(mlua::MetaMethod::Concat, |_, str, rhs: MaybeBig| {
            let bytes = [str.data, &rhs.as_bytes()].concat();

            Ok(LuaBigString::from(bytes))
        });

        methods.add_meta_method(mlua::MetaMethod::Len, |_, str, ()| Ok(str.data.len()));

        methods.add_meta_method(mlua::MetaMethod::Lt, |_, str, rhs: MaybeBig| {
            match str.data.cmp(&rhs.as_bytes()) {
                std::cmp::Ordering::Less => Ok(true),
                _ => Ok(false),
            }
        });

        methods.add_meta_method(mlua::MetaMethod::Le, |_, str, rhs: MaybeBig| {
            match str.data.cmp(&rhs.as_bytes()) {
                std::cmp::Ordering::Less | std::cmp::Ordering::Equal => Ok(true),
                _ => Ok(false),
            }
        });

        methods.add_meta_method(mlua::MetaMethod::Eq, |_, str, rhs: Self| Ok(str == &rhs));
    }
}

impl<T: AsRef<[u8]>> From<T> for LuaBigString {
    fn from(value: T) -> Self {
        let rc: Rc<[u8]> = Rc::from(value.as_ref());
        // Safety: This is safe as reference will live as long as Self exists.
        let data: &'static [u8] = unsafe { std::mem::transmute(rc.as_ref()) };
        Self { rc, data }
    }
}
